use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    Idea,
    Memory,
    Project,
    Person,
    Artifact,
    Media,
    Process,
    World,
    Ghost,
    Fossil,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    Connection,
    Resonance,
    Temporal,
    Causal,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Position3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeData {
    pub id: Uuid,
    pub node_type: NodeType,
    pub content: String,
    pub metadata: BTreeMap<String, Value>,
    pub entropy: f32,
    pub gravity: f32,
    pub velocity: f32,
    pub access_count: u64,
    pub created_at: DateTime<Utc>,
    pub accessed_at: DateTime<Utc>,
    pub is_ghost: bool,
    pub is_fossil: bool,
    pub is_void: bool,
    pub position: Position3,
    pub aura_color: String,
    pub soul_signature: BTreeMap<String, Value>,
    pub civilization_id: Option<Uuid>,
}

impl NodeData {
    pub fn new(node_type: NodeType, content: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            node_type,
            content: content.into(),
            metadata: BTreeMap::new(),
            entropy: 0.0,
            gravity: 1.0,
            velocity: 0.0,
            access_count: 0,
            created_at: now,
            accessed_at: now,
            is_ghost: false,
            is_fossil: false,
            is_void: false,
            position: Position3::default(),
            aura_color: "#7dd3fc".to_string(),
            soul_signature: BTreeMap::new(),
            civilization_id: None,
        }
    }

    pub fn touch(&mut self, at: DateTime<Utc>) {
        self.accessed_at = at;
        self.entropy *= 0.1;
        self.access_count += 1;
        if self.is_ghost {
            self.is_ghost = false;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeData {
    pub source_id: Uuid,
    pub target_id: Uuid,
    pub weight: f32,
    pub edge_type: EdgeType,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
}

impl EdgeData {
    pub fn new(source_id: Uuid, target_id: Uuid, edge_type: EdgeType, weight: f32) -> Self {
        let now = Utc::now();
        Self {
            source_id,
            target_id,
            weight,
            edge_type,
            created_at: now,
            last_active: now,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub ghost_count: usize,
    pub fossil_count: usize,
    pub void_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FocusDepth {
    Glance,
    Read,
    Edit,
    DeepWork,
}

impl FocusDepth {
    pub fn weight(self) -> f32 {
        match self {
            Self::Glance => 0.25,
            Self::Read => 0.5,
            Self::Edit => 0.8,
            Self::DeepWork => 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FocusEvent {
    pub node_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub duration_seconds: f32,
    pub depth: FocusDepth,
    pub session_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalEntry {
    pub id: Uuid,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub linked_nodes: Vec<Uuid>,
    pub mood_signature: BTreeMap<String, Value>,
    pub season: Option<String>,
}

impl JournalEntry {
    pub fn new(
        content: impl Into<String>,
        linked_nodes: Vec<Uuid>,
        season: Option<String>,
        mood_signature: BTreeMap<String, Value>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            content: content.into(),
            timestamp: Utc::now(),
            linked_nodes,
            mood_signature,
            season,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    Created,
    Modified,
    Accessed,
    StateChanged,
    Connected,
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemporalSnapshot {
    pub id: Uuid,
    pub node_id: Uuid,
    pub snapshot: BTreeMap<String, Value>,
    pub timestamp: DateTime<Utc>,
    pub change_type: ChangeType,
}

impl TemporalSnapshot {
    pub fn new(node_id: Uuid, snapshot: BTreeMap<String, Value>, change_type: ChangeType) -> Self {
        Self {
            id: Uuid::new_v4(),
            node_id,
            snapshot,
            timestamp: Utc::now(),
            change_type,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArcType {
    Origin,
    Conflict,
    Resolution,
    Revelation,
    Transformation,
    Legacy,
    /// A structural tectonic shift in the cognitive universe.
    Tectonic,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoreEntry {
    pub id: Uuid,
    pub title: String,
    pub arc_type: ArcType,
    pub timestamp: DateTime<Utc>,
    pub linked_nodes: Vec<Uuid>,
    pub narrative: String,
    pub significance: f32,
}

impl LoreEntry {
    pub fn new(
        title: impl Into<String>,
        arc_type: ArcType,
        narrative: impl Into<String>,
        linked_nodes: Vec<Uuid>,
        significance: f32,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            arc_type,
            timestamp: Utc::now(),
            linked_nodes,
            narrative: narrative.into(),
            significance,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractState {
    Dormant,
    Awakening,
    Active,
    Resolved,
    Broken,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SilentContract {
    pub id: Uuid,
    pub related_node: Uuid,
    pub detected_at: DateTime<Utc>,
    pub intensity: f32,
    pub age_days: f32,
    pub state: ContractState,
}

impl SilentContract {
    pub fn new(related_node: Uuid, intensity: f32, age_days: f32) -> Self {
        Self {
            id: Uuid::new_v4(),
            related_node,
            detected_at: Utc::now(),
            intensity,
            age_days,
            state: ContractState::Dormant,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessRecord {
    pub id: Uuid,
    pub pid: i64,
    pub name: String,
    pub linked_node: Uuid,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub cpu_usage: f32,
    pub memory_mb: f32,
}

impl ProcessRecord {
    pub fn new(pid: i64, name: impl Into<String>, linked_node: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            pid,
            name: name.into(),
            linked_node,
            started_at: Utc::now(),
            ended_at: None,
            cpu_usage: 0.0,
            memory_mb: 0.0,
        }
    }
}
