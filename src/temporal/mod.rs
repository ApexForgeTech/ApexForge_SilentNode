// Phase 5 — Temporal Systems
//
// Modules:
//   • TemporalEngine      — snapshot store, node/universe reconstruction
//   • ArchaeologySession  — cursor-driven descent into a node's past
//   • MemoryReconstructor — full-day cognitive state reconstruction
//   • FossilEngine        — fossilization lifecycle
//   • LoreArcDetector     — narrative arc detection and lore generation

pub mod archaeology;
pub mod fossils;
pub mod lore;
pub mod reconstruction;

pub use archaeology::{ArchaeologySession, TemporalDiff, TemporalMarker};
pub use fossils::{FossilEngine, FossilizationCheck};
pub use lore::LoreArcDetector;
pub use reconstruction::{DayComparison, DayReconstruction, MemoryReconstructor};

use crate::domain::{ChangeType, NodeData, NodeType, TemporalSnapshot};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

/// Central temporal record-keeper. Stores all node change snapshots and
/// answers point-in-time reconstruction queries.
#[derive(Debug, Clone, Default)]
pub struct TemporalEngine {
    /// All snapshots ever recorded, in insertion order.
    snapshots: Vec<TemporalSnapshot>,
}

impl TemporalEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed from previously persisted snapshots (e.g. loaded from SQLite).
    pub fn from_snapshots(snapshots: Vec<TemporalSnapshot>) -> Self {
        Self { snapshots }
    }

    /// Record a change event for a node, capturing its full serialised state.
    pub fn record_change(&mut self, node: &NodeData, change_type: ChangeType) {
        let snapshot_map = node_to_map(node);
        self.snapshots
            .push(TemporalSnapshot::new(node.id, snapshot_map, change_type));
    }

    /// All snapshots for one node, chronologically ordered.
    pub fn snapshots_for(&self, node_id: Uuid) -> Vec<&TemporalSnapshot> {
        let mut v: Vec<&TemporalSnapshot> = self
            .snapshots
            .iter()
            .filter(|s| s.node_id == node_id)
            .collect();
        v.sort_by_key(|s| s.timestamp);
        v
    }

    /// Reconstruct the field map of a node as it was at `at`. Returns `None`
    /// if the node didn't exist yet (no snapshot before `at`).
    pub fn reconstruct_node_at(
        &self,
        node_id: Uuid,
        at: DateTime<Utc>,
    ) -> Option<BTreeMap<String, Value>> {
        self.snapshots_for(node_id)
            .into_iter()
            .filter(|s| s.timestamp <= at)
            .last()
            .map(|s| s.snapshot.clone())
    }

    /// Reconstruct the full universe (all nodes that had ≥1 snapshot before
    /// `at`) as their last-known state up to `at`.
    pub fn reconstruct_universe_at(
        &self,
        at: DateTime<Utc>,
    ) -> HashMap<Uuid, BTreeMap<String, Value>> {
        let mut universe: HashMap<Uuid, (DateTime<Utc>, &TemporalSnapshot)> = HashMap::new();
        for s in self.snapshots.iter().filter(|s| s.timestamp <= at) {
            universe
                .entry(s.node_id)
                .and_modify(|(ts, prev)| {
                    if s.timestamp > *ts {
                        *ts = s.timestamp;
                        *prev = s;
                    }
                })
                .or_insert((s.timestamp, s));
        }
        universe
            .into_values()
            .map(|(_, s)| (s.node_id, s.snapshot.clone()))
            .collect()
    }

    /// Total number of recorded snapshots.
    pub fn snapshot_count(&self) -> usize {
        self.snapshots.len()
    }

    /// Borrow all raw snapshots (e.g. for persistence).
    pub fn all_snapshots(&self) -> &[TemporalSnapshot] {
        &self.snapshots
    }

    /// How many distinct nodes have been recorded.
    pub fn tracked_node_count(&self) -> usize {
        self.snapshots
            .iter()
            .map(|s| s.node_id)
            .collect::<std::collections::HashSet<_>>()
            .len()
    }
}

// ── Serialisation helpers ────────────────────────────────────────────────────

pub(crate) fn node_to_map(node: &NodeData) -> BTreeMap<String, Value> {
    let mut m = BTreeMap::new();
    m.insert("id".into(), Value::String(node.id.to_string()));
    m.insert(
        "node_type".into(),
        Value::String(format!("{:?}", node.node_type)),
    );
    m.insert("content".into(), Value::String(node.content.clone()));
    m.insert(
        "entropy".into(),
        Value::Number(
            serde_json::Number::from_f64(node.entropy as f64)
                .unwrap_or(serde_json::Number::from(0)),
        ),
    );
    m.insert(
        "gravity".into(),
        Value::Number(
            serde_json::Number::from_f64(node.gravity as f64)
                .unwrap_or(serde_json::Number::from(1)),
        ),
    );
    m.insert(
        "velocity".into(),
        Value::Number(
            serde_json::Number::from_f64(node.velocity as f64)
                .unwrap_or(serde_json::Number::from(0)),
        ),
    );
    m.insert(
        "access_count".into(),
        Value::Number(serde_json::Number::from(node.access_count)),
    );
    m.insert("is_ghost".into(), Value::Bool(node.is_ghost));
    m.insert("is_fossil".into(), Value::Bool(node.is_fossil));
    m.insert("is_void".into(), Value::Bool(node.is_void));
    m.insert("aura_color".into(), Value::String(node.aura_color.clone()));
    m.insert(
        "created_at".into(),
        Value::String(node.created_at.to_rfc3339()),
    );
    m.insert(
        "accessed_at".into(),
        Value::String(node.accessed_at.to_rfc3339()),
    );
    m.insert(
        "pos_x".into(),
        Value::Number(
            serde_json::Number::from_f64(node.position.x as f64)
                .unwrap_or(serde_json::Number::from(0)),
        ),
    );
    m.insert(
        "pos_y".into(),
        Value::Number(
            serde_json::Number::from_f64(node.position.y as f64)
                .unwrap_or(serde_json::Number::from(0)),
        ),
    );
    m.insert(
        "pos_z".into(),
        Value::Number(
            serde_json::Number::from_f64(node.position.z as f64)
                .unwrap_or(serde_json::Number::from(0)),
        ),
    );
    m
}

/// Attempt to parse a NodeType from the snapshot string form produced by `node_to_map`.
pub(crate) fn parse_node_type(s: &str) -> NodeType {
    match s {
        "Idea" => NodeType::Idea,
        "Memory" => NodeType::Memory,
        "Project" => NodeType::Project,
        "Person" => NodeType::Person,
        "Artifact" => NodeType::Artifact,
        "Media" => NodeType::Media,
        "Process" => NodeType::Process,
        "World" => NodeType::World,
        "Ghost" => NodeType::Ghost,
        "Fossil" => NodeType::Fossil,
        _ => NodeType::Idea,
    }
}
