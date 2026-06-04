use crate::contagion::ContagionEngine;
use crate::domain::{
    ChangeType, EdgeType, FocusDepth, FocusEvent, JournalEntry, LoreEntry, NodeData,
};
use crate::entropy::EntropyEngine;
use crate::focus::FocusTrailEngine;
use crate::graph::CognitiveGraph;
use crate::gravity::GravityEngine;
use crate::journal::JournalEngine;
use crate::materialize::{MaterializationEngine, MaterializationResult};
use crate::silence::{ImpliedConcept, MissingBridge, SilenceAnalyzer};
use crate::storage::{StoredGraph, WorkspaceSnapshot};
use crate::systems::{
    derive_all_souls, Civilization, CivilizationDetector, CivilizationEvent, CognitiveHeatmap,
    CognitiveMirror, CognitivePortrait, CognitiveSeasonDetector, CognitiveWeightSystem,
    CrystallizationCheck, CrystallizationEngine, DetectedContract, DigitalShadow,
    DigitalShadowDetector, EmergenceCheck, GraphSnapshot, KnowledgeCrystal, NeglectedRegion,
    ObsessiveLoop, OracleLayer, OracleSignal, ProjectSoul, ResonanceChamber,
    ResonanceChamberEngine, ResonancePair, Ritual, RitualEngine, SeasonReport,
    SilentContractDetector, TectonicDetector, TectonicEvent, ThoughtHeatmapEngine, VoidManager,
    VoidZone, WeatherSystem, WeightReport,
};
use crate::temporal::{
    ArchaeologySession, DayComparison, DayReconstruction, FossilEngine, FossilizationCheck,
    LoreArcDetector, MemoryReconstructor, TemporalEngine,
};
use crate::visualization::VisualizationEngine;
use crate::GraphError;
// Phase 8
use crate::calendar::CalendarEngine;
use crate::membrane::DigitalMembrane;
use crate::portals::PortalManager;
use crate::process::ProcessSovereignty;
// Phase 9
#[cfg(feature = "audio")]
use crate::audio::{atmosphere_from_entropy, atmosphere_from_season, AtmosphereKind, AudioEngine};
// Phase 10
use crate::identity::{IdentityEngine, LivingSignature, ShadowProject, ShadowProjectDetector};
use chrono::{Duration, NaiveDate, Utc};
use std::collections::HashSet;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SilentNodeWorkspace {
    pub graph: CognitiveGraph,
    pub focus: FocusTrailEngine,
    pub journal: JournalEngine,
    pub temporal: TemporalEngine,
    pub system_mode: Option<String>,
    // Phase 7 — Void Zone tracking
    pub void_zones: Vec<VoidZone>,
    // Phase 10 — Identity Engine
    pub identity: IdentityEngine,
    // Phase 8 — External World Integration
    pub membrane: DigitalMembrane,
    pub portals: PortalManager,
    pub calendar: CalendarEngine,
    pub processes: ProcessSovereignty,
}

impl Default for SilentNodeWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

impl SilentNodeWorkspace {
    pub fn new() -> Self {
        Self {
            graph: CognitiveGraph::new(),
            focus: FocusTrailEngine::new(),
            journal: JournalEngine::new(),
            temporal: TemporalEngine::new(),
            system_mode: None,
            void_zones: Vec::new(),
            identity: IdentityEngine::new(),
            membrane: DigitalMembrane::new(),
            portals: PortalManager::new(),
            calendar: CalendarEngine::new(),
            processes: ProcessSovereignty::new(),
        }
    }

    pub fn from_snapshot(snapshot: WorkspaceSnapshot) -> Result<Self, GraphError> {
        Ok(Self {
            graph: CognitiveGraph::from_parts(snapshot.graph.nodes, snapshot.graph.edges)?,
            focus: FocusTrailEngine::from_events(snapshot.focus_events),
            journal: JournalEngine::from_entries(snapshot.journal_entries),
            temporal: TemporalEngine::from_snapshots(snapshot.temporal_snapshots),
            system_mode: snapshot.system_mode,
            void_zones: Vec::new(),
            identity: IdentityEngine::new(),
            membrane: DigitalMembrane::new(),
            portals: PortalManager::new(),
            calendar: CalendarEngine::from_events(snapshot.calendar_events),
            processes: ProcessSovereignty::new(),
        })
    }

    pub fn snapshot(&self) -> WorkspaceSnapshot {
        let export = self.graph.export();
        WorkspaceSnapshot {
            graph: StoredGraph {
                nodes: export.nodes,
                edges: export.edges,
            },
            focus_events: self.focus.events().to_vec(),
            journal_entries: self.journal.entries().to_vec(),
            system_mode: self.system_mode.clone(),
            temporal_snapshots: self.temporal.all_snapshots().to_vec(),
            lore_entries: Vec::new(),
            silent_contracts: Vec::new(),
            process_records: Vec::new(),
            calendar_events: self.calendar.all_events().to_vec(),
        }
    }

    pub fn set_system_mode(&mut self, mode: Option<String>) {
        self.system_mode = mode;
    }

    pub fn materialize_thought(
        &mut self,
        engine: &MaterializationEngine,
        raw_text: &str,
    ) -> Result<MaterializationResult, GraphError> {
        let result = engine.materialize(raw_text, &mut self.graph)?;
        // New thought awakens connected nodes — propagate contagion from source
        if self.graph.degree(result.node_id) > 0 {
            let contagion = ContagionEngine::new();
            contagion.propagate(&mut self.graph, result.node_id, 1.0);
        }
        Ok(result)
    }

    pub fn record_focus(
        &mut self,
        node_id: Uuid,
        duration_seconds: f32,
        depth: FocusDepth,
    ) -> Result<FocusEvent, GraphError> {
        self.graph.touch_node(node_id, Utc::now())?;
        let duration_seconds = if duration_seconds.is_finite() && duration_seconds > 0.0 {
            duration_seconds
        } else {
            1.0
        };
        Ok(self.focus.record(node_id, duration_seconds, depth))
    }

    pub fn remove_focus_session(&mut self, session_id: Uuid) -> bool {
        self.focus.remove_session(session_id)
    }

    pub fn add_journal_entry(
        &mut self,
        content: impl Into<String>,
        season: Option<String>,
    ) -> JournalEntry {
        let content = content.into();
        let linked_nodes = self.journal_link_candidates(&content);
        self.journal.add_entry(content, linked_nodes, season)
    }

    pub fn add_journal_entry_with_links(
        &mut self,
        content: impl Into<String>,
        season: Option<String>,
        explicit_links: impl IntoIterator<Item = Uuid>,
    ) -> JournalEntry {
        let content = content.into();
        let linked_nodes = self.merge_journal_links(
            self.journal_link_candidates(&content),
            explicit_links,
        );
        self.journal.add_entry(content, linked_nodes, season)
    }

    pub fn repair_journal_links(&mut self) -> usize {
        let repairs = self
            .journal
            .entries()
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                if entry.linked_nodes.is_empty() {
                    let links = self.journal_link_candidates(&entry.content);
                    if links.is_empty() {
                        None
                    } else {
                        Some((index, links))
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let repaired = repairs.len();
        for (index, links) in repairs {
            if let Some(entry) = self.journal.entries_mut().get_mut(index) {
                entry.linked_nodes = links;
            }
        }
        repaired
    }

    pub fn update_journal_entry(
        &mut self,
        entry_id: Uuid,
        content: impl Into<String>,
        season: Option<String>,
    ) -> Option<JournalEntry> {
        let content = content.into();
        let linked_nodes = self.journal_link_candidates(&content);
        self.journal
            .update_entry(entry_id, content, linked_nodes, season)
    }

    pub fn update_journal_entry_with_links(
        &mut self,
        entry_id: Uuid,
        content: impl Into<String>,
        season: Option<String>,
        explicit_links: impl IntoIterator<Item = Uuid>,
    ) -> Option<JournalEntry> {
        let content = content.into();
        let linked_nodes = self.merge_journal_links(
            self.journal_link_candidates(&content),
            explicit_links,
        );
        self.journal
            .update_entry(entry_id, content, linked_nodes, season)
    }

    pub fn remove_journal_entry(&mut self, entry_id: Uuid) -> Option<JournalEntry> {
        self.journal.remove_entry(entry_id)
    }

    fn journal_link_candidates(&self, content: &str) -> Vec<Uuid> {
        let mut seen = HashSet::new();
        let mut linked_nodes = self
            .focus
            .active_nodes_since(Utc::now() - Duration::hours(1))
            .into_iter()
            .filter(|id| seen.insert(*id))
            .collect::<Vec<_>>();

        for node_id in self.semantic_journal_matches(content, 4) {
            if seen.insert(node_id) {
                linked_nodes.push(node_id);
            }
        }

        linked_nodes
    }

    fn merge_journal_links(
        &self,
        base_links: Vec<Uuid>,
        explicit_links: impl IntoIterator<Item = Uuid>,
    ) -> Vec<Uuid> {
        let mut seen = HashSet::new();
        base_links
            .into_iter()
            .chain(explicit_links)
            .filter(|id| self.graph.get_node(*id).is_some() && seen.insert(*id))
            .collect()
    }

    fn semantic_journal_matches(&self, content: &str, limit: usize) -> Vec<Uuid> {
        let journal_tokens = meaningful_tokens(content);
        if journal_tokens.is_empty() {
            return Vec::new();
        }

        let mut scored = self
            .graph
            .nodes()
            .filter(|node| !node.is_ghost && !node.is_fossil)
            .filter_map(|node| {
                let node_text = node_search_text(node);
                let node_tokens = meaningful_tokens(&node_text);
                if node_tokens.is_empty() {
                    return None;
                }

                let overlap = journal_tokens.intersection(&node_tokens).count();
                let exact_title_bonus = node
                    .content
                    .lines()
                    .next()
                    .map(|title| {
                        let title = title.trim().to_lowercase();
                        title.chars().count() >= 5 && content.to_lowercase().contains(&title)
                    })
                    .unwrap_or(false);

                let score = if exact_title_bonus {
                    1.0
                } else {
                    overlap as f32
                        / ((journal_tokens.len() as f32).sqrt() * (node_tokens.len() as f32).sqrt())
                };

                let strong_overlap = overlap >= 2 && score >= 0.16;
                if exact_title_bonus || strong_overlap || score >= 0.28 {
                    Some((node.id, score, node.gravity, node.access_count))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    b.2.partial_cmp(&a.2)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| b.3.cmp(&a.3))
        });

        scored
            .into_iter()
            .take(limit)
            .map(|(node_id, _, _, _)| node_id)
            .collect()
    }

    pub fn tick_entropy(&mut self, entropy_engine: &EntropyEngine) {
        entropy_engine.tick(&mut self.graph, Utc::now());
    }

    pub fn reverse_entropy(&mut self, entropy_engine: &EntropyEngine, node_id: Uuid) {
        entropy_engine.reverse_entropy(&mut self.graph, node_id, Utc::now());
    }

    pub fn step_gravity(&mut self, gravity_engine: &GravityEngine, delta_time: f32) {
        let heatmap = self
            .focus
            .heatmap(Utc::now() - Duration::days(7), Utc::now());
        gravity_engine.recalculate_masses(&mut self.graph, &heatmap);
        gravity_engine.step(&mut self.graph, delta_time);
    }

    pub fn render_html(
        &self,
        engine: &VisualizationEngine,
        output_path: impl AsRef<Path>,
    ) -> std::io::Result<()> {
        engine.render_to_file(self, output_path)
    }

    pub fn connect_nodes(
        &mut self,
        source_id: Uuid,
        target_id: Uuid,
        edge_type: EdgeType,
        weight: f32,
    ) -> Result<(), GraphError> {
        self.graph.connect(source_id, target_id, edge_type, weight)
    }

    pub fn remove_node(&mut self, node_id: Uuid) -> Result<NodeData, GraphError> {
        self.graph.remove_node(node_id)
    }

    pub fn disconnect_nodes(&mut self, source_id: Uuid, target_id: Uuid) -> Result<(), GraphError> {
        self.graph.disconnect(source_id, target_id)
    }

    pub fn revive_node(&mut self, node_id: Uuid) -> Result<(), GraphError> {
        self.graph.touch_node(node_id, Utc::now())
    }

    pub fn search_nodes(&self, query: &str) -> Vec<&NodeData> {
        self.graph.search_nodes(query)
    }

    pub fn get_node(&self, node_id: Uuid) -> Option<&NodeData> {
        self.graph.get_node(node_id)
    }

    pub fn recent_trail(&self, hours: i64) -> Vec<FocusEvent> {
        let now = Utc::now();
        self.focus.trail_between(now - Duration::hours(hours), now)
    }

    pub fn heatmap_report(&self, days: i64) -> Vec<(Uuid, f32)> {
        let now = Utc::now();
        let mut entries = self
            .focus
            .heatmap(now - Duration::days(days), now)
            .into_iter()
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| right.1.total_cmp(&left.1));
        entries
    }

    pub fn search_journal(&self, query: &str) -> Vec<JournalEntry> {
        self.journal.search(query)
    }

    pub fn recent_journal(&self, days: i64) -> Vec<JournalEntry> {
        let now = Utc::now();
        self.journal.between(now - Duration::days(days), now)
    }

    pub fn journal_between(
        &self,
        from: chrono::DateTime<Utc>,
        to: chrono::DateTime<Utc>,
    ) -> Vec<JournalEntry> {
        self.journal.between(from, to)
    }

    pub fn focus_history_for_node(&self, node_id: Uuid, hours: i64) -> Vec<FocusEvent> {
        self.recent_trail(hours)
            .into_iter()
            .filter(|event| event.node_id == node_id)
            .collect()
    }

    pub fn journal_for_node(&self, node_id: Uuid, days: i64) -> Vec<JournalEntry> {
        self.recent_journal(days)
            .into_iter()
            .filter(|entry| entry.linked_nodes.contains(&node_id))
            .collect()
    }

    /// Spread contagion energy from a node through the graph using BFS.
    pub fn spread_contagion(&mut self, engine: &ContagionEngine, source_id: Uuid, strength: f32) {
        engine.propagate(&mut self.graph, source_id, strength);
    }

    /// Find conceptually similar but structurally disconnected node pairs.
    pub fn find_missing_bridges(&self, analyzer: &SilenceAnalyzer) -> Vec<MissingBridge> {
        analyzer.find_missing_bridges(&self.graph)
    }

    /// Find implied concepts based on semantic gaps between connected clusters.
    pub fn find_implied_concepts(&self, analyzer: &SilenceAnalyzer) -> Vec<ImpliedConcept> {
        analyzer.find_implied_concepts(&self.graph)
    }

    // ─── Phase 4: Living Universe ────────────────────────────────────────────

    /// Re-derive ambient weather state from current graph metrics and focus trail.
    pub fn derive_weather(&self, weather: &mut WeatherSystem) {
        let nodes: Vec<&NodeData> = self.graph.nodes().collect();
        let events = self.focus.events();
        weather.derive(&nodes, events, Utc::now());
    }

    /// Calculate cognitive weight — the accumulated burden of ghosts, fossils, and voids.
    pub fn cognitive_weight(&self) -> WeightReport {
        let nodes: Vec<&NodeData> = self.graph.nodes().collect();
        let isolated_ids: Vec<Uuid> = self
            .graph
            .nodes()
            .filter(|n| self.graph.degree(n.id) == 0)
            .map(|n| n.id)
            .collect();
        CognitiveWeightSystem::new().calculate(&nodes, &[], &isolated_ids)
    }

    /// Derive visual souls for all Project/World nodes from content hashes and history.
    pub fn derive_souls(&self) -> Vec<ProjectSoul> {
        let total = self.graph.node_count();
        // Build degree map so social_density is accurate per node
        let degree_map: std::collections::HashMap<uuid::Uuid, usize> = self
            .graph
            .node_ids()
            .into_iter()
            .map(|id| (id, self.graph.degree(id)))
            .collect();
        // Include temporal snapshots so activity_level reflects real history
        let snapshots = self.temporal.all_snapshots();
        derive_all_souls(
            self.graph.nodes().cloned(),
            &snapshots,
            total,
            Some(&degree_map),
        )
    }

    /// Capture current graph as a snapshot for tectonic comparison.
    pub fn tectonic_snapshot(&self) -> GraphSnapshot {
        GraphSnapshot::from_graph(&self.graph)
    }

    /// Detect structural shifts since the given snapshot.
    pub fn check_tectonics(
        &self,
        before: &GraphSnapshot,
        detector: &TectonicDetector,
    ) -> Option<TectonicEvent> {
        detector.check(before, &self.graph)
    }

    // ─── Phase 5: Temporal Systems ───────────────────────────────────────────

    /// Record a change event for a node into the temporal engine.
    pub fn record_temporal_change(&mut self, node_id: Uuid, change_type: ChangeType) {
        if let Some(node) = self.graph.get_node(node_id) {
            let owned = node.clone();
            self.temporal.record_change(&owned, change_type);
        }
    }

    /// Record creation events for all current nodes (snapshot bootstrap).
    pub fn snapshot_all_nodes(&mut self) {
        let nodes: Vec<NodeData> = self.graph.nodes().cloned().collect();
        for node in &nodes {
            self.temporal.record_change(node, ChangeType::Created);
        }
    }

    /// Open an archaeology session for a specific node.
    pub fn open_archaeology(&self, node_id: Uuid) -> Option<ArchaeologySession> {
        ArchaeologySession::open(&self.temporal, node_id)
    }

    /// Reconstruct the cognitive state for a full calendar day.
    pub fn reconstruct_day(&self, date: NaiveDate) -> DayReconstruction {
        let reconstructor = MemoryReconstructor::new(&self.temporal, &self.focus, &self.journal);
        reconstructor.reconstruct_day(date)
    }

    /// Compare two days to identify what changed between them.
    pub fn compare_days(&self, day_a: NaiveDate, day_b: NaiveDate) -> DayComparison {
        let reconstructor = MemoryReconstructor::new(&self.temporal, &self.focus, &self.journal);
        reconstructor.compare_days(day_a, day_b)
    }

    /// Check whether a node qualifies for fossilization.
    pub fn check_fossilization(&self, node_id: Uuid) -> Option<FossilizationCheck> {
        let node = self.graph.get_node(node_id)?;
        let engine = FossilEngine::new();
        let degree = self.graph.degree(node_id);
        Some(engine.check_fossilization(node, &self.temporal, degree, Utc::now()))
    }

    /// Fossilize a node that qualifies.
    pub fn fossilize_node(&mut self, node_id: Uuid) -> Result<(), GraphError> {
        // Snapshot the pre-fossilization state first
        if let Some(node) = self.graph.get_node(node_id) {
            let owned = node.clone();
            self.temporal
                .record_change(&owned, ChangeType::StateChanged);
        }
        let node = self
            .graph
            .get_node_mut(node_id)
            .ok_or(GraphError::NodeNotFound(node_id))?;
        node.is_fossil = true;
        node.is_ghost = true;
        node.node_type = crate::domain::NodeType::Fossil;
        node.aura_color = "#6b7280".to_string();
        let owned_after = node.clone();
        self.temporal
            .record_change(&owned_after, ChangeType::StateChanged);
        Ok(())
    }

    /// Excavate a fossilized node back to a ghost state.
    pub fn excavate_node(&mut self, node_id: Uuid) -> Result<(), GraphError> {
        let node = self
            .graph
            .get_node_mut(node_id)
            .ok_or(GraphError::NodeNotFound(node_id))?;
        if !node.is_fossil {
            return Ok(());
        }
        node.is_fossil = false;
        node.node_type = crate::domain::NodeType::Ghost;
        node.is_ghost = true;
        node.aura_color = "#a78bfa".to_string();
        node.entropy *= 0.5;
        let owned_after = node.clone();
        self.temporal
            .record_change(&owned_after, ChangeType::StateChanged);
        Ok(())
    }

    /// Detect all narrative arcs and return lore entries.
    pub fn detect_lore(&self, tectonic_events: &[TectonicEvent]) -> Vec<LoreEntry> {
        let nodes: Vec<&NodeData> = self.graph.nodes().collect();
        let detector = LoreArcDetector::new();
        detector.detect_arcs(&nodes, &self.temporal, tectonic_events, Utc::now())
    }

    /// Snapshot count in the temporal engine.
    pub fn temporal_snapshot_count(&self) -> usize {
        self.temporal.snapshot_count()
    }

    // ─── Phase 6: Pattern Recognition ───────────────────────────────────────

    /// Detect repeated behavioral sequences (rituals) from focus history.
    pub fn detect_rituals(&self) -> Vec<Ritual> {
        let engine = RitualEngine::new();
        let mut rituals = engine.detect_rituals(self.focus.events());
        // Enrich ritual names with actual node content (vision.md: rituals should be
        // described as behavioral transitions, not generic "N-step ritual" labels)
        let content_map: std::collections::HashMap<Uuid, String> = self
            .graph
            .nodes()
            .map(|n| (n.id, n.content.clone()))
            .collect();
        for r in &mut rituals {
            r.name = crate::systems::name_ritual(&r.sequence, &content_map);
        }
        rituals
    }

    /// Derive the current cognitive season from workspace metrics.
    pub fn cognitive_season(&self) -> SeasonReport {
        let detector = CognitiveSeasonDetector::new();
        let nodes: Vec<&NodeData> = self.graph.nodes().collect();
        detector.detect_season(
            &nodes,
            self.focus.events(),
            self.journal.entries(),
            Utc::now(),
        )
    }

    /// Generate oracle signals: anticipation, ghost returns, season shifts.
    pub fn oracle_signals(&self) -> Vec<OracleSignal> {
        let oracle = OracleLayer::new();
        let nodes: Vec<&NodeData> = self.graph.nodes().collect();
        oracle.generate_signals(
            &nodes,
            self.focus.events(),
            self.journal.entries(),
            Utc::now(),
        )
    }

    /// Generate a cognitive mirror portrait of stated vs actual focus patterns.
    /// Includes Evolution Portrait (temporal trends) and Creative Pattern (peak hours/days).
    pub fn cognitive_mirror(&self, days: u32) -> CognitivePortrait {
        let mirror = CognitiveMirror;
        let nodes: Vec<&NodeData> = self.graph.nodes().collect();
        let adjacency = self.build_adjacency();
        let snapshots = self.temporal.all_snapshots();
        mirror.generate_portrait(
            &nodes,
            self.focus.events(),
            &adjacency,
            Utc::now(),
            days,
            snapshots,
        )
    }

    /// Calculate the thought heatmap for the last `window_days` days.
    pub fn thought_heatmap(&self, window_days: u32) -> CognitiveHeatmap {
        let engine = ThoughtHeatmapEngine::new();
        let nodes: Vec<&NodeData> = self.graph.nodes().collect();
        engine.calculate(&nodes, self.focus.events(), Utc::now(), window_days)
    }

    /// Find obsessive loops: high focus + high entropy without progress.
    pub fn obsessive_loops(&self) -> Vec<ObsessiveLoop> {
        let engine = ThoughtHeatmapEngine::new();
        let nodes: Vec<&NodeData> = self.graph.nodes().collect();
        let heatmap = engine.calculate(&nodes, self.focus.events(), Utc::now(), 30);
        engine.find_obsessive_loops(&heatmap, &nodes, 3, self.focus.events())
    }

    /// Find neglected regions: nodes connected to active ones but unvisited.
    pub fn neglected_regions(&self) -> Vec<NeglectedRegion> {
        let engine = ThoughtHeatmapEngine::new();
        let nodes: Vec<&NodeData> = self.graph.nodes().collect();
        let heatmap = engine.calculate(&nodes, self.focus.events(), Utc::now(), 30);
        let adjacency = self.build_adjacency();
        engine.find_neglected_regions(&heatmap, &adjacency)
    }

    /// Detect silent contracts: implicit commitments that were never acted on.
    pub fn detect_contracts(&self) -> Vec<DetectedContract> {
        let detector = SilentContractDetector::new();
        let nodes: Vec<&NodeData> = self.graph.nodes().collect();
        let adjacency = self.build_adjacency();
        detector.detect(
            &nodes,
            self.focus.events(),
            self.journal.entries(),
            &adjacency,
            Utc::now(),
        )
    }

    /// Find cross-civilization semantically resonant node pairs via TF-IDF cosine similarity.
    /// Vision.md: resonance is specifically non-obvious connections across different clusters —
    /// "the nodes do not share visible proximity or direct thematic overlap."
    pub fn resonant_pairs(&self) -> Vec<ResonancePair> {
        let mut engine = ResonanceChamberEngine::new();
        engine.cross_civ_only = true;
        let nodes: Vec<&NodeData> = self.graph.nodes().collect();
        engine.find_resonances(&nodes)
    }

    /// Find all resonant pairs including same-civilization ones (for full analysis).
    pub fn resonant_pairs_all(&self) -> Vec<ResonancePair> {
        let engine = ResonanceChamberEngine::new();
        let nodes: Vec<&NodeData> = self.graph.nodes().collect();
        engine.find_resonances(&nodes)
    }

    // ─── Phase 7: Social Graph ────────────────────────────────────────────────

    /// Detect thought civilizations (community clusters).
    pub fn detect_civilizations(&self) -> Vec<Civilization> {
        let detector = CivilizationDetector::new();
        let nodes: Vec<&NodeData> = self.graph.nodes().collect();
        let edges: Vec<_> = self.graph.edges().cloned().collect();
        detector.detect(&nodes, &edges)
    }

    /// Detect civilization events by comparing a previous snapshot to the current state.
    /// Includes Trade (cross-civ edges) and Conflict (territorial contention) detection.
    pub fn detect_civilization_events(&self, prev_civs: &[Civilization]) -> Vec<CivilizationEvent> {
        let detector = CivilizationDetector::new();
        let curr_civs = self.detect_civilizations();
        let edges: Vec<_> = self.graph.edges().cloned().collect();
        detector.detect_events(prev_civs, &curr_civs, &edges)
    }

    /// Check whether a civilization qualifies for crystallization.
    pub fn check_crystallization(&self, civ: &Civilization) -> CrystallizationCheck {
        let engine = CrystallizationEngine::new();
        let nodes: Vec<&NodeData> = self.graph.nodes().collect();
        let edges: Vec<_> = self.graph.edges().cloned().collect();
        engine.check(civ, &nodes, &edges)
    }

    /// Perform knowledge crystallization for a qualifying civilization.
    pub fn crystallize_civilization(&self, civ: &Civilization) -> KnowledgeCrystal {
        let engine = CrystallizationEngine::new();
        let nodes: Vec<&NodeData> = self.graph.nodes().collect();
        let edges: Vec<_> = self.graph.edges().cloned().collect();
        engine.crystallize(civ, &nodes, &edges)
    }

    /// Send a node to the void (sets is_void = true) and creates a VoidZone entry.
    /// Vision.md: "no entropy — no visibility — no classification pressure"
    pub fn send_to_void(&mut self, node_id: Uuid) -> Result<(), GraphError> {
        let node = self
            .graph
            .get_node_mut(node_id)
            .ok_or(GraphError::NodeNotFound(node_id))?;
        node.is_void = true;
        node.entropy = 0.0; // void nodes don't decay
        node.velocity = 0.0; // void nodes don't move
        node.accessed_at = Utc::now(); // used as persisted void-entry time if runtime zones reset
                             // Create or update a VoidZone tracking this node
        let already_tracked = self
            .void_zones
            .iter()
            .any(|z| z.entities.contains(&node_id));
        if !already_tracked {
            self.void_zones
                .push(VoidManager::create_zone(vec![node_id]));
        }
        Ok(())
    }

    /// Extract a node from the void. Removes it from the VoidZone record.
    pub fn extract_from_void(&mut self, node_id: Uuid) -> Result<(), GraphError> {
        let node = self
            .graph
            .get_node_mut(node_id)
            .ok_or(GraphError::NodeNotFound(node_id))?;
        node.is_void = false;
        node.accessed_at = Utc::now();
        // Remove from void zone tracking
        self.void_zones.retain(|z| !z.entities.contains(&node_id));
        Ok(())
    }

    /// List all tracked void zones with their incubation status.
    pub fn void_zone_status(&self) -> Vec<String> {
        let now = chrono::Utc::now();
        self.void_zones.iter().map(|z| z.summary(now)).collect()
    }

    /// Check whether a void node is ready to re-emerge.
    pub fn check_void_emergence(&self, node_id: Uuid) -> Option<EmergenceCheck> {
        let void_node = self.graph.get_node(node_id)?;
        if !void_node.is_void {
            return None;
        }
        let active_nodes: Vec<&NodeData> = self.graph.nodes().filter(|n| !n.is_void).collect();
        let engine = ResonanceChamberEngine::new();
        Some(VoidManager.check_emergence(void_node, &active_nodes, &engine))
    }

    /// Detect digital shadows: incomplete ideas revisited without resolution.
    pub fn digital_shadows(&self) -> Vec<DigitalShadow> {
        let detector = DigitalShadowDetector::new();
        let nodes: Vec<&NodeData> = self.graph.nodes().collect();
        detector.detect(&nodes, self.focus.events(), Utc::now())
    }

    // ─── Phase 10: Identity & Narrative ──────────────────────────────────────

    /// Derive (or re-derive) the Living Signature from the current workspace state.
    /// Vision.md: "No two signatures are alike. No signature is ever finished."
    pub fn derive_living_signature(&mut self) -> &LivingSignature {
        let nodes: Vec<&NodeData> = self.graph.nodes().collect();
        let rituals = self.detect_rituals();

        // Build lore entries from the workspace's lore detector
        let lore_entries = {
            use crate::temporal::LoreArcDetector;
            let detector = LoreArcDetector::new();
            let node_refs: Vec<&NodeData> = self.graph.nodes().collect();
            let tectonic_events: Vec<crate::systems::TectonicEvent> = vec![];
            detector.detect_arcs(
                &node_refs,
                &self.temporal,
                &tectonic_events,
                chrono::Utc::now(),
            )
        };

        let civs = self.detect_civilizations();
        let civ_colors: Vec<[f32; 4]> = civs.iter().map(|c| c.color).collect();

        // Crystals from civilizations that qualify
        let crystals: Vec<crate::systems::KnowledgeCrystal> = civs
            .iter()
            .filter(|c| self.check_crystallization(c).qualifies)
            .map(|c| self.crystallize_civilization(c))
            .collect();

        let season = self.cognitive_season().season;
        let evo_count = self
            .identity
            .current_signature
            .as_ref()
            .map(|s| s.evolution_count + 1)
            .unwrap_or(1);

        self.identity.derive(
            &nodes,
            &rituals,
            &lore_entries,
            &crystals,
            &civ_colors,
            season,
            evo_count,
        )
    }

    /// Detect Shadow Projects — directions deliberately not taken.
    /// Vision.md: "The most revealing thing about a creator is not what they built."
    pub fn detect_shadow_projects(&self) -> Vec<ShadowProject> {
        let detector = ShadowProjectDetector::new();
        let nodes: Vec<&NodeData> = self.graph.nodes().collect();
        let shadows = self.digital_shadows();
        detector.detect(&nodes, &self.void_zones, &shadows, chrono::Utc::now())
    }

    // ─── Phase 9: Audio ──────────────────────────────────────────────────────

    /// Derive the current audio atmosphere from workspace cognitive state and
    /// apply it to the given AudioEngine.
    ///
    /// Vision.md: "the composition shifts continuously as the universe changes"
    /// Call this every 5s from the window renderer (same cadence as weather).
    #[cfg(feature = "audio")]
    pub fn derive_audio_atmosphere(&self, engine: &AudioEngine) {
        let season_report = self.cognitive_season();
        let season_str = season_report.season.name().to_lowercase();
        let season_kind = atmosphere_from_season(&season_str);

        let node_count = self.graph.node_count();
        let avg_entropy: f32 = if node_count > 0 {
            self.graph.nodes().map(|n| n.entropy).sum::<f32>() / node_count as f32
        } else {
            0.0
        };

        let ghost_ratio: f32 = if node_count > 0 {
            self.graph.nodes().filter(|n| n.is_ghost).count() as f32 / node_count as f32
        } else {
            0.0
        };

        let void_ratio: f32 = if node_count > 0 {
            self.graph.nodes().filter(|n| n.is_void).count() as f32 / node_count as f32
        } else {
            0.0
        };

        // Void dominance → Void atmosphere
        if void_ratio > 0.5 {
            engine.set_atmosphere(AtmosphereKind::Void);
            return;
        }
        // Ghost dominance → Ghost atmosphere
        if ghost_ratio > 0.45 {
            engine.set_atmosphere(AtmosphereKind::Ghost);
            return;
        }
        // High entropy → dissonant atmosphere
        if avg_entropy > 0.75 {
            engine.set_atmosphere(AtmosphereKind::HighEntropy);
            return;
        }

        // Otherwise blend: season (primary) + entropy character (secondary) at 0.25
        let entropy_kind = atmosphere_from_entropy(avg_entropy);
        // entropy_from_entropy returns Ghost/HighEntropy/Ambient — only blend if Ambient
        let blend = if entropy_kind == AtmosphereKind::Ambient {
            0.0
        } else {
            0.22
        };
        if blend > 0.0 {
            engine.set_blended_atmosphere(season_kind, entropy_kind, blend);
        } else {
            engine.set_atmosphere(season_kind);
        }
    }

    // ─── Phase 6-7 interaction APIs ──────────────────────────────────────────

    // ── Silent Contract: Fulfill / Release ────────────────────────────────────

    /// Fulfill a silent contract — records a DeepWork focus event on the node,
    /// signalling that the user committed to it.
    pub fn fulfill_contract(&mut self, node_id: Uuid) -> Result<(), GraphError> {
        if self.graph.get_node(node_id).is_none() {
            return Err(GraphError::NodeNotFound(node_id));
        }
        self.record_focus(node_id, 1.0, crate::domain::FocusDepth::DeepWork)?;
        if let Some(node) = self.graph.get_node(node_id) {
            self.temporal
                .record_change(node, crate::domain::ChangeType::Accessed);
        }
        Ok(())
    }

    /// Release a silent contract — consciously dissolve it by recording a
    /// journal entry noting the decision.
    pub fn release_contract(&mut self, node_id: Uuid) -> Result<(), GraphError> {
        let label = self
            .graph
            .get_node(node_id)
            .map(|n| n.content.chars().take(40).collect::<String>())
            .ok_or(GraphError::NodeNotFound(node_id))?;
        self.add_journal_entry(
            &format!("Released silent contract on: {label}"),
            Some("Autumn".to_string()),
        );
        Ok(())
    }

    // ── Resonance Chamber lifecycle ───────────────────────────────────────────

    /// Find cross-civilization resonant pairs and open chambers for them.
    /// Returns chambers in `ChamberState::Open`.
    pub fn open_resonance_chambers(&self, min_similarity: f32) -> Vec<ResonanceChamber> {
        let mut engine = ResonanceChamberEngine::new();
        engine.min_similarity = min_similarity;
        engine.cross_civ_only = true;
        let nodes: Vec<&NodeData> = self.graph.nodes().collect();
        engine
            .find_resonances(&nodes)
            .into_iter()
            .map(|p| ResonanceChamber::open(&p))
            .collect()
    }

    /// Accept a resonance chamber: create a permanent Resonance edge between
    /// the two nodes.
    pub fn accept_resonance(&mut self, chamber: &mut ResonanceChamber) -> Result<(), GraphError> {
        self.connect_nodes(
            chamber.node_a,
            chamber.node_b,
            crate::domain::EdgeType::Resonance,
            chamber.similarity,
        )?;
        chamber.accept();
        Ok(())
    }

    /// Note a resonance chamber: record it in the journal without creating an edge.
    pub fn note_resonance(&mut self, chamber: &mut ResonanceChamber) {
        let label_a = self
            .graph
            .get_node(chamber.node_a)
            .map(|n| n.content.chars().take(25).collect::<String>())
            .unwrap_or_default();
        let label_b = self
            .graph
            .get_node(chamber.node_b)
            .map(|n| n.content.chars().take(25).collect::<String>())
            .unwrap_or_default();
        self.add_journal_entry(
            &format!(
                "Resonance noted between '{label_a}' and '{label_b}' (similarity={:.2})",
                chamber.similarity
            ),
            None,
        );
        chamber.note();
    }

    /// Dismiss a resonance chamber without any record.
    pub fn dismiss_resonance(&self, chamber: &mut ResonanceChamber) {
        chamber.dismiss();
    }

    // ── Knowledge Crystal: Shatter ────────────────────────────────────────────

    /// Shatter a knowledge crystal — disperse all member nodes back to individual
    /// status, removing their civilization binding and restoring normal decay.
    /// Returns the number of nodes released.
    pub fn shatter_crystal(&mut self, crystal: &KnowledgeCrystal) -> usize {
        let mut released = 0;
        for &node_id in &crystal.member_nodes {
            if let Some(node) = self.graph.get_node_mut(node_id) {
                node.civilization_id = None;
                node.entropy = (node.entropy + 0.08).min(0.45);
                node.aura_color = "#7dd3fc".to_string();
                released += 1;
            }
        }
        released
    }

    // ── Digital Shadow interactions ───────────────────────────────────────────

    /// Illuminate a shadow: bring it fully into the active universe.
    /// Boosts gravity, resets entropy, marks with amber aura.
    pub fn illuminate_shadow(&mut self, node_id: Uuid) -> Result<(), GraphError> {
        let node = self
            .graph
            .get_node_mut(node_id)
            .ok_or(GraphError::NodeNotFound(node_id))?;
        node.gravity = (node.gravity + 2.0).min(5.0);
        node.entropy *= 0.15;
        node.aura_color = "#fbbf24".to_string(); // amber — illuminated
        Ok(())
    }

    /// Name a shadow: formally acknowledge it without full commitment.
    /// Renames the node if a non-empty name is provided; marks with violet aura.
    pub fn name_shadow(&mut self, node_id: Uuid, name: Option<String>) -> Result<(), GraphError> {
        let node = self
            .graph
            .get_node_mut(node_id)
            .ok_or(GraphError::NodeNotFound(node_id))?;
        if let Some(n) = name.filter(|s| !s.is_empty()) {
            node.content = n;
        }
        node.aura_color = "#c4b5fd".to_string(); // violet — named but latent
        Ok(())
    }

    /// Release a shadow: consciously dissolve it by sending it to the Void
    /// and recording the closure in the temporal engine.
    pub fn release_shadow(&mut self, node_id: Uuid) -> Result<(), GraphError> {
        self.send_to_void(node_id)?;
        if let Some(node) = self.graph.get_node(node_id) {
            self.temporal
                .record_change(node, crate::domain::ChangeType::StateChanged);
        }
        Ok(())
    }

    // ─── Internal helpers ────────────────────────────────────────────────────

    fn build_adjacency(&self) -> std::collections::HashMap<Uuid, Vec<Uuid>> {
        let mut adj: std::collections::HashMap<Uuid, Vec<Uuid>> = std::collections::HashMap::new();
        for edge in self.graph.edges() {
            adj.entry(edge.source_id).or_default().push(edge.target_id);
            adj.entry(edge.target_id).or_default().push(edge.source_id);
        }
        adj
    }
}

fn node_search_text(node: &NodeData) -> String {
    let mut parts = vec![node.content.clone(), format!("{:?}", node.node_type)];
    for key in ["nickname", "custom_type", "source", "tags"] {
        if let Some(value) = node.metadata.get(key) {
            if let Some(text) = value.as_str() {
                parts.push(text.to_string());
            } else if value.is_array() {
                parts.push(value.to_string());
            }
        }
    }
    parts.join(" ")
}

fn meaningful_tokens(input: &str) -> HashSet<String> {
    input
        .split(|ch: char| !ch.is_alphanumeric())
        .filter_map(|raw| {
            let token = raw.trim().to_lowercase();
            if token.chars().count() < 3 || is_journal_stopword(&token) {
                None
            } else {
                Some(token)
            }
        })
        .collect()
}

fn is_journal_stopword(token: &str) -> bool {
    matches!(
        token,
        "and"
            | "the"
            | "for"
            | "with"
            | "this"
            | "that"
            | "from"
            | "not"
            | "but"
            | "are"
            | "was"
            | "were"
            | "bir"
            | "iki"
            | "uc"
            | "üç"
            | "ve"
            | "və"
            | "ile"
            | "ilə"
            | "ki"
            | "bu"
            | "da"
            | "de"
            | "dedi"
            | "note"
            | "amma"
            | "lazim"
            | "lazımdır"
            | "gerek"
            | "gərək"
            | "qaldi"
            | "qaldı"
            | "etdim"
            | "bugun"
    )
}
