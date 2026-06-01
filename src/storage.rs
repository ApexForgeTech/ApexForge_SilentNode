use crate::calendar::{CalendarEvent, EventCategory};
use crate::domain::{
    ArcType, ChangeType, ContractState, EdgeData, EdgeType, FocusDepth, FocusEvent, JournalEntry,
    LoreEntry, NodeData, NodeType, Position3, ProcessRecord, SilentContract, TemporalSnapshot,
};
use crate::error::{GraphError, StorageError};
use crate::graph::CognitiveGraph;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub trait GraphStore {
    fn save(&mut self, graph: &CognitiveGraph) -> Result<(), GraphError>;
    fn load(&self) -> Result<Option<StoredGraph>, GraphError>;
}

pub trait WorkspaceStore {
    fn save_snapshot(&mut self, snapshot: &WorkspaceSnapshot) -> Result<(), StorageError>;
    fn load_snapshot(&self) -> Result<Option<WorkspaceSnapshot>, StorageError>;
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryGraphStore {
    snapshot: Option<StoredGraph>,
}

impl InMemoryGraphStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl GraphStore for InMemoryGraphStore {
    fn save(&mut self, graph: &CognitiveGraph) -> Result<(), GraphError> {
        let exported = graph.export();
        self.snapshot = Some(StoredGraph {
            nodes: exported.nodes,
            edges: exported.edges,
        });
        Ok(())
    }

    fn load(&self) -> Result<Option<StoredGraph>, GraphError> {
        Ok(self.snapshot.clone())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StoredGraph {
    pub nodes: Vec<NodeData>,
    pub edges: Vec<EdgeData>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceSnapshot {
    pub graph: StoredGraph,
    pub focus_events: Vec<FocusEvent>,
    pub journal_entries: Vec<JournalEntry>,
    #[serde(default)]
    pub system_mode: Option<String>,
    #[serde(default)]
    pub temporal_snapshots: Vec<TemporalSnapshot>,
    #[serde(default)]
    pub lore_entries: Vec<LoreEntry>,
    #[serde(default)]
    pub silent_contracts: Vec<SilentContract>,
    #[serde(default)]
    pub process_records: Vec<ProcessRecord>,
    #[serde(default)]
    pub calendar_events: Vec<CalendarEvent>,
}

#[derive(Debug, Clone)]
pub struct SqliteWorkspaceStore {
    path: PathBuf,
}

impl SqliteWorkspaceStore {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, StorageError> {
        let store = Self { path: path.into() };
        store.initialize_schema()?;
        Ok(store)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn connection(&self) -> Result<Connection, StorageError> {
        Ok(Connection::open(&self.path)?)
    }

    fn initialize_schema(&self) -> Result<(), StorageError> {
        let connection = self.connection()?;
        connection.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS node (
                id TEXT PRIMARY KEY,
                node_type TEXT NOT NULL,
                content TEXT NOT NULL,
                metadata_json TEXT NOT NULL,
                entropy REAL NOT NULL,
                gravity REAL NOT NULL,
                velocity REAL NOT NULL,
                access_count INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                accessed_at TEXT NOT NULL,
                is_ghost INTEGER NOT NULL,
                is_fossil INTEGER NOT NULL,
                is_void INTEGER NOT NULL,
                position_x REAL NOT NULL,
                position_y REAL NOT NULL,
                position_z REAL NOT NULL,
                aura_color TEXT NOT NULL,
                soul_signature_json TEXT NOT NULL,
                civilization_id TEXT
            );

            CREATE TABLE IF NOT EXISTS edge (
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                edge_type TEXT NOT NULL,
                weight REAL NOT NULL,
                created_at TEXT NOT NULL,
                last_active TEXT NOT NULL,
                PRIMARY KEY (source_id, target_id)
            );

            CREATE TABLE IF NOT EXISTS focus_event (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                node_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                duration_seconds REAL NOT NULL,
                depth TEXT NOT NULL,
                session_id TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS journal_entry (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                season TEXT,
                mood_signature_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS journal_link (
                journal_id TEXT NOT NULL,
                node_id TEXT NOT NULL,
                PRIMARY KEY (journal_id, node_id)
            );

            CREATE TABLE IF NOT EXISTS temporal_snapshot (
                id TEXT PRIMARY KEY,
                node_id TEXT NOT NULL,
                snapshot_json TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                change_type TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_temporal_snapshot_node_id ON temporal_snapshot (node_id);
            CREATE INDEX IF NOT EXISTS idx_temporal_snapshot_timestamp ON temporal_snapshot (timestamp);

            CREATE TABLE IF NOT EXISTS lore_entry (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                arc_type TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                narrative TEXT NOT NULL,
                significance REAL NOT NULL
            );

            CREATE TABLE IF NOT EXISTS lore_link (
                lore_id TEXT NOT NULL,
                node_id TEXT NOT NULL,
                PRIMARY KEY (lore_id, node_id)
            );

            CREATE INDEX IF NOT EXISTS idx_lore_link_node_id ON lore_link (node_id);

            CREATE TABLE IF NOT EXISTS silent_contract (
                id TEXT PRIMARY KEY,
                related_node TEXT NOT NULL,
                detected_at TEXT NOT NULL,
                intensity REAL NOT NULL,
                age_days REAL NOT NULL,
                state TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_silent_contract_node ON silent_contract (related_node);

            CREATE TABLE IF NOT EXISTS process_record (
                id TEXT PRIMARY KEY,
                pid INTEGER NOT NULL,
                name TEXT NOT NULL,
                linked_node TEXT NOT NULL,
                started_at TEXT NOT NULL,
                ended_at TEXT,
                cpu_usage REAL NOT NULL,
                memory_mb REAL NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_process_record_node ON process_record (linked_node);

            CREATE TABLE IF NOT EXISTS calendar_event (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                category TEXT NOT NULL,
                start_at TEXT NOT NULL,
                end_at TEXT NOT NULL,
                linked_nodes_json TEXT NOT NULL,
                is_recurring INTEGER NOT NULL,
                anticipation_days INTEGER NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_calendar_event_start_at ON calendar_event (start_at);

            CREATE TABLE IF NOT EXISTS workspace_setting (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS ml_feedback (
                id TEXT PRIMARY KEY,
                node_id TEXT,
                content TEXT NOT NULL,
                nickname TEXT,
                predicted_type TEXT,
                selected_type TEXT NOT NULL,
                confidence REAL NOT NULL,
                source TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_ml_feedback_selected_type ON ml_feedback (selected_type);
            CREATE INDEX IF NOT EXISTS idx_ml_feedback_created_at ON ml_feedback (created_at);
            "#,
        )?;

        // Migrate databases created before civilization_id was added.
        // ALTER TABLE ADD COLUMN fails silently if the column already exists.
        let _ = connection.execute("ALTER TABLE node ADD COLUMN civilization_id TEXT", []);

        Ok(())
    }
}

impl WorkspaceStore for SqliteWorkspaceStore {
    fn save_snapshot(&mut self, snapshot: &WorkspaceSnapshot) -> Result<(), StorageError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;

        transaction.execute("DELETE FROM calendar_event", [])?;
        transaction.execute("DELETE FROM process_record", [])?;
        transaction.execute("DELETE FROM silent_contract", [])?;
        transaction.execute("DELETE FROM lore_link", [])?;
        transaction.execute("DELETE FROM lore_entry", [])?;
        transaction.execute("DELETE FROM temporal_snapshot", [])?;
        transaction.execute("DELETE FROM journal_link", [])?;
        transaction.execute("DELETE FROM journal_entry", [])?;
        transaction.execute("DELETE FROM focus_event", [])?;
        transaction.execute("DELETE FROM edge", [])?;
        transaction.execute("DELETE FROM node", [])?;
        transaction.execute("DELETE FROM workspace_setting", [])?;

        if let Some(mode) = &snapshot.system_mode {
            transaction.execute(
                "INSERT INTO workspace_setting (key, value) VALUES ('system_mode', ?1)",
                params![mode],
            )?;
        }

        for node in &snapshot.graph.nodes {
            transaction.execute(
                r#"
                INSERT INTO node (
                    id, node_type, content, metadata_json, entropy, gravity, velocity, access_count,
                    created_at, accessed_at, is_ghost, is_fossil, is_void,
                    position_x, position_y, position_z, aura_color, soul_signature_json,
                    civilization_id
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)
                "#,
                params![
                    node.id.to_string(),
                    node_type_to_str(node.node_type),
                    node.content,
                    serde_json::to_string(&node.metadata)?,
                    node.entropy,
                    node.gravity,
                    node.velocity,
                    node.access_count as i64,
                    node.created_at.to_rfc3339(),
                    node.accessed_at.to_rfc3339(),
                    node.is_ghost as i64,
                    node.is_fossil as i64,
                    node.is_void as i64,
                    node.position.x,
                    node.position.y,
                    node.position.z,
                    node.aura_color,
                    serde_json::to_string(&node.soul_signature)?,
                    node.civilization_id.map(|id| id.to_string()),
                ],
            )?;
        }

        for edge in &snapshot.graph.edges {
            transaction.execute(
                r#"
                INSERT INTO edge (
                    source_id, target_id, edge_type, weight, created_at, last_active
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
                params![
                    edge.source_id.to_string(),
                    edge.target_id.to_string(),
                    edge_type_to_str(edge.edge_type),
                    edge.weight,
                    edge.created_at.to_rfc3339(),
                    edge.last_active.to_rfc3339(),
                ],
            )?;
        }

        for event in &snapshot.focus_events {
            transaction.execute(
                r#"
                INSERT INTO focus_event (
                    node_id, timestamp, duration_seconds, depth, session_id
                ) VALUES (?1, ?2, ?3, ?4, ?5)
                "#,
                params![
                    event.node_id.to_string(),
                    event.timestamp.to_rfc3339(),
                    event.duration_seconds,
                    focus_depth_to_str(event.depth),
                    event.session_id.to_string(),
                ],
            )?;
        }

        for entry in &snapshot.journal_entries {
            transaction.execute(
                r#"
                INSERT INTO journal_entry (
                    id, content, timestamp, season, mood_signature_json
                ) VALUES (?1, ?2, ?3, ?4, ?5)
                "#,
                params![
                    entry.id.to_string(),
                    entry.content,
                    entry.timestamp.to_rfc3339(),
                    entry.season,
                    serde_json::to_string(&entry.mood_signature)?,
                ],
            )?;

            for linked_node in &entry.linked_nodes {
                transaction.execute(
                    "INSERT INTO journal_link (journal_id, node_id) VALUES (?1, ?2)",
                    params![entry.id.to_string(), linked_node.to_string()],
                )?;
            }
        }

        for ts in &snapshot.temporal_snapshots {
            transaction.execute(
                r#"
                INSERT INTO temporal_snapshot (id, node_id, snapshot_json, timestamp, change_type)
                VALUES (?1, ?2, ?3, ?4, ?5)
                "#,
                params![
                    ts.id.to_string(),
                    ts.node_id.to_string(),
                    serde_json::to_string(&ts.snapshot)?,
                    ts.timestamp.to_rfc3339(),
                    change_type_to_str(ts.change_type),
                ],
            )?;
        }

        for entry in &snapshot.lore_entries {
            transaction.execute(
                r#"
                INSERT INTO lore_entry (id, title, arc_type, timestamp, narrative, significance)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
                params![
                    entry.id.to_string(),
                    entry.title,
                    arc_type_to_str(entry.arc_type),
                    entry.timestamp.to_rfc3339(),
                    entry.narrative,
                    entry.significance,
                ],
            )?;
            for linked_node in &entry.linked_nodes {
                transaction.execute(
                    "INSERT INTO lore_link (lore_id, node_id) VALUES (?1, ?2)",
                    params![entry.id.to_string(), linked_node.to_string()],
                )?;
            }
        }

        for contract in &snapshot.silent_contracts {
            transaction.execute(
                r#"
                INSERT INTO silent_contract (id, related_node, detected_at, intensity, age_days, state)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
                params![
                    contract.id.to_string(),
                    contract.related_node.to_string(),
                    contract.detected_at.to_rfc3339(),
                    contract.intensity,
                    contract.age_days,
                    contract_state_to_str(contract.state),
                ],
            )?;
        }

        for record in &snapshot.process_records {
            transaction.execute(
                r#"
                INSERT INTO process_record (id, pid, name, linked_node, started_at, ended_at, cpu_usage, memory_mb)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                "#,
                params![
                    record.id.to_string(),
                    record.pid,
                    record.name,
                    record.linked_node.to_string(),
                    record.started_at.to_rfc3339(),
                    record.ended_at.map(|dt| dt.to_rfc3339()),
                    record.cpu_usage,
                    record.memory_mb,
                ],
            )?;
        }

        for event in &snapshot.calendar_events {
            transaction.execute(
                r#"
                INSERT INTO calendar_event (
                    id, title, description, category, start_at, end_at, linked_nodes_json,
                    is_recurring, anticipation_days, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                "#,
                params![
                    event.id.to_string(),
                    event.title,
                    event.description,
                    event_category_to_str(&event.category),
                    event.start_at.to_rfc3339(),
                    event.end_at.to_rfc3339(),
                    serde_json::to_string(&event.linked_nodes)?,
                    event.is_recurring as i64,
                    event.anticipation_days as i64,
                    event.created_at.to_rfc3339(),
                ],
            )?;
        }

        transaction.commit()?;
        Ok(())
    }

    fn load_snapshot(&self) -> Result<Option<WorkspaceSnapshot>, StorageError> {
        let connection = self.connection()?;
        let node_count: i64 =
            connection.query_row("SELECT COUNT(*) FROM node", [], |row| row.get(0))?;
        let setting_count: i64 =
            connection.query_row("SELECT COUNT(*) FROM workspace_setting", [], |row| {
                row.get(0)
            })?;
        let calendar_count: i64 =
            connection.query_row("SELECT COUNT(*) FROM calendar_event", [], |row| row.get(0))?;
        if node_count == 0 && setting_count == 0 && calendar_count == 0 {
            return Ok(None);
        }

        let node_rows = {
            let mut statement = connection.prepare(
                r#"
                SELECT
                    id, node_type, content, metadata_json, entropy, gravity, velocity, access_count,
                    created_at, accessed_at, is_ghost, is_fossil, is_void,
                    position_x, position_y, position_z, aura_color, soul_signature_json,
                    civilization_id
                FROM node
                "#,
            )?;
            let rows = statement.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, f32>(4)?,
                    row.get::<_, f32>(5)?,
                    row.get::<_, f32>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, i64>(11)?,
                    row.get::<_, i64>(12)?,
                    row.get::<_, f32>(13)?,
                    row.get::<_, f32>(14)?,
                    row.get::<_, f32>(15)?,
                    row.get::<_, String>(16)?,
                    row.get::<_, String>(17)?,
                    row.get::<_, Option<String>>(18)?,
                ))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        let edge_rows = {
            let mut statement = connection.prepare(
                "SELECT source_id, target_id, edge_type, weight, created_at, last_active FROM edge",
            )?;
            let rows = statement.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, f32>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        let focus_rows = {
            let mut statement = connection.prepare(
                "SELECT node_id, timestamp, duration_seconds, depth, session_id FROM focus_event ORDER BY timestamp ASC",
            )?;
            let rows = statement.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, f32>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        let journal_rows = {
            let mut statement = connection.prepare(
                "SELECT id, content, timestamp, season, mood_signature_json FROM journal_entry ORDER BY timestamp ASC",
            )?;
            let rows = statement.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        let link_rows = {
            let mut statement = connection
                .prepare("SELECT journal_id, node_id FROM journal_link ORDER BY journal_id ASC")?;
            let rows = statement.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        let temporal_rows = {
            let mut statement = connection.prepare(
                "SELECT id, node_id, snapshot_json, timestamp, change_type FROM temporal_snapshot ORDER BY timestamp ASC",
            )?;
            let rows = statement.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        let lore_rows = {
            let mut statement = connection.prepare(
                "SELECT id, title, arc_type, timestamp, narrative, significance FROM lore_entry ORDER BY timestamp ASC",
            )?;
            let rows = statement.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, f32>(5)?,
                ))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        let lore_link_rows = {
            let mut statement = connection
                .prepare("SELECT lore_id, node_id FROM lore_link ORDER BY lore_id ASC")?;
            let rows = statement.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        let contract_rows = {
            let mut statement = connection.prepare(
                "SELECT id, related_node, detected_at, intensity, age_days, state FROM silent_contract",
            )?;
            let rows = statement.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, f32>(3)?,
                    row.get::<_, f32>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        let process_rows = {
            let mut statement = connection.prepare(
                "SELECT id, pid, name, linked_node, started_at, ended_at, cpu_usage, memory_mb FROM process_record ORDER BY started_at ASC",
            )?;
            let rows = statement.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, f32>(6)?,
                    row.get::<_, f32>(7)?,
                ))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        let calendar_rows = {
            let mut statement = connection.prepare(
                "SELECT id, title, description, category, start_at, end_at, linked_nodes_json, is_recurring, anticipation_days, created_at FROM calendar_event ORDER BY start_at ASC",
            )?;
            let rows = statement.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, String>(9)?,
                ))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        let mut linked_nodes = HashMap::<String, Vec<Uuid>>::new();
        for (journal_id, node_id) in link_rows {
            linked_nodes
                .entry(journal_id)
                .or_default()
                .push(parse_uuid(&node_id)?);
        }

        let mut lore_linked_nodes = HashMap::<String, Vec<Uuid>>::new();
        for (lore_id, node_id) in lore_link_rows {
            lore_linked_nodes
                .entry(lore_id)
                .or_default()
                .push(parse_uuid(&node_id)?);
        }

        let nodes = node_rows
            .into_iter()
            .map(
                |(
                    id,
                    node_type,
                    content,
                    metadata_json,
                    entropy,
                    gravity,
                    velocity,
                    access_count,
                    created_at,
                    accessed_at,
                    is_ghost,
                    is_fossil,
                    is_void,
                    position_x,
                    position_y,
                    position_z,
                    aura_color,
                    soul_signature_json,
                    civilization_id_str,
                )| {
                    let civilization_id =
                        civilization_id_str.as_deref().map(parse_uuid).transpose()?;
                    Ok(NodeData {
                        id: parse_uuid(&id)?,
                        node_type: parse_node_type(&node_type)?,
                        content,
                        metadata: serde_json::from_str(&metadata_json)?,
                        entropy,
                        gravity,
                        velocity,
                        access_count: access_count as u64,
                        created_at: parse_datetime(&created_at)?,
                        accessed_at: parse_datetime(&accessed_at)?,
                        is_ghost: is_ghost != 0,
                        is_fossil: is_fossil != 0,
                        is_void: is_void != 0,
                        position: Position3 {
                            x: position_x,
                            y: position_y,
                            z: position_z,
                        },
                        aura_color,
                        soul_signature: serde_json::from_str(&soul_signature_json)?,
                        civilization_id,
                    })
                },
            )
            .collect::<Result<Vec<_>, StorageError>>()?;

        let edges = edge_rows
            .into_iter()
            .map(
                |(source_id, target_id, edge_type, weight, created_at, last_active)| {
                    Ok(EdgeData {
                        source_id: parse_uuid(&source_id)?,
                        target_id: parse_uuid(&target_id)?,
                        edge_type: parse_edge_type(&edge_type)?,
                        weight,
                        created_at: parse_datetime(&created_at)?,
                        last_active: parse_datetime(&last_active)?,
                    })
                },
            )
            .collect::<Result<Vec<_>, StorageError>>()?;

        let focus_events = focus_rows
            .into_iter()
            .map(
                |(node_id, timestamp, duration_seconds, depth, session_id)| {
                    Ok(FocusEvent {
                        node_id: parse_uuid(&node_id)?,
                        timestamp: parse_datetime(&timestamp)?,
                        duration_seconds,
                        depth: parse_focus_depth(&depth)?,
                        session_id: parse_uuid(&session_id)?,
                    })
                },
            )
            .collect::<Result<Vec<_>, StorageError>>()?;

        let journal_entries = journal_rows
            .into_iter()
            .map(|(id, content, timestamp, season, mood_signature_json)| {
                Ok(JournalEntry {
                    id: parse_uuid(&id)?,
                    content,
                    timestamp: parse_datetime(&timestamp)?,
                    linked_nodes: linked_nodes.remove(&id).unwrap_or_default(),
                    mood_signature: serde_json::from_str(&mood_signature_json)?,
                    season,
                })
            })
            .collect::<Result<Vec<_>, StorageError>>()?;

        let temporal_snapshots = temporal_rows
            .into_iter()
            .map(|(id, node_id, snapshot_json, timestamp, change_type)| {
                Ok(TemporalSnapshot {
                    id: parse_uuid(&id)?,
                    node_id: parse_uuid(&node_id)?,
                    snapshot: serde_json::from_str(&snapshot_json)?,
                    timestamp: parse_datetime(&timestamp)?,
                    change_type: parse_change_type(&change_type)?,
                })
            })
            .collect::<Result<Vec<_>, StorageError>>()?;

        let lore_entries = lore_rows
            .into_iter()
            .map(
                |(id, title, arc_type, timestamp, narrative, significance)| {
                    Ok(LoreEntry {
                        id: parse_uuid(&id)?,
                        title,
                        arc_type: parse_arc_type(&arc_type)?,
                        timestamp: parse_datetime(&timestamp)?,
                        linked_nodes: lore_linked_nodes.remove(&id).unwrap_or_default(),
                        narrative,
                        significance,
                    })
                },
            )
            .collect::<Result<Vec<_>, StorageError>>()?;

        let silent_contracts = contract_rows
            .into_iter()
            .map(
                |(id, related_node, detected_at, intensity, age_days, state)| {
                    Ok(SilentContract {
                        id: parse_uuid(&id)?,
                        related_node: parse_uuid(&related_node)?,
                        detected_at: parse_datetime(&detected_at)?,
                        intensity,
                        age_days,
                        state: parse_contract_state(&state)?,
                    })
                },
            )
            .collect::<Result<Vec<_>, StorageError>>()?;

        let process_records = process_rows
            .into_iter()
            .map(
                |(id, pid, name, linked_node, started_at, ended_at, cpu_usage, memory_mb)| {
                    Ok(ProcessRecord {
                        id: parse_uuid(&id)?,
                        pid,
                        name,
                        linked_node: parse_uuid(&linked_node)?,
                        started_at: parse_datetime(&started_at)?,
                        ended_at: ended_at.map(|s| parse_datetime(&s)).transpose()?,
                        cpu_usage,
                        memory_mb,
                    })
                },
            )
            .collect::<Result<Vec<_>, StorageError>>()?;

        let calendar_events = calendar_rows
            .into_iter()
            .map(
                |(
                    id,
                    title,
                    description,
                    category,
                    start_at,
                    end_at,
                    linked_nodes_json,
                    is_recurring,
                    anticipation_days,
                    created_at,
                )| {
                    Ok(CalendarEvent {
                        id: parse_uuid(&id)?,
                        title,
                        description,
                        category: parse_event_category(&category)?,
                        start_at: parse_datetime(&start_at)?,
                        end_at: parse_datetime(&end_at)?,
                        linked_nodes: serde_json::from_str(&linked_nodes_json)?,
                        is_recurring: is_recurring != 0,
                        anticipation_days: anticipation_days as u32,
                        created_at: parse_datetime(&created_at)?,
                    })
                },
            )
            .collect::<Result<Vec<_>, StorageError>>()?;

        let system_mode = connection
            .query_row(
                "SELECT value FROM workspace_setting WHERE key = 'system_mode'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok();

        Ok(Some(WorkspaceSnapshot {
            graph: StoredGraph { nodes, edges },
            focus_events,
            journal_entries,
            system_mode,
            temporal_snapshots,
            lore_entries,
            silent_contracts,
            process_records,
            calendar_events,
        }))
    }
}

fn event_category_to_str(category: &EventCategory) -> &'static str {
    category.as_str()
}

fn node_type_to_str(node_type: NodeType) -> &'static str {
    match node_type {
        NodeType::Idea => "idea",
        NodeType::Memory => "memory",
        NodeType::Project => "project",
        NodeType::Person => "person",
        NodeType::Artifact => "artifact",
        NodeType::Media => "media",
        NodeType::Process => "process",
        NodeType::World => "world",
        NodeType::Ghost => "ghost",
        NodeType::Fossil => "fossil",
        NodeType::Other => "other",
    }
}

fn edge_type_to_str(edge_type: EdgeType) -> &'static str {
    match edge_type {
        EdgeType::Connection => "connection",
        EdgeType::Resonance => "resonance",
        EdgeType::Temporal => "temporal",
        EdgeType::Causal => "causal",
    }
}

fn focus_depth_to_str(depth: FocusDepth) -> &'static str {
    match depth {
        FocusDepth::Glance => "glance",
        FocusDepth::Read => "read",
        FocusDepth::Edit => "edit",
        FocusDepth::DeepWork => "deep_work",
    }
}

fn parse_node_type(value: &str) -> Result<NodeType, StorageError> {
    match value {
        "idea" => Ok(NodeType::Idea),
        "memory" => Ok(NodeType::Memory),
        "project" => Ok(NodeType::Project),
        "person" => Ok(NodeType::Person),
        "artifact" => Ok(NodeType::Artifact),
        "media" => Ok(NodeType::Media),
        "process" => Ok(NodeType::Process),
        "world" => Ok(NodeType::World),
        "ghost" => Ok(NodeType::Ghost),
        "fossil" => Ok(NodeType::Fossil),
        "other" => Ok(NodeType::Other),
        other => Err(StorageError::InvalidNodeType(other.to_string())),
    }
}

fn parse_edge_type(value: &str) -> Result<EdgeType, StorageError> {
    match value {
        "connection" => Ok(EdgeType::Connection),
        "resonance" => Ok(EdgeType::Resonance),
        "temporal" => Ok(EdgeType::Temporal),
        "causal" => Ok(EdgeType::Causal),
        other => Err(StorageError::InvalidEdgeType(other.to_string())),
    }
}

fn parse_focus_depth(value: &str) -> Result<FocusDepth, StorageError> {
    match value {
        "glance" => Ok(FocusDepth::Glance),
        "read" => Ok(FocusDepth::Read),
        "edit" | "think" => Ok(FocusDepth::Edit),
        "deep_work" => Ok(FocusDepth::DeepWork),
        other => Err(StorageError::InvalidFocusDepth(other.to_string())),
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

fn parse_change_type(value: &str) -> Result<ChangeType, StorageError> {
    match value {
        "created" => Ok(ChangeType::Created),
        "modified" => Ok(ChangeType::Modified),
        "accessed" => Ok(ChangeType::Accessed),
        "state_changed" => Ok(ChangeType::StateChanged),
        "connected" => Ok(ChangeType::Connected),
        "disconnected" => Ok(ChangeType::Disconnected),
        other => Err(StorageError::InvalidChangeType(other.to_string())),
    }
}

fn parse_arc_type(value: &str) -> Result<ArcType, StorageError> {
    match value {
        "origin" => Ok(ArcType::Origin),
        "conflict" => Ok(ArcType::Conflict),
        "resolution" => Ok(ArcType::Resolution),
        "revelation" => Ok(ArcType::Revelation),
        "transformation" => Ok(ArcType::Transformation),
        "legacy" => Ok(ArcType::Legacy),
        "tectonic" => Ok(ArcType::Tectonic),
        other => Err(StorageError::InvalidArcType(other.to_string())),
    }
}

fn parse_contract_state(value: &str) -> Result<ContractState, StorageError> {
    match value {
        "dormant" => Ok(ContractState::Dormant),
        "awakening" => Ok(ContractState::Awakening),
        "active" => Ok(ContractState::Active),
        "resolved" => Ok(ContractState::Resolved),
        "broken" => Ok(ContractState::Broken),
        other => Err(StorageError::InvalidContractState(other.to_string())),
    }
}

fn parse_event_category(value: &str) -> Result<EventCategory, StorageError> {
    match value {
        "meeting" => Ok(EventCategory::Meeting),
        "deadline" => Ok(EventCategory::Deadline),
        "task" => Ok(EventCategory::Task),
        "review" => Ok(EventCategory::Review),
        "personal" => Ok(EventCategory::Personal),
        "recurring" => Ok(EventCategory::Recurring),
        "milestone" => Ok(EventCategory::Milestone),
        other => Err(StorageError::InvalidNodeType(other.to_string())),
    }
}

fn parse_uuid(value: &str) -> Result<Uuid, StorageError> {
    Uuid::parse_str(value).map_err(|_| StorageError::InvalidNodeType(value.to_string()))
}

fn parse_datetime(value: &str) -> Result<DateTime<Utc>, StorageError> {
    Ok(DateTime::parse_from_rfc3339(value)
        .map_err(|_| StorageError::InvalidDateTime(value.to_string()))?
        .with_timezone(&Utc))
}
