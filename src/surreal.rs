use crate::domain::{
    ArcType, ChangeType, ContractState, EdgeData, EdgeType, FocusDepth, FocusEvent, JournalEntry,
    LoreEntry, NodeData, NodeType, Position3, ProcessRecord, SilentContract, TemporalSnapshot,
};
use crate::error::SurrealStoreError;
use crate::storage::{StoredGraph, WorkspaceSnapshot};
use crate::StorageError;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;
use surrealdb::engine::local::{Db, Mem, SurrealKv};
use surrealdb::{RecordId, Surreal};
use uuid::Uuid;

pub const SURREAL_SCHEMA: &str = include_str!("../schema/silentnode.surql");

#[derive(Debug)]
pub struct SurrealWorkspaceStore {
    db: Surreal<Db>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurrealTableCounts {
    pub nodes: usize,
    pub edges: usize,
    pub focus_events: usize,
    pub journal_entries: usize,
    pub temporal_snapshots: usize,
    pub lore_entries: usize,
    pub silent_contracts: usize,
    pub process_records: usize,
}

impl SurrealWorkspaceStore {
    pub async fn new_in_memory() -> Result<Self, SurrealStoreError> {
        let db = Surreal::new::<Mem>(()).await?;
        db.use_ns("silentnode").use_db("main").await?;
        let store = Self { db };
        store.apply_schema().await?;
        Ok(store)
    }

    pub async fn new_local(path: impl AsRef<Path>) -> Result<Self, SurrealStoreError> {
        let db = Surreal::new::<SurrealKv>(path.as_ref()).await?;
        db.use_ns("silentnode").use_db("main").await?;
        let store = Self { db };
        store.apply_schema().await?;
        Ok(store)
    }

    pub async fn apply_schema(&self) -> Result<(), SurrealStoreError> {
        self.db.query(SURREAL_SCHEMA).await?.check()?;
        Ok(())
    }

    pub async fn healthcheck(&self) -> Result<SurrealTableCounts, SurrealStoreError> {
        #[derive(Debug, Deserialize)]
        struct CountRow {
            count: usize,
        }

        let mut response = self
            .db
            .query(
                r#"
                RETURN (SELECT count() AS count FROM node GROUP ALL);
                RETURN (SELECT count() AS count FROM connects GROUP ALL);
                RETURN (SELECT count() AS count FROM focus_event GROUP ALL);
                RETURN (SELECT count() AS count FROM journal_entry GROUP ALL);
                RETURN (SELECT count() AS count FROM temporal_snapshot GROUP ALL);
                RETURN (SELECT count() AS count FROM lore_entry GROUP ALL);
                RETURN (SELECT count() AS count FROM silent_contract GROUP ALL);
                RETURN (SELECT count() AS count FROM process_record GROUP ALL);
                "#,
            )
            .await?;
        let nodes: Vec<CountRow> = response.take(0)?;
        let edges: Vec<CountRow> = response.take(1)?;
        let focus_events: Vec<CountRow> = response.take(2)?;
        let journal_entries: Vec<CountRow> = response.take(3)?;
        let temporal_snapshots: Vec<CountRow> = response.take(4)?;
        let lore_entries: Vec<CountRow> = response.take(5)?;
        let silent_contracts: Vec<CountRow> = response.take(6)?;
        let process_records: Vec<CountRow> = response.take(7)?;

        Ok(SurrealTableCounts {
            nodes: nodes.first().map(|row| row.count).unwrap_or(0),
            edges: edges.first().map(|row| row.count).unwrap_or(0),
            focus_events: focus_events.first().map(|row| row.count).unwrap_or(0),
            journal_entries: journal_entries.first().map(|row| row.count).unwrap_or(0),
            temporal_snapshots: temporal_snapshots.first().map(|row| row.count).unwrap_or(0),
            lore_entries: lore_entries.first().map(|row| row.count).unwrap_or(0),
            silent_contracts: silent_contracts.first().map(|row| row.count).unwrap_or(0),
            process_records: process_records.first().map(|row| row.count).unwrap_or(0),
        })
    }

    pub async fn save_snapshot(
        &self,
        snapshot: &WorkspaceSnapshot,
    ) -> Result<(), SurrealStoreError> {
        self.db
            .query("DELETE node; DELETE connects; DELETE focus_event; DELETE journal_entry; DELETE temporal_snapshot; DELETE lore_entry; DELETE silent_contract; DELETE process_record;")
            .await?
            .check()?;

        for node in &snapshot.graph.nodes {
            self.db
                .query(
                    r#"
                    LET $node = type::thing("node", $id);
                    UPSERT $node CONTENT {
                        id: $node,
                        type: $node_type,
                        content: $content,
                        metadata: $metadata,
                        entropy: $entropy,
                        gravity: $gravity,
                        velocity: $velocity,
                        access_count: $access_count,
                        created_at: <datetime>$created_at,
                        accessed_at: <datetime>$accessed_at,
                        is_ghost: $is_ghost,
                        is_fossil: $is_fossil,
                        is_void: $is_void,
                        position: $position,
                        aura_color: $aura_color,
                        soul_signature: $soul_signature,
                        civilization_id: <option<uuid>>$civilization_id
                    };
                    "#,
                )
                .bind(("id", node.id.to_string()))
                .bind(("node_type", serde_json::to_value(node.node_type)?))
                .bind(("content", node.content.clone()))
                .bind(("metadata", serde_json::to_value(&node.metadata)?))
                .bind(("entropy", node.entropy))
                .bind(("gravity", node.gravity))
                .bind(("velocity", node.velocity))
                .bind(("access_count", node.access_count as i64))
                .bind(("created_at", node.created_at))
                .bind(("accessed_at", node.accessed_at))
                .bind(("is_ghost", node.is_ghost))
                .bind(("is_fossil", node.is_fossil))
                .bind(("is_void", node.is_void))
                .bind(("position", serde_json::to_value(node.position)?))
                .bind(("aura_color", node.aura_color.clone()))
                .bind((
                    "soul_signature",
                    serde_json::to_value(&node.soul_signature)?,
                ))
                .bind((
                    "civilization_id",
                    node.civilization_id.map(|id| id.to_string()),
                ))
                .await?
                .check()?;
        }

        for edge in &snapshot.graph.edges {
            self.db
                .query(
                    r#"
                    LET $source = type::thing("node", $source_id);
                    LET $target = type::thing("node", $target_id);
                    RELATE $source->connects->$target CONTENT {
                        source_id: $source_id,
                        target_id: $target_id,
                        weight: $weight,
                        type: $edge_type,
                        created_at: <datetime>$created_at,
                        last_active: <datetime>$last_active
                    };
                    "#,
                )
                .bind(("source_id", edge.source_id.to_string()))
                .bind(("target_id", edge.target_id.to_string()))
                .bind(("weight", edge.weight))
                .bind(("edge_type", serde_json::to_value(edge.edge_type)?))
                .bind(("created_at", edge.created_at))
                .bind(("last_active", edge.last_active))
                .await?
                .check()?;
        }

        for event in &snapshot.focus_events {
            self.db
                .query(
                    r#"
                    LET $node = type::thing("node", $node_id);
                    CREATE focus_event CONTENT {
                        node_id: $node,
                        timestamp: <datetime>$timestamp,
                        duration: $duration,
                        depth: $depth,
                        session_id: <uuid>$session_id
                    };
                    "#,
                )
                .bind(("node_id", event.node_id.to_string()))
                .bind(("timestamp", event.timestamp))
                .bind(("duration", event.duration_seconds))
                .bind(("depth", serde_json::to_value(event.depth)?))
                .bind(("session_id", event.session_id.to_string()))
                .await?
                .check()?;
        }

        for entry in &snapshot.journal_entries {
            let linked_nodes = entry
                .linked_nodes
                .iter()
                .map(|node_id| RecordId::from(("node", *node_id)))
                .collect::<Vec<_>>();
            self.db
                .query(
                    r#"
                    CREATE type::thing("journal_entry", $id) CONTENT {
                        content: $content,
                        timestamp: <datetime>$timestamp,
                        linked_nodes: $linked_nodes,
                        mood_signature: $mood_signature,
                        season: $season
                    };
                    "#,
                )
                .bind(("id", entry.id.to_string()))
                .bind(("content", entry.content.clone()))
                .bind(("timestamp", entry.timestamp))
                .bind(("linked_nodes", linked_nodes))
                .bind((
                    "mood_signature",
                    serde_json::to_value(&entry.mood_signature)?,
                ))
                .bind(("season", entry.season.clone()))
                .await?
                .check()?;
        }

        for ts in &snapshot.temporal_snapshots {
            let node_ref = RecordId::from(("node", ts.node_id));
            self.db
                .query(
                    r#"
                    CREATE type::thing("temporal_snapshot", $id) CONTENT {
                        node_id: $node_id,
                        snapshot: $snapshot,
                        timestamp: <datetime>$timestamp,
                        change_type: $change_type
                    };
                    "#,
                )
                .bind(("id", ts.id.to_string()))
                .bind(("node_id", node_ref))
                .bind(("snapshot", serde_json::to_value(&ts.snapshot)?))
                .bind(("timestamp", ts.timestamp))
                .bind(("change_type", change_type_to_str(ts.change_type)))
                .await?
                .check()?;
        }

        for entry in &snapshot.lore_entries {
            let linked_nodes = entry
                .linked_nodes
                .iter()
                .map(|node_id| RecordId::from(("node", *node_id)))
                .collect::<Vec<_>>();
            self.db
                .query(
                    r#"
                    CREATE type::thing("lore_entry", $id) CONTENT {
                        title: $title,
                        arc_type: $arc_type,
                        timestamp: <datetime>$timestamp,
                        linked_nodes: $linked_nodes,
                        narrative: $narrative,
                        significance: $significance
                    };
                    "#,
                )
                .bind(("id", entry.id.to_string()))
                .bind(("title", entry.title.clone()))
                .bind(("arc_type", arc_type_to_str(entry.arc_type)))
                .bind(("timestamp", entry.timestamp))
                .bind(("linked_nodes", linked_nodes))
                .bind(("narrative", entry.narrative.clone()))
                .bind(("significance", entry.significance))
                .await?
                .check()?;
        }

        for contract in &snapshot.silent_contracts {
            let node_ref = RecordId::from(("node", contract.related_node));
            self.db
                .query(
                    r#"
                    CREATE type::thing("silent_contract", $id) CONTENT {
                        related_node: $related_node,
                        detected_at: <datetime>$detected_at,
                        intensity: $intensity,
                        age_days: $age_days,
                        state: $state
                    };
                    "#,
                )
                .bind(("id", contract.id.to_string()))
                .bind(("related_node", node_ref))
                .bind(("detected_at", contract.detected_at))
                .bind(("intensity", contract.intensity))
                .bind(("age_days", contract.age_days))
                .bind(("state", contract_state_to_str(contract.state)))
                .await?
                .check()?;
        }

        for record in &snapshot.process_records {
            let node_ref = RecordId::from(("node", record.linked_node));
            self.db
                .query(
                    r#"
                    CREATE type::thing("process_record", $id) CONTENT {
                        pid: $pid,
                        name: $name,
                        linked_node: $linked_node,
                        started_at: <datetime>$started_at,
                        ended_at: $ended_at,
                        cpu_usage: $cpu_usage,
                        memory_mb: $memory_mb
                    };
                    "#,
                )
                .bind(("id", record.id.to_string()))
                .bind(("pid", record.pid))
                .bind(("name", record.name.clone()))
                .bind(("linked_node", node_ref))
                .bind(("started_at", record.started_at))
                .bind(("ended_at", record.ended_at))
                .bind(("cpu_usage", record.cpu_usage))
                .bind(("memory_mb", record.memory_mb))
                .await?
                .check()?;
        }

        Ok(())
    }

    pub async fn load_snapshot(&self) -> Result<WorkspaceSnapshot, SurrealStoreError> {
        let nodes = self.select_nodes().await?;
        let edges = self.select_edges().await?;
        let focus_events = self.select_focus_events().await?;
        let journal_entries = self.select_journal_entries().await?;
        let temporal_snapshots = self.select_temporal_snapshots().await?;
        let lore_entries = self.select_lore_entries().await?;
        let silent_contracts = self.select_silent_contracts().await?;
        let process_records = self.select_process_records().await?;

        Ok(WorkspaceSnapshot {
            graph: StoredGraph { nodes, edges },
            focus_events,
            journal_entries,
            system_mode: None,
            temporal_snapshots,
            lore_entries,
            silent_contracts,
            process_records,
            calendar_events: Vec::new(),
        })
    }

    async fn select_nodes(&self) -> Result<Vec<NodeData>, SurrealStoreError> {
        #[derive(Debug, Deserialize)]
        struct NodeRecord {
            id: String,
            node_type: NodeType,
            content: String,
            metadata: BTreeMap<String, Value>,
            entropy: f32,
            gravity: f32,
            velocity: f32,
            access_count: u64,
            created_at: DateTime<Utc>,
            accessed_at: DateTime<Utc>,
            is_ghost: bool,
            is_fossil: bool,
            is_void: bool,
            pos_x: Option<f32>,
            pos_y: Option<f32>,
            pos_z: Option<f32>,
            aura_color: String,
            soul_signature: BTreeMap<String, Value>,
            civilization_id: Option<Uuid>,
        }

        let mut response = self
            .db
            .query(
                r#"
                SELECT
                    id.id() AS id,
                    type AS node_type,
                    content,
                    metadata,
                    entropy,
                    gravity,
                    velocity,
                    access_count,
                    created_at,
                    accessed_at,
                    is_ghost,
                    is_fossil,
                    is_void,
                    position.x AS pos_x,
                    position.y AS pos_y,
                    position.z AS pos_z,
                    aura_color,
                    soul_signature,
                    civilization_id
                FROM node;
                "#,
            )
            .await?;
        let records: Vec<NodeRecord> = response.take(0)?;
        records
            .into_iter()
            .map(|record| {
                Ok(NodeData {
                    id: parse_uuid(&record.id)?,
                    node_type: record.node_type,
                    content: record.content,
                    metadata: record.metadata,
                    entropy: record.entropy,
                    gravity: record.gravity,
                    velocity: record.velocity,
                    access_count: record.access_count,
                    created_at: record.created_at,
                    accessed_at: record.accessed_at,
                    is_ghost: record.is_ghost,
                    is_fossil: record.is_fossil,
                    is_void: record.is_void,
                    position: Position3 {
                        x: record.pos_x.unwrap_or_default(),
                        y: record.pos_y.unwrap_or_default(),
                        z: record.pos_z.unwrap_or_default(),
                    },
                    aura_color: record.aura_color,
                    soul_signature: record.soul_signature,
                    civilization_id: record.civilization_id,
                })
            })
            .collect()
    }

    async fn select_edges(&self) -> Result<Vec<EdgeData>, SurrealStoreError> {
        #[derive(Debug, Deserialize)]
        struct EdgeRecord {
            source_id: String,
            target_id: String,
            weight: f32,
            edge_type: EdgeType,
            created_at: DateTime<Utc>,
            last_active: DateTime<Utc>,
        }

        let mut response = self
            .db
            .query(
                r#"
                SELECT
                    source_id,
                    target_id,
                    weight,
                    type AS edge_type,
                    created_at,
                    last_active
                FROM connects;
                "#,
            )
            .await?;
        let records: Vec<EdgeRecord> = response.take(0)?;
        records
            .into_iter()
            .map(|record| {
                Ok(EdgeData {
                    source_id: parse_uuid(&record.source_id)?,
                    target_id: parse_uuid(&record.target_id)?,
                    weight: record.weight,
                    edge_type: record.edge_type,
                    created_at: record.created_at,
                    last_active: record.last_active,
                })
            })
            .collect()
    }

    async fn select_focus_events(&self) -> Result<Vec<FocusEvent>, SurrealStoreError> {
        #[derive(Debug, Deserialize)]
        struct FocusRecord {
            node_id: String,
            timestamp: DateTime<Utc>,
            duration_seconds: f32,
            depth: FocusDepth,
            session_id: String,
        }

        let mut response = self
            .db
            .query(
                r#"
                SELECT
                    node_id.id() AS node_id,
                    timestamp,
                    duration AS duration_seconds,
                    depth,
                    <string>session_id AS session_id
                FROM focus_event
                ORDER BY timestamp ASC;
                "#,
            )
            .await?;
        let records: Vec<FocusRecord> = response.take(0)?;
        records
            .into_iter()
            .map(|record| {
                Ok(FocusEvent {
                    node_id: parse_uuid(&record.node_id)?,
                    timestamp: record.timestamp,
                    duration_seconds: record.duration_seconds,
                    depth: record.depth,
                    session_id: parse_uuid(&record.session_id)?,
                })
            })
            .collect()
    }

    async fn select_journal_entries(&self) -> Result<Vec<JournalEntry>, SurrealStoreError> {
        #[derive(Debug, Deserialize)]
        struct JournalRecord {
            id: String,
            content: String,
            timestamp: DateTime<Utc>,
            linked_nodes: Vec<Uuid>,
            mood_signature: BTreeMap<String, Value>,
            season: Option<String>,
        }

        let mut response = self
            .db
            .query(
                r#"
                SELECT
                    id.id() AS id,
                    content,
                    timestamp,
                    linked_nodes[*].id() AS linked_nodes,
                    mood_signature,
                    season
                FROM journal_entry
                ORDER BY timestamp ASC;
                "#,
            )
            .await?;
        let records: Vec<JournalRecord> = response.take(0)?;
        records
            .into_iter()
            .map(|record| {
                Ok(JournalEntry {
                    id: parse_uuid(&record.id)?,
                    content: record.content,
                    timestamp: record.timestamp,
                    linked_nodes: record.linked_nodes,
                    mood_signature: record.mood_signature,
                    season: record.season,
                })
            })
            .collect()
    }

    async fn select_temporal_snapshots(&self) -> Result<Vec<TemporalSnapshot>, SurrealStoreError> {
        #[derive(Debug, Deserialize)]
        struct TsRecord {
            id: String,
            node_id: Uuid,
            snapshot: BTreeMap<String, Value>,
            timestamp: DateTime<Utc>,
            change_type: String,
        }

        let mut response = self
            .db
            .query(
                r#"
                SELECT
                    id.id() AS id,
                    node_id.id() AS node_id,
                    snapshot,
                    timestamp,
                    change_type
                FROM temporal_snapshot
                ORDER BY timestamp ASC;
                "#,
            )
            .await?;
        let records: Vec<TsRecord> = response.take(0)?;
        records
            .into_iter()
            .map(|r| {
                Ok(TemporalSnapshot {
                    id: parse_uuid(&r.id)?,
                    node_id: r.node_id,
                    snapshot: r.snapshot,
                    timestamp: r.timestamp,
                    change_type: parse_change_type(&r.change_type)?,
                })
            })
            .collect()
    }

    async fn select_lore_entries(&self) -> Result<Vec<LoreEntry>, SurrealStoreError> {
        #[derive(Debug, Deserialize)]
        struct LoreRecord {
            id: String,
            title: String,
            arc_type: String,
            timestamp: DateTime<Utc>,
            linked_nodes: Vec<Uuid>,
            narrative: String,
            significance: f32,
        }

        let mut response = self
            .db
            .query(
                r#"
                SELECT
                    id.id() AS id,
                    title,
                    arc_type,
                    timestamp,
                    linked_nodes[*].id() AS linked_nodes,
                    narrative,
                    significance
                FROM lore_entry
                ORDER BY timestamp ASC;
                "#,
            )
            .await?;
        let records: Vec<LoreRecord> = response.take(0)?;
        records
            .into_iter()
            .map(|r| {
                Ok(LoreEntry {
                    id: parse_uuid(&r.id)?,
                    title: r.title,
                    arc_type: parse_arc_type(&r.arc_type)?,
                    timestamp: r.timestamp,
                    linked_nodes: r.linked_nodes,
                    narrative: r.narrative,
                    significance: r.significance,
                })
            })
            .collect()
    }

    async fn select_silent_contracts(&self) -> Result<Vec<SilentContract>, SurrealStoreError> {
        #[derive(Debug, Deserialize)]
        struct ContractRecord {
            id: String,
            related_node: Uuid,
            detected_at: DateTime<Utc>,
            intensity: f32,
            age_days: f32,
            state: String,
        }

        let mut response = self
            .db
            .query(
                r#"
                SELECT
                    id.id() AS id,
                    related_node.id() AS related_node,
                    detected_at,
                    intensity,
                    age_days,
                    state
                FROM silent_contract;
                "#,
            )
            .await?;
        let records: Vec<ContractRecord> = response.take(0)?;
        records
            .into_iter()
            .map(|r| {
                Ok(SilentContract {
                    id: parse_uuid(&r.id)?,
                    related_node: r.related_node,
                    detected_at: r.detected_at,
                    intensity: r.intensity,
                    age_days: r.age_days,
                    state: parse_contract_state(&r.state)?,
                })
            })
            .collect()
    }

    async fn select_process_records(&self) -> Result<Vec<ProcessRecord>, SurrealStoreError> {
        #[derive(Debug, Deserialize)]
        struct ProcRecord {
            id: String,
            pid: i64,
            name: String,
            linked_node: Uuid,
            started_at: DateTime<Utc>,
            ended_at: Option<DateTime<Utc>>,
            cpu_usage: f32,
            memory_mb: f32,
        }

        let mut response = self
            .db
            .query(
                r#"
                SELECT
                    id.id() AS id,
                    pid,
                    name,
                    linked_node.id() AS linked_node,
                    started_at,
                    ended_at,
                    cpu_usage,
                    memory_mb
                FROM process_record
                ORDER BY started_at ASC;
                "#,
            )
            .await?;
        let records: Vec<ProcRecord> = response.take(0)?;
        records
            .into_iter()
            .map(|r| {
                Ok(ProcessRecord {
                    id: parse_uuid(&r.id)?,
                    pid: r.pid,
                    name: r.name,
                    linked_node: r.linked_node,
                    started_at: r.started_at,
                    ended_at: r.ended_at,
                    cpu_usage: r.cpu_usage,
                    memory_mb: r.memory_mb,
                })
            })
            .collect()
    }
}

fn change_type_to_str(ct: ChangeType) -> &'static str {
    match ct {
        ChangeType::Created => "created",
        ChangeType::Modified => "modified",
        ChangeType::Accessed => "accessed",
        ChangeType::StateChanged => "state_changed",
        ChangeType::Connected => "connected",
        ChangeType::Disconnected => "disconnected",
    }
}

fn arc_type_to_str(at: ArcType) -> &'static str {
    match at {
        ArcType::Origin => "origin",
        ArcType::Conflict => "conflict",
        ArcType::Resolution => "resolution",
        ArcType::Revelation => "revelation",
        ArcType::Transformation => "transformation",
        ArcType::Legacy => "legacy",
        ArcType::Tectonic => "tectonic",
    }
}

fn contract_state_to_str(cs: ContractState) -> &'static str {
    match cs {
        ContractState::Dormant => "dormant",
        ContractState::Awakening => "awakening",
        ContractState::Active => "active",
        ContractState::Resolved => "resolved",
        ContractState::Broken => "broken",
    }
}

fn parse_change_type(value: &str) -> Result<ChangeType, SurrealStoreError> {
    match value {
        "created" => Ok(ChangeType::Created),
        "modified" => Ok(ChangeType::Modified),
        "accessed" => Ok(ChangeType::Accessed),
        "state_changed" => Ok(ChangeType::StateChanged),
        "connected" => Ok(ChangeType::Connected),
        "disconnected" => Ok(ChangeType::Disconnected),
        other => Err(SurrealStoreError::Storage(StorageError::InvalidChangeType(
            other.to_string(),
        ))),
    }
}

fn parse_arc_type(value: &str) -> Result<ArcType, SurrealStoreError> {
    match value {
        "origin" => Ok(ArcType::Origin),
        "conflict" => Ok(ArcType::Conflict),
        "resolution" => Ok(ArcType::Resolution),
        "revelation" => Ok(ArcType::Revelation),
        "transformation" => Ok(ArcType::Transformation),
        "legacy" => Ok(ArcType::Legacy),
        "tectonic" => Ok(ArcType::Tectonic),
        other => Err(SurrealStoreError::Storage(StorageError::InvalidArcType(
            other.to_string(),
        ))),
    }
}

fn parse_contract_state(value: &str) -> Result<ContractState, SurrealStoreError> {
    match value {
        "dormant" => Ok(ContractState::Dormant),
        "awakening" => Ok(ContractState::Awakening),
        "active" => Ok(ContractState::Active),
        "resolved" => Ok(ContractState::Resolved),
        "broken" => Ok(ContractState::Broken),
        other => Err(SurrealStoreError::Storage(
            StorageError::InvalidContractState(other.to_string()),
        )),
    }
}

fn parse_uuid(value: &str) -> Result<Uuid, SurrealStoreError> {
    Uuid::parse_str(value)
        .map_err(|_| SurrealStoreError::Storage(StorageError::InvalidNodeType(value.to_string())))
}
