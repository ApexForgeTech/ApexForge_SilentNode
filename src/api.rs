use crate::analytics::AnalyticsEngine;
use crate::dashboard::export_html_dashboard;
use crate::domain::{FocusDepth, NodeType, Position3};
use crate::dream::DreamEngine;
use crate::export::{export_csv, export_dot, export_edges_csv, export_markdown};
use crate::intelligence::SuggestionEngine;
use crate::materialize::MaterializationEngine;
use crate::synthesis::SynthesisEngine;
use crate::systems::ResonanceChamberEngine;
use crate::workspace::SilentNodeWorkspace;
use axum::extract::FromRef;
use axum::{
    extract::{Path, Query, State},
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::net::SocketAddr;
use std::path::{Path as StdPath, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

pub type SharedWorkspace = Arc<RwLock<SilentNodeWorkspace>>;

// ── Vault registry ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntry {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultRegistry {
    pub vaults: Vec<VaultEntry>,
    pub current: String,
    #[serde(skip)]
    pub registry_path: PathBuf,
}

impl VaultRegistry {
    pub fn load_or_create(registry_path: PathBuf, default_sqlite: &PathBuf) -> Self {
        if let Ok(data) = std::fs::read_to_string(&registry_path) {
            if let Ok(mut reg) = serde_json::from_str::<VaultRegistry>(&data) {
                reg.registry_path = registry_path;
                return reg;
            }
        }
        let default = VaultEntry {
            name: "Default".into(),
            path: default_sqlite.to_string_lossy().into_owned(),
        };
        let reg = VaultRegistry {
            vaults: vec![default],
            current: "Default".into(),
            registry_path,
        };
        reg.save();
        reg
    }

    pub fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&self.registry_path, json);
        }
    }

    pub fn current_path(&self) -> PathBuf {
        self.vaults
            .iter()
            .find(|v| v.name == self.current)
            .map(|v| PathBuf::from(&v.path))
            .unwrap_or_else(|| PathBuf::from("data/silentnode.sqlite"))
    }
}

pub type SharedVaultState = Arc<RwLock<VaultRegistry>>;

// ── App state (workspace + vault registry) ────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub workspace: SharedWorkspace,
    pub vaults: SharedVaultState,
}

impl FromRef<AppState> for SharedWorkspace {
    fn from_ref(state: &AppState) -> Self {
        state.workspace.clone()
    }
}

impl FromRef<AppState> for SharedVaultState {
    fn from_ref(state: &AppState) -> Self {
        state.vaults.clone()
    }
}

// ── Response shapes ───────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct StatusResponse {
    pub node_count: usize,
    pub edge_count: usize,
    pub ghost_count: usize,
    pub fossil_count: usize,
    pub void_count: usize,
    pub focus_events: usize,
    pub journal_entries: usize,
}

#[derive(Serialize)]
pub struct NodeResponse {
    pub id: String,
    pub node_type: String,
    pub custom_type: Option<String>,
    pub custom_color: Option<String>,
    pub schedule: Option<NodeScheduleResponse>,
    pub nickname: String,
    pub content: String,
    pub entropy: f32,
    pub gravity: f32,
    pub velocity: f32,
    pub access_count: u64,
    pub is_ghost: bool,
    pub is_fossil: bool,
    pub is_void: bool,
    pub position: PositionResponse,
    pub created_at: String,
    pub accessed_at: String,
    pub aura_color: String,
    pub entropy_state: String,
    pub velocity_state: String,
    pub visual_weight: f32,
    pub contagion_heat: f32,
}

#[derive(Serialize)]
pub struct PositionResponse {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Serialize)]
pub struct EdgeResponse {
    pub source_id: String,
    pub target_id: String,
    pub edge_type: String,
    pub weight: f32,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct CivilizationResponse {
    pub id: String,
    pub member_count: usize,
    pub internal_density: f32,
    pub age_days: f32,
    pub territory_radius: f32,
    pub color: [f32; 4],
    pub dominant_node: Option<String>,
    pub dominant_preview: String,
}

#[derive(Serialize)]
pub struct CivilizationEventResponse {
    pub kind: String,
    pub magnitude: f32,
    pub involved_civs: Vec<String>,
    pub description: String,
}

#[derive(Serialize)]
pub struct ResonancePairResponse {
    pub node_a: String,
    pub node_b: String,
    pub similarity: f32,
    pub same_civilization: bool,
}

#[derive(Serialize)]
pub struct SuggestionResponse {
    pub node_id: String,
    pub score: f32,
    pub content_preview: String,
    pub reason: String,
}

#[derive(Serialize)]
pub struct SeasonResponse {
    pub season: String,
    pub creation_rate: f32,
    pub focus_density: f32,
    pub exploration_ratio: f32,
    pub revisit_ratio: f32,
    pub avg_entropy: f32,
}

#[derive(Serialize)]
pub struct JournalEntryResponse {
    pub id: String,
    pub content: String,
    pub timestamp: String,
    pub season: Option<String>,
    pub linked_nodes: Vec<String>,
}

// ── Request shapes ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateNodeRequest {
    pub content: String,
    pub node_type: Option<String>,
    pub nickname: Option<String>,
    pub custom_type: Option<String>,
    pub custom_color: Option<String>,
    pub schedule: Option<NodeScheduleRequest>,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub z: Option<f32>,
}

#[derive(Deserialize)]
pub struct UpdateNodeRequest {
    pub content: Option<String>,
    pub node_type: Option<String>,
    pub nickname: Option<String>,
    pub aura_color: Option<String>,
    pub custom_type: Option<String>,
    pub custom_color: Option<String>,
    pub schedule: Option<NodeScheduleRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeScheduleRequest {
    pub mode: String,
    pub status: Option<String>,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub time_of_day: Option<String>,
    pub interval_minutes: Option<u32>,
    pub days_of_week: Option<Vec<u8>>,
    pub reminder_enabled: Option<bool>,
    pub reminder_minutes_before: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NodeScheduleResponse {
    pub mode: String,
    pub status: String,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub time_of_day: Option<String>,
    pub interval_minutes: Option<u32>,
    pub days_of_week: Vec<u8>,
    pub reminder_enabled: bool,
    pub reminder_minutes_before: u32,
}

#[derive(Deserialize)]
pub struct AddThoughtRequest {
    pub text: String,
    pub nickname: Option<String>,
}

#[derive(Deserialize)]
pub struct FocusRequest {
    pub node_id: String,
    pub seconds: f32,
    pub depth: Option<String>,
}

#[derive(Deserialize)]
pub struct JournalRequest {
    pub text: String,
    pub season: Option<String>,
}

#[derive(Deserialize)]
pub struct SetModeRequest {
    pub mode: Option<String>,
}

#[derive(Deserialize)]
pub struct ResonanceQuery {
    pub threshold: Option<f32>,
}

#[derive(Deserialize)]
pub struct SuggestionQuery {
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct ClusterQuery {
    pub k: Option<usize>,
}

#[derive(Deserialize)]
pub struct PageRankQuery {
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct GapsQuery {
    pub topic: Option<String>,
}

#[derive(Deserialize)]
pub struct SynthesizeRequest {
    pub query: String,
}

#[derive(Deserialize)]
pub struct ChainRequest {
    pub from: String,
    pub to: String,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub score: f64,
    pub label: String,
    pub avg_entropy: f64,
    pub density: f64,
    pub activity_rate: f64,
    pub decay_ratio: f64,
    pub component_count: usize,
    pub node_count: usize,
    pub edge_count: usize,
    pub bridge_count: usize,
    pub summary: String,
}

#[derive(Serialize)]
pub struct PageRankResponse {
    pub node_id: String,
    pub score: f64,
    pub content_preview: String,
}

#[derive(Serialize)]
pub struct BridgeResponse {
    pub source_id: String,
    pub target_id: String,
    pub weight: f32,
    pub source_preview: String,
    pub target_preview: String,
}

#[derive(Serialize)]
pub struct ProposalResponse {
    pub id: String,
    pub kind: String,
    pub confidence: f32,
    pub description: String,
}

#[derive(Serialize)]
pub struct ContractResponse {
    pub node_id: String,
    pub description: String,
    pub strength: f32,
    pub age_days: f64,
}

#[derive(Serialize)]
pub struct WeightResponse {
    pub ghost_nodes: usize,
    pub fossil_nodes: usize,
    pub void_nodes: usize,
    pub isolated_nodes: usize,
    pub pending_contracts: usize,
    pub total: f32,
    pub summary: String,
}

#[derive(Serialize)]
pub struct RitualResponse {
    pub name: String,
    pub occurrence_count: usize,
    pub strength: f32,
    pub sequence_len: usize,
}

#[derive(Serialize)]
pub struct LoreEntryResponse {
    pub id: String,
    pub title: String,
    pub arc_type: String,
    pub narrative: String,
    pub significance: f32,
    pub timestamp: String,
}

#[derive(Serialize)]
pub struct SignatureResponse {
    pub geometry: String,
    pub symmetry: String,
    pub motion: String,
    pub complexity: f32,
    pub vitality: f32,
    pub depth: f32,
    pub description: String,
    pub evolution_count: u64,
}

#[derive(Serialize)]
pub struct ShadowProjectResponse {
    pub label: String,
    pub description: String,
    pub age_days: f32,
    pub luminescence: f32,
    pub origin_kind: String,
}

#[derive(Deserialize)]
pub struct ConnectRequest {
    pub source_id: String,
    pub target_id: String,
    pub weight: Option<f32>,
}

#[derive(Serialize)]
pub struct HeatmapEntryResponse {
    pub node_id: String,
    pub content_preview: String,
    pub energy: f32,
    pub raw_score: f32,
}

#[derive(Serialize)]
pub struct HeatmapResponse {
    pub entries: Vec<HeatmapEntryResponse>,
    pub window_days: u32,
    pub obsessive_loops: Vec<ObsessiveLoopResponse>,
    pub neglected_regions: Vec<NeglectedResponse>,
}

#[derive(Serialize)]
pub struct ObsessiveLoopResponse {
    pub node_id: String,
    pub content_preview: String,
    pub revisit_count: usize,
    pub avg_session_seconds: f32,
    pub entropy: f32,
}

#[derive(Serialize)]
pub struct NeglectedResponse {
    pub node_id: String,
    pub content_preview: String,
    pub connected_active_nodes: usize,
    pub days_since_access: f32,
}

#[derive(Serialize)]
pub struct MirrorResponse {
    pub priority_gaps: Vec<PriorityGapResponse>,
    pub blind_spots: Vec<BlindSpotResponse>,
    pub obsessions: Vec<ObsessionResponse>,
    pub peak_hour: Option<u8>,
    pub peak_weekday: Option<u8>,
    pub focus_period: String,
    pub deep_work_event_count: usize,
    pub evolution: Vec<EvolutionResponse>,
}

#[derive(Serialize)]
pub struct PriorityGapResponse {
    pub node_id: String,
    pub content_preview: String,
    pub stated_rank: usize,
    pub actual_rank: usize,
    pub gap: i32,
}

#[derive(Serialize)]
pub struct BlindSpotResponse {
    pub node_id: String,
    pub content_preview: String,
    pub last_accessed_days_ago: f32,
}

#[derive(Serialize)]
pub struct ObsessionResponse {
    pub node_id: String,
    pub content_preview: String,
    pub focus_score: f32,
    pub entropy: f32,
    pub revisit_count: usize,
}

#[derive(Serialize)]
pub struct EvolutionResponse {
    pub node_id: String,
    pub label: String,
    pub entropy_start: f32,
    pub entropy_now: f32,
    pub trajectory: String,
    pub was_central: bool,
    pub state_changes: usize,
}

#[derive(Serialize)]
pub struct WeatherResponse {
    pub state: String,
    pub intensity: f32,
    pub color_r: f32,
    pub color_g: f32,
    pub color_b: f32,
    pub description: String,
}

#[derive(Serialize)]
pub struct SoulResponse {
    pub project_id: String,
    pub content_preview: String,
    pub primary_color: [f32; 4],
    pub secondary_color: [f32; 4],
    pub particle_style: String,
    pub glow_pattern: String,
    pub activity_level: f32,
    pub maturity: f32,
    pub social_density: f32,
}

#[derive(Serialize)]
pub struct BridgeGapResponse {
    pub node_a: String,
    pub node_b: String,
    pub preview_a: String,
    pub preview_b: String,
    pub similarity: f32,
    pub reason: String,
}

#[derive(Serialize)]
pub struct ImpliedConceptResponse {
    pub suggested_content: String,
    pub confidence: f32,
    pub implied_by_previews: Vec<String>,
}

#[derive(Serialize)]
pub struct SilenceResponse {
    pub missing_bridges: Vec<BridgeGapResponse>,
    pub implied_concepts: Vec<ImpliedConceptResponse>,
}

#[derive(Serialize)]
pub struct FocusTrailResponse {
    pub node_id: String,
    pub content_preview: String,
    pub timestamp: String,
    pub duration_seconds: f32,
    pub depth: String,
    pub order: usize,
}

#[derive(Serialize)]
pub struct TectonicResponse {
    pub magnitude: f32,
    pub epicenter_id: Option<String>,
    pub epicenter_preview: String,
    pub affected_node_count: usize,
    pub stress_nodes: Vec<TectonicNodeResponse>,
    pub description: String,
}

#[derive(Serialize)]
pub struct TectonicNodeResponse {
    pub node_id: String,
    pub content_preview: String,
    pub stress: f32,
    pub velocity: f32,
    pub entropy: f32,
}

#[derive(Serialize)]
pub struct ModeResponse {
    pub id: String,
    pub label: String,
    pub active: bool,
    pub intensity: f32,
    pub description: String,
    pub source: String,
}

#[derive(Serialize)]
pub struct AtmosphereResponse {
    pub id: String,
    pub label: String,
    pub region: String,
    pub intensity: f32,
    pub color: String,
    pub audio_signature: String,
    pub visual_signature: String,
}

#[derive(Serialize)]
pub struct ForgeArtifactResponse {
    pub node_id: String,
    pub label: String,
    pub artifact_type: String,
    pub parent_ids: Vec<String>,
    pub child_ids: Vec<String>,
    pub generation: usize,
    pub heat: f32,
}

#[derive(Serialize)]
pub struct TerminalContextResponse {
    pub active_processes: usize,
    pub linked_processes: usize,
    pub dominant_process: String,
    pub suggested_node_id: Option<String>,
    pub suggested_node_preview: String,
    pub lines: Vec<String>,
}

#[derive(Serialize)]
pub struct ConstellationResponse {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub member_ids: Vec<String>,
    pub member_previews: Vec<String>,
    pub gravity: f32,
    pub emotional_weight: f32,
}

#[derive(Serialize)]
pub struct VisionCoverageResponse {
    pub generated_from: String,
    pub summary: String,
    pub completion_ratio: f32,
    pub items: Vec<VisionCoverageItemResponse>,
}

#[derive(Serialize)]
pub struct VisionCoverageItemResponse {
    pub concept: String,
    pub area: String,
    pub status: String,
    pub confidence: f32,
    pub backend_evidence: Vec<String>,
    pub web_evidence: Vec<String>,
    pub gap: String,
}

#[derive(Deserialize)]
pub struct HeatmapQuery {
    pub days: Option<u32>,
}

#[derive(Deserialize)]
pub struct MirrorQuery {
    pub days: Option<u32>,
}

#[derive(Deserialize)]
pub struct TrailQuery {
    pub hours: Option<i64>,
}

// ── Extended response types ────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ArchaeologyTimelineEntry {
    pub index: usize,
    pub timestamp: String,
    pub change_type: String,
}

#[derive(Serialize)]
pub struct ArchaeologyResponse {
    pub node_id: String,
    pub snapshot_count: usize,
    pub cursor: usize,
    pub current_timestamp: String,
    pub current_content: String,
    pub current_entropy: f32,
    pub timeline: Vec<ArchaeologyTimelineEntry>,
}

#[derive(Serialize)]
pub struct DayReconstructionResponse {
    pub date: String,
    pub nodes_touched: usize,
    pub total_focus_seconds: f32,
    pub journal_entries_count: usize,
    pub dominant_node_id: Option<String>,
    pub dominant_node_preview: String,
    pub aura_state: String,
    pub aura_intensity: f32,
    pub primary_color: [f32; 4],
    pub focus_events_count: usize,
}

#[derive(Serialize)]
pub struct DayComparisonEntry {
    pub field: String,
    pub day_a_value: String,
    pub day_b_value: String,
}

#[derive(Serialize)]
pub struct DayComparisonResponse {
    pub day_a: String,
    pub day_b: String,
    pub entries: Vec<DayComparisonEntry>,
}

#[derive(Serialize)]
pub struct VoidZoneNodeResponse {
    pub node_id: String,
    pub content_preview: String,
    pub incubation_days: f32,
    pub is_mature: bool,
    pub resonance_readiness: f32,
    pub entropy: f32,
}

#[derive(Serialize)]
pub struct ResonanceChamberResponse {
    pub id: String,
    pub node_a: String,
    pub preview_a: String,
    pub node_b: String,
    pub preview_b: String,
    pub similarity: f32,
    pub state: String,
}

#[derive(Serialize)]
pub struct CalendarEventResponse {
    pub id: String,
    pub title: String,
    pub description: String,
    pub category: String,
    pub start_at: String,
    pub end_at: Option<String>,
    pub linked_node_id: Option<String>,
    pub computed_gravity: f32,
    pub hours_until: f32,
    pub is_approaching: bool,
}

#[derive(Serialize)]
pub struct DailyTaskResponse {
    pub id: String,
    pub node_id: String,
    pub title: String,
    pub status: String,
    pub date: Option<String>,
    pub tags: Vec<String>,
    pub source: String,
    pub calendar_event_id: Option<String>,
    pub gravity: f32,
    pub entropy: f32,
}

#[derive(Serialize)]
pub struct FocusWindowResponse {
    pub start_hour: u8,
    pub end_hour: u8,
    pub score: f32,
    pub reason: String,
}

#[derive(Serialize)]
pub struct MembraneRuleResponse {
    pub id: String,
    pub pattern: String,
    pub direction: String,
    pub allow: bool,
    pub description: String,
}

#[derive(Serialize)]
pub struct MembraneStatusResponse {
    pub integrity_score: f32,
    pub rule_count: usize,
    pub blocked_count: usize,
    pub rules: Vec<MembraneRuleResponse>,
}

#[derive(Serialize)]
pub struct ProcessResponse {
    pub pid: i64,
    pub name: String,
    pub command: String,
    pub cpu_usage: f32,
    pub memory_mb: f32,
    pub uptime_seconds: f32,
    pub status: String,
    pub linked_node_id: Option<String>,
}

#[derive(Serialize)]
pub struct CrystallizationCheckResponse {
    pub civ_id: String,
    pub qualifies: bool,
    pub internal_density: f32,
    pub stability_score: f32,
    pub size: usize,
    pub crystal_id: Option<String>,
}

// ── Extended request types ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ArchaeologyQuery {
    pub steps: Option<usize>,
}

#[derive(Deserialize)]
pub struct TemporalDayQuery {
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Deserialize)]
pub struct ResonanceChamberQuery {
    pub threshold: Option<f32>,
}

#[derive(Deserialize)]
pub struct AddCalendarEventRequest {
    pub title: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub start_at: String,
    pub end_at: Option<String>,
    pub linked_node_id: Option<String>,
}

#[derive(Deserialize)]
pub struct TaskQuery {
    pub date: Option<String>,
}

#[derive(Deserialize)]
pub struct AddDailyTaskRequest {
    pub title: String,
    pub date: Option<String>,
    pub tags: Option<Vec<String>>,
    pub due_at: Option<String>,
    pub notes: Option<String>,
}

#[derive(Deserialize)]
pub struct CompleteDailyTaskRequest {
    pub done: bool,
}

#[derive(Deserialize)]
pub struct AddMembraneRuleRequest {
    pub pattern: String,
    pub direction: Option<String>,
    pub allow: Option<bool>,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct LinkProcessRequest {
    pub node_id: String,
}

#[derive(Deserialize)]
pub struct NameShadowRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub telegram_enabled: bool,
    pub telegram_bot_token: Option<String>,
    pub telegram_chat_id: Option<String>,
    pub default_channel: String,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            telegram_enabled: false,
            telegram_bot_token: None,
            telegram_chat_id: None,
            default_channel: "app".into(),
        }
    }
}

#[derive(Serialize)]
pub struct NotificationSettingsResponse {
    pub telegram_enabled: bool,
    pub telegram_token_set: bool,
    pub telegram_token_preview: Option<String>,
    pub telegram_chat_id: Option<String>,
    pub default_channel: String,
}

#[derive(Deserialize)]
pub struct NotificationSettingsRequest {
    pub telegram_enabled: Option<bool>,
    pub telegram_bot_token: Option<String>,
    pub telegram_chat_id: Option<String>,
    pub default_channel: Option<String>,
}

#[derive(Deserialize)]
pub struct NotificationTestRequest {
    pub message: Option<String>,
}

#[derive(Serialize)]
pub struct NotificationTestResponse {
    pub ok: bool,
    pub channel: String,
}

// ── Error helper ───────────────────────────────────────────────────────────────

struct ApiError(String, StatusCode);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = serde_json::json!({ "error": self.0 });
        (self.1, Json(body)).into_response()
    }
}

impl<E: std::fmt::Display> From<E> for ApiError {
    fn from(e: E) -> Self {
        ApiError(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR)
    }
}

type ApiResult<T> = Result<T, ApiError>;

// ── Local settings ─────────────────────────────────────────────────────────────

const NOTIFICATION_SETTINGS_PATH: &str = "data/settings.local.json";

fn load_notification_settings() -> NotificationSettings {
    std::fs::read_to_string(NOTIFICATION_SETTINGS_PATH)
        .ok()
        .and_then(|data| serde_json::from_str::<NotificationSettings>(&data).ok())
        .unwrap_or_default()
}

fn save_notification_settings(settings: &NotificationSettings) -> ApiResult<()> {
    if let Some(parent) = StdPath::new(NOTIFICATION_SETTINGS_PATH).parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            ApiError(
                format!("settings directory error: {e}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(|e| {
        ApiError(
            format!("settings serialize error: {e}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;
    std::fs::write(NOTIFICATION_SETTINGS_PATH, json).map_err(|e| {
        ApiError(
            format!("settings save error: {e}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })
}

fn notification_settings_response(settings: NotificationSettings) -> NotificationSettingsResponse {
    NotificationSettingsResponse {
        telegram_enabled: settings.telegram_enabled,
        telegram_token_set: settings
            .telegram_bot_token
            .as_deref()
            .is_some_and(|t| !t.trim().is_empty()),
        telegram_token_preview: settings.telegram_bot_token.as_deref().and_then(mask_secret),
        telegram_chat_id: settings.telegram_chat_id,
        default_channel: settings.default_channel,
    }
}

fn mask_secret(secret: &str) -> Option<String> {
    let trimmed = secret.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.len() <= 10 {
        return Some("configured".into());
    }
    let start = &trimmed[..6];
    let end = &trimmed[trimmed.len().saturating_sub(4)..];
    Some(format!("{start}...{end}"))
}

fn normalize_notification_channel(value: Option<String>) -> String {
    match value
        .unwrap_or_else(|| "app".into())
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "telegram" => "telegram".into(),
        "both" => "both".into(),
        _ => "app".into(),
    }
}

// ── Conversion helpers ─────────────────────────────────────────────────────────

fn node_to_response(n: &crate::domain::NodeData) -> NodeResponse {
    let entropy_state = if n.is_ghost {
        "Ghost"
    } else if n.is_void {
        "Void"
    } else if n.entropy >= 0.82 {
        "Crystallizing"
    } else if n.entropy >= 0.58 {
        "Fading"
    } else if n.entropy >= 0.28 {
        "Cooling"
    } else {
        "Vibrant"
    };
    let velocity_state = if n.velocity >= 6.0 {
        "Surging"
    } else if n.velocity >= 2.0 {
        "Moving"
    } else if n.velocity > 0.15 {
        "Stirring"
    } else {
        "Still"
    };
    NodeResponse {
        id: n.id.to_string(),
        node_type: node_type_response(n.node_type).to_string(),
        custom_type: node_custom_type(n),
        custom_color: node_custom_color(n),
        schedule: node_schedule(n),
        nickname: node_nickname(n),
        content: n.content.clone(),
        entropy: n.entropy,
        gravity: n.gravity,
        velocity: n.velocity,
        access_count: n.access_count,
        is_ghost: n.is_ghost,
        is_fossil: n.is_fossil,
        is_void: n.is_void,
        position: PositionResponse {
            x: n.position.x,
            y: n.position.y,
            z: n.position.z,
        },
        created_at: n.created_at.to_rfc3339(),
        accessed_at: n.accessed_at.to_rfc3339(),
        aura_color: n.aura_color.clone(),
        entropy_state: entropy_state.into(),
        velocity_state: velocity_state.into(),
        visual_weight: (n.gravity * (1.0 + n.velocity * 0.08) * (1.0 - n.entropy * 0.25))
            .clamp(0.2, 14.0),
        contagion_heat: (n.velocity / 10.0).clamp(0.0, 1.0),
    }
}

fn node_type_response(node_type: NodeType) -> &'static str {
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

fn node_custom_type(node: &crate::domain::NodeData) -> Option<String> {
    node.metadata
        .get("custom_type")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn node_custom_color(node: &crate::domain::NodeData) -> Option<String> {
    node.metadata
        .get("custom_color")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn node_schedule(node: &crate::domain::NodeData) -> Option<NodeScheduleResponse> {
    let schedule = node.metadata.get("schedule")?.as_object()?;
    let mode = schedule
        .get("mode")
        .and_then(|value| value.as_str())
        .unwrap_or("none")
        .to_string();
    if mode == "none" {
        return None;
    }
    let days_of_week = schedule
        .get("days_of_week")
        .and_then(|value| value.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_u64().map(|day| day as u8))
                .filter(|day| *day <= 6)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Some(NodeScheduleResponse {
        mode,
        status: schedule
            .get("status")
            .and_then(|value| value.as_str())
            .unwrap_or("active")
            .to_string(),
        start_at: schedule
            .get("start_at")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        end_at: schedule
            .get("end_at")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        time_of_day: schedule
            .get("time_of_day")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        interval_minutes: schedule
            .get("interval_minutes")
            .and_then(|value| value.as_u64())
            .map(|value| value as u32),
        days_of_week,
        reminder_enabled: schedule
            .get("reminder_enabled")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
        reminder_minutes_before: schedule
            .get("reminder_minutes_before")
            .and_then(|value| value.as_u64())
            .map(|value| value as u32)
            .unwrap_or(10),
    })
}

fn node_nickname(node: &crate::domain::NodeData) -> String {
    node.metadata
        .get("nickname")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| default_nickname(&node.content))
}

fn default_nickname(content: &str) -> String {
    let words = content
        .split_whitespace()
        .take(3)
        .collect::<Vec<_>>()
        .join(" ");
    if words.is_empty() {
        "Untitled".to_string()
    } else {
        words
    }
}

fn set_node_nickname(node: &mut crate::domain::NodeData, nickname: Option<String>) {
    let nickname = nickname
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default_nickname(&node.content));
    node.metadata
        .insert("nickname".to_string(), serde_json::Value::String(nickname));
}

fn set_node_custom_fields(
    node: &mut crate::domain::NodeData,
    custom_type: Option<String>,
    custom_color: Option<String>,
) {
    if node.node_type != NodeType::Other {
        node.metadata.remove("custom_type");
        node.metadata.remove("custom_color");
        return;
    }

    if let Some(custom_type) = custom_type
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        node.metadata.insert(
            "custom_type".to_string(),
            serde_json::Value::String(custom_type),
        );
    }
    if let Some(custom_color) = custom_color
        .map(|value| value.trim().to_string())
        .filter(|value| value.starts_with('#') && (value.len() == 4 || value.len() == 7))
    {
        node.aura_color = custom_color.clone();
        node.metadata.insert(
            "custom_color".to_string(),
            serde_json::Value::String(custom_color),
        );
    }
}

fn set_node_schedule(node: &mut crate::domain::NodeData, schedule: Option<NodeScheduleRequest>) {
    let Some(schedule) = schedule else {
        return;
    };
    let mode = schedule.mode.trim().to_lowercase();
    if mode.is_empty() || mode == "none" {
        node.metadata.remove("schedule");
        return;
    }
    let mode = match mode.as_str() {
        "once" | "daily" | "weekly" | "interval" | "custom_days" => mode,
        _ => "once".to_string(),
    };
    let status = schedule
        .status
        .unwrap_or_else(|| "active".to_string())
        .trim()
        .to_lowercase();
    let status = match status.as_str() {
        "active" | "paused" | "completed" => status,
        _ => "active".to_string(),
    };
    let days = schedule
        .days_of_week
        .unwrap_or_default()
        .into_iter()
        .filter(|day| *day <= 6)
        .map(serde_json::Value::from)
        .collect::<Vec<_>>();
    let mut map = serde_json::Map::new();
    map.insert("mode".into(), serde_json::Value::String(mode));
    map.insert("status".into(), serde_json::Value::String(status));
    if let Some(start_at) = schedule.start_at.filter(|value| !value.trim().is_empty()) {
        map.insert(
            "start_at".into(),
            serde_json::Value::String(start_at.trim().to_string()),
        );
    }
    if let Some(end_at) = schedule.end_at.filter(|value| !value.trim().is_empty()) {
        map.insert(
            "end_at".into(),
            serde_json::Value::String(end_at.trim().to_string()),
        );
    }
    if let Some(time_of_day) = schedule
        .time_of_day
        .filter(|value| !value.trim().is_empty())
    {
        map.insert(
            "time_of_day".into(),
            serde_json::Value::String(time_of_day.trim().to_string()),
        );
    }
    if let Some(interval) = schedule.interval_minutes.filter(|value| *value > 0) {
        map.insert("interval_minutes".into(), serde_json::Value::from(interval));
    }
    if !days.is_empty() {
        map.insert("days_of_week".into(), serde_json::Value::Array(days));
    }
    map.insert(
        "reminder_enabled".into(),
        serde_json::Value::Bool(schedule.reminder_enabled.unwrap_or(false)),
    );
    map.insert(
        "reminder_minutes_before".into(),
        serde_json::Value::from(schedule.reminder_minutes_before.unwrap_or(10)),
    );
    node.metadata
        .insert("schedule".to_string(), serde_json::Value::Object(map));
}

fn parse_node_type(s: &str) -> NodeType {
    match s.to_lowercase().as_str() {
        "idea" => NodeType::Idea,
        "memory" => NodeType::Memory,
        "project" => NodeType::Project,
        "person" => NodeType::Person,
        "artifact" => NodeType::Artifact,
        "media" => NodeType::Media,
        "process" => NodeType::Process,
        "world" => NodeType::World,
        "ghost" => NodeType::Ghost,
        "fossil" => NodeType::Fossil,
        "other" => NodeType::Other,
        _ => NodeType::Idea,
    }
}

fn parse_focus_depth(s: &str) -> FocusDepth {
    match s.to_lowercase().as_str() {
        "read" => FocusDepth::Read,
        "edit" | "think" => FocusDepth::Edit,
        "deep_work" | "deep-work" => FocusDepth::DeepWork,
        _ => FocusDepth::Glance,
    }
}

async fn save_current_snapshot(
    app: &AppState,
    snapshot: crate::storage::WorkspaceSnapshot,
    context: &str,
) -> ApiResult<()> {
    let path = {
        let reg = app.vaults.read().await;
        reg.current_path()
    };
    use crate::storage::WorkspaceStore;
    let mut store = crate::storage::SqliteWorkspaceStore::new(path).map_err(|err| {
        ApiError(
            format!("{context} save failed: {err}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;
    store.save_snapshot(&snapshot).map_err(|err| {
        ApiError(
            format!("{context} save failed: {err}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;
    Ok(())
}

// ── Handlers ───────────────────────────────────────────────────────────────────

async fn get_status(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let stats = ws.graph.stats();
    Json(StatusResponse {
        node_count: stats.node_count,
        edge_count: stats.edge_count,
        ghost_count: stats.ghost_count,
        fossil_count: stats.fossil_count,
        void_count: stats.void_count,
        focus_events: ws.focus.events().len(),
        journal_entries: ws.journal.entries().len(),
    })
}

async fn get_notification_settings() -> ApiResult<Json<NotificationSettingsResponse>> {
    Ok(Json(notification_settings_response(
        load_notification_settings(),
    )))
}

async fn put_notification_settings(
    Json(req): Json<NotificationSettingsRequest>,
) -> ApiResult<Json<NotificationSettingsResponse>> {
    let mut settings = load_notification_settings();

    if let Some(enabled) = req.telegram_enabled {
        settings.telegram_enabled = enabled;
    }
    if let Some(token) = req.telegram_bot_token {
        let token = token.trim().to_string();
        if !token.is_empty() {
            settings.telegram_bot_token = Some(token);
        }
    }
    if let Some(chat_id) = req.telegram_chat_id {
        let chat_id = chat_id.trim().to_string();
        settings.telegram_chat_id = if chat_id.is_empty() {
            None
        } else {
            Some(chat_id)
        };
    }
    if req.default_channel.is_some() {
        settings.default_channel = normalize_notification_channel(req.default_channel);
    }

    save_notification_settings(&settings)?;
    Ok(Json(notification_settings_response(settings)))
}

async fn post_notification_test(
    Json(req): Json<NotificationTestRequest>,
) -> ApiResult<Json<NotificationTestResponse>> {
    let settings = load_notification_settings();
    send_telegram_notification(
        &settings,
        req.message
            .as_deref()
            .unwrap_or("SilentNode test notification"),
    )
    .await?;
    Ok(Json(NotificationTestResponse {
        ok: true,
        channel: "telegram".into(),
    }))
}

async fn send_telegram_notification(
    settings: &NotificationSettings,
    message: &str,
) -> ApiResult<()> {
    if !settings.telegram_enabled {
        return Err(ApiError(
            "telegram notifications are disabled".into(),
            StatusCode::BAD_REQUEST,
        ));
    }

    let token = settings
        .telegram_bot_token
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| {
            ApiError(
                "telegram bot token is missing".into(),
                StatusCode::BAD_REQUEST,
            )
        })?;
    let chat_id = settings
        .telegram_chat_id
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| {
            ApiError(
                "telegram chat id is missing".into(),
                StatusCode::BAD_REQUEST,
            )
        })?;

    let response = reqwest::Client::new()
        .post(format!("https://api.telegram.org/bot{token}/sendMessage"))
        .json(&serde_json::json!({
            "chat_id": chat_id,
            "text": message,
            "disable_web_page_preview": true,
        }))
        .send()
        .await
        .map_err(|e| {
            ApiError(
                format!("telegram request failed: {e}"),
                StatusCode::BAD_GATEWAY,
            )
        })?;

    if !response.status().is_success() {
        return Err(ApiError(
            format!("telegram rejected the notification: {}", response.status()),
            StatusCode::BAD_GATEWAY,
        ));
    }

    Ok(())
}

async fn get_nodes(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let nodes: Vec<NodeResponse> = ws.graph.nodes().map(node_to_response).collect();
    Json(nodes)
}

async fn post_node(
    State(app): State<AppState>,
    Json(req): Json<CreateNodeRequest>,
) -> ApiResult<impl IntoResponse> {
    let node_type = req
        .node_type
        .as_deref()
        .map(parse_node_type)
        .unwrap_or(NodeType::Idea);
    let mut node = crate::domain::NodeData::new(node_type, req.content);
    set_node_nickname(&mut node, req.nickname);
    set_node_custom_fields(&mut node, req.custom_type, req.custom_color);
    set_node_schedule(&mut node, req.schedule);
    node.position = Position3 {
        x: req.x.unwrap_or(0.0),
        y: req.y.unwrap_or(0.0),
        z: req.z.unwrap_or(0.0),
    };
    let (id, snapshot) = {
        let mut ws = app.workspace.write().await;
        let id = ws.graph.add_node(node)?;
        (id, ws.snapshot())
    };
    save_current_snapshot(&app, snapshot, "node create").await?;
    let ws = app.workspace.read().await;
    let created = ws
        .graph
        .get_node(id)
        .ok_or_else(|| ApiError("node not found".into(), StatusCode::NOT_FOUND))?;
    Ok((StatusCode::CREATED, Json(node_to_response(created))))
}

async fn get_node(
    State(ws): State<SharedWorkspace>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let ws = ws.read().await;
    let node_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError("invalid UUID".into(), StatusCode::BAD_REQUEST))?;
    let node = ws
        .graph
        .get_node(node_id)
        .ok_or_else(|| ApiError("node not found".into(), StatusCode::NOT_FOUND))?;
    Ok(Json(node_to_response(node)))
}

async fn delete_node(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let node_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError("invalid UUID".into(), StatusCode::BAD_REQUEST))?;
    let snapshot = {
        let mut ws = app.workspace.write().await;
        ws.graph.remove_node(node_id)?;
        ws.snapshot()
    };
    save_current_snapshot(&app, snapshot, "node delete").await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn put_node(
    State(app): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateNodeRequest>,
) -> ApiResult<impl IntoResponse> {
    let node_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError("invalid UUID".into(), StatusCode::BAD_REQUEST))?;
    let snapshot = {
        let mut ws = app.workspace.write().await;
        let node = ws
            .graph
            .get_node_mut(node_id)
            .ok_or_else(|| ApiError("node not found".into(), StatusCode::NOT_FOUND))?;
        if let Some(content) = req.content {
            let content = content.trim().to_string();
            if !content.is_empty() {
                node.content = content;
            }
        }
        if let Some(nt) = req.node_type {
            node.node_type = parse_node_type(&nt);
        }
        if let Some(color) = req.aura_color {
            let color = color.trim().to_string();
            if color.starts_with('#') && (color.len() == 4 || color.len() == 7) {
                node.aura_color = color;
            }
        }
        if let Some(nick) = req.nickname {
            let nick = nick.trim().to_string();
            if nick.is_empty() {
                set_node_nickname(node, None);
            } else {
                node.metadata
                    .insert("nickname".to_string(), serde_json::Value::String(nick));
            }
        }
        set_node_custom_fields(node, req.custom_type, req.custom_color);
        set_node_schedule(node, req.schedule);
        ws.snapshot()
    };
    save_current_snapshot(&app, snapshot, "node update").await?;
    let ws = app.workspace.read().await;
    let updated = ws
        .graph
        .get_node(node_id)
        .ok_or_else(|| ApiError("node not found".into(), StatusCode::NOT_FOUND))?;
    Ok(Json(node_to_response(updated)))
}

async fn post_focus(
    State(app): State<AppState>,
    Json(req): Json<FocusRequest>,
) -> ApiResult<impl IntoResponse> {
    let node_id = Uuid::parse_str(&req.node_id)
        .map_err(|_| ApiError("invalid UUID".into(), StatusCode::BAD_REQUEST))?;
    let depth = req
        .depth
        .as_deref()
        .map(parse_focus_depth)
        .unwrap_or(FocusDepth::Glance);
    let snapshot = {
        let mut ws = app.workspace.write().await;
        ws.record_focus(node_id, req.seconds, depth)?;
        ws.snapshot()
    };
    save_current_snapshot(&app, snapshot, "focus").await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn post_journal(
    State(ws): State<SharedWorkspace>,
    Json(req): Json<JournalRequest>,
) -> impl IntoResponse {
    let mut ws = ws.write().await;
    let entry = ws.add_journal_entry(req.text, req.season);
    Json(JournalEntryResponse {
        id: entry.id.to_string(),
        content: entry.content.clone(),
        timestamp: entry.timestamp.to_rfc3339(),
        season: entry.season.clone(),
        linked_nodes: entry.linked_nodes.iter().map(|id| id.to_string()).collect(),
    })
}

async fn get_journal(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let entries: Vec<JournalEntryResponse> = ws
        .journal
        .entries()
        .iter()
        .map(|e| JournalEntryResponse {
            id: e.id.to_string(),
            content: e.content.clone(),
            timestamp: e.timestamp.to_rfc3339(),
            season: e.season.clone(),
            linked_nodes: e.linked_nodes.iter().map(|id| id.to_string()).collect(),
        })
        .collect();
    Json(entries)
}

async fn get_season(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let report = ws.cognitive_season();
    Json(SeasonResponse {
        season: format!("{:?}", report.season),
        creation_rate: report.creation_rate,
        focus_density: report.focus_density,
        exploration_ratio: report.exploration_ratio,
        revisit_ratio: report.revisit_ratio,
        avg_entropy: report.avg_entropy,
    })
}

async fn get_civilizations(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let civs = ws.detect_civilizations();
    let resp: Vec<CivilizationResponse> = civs
        .iter()
        .map(|c| CivilizationResponse {
            id: c.id.to_string(),
            member_count: c.member_nodes.len(),
            internal_density: c.internal_density,
            age_days: c.age_days,
            territory_radius: c.territory_radius,
            color: c.color,
            dominant_node: c.dominant_node.map(|id| id.to_string()),
            dominant_preview: c
                .dominant_node
                .and_then(|id| ws.graph.get_node(id))
                .map(|n| n.content.chars().take(44).collect::<String>())
                .unwrap_or_default(),
        })
        .collect();
    Json(resp)
}

async fn get_civilization_events(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let curr = ws.detect_civilizations();
    let events = ws.detect_civilization_events(&curr);
    let resp: Vec<CivilizationEventResponse> = events
        .into_iter()
        .map(|event| CivilizationEventResponse {
            kind: format!("{:?}", event.kind),
            magnitude: event.magnitude,
            involved_civs: event
                .involved_civs
                .into_iter()
                .map(|id| id.to_string())
                .collect(),
            description: event.description,
        })
        .collect();
    Json(resp)
}

async fn get_resonances(
    State(ws): State<SharedWorkspace>,
    Query(q): Query<ResonanceQuery>,
) -> impl IntoResponse {
    let ws = ws.read().await;
    let mut engine = ResonanceChamberEngine::new();
    if let Some(t) = q.threshold {
        engine.min_similarity = t;
    }
    let nodes: Vec<&crate::domain::NodeData> = ws.graph.nodes().collect();
    let pairs = engine.find_resonances(&nodes);
    let resp: Vec<ResonancePairResponse> = pairs
        .iter()
        .map(|p| ResonancePairResponse {
            node_a: p.node_a.to_string(),
            node_b: p.node_b.to_string(),
            similarity: p.similarity,
            same_civilization: p.same_civilization,
        })
        .collect();
    Json(resp)
}

async fn get_suggestions(
    State(ws): State<SharedWorkspace>,
    Query(q): Query<SuggestionQuery>,
) -> impl IntoResponse {
    let ws = ws.read().await;
    let engine = SuggestionEngine::new();
    let limit = q.limit.unwrap_or(10);
    let suggestions = engine.suggest_next_focus(&ws, limit);
    let resp: Vec<SuggestionResponse> = suggestions
        .into_iter()
        .map(|s| SuggestionResponse {
            node_id: s.node_id.to_string(),
            score: s.score,
            content_preview: s.content_preview,
            reason: s.reason,
        })
        .collect();
    Json(resp)
}

async fn get_related(
    State(ws): State<SharedWorkspace>,
    Path(id): Path<String>,
    Query(q): Query<SuggestionQuery>,
) -> ApiResult<impl IntoResponse> {
    let ws = ws.read().await;
    let node_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError("invalid UUID".into(), StatusCode::BAD_REQUEST))?;
    let engine = SuggestionEngine::new();
    let limit = q.limit.unwrap_or(10);
    let related = engine.suggest_related(&ws, node_id, limit);
    let resp: Vec<serde_json::Value> = related
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "node_id": r.node_id.to_string(),
                "similarity": r.similarity,
                "content_preview": r.content_preview,
            })
        })
        .collect();
    Ok(Json(resp))
}

async fn get_clusters(
    State(ws): State<SharedWorkspace>,
    Query(q): Query<ClusterQuery>,
) -> impl IntoResponse {
    let ws = ws.read().await;
    let engine = SuggestionEngine::new();
    let k = q.k.unwrap_or(5).clamp(1, 20);
    let clusters = engine.cluster_content(&ws, k);
    let resp: Vec<serde_json::Value> = clusters
        .into_iter()
        .map(|c| {
            serde_json::json!({
                "centroid_id": c.centroid_id.to_string(),
                "label": c.label,
                "member_count": c.member_ids.len(),
                "member_ids": c.member_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
            })
        })
        .collect();
    Json(resp)
}

async fn post_thought(
    State(ws): State<SharedWorkspace>,
    Json(req): Json<AddThoughtRequest>,
) -> ApiResult<impl IntoResponse> {
    let mut ws = ws.write().await;
    let result = ws.materialize_thought(&MaterializationEngine::new(), &req.text)?;
    if let Some(node) = ws.graph.get_node_mut(result.node_id) {
        set_node_nickname(node, req.nickname);
    }
    Ok(Json(serde_json::json!({
        "node_id": result.node_id.to_string(),
        "node_type": format!("{:?}", result.node_type),
        "suggestions": result.suggestions.iter().take(5).map(|s| serde_json::json!({
            "node_id": s.node_id.to_string(),
            "score": s.score,
            "content_preview": s.content_preview,
        })).collect::<Vec<_>>(),
    })))
}

async fn get_edges(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let edges: Vec<EdgeResponse> = ws
        .graph
        .edges()
        .map(|e| EdgeResponse {
            source_id: e.source_id.to_string(),
            target_id: e.target_id.to_string(),
            edge_type: format!("{:?}", e.edge_type),
            weight: e.weight,
            created_at: e.created_at.to_rfc3339(),
        })
        .collect();
    Json(edges)
}

async fn get_oracle(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let signals = ws.oracle_signals();
    let resp: Vec<serde_json::Value> = signals
        .iter()
        .map(|s| {
            serde_json::json!({
                "kind": format!("{:?}", s.kind),
                "strength": s.strength,
                "description": s.description,
            })
        })
        .collect();
    Json(resp)
}

async fn export_dot_handler(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let dot = export_dot(&ws);
    (
        [(axum::http::header::CONTENT_TYPE, "text/vnd.graphviz")],
        dot,
    )
}

async fn export_csv_handler(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let csv = export_csv(&ws);
    ([(axum::http::header::CONTENT_TYPE, "text/csv")], csv)
}

async fn export_edges_csv_handler(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let csv = export_edges_csv(&ws);
    ([(axum::http::header::CONTENT_TYPE, "text/csv")], csv)
}

async fn export_markdown_handler(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let md = export_markdown(&ws);
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/markdown; charset=utf-8",
        )],
        md,
    )
}

async fn dashboard_handler(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let html = export_html_dashboard(&ws);
    (
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    )
}

// ── Phase 12: Analytics handlers ──────────────────────────────────────────────

async fn get_analytics_health(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let report = AnalyticsEngine::new().health_report(&ws);
    let label = if report.score > 0.75 {
        "Thriving"
    } else if report.score > 0.5 {
        "Healthy"
    } else if report.score > 0.25 {
        "Fragile"
    } else {
        "Critical"
    };
    Json(HealthResponse {
        score: report.score,
        label: label.into(),
        avg_entropy: report.avg_entropy,
        density: report.density,
        activity_rate: report.activity_rate,
        decay_ratio: report.decay_ratio,
        component_count: report.component_count,
        node_count: report.node_count,
        edge_count: report.edge_count,
        bridge_count: report.bridge_count,
        summary: report.summary(),
    })
}

async fn get_analytics_pagerank(
    State(ws): State<SharedWorkspace>,
    Query(q): Query<PageRankQuery>,
) -> impl IntoResponse {
    let ws = ws.read().await;
    let limit = q.limit.unwrap_or(20).clamp(1, 100);
    let entries = AnalyticsEngine::new().pagerank(&ws, limit);
    let resp: Vec<PageRankResponse> = entries
        .into_iter()
        .map(|e| PageRankResponse {
            node_id: e.node_id.to_string(),
            score: e.score,
            content_preview: e.content_preview,
        })
        .collect();
    Json(resp)
}

async fn get_analytics_bridges(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let bridges = AnalyticsEngine::new().find_bridges(&ws);
    let resp: Vec<BridgeResponse> = bridges
        .into_iter()
        .map(|b| BridgeResponse {
            source_id: b.source_id.to_string(),
            target_id: b.target_id.to_string(),
            weight: b.weight,
            source_preview: b.source_preview,
            target_preview: b.target_preview,
        })
        .collect();
    Json(resp)
}

// ── Phase 13: Dream + Synthesis handlers ──────────────────────────────────────

async fn get_dream_proposals(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let proposals = DreamEngine::new().generate(&ws);
    let resp: Vec<ProposalResponse> = proposals
        .into_iter()
        .map(|p| {
            let kind = match &p.kind {
                crate::dream::ProposalKind::SuggestEdge { .. } => "SuggestEdge",
                crate::dream::ProposalKind::ReviveGhost { .. } => "ReviveGhost",
                crate::dream::ProposalKind::MergeNodes { .. } => "MergeNodes",
                crate::dream::ProposalKind::EntropyAlert { .. } => "EntropyAlert",
            };
            ProposalResponse {
                id: p.id.to_string(),
                kind: kind.into(),
                confidence: p.confidence,
                description: p.description,
            }
        })
        .collect();
    Json(resp)
}

async fn post_synthesize(
    State(ws): State<SharedWorkspace>,
    Json(req): Json<SynthesizeRequest>,
) -> impl IntoResponse {
    let ws = ws.read().await;
    let text = SynthesisEngine::new().synthesize_topic(&ws, &req.query);
    Json(serde_json::json!({ "query": req.query, "synthesis": text }))
}

async fn get_knowledge_gaps(
    State(ws): State<SharedWorkspace>,
    Query(q): Query<GapsQuery>,
) -> impl IntoResponse {
    let ws = ws.read().await;
    let topic = q.topic.unwrap_or_default();
    let gaps = SynthesisEngine::new().find_knowledge_gaps(&ws, &topic);
    Json(serde_json::json!({ "topic": topic, "gaps": gaps }))
}

async fn post_thought_chain(
    State(ws): State<SharedWorkspace>,
    Json(req): Json<ChainRequest>,
) -> ApiResult<impl IntoResponse> {
    let ws = ws.read().await;
    let from = Uuid::parse_str(&req.from)
        .map_err(|_| ApiError("invalid from UUID".into(), StatusCode::BAD_REQUEST))?;
    let to = Uuid::parse_str(&req.to)
        .map_err(|_| ApiError("invalid to UUID".into(), StatusCode::BAD_REQUEST))?;
    let engine = SynthesisEngine::new();
    let path = engine.trace_causal_chain(&ws, from, to);
    let formatted = path.as_deref().map(|p| engine.format_chain(&ws, p));
    Ok(Json(serde_json::json!({
        "from": from.to_string(),
        "to": to.to_string(),
        "found": path.is_some(),
        "length": path.as_ref().map(|p| p.len()),
        "path": path.map(|p| p.iter().map(|id| id.to_string()).collect::<Vec<_>>()),
        "formatted": formatted,
    })))
}

// ── Missing feature handlers ───────────────────────────────────────────────────

async fn post_void_toggle(
    Path(id): Path<String>,
    State(ws): State<SharedWorkspace>,
) -> ApiResult<impl IntoResponse> {
    let mut ws = ws.write().await;
    let node_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError("invalid UUID".into(), StatusCode::BAD_REQUEST))?;
    let is_void = ws
        .graph
        .get_node(node_id)
        .map(|n| n.is_void)
        .unwrap_or(false);
    if is_void {
        ws.extract_from_void(node_id)?;
        Ok(Json(serde_json::json!({"voided": false})))
    } else {
        ws.send_to_void(node_id)?;
        Ok(Json(serde_json::json!({"voided": true})))
    }
}

async fn post_fossilize(
    Path(id): Path<String>,
    State(ws): State<SharedWorkspace>,
) -> ApiResult<impl IntoResponse> {
    let mut ws = ws.write().await;
    let node_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError("invalid UUID".into(), StatusCode::BAD_REQUEST))?;
    ws.fossilize_node(node_id)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn post_excavate(
    Path(id): Path<String>,
    State(ws): State<SharedWorkspace>,
) -> ApiResult<impl IntoResponse> {
    let mut ws = ws.write().await;
    let node_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError("invalid UUID".into(), StatusCode::BAD_REQUEST))?;
    ws.excavate_node(node_id)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn post_revive(
    Path(id): Path<String>,
    State(ws): State<SharedWorkspace>,
) -> ApiResult<impl IntoResponse> {
    let mut ws = ws.write().await;
    let node_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError("invalid UUID".into(), StatusCode::BAD_REQUEST))?;
    let engine = crate::entropy::EntropyEngine::new();
    ws.reverse_entropy(&engine, node_id);
    ws.revive_node(node_id)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn post_connect(
    State(app): State<AppState>,
    Json(req): Json<ConnectRequest>,
) -> ApiResult<impl IntoResponse> {
    let src = Uuid::parse_str(&req.source_id)
        .map_err(|_| ApiError("invalid source UUID".into(), StatusCode::BAD_REQUEST))?;
    let dst = Uuid::parse_str(&req.target_id)
        .map_err(|_| ApiError("invalid target UUID".into(), StatusCode::BAD_REQUEST))?;
    let snapshot = {
        let mut ws = app.workspace.write().await;
        match ws.connect_nodes(
            src,
            dst,
            crate::domain::EdgeType::Connection,
            req.weight.unwrap_or(1.0),
        ) {
            Ok(()) => {}
            Err(crate::error::GraphError::DuplicateEdge { .. }) => {
                return Ok(StatusCode::NO_CONTENT)
            }
            Err(err) => return Err(err.into()),
        }
        ws.snapshot()
    };
    save_current_snapshot(&app, snapshot, "connect").await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_contracts(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let contracts = ws.detect_contracts();
    let resp: Vec<ContractResponse> = contracts
        .iter()
        .map(|c| ContractResponse {
            node_id: c.node_id.to_string(),
            description: c.description.clone(),
            strength: c.strength,
            age_days: c.age_days as f64,
        })
        .collect();
    Json(resp)
}

async fn get_weight(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let w = ws.cognitive_weight();
    Json(WeightResponse {
        ghost_nodes: w.ghost_nodes,
        fossil_nodes: w.fossil_nodes,
        void_nodes: w.void_nodes,
        isolated_nodes: w.isolated_nodes,
        pending_contracts: w.pending_contracts,
        total: w.total,
        summary: w.summary(),
    })
}

async fn get_rituals(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let rituals = ws.detect_rituals();
    let resp: Vec<RitualResponse> = rituals
        .iter()
        .map(|r| RitualResponse {
            name: r.name.clone(),
            occurrence_count: r.occurrence_count,
            strength: r.strength,
            sequence_len: r.sequence.len(),
        })
        .collect();
    Json(resp)
}

async fn get_lore(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let arcs = ws.detect_lore(&[]);
    let resp: Vec<LoreEntryResponse> = arcs
        .iter()
        .map(|e| LoreEntryResponse {
            id: e.id.to_string(),
            title: e.title.clone(),
            arc_type: format!("{:?}", e.arc_type).to_lowercase(),
            narrative: e.narrative.clone(),
            significance: e.significance,
            timestamp: e.timestamp.to_rfc3339(),
        })
        .collect();
    Json(resp)
}

async fn get_signature(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let mut ws = ws.write().await;
    let sig = ws.derive_living_signature();
    Json(SignatureResponse {
        geometry: sig.geometry.as_str().to_string(),
        symmetry: sig.symmetry.as_str().to_string(),
        motion: sig.motion.as_str().to_string(),
        complexity: sig.complexity,
        vitality: sig.vitality,
        depth: sig.depth,
        description: sig.description.clone(),
        evolution_count: sig.evolution_count,
    })
}

async fn get_shadow_projects(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let shadows = ws.detect_shadow_projects();
    let resp: Vec<ShadowProjectResponse> = shadows
        .iter()
        .map(|s| {
            let origin_kind = match &s.origin {
                crate::identity::ShadowOrigin::LongIncubation { .. } => "void",
                crate::identity::ShadowOrigin::ReleasedShadow { .. } => "released",
                crate::identity::ShadowOrigin::AbandonedHighGravity { .. } => "abandoned",
                crate::identity::ShadowOrigin::OrphanedArchitecture { .. } => "orphaned",
            };
            ShadowProjectResponse {
                label: s.label.clone(),
                description: s.description.clone(),
                age_days: s.age_days,
                luminescence: s.luminescence,
                origin_kind: origin_kind.to_string(),
            }
        })
        .collect();
    Json(resp)
}

async fn delete_nodes_bulk(
    State(ws): State<SharedWorkspace>,
    Json(ids): Json<Vec<String>>,
) -> ApiResult<impl IntoResponse> {
    let mut ws = ws.write().await;
    let mut deleted = 0usize;
    for id_str in &ids {
        if let Ok(uid) = Uuid::parse_str(id_str) {
            if ws.remove_node(uid).is_ok() {
                deleted += 1;
            }
        }
    }
    Ok(Json(serde_json::json!({"deleted": deleted})))
}

// ── New feature handlers ───────────────────────────────────────────────────────

async fn get_heatmap(
    State(ws): State<SharedWorkspace>,
    Query(q): Query<HeatmapQuery>,
) -> impl IntoResponse {
    let ws = ws.read().await;
    let days = q.days.unwrap_or(30);
    let heatmap = ws.thought_heatmap(days);
    let loops = ws.obsessive_loops();
    let neglected = ws.neglected_regions();

    let entries: Vec<HeatmapEntryResponse> = heatmap
        .entries
        .iter()
        .map(|e| {
            let preview = ws
                .graph
                .get_node(e.node_id)
                .map(|n| n.content.chars().take(40).collect::<String>())
                .unwrap_or_default();
            HeatmapEntryResponse {
                node_id: e.node_id.to_string(),
                content_preview: preview,
                energy: e.energy,
                raw_score: e.raw_score,
            }
        })
        .collect();

    let obsessive: Vec<ObsessiveLoopResponse> = loops
        .iter()
        .map(|l| {
            let preview = ws
                .graph
                .get_node(l.node_id)
                .map(|n| n.content.chars().take(40).collect::<String>())
                .unwrap_or_default();
            ObsessiveLoopResponse {
                node_id: l.node_id.to_string(),
                content_preview: preview,
                revisit_count: l.revisit_count,
                avg_session_seconds: l.avg_session_seconds,
                entropy: l.entropy,
            }
        })
        .collect();

    let neg: Vec<NeglectedResponse> = neglected
        .iter()
        .map(|n| {
            let preview = ws
                .graph
                .get_node(n.node_id)
                .map(|nd| nd.content.chars().take(40).collect::<String>())
                .unwrap_or_default();
            NeglectedResponse {
                node_id: n.node_id.to_string(),
                content_preview: preview,
                connected_active_nodes: n.connected_active_nodes,
                days_since_access: n.days_since_access,
            }
        })
        .collect();

    Json(HeatmapResponse {
        entries,
        window_days: days,
        obsessive_loops: obsessive,
        neglected_regions: neg,
    })
}

async fn get_mirror(
    State(ws): State<SharedWorkspace>,
    Query(q): Query<MirrorQuery>,
) -> impl IntoResponse {
    let ws = ws.read().await;
    let days = q.days.unwrap_or(30);
    let portrait = ws.cognitive_mirror(days);

    let gaps: Vec<PriorityGapResponse> = portrait
        .priority_gaps
        .iter()
        .map(|g| {
            let preview = ws
                .graph
                .get_node(g.node_id)
                .map(|n| n.content.chars().take(40).collect::<String>())
                .unwrap_or_default();
            PriorityGapResponse {
                node_id: g.node_id.to_string(),
                content_preview: preview,
                stated_rank: g.stated_rank,
                actual_rank: g.actual_rank,
                gap: g.gap,
            }
        })
        .collect();

    let blinds: Vec<BlindSpotResponse> = portrait
        .blind_spots
        .iter()
        .map(|b| {
            let preview = ws
                .graph
                .get_node(b.node_id)
                .map(|n| n.content.chars().take(40).collect::<String>())
                .unwrap_or_default();
            BlindSpotResponse {
                node_id: b.node_id.to_string(),
                content_preview: preview,
                last_accessed_days_ago: b.last_accessed_days_ago,
            }
        })
        .collect();

    let obsessions: Vec<ObsessionResponse> = portrait
        .obsessions
        .iter()
        .map(|o| {
            let preview = ws
                .graph
                .get_node(o.node_id)
                .map(|n| n.content.chars().take(40).collect::<String>())
                .unwrap_or_default();
            ObsessionResponse {
                node_id: o.node_id.to_string(),
                content_preview: preview,
                focus_score: o.focus_score,
                entropy: o.entropy,
                revisit_count: o.revisit_count,
            }
        })
        .collect();

    let evolution: Vec<EvolutionResponse> = portrait
        .evolution
        .iter()
        .map(|e| EvolutionResponse {
            node_id: e.node_id.to_string(),
            label: e.label.clone(),
            entropy_start: e.entropy_start,
            entropy_now: e.entropy_now,
            trajectory: e.trajectory.clone(),
            was_central: e.was_central,
            state_changes: e.state_changes,
        })
        .collect();

    let (peak_h, peak_w, period, count) = portrait
        .creative_pattern
        .as_ref()
        .map(|cp| {
            (
                cp.peak_hour,
                cp.peak_weekday,
                cp.focus_period.clone(),
                cp.deep_work_event_count,
            )
        })
        .unwrap_or((None, None, "Unknown".into(), 0));

    Json(MirrorResponse {
        priority_gaps: gaps,
        blind_spots: blinds,
        obsessions,
        peak_hour: peak_h,
        peak_weekday: peak_w,
        focus_period: period,
        deep_work_event_count: count,
        evolution,
    })
}

async fn get_weather(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    use crate::systems::WeatherSystem;
    let ws = ws.read().await;
    let mut weather = WeatherSystem::new();
    ws.derive_weather(&mut weather);
    let (state, intensity, r, g, b, desc) = match &weather.current {
        crate::systems::WeatherState::Energetic {
            pulse_rate,
            expansion,
        } => (
            "Energetic",
            (*pulse_rate + *expansion) / 2.0,
            0.25f32,
            0.85,
            0.45,
            "High creative energy — ideas expanding outward",
        ),
        crate::systems::WeatherState::Calm { clarity, .. } => (
            "Calm",
            *clarity,
            0.25,
            0.60,
            0.95,
            "Clear and focused — deep work state",
        ),
        crate::systems::WeatherState::Fading { dim_factor, .. } => (
            "Fading",
            *dim_factor,
            0.50,
            0.50,
            0.65,
            "Cognitive depletion — rest and reflection recommended",
        ),
        crate::systems::WeatherState::Reflective {
            ghost_visibility, ..
        } => (
            "Reflective",
            *ghost_visibility,
            0.60,
            0.45,
            0.80,
            "Contemplative state — past echoes are visible",
        ),
        crate::systems::WeatherState::Turbulent { intensity, .. } => (
            "Turbulent",
            *intensity,
            0.85,
            0.35,
            0.25,
            "Cognitive overload — scattered attention",
        ),
    };
    Json(WeatherResponse {
        state: state.into(),
        intensity,
        color_r: r,
        color_g: g,
        color_b: b,
        description: desc.into(),
    })
}

async fn get_souls(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let souls = ws.derive_souls();
    let resp: Vec<SoulResponse> = souls
        .iter()
        .map(|s| {
            use crate::systems::{GlowPattern, ParticleStyle};
            let preview = ws
                .graph
                .get_node(s.project_id)
                .map(|n| n.content.chars().take(40).collect::<String>())
                .unwrap_or_default();
            let particle = match s.particle_style {
                ParticleStyle::Aggressive => "aggressive",
                ParticleStyle::Crystalline => "crystalline",
                ParticleStyle::Fluid => "fluid",
                ParticleStyle::Organic => "organic",
            };
            let glow = match s.glow_pattern {
                GlowPattern::Pulse => "pulse",
                GlowPattern::Radiate => "radiate",
                GlowPattern::Breathe => "breathe",
                GlowPattern::Flicker => "flicker",
                GlowPattern::Steady => "steady",
            };
            SoulResponse {
                project_id: s.project_id.to_string(),
                content_preview: preview,
                primary_color: s.primary_color,
                secondary_color: s.secondary_color,
                particle_style: particle.into(),
                glow_pattern: glow.into(),
                activity_level: s.activity_level,
                maturity: s.maturity,
                social_density: s.social_density,
            }
        })
        .collect();
    Json(resp)
}

async fn get_silence(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    use crate::silence::SilenceAnalyzer;
    let ws = ws.read().await;
    let analyzer = SilenceAnalyzer::new();
    let bridges = ws.find_missing_bridges(&analyzer);
    let implied = ws.find_implied_concepts(&analyzer);

    let br: Vec<BridgeGapResponse> = bridges
        .iter()
        .take(20)
        .map(|b| {
            let pa = ws
                .graph
                .get_node(b.node_a)
                .map(|n| n.content.chars().take(30).collect::<String>())
                .unwrap_or_default();
            let pb = ws
                .graph
                .get_node(b.node_b)
                .map(|n| n.content.chars().take(30).collect::<String>())
                .unwrap_or_default();
            BridgeGapResponse {
                node_a: b.node_a.to_string(),
                node_b: b.node_b.to_string(),
                preview_a: pa,
                preview_b: pb,
                similarity: b.similarity,
                reason: b.reason.clone(),
            }
        })
        .collect();

    let im: Vec<ImpliedConceptResponse> = implied
        .iter()
        .take(10)
        .map(|i| {
            let by: Vec<String> = i
                .implied_by
                .iter()
                .filter_map(|id| ws.graph.get_node(*id))
                .map(|n| n.content.chars().take(20).collect::<String>())
                .collect();
            ImpliedConceptResponse {
                suggested_content: i.suggested_content.clone(),
                confidence: i.confidence,
                implied_by_previews: by,
            }
        })
        .collect();

    Json(SilenceResponse {
        missing_bridges: br,
        implied_concepts: im,
    })
}

async fn get_trail(
    State(ws): State<SharedWorkspace>,
    Query(q): Query<TrailQuery>,
) -> impl IntoResponse {
    let ws = ws.read().await;
    let hours = q.hours.unwrap_or(48);
    let trail = ws.recent_trail(hours);
    let resp: Vec<FocusTrailResponse> = trail
        .iter()
        .enumerate()
        .map(|(order, ev)| {
            let preview = ws
                .graph
                .get_node(ev.node_id)
                .map(|n| n.content.chars().take(40).collect::<String>())
                .unwrap_or_default();
            let depth = format!("{:?}", ev.depth);
            FocusTrailResponse {
                node_id: ev.node_id.to_string(),
                content_preview: preview,
                timestamp: ev.timestamp.to_rfc3339(),
                duration_seconds: ev.duration_seconds,
                depth,
                order,
            }
        })
        .collect();
    Json(resp)
}

async fn get_tectonics(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let mut stress_nodes: Vec<TectonicNodeResponse> = ws
        .graph
        .nodes()
        .map(|n| {
            let stress =
                (n.velocity / 10.0 * 0.55 + n.entropy * 0.35 + (n.gravity / 10.0).min(1.0) * 0.10)
                    .clamp(0.0, 1.0);
            TectonicNodeResponse {
                node_id: n.id.to_string(),
                content_preview: n.content.chars().take(44).collect(),
                stress,
                velocity: n.velocity,
                entropy: n.entropy,
            }
        })
        .collect();
    stress_nodes.sort_by(|a, b| b.stress.total_cmp(&a.stress));
    stress_nodes.truncate(12);
    let magnitude = if ws.graph.node_count() == 0 {
        0.0
    } else {
        (ws.graph
            .nodes()
            .map(|n| n.velocity / 10.0 + n.entropy)
            .sum::<f32>()
            / ws.graph.node_count() as f32
            / 2.0)
            .clamp(0.0, 1.0)
    };
    let epicenter = stress_nodes.first();
    let description = if magnitude > 0.68 {
        "High tectonic pressure: several ideas are moving or decaying at once"
    } else if magnitude > 0.34 {
        "Moderate structural drift: the graph is reorganizing around recent activity"
    } else {
        "Low tectonic pressure: the universe is structurally calm"
    };
    Json(TectonicResponse {
        magnitude,
        epicenter_id: epicenter.map(|n| n.node_id.clone()),
        epicenter_preview: epicenter
            .map(|n| n.content_preview.clone())
            .unwrap_or_default(),
        affected_node_count: stress_nodes.iter().filter(|n| n.stress > 0.25).count(),
        stress_nodes,
        description: description.into(),
    })
}

async fn get_modes(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let node_count = ws.graph.node_count().max(1) as f32;
    let project_ratio = ws
        .graph
        .nodes()
        .filter(|n| {
            matches!(
                n.node_type,
                crate::domain::NodeType::Project
                    | crate::domain::NodeType::Artifact
                    | crate::domain::NodeType::Process
            )
        })
        .count() as f32
        / node_count;
    let memory_ratio = ws
        .graph
        .nodes()
        .filter(|n| {
            matches!(
                n.node_type,
                crate::domain::NodeType::Memory | crate::domain::NodeType::Person
            )
        })
        .count() as f32
        / node_count;
    let ghost_ratio =
        ws.graph.nodes().filter(|n| n.is_ghost || n.is_void).count() as f32 / node_count;
    let focus_count = ws.recent_trail(24).len() as f32;
    let research_signal = ws.detect_civilizations().len() as f32 / 5.0;
    let modes = [
        (
            "builder",
            "Builder",
            project_ratio + (focus_count / 12.0).min(0.4),
            "Creation, code, artifacts, processes, active making",
        ),
        (
            "researcher",
            "Researcher",
            research_signal.min(1.0),
            "Exploration, relation discovery, knowledge clustering",
        ),
        (
            "memory",
            "Memory",
            memory_ratio + (ws.journal.entries().len() as f32 / 20.0).min(0.4),
            "Temporal recall, journal context, personal continuity",
        ),
        (
            "ghost",
            "Ghost",
            ghost_ratio,
            "Dormant ideas, void zones, shadows, unfinished paths",
        ),
    ];
    let max_id = modes
        .iter()
        .max_by(|a, b| a.2.total_cmp(&b.2))
        .map(|m| m.0)
        .unwrap_or("builder");
    let selected = ws.system_mode.as_deref().unwrap_or(max_id);
    Json(
        modes
            .into_iter()
            .map(|(id, label, intensity, description)| ModeResponse {
                id: id.into(),
                label: label.into(),
                active: id == selected,
                intensity: if id == selected && ws.system_mode.is_some() {
                    intensity.max(0.72)
                } else {
                    intensity
                }
                .clamp(0.0, 1.0),
                description: description.into(),
                source: if ws.system_mode.is_some() {
                    "manual"
                } else {
                    "inferred"
                }
                .into(),
            })
            .collect::<Vec<_>>(),
    )
}

async fn post_mode(
    State(app): State<AppState>,
    Json(req): Json<SetModeRequest>,
) -> ApiResult<impl IntoResponse> {
    let requested = req
        .mode
        .map(|mode| mode.trim().to_lowercase())
        .filter(|mode| !mode.is_empty() && mode != "auto" && mode != "inferred");
    if let Some(mode) = requested.as_deref() {
        let allowed = matches!(mode, "builder" | "researcher" | "ghost" | "memory");
        if !allowed {
            return Err(ApiError(
                "mode must be builder, researcher, ghost, memory, or auto".into(),
                StatusCode::BAD_REQUEST,
            ));
        }
    }
    let path = {
        let reg = app.vaults.read().await;
        reg.current_path()
    };
    let snapshot = {
        let mut ws = app.workspace.write().await;
        ws.set_system_mode(requested.clone());
        ws.snapshot()
    };
    {
        use crate::storage::WorkspaceStore;
        let mut store = crate::storage::SqliteWorkspaceStore::new(path).map_err(|err| {
            ApiError(
                format!("mode save failed: {err}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;
        store.save_snapshot(&snapshot).map_err(|err| {
            ApiError(
                format!("mode save failed: {err}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;
    }
    Ok(Json(serde_json::json!({
        "mode": requested,
        "source": if requested.is_some() { "manual" } else { "inferred" }
    })))
}

async fn get_atmospheres(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let avg_entropy = if ws.graph.node_count() == 0 {
        0.0
    } else {
        ws.graph.nodes().map(|n| n.entropy).sum::<f32>() / ws.graph.node_count() as f32
    };
    let technical = ws
        .graph
        .nodes()
        .filter(|n| {
            matches!(
                n.node_type,
                crate::domain::NodeType::Process | crate::domain::NodeType::Artifact
            )
        })
        .count() as f32
        / ws.graph.node_count().max(1) as f32;
    let creative = ws
        .graph
        .nodes()
        .filter(|n| {
            matches!(
                n.node_type,
                crate::domain::NodeType::Idea | crate::domain::NodeType::Project
            )
        })
        .count() as f32
        / ws.graph.node_count().max(1) as f32;
    let personal = ws
        .graph
        .nodes()
        .filter(|n| {
            matches!(
                n.node_type,
                crate::domain::NodeType::Person | crate::domain::NodeType::Memory
            )
        })
        .count() as f32
        / ws.graph.node_count().max(1) as f32;
    let research = ws.detect_civilizations().len() as f32 / 6.0;
    let rows = [
        (
            "research",
            "Research",
            research,
            "#38bdf8",
            "low drone, wide reverb, slow LFO",
            "cool depth, distant layered graph",
        ),
        (
            "creative",
            "Creative",
            creative,
            "#fbbf24",
            "warm harmonic bloom, organic drift",
            "amber motion, expansive halos",
        ),
        (
            "technical",
            "Technical",
            technical,
            "#2dd4bf",
            "precise pulse, tight reverb, low noise",
            "grid pressure, sharper edges",
        ),
        (
            "personal",
            "Personal",
            personal,
            "#f472b6",
            "soft intimate A3, heavy reverb",
            "close memory glow, gentle contrast",
        ),
        (
            "high_entropy",
            "High Entropy",
            avg_entropy,
            "#f87171",
            "fragmented harmonics, noisy turbulence",
            "stress shells, unstable motion",
        ),
    ];
    Json(
        rows.into_iter()
            .map(
                |(id, label, intensity, color, audio, visual)| AtmosphereResponse {
                    id: id.into(),
                    label: label.into(),
                    region: if intensity > 0.45 {
                        "dominant"
                    } else {
                        "latent"
                    }
                    .into(),
                    intensity: intensity.clamp(0.0, 1.0),
                    color: color.into(),
                    audio_signature: audio.into(),
                    visual_signature: visual.into(),
                },
            )
            .collect::<Vec<_>>(),
    )
}

async fn get_forge_genealogy(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let artifacts: Vec<_> = ws
        .graph
        .nodes()
        .filter(|n| {
            matches!(
                n.node_type,
                crate::domain::NodeType::Artifact
                    | crate::domain::NodeType::Project
                    | crate::domain::NodeType::World
            )
        })
        .map(|n| {
            let parents: Vec<String> = ws
                .graph
                .incoming_edges(n.id)
                .unwrap_or_default()
                .into_iter()
                .map(|e| e.source_id.to_string())
                .collect();
            let children: Vec<String> = ws
                .graph
                .outgoing_edges(n.id)
                .unwrap_or_default()
                .into_iter()
                .map(|e| e.target_id.to_string())
                .collect();
            ForgeArtifactResponse {
                node_id: n.id.to_string(),
                label: n.content.chars().take(56).collect(),
                artifact_type: format!("{:?}", n.node_type),
                generation: parents.len().min(9),
                parent_ids: parents,
                child_ids: children,
                heat: ((n.gravity / 10.0) + (n.velocity / 10.0)).clamp(0.0, 1.0),
            }
        })
        .collect();
    Json(artifacts)
}

async fn get_terminal_context(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let procs = ws.processes.scan();
    let linked = procs
        .iter()
        .filter(|p| ws.processes.linked_node(p.pid).is_some())
        .count();
    let dominant = procs.iter().max_by(|a, b| {
        (a.cpu_usage + a.memory_mb / 256.0).total_cmp(&(b.cpu_usage + b.memory_mb / 256.0))
    });
    let suggested = ws
        .recent_trail(6)
        .last()
        .and_then(|ev| ws.graph.get_node(ev.node_id))
        .or_else(|| {
            ws.graph
                .nodes()
                .max_by(|a, b| a.gravity.total_cmp(&b.gravity))
        });
    let mut lines = Vec::new();
    lines.push(format!(
        "silentnode://processes scanned={} linked={}",
        procs.len(),
        linked
    ));
    if let Some(proc_) = dominant {
        lines.push(format!(
            "dominant process: {} pid={} mem={:.0}MB",
            proc_.name, proc_.pid, proc_.memory_mb
        ));
    }
    if let Some(node) = suggested {
        lines.push(format!(
            "context node: {} entropy={:.2} gravity={:.2}",
            node.content.chars().take(42).collect::<String>(),
            node.entropy,
            node.gravity
        ));
    }
    lines.push(
        "terminal is observational in this build; execution remains outside SilentNode".into(),
    );
    Json(TerminalContextResponse {
        active_processes: procs.len(),
        linked_processes: linked,
        dominant_process: dominant.map(|p| p.name.clone()).unwrap_or_default(),
        suggested_node_id: suggested.map(|n| n.id.to_string()),
        suggested_node_preview: suggested
            .map(|n| n.content.chars().take(44).collect())
            .unwrap_or_default(),
        lines,
    })
}

async fn get_constellations(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let anchors: Vec<_> = ws
        .graph
        .nodes()
        .filter(|n| {
            matches!(
                n.node_type,
                crate::domain::NodeType::Person
                    | crate::domain::NodeType::Memory
                    | crate::domain::NodeType::Project
                    | crate::domain::NodeType::World
            )
        })
        .collect();
    let resp: Vec<ConstellationResponse> = anchors
        .into_iter()
        .take(16)
        .map(|anchor| {
            let mut members: Vec<_> = ws
                .graph
                .neighbors(anchor.id)
                .unwrap_or_default()
                .into_iter()
                .take(8)
                .collect();
            if members.is_empty() {
                members.push(anchor);
            }
            let kind = match anchor.node_type {
                crate::domain::NodeType::Person => "life",
                crate::domain::NodeType::Memory => "memory",
                crate::domain::NodeType::Project | crate::domain::NodeType::World => "goal",
                _ => "constellation",
            };
            let emotional_weight = (anchor.gravity * 0.12
                + (1.0 - anchor.entropy) * 0.5
                + anchor.access_count as f32 * 0.03)
                .clamp(0.0, 1.0);
            ConstellationResponse {
                id: anchor.id.to_string(),
                label: anchor.content.chars().take(48).collect(),
                kind: kind.into(),
                member_ids: members.iter().map(|n| n.id.to_string()).collect(),
                member_previews: members
                    .iter()
                    .map(|n| n.content.chars().take(28).collect())
                    .collect(),
                gravity: anchor.gravity,
                emotional_weight,
            }
        })
        .collect();
    Json(resp)
}

async fn get_vision_coverage(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let node_count = ws.graph.node_count();
    let world_count = ws
        .graph
        .nodes()
        .filter(|n| matches!(n.node_type, crate::domain::NodeType::World))
        .count();
    let focus_count = ws.recent_trail(24 * 365 * 10).len();
    let journal_count = ws.journal.entries().len();
    let civilization_count = ws.detect_civilizations().len();
    let void_count = ws.graph.nodes().filter(|n| n.is_void).count();
    let process_count = ws.processes.scan().len();

    let mut items = Vec::new();
    let mut add = |concept: &str,
                   area: &str,
                   status: &str,
                   confidence: f32,
                   backend: Vec<&str>,
                   web: Vec<&str>,
                   gap: &str| {
        items.push(VisionCoverageItemResponse {
            concept: concept.into(),
            area: area.into(),
            status: status.into(),
            confidence,
            backend_evidence: backend.into_iter().map(str::to_string).collect(),
            web_evidence: web.into_iter().map(str::to_string).collect(),
            gap: gap.into(),
        });
    };

    add(
        "Void Zone",
        "memory dynamics",
        if void_count > 0 { "live" } else { "partial" },
        if void_count > 0 { 0.78 } else { 0.62 },
        vec!["VoidZone tracking in workspace", "GET /void-zones", "POST /nodes/:id/void"],
        vec!["Void panel", "graph void styling"],
        "Incubation/readiness is visible, but void zones are still node-centric rather than a full editable ritual space.",
    );
    add(
        "Civilization Dynamics",
        "graph intelligence",
        if civilization_count > 0 { "live" } else { "partial" },
        if civilization_count > 0 { 0.82 } else { 0.66 },
        vec!["detect_civilizations()", "GET /civilizations", "GET /civilization-events"],
        vec!["Civilization Dynamics summary", "Intelligence panel"],
        "Expansion/trade/conflict/collapse are detected snapshots, not a persisted simulation timeline yet.",
    );
    add(
        "The Silence Between",
        "ambient intelligence",
        "live",
        0.84,
        vec![
            "SilenceAnalyzer",
            "find_missing_bridges()",
            "find_implied_concepts()",
            "GET /silence",
        ],
        vec!["Silence panel"],
        "Bridge suggestions exist; accepting/rejecting them is not a first-class workflow yet.",
    );
    add(
        "Entropy Visualization",
        "visual language",
        "live",
        0.78,
        vec![
            "NodeResponse entropy_state",
            "visual_weight",
            "contagion_heat",
        ],
        vec!["Graph shells", "Node detail entropy/velocity badges"],
        "States render in graph/detail, but there is not yet a dedicated entropy legend/timeline.",
    );
    add(
        "Thought Velocity",
        "activity physics",
        if focus_count > 0 { "live" } else { "partial" },
        if focus_count > 0 { 0.74 } else { 0.58 },
        vec![
            "velocity field",
            "NodeResponse velocity_state",
            "focus trail",
        ],
        vec!["Node detail velocity gauge", "Live Pulse list"],
        "Velocity is computed from activity, but the UI can still explain causality better.",
    );
    add(
        "Contagion",
        "graph propagation",
        "partial",
        0.64,
        vec!["Contagion heat exposed on nodes"],
        vec!["Graph heat rings", "Live Pulse ranking"],
        "Ripple history is visualized as heat, not as an animated propagation event stream.",
    );
    add(
        "World Nodes",
        "world modeling",
        if world_count > 0 { "live" } else { "partial" },
        if world_count > 0 { 0.70 } else { 0.48 },
        vec!["NodeType::World", "POST /nodes supports world"],
        vec!["Add node type selector", "Graph world color"],
        "Portal ingestion is still not wired into automatic World creation.",
    );
    add(
        "Focus Trail",
        "temporal memory",
        if focus_count > 0 { "live" } else { "partial" },
        if focus_count > 0 { 0.82 } else { 0.62 },
        vec!["focus_trail recording", "GET /trail"],
        vec!["Trail panel", "graph moving trail links"],
        "The path is visible, but replay/scrubbing is not implemented.",
    );
    add(
        "Tectonic Visualization",
        "system weather",
        "live",
        0.76,
        vec!["GET /tectonics", "stress node calculation"],
        vec!["Tectonics metric", "Live Pulse", "hero status"],
        "Tectonic pressure is rendered as dashboard intelligence, not yet as graph terrain deformation.",
    );
    add(
        "Project Soul",
        "identity and atmosphere",
        "live",
        0.72,
        vec!["derive_souls()", "GET /souls"],
        vec!["Souls panel", "graph soul color mapping"],
        "Soul colors affect nodes; deeper per-project atmosphere controls are still partial.",
    );
    add(
        "Ghost Mode",
        "reflection mode",
        "live",
        0.70,
        vec!["ghost/fossil/void flags", "GET /modes"],
        vec!["Modes panel", "Deep Focus overlay", "graph ghost styling"],
        "Ghost mode is detected and surfaced, but not a persistent user-selected mode yet.",
    );
    add(
        "Dream Mode",
        "creative synthesis",
        "partial",
        0.66,
        vec!["DreamEngine", "GET /dream/proposals", "POST /synthesize"],
        vec!["Dream space atmosphere", "Dream proposal panel"],
        "The atmosphere is improved, but dream incubation is still proposal-driven rather than an immersive workspace.",
    );
    add(
        "Ambient Sound Architecture",
        "atmosphere",
        "stub",
        0.36,
        vec!["GET /atmospheres exposes audio signatures"],
        vec!["Modes/atmosphere cards"],
        "Audio signatures are data only; browser audio playback and cpal runtime are not wired.",
    );
    add(
        "Memory Atmospheres",
        "atmosphere",
        "partial",
        0.58,
        vec!["GET /atmospheres derives research/creative/technical/personal intensities"],
        vec!["System Modes view"],
        "Atmospheres are inferred and visual; audio plus per-region environment switching is not complete.",
    );
    add(
        "The Forge",
        "artifact creation",
        "partial",
        0.62,
        vec!["Forge endpoints", "GET /forge/genealogy"],
        vec!["Forge panel", "Lineage panel"],
        "Artifact genealogy is derived from graph edges, not stored as a first-class artifact lineage model.",
    );
    add(
        "Deep Focus Mode",
        "attention environment",
        "partial",
        0.60,
        vec!["GET /modes"],
        vec!["Deep Focus overlay"],
        "Minimal visual overlay exists; distraction blocking and ambient sound are not complete.",
    );
    add(
        "Living Terminal",
        "project integration",
        if process_count > 0 { "partial" } else { "stub" },
        if process_count > 0 { 0.56 } else { 0.42 },
        vec!["process scanner", "process links", "GET /terminal/context"],
        vec!["Terminal panel"],
        "Terminal is contextual/observational only; command execution remains outside SilentNode.",
    );
    add(
        "System Modes",
        "operating model",
        "partial",
        0.64,
        vec!["GET /modes derives Builder/Researcher/Ghost/Memory"],
        vec!["Modes panel"],
        "Modes are inferred each request; there is no persisted mode switch controlling all subsystems.",
    );
    add(
        "Life Constellations",
        "personal memory graph",
        "partial",
        0.60,
        vec!["GET /constellations"],
        vec!["Life panel"],
        "People/memories/goals are grouped from graph structure, not a dedicated constellation editing layer.",
    );
    add(
        "Local-first persistence",
        "data safety",
        if node_count > 0 || journal_count > 0 {
            "live"
        } else {
            "partial"
        },
        0.78,
        vec!["SQLite workspace store", "API autosave"],
        vec!["offline/local status"],
        "Autosave works through the API server; encrypted-by-default storage is not complete.",
    );

    let total = items.len().max(1) as f32;
    let score = items
        .iter()
        .map(|item| match item.status.as_str() {
            "live" => 1.0,
            "partial" => 0.55,
            "stub" => 0.25,
            _ => 0.0,
        })
        .sum::<f32>()
        / total;

    Json(VisionCoverageResponse {
        generated_from: "docs/vision.md plus current backend/web audit; NightOS and AI-provider items intentionally excluded".into(),
        summary: format!("{} coverage items tracked; {:.0}% are live or meaningfully partial.", items.len(), score * 100.0),
        completion_ratio: score,
        items,
    })
}

// ── Extended handlers ──────────────────────────────────────────────────────────

async fn get_archaeology(
    State(ws): State<SharedWorkspace>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let ws = ws.read().await;
    let node_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError("invalid UUID".into(), StatusCode::BAD_REQUEST))?;
    let Some(session) = ws.open_archaeology(node_id) else {
        let node = ws
            .graph
            .get_node(node_id)
            .ok_or_else(|| ApiError("node not found".into(), StatusCode::NOT_FOUND))?;

        return Ok(Json(ArchaeologyResponse {
            node_id: node_id.to_string(),
            snapshot_count: 0,
            cursor: 0,
            current_timestamp: node.accessed_at.to_rfc3339(),
            current_content: node.content.clone(),
            current_entropy: node.entropy,
            timeline: Vec::new(),
        }));
    };

    let timeline = session
        .timeline()
        .iter()
        .map(|(idx, ts, ct)| ArchaeologyTimelineEntry {
            index: *idx,
            timestamp: ts.to_rfc3339(),
            change_type: format!("{:?}", ct),
        })
        .collect();

    let current = session.resurrect();
    Ok(Json(ArchaeologyResponse {
        node_id: node_id.to_string(),
        snapshot_count: session.depth(),
        cursor: session.cursor(),
        current_timestamp: session.current_timestamp().to_rfc3339(),
        current_content: current.content,
        current_entropy: current.entropy,
        timeline,
    }))
}

async fn post_archaeology_resurrect(
    State(ws): State<SharedWorkspace>,
    Path((id, index)): Path<(String, usize)>,
) -> ApiResult<impl IntoResponse> {
    let ws = ws.read().await;
    let node_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError("invalid UUID".into(), StatusCode::BAD_REQUEST))?;
    let mut session = ws.open_archaeology(node_id).ok_or_else(|| {
        ApiError(
            "no temporal history for this node".into(),
            StatusCode::NOT_FOUND,
        )
    })?;
    session.seek(index);
    let node = session.resurrect();
    Ok(Json(serde_json::json!({
        "node_id": node_id.to_string(),
        "snapshot_index": index,
        "timestamp": session.current_timestamp().to_rfc3339(),
        "content": node.content,
        "entropy": node.entropy,
        "gravity": node.gravity,
        "is_ghost": node.is_ghost,
        "is_fossil": node.is_fossil,
    })))
}

async fn get_temporal_day(
    State(ws): State<SharedWorkspace>,
    Path(date_str): Path<String>,
) -> ApiResult<impl IntoResponse> {
    use chrono::NaiveDate;
    let ws = ws.read().await;
    let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").map_err(|_| {
        ApiError(
            "invalid date format (YYYY-MM-DD)".into(),
            StatusCode::BAD_REQUEST,
        )
    })?;
    let rec = ws.reconstruct_day(date);
    let dominant_preview = rec
        .dominant_node
        .and_then(|id| ws.graph.get_node(id))
        .map(|n| n.content.chars().take(40).collect::<String>())
        .unwrap_or_default();
    Ok(Json(DayReconstructionResponse {
        date: rec.date.to_string(),
        nodes_touched: rec.nodes_touched,
        total_focus_seconds: rec.total_focus_seconds,
        journal_entries_count: rec.journal_entries.len(),
        dominant_node_id: rec.dominant_node.map(|id| id.to_string()),
        dominant_node_preview: dominant_preview,
        aura_state: rec.aura_signature.state_name.clone(),
        aura_intensity: rec.aura_signature.intensity,
        primary_color: rec.aura_signature.primary_color,
        focus_events_count: rec.focus_events.len(),
    }))
}

async fn get_temporal_compare(
    State(ws): State<SharedWorkspace>,
    Query(q): Query<TemporalDayQuery>,
) -> ApiResult<impl IntoResponse> {
    use chrono::NaiveDate;
    let ws = ws.read().await;
    let a_str = q
        .from
        .ok_or_else(|| ApiError("from required".into(), StatusCode::BAD_REQUEST))?;
    let b_str =
        q.to.ok_or_else(|| ApiError("to required".into(), StatusCode::BAD_REQUEST))?;
    let day_a = NaiveDate::parse_from_str(&a_str, "%Y-%m-%d")
        .map_err(|_| ApiError("invalid from date".into(), StatusCode::BAD_REQUEST))?;
    let day_b = NaiveDate::parse_from_str(&b_str, "%Y-%m-%d")
        .map_err(|_| ApiError("invalid to date".into(), StatusCode::BAD_REQUEST))?;
    let cmp = ws.compare_days(day_a, day_b);

    let entries = vec![
        DayComparisonEntry {
            field: "new_nodes".into(),
            day_a_value: "—".into(),
            day_b_value: cmp.new_nodes.len().to_string(),
        },
        DayComparisonEntry {
            field: "removed_nodes".into(),
            day_a_value: cmp.removed_nodes.len().to_string(),
            day_b_value: "—".into(),
        },
        DayComparisonEntry {
            field: "changed_nodes".into(),
            day_a_value: "—".into(),
            day_b_value: cmp.changed_nodes.len().to_string(),
        },
        DayComparisonEntry {
            field: "focus_delta_seconds".into(),
            day_a_value: "—".into(),
            day_b_value: format!("{:+.0}", cmp.focus_delta_seconds),
        },
        DayComparisonEntry {
            field: "journal_entry_delta".into(),
            day_a_value: "—".into(),
            day_b_value: format!("{:+}", cmp.journal_entry_delta),
        },
    ];
    Ok(Json(DayComparisonResponse {
        day_a: a_str,
        day_b: b_str,
        entries,
    }))
}

async fn post_temporal_snapshot(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let mut ws = ws.write().await;
    ws.snapshot_all_nodes();
    let count = ws.temporal_snapshot_count();
    Json(serde_json::json!({ "ok": true, "total_snapshots": count }))
}

async fn get_void_zones(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    use chrono::Utc;
    let ws = ws.read().await;
    let now = Utc::now();
    let mut void_nodes: Vec<VoidZoneNodeResponse> = Vec::new();
    for zone in &ws.void_zones {
        for node_id in &zone.entities {
            if let Some(n) = ws.graph.get_node(*node_id) {
                let check = ws.check_void_emergence(n.id);
                let readiness = check
                    .as_ref()
                    .map(|c| c.resonance_score)
                    .unwrap_or(zone.resonance_readiness);
                void_nodes.push(VoidZoneNodeResponse {
                    node_id: n.id.to_string(),
                    content_preview: n.content.chars().take(50).collect(),
                    incubation_days: zone.incubation_days(now),
                    is_mature: zone.is_mature(now, 7.0)
                        || check.as_ref().map(|c| c.emergence_likely).unwrap_or(false),
                    resonance_readiness: readiness,
                    entropy: n.entropy,
                });
            }
        }
    }
    for n in ws.graph.nodes().filter(|n| n.is_void) {
        if void_nodes.iter().any(|v| v.node_id == n.id.to_string()) {
            continue;
        }
        let check = ws.check_void_emergence(n.id);
        let incubation = (now - n.accessed_at).num_seconds().max(0) as f32 / 86400.0;
        void_nodes.push(VoidZoneNodeResponse {
            node_id: n.id.to_string(),
            content_preview: n.content.chars().take(50).collect(),
            incubation_days: incubation,
            is_mature: check.as_ref().map(|c| c.emergence_likely).unwrap_or(false),
            resonance_readiness: check.as_ref().map(|c| c.resonance_score).unwrap_or(0.0),
            entropy: n.entropy,
        });
    }
    Json(void_nodes)
}

async fn post_fulfill_contract(
    State(ws): State<SharedWorkspace>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let mut ws = ws.write().await;
    let node_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError("invalid UUID".into(), StatusCode::BAD_REQUEST))?;
    ws.fulfill_contract(node_id)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn post_release_contract(
    State(ws): State<SharedWorkspace>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let mut ws = ws.write().await;
    let node_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError("invalid UUID".into(), StatusCode::BAD_REQUEST))?;
    ws.release_contract(node_id)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn post_crystallize_civ(
    State(ws): State<SharedWorkspace>,
    Path(civ_id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let ws = ws.read().await;
    let cid = Uuid::parse_str(&civ_id)
        .map_err(|_| ApiError("invalid UUID".into(), StatusCode::BAD_REQUEST))?;
    let civs = ws.detect_civilizations();
    let civ = civs
        .iter()
        .find(|c| c.id == cid)
        .ok_or_else(|| ApiError("civilization not found".into(), StatusCode::NOT_FOUND))?;
    let check = ws.check_crystallization(civ);
    if !check.qualifies {
        return Ok(Json(CrystallizationCheckResponse {
            civ_id: civ_id.clone(),
            qualifies: false,
            internal_density: check.internal_density,
            stability_score: check.stability_score,
            size: check.size,
            crystal_id: None,
        }));
    }
    let crystal = ws.crystallize_civilization(civ);
    Ok(Json(CrystallizationCheckResponse {
        civ_id: civ_id.clone(),
        qualifies: true,
        internal_density: check.internal_density,
        stability_score: check.stability_score,
        size: check.size,
        crystal_id: Some(crystal.id.to_string()),
    }))
}

async fn get_resonance_chambers(
    State(ws): State<SharedWorkspace>,
    Query(q): Query<ResonanceChamberQuery>,
) -> impl IntoResponse {
    let ws = ws.read().await;
    let threshold = q.threshold.unwrap_or(0.5);
    let chambers = ws.open_resonance_chambers(threshold);
    let resp: Vec<ResonanceChamberResponse> = chambers
        .iter()
        .map(|ch| {
            let pa = ws
                .graph
                .get_node(ch.node_a)
                .map(|n| n.content.chars().take(40).collect())
                .unwrap_or_default();
            let pb = ws
                .graph
                .get_node(ch.node_b)
                .map(|n| n.content.chars().take(40).collect())
                .unwrap_or_default();
            let state = format!("{:?}", ch.state);
            ResonanceChamberResponse {
                id: ch.id.to_string(),
                node_a: ch.node_a.to_string(),
                preview_a: pa,
                node_b: ch.node_b.to_string(),
                preview_b: pb,
                similarity: ch.similarity,
                state,
            }
        })
        .collect();
    Json(resp)
}

async fn post_illuminate_shadow(
    State(ws): State<SharedWorkspace>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let mut ws = ws.write().await;
    let node_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError("invalid UUID".into(), StatusCode::BAD_REQUEST))?;
    ws.illuminate_shadow(node_id)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn post_name_shadow(
    State(ws): State<SharedWorkspace>,
    Path(id): Path<String>,
    Json(req): Json<NameShadowRequest>,
) -> ApiResult<impl IntoResponse> {
    let mut ws = ws.write().await;
    let node_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError("invalid UUID".into(), StatusCode::BAD_REQUEST))?;
    ws.name_shadow(node_id, Some(req.name))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn post_release_shadow(
    State(ws): State<SharedWorkspace>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let mut ws = ws.write().await;
    let node_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError("invalid UUID".into(), StatusCode::BAD_REQUEST))?;
    ws.release_shadow(node_id)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_calendar(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    use chrono::Utc;
    let ws = ws.read().await;
    let now = Utc::now();
    let events: Vec<CalendarEventResponse> = ws
        .calendar
        .all_events()
        .iter()
        .map(|e| CalendarEventResponse {
            id: e.id.to_string(),
            title: e.title.clone(),
            description: e.description.clone(),
            category: e.category.as_str().to_string(),
            start_at: e.start_at.to_rfc3339(),
            end_at: Some(e.end_at.to_rfc3339()),
            linked_node_id: e.linked_nodes.first().map(|id| id.to_string()),
            computed_gravity: e.computed_gravity(now),
            hours_until: e.hours_until(now),
            is_approaching: e.is_approaching(now, 24.0),
        })
        .collect();
    Json(events)
}

async fn post_calendar_event(
    State(app): State<AppState>,
    Json(req): Json<AddCalendarEventRequest>,
) -> ApiResult<impl IntoResponse> {
    use crate::calendar::{CalendarEvent, EventCategory};
    use chrono::DateTime;
    let start = DateTime::parse_from_rfc3339(&req.start_at)
        .map(|t| t.with_timezone(&chrono::Utc))
        .map_err(|_| ApiError("invalid start_at date".into(), StatusCode::BAD_REQUEST))?;
    let end = req
        .end_at
        .as_deref()
        .and_then(|s| {
            DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|t| t.with_timezone(&chrono::Utc))
        })
        .unwrap_or_else(|| start + chrono::Duration::hours(1));
    let category = match req.category.as_deref().unwrap_or("meeting") {
        "deadline" => EventCategory::Deadline,
        "task" => EventCategory::Task,
        "review" => EventCategory::Review,
        "personal" => EventCategory::Personal,
        "recurring" => EventCategory::Recurring,
        "milestone" => EventCategory::Milestone,
        _ => EventCategory::Meeting,
    };
    let mut event = CalendarEvent::new(req.title, category, start, end);
    if let Some(desc) = req.description {
        event.description = desc;
    }
    if let Some(id_str) = req.linked_node_id {
        if let Ok(nid) = Uuid::parse_str(&id_str) {
            event.linked_nodes.push(nid);
        }
    }
    let id = event.id.to_string();
    let snapshot = {
        let mut ws = app.workspace.write().await;
        ws.calendar.add_event(event);
        ws.snapshot()
    };
    save_current_snapshot(&app, snapshot, "calendar").await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": id }))))
}

async fn delete_calendar_event(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let ev_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError("invalid UUID".into(), StatusCode::BAD_REQUEST))?;
    let snapshot = {
        let mut ws = app.workspace.write().await;
        ws.calendar.remove_event(ev_id);
        ws.snapshot()
    };
    save_current_snapshot(&app, snapshot, "calendar").await?;
    Ok(StatusCode::NO_CONTENT)
}

fn task_response(node: &crate::domain::NodeData) -> Option<DailyTaskResponse> {
    if node.metadata.get("source_kind").and_then(|v| v.as_str()) != Some("task") {
        return None;
    }
    let tags = node
        .metadata
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str().map(str::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Some(DailyTaskResponse {
        id: node.id.to_string(),
        node_id: node.id.to_string(),
        title: node.content.clone(),
        status: node
            .metadata
            .get("task_status")
            .and_then(|v| v.as_str())
            .unwrap_or("todo")
            .to_string(),
        date: node
            .metadata
            .get("date")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        tags,
        source: node
            .metadata
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("silentnode")
            .to_string(),
        calendar_event_id: node
            .metadata
            .get("calendar_event_id")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        gravity: node.gravity,
        entropy: node.entropy,
    })
}

async fn get_tasks(
    State(ws): State<SharedWorkspace>,
    Query(query): Query<TaskQuery>,
) -> impl IntoResponse {
    let ws = ws.read().await;
    let mut tasks: Vec<_> = ws
        .graph
        .nodes()
        .filter_map(task_response)
        .filter(|task| {
            query
                .date
                .as_ref()
                .map(|date| task.date.as_deref() == Some(date.as_str()))
                .unwrap_or(true)
        })
        .collect();
    tasks.sort_by(|a, b| {
        a.date
            .cmp(&b.date)
            .then_with(|| a.status.cmp(&b.status))
            .then_with(|| b.gravity.total_cmp(&a.gravity))
    });
    Json(tasks)
}

async fn post_task(
    State(app): State<AppState>,
    Json(req): Json<AddDailyTaskRequest>,
) -> ApiResult<impl IntoResponse> {
    use crate::calendar::{CalendarEvent, EventCategory};
    use crate::domain::{EdgeType, NodeData};
    use chrono::{DateTime, NaiveDate, NaiveTime, Utc};

    let title = req.title.trim();
    if title.is_empty() {
        return Err(ApiError("title required".into(), StatusCode::BAD_REQUEST));
    }
    let date = req
        .date
        .as_deref()
        .map(|raw| {
            NaiveDate::parse_from_str(raw, "%Y-%m-%d")
                .map_err(|_| ApiError("date must be YYYY-MM-DD".into(), StatusCode::BAD_REQUEST))
        })
        .transpose()?
        .unwrap_or_else(|| Utc::now().date_naive());
    let tags = req
        .tags
        .unwrap_or_default()
        .into_iter()
        .map(|tag| tag.trim().trim_start_matches('#').to_lowercase())
        .filter(|tag| !tag.is_empty())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let due_at = req
        .due_at
        .as_deref()
        .and_then(|raw| {
            DateTime::parse_from_rfc3339(raw)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
        })
        .unwrap_or_else(|| {
            let time = NaiveTime::from_hms_opt(12, 0, 0).unwrap();
            DateTime::<Utc>::from_naive_utc_and_offset(date.and_time(time), Utc)
        });
    let end_at = due_at + chrono::Duration::minutes(30);

    let snapshot = {
        let mut ws = app.workspace.write().await;
        let mut node = NodeData::new(NodeType::Process, title.to_string());
        node.gravity = 1.8;
        node.entropy = 0.03;
        node.aura_color = "#2dd4bf".into();
        node.metadata = metadata(&[
            ("source", Value::String("silentnode".into())),
            ("source_kind", Value::String("task".into())),
            (
                "source_key",
                Value::String(format!("silentnode:task:{}", node.id)),
            ),
            ("task_status", Value::String("todo".into())),
            ("date", Value::String(date.to_string())),
            (
                "tags",
                Value::Array(tags.iter().cloned().map(Value::String).collect()),
            ),
            (
                "notes",
                req.notes.clone().map(Value::String).unwrap_or(Value::Null),
            ),
        ]);
        let task_id = ws.graph.add_node(node)?;

        let mut event = CalendarEvent::new(title.to_string(), EventCategory::Task, due_at, end_at);
        event.description = req.notes.unwrap_or_default();
        event.linked_nodes.push(task_id);
        event.anticipation_days = 1;
        let calendar_event_id = event.id;
        ws.calendar.add_event(event);

        if let Some(task_node) = ws.graph.get_node_mut(task_id) {
            task_node.metadata.insert(
                "calendar_event_id".into(),
                Value::String(calendar_event_id.to_string()),
            );
        }

        for tag in &tags {
            let key = format!("silentnode:tag:{tag}");
            let tag_id = if let Some(existing) = find_node_by_source_key(&ws, &key) {
                existing
            } else {
                let mut tag_node = NodeData::new(NodeType::Project, format!("#{tag}"));
                tag_node.gravity = 1.2;
                tag_node.metadata = metadata(&[
                    ("source", Value::String("silentnode".into())),
                    ("source_kind", Value::String("tag".into())),
                    ("source_key", Value::String(key)),
                ]);
                ws.graph.add_node(tag_node)?
            };
            let _ = ws.connect_nodes(task_id, tag_id, EdgeType::Resonance, 0.7);
        }

        let day_key = format!("silentnode:day:{date}");
        let day_id = if let Some(existing) = find_node_by_source_key(&ws, &day_key) {
            existing
        } else {
            let mut day_node = NodeData::new(NodeType::Memory, format!("[Daily] {date}"));
            day_node.gravity = 1.2;
            day_node.metadata = metadata(&[
                ("source", Value::String("silentnode".into())),
                ("source_kind", Value::String("daily_note".into())),
                ("source_key", Value::String(day_key)),
                ("date", Value::String(date.to_string())),
            ]);
            ws.graph.add_node(day_node)?
        };
        let _ = ws.connect_nodes(day_id, task_id, EdgeType::Temporal, 0.9);

        let response = ws
            .graph
            .get_node(task_id)
            .and_then(task_response)
            .ok_or_else(|| {
                ApiError(
                    "task creation failed".into(),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?;
        (ws.snapshot(), response)
    };
    let (snapshot, response) = snapshot;
    save_current_snapshot(&app, snapshot, "task").await?;
    Ok((StatusCode::CREATED, Json(response)))
}

async fn post_task_complete(
    State(app): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<CompleteDailyTaskRequest>,
) -> ApiResult<impl IntoResponse> {
    let node_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError("invalid UUID".into(), StatusCode::BAD_REQUEST))?;
    let snapshot = {
        let mut ws = app.workspace.write().await;
        let node = ws
            .graph
            .get_node_mut(node_id)
            .ok_or_else(|| ApiError("task not found".into(), StatusCode::NOT_FOUND))?;
        if node.metadata.get("source_kind").and_then(|v| v.as_str()) != Some("task") {
            return Err(ApiError(
                "node is not a task".into(),
                StatusCode::BAD_REQUEST,
            ));
        }
        node.metadata.insert(
            "task_status".into(),
            Value::String(if req.done { "done" } else { "todo" }.into()),
        );
        node.entropy = if req.done { 0.55 } else { 0.03 };
        node.gravity = if req.done { 0.8 } else { 1.8 };
        ws.snapshot()
    };
    save_current_snapshot(&app, snapshot, "task").await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_calendar_focus_windows(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    use chrono::Utc;
    let ws = ws.read().await;
    let now = Utc::now();
    let upcoming = ws.calendar.upcoming(now);
    let suggestions = ws.calendar.intelligence.suggest_focus_windows(
        ws.focus.events(),
        &upcoming.iter().map(|e| *e).cloned().collect::<Vec<_>>(),
        now,
    );
    let windows: Vec<FocusWindowResponse> = suggestions
        .into_iter()
        .map(|(_, hour, reason)| FocusWindowResponse {
            start_hour: hour,
            end_hour: (hour + 2).min(23),
            score: 0.75,
            reason,
        })
        .collect();
    Json(windows)
}

async fn get_membrane(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let rules: Vec<MembraneRuleResponse> = ws
        .membrane
        .rules
        .iter()
        .map(|r| {
            let dir = match r.direction {
                crate::membrane::CrossingDirection::Inbound => "inbound",
                crate::membrane::CrossingDirection::Outbound => "outbound",
                crate::membrane::CrossingDirection::Both => "both",
            };
            MembraneRuleResponse {
                id: r.id.to_string(),
                pattern: r.pattern.clone(),
                direction: dir.to_string(),
                allow: r.allow,
                description: r.description.clone(),
            }
        })
        .collect();
    Json(MembraneStatusResponse {
        integrity_score: ws.membrane.integrity_score(),
        rule_count: rules.len(),
        blocked_count: ws.membrane.blocked_count(),
        rules,
    })
}

async fn post_membrane_rule(
    State(ws): State<SharedWorkspace>,
    Json(req): Json<AddMembraneRuleRequest>,
) -> impl IntoResponse {
    use crate::membrane::{CrossingDirection, MembraneRule};
    let mut ws = ws.write().await;
    let dir = match req.direction.as_deref().unwrap_or("both") {
        "inbound" => CrossingDirection::Inbound,
        "outbound" => CrossingDirection::Outbound,
        _ => CrossingDirection::Both,
    };
    let allow = req.allow.unwrap_or(true);
    let mut rule = if allow {
        MembraneRule::allow(req.pattern, dir)
    } else {
        MembraneRule::block(req.pattern, dir)
    };
    if let Some(desc) = req.description {
        rule = rule.with_description(desc);
    }
    let id = rule.id.to_string();
    ws.membrane.add_rule(rule);
    (StatusCode::CREATED, Json(serde_json::json!({ "id": id })))
}

async fn delete_membrane_rule(
    State(ws): State<SharedWorkspace>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let mut ws = ws.write().await;
    let rule_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError("invalid UUID".into(), StatusCode::BAD_REQUEST))?;
    ws.membrane.remove_rule(rule_id);
    Ok(StatusCode::NO_CONTENT)
}

async fn get_processes(State(ws): State<SharedWorkspace>) -> impl IntoResponse {
    let ws = ws.read().await;
    let procs = ws.processes.scan();
    let resp: Vec<ProcessResponse> = procs
        .iter()
        .map(|p| {
            let linked = ws.processes.linked_node(p.pid).map(|id| id.to_string());
            ProcessResponse {
                pid: p.pid,
                name: p.name.clone(),
                command: p.command.clone(),
                cpu_usage: p.cpu_usage,
                memory_mb: p.memory_mb,
                uptime_seconds: p.uptime_seconds,
                status: p.status.as_str().to_string(),
                linked_node_id: linked,
            }
        })
        .collect();
    Json(resp)
}

async fn post_process_link(
    State(ws): State<SharedWorkspace>,
    Path(pid): Path<i64>,
    Json(req): Json<LinkProcessRequest>,
) -> ApiResult<impl IntoResponse> {
    let mut ws = ws.write().await;
    let node_id = Uuid::parse_str(&req.node_id)
        .map_err(|_| ApiError("invalid UUID".into(), StatusCode::BAD_REQUEST))?;
    ws.processes.link_to_node(pid, node_id);
    Ok(StatusCode::NO_CONTENT)
}

// fix synthesis: return narrative + related_nodes
async fn post_synthesize_v2(
    State(ws): State<SharedWorkspace>,
    Json(req): Json<SynthesizeRequest>,
) -> impl IntoResponse {
    let ws = ws.read().await;
    let narrative = SynthesisEngine::new().synthesize_topic(&ws, &req.query);
    let nodes = ws
        .graph
        .nodes()
        .filter(|n| {
            let q = req.query.to_lowercase();
            n.content.to_lowercase().contains(&q)
        })
        .take(8)
        .map(|n| n.id.to_string())
        .collect::<Vec<_>>();
    Json(serde_json::json!({ "narrative": narrative, "related_nodes": nodes }))
}

// ── Vault handlers ─────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct VaultListResponse {
    vaults: Vec<VaultEntry>,
    current: String,
}

#[derive(Deserialize)]
struct CreateVaultRequest {
    name: String,
    path: Option<String>,
}

#[derive(Deserialize)]
struct SwitchVaultRequest {
    name: String,
}

#[derive(Deserialize)]
struct ObsidianPreviewRequest {
    path: String,
    max_files: Option<usize>,
}

#[derive(Deserialize)]
struct ObsidianImportRequest {
    path: String,
    max_files: Option<usize>,
    include_completed: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
struct ObsidianTaskPreview {
    source_file: String,
    line: usize,
    text: String,
    completed: bool,
    tags: Vec<String>,
    date: Option<String>,
    duplicate: bool,
}

#[derive(Debug, Clone, Serialize)]
struct ObsidianPreviewResponse {
    vault_path: String,
    files_scanned: usize,
    tasks: Vec<ObsidianTaskPreview>,
    tags: Vec<String>,
    warnings: Vec<String>,
}

#[derive(Serialize)]
struct ObsidianImportResponse {
    files_scanned: usize,
    tasks_created: usize,
    tasks_skipped: usize,
    tag_nodes_created: usize,
    day_nodes_created: usize,
    edges_created: usize,
    warnings: Vec<String>,
}

async fn get_vaults(State(vaults): State<SharedVaultState>) -> impl IntoResponse {
    let reg = vaults.read().await;
    Json(VaultListResponse {
        vaults: reg.vaults.clone(),
        current: reg.current.clone(),
    })
}

async fn post_create_vault(
    State(app): State<AppState>,
    Json(req): Json<CreateVaultRequest>,
) -> impl IntoResponse {
    let name = req.name.trim().to_string();
    if name.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "name required"})),
        )
            .into_response();
    }
    let mut reg = app.vaults.write().await;
    if reg.vaults.iter().any(|v| v.name == name) {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"error": "vault already exists"})),
        )
            .into_response();
    }
    let path = req
        .path
        .unwrap_or_else(|| format!("data/{}.sqlite", name.to_lowercase().replace(' ', "_")));
    // ensure parent dir exists
    if let Some(parent) = PathBuf::from(&path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    reg.vaults.push(VaultEntry {
        name: name.clone(),
        path: path.clone(),
    });
    reg.save();
    (
        StatusCode::CREATED,
        Json(serde_json::json!({"name": name, "path": path})),
    )
        .into_response()
}

async fn post_switch_vault(
    State(app): State<AppState>,
    Json(req): Json<SwitchVaultRequest>,
) -> impl IntoResponse {
    let name = req.name.trim().to_string();
    let sqlite_path = {
        let reg = app.vaults.read().await;
        match reg.vaults.iter().find(|v| v.name == name) {
            Some(v) => PathBuf::from(&v.path),
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": "vault not found"})),
                )
                    .into_response()
            }
        }
    };
    // load workspace from target sqlite
    use crate::storage::{SqliteWorkspaceStore, WorkspaceStore};
    let new_ws = if sqlite_path.exists() {
        match SqliteWorkspaceStore::new(sqlite_path.clone()) {
            Ok(store) => match store.load_snapshot() {
                Ok(Some(snap)) => SilentNodeWorkspace::from_snapshot(snap).unwrap_or_default(),
                _ => SilentNodeWorkspace::new(),
            },
            Err(_) => SilentNodeWorkspace::new(),
        }
    } else {
        SilentNodeWorkspace::new()
    };
    {
        let mut ws = app.workspace.write().await;
        *ws = new_ws;
    }
    {
        let mut reg = app.vaults.write().await;
        reg.current = name.clone();
        reg.save();
    }
    Json(serde_json::json!({"switched_to": name})).into_response()
}

async fn delete_vault(State(app): State<AppState>, Path(name): Path<String>) -> impl IntoResponse {
    let mut reg = app.vaults.write().await;
    if reg.current == name {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "cannot delete active vault"})),
        )
            .into_response();
    }
    let before = reg.vaults.len();
    reg.vaults.retain(|v| v.name != name);
    if reg.vaults.len() == before {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "vault not found"})),
        )
            .into_response();
    }
    reg.save();
    Json(serde_json::json!({"deleted": name})).into_response()
}

fn normalize_obsidian_tag(raw: &str) -> String {
    raw.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '-' && c != '/')
        .trim_start_matches('#')
        .to_lowercase()
}

fn extract_obsidian_tags(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    for token in line.split_whitespace() {
        if let Some(rest) = token.strip_prefix('#') {
            let tag = normalize_obsidian_tag(rest);
            if !tag.is_empty() && !out.contains(&tag) {
                out.push(tag);
            }
        }
    }
    out
}

fn extract_obsidian_task(line: &str) -> Option<(bool, String)> {
    let trimmed = line.trim_start();
    let rest = trimmed
        .strip_prefix("- [")
        .or_else(|| trimmed.strip_prefix("* ["))?;
    let mut chars = rest.chars();
    let marker = chars.next()?;
    if chars.next()? != ']' {
        return None;
    }
    let completed = matches!(marker, 'x' | 'X');
    let text = chars.as_str().trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some((completed, text))
    }
}

fn extract_date_from_path(path: &StdPath) -> Option<String> {
    let name = path.file_stem()?.to_string_lossy();
    let bytes = name.as_bytes();
    if bytes.len() < 10 {
        return None;
    }
    for i in 0..=bytes.len() - 10 {
        let s = &name[i..i + 10];
        let valid = s.as_bytes().get(4) == Some(&b'-')
            && s.as_bytes().get(7) == Some(&b'-')
            && s.chars()
                .enumerate()
                .all(|(idx, ch)| idx == 4 || idx == 7 || ch.is_ascii_digit());
        if valid {
            return Some(s.to_string());
        }
    }
    None
}

fn collect_markdown_files(
    root: &StdPath,
    max_files: usize,
    warnings: &mut Vec<String>,
) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(path) = stack.pop() {
        if out.len() >= max_files {
            warnings.push(format!("scan limited to {max_files} markdown files"));
            break;
        }
        let entries = match std::fs::read_dir(&path) {
            Ok(entries) => entries,
            Err(err) => {
                warnings.push(format!("could not read {}: {err}", path.display()));
                continue;
            }
        };
        for entry in entries.flatten() {
            let p = entry.path();
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            if name.starts_with('.') || matches!(name, "node_modules" | ".git") {
                continue;
            }
            if p.is_dir() {
                stack.push(p);
            } else if p
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("md"))
            {
                out.push(p);
                if out.len() >= max_files {
                    break;
                }
            }
        }
    }
    out.sort();
    Ok(out)
}

fn obsidian_source_key(path: &StdPath, line: usize, text: &str) -> String {
    let input = format!("{}:{line}:{text}", path.to_string_lossy());
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    format!("obsidian:task:{:016x}", hasher.finish())
}

fn existing_source_keys(ws: &SilentNodeWorkspace) -> HashSet<String> {
    ws.graph
        .nodes()
        .filter_map(|node| {
            node.metadata
                .get("source_key")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
        .collect()
}

fn scan_obsidian_vault(
    ws: &SilentNodeWorkspace,
    root: &StdPath,
    max_files: usize,
) -> Result<ObsidianPreviewResponse, ApiError> {
    if !root.exists() || !root.is_dir() {
        return Err(ApiError(
            "path must be an existing directory".into(),
            StatusCode::BAD_REQUEST,
        ));
    }
    let mut warnings = Vec::new();
    let files = collect_markdown_files(root, max_files, &mut warnings)
        .map_err(|err| ApiError(format!("scan failed: {err}"), StatusCode::BAD_REQUEST))?;
    let existing = existing_source_keys(ws);
    let mut tags = HashSet::new();
    let mut tasks = Vec::new();

    for file in &files {
        let date = extract_date_from_path(file);
        let content = match std::fs::read_to_string(file) {
            Ok(content) => content,
            Err(err) => {
                warnings.push(format!("could not read {}: {err}", file.display()));
                continue;
            }
        };
        for (idx, line) in content.lines().enumerate() {
            if let Some((completed, text)) = extract_obsidian_task(line) {
                let line_tags = extract_obsidian_tags(line);
                for tag in &line_tags {
                    tags.insert(tag.clone());
                }
                let source_key = obsidian_source_key(file, idx + 1, &text);
                tasks.push(ObsidianTaskPreview {
                    source_file: file.to_string_lossy().into_owned(),
                    line: idx + 1,
                    text,
                    completed,
                    tags: line_tags,
                    date: date.clone(),
                    duplicate: existing.contains(&source_key),
                });
            } else {
                for tag in extract_obsidian_tags(line) {
                    tags.insert(tag);
                }
            }
        }
    }

    let mut tags = tags.into_iter().collect::<Vec<_>>();
    tags.sort();
    Ok(ObsidianPreviewResponse {
        vault_path: root.to_string_lossy().into_owned(),
        files_scanned: files.len(),
        tasks,
        tags,
        warnings,
    })
}

fn metadata(entries: &[(&str, Value)]) -> BTreeMap<String, Value> {
    entries
        .iter()
        .map(|(k, v)| ((*k).to_string(), v.clone()))
        .collect()
}

fn find_node_by_source_key(ws: &SilentNodeWorkspace, source_key: &str) -> Option<Uuid> {
    ws.graph
        .nodes()
        .find(|node| node.metadata.get("source_key").and_then(|v| v.as_str()) == Some(source_key))
        .map(|node| node.id)
}

async fn post_obsidian_preview(
    State(app): State<AppState>,
    Json(req): Json<ObsidianPreviewRequest>,
) -> ApiResult<impl IntoResponse> {
    let root = PathBuf::from(req.path.trim());
    let max_files = req.max_files.unwrap_or(500).clamp(1, 5000);
    let ws = app.workspace.read().await;
    Ok(Json(scan_obsidian_vault(&ws, &root, max_files)?))
}

async fn post_obsidian_import(
    State(app): State<AppState>,
    Json(req): Json<ObsidianImportRequest>,
) -> ApiResult<impl IntoResponse> {
    let root = PathBuf::from(req.path.trim());
    let max_files = req.max_files.unwrap_or(500).clamp(1, 5000);
    let include_completed = req.include_completed.unwrap_or(true);
    let preview = {
        let ws = app.workspace.read().await;
        scan_obsidian_vault(&ws, &root, max_files)?
    };

    let mut tag_nodes_created = 0;
    let mut day_nodes_created = 0;
    let mut tasks_created = 0;
    let mut tasks_skipped = 0;
    let mut edges_created = 0;
    let snapshot = {
        let mut ws = app.workspace.write().await;
        let mut tag_nodes: HashMap<String, Uuid> = HashMap::new();
        let mut day_nodes: HashMap<String, Uuid> = HashMap::new();

        for tag in &preview.tags {
            let key = format!("obsidian:tag:{tag}");
            let id = if let Some(id) = find_node_by_source_key(&ws, &key) {
                id
            } else {
                let mut node = crate::domain::NodeData::new(NodeType::Project, format!("#{tag}"));
                node.gravity = 1.4;
                node.metadata = metadata(&[
                    ("source", Value::String("obsidian".into())),
                    ("source_kind", Value::String("tag".into())),
                    ("source_key", Value::String(key)),
                ]);
                let id = ws.graph.add_node(node)?;
                tag_nodes_created += 1;
                id
            };
            tag_nodes.insert(tag.clone(), id);
        }

        for task in preview.tasks.iter().filter(|task| task.date.is_some()) {
            let date = task.date.clone().unwrap_or_default();
            let key = format!("obsidian:day:{date}");
            if day_nodes.contains_key(&date) {
                continue;
            }
            let id = if let Some(id) = find_node_by_source_key(&ws, &key) {
                id
            } else {
                let mut node =
                    crate::domain::NodeData::new(NodeType::Memory, format!("[Daily] {date}"));
                node.gravity = 1.2;
                node.metadata = metadata(&[
                    ("source", Value::String("obsidian".into())),
                    ("source_kind", Value::String("daily_note".into())),
                    ("source_key", Value::String(key)),
                    ("date", Value::String(date.clone())),
                ]);
                let id = ws.graph.add_node(node)?;
                day_nodes_created += 1;
                id
            };
            day_nodes.insert(date, id);
        }

        for task in &preview.tasks {
            if task.duplicate || (!include_completed && task.completed) {
                tasks_skipped += 1;
                continue;
            }
            let key = obsidian_source_key(StdPath::new(&task.source_file), task.line, &task.text);
            if find_node_by_source_key(&ws, &key).is_some() {
                tasks_skipped += 1;
                continue;
            }
            let mut node = crate::domain::NodeData::new(NodeType::Process, task.text.clone());
            node.gravity = if task.completed { 0.8 } else { 1.8 };
            node.entropy = if task.completed { 0.55 } else { 0.05 };
            node.metadata = metadata(&[
                ("source", Value::String("obsidian".into())),
                ("source_kind", Value::String("task".into())),
                ("source_key", Value::String(key)),
                ("source_file", Value::String(task.source_file.clone())),
                ("source_line", Value::Number((task.line as u64).into())),
                (
                    "task_status",
                    Value::String(if task.completed { "done" } else { "todo" }.into()),
                ),
                (
                    "date",
                    task.date.clone().map(Value::String).unwrap_or(Value::Null),
                ),
                (
                    "tags",
                    Value::Array(task.tags.iter().cloned().map(Value::String).collect()),
                ),
            ]);
            let task_id = ws.graph.add_node(node)?;
            tasks_created += 1;

            for tag in &task.tags {
                if let Some(tag_id) = tag_nodes.get(tag) {
                    ws.connect_nodes(task_id, *tag_id, crate::domain::EdgeType::Resonance, 0.7)?;
                    edges_created += 1;
                }
            }
            if let Some(date) = &task.date {
                if let Some(day_id) = day_nodes.get(date) {
                    ws.connect_nodes(*day_id, task_id, crate::domain::EdgeType::Temporal, 0.9)?;
                    edges_created += 1;
                }
            }
        }
        ws.snapshot()
    };

    let path = {
        let reg = app.vaults.read().await;
        reg.current_path()
    };
    {
        use crate::storage::WorkspaceStore;
        let mut store = crate::storage::SqliteWorkspaceStore::new(path).map_err(|err| {
            ApiError(
                format!("import save failed: {err}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;
        store.save_snapshot(&snapshot).map_err(|err| {
            ApiError(
                format!("import save failed: {err}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;
    }

    Ok(Json(ObsidianImportResponse {
        files_scanned: preview.files_scanned,
        tasks_created,
        tasks_skipped,
        tag_nodes_created,
        day_nodes_created,
        edges_created,
        warnings: preview.warnings,
    }))
}

// ── Router + server ────────────────────────────────────────────────────────────

pub fn build_router(app: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::PUT])
        .allow_headers(Any);

    Router::new()
        // status + graph shape
        .route("/status", get(get_status))
        .route(
            "/settings/notifications",
            get(get_notification_settings).put(put_notification_settings),
        )
        .route("/settings/notifications/test", post(post_notification_test))
        .route("/nodes", get(get_nodes).post(post_node))
        .route(
            "/nodes/bulk-delete",
            axum::routing::delete(delete_nodes_bulk),
        )
        .route(
            "/nodes/:id",
            get(get_node).delete(delete_node).put(put_node),
        )
        .route("/nodes/:id/related", get(get_related))
        .route("/nodes/:id/void", axum::routing::post(post_void_toggle))
        .route("/nodes/:id/fossilize", axum::routing::post(post_fossilize))
        .route("/nodes/:id/excavate", axum::routing::post(post_excavate))
        .route("/nodes/:id/revive", axum::routing::post(post_revive))
        .route("/edges", get(get_edges))
        .route("/connect", axum::routing::post(post_connect))
        // cognitive interaction
        .route("/thought", post(post_thought))
        .route("/focus", post(post_focus))
        .route("/journal", get(get_journal).post(post_journal))
        .route("/tasks", get(get_tasks).post(post_task))
        .route(
            "/tasks/:id/complete",
            axum::routing::post(post_task_complete),
        )
        // intelligence
        .route("/season", get(get_season))
        .route("/civilizations", get(get_civilizations))
        .route("/civilization-events", get(get_civilization_events))
        .route("/resonances", get(get_resonances))
        .route("/suggestions", get(get_suggestions))
        .route("/clusters", get(get_clusters))
        .route("/oracle", get(get_oracle))
        .route("/contracts", get(get_contracts))
        .route("/weight", get(get_weight))
        .route("/rituals", get(get_rituals))
        .route("/lore", get(get_lore))
        .route("/signature", get(get_signature))
        .route("/shadow-projects", get(get_shadow_projects))
        .route("/heatmap", get(get_heatmap))
        .route("/mirror", get(get_mirror))
        .route("/weather", get(get_weather))
        .route("/souls", get(get_souls))
        .route("/silence", get(get_silence))
        .route("/trail", get(get_trail))
        .route("/tectonics", get(get_tectonics))
        .route("/modes", get(get_modes).post(post_mode))
        .route("/atmospheres", get(get_atmospheres))
        .route("/forge/genealogy", get(get_forge_genealogy))
        .route("/terminal/context", get(get_terminal_context))
        .route("/constellations", get(get_constellations))
        .route("/vision/coverage", get(get_vision_coverage))
        // export
        .route("/export/dot", get(export_dot_handler))
        .route("/export/csv", get(export_csv_handler))
        .route("/export/edges-csv", get(export_edges_csv_handler))
        .route("/export/markdown", get(export_markdown_handler))
        .route("/dashboard", get(dashboard_handler))
        // analytics
        .route("/analytics", get(get_analytics_health))
        .route("/analytics/pagerank", get(get_analytics_pagerank))
        .route("/analytics/bridges", get(get_analytics_bridges))
        // dream + synthesis
        .route("/dream/proposals", get(get_dream_proposals))
        .route("/synthesize", post(post_synthesize_v2))
        .route("/synthesis/gaps", get(get_knowledge_gaps))
        .route("/synthesis/chain", post(post_thought_chain))
        // temporal / archaeology
        .route("/archaeology/:id", get(get_archaeology))
        .route(
            "/archaeology/:id/resurrect/:index",
            axum::routing::post(post_archaeology_resurrect),
        )
        .route("/temporal/day/:date", get(get_temporal_day))
        .route("/temporal/compare", get(get_temporal_compare))
        .route(
            "/temporal/snapshot",
            axum::routing::post(post_temporal_snapshot),
        )
        // void zones (enhanced)
        .route("/void-zones", get(get_void_zones))
        // contract actions
        .route(
            "/contracts/:id/fulfill",
            axum::routing::post(post_fulfill_contract),
        )
        .route(
            "/contracts/:id/release",
            axum::routing::post(post_release_contract),
        )
        // crystallization
        .route(
            "/crystallize/:civ_id",
            axum::routing::post(post_crystallize_civ),
        )
        // resonance chambers
        .route("/resonance-chambers", get(get_resonance_chambers))
        // shadow actions
        .route(
            "/shadows/:id/illuminate",
            axum::routing::post(post_illuminate_shadow),
        )
        .route("/shadows/:id/name", axum::routing::post(post_name_shadow))
        .route(
            "/shadows/:id/release",
            axum::routing::post(post_release_shadow),
        )
        // calendar
        .route("/calendar", get(get_calendar).post(post_calendar_event))
        .route(
            "/calendar/:id",
            axum::routing::delete(delete_calendar_event),
        )
        .route("/calendar/focus-windows", get(get_calendar_focus_windows))
        // membrane
        .route("/membrane", get(get_membrane))
        .route("/membrane/rules", axum::routing::post(post_membrane_rule))
        .route(
            "/membrane/rules/:id",
            axum::routing::delete(delete_membrane_rule),
        )
        // processes
        .route("/processes", get(get_processes))
        .route(
            "/processes/:pid/link",
            axum::routing::post(post_process_link),
        )
        // vault
        .route("/vaults", get(get_vaults).post(post_create_vault))
        .route("/vaults/switch", axum::routing::post(post_switch_vault))
        .route("/vaults/:name", axum::routing::delete(delete_vault))
        .route(
            "/obsidian/preview",
            axum::routing::post(post_obsidian_preview),
        )
        .route(
            "/obsidian/import",
            axum::routing::post(post_obsidian_import),
        )
        // ML endpoints
        .route("/ml/status", get(ml_status_handler))
        .route("/ml/classify", axum::routing::post(ml_classify_handler))
        .route("/ml/feedback", axum::routing::post(ml_feedback_handler))
        .route("/ml/ghost-risk", get(ml_ghost_risk_handler))
        .route("/ml/next-focus/:node_id", get(ml_next_focus_handler))
        .route("/ml/daily-plan", get(ml_daily_plan_handler))
        .route("/ml/diagnostics", get(ml_diagnostics_handler))
        .route("/ml/clusters", get(ml_clusters_handler))
        .route("/ml/train", axum::routing::post(ml_train_handler))
        .layer(cors)
        .with_state(app)
}

// ── ML handlers — Python CLI-yi çağırır ─────────────────────────────────────

fn run_ml_cli(args: &[&str]) -> Result<serde_json::Value, String> {
    let output = std::process::Command::new(python_executable())
        .args(["-m", "silentnode_py.ml.cli"])
        .args(args)
        .output()
        .map_err(|e| format!("python executable not found: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_json_from_output(&stdout).map_err(|e| {
        format!(
            "parse error: {e}\nraw: {}",
            &stdout[..stdout.len().min(200)]
        )
    })
}

fn python_executable() -> &'static str {
    if StdPath::new(".venv/bin/python").exists() {
        ".venv/bin/python"
    } else {
        "python3"
    }
}

fn parse_json_from_output(stdout: &str) -> Result<serde_json::Value, serde_json::Error> {
    let trimmed = stdout.trim();
    if let Ok(value) = serde_json::from_str(trimmed) {
        return Ok(value);
    }

    for (idx, ch) in trimmed.char_indices() {
        if ch == '{' || ch == '[' {
            if let Ok(value) = serde_json::from_str(&trimmed[idx..]) {
                return Ok(value);
            }
        }
    }

    serde_json::from_str(trimmed)
}

async fn ml_status_handler() -> impl IntoResponse {
    match run_ml_cli(&["status"]) {
        Ok(v) => (axum::http::StatusCode::OK, axum::Json(v)).into_response(),
        Err(e) => (
            axum::http::StatusCode::OK,
            axum::Json(serde_json::json!({"status":"not_trained","error":e})),
        )
            .into_response(),
    }
}

#[derive(serde::Deserialize)]
struct MlClassifyReq {
    content: String,
    nickname: Option<String>,
}

#[derive(serde::Deserialize)]
struct MlFeedbackReq {
    node_id: Option<String>,
    content: String,
    nickname: Option<String>,
    predicted_type: Option<String>,
    selected_type: String,
    confidence: Option<f32>,
    source: Option<String>,
}

async fn ml_classify_handler(axum::Json(req): axum::Json<MlClassifyReq>) -> impl IntoResponse {
    let text = req
        .nickname
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|nickname| format!("{nickname}\n{}", req.content))
        .unwrap_or(req.content);
    match run_ml_cli(&["classify", &text]) {
        Ok(v) => (axum::http::StatusCode::OK, axum::Json(v)).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({"error":e})),
        )
            .into_response(),
    }
}

async fn ml_feedback_handler(
    State(app): State<AppState>,
    axum::Json(req): axum::Json<MlFeedbackReq>,
) -> impl IntoResponse {
    let selected_type = req.selected_type.trim().to_lowercase();
    let content = req.content.trim().to_string();
    if selected_type.is_empty() || content.is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({"error":"content and selected_type required"})),
        )
            .into_response();
    }

    let path = {
        let reg = app.vaults.read().await;
        reg.current_path()
    };

    let result = (|| -> Result<(), String> {
        let conn = rusqlite::Connection::open(path).map_err(|e| e.to_string())?;
        conn.execute_batch(
            r#"
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
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO ml_feedback (
                id, node_id, content, nickname, predicted_type, selected_type, confidence, source, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                Uuid::new_v4().to_string(),
                req.node_id,
                content,
                req.nickname.and_then(|value| {
                    let trimmed = value.trim().to_string();
                    (!trimmed.is_empty()).then_some(trimmed)
                }),
                req.predicted_type.map(|value| value.to_lowercase()),
                selected_type,
                req.confidence.unwrap_or(0.0),
                req.source.unwrap_or_else(|| "add_node_dialog".to_string()),
                chrono::Utc::now().to_rfc3339(),
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })();

    match result {
        Ok(()) => (
            axum::http::StatusCode::CREATED,
            axum::Json(serde_json::json!({"ok":true})),
        )
            .into_response(),
        Err(error) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({"error":error})),
        )
            .into_response(),
    }
}

async fn ml_ghost_risk_handler() -> impl IntoResponse {
    match run_ml_cli(&["ghost-risk"]) {
        Ok(v) => (axum::http::StatusCode::OK, axum::Json(v)).into_response(),
        Err(_) => (
            axum::http::StatusCode::OK,
            axum::Json(serde_json::json!([])),
        )
            .into_response(),
    }
}

async fn ml_next_focus_handler(
    axum::extract::Path(node_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    match run_ml_cli(&["next-focus", &node_id]) {
        Ok(v) => (axum::http::StatusCode::OK, axum::Json(v)).into_response(),
        Err(_) => (
            axum::http::StatusCode::OK,
            axum::Json(serde_json::json!([])),
        )
            .into_response(),
    }
}

async fn ml_daily_plan_handler() -> impl IntoResponse {
    match run_ml_cli(&["daily-plan"]) {
        Ok(v) => (axum::http::StatusCode::OK, axum::Json(v)).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({"error":e})),
        )
            .into_response(),
    }
}

async fn ml_diagnostics_handler() -> impl IntoResponse {
    match run_ml_cli(&["diagnostics"]) {
        Ok(v) => (axum::http::StatusCode::OK, axum::Json(v)).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({"error":e})),
        )
            .into_response(),
    }
}

async fn ml_clusters_handler() -> impl IntoResponse {
    match run_ml_cli(&["clusters"]) {
        Ok(v) => (axum::http::StatusCode::OK, axum::Json(v)).into_response(),
        Err(_) => (
            axum::http::StatusCode::OK,
            axum::Json(serde_json::json!([])),
        )
            .into_response(),
    }
}

async fn ml_train_handler() -> impl IntoResponse {
    match run_ml_cli(&["train"]) {
        Ok(v) => (axum::http::StatusCode::OK, axum::Json(v)).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({"error":e})),
        )
            .into_response(),
    }
}

/// Start the REST API server. Blocks until shutdown.
pub async fn start_api_server(
    workspace: SilentNodeWorkspace,
    port: u16,
    sqlite_path: PathBuf,
) -> std::io::Result<()> {
    // vault registry lives next to the sqlite file
    let registry_path = sqlite_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("data"))
        .join("vaults.json");
    let registry = VaultRegistry::load_or_create(registry_path, &sqlite_path);
    let current_path = registry.current_path();
    let workspace = if current_path.exists() {
        use crate::storage::WorkspaceStore;
        match crate::storage::SqliteWorkspaceStore::new(current_path.clone())
            .and_then(|store| store.load_snapshot())
        {
            Ok(Some(snapshot)) => SilentNodeWorkspace::from_snapshot(snapshot).unwrap_or(workspace),
            _ => workspace,
        }
    } else {
        workspace
    };
    let shared: SharedWorkspace = Arc::new(RwLock::new(workspace));
    let vault_state: SharedVaultState = Arc::new(RwLock::new(registry));

    let app_state = AppState {
        workspace: shared.clone(),
        vaults: vault_state.clone(),
    };

    // autosave — always saves to the current vault's path
    {
        let autosave_ws = shared.clone();
        let autosave_vaults = vault_state.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(std::time::Duration::from_secs(3));
            loop {
                ticker.tick().await;
                let (snapshot, path) = {
                    let ws = autosave_ws.read().await;
                    let reg = autosave_vaults.read().await;
                    (ws.snapshot(), reg.current_path())
                };
                match crate::storage::SqliteWorkspaceStore::new(path) {
                    Ok(mut store) => {
                        use crate::storage::WorkspaceStore;
                        if let Err(err) = store.save_snapshot(&snapshot) {
                            eprintln!("[api] autosave failed: {err}");
                        }
                    }
                    Err(err) => eprintln!("[api] autosave store failed: {err}"),
                }
            }
        });
    }

    let router = build_router(app_state);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("[api] Listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await
}
