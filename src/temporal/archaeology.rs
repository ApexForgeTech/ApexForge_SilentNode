// Archaeology — cursor-driven descent into a node's temporal record.

use super::{parse_node_type, TemporalEngine};
use crate::domain::{ChangeType, NodeData, Position3, TemporalSnapshot};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use uuid::Uuid;

/// A user-placed marker at a point in a node's history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalMarker {
    pub id: Uuid,
    pub node_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub label: String,
    pub snapshot_index: usize,
}

/// Field-level diff between two temporal states.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDiff {
    pub field: String,
    pub before: Value,
    pub after: Value,
}

/// Diff result between two point-in-time states of a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalDiff {
    pub node_id: Uuid,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub changes: Vec<FieldDiff>,
    pub entropy_delta: f32,
    pub access_delta: i64,
}

impl TemporalDiff {
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    pub fn change_count(&self) -> usize {
        self.changes.len()
    }
}

/// Interactive session for exploring the temporal record of a single node.
///
/// The session maintains a cursor into the snapshot list. Calling `descend()`
/// moves the cursor backward (toward the past). `resurrect()` returns a
/// reconstructed `NodeData` at the cursor position.
pub struct ArchaeologySession {
    node_id: Uuid,
    /// Chronologically ordered snapshots for this node.
    history: Vec<TemporalSnapshot>,
    /// Current cursor position (index into `history`), starting at the latest.
    cursor: usize,
    /// User-placed temporal markers.
    markers: Vec<TemporalMarker>,
}

impl ArchaeologySession {
    /// Open an archaeology session for `node_id` using records from `engine`.
    /// Returns `None` when the node has no recorded history.
    pub fn open(engine: &TemporalEngine, node_id: Uuid) -> Option<Self> {
        let history: Vec<TemporalSnapshot> =
            engine.snapshots_for(node_id).into_iter().cloned().collect();
        if history.is_empty() {
            return None;
        }
        let cursor = history.len().saturating_sub(1);
        Some(Self {
            node_id,
            history,
            cursor,
            markers: Vec::new(),
        })
    }

    /// Number of recorded snapshots (depth of history).
    pub fn depth(&self) -> usize {
        self.history.len()
    }

    /// Current cursor position (0 = oldest, depth-1 = newest).
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Timestamp of the snapshot at the current cursor.
    pub fn current_timestamp(&self) -> DateTime<Utc> {
        self.history[self.cursor].timestamp
    }

    /// Move cursor `steps` snapshots into the past. Clamps at the oldest entry.
    /// Returns how many steps were actually taken.
    pub fn descend(&mut self, steps: usize) -> usize {
        let actual = steps.min(self.cursor);
        self.cursor -= actual;
        actual
    }

    /// Move cursor `steps` snapshots toward the present. Clamps at the newest.
    /// Returns how many steps were actually taken.
    pub fn ascend(&mut self, steps: usize) -> usize {
        let actual = steps.min(self.history.len() - 1 - self.cursor);
        self.cursor += actual;
        actual
    }

    /// Jump cursor to a specific index.
    pub fn seek(&mut self, index: usize) {
        self.cursor = index.min(self.history.len() - 1);
    }

    /// Reconstruct a `NodeData` from the snapshot at the current cursor position.
    pub fn resurrect(&self) -> NodeData {
        snapshot_to_node(&self.history[self.cursor].snapshot, self.node_id)
    }

    /// Reconstruct a `NodeData` at an explicit `DateTime`. Returns the closest
    /// snapshot that is ≤ the requested time.
    pub fn resurrect_at(&self, at: DateTime<Utc>) -> Option<NodeData> {
        let snap = self.history.iter().filter(|s| s.timestamp <= at).last()?;
        Some(snapshot_to_node(&snap.snapshot, self.node_id))
    }

    /// Produce a diff between snapshot at cursor and snapshot at
    /// `cursor + steps_forward`. Returns `None` when there is no future snapshot
    /// that many steps ahead.
    pub fn compare_forward(&self, steps_forward: usize) -> Option<TemporalDiff> {
        let later_idx = self.cursor.checked_add(steps_forward)?;
        if later_idx >= self.history.len() {
            return None;
        }
        Some(diff_snapshots(
            self.node_id,
            &self.history[self.cursor],
            &self.history[later_idx],
        ))
    }

    /// Produce a diff between two absolute timestamps.
    pub fn compare(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> Option<TemporalDiff> {
        let snap_from = self.history.iter().filter(|s| s.timestamp <= from).last()?;
        let snap_to = self.history.iter().filter(|s| s.timestamp <= to).last()?;
        if snap_from.id == snap_to.id {
            return Some(TemporalDiff {
                node_id: self.node_id,
                from,
                to,
                changes: vec![],
                entropy_delta: 0.0,
                access_delta: 0,
            });
        }
        Some(diff_snapshots(self.node_id, snap_from, snap_to))
    }

    /// Place a named temporal marker at the current cursor position.
    pub fn mark(&mut self, label: impl Into<String>) -> &TemporalMarker {
        let marker = TemporalMarker {
            id: Uuid::new_v4(),
            node_id: self.node_id,
            timestamp: self.history[self.cursor].timestamp,
            label: label.into(),
            snapshot_index: self.cursor,
        };
        self.markers.push(marker);
        self.markers.last().unwrap()
    }

    pub fn markers(&self) -> &[TemporalMarker] {
        &self.markers
    }

    /// Change type of the snapshot at the current cursor.
    pub fn current_change_type(&self) -> ChangeType {
        self.history[self.cursor].change_type
    }

    /// Summary of every snapshot: (index, timestamp, change_type).
    pub fn timeline(&self) -> Vec<(usize, DateTime<Utc>, ChangeType)> {
        self.history
            .iter()
            .enumerate()
            .map(|(i, s)| (i, s.timestamp, s.change_type))
            .collect()
    }
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn get_f32(map: &BTreeMap<String, Value>, key: &str) -> f32 {
    map.get(key)
        .and_then(Value::as_f64)
        .map(|v| v as f32)
        .unwrap_or(0.0)
}

fn get_u64(map: &BTreeMap<String, Value>, key: &str) -> u64 {
    map.get(key).and_then(Value::as_u64).unwrap_or(0)
}

fn get_bool(map: &BTreeMap<String, Value>, key: &str) -> bool {
    map.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn get_str<'a>(map: &'a BTreeMap<String, Value>, key: &str) -> &'a str {
    map.get(key).and_then(Value::as_str).unwrap_or("")
}

pub(crate) fn snapshot_to_node(map: &BTreeMap<String, Value>, node_id: Uuid) -> NodeData {
    let node_type = parse_node_type(get_str(map, "node_type"));
    let content = get_str(map, "content").to_string();
    let entropy = get_f32(map, "entropy");
    let gravity = get_f32(map, "gravity");
    let velocity = get_f32(map, "velocity");
    let access_count = get_u64(map, "access_count");
    let is_ghost = get_bool(map, "is_ghost");
    let is_fossil = get_bool(map, "is_fossil");
    let is_void = get_bool(map, "is_void");
    let aura_color = get_str(map, "aura_color").to_string();
    let created_at = map
        .get("created_at")
        .and_then(Value::as_str)
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    let accessed_at = map
        .get("accessed_at")
        .and_then(Value::as_str)
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    let pos_x = get_f32(map, "pos_x");
    let pos_y = get_f32(map, "pos_y");
    let pos_z = get_f32(map, "pos_z");

    NodeData {
        id: node_id,
        node_type,
        content,
        metadata: BTreeMap::new(),
        entropy,
        gravity,
        velocity,
        access_count,
        created_at,
        accessed_at,
        is_ghost,
        is_fossil,
        is_void,
        position: Position3 {
            x: pos_x,
            y: pos_y,
            z: pos_z,
        },
        aura_color: if aura_color.is_empty() {
            "#7dd3fc".to_string()
        } else {
            aura_color
        },
        soul_signature: BTreeMap::new(),
        civilization_id: None,
    }
}

fn diff_snapshots(
    node_id: Uuid,
    before: &TemporalSnapshot,
    after: &TemporalSnapshot,
) -> TemporalDiff {
    let mut changes = Vec::new();
    let all_keys: std::collections::BTreeSet<&String> = before
        .snapshot
        .keys()
        .chain(after.snapshot.keys())
        .collect();

    for key in all_keys {
        let bval = before
            .snapshot
            .get(key.as_str())
            .cloned()
            .unwrap_or(Value::Null);
        let aval = after
            .snapshot
            .get(key.as_str())
            .cloned()
            .unwrap_or(Value::Null);
        if bval != aval {
            changes.push(FieldDiff {
                field: key.clone(),
                before: bval,
                after: aval,
            });
        }
    }

    let entropy_before = before
        .snapshot
        .get("entropy")
        .and_then(Value::as_f64)
        .unwrap_or(0.0) as f32;
    let entropy_after = after
        .snapshot
        .get("entropy")
        .and_then(Value::as_f64)
        .unwrap_or(0.0) as f32;
    let access_before = before
        .snapshot
        .get("access_count")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let access_after = after
        .snapshot
        .get("access_count")
        .and_then(Value::as_i64)
        .unwrap_or(0);

    TemporalDiff {
        node_id,
        from: before.timestamp,
        to: after.timestamp,
        changes,
        entropy_delta: entropy_after - entropy_before,
        access_delta: access_after - access_before,
    }
}
