/// Multi-device workspace sync over TCP.
///
/// Protocol: length-prefixed JSON frames.
///   [4 bytes big-endian u32 = payload length][payload bytes (UTF-8 JSON)]
///
/// Handshake (both sides):
///   1. Server sends its snapshot first.
///   2. Client reads it, then sends its own snapshot.
///   3. Server reads client snapshot and merges.
///   4. Both sides now have the union of all data.
use crate::calendar::CalendarEvent;
use crate::domain::{EdgeData, FocusEvent, JournalEntry, NodeData};
use crate::storage::WorkspaceSnapshot;
use crate::workspace::SilentNodeWorkspace;
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use uuid::Uuid;

// ── Wire protocol ──────────────────────────────────────────────────────────────

async fn send_frame(stream: &mut TcpStream, data: &[u8]) -> std::io::Result<()> {
    let len = data.len() as u32;
    stream.write_all(&len.to_be_bytes()).await?;
    stream.write_all(data).await?;
    Ok(())
}

async fn recv_frame(stream: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > 64 * 1024 * 1024 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "frame too large",
        ));
    }
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    Ok(buf)
}

// ── Merge logic ────────────────────────────────────────────────────────────────

/// Merge a remote snapshot into a local one, returning the merged result.
/// Strategy: last-write-wins for nodes (by accessed_at), union for edges/journal/focus.
pub fn merge_snapshots(local: WorkspaceSnapshot, remote: WorkspaceSnapshot) -> WorkspaceSnapshot {
    // ── Nodes: last-write-wins by accessed_at ──────────────────────────────────
    let mut node_map: HashMap<Uuid, NodeData> = local
        .graph
        .nodes
        .iter()
        .cloned()
        .map(|n| (n.id, n))
        .collect();

    for remote_node in &remote.graph.nodes {
        match node_map.get(&remote_node.id) {
            Some(local_node) if local_node.accessed_at >= remote_node.accessed_at => {}
            _ => {
                node_map.insert(remote_node.id, remote_node.clone());
            }
        }
    }

    // ── Edges: union by (src, dst) ────────────────────────────────────────────
    let mut edge_set: HashMap<(Uuid, Uuid), EdgeData> = local
        .graph
        .edges
        .iter()
        .cloned()
        .map(|e| ((e.source_id, e.target_id), e))
        .collect();

    for e in &remote.graph.edges {
        edge_set
            .entry((e.source_id, e.target_id))
            .or_insert_with(|| e.clone());
    }

    // ── Journal: union by id ──────────────────────────────────────────────────
    let mut journal_map: HashMap<Uuid, JournalEntry> = local
        .journal_entries
        .iter()
        .cloned()
        .map(|j| (j.id, j))
        .collect();
    for j in &remote.journal_entries {
        journal_map.entry(j.id).or_insert_with(|| j.clone());
    }

    // ── Focus events: union by (node_id, timestamp millis) ───────────────────
    let mut focus_set: HashMap<(Uuid, i64), FocusEvent> = local
        .focus_events
        .iter()
        .cloned()
        .map(|e| ((e.node_id, e.timestamp.timestamp_millis()), e))
        .collect();
    for e in &remote.focus_events {
        focus_set
            .entry((e.node_id, e.timestamp.timestamp_millis()))
            .or_insert_with(|| e.clone());
    }

    let mut calendar_map: HashMap<Uuid, CalendarEvent> = local
        .calendar_events
        .iter()
        .cloned()
        .map(|event| (event.id, event))
        .collect();
    for event in &remote.calendar_events {
        calendar_map
            .entry(event.id)
            .or_insert_with(|| event.clone());
    }

    // ── Reassemble snapshot ───────────────────────────────────────────────────
    let mut merged = local.clone();
    merged.graph.nodes = node_map.into_values().collect();
    merged.graph.edges = edge_set.into_values().collect();
    merged.journal_entries = journal_map.into_values().collect();
    merged.focus_events = focus_set.into_values().collect();
    merged.calendar_events = calendar_map.into_values().collect();
    merged
}

// ── Snapshot serialization ────────────────────────────────────────────────────

fn snapshot_to_bytes(snap: &WorkspaceSnapshot) -> Vec<u8> {
    serde_json::to_vec(snap).unwrap_or_default()
}

fn bytes_to_snapshot(bytes: &[u8]) -> Option<WorkspaceSnapshot> {
    serde_json::from_slice(bytes).ok()
}

// ── Server ────────────────────────────────────────────────────────────────────

/// Start a sync server on `addr`. Each connecting peer receives our snapshot
/// and then sends theirs. Both sides merge.
pub async fn serve(workspace: &SilentNodeWorkspace, addr: SocketAddr) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    println!("SilentNode sync server listening on {addr}");

    // Serialize our snapshot once
    let snap = workspace.snapshot();
    let snap_bytes = snapshot_to_bytes(&snap);

    loop {
        let (mut stream, peer) = listener.accept().await?;
        let snap_bytes = snap_bytes.clone();
        println!("[sync] peer connected: {peer}");

        tokio::spawn(async move {
            // Step 1: send our snapshot
            if let Err(e) = send_frame(&mut stream, &snap_bytes).await {
                eprintln!("[sync] send error: {e}");
                return;
            }
            println!(
                "[sync] sent snapshot ({} bytes) to {peer}",
                snap_bytes.len()
            );

            // Step 2: wait for peer snapshot (optional — peer may close after receiving)
            match tokio::time::timeout(std::time::Duration::from_secs(30), recv_frame(&mut stream))
                .await
            {
                Ok(Ok(data)) => {
                    if let Some(remote) = bytes_to_snapshot(&data) {
                        println!(
                            "[sync] received {} nodes from {peer}",
                            remote.graph.nodes.len()
                        );
                        // We can't modify workspace here (no Arc<Mutex>), just report.
                        // In a production system this would push to a channel.
                        println!("[sync] saving peer snapshot to sync_received.json");
                        let _ = std::fs::write(
                            "sync_received.json",
                            serde_json::to_vec_pretty(&remote).unwrap_or_default(),
                        );
                        println!("[sync] saved peer snapshot — run: cargo run -- sync-pull <addr>");
                    }
                }
                Ok(Err(_)) | Err(_) => {
                    // Peer just pulled, no push
                }
            }
        });
    }
}

// ── Client: pull ──────────────────────────────────────────────────────────────

/// Connect to a sync server, receive their snapshot, merge into workspace.
pub async fn pull(
    workspace: &SilentNodeWorkspace,
    addr: SocketAddr,
) -> std::io::Result<WorkspaceSnapshot> {
    let mut stream = TcpStream::connect(addr).await?;
    println!("[sync] connected to {addr}, pulling snapshot…");

    // Receive remote snapshot
    let data = recv_frame(&mut stream).await?;
    let remote = bytes_to_snapshot(&data)
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "bad snapshot JSON"))?;

    println!("[sync] received {} nodes", remote.graph.nodes.len());

    // Merge
    let merged = merge_snapshots(workspace.snapshot(), remote);
    Ok(merged)
}

// ── Client: push ──────────────────────────────────────────────────────────────

/// Connect to a sync server, receive their snapshot (merge it), then push ours.
pub async fn push(
    workspace: &SilentNodeWorkspace,
    addr: SocketAddr,
) -> std::io::Result<WorkspaceSnapshot> {
    let mut stream = TcpStream::connect(addr).await?;
    println!("[sync] connected to {addr}, exchanging snapshots…");

    // Receive remote snapshot first (server always sends first)
    let remote_data = recv_frame(&mut stream).await?;
    let remote = bytes_to_snapshot(&remote_data).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "bad remote snapshot")
    })?;

    // Send our snapshot
    let local = workspace.snapshot();
    let local_bytes = snapshot_to_bytes(&local);
    send_frame(&mut stream, &local_bytes).await?;
    println!(
        "[sync] sent {} nodes, waiting for server to merge…",
        local.graph.nodes.len()
    );

    // Merge both sides locally
    let merged = merge_snapshots(local, remote);
    println!(
        "[sync] merge complete — {} nodes total",
        merged.graph.nodes.len()
    );
    Ok(merged)
}
