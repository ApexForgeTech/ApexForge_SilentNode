pub mod cognitive_weight;
pub mod souls;
pub mod tectonics;
pub mod weather;

// Phase 6 — Pattern Recognition
pub mod contracts;
pub mod heatmap;
pub mod mirror;
pub mod oracle;
pub mod resonance;
pub mod ritual;
pub mod seasons;

// Phase 7 — Social Graph
pub mod civilization;
pub mod crystallization;
pub mod shadow;
pub mod void_manager;

pub use cognitive_weight::{CognitiveWeightSystem, WeightReport};
pub use souls::{derive_all_souls, GlowPattern, ParticleStyle, ProjectSoul};
pub use tectonics::{GraphSnapshot, TectonicDetector, TectonicEvent};
pub use weather::{WeatherState, WeatherSystem};

// Phase 6 re-exports
pub use contracts::{DetectedContract, SilentContractDetector};
pub use heatmap::{
    CognitiveHeatmap, HeatmapEntry, NeglectedRegion, ObsessiveLoop, ThoughtHeatmapEngine,
};
pub use mirror::{
    BlindSpot, CognitiveMirror, CognitivePortrait, CreativePattern, EvolutionEntry, ObsessionEntry,
    PriorityGap,
};
pub use oracle::{OracleLayer, OracleSignal, OracleSignalKind};
pub use resonance::{ChamberState, ResonanceChamber, ResonanceChamberEngine, ResonancePair};
pub use ritual::{name_ritual, Ritual, RitualEngine};
pub use seasons::{season_aura_colors, CognitiveSeason, CognitiveSeasonDetector, SeasonReport};

// Phase 7 re-exports
pub use civilization::{
    civilization_color, CivEventKind, Civilization, CivilizationDetector, CivilizationEvent,
};
pub use crystallization::{CrystallizationCheck, CrystallizationEngine, KnowledgeCrystal};
pub use shadow::{DigitalShadow, DigitalShadowDetector};
pub use void_manager::{EmergenceCheck, VoidManager, VoidZone};
