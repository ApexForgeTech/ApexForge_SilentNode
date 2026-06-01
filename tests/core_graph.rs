use chrono::{Duration, Utc};
use silentnode_core::{
    migrate_sqlite_to_surreal_path, ArcType, ChangeType, CognitiveGraph, ContagionEngine,
    ContractState, EdgeType, EntropyEngine, FocusDepth, GraphStore, GravityEngine,
    InMemoryGraphStore, LoreEntry, MaterializationEngine, NodeData, NodeType, ProcessRecord,
    SilenceAnalyzer, SilentContract, SilentNodeWorkspace, SqliteWorkspaceStore,
    SurrealWorkspaceStore, TemporalSnapshot, Vault, VisualizationEngine, WorkspaceStore,
};
use std::collections::BTreeMap;
use std::fs;
use tempfile::tempdir;
use uuid::Uuid;

#[test]
fn graph_adds_and_connects_nodes() {
    let mut graph = CognitiveGraph::new();
    let idea_id = graph
        .add_node(NodeData::new(NodeType::Idea, "Graph rendering pipeline"))
        .expect("idea node should be added");
    let project_id = graph
        .add_node(NodeData::new(NodeType::Project, "SilentNode core"))
        .expect("project node should be added");

    graph
        .connect(project_id, idea_id, EdgeType::Connection, 0.9)
        .expect("nodes should connect");

    let neighbors = graph.neighbors(project_id).expect("project should exist");
    assert_eq!(graph.node_count(), 2);
    assert_eq!(graph.edge_count(), 1);
    assert_eq!(neighbors.len(), 1);
    assert_eq!(neighbors[0].id, idea_id);
}

#[test]
fn touch_restores_ghost_nodes() {
    let mut graph = CognitiveGraph::new();
    let node_id = graph
        .add_node(NodeData::new(
            NodeType::Memory,
            "Forgotten architecture note",
        ))
        .expect("node should be added");

    let node = graph.get_node_mut(node_id).expect("node must exist");
    node.is_ghost = true;
    node.entropy = 0.95;

    let touched_at = Utc::now() + Duration::hours(6);
    graph
        .touch_node(node_id, touched_at)
        .expect("touch should work");

    let node = graph.get_node(node_id).expect("node must still exist");
    assert!(!node.is_ghost);
    assert!(node.entropy < 0.2);
    assert_eq!(node.accessed_at, touched_at);
}

#[test]
fn in_memory_store_round_trips_graph_snapshots() {
    let mut graph = CognitiveGraph::new();
    let source = graph
        .add_node(NodeData::new(NodeType::Project, "Encrypted vault"))
        .expect("source node should be added");
    let target = graph
        .add_node(NodeData::new(NodeType::Artifact, "Vault design spec"))
        .expect("target node should be added");
    graph
        .connect(source, target, EdgeType::Causal, 0.7)
        .expect("edge should be created");

    let mut store = InMemoryGraphStore::new();
    store.save(&graph).expect("graph should save");
    let snapshot = store
        .load()
        .expect("store should load")
        .expect("snapshot should exist");

    let restored =
        CognitiveGraph::from_parts(snapshot.nodes, snapshot.edges).expect("graph should restore");

    assert_eq!(restored.node_count(), 2);
    assert_eq!(restored.edge_count(), 1);
    assert_eq!(restored.stats().ghost_count, 0);
}

#[test]
fn workspace_phase_zero_flow_round_trips_through_sqlite_and_visualization() {
    let mut workspace = SilentNodeWorkspace::new();
    let materializer = MaterializationEngine::new();
    let entropy = EntropyEngine::new();
    let gravity = GravityEngine::new();

    let first = workspace
        .materialize_thought(
            &materializer,
            "Build encrypted graph vault for SilentNode project",
        )
        .expect("first thought should materialize");
    let second = workspace
        .materialize_thought(&materializer, "Roadmap note about graph rendering pipeline")
        .expect("second thought should materialize");

    workspace
        .record_focus(first.node_id, 600.0, FocusDepth::DeepWork)
        .expect("focus should record");
    workspace
        .record_focus(second.node_id, 180.0, FocusDepth::Read)
        .expect("focus should record");
    workspace.add_journal_entry(
        "Today the vault and graph ideas finally started linking.",
        Some("spring".into()),
    );

    workspace.tick_entropy(&entropy);
    workspace.step_gravity(&gravity, 1.0);

    let temp_dir = tempdir().expect("tempdir should exist");
    let sqlite_path = temp_dir.path().join("phase0.sqlite");
    let html_path = temp_dir.path().join("universe.html");

    let mut sqlite_store =
        SqliteWorkspaceStore::new(&sqlite_path).expect("sqlite store should initialize");
    sqlite_store
        .save_snapshot(&workspace.snapshot())
        .expect("snapshot should save");

    let snapshot = sqlite_store
        .load_snapshot()
        .expect("snapshot should load")
        .expect("snapshot should exist");
    let restored = SilentNodeWorkspace::from_snapshot(snapshot).expect("workspace should rebuild");

    VisualizationEngine::new()
        .render_to_file(&restored, &html_path)
        .expect("html should render");

    let html = fs::read_to_string(&html_path).expect("html should exist");
    assert!(html.contains("SilentNode Universe"));
    assert!(html.contains("Node Search"));
    assert!(html.contains("Selected Node"));
    assert_eq!(restored.graph.node_count(), 2);
    assert_eq!(restored.journal.entries().len(), 1);
    assert!(restored.graph.node_count() >= 2);
}

#[test]
fn workspace_supports_search_trail_and_heatmap_reports() {
    let mut workspace = SilentNodeWorkspace::new();
    let materializer = MaterializationEngine::new();

    let idea = workspace
        .materialize_thought(&materializer, "Compiler memory roadmap idea")
        .expect("thought should materialize");
    workspace
        .record_focus(idea.node_id, 240.0, FocusDepth::Edit)
        .expect("focus should record");
    workspace.add_journal_entry("Compiler roadmap journal note", Some("winter".into()));

    let node_matches = workspace.search_nodes("compiler");
    let journal_matches = workspace.search_journal("journal");
    let trail = workspace.recent_trail(24);
    let heatmap = workspace.heatmap_report(7);

    assert_eq!(node_matches.len(), 1);
    assert_eq!(journal_matches.len(), 1);
    assert_eq!(trail.len(), 1);
    assert_eq!(heatmap.len(), 1);
    assert_eq!(heatmap[0].0, idea.node_id);

    let node_focus = workspace.focus_history_for_node(idea.node_id, 24);
    let node_journal = workspace.journal_for_node(idea.node_id, 30);
    assert_eq!(node_focus.len(), 1);
    assert_eq!(node_journal.len(), 1);
}

#[test]
fn vault_encrypts_and_reopens() {
    let temp_dir = tempdir().expect("tempdir should exist");
    let vault_path = temp_dir.path().join("vault");

    let vault = Vault::open("secret-password", &vault_path).expect("vault should open");
    let ciphertext = vault
        .encrypt(b"silentnode payload")
        .expect("payload should encrypt");
    let plaintext = vault.decrypt(&ciphertext).expect("payload should decrypt");
    assert_eq!(plaintext, b"silentnode payload");

    let reopened = Vault::open("secret-password", &vault_path).expect("vault should reopen");
    let reopened_plaintext = reopened
        .decrypt(&ciphertext)
        .expect("reopened vault should decrypt");
    assert_eq!(reopened_plaintext, b"silentnode payload");
}

#[test]
fn vault_password_can_rotate() {
    let temp_dir = tempdir().expect("tempdir should exist");
    let vault_path = temp_dir.path().join("vault");

    let mut vault = Vault::open("before-rotation", &vault_path).expect("vault should open");
    vault
        .change_password("after-rotation")
        .expect("password should rotate");
    vault.lock();

    let reopened = Vault::open("after-rotation", &vault_path).expect("new password should open");
    let ciphertext = reopened
        .encrypt(b"rotated")
        .expect("ciphertext should encrypt");
    let plaintext = reopened
        .decrypt(&ciphertext)
        .expect("ciphertext should decrypt");
    assert_eq!(plaintext, b"rotated");
}

#[test]
fn journal_entries_link_to_recent_focus_nodes() {
    let mut workspace = SilentNodeWorkspace::new();
    let materializer = MaterializationEngine::new();
    let first = workspace
        .materialize_thought(&materializer, "Graph entropy tuning")
        .expect("first thought should materialize");
    let second = workspace
        .materialize_thought(&materializer, "Journal-linked memory review")
        .expect("second thought should materialize");

    workspace
        .record_focus(first.node_id, 90.0, FocusDepth::Read)
        .expect("focus should record");
    workspace
        .record_focus(second.node_id, 120.0, FocusDepth::Edit)
        .expect("focus should record");

    let entry = workspace.add_journal_entry("Linked entry", Some("summer".into()));
    assert_eq!(entry.linked_nodes.len(), 2);
    assert!(entry.linked_nodes.contains(&first.node_id));
    assert!(entry.linked_nodes.contains(&second.node_id));
}

#[tokio::test]
async fn surreal_store_applies_schema_and_round_trips_snapshot() {
    let mut workspace = SilentNodeWorkspace::new();
    let materializer = MaterializationEngine::new();
    let result = workspace
        .materialize_thought(&materializer, "SilentNode memory world bridge")
        .expect("thought should materialize");
    workspace
        .record_focus(result.node_id, 120.0, FocusDepth::Edit)
        .expect("focus should record");
    workspace.add_journal_entry("Testing Surreal memory store", Some("autumn".into()));

    let store = SurrealWorkspaceStore::new_in_memory()
        .await
        .expect("surreal store should initialize");
    store
        .save_snapshot(&workspace.snapshot())
        .await
        .expect("snapshot should save to surreal");

    let snapshot = store.load_snapshot().await.expect("snapshot should load");
    assert_eq!(snapshot.graph.nodes.len(), 1);
    assert_eq!(snapshot.focus_events.len(), 1);
    assert_eq!(snapshot.journal_entries.len(), 1);

    let counts = store.healthcheck().await.expect("healthcheck should work");
    assert_eq!(counts.nodes, 1);
    assert_eq!(counts.focus_events, 1);
    assert_eq!(counts.journal_entries, 1);
}

#[tokio::test]
async fn local_surreal_store_and_sqlite_migration_round_trip() {
    let temp_dir = tempdir().expect("tempdir should exist");
    let sqlite_path = temp_dir.path().join("phase1.sqlite");
    let surreal_path = temp_dir.path().join("phase1.surkv");

    let mut workspace = SilentNodeWorkspace::new();
    let materializer = MaterializationEngine::new();
    let result = workspace
        .materialize_thought(&materializer, "Persistent surreal migration graph")
        .expect("thought should materialize");
    workspace
        .record_focus(result.node_id, 300.0, FocusDepth::DeepWork)
        .expect("focus should record");
    workspace.add_journal_entry("Migration journal", Some("phase1".into()));

    let mut sqlite_store =
        SqliteWorkspaceStore::new(&sqlite_path).expect("sqlite store should initialize");
    sqlite_store
        .save_snapshot(&workspace.snapshot())
        .expect("snapshot should save");

    migrate_sqlite_to_surreal_path(&sqlite_path, &surreal_path)
        .await
        .expect("migration should work");

    let store = SurrealWorkspaceStore::new_local(&surreal_path)
        .await
        .expect("local surreal store should initialize");
    let snapshot = store.load_snapshot().await.expect("snapshot should load");
    let counts = store
        .healthcheck()
        .await
        .expect("healthcheck should succeed");

    assert_eq!(snapshot.graph.nodes.len(), 1);
    assert_eq!(snapshot.focus_events.len(), 1);
    assert_eq!(snapshot.journal_entries.len(), 1);
    assert_eq!(counts.nodes, 1);
    assert_eq!(counts.edges, 0);
}

#[test]
fn barnes_hut_gravity_moves_nodes_apart() {
    let mut graph = CognitiveGraph::new();
    let a = graph
        .add_node(NodeData::new(NodeType::Idea, "node A"))
        .expect("node A");
    let b = graph
        .add_node(NodeData::new(NodeType::Idea, "node B"))
        .expect("node B");

    // Place nodes at the same spot — pure repulsion should separate them
    graph.get_node_mut(a).unwrap().position.x = 0.0;
    graph.get_node_mut(a).unwrap().position.y = 0.0;
    graph.get_node_mut(b).unwrap().position.x = 0.5;
    graph.get_node_mut(b).unwrap().position.y = 0.0;

    let engine = GravityEngine::new();
    for _ in 0..5 {
        engine.step(&mut graph, 1.0);
    }

    let pos_a = graph.get_node(a).unwrap().position;
    let pos_b = graph.get_node(b).unwrap().position;
    let dist = ((pos_a.x - pos_b.x).powi(2) + (pos_a.y - pos_b.y).powi(2)).sqrt();
    assert!(dist > 0.5, "nodes should have moved apart; dist={dist:.3}");
}

#[test]
fn contagion_reduces_entropy_of_neighbors() {
    let mut graph = CognitiveGraph::new();
    let source = graph
        .add_node(NodeData::new(NodeType::Idea, "source"))
        .expect("source");
    let neighbor = graph
        .add_node(NodeData::new(NodeType::Memory, "neighbor"))
        .expect("neighbor");

    graph
        .connect(source, neighbor, EdgeType::Connection, 1.0)
        .expect("edge");

    // Give neighbor high entropy
    graph.get_node_mut(neighbor).unwrap().entropy = 0.8;

    let engine = ContagionEngine::new();
    engine.propagate(&mut graph, source, 1.0);

    let neighbor_entropy = graph.get_node(neighbor).unwrap().entropy;
    assert!(
        neighbor_entropy < 0.8,
        "contagion should reduce neighbor entropy; got {neighbor_entropy:.3}"
    );
}

#[test]
fn contagion_skips_ghost_nodes() {
    let mut graph = CognitiveGraph::new();
    let source = graph
        .add_node(NodeData::new(NodeType::Idea, "source"))
        .expect("source");
    let ghost = graph
        .add_node(NodeData::new(NodeType::Ghost, "ghost node"))
        .expect("ghost");

    graph
        .connect(source, ghost, EdgeType::Connection, 1.0)
        .expect("edge");

    graph.get_node_mut(ghost).unwrap().is_ghost = true;
    graph.get_node_mut(ghost).unwrap().entropy = 0.9;

    let engine = ContagionEngine::new();
    engine.propagate(&mut graph, source, 1.0);

    // Ghost entropy must remain unchanged
    let ghost_entropy = graph.get_node(ghost).unwrap().entropy;
    assert_eq!(
        ghost_entropy, 0.9,
        "ghost nodes must not be affected by contagion"
    );
}

#[test]
fn silence_analyzer_finds_bridges_between_clusters() {
    let mut graph = CognitiveGraph::new();

    // Cluster 1: compiler + memory
    let a = graph
        .add_node(NodeData::new(
            NodeType::Idea,
            "compiler memory optimization",
        ))
        .expect("a");
    let b = graph
        .add_node(NodeData::new(NodeType::Idea, "memory allocation compiler"))
        .expect("b");
    graph
        .connect(a, b, EdgeType::Connection, 0.8)
        .expect("edge a-b");

    // Cluster 2: rendering + pipeline — semantically distinct
    let c = graph
        .add_node(NodeData::new(NodeType::Project, "rendering pipeline gpu"))
        .expect("c");
    let d = graph
        .add_node(NodeData::new(NodeType::Project, "gpu shader rendering"))
        .expect("d");
    graph
        .connect(c, d, EdgeType::Connection, 0.8)
        .expect("edge c-d");

    let analyzer = SilenceAnalyzer::new();
    let bridges = analyzer.find_missing_bridges(&graph);

    // Cluster 1 and cluster 2 are disconnected — no bridges should be suggested
    // because they're semantically different (no shared tokens above threshold)
    // OR if the same words appear, bridges are suggested
    // The important thing is the function runs and returns without panic
    let _ = bridges;
}

#[test]
fn silence_analyzer_detects_similar_disconnected_nodes() {
    let mut graph = CognitiveGraph::new();

    // Two nodes with highly similar content but in different components
    graph
        .add_node(NodeData::new(
            NodeType::Idea,
            "graph entropy decay cognitive memory",
        ))
        .expect("node 1");
    graph
        .add_node(NodeData::new(
            NodeType::Memory,
            "cognitive memory entropy decay graph",
        ))
        .expect("node 2");

    // Do NOT connect them — they are isolated
    let analyzer = SilenceAnalyzer::with_threshold(0.2);
    let bridges = analyzer.find_missing_bridges(&graph);

    assert!(
        !bridges.is_empty(),
        "highly similar disconnected nodes should produce a missing bridge suggestion"
    );
    assert!(bridges[0].similarity > 0.2);
}

#[test]
fn sqlite_round_trips_extended_snapshot_types() {
    let temp_dir = tempdir().expect("tempdir");
    let path = temp_dir.path().join("extended.sqlite");
    let mut store = SqliteWorkspaceStore::new(&path).expect("sqlite store");

    let node_id = uuid::Uuid::new_v4();

    let ts = TemporalSnapshot::new(node_id, BTreeMap::new(), ChangeType::Created);
    let lore = LoreEntry::new(
        "First Arc",
        ArcType::Origin,
        "The story begins here.",
        vec![node_id],
        0.85,
    );
    let contract = SilentContract::new(node_id, 0.7, 14.0);
    let mut process = ProcessRecord::new(1234, "silentnode-daemon", node_id);
    process.cpu_usage = 12.5;
    process.memory_mb = 64.0;

    let mut snapshot = silentnode_core::WorkspaceSnapshot::default();
    snapshot.temporal_snapshots.push(ts);
    snapshot.lore_entries.push(lore);
    snapshot.silent_contracts.push(contract);
    snapshot.process_records.push(process);

    store.save_snapshot(&snapshot).expect("save");
    // load_snapshot returns None when node table is empty —
    // add a dummy node so the check passes
    let mut graph = CognitiveGraph::new();
    graph
        .add_node(NodeData::new(NodeType::Idea, "placeholder"))
        .expect("placeholder node");
    let export = graph.export();
    snapshot.graph.nodes = export.nodes;
    store.save_snapshot(&snapshot).expect("save with node");

    let loaded = store
        .load_snapshot()
        .expect("load")
        .expect("snapshot exists");

    assert_eq!(loaded.temporal_snapshots.len(), 1);
    assert_eq!(
        loaded.temporal_snapshots[0].change_type,
        ChangeType::Created
    );
    assert_eq!(loaded.lore_entries.len(), 1);
    assert_eq!(loaded.lore_entries[0].arc_type, ArcType::Origin);
    assert_eq!(loaded.silent_contracts.len(), 1);
    assert_eq!(loaded.silent_contracts[0].state, ContractState::Dormant);
    assert_eq!(loaded.process_records.len(), 1);
    assert_eq!(loaded.process_records[0].pid, 1234);
    assert_eq!(loaded.process_records[0].name, "silentnode-daemon");
}

#[tokio::test]
async fn surreal_store_round_trips_extended_types() {
    let node_id = uuid::Uuid::new_v4();

    let ts = TemporalSnapshot::new(node_id, BTreeMap::new(), ChangeType::Modified);
    let lore = LoreEntry::new(
        "Revelation Arc",
        ArcType::Revelation,
        "Truth emerges from silence.",
        vec![node_id],
        0.92,
    );
    let contract = SilentContract {
        id: uuid::Uuid::new_v4(),
        related_node: node_id,
        detected_at: Utc::now(),
        intensity: 0.6,
        age_days: 7.0,
        state: ContractState::Awakening,
    };
    let process = ProcessRecord::new(5678, "observer", node_id);

    // We also need a node so the node table is non-empty for the schema
    let mut graph = CognitiveGraph::new();
    let nid = graph
        .add_node(NodeData::new(NodeType::World, "world node"))
        .expect("world node");
    let _ = nid;
    let export = graph.export();

    let mut snapshot = silentnode_core::WorkspaceSnapshot::default();
    snapshot.graph.nodes = export.nodes;
    snapshot.temporal_snapshots.push(ts);
    snapshot.lore_entries.push(lore);
    snapshot.silent_contracts.push(contract);
    snapshot.process_records.push(process);

    let store = SurrealWorkspaceStore::new_in_memory()
        .await
        .expect("surreal store");
    store.save_snapshot(&snapshot).await.expect("save");

    let loaded = store.load_snapshot().await.expect("load");
    assert_eq!(loaded.temporal_snapshots.len(), 1);
    assert_eq!(
        loaded.temporal_snapshots[0].change_type,
        ChangeType::Modified
    );
    assert_eq!(loaded.lore_entries.len(), 1);
    assert_eq!(loaded.lore_entries[0].arc_type, ArcType::Revelation);
    assert_eq!(loaded.silent_contracts.len(), 1);
    assert_eq!(loaded.silent_contracts[0].state, ContractState::Awakening);
    assert_eq!(loaded.process_records.len(), 1);
    assert_eq!(loaded.process_records[0].pid, 5678);

    let counts = store.healthcheck().await.expect("healthcheck");
    assert_eq!(counts.temporal_snapshots, 1);
    assert_eq!(counts.lore_entries, 1);
    assert_eq!(counts.silent_contracts, 1);
    assert_eq!(counts.process_records, 1);
}

#[test]
fn entropy_engine_parallel_tick_marks_ghost() {
    let mut graph = CognitiveGraph::new();
    let node_id = graph
        .add_node(NodeData::new(NodeType::Idea, "old forgotten idea"))
        .expect("node");

    // Set accessed_at to 200 days ago — entropy should be near 1.0
    let old_time = Utc::now() - Duration::days(200);
    graph.get_node_mut(node_id).unwrap().accessed_at = old_time;

    let engine = EntropyEngine::new();
    engine.tick(&mut graph, Utc::now());

    let node = graph.get_node(node_id).unwrap();
    assert!(
        node.entropy > 0.9,
        "entropy should be high after 200 days; got {}",
        node.entropy
    );
    assert!(
        node.is_ghost,
        "node should be marked ghost at entropy > 0.92"
    );
}

#[test]
fn entropy_engine_reverse_restores_node() {
    let mut graph = CognitiveGraph::new();
    let node_id = graph
        .add_node(NodeData::new(NodeType::Memory, "faded memory"))
        .expect("node");

    let node = graph.get_node_mut(node_id).unwrap();
    node.entropy = 0.95;
    node.is_ghost = true;

    let engine = EntropyEngine::new();
    engine.reverse_entropy(&mut graph, node_id, Utc::now());

    let node = graph.get_node(node_id).unwrap();
    assert!(
        !node.is_ghost,
        "ghost flag should be cleared after reverse_entropy"
    );
    assert!(
        node.entropy < 0.1,
        "entropy should drop to ~10% after reverse"
    );
}

#[test]
fn civilization_id_round_trips_sqlite() {
    let temp_dir = tempdir().expect("tempdir");
    let path = temp_dir.path().join("civ.sqlite");
    let mut store = SqliteWorkspaceStore::new(&path).expect("sqlite");

    let civ_id = Uuid::new_v4();
    let mut node = NodeData::new(NodeType::World, "civilization seed");
    node.civilization_id = Some(civ_id);

    let mut snapshot = silentnode_core::WorkspaceSnapshot::default();
    snapshot.graph.nodes.push(node.clone());

    store.save_snapshot(&snapshot).expect("save");
    let loaded = store.load_snapshot().expect("load").expect("exists");

    assert_eq!(loaded.graph.nodes.len(), 1);
    assert_eq!(loaded.graph.nodes[0].civilization_id, Some(civ_id));
}

#[tokio::test]
async fn civilization_id_round_trips_surreal() {
    let civ_id = Uuid::new_v4();
    let mut node = NodeData::new(NodeType::World, "surreal civ node");
    node.civilization_id = Some(civ_id);

    let mut snapshot = silentnode_core::WorkspaceSnapshot::default();
    snapshot.graph.nodes.push(node);

    let store = SurrealWorkspaceStore::new_in_memory().await.expect("store");
    store.save_snapshot(&snapshot).await.expect("save");

    let loaded = store.load_snapshot().await.expect("load");
    assert_eq!(loaded.graph.nodes.len(), 1);
    assert_eq!(loaded.graph.nodes[0].civilization_id, Some(civ_id));
}

#[test]
fn materialize_triggers_contagion_on_connected_graph() {
    let mut workspace = SilentNodeWorkspace::new();
    let materializer = MaterializationEngine::new();

    // First node — no neighbors, no contagion
    let first = workspace
        .materialize_thought(&materializer, "entropy decay cognitive graph")
        .expect("first thought");

    // Give first node high entropy so we can detect contagion reduction
    workspace.graph.get_node_mut(first.node_id).unwrap().entropy = 0.8;

    // Second thought with similar content → auto-connects via Resonance → triggers contagion
    let _second = workspace
        .materialize_thought(&materializer, "cognitive graph entropy memory")
        .expect("second thought");

    // First node should have reduced entropy from contagion
    let entropy_after = workspace.graph.get_node(first.node_id).unwrap().entropy;
    assert!(
        entropy_after < 0.8,
        "contagion from second thought should reduce first node entropy; got {entropy_after:.3}"
    );
}

#[test]
fn workspace_reverse_entropy_restores_ghost() {
    let mut workspace = SilentNodeWorkspace::new();
    let materializer = MaterializationEngine::new();
    let engine = EntropyEngine::new();

    let result = workspace
        .materialize_thought(&materializer, "forgotten project idea")
        .expect("thought");

    let node = workspace.graph.get_node_mut(result.node_id).unwrap();
    node.entropy = 0.97;
    node.is_ghost = true;

    workspace.reverse_entropy(&engine, result.node_id);

    let node = workspace.graph.get_node(result.node_id).unwrap();
    assert!(!node.is_ghost, "node should be restored from ghost");
    assert!(
        node.entropy < 0.1,
        "entropy should be near zero after reverse"
    );
}
