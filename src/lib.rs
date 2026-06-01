#[cfg(feature = "audio")]
pub mod audio;
// Phase 10 — Identity & Narrative Systems
pub mod identity;
// Phase 8 — External World Integration
pub mod analytics;
pub mod api;
pub mod calendar;
pub mod contagion;
pub mod dashboard;
pub mod domain;
pub mod dream;
pub mod entropy;
pub mod error;
pub mod export;
pub mod focus;
pub mod graph;
pub mod gravity;
pub mod intelligence;
pub mod journal;
pub mod materialize;
pub mod membrane;
pub mod migration;
pub mod portals;
pub mod process;
#[cfg(any(feature = "python", feature = "python-ext"))]
pub mod python;
pub mod renderer;
pub mod silence;
pub mod storage;
pub mod surreal;
pub mod sync;
pub mod synthesis;
pub mod systems;
pub mod temporal;
pub mod tui;
pub mod vault;
pub mod visualization;
pub mod workspace;

pub use analytics::{
    AnalyticsEngine, BridgeEdge, CentralityEntry, GraphHealthReport, PageRankEntry,
};
pub use api::{build_router, start_api_server, SharedWorkspace};
pub use contagion::ContagionEngine;
pub use dashboard::export_html_dashboard;
pub use domain::{
    ArcType, ChangeType, ContractState, EdgeData, EdgeType, FocusDepth, FocusEvent, GraphStats,
    JournalEntry, LoreEntry, NodeData, NodeType, Position3, ProcessRecord, SilentContract,
    TemporalSnapshot,
};
pub use dream::{DreamEngine, DreamProposal, ProposalKind};
pub use entropy::EntropyEngine;
pub use error::{GraphError, StorageError, SurrealStoreError, VaultError};
pub use export::{export_csv, export_dot, export_edges_csv, export_markdown};
pub use focus::FocusTrailEngine;
pub use graph::CognitiveGraph;
pub use gravity::GravityEngine;
pub use intelligence::{ContentCluster, FocusSuggestion, RelatedSuggestion, SuggestionEngine};
pub use journal::JournalEngine;
pub use materialize::{MaterializationEngine, MaterializationResult, SimilarNodeSuggestion};
pub use migration::{migrate_sqlite_to_surreal, migrate_sqlite_to_surreal_path};
pub use renderer::{
    edge_to_vertices, launch, Camera, CameraController, EdgePipeline, EdgeVertex, FrameStats,
    NodeInstance, NodePipeline, Particle, ParticleSystem, RenderConfig, RenderDevice,
};
pub use silence::{ImpliedConcept, MissingBridge, SilenceAnalyzer};
pub use storage::{
    GraphStore, InMemoryGraphStore, SqliteWorkspaceStore, StoredGraph, WorkspaceSnapshot,
    WorkspaceStore,
};
pub use surreal::{SurrealTableCounts, SurrealWorkspaceStore, SURREAL_SCHEMA};
pub use sync::{merge_snapshots, pull, push, serve};
pub use synthesis::SynthesisEngine;
pub use systems::{
    civilization_color,
    derive_all_souls,
    name_ritual,
    season_aura_colors,
    // Phase 6
    BlindSpot,
    ChamberState,
    // Phase 7
    CivEventKind,
    Civilization,
    CivilizationDetector,
    CivilizationEvent,
    CognitiveHeatmap,
    CognitiveMirror,
    CognitivePortrait,
    CognitiveSeason,
    CognitiveSeasonDetector,
    // Phase 4
    CognitiveWeightSystem,
    CreativePattern,
    CrystallizationCheck,
    CrystallizationEngine,
    DetectedContract,
    DigitalShadow,
    DigitalShadowDetector,
    EmergenceCheck,
    EvolutionEntry,
    GlowPattern,
    GraphSnapshot,
    HeatmapEntry,
    KnowledgeCrystal,
    NeglectedRegion,
    ObsessionEntry,
    ObsessiveLoop,
    OracleLayer,
    OracleSignal,
    OracleSignalKind,
    ParticleStyle,
    PriorityGap,
    ProjectSoul,
    ResonanceChamber,
    ResonanceChamberEngine,
    ResonancePair,
    Ritual,
    RitualEngine,
    SeasonReport,
    SilentContractDetector,
    TectonicDetector,
    TectonicEvent,
    ThoughtHeatmapEngine,
    VoidManager,
    VoidZone,
    WeatherState,
    WeatherSystem,
    WeightReport,
};
pub use temporal::{
    ArchaeologySession, DayComparison, DayReconstruction, FossilEngine, FossilizationCheck,
    LoreArcDetector, MemoryReconstructor, TemporalDiff, TemporalEngine, TemporalMarker,
};
pub use tui::run_tui;
pub use vault::Vault;
pub use visualization::VisualizationEngine;
pub use workspace::SilentNodeWorkspace;
// Phase 10 re-exports
pub use identity::{
    GeometryKind, IdentityEngine, LivingSignature, MotionKind, ShadowProject,
    ShadowProjectDetector, SignatureShift, SymmetryKind,
};
// Phase 8 re-exports
pub use calendar::{
    CalendarEngine, CalendarEvent, CalendarIntelligence, EventCategory, PreparationAnalysis,
};
pub use membrane::{
    CrossingDirection, CrossingEvent, DataType, DigitalMembrane, MembraneDecision, MembraneRule,
    Protocol,
};
pub use portals::{
    ActivityKind, ExternalPortal, FilesystemPortal, IngestionProposal, IngestionProposalKind,
    PortalActivity, PortalManager, PortalType,
};
pub use process::{ProcessActivityReport, ProcessSovereignty, ProcessStatus, RunningProcess};
