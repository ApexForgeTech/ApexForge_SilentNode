/// Phase 6 & 7 — Pattern Recognition and Social Graph test suite.
use chrono::{Duration, Utc};
use silentnode_core::{
    civilization_color,
    season_aura_colors,
    // Phase 6
    BlindSpot,
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
    CrystallizationCheck,
    CrystallizationEngine,
    DetectedContract,
    DigitalShadow,
    DigitalShadowDetector,
    // Domain
    EdgeData,
    EdgeType,
    EmergenceCheck,
    FocusDepth,
    FocusEvent,
    HeatmapEntry,
    JournalEntry,
    KnowledgeCrystal,
    NeglectedRegion,
    NodeData,
    NodeType,
    ObsessionEntry,
    ObsessiveLoop,
    OracleLayer,
    OracleSignal,
    OracleSignalKind,
    PriorityGap,
    ResonanceChamberEngine,
    ResonancePair,
    Ritual,
    RitualEngine,
    SeasonReport,
    SilentContractDetector,
    SilentNodeWorkspace,
    ThoughtHeatmapEngine,
    VoidManager,
    VoidZone,
};
use uuid::Uuid;

// ── helpers ───────────────────────────────────────────────────────────────────

fn make_node(nt: NodeType, content: &str) -> NodeData {
    NodeData::new(nt, content)
}

fn make_focus_event(
    node_id: Uuid,
    seconds_ago: i64,
    duration_secs: f32,
    depth: FocusDepth,
) -> FocusEvent {
    FocusEvent {
        node_id,
        timestamp: Utc::now() - Duration::seconds(seconds_ago),
        duration_seconds: duration_secs,
        depth,
        session_id: Uuid::new_v4(),
    }
}

fn make_journal(content: &str, days_ago: i64) -> JournalEntry {
    JournalEntry {
        id: Uuid::new_v4(),
        content: content.to_string(),
        timestamp: Utc::now() - Duration::days(days_ago),
        season: None,
        linked_nodes: vec![],
        mood_signature: Default::default(),
    }
}

fn workspace_with_graph() -> (SilentNodeWorkspace, Vec<Uuid>) {
    let mut ws = SilentNodeWorkspace::new();
    let ids: Vec<Uuid> = (0..6)
        .map(|i| {
            let nt = match i % 3 {
                0 => NodeType::Idea,
                1 => NodeType::Memory,
                _ => NodeType::Project,
            };
            ws.graph
                .add_node(make_node(
                    nt,
                    &format!("node_{i} knowledge graph rust engine"),
                ))
                .unwrap()
        })
        .collect();
    ws.graph
        .connect(ids[0], ids[1], EdgeType::Connection, 0.9)
        .unwrap();
    ws.graph
        .connect(ids[1], ids[2], EdgeType::Causal, 0.8)
        .unwrap();
    ws.graph
        .connect(ids[2], ids[3], EdgeType::Connection, 0.7)
        .unwrap();
    ws.graph
        .connect(ids[3], ids[4], EdgeType::Connection, 0.8)
        .unwrap();
    ws.graph
        .connect(ids[4], ids[5], EdgeType::Causal, 0.6)
        .unwrap();
    (ws, ids)
}

// ═══════════════════════════════════════════════════════════════════════════════
// Phase 6 — Pattern Recognition
// ═══════════════════════════════════════════════════════════════════════════════

// ── CognitiveSeasonDetector ───────────────────────────────────────────────────

#[test]
fn season_detector_returns_report_on_empty_graph() {
    let detector = CognitiveSeasonDetector::new();
    let report = detector.detect_season(&[], &[], &[], Utc::now());
    // With no data, should still return a valid enum variant
    let _ = report.season; // just ensure it's accessible
    assert!(report.creation_rate >= 0.0 && report.creation_rate <= 1.0);
}

#[test]
fn season_summer_from_high_creation_and_focus() {
    let detector = CognitiveSeasonDetector::new();
    // Summer: creation_rate in (0.3, 0.4] + high focus density, low exploration_ratio
    // Spring fires first if creation_rate > 0.4 AND exploration_ratio > 0.4, so we avoid that.
    let mut nodes: Vec<NodeData> = Vec::new();
    // 20 old nodes (outside 30-day window)
    for i in 0..13 {
        let mut n = make_node(NodeType::Idea, &format!("old_idea_{i}"));
        n.created_at = Utc::now() - Duration::days(60);
        nodes.push(n);
    }
    // 7 recently created (creation_rate = 7/20 = 0.35, in (0.3, 0.4])
    for i in 0..7 {
        let mut n = make_node(NodeType::Idea, &format!("fresh_{i}"));
        n.created_at = Utc::now() - Duration::days(1);
        nodes.push(n);
    }
    let node_refs: Vec<&NodeData> = nodes.iter().collect();

    // Many focus events on just 2 nodes → exploration_ratio = 2/20 = 0.1 (no Spring)
    // 25 events → focus_density = 25/20 = 1.25, clamped to 1.0 (Summer)
    let events: Vec<FocusEvent> = (0..25)
        .map(|i| {
            make_focus_event(
                nodes[i % 2].id,
                i as i64 * 3600,
                300.0,
                FocusDepth::DeepWork,
            )
        })
        .collect();

    let report = detector.detect_season(&node_refs, &events, &[], Utc::now());
    assert_eq!(
        report.season,
        CognitiveSeason::Summer,
        "creation_rate~0.35 + high focus density + low exploration should yield Summer, got {:?}",
        report.season
    );
}

#[test]
fn season_winter_from_stale_graph() {
    let detector = CognitiveSeasonDetector::new();
    // Old nodes, no recent events → Winter
    let nodes: Vec<NodeData> = (0..10)
        .map(|i| {
            let mut n = make_node(NodeType::Memory, &format!("old_memory_{i}"));
            n.created_at = Utc::now() - Duration::days(90);
            n.entropy = 0.8;
            n
        })
        .collect();
    let node_refs: Vec<&NodeData> = nodes.iter().collect();

    let report = detector.detect_season(&node_refs, &[], &[], Utc::now());
    assert_eq!(
        report.season,
        CognitiveSeason::Winter,
        "old stale nodes with no focus should yield Winter, got {:?}",
        report.season
    );
}

#[test]
fn season_aura_colors_all_seasons_valid() {
    for season in [
        CognitiveSeason::Spring,
        CognitiveSeason::Summer,
        CognitiveSeason::Autumn,
        CognitiveSeason::Winter,
    ] {
        let (primary, secondary) = season_aura_colors(season);
        assert_eq!(primary.len(), 4);
        assert_eq!(secondary.len(), 4);
        // All alpha values should be 1.0
        assert_eq!(primary[3], 1.0, "season {:?} primary alpha != 1.0", season);
        assert_eq!(
            secondary[3], 1.0,
            "season {:?} secondary alpha != 1.0",
            season
        );
        // RGB values should be in [0,1]
        for v in primary.iter().chain(secondary.iter()) {
            assert!(*v >= 0.0 && *v <= 1.0, "color value out of range: {v}");
        }
    }
}

#[test]
fn season_report_all_fields_bounded() {
    let detector = CognitiveSeasonDetector::new();
    let nodes: Vec<NodeData> = (0..5)
        .map(|i| make_node(NodeType::Idea, &format!("n{i}")))
        .collect();
    let node_refs: Vec<&NodeData> = nodes.iter().collect();
    let events: Vec<FocusEvent> = (0..5)
        .map(|i| make_focus_event(nodes[i].id, i as i64 * 100, 60.0, FocusDepth::Read))
        .collect();

    let report = detector.detect_season(&node_refs, &events, &[], Utc::now());
    assert!(report.creation_rate >= 0.0 && report.creation_rate <= 1.0);
    assert!(report.focus_density >= 0.0 && report.focus_density <= 1.0);
    assert!(report.exploration_ratio >= 0.0 && report.exploration_ratio <= 1.0);
    assert!(report.avg_entropy >= 0.0 && report.avg_entropy <= 1.0);
    assert!(report.revisit_ratio >= 0.0 && report.revisit_ratio <= 1.0);
}

// ── OracleLayer ───────────────────────────────────────────────────────────────

#[test]
fn oracle_empty_graph_no_signals() {
    let oracle = OracleLayer::new();
    let signals = oracle.generate_signals(&[], &[], &[], Utc::now());
    assert!(
        signals.is_empty(),
        "empty graph should produce no oracle signals"
    );
}

#[test]
fn oracle_signals_sorted_by_strength() {
    let oracle = OracleLayer::new();
    let nodes: Vec<NodeData> = (0..4)
        .map(|i| make_node(NodeType::Idea, &format!("topic_{i}")))
        .collect();
    let node_refs: Vec<&NodeData> = nodes.iter().collect();

    // Give nodes repeated access patterns over 30 days
    let mut events: Vec<FocusEvent> = Vec::new();
    for day in 0..25i64 {
        for n in &nodes {
            events.push(make_focus_event(n.id, day * 86400, 120.0, FocusDepth::Read));
        }
    }

    let signals = oracle.generate_signals(&node_refs, &events, &[], Utc::now());
    // If any signals, they must be sorted descending by strength
    for w in signals.windows(2) {
        assert!(
            w[0].strength >= w[1].strength,
            "oracle signals not sorted: {} < {}",
            w[0].strength,
            w[1].strength
        );
    }
}

#[test]
fn oracle_signal_strength_bounded() {
    let oracle = OracleLayer::new();
    let nodes: Vec<NodeData> = (0..4)
        .map(|i| {
            let mut n = make_node(NodeType::Idea, &format!("topic_{i}"));
            n.is_ghost = i == 0; // one ghost node for GhostReturn signal
            n
        })
        .collect();
    let node_refs: Vec<&NodeData> = nodes.iter().collect();

    let events: Vec<FocusEvent> = (0..30)
        .map(|i| make_focus_event(nodes[i % 4].id, i as i64 * 86400, 200.0, FocusDepth::Edit))
        .collect();

    let signals = oracle.generate_signals(&node_refs, &events, &[], Utc::now());
    for sig in &signals {
        assert!(
            sig.strength >= 0.0 && sig.strength <= 1.0,
            "oracle signal strength {} out of [0,1]",
            sig.strength
        );
        assert!(
            !sig.description.is_empty(),
            "oracle signal has empty description"
        );
    }
}

// ── RitualEngine ──────────────────────────────────────────────────────────────

#[test]
fn ritual_engine_no_events_no_rituals() {
    let engine = RitualEngine::new();
    let rituals = engine.detect_rituals(&[]);
    assert!(rituals.is_empty());
}

#[test]
fn ritual_engine_detects_repeated_sequence() {
    let engine = RitualEngine::new();
    let n1 = Uuid::new_v4();
    let n2 = Uuid::new_v4();
    let n3 = Uuid::new_v4();

    // Create 4 sessions with the same sequence n1 → n2 → n3
    let mut events: Vec<FocusEvent> = Vec::new();
    for session in 0..4u64 {
        let base_secs = session * 7200 + 86400 * session; // each session ~2h, days apart
        let sid = Uuid::new_v4();
        for (i, &nid) in [n1, n2, n3].iter().enumerate() {
            events.push(FocusEvent {
                node_id: nid,
                timestamp: Utc::now() - Duration::seconds(base_secs as i64 + i as i64 * 300),
                duration_seconds: 120.0,
                depth: FocusDepth::DeepWork,
                session_id: sid,
            });
        }
    }

    let rituals = engine.detect_rituals(&events);
    // Should detect at least one ritual involving the repeated sequence
    assert!(
        !rituals.is_empty(),
        "expected at least one ritual from 4 repeated sessions, got none"
    );

    for r in &rituals {
        assert!(
            r.occurrence_count >= 2,
            "ritual occurrence_count too low: {}",
            r.occurrence_count
        );
        assert!(
            r.strength >= 0.0 && r.strength <= 1.0,
            "ritual strength out of bounds"
        );
        assert!(!r.sequence.is_empty(), "ritual has empty sequence");
    }
}

#[test]
fn ritual_predict_next_step_works() {
    let n1 = Uuid::new_v4();
    let n2 = Uuid::new_v4();
    let n3 = Uuid::new_v4();

    let ritual = Ritual {
        id: Uuid::new_v4(),
        name: "test ritual".to_string(),
        sequence: vec![n1, n2, n3],
        occurrence_count: 5,
        avg_interval_hours: 24.0,
        strength: 0.8,
        last_seen: Utc::now(),
    };

    let next = RitualEngine::predict_next_step(&[n1, n2], &ritual);
    assert_eq!(next, Some(n3), "predicting after [n1,n2] should yield n3");

    let next_none = RitualEngine::predict_next_step(&[n2, n3], &ritual);
    assert_eq!(next_none, None, "at end of sequence should return None");
}

// ── CognitiveMirror ───────────────────────────────────────────────────────────

#[test]
fn mirror_empty_workspace_returns_portrait() {
    let mirror = CognitiveMirror::new();
    let portrait = mirror.generate_portrait(
        &[],
        &[],
        &std::collections::HashMap::new(),
        Utc::now(),
        30,
        &[],
    );
    assert!(portrait.priority_gaps.is_empty());
    assert!(portrait.blind_spots.is_empty());
    assert!(portrait.obsessions.is_empty());
}

#[test]
fn mirror_detects_obsession_on_heavily_focused_node() {
    let mirror = CognitiveMirror::new();
    // Obsession requires: normalized focus > 0.6 AND node.entropy > 0.5
    let mut nodes: Vec<NodeData> = (0..5)
        .map(|i| make_node(NodeType::Idea, &format!("concept_{i}")))
        .collect();
    nodes[0].entropy = 0.7; // must be > 0.5 for obsession detection
    let node_refs: Vec<&NodeData> = nodes.iter().collect();

    // 30 deep focus events on node[0], nothing on others → normalized focus = 1.0 > 0.6
    let events: Vec<FocusEvent> = (0..30)
        .map(|i| make_focus_event(nodes[0].id, i as i64 * 600, 600.0, FocusDepth::DeepWork))
        .collect();

    let adj: std::collections::HashMap<Uuid, Vec<Uuid>> = std::collections::HashMap::new();
    let portrait = mirror.generate_portrait(&node_refs, &events, &adj, Utc::now(), 30, &[]);
    assert!(
        !portrait.obsessions.is_empty(),
        "heavily focused high-entropy node should be detected as obsession"
    );
    assert_eq!(portrait.obsessions[0].node_id, nodes[0].id);
    assert!(
        !portrait.most_obsessed.is_none(),
        "most_obsessed should be set"
    );
}

#[test]
fn mirror_detects_blind_spot_on_unaccessed_neighbor() {
    let mirror = CognitiveMirror::new();
    // Blind spot: node.accessed_at < now - 21 days AND has neighbor in active_nodes (accessed within 30 days)
    let active = make_node(NodeType::Idea, "active_concept");
    let mut blind = make_node(NodeType::Idea, "ignored_neighbor");
    blind.accessed_at = Utc::now() - Duration::days(25); // older than 21 days → qualifies as blind

    let node_refs: Vec<&NodeData> = vec![&active, &blind];

    // Only focus on active node (active.accessed_at defaults to now → in active_nodes set)
    let events: Vec<FocusEvent> = (0..15)
        .map(|i| make_focus_event(active.id, i as i64 * 3600, 300.0, FocusDepth::Edit))
        .collect();

    // adjacency: blind → [active] so that blind has an active neighbor
    let mut adj: std::collections::HashMap<Uuid, Vec<Uuid>> = std::collections::HashMap::new();
    adj.insert(blind.id, vec![active.id]);

    let portrait = mirror.generate_portrait(&node_refs, &events, &adj, Utc::now(), 30, &[]);
    let blind_ids: Vec<Uuid> = portrait.blind_spots.iter().map(|b| b.node_id).collect();
    assert!(
        blind_ids.contains(&blind.id),
        "node last accessed 25 days ago with active neighbor should be a blind spot"
    );
}

// ── ThoughtHeatmapEngine ──────────────────────────────────────────────────────

#[test]
fn heatmap_empty_returns_empty_entries() {
    let engine = ThoughtHeatmapEngine::new();
    let nodes: Vec<NodeData> = vec![];
    let hm = engine.calculate(&[], &[], Utc::now(), 7);
    assert!(hm.entries.is_empty());
}

#[test]
fn heatmap_entries_sorted_by_energy_desc() {
    let engine = ThoughtHeatmapEngine::new();
    let nodes: Vec<NodeData> = (0..4)
        .map(|i| make_node(NodeType::Idea, &format!("n{i}")))
        .collect();
    let node_refs: Vec<&NodeData> = nodes.iter().collect();

    let events: Vec<FocusEvent> = (0..20)
        .map(|i| {
            // Node 0 gets most events (highest energy)
            let nid = if i < 12 {
                nodes[0].id
            } else {
                nodes[1 + (i % 3)].id
            };
            make_focus_event(nid, i as i64 * 1800, 120.0, FocusDepth::Read)
        })
        .collect();

    let hm = engine.calculate(&node_refs, &events, Utc::now(), 7);
    for w in hm.entries.windows(2) {
        assert!(
            w[0].energy >= w[1].energy,
            "heatmap not sorted: {} < {}",
            w[0].energy,
            w[1].energy
        );
    }
}

#[test]
fn heatmap_energy_is_positive_and_bounded() {
    let engine = ThoughtHeatmapEngine::new();
    let nodes: Vec<NodeData> = (0..3)
        .map(|i| make_node(NodeType::Idea, &format!("n{i}")))
        .collect();
    let node_refs: Vec<&NodeData> = nodes.iter().collect();

    let events: Vec<FocusEvent> = (0..10)
        .map(|i| make_focus_event(nodes[i % 3].id, i as i64 * 7200, 60.0, FocusDepth::Glance))
        .collect();

    let hm = engine.calculate(&node_refs, &events, Utc::now(), 7);
    for entry in &hm.entries {
        assert!(entry.energy >= 0.0, "negative energy: {}", entry.energy);
    }
}

#[test]
fn obsessive_loop_detection_on_tight_cluster() {
    let engine = ThoughtHeatmapEngine::new();
    // Obsessive loop requires: in top 20% of heatmap AND node.entropy > 0.4
    let mut nodes: Vec<NodeData> = (0..3)
        .map(|i| make_node(NodeType::Idea, &format!("loop_{i}")))
        .collect();
    for n in nodes.iter_mut() {
        n.entropy = 0.6; // must be > 0.4 for obsessive loop detection
    }
    let node_refs: Vec<&NodeData> = nodes.iter().collect();

    // Visit nodes in a tight loop many times
    let pattern = [0usize, 1, 2, 0, 1, 2, 0, 1, 2, 0, 1, 2];
    let events: Vec<FocusEvent> = pattern
        .iter()
        .enumerate()
        .map(|(i, &idx)| make_focus_event(nodes[idx].id, i as i64 * 1200, 180.0, FocusDepth::Edit))
        .collect();

    let hm = engine.calculate(&node_refs, &events, Utc::now(), 7);
    let loops = engine.find_obsessive_loops(&hm, &node_refs, 3, &events);
    assert!(
        !loops.is_empty(),
        "high-entropy nodes with 4 revisits should yield obsessive loops"
    );
}

// ── SilentContractDetector ────────────────────────────────────────────────────

#[test]
fn contract_empty_no_contracts() {
    let detector = SilentContractDetector::new();
    let empty_adj: std::collections::HashMap<Uuid, Vec<Uuid>> = std::collections::HashMap::new();
    let contracts = detector.detect(&[], &[], &[], &empty_adj, Utc::now());
    assert!(contracts.is_empty());
}

#[test]
fn contract_ghost_node_detected() {
    let detector = SilentContractDetector::new();
    // Contract strength = approach_score*0.4 + journal_score*0.4 + gravity_gap*0.2
    // journal_score uses linked_nodes, not content. approach_score uses focus events.
    let mut ghost = make_node(NodeType::Idea, "unresolved_obligation");
    ghost.gravity = 5.0; // high gravity, zero access_count → gravity_gap = 1.0 → 0.2 contribution

    let node_refs: Vec<&NodeData> = vec![&ghost];

    // Shallow focus events (Glance only, no DeepWork) → approach_score = 1.0 → 0.4 contribution
    let events: Vec<FocusEvent> = (0..4)
        .map(|i| make_focus_event(ghost.id, i as i64 * 7200, 30.0, FocusDepth::Glance))
        .collect();

    // Journal entries with linked_nodes=[ghost.id] and no follow-up focus events within 48h
    let mut j1 = make_journal("must resolve", 10);
    j1.linked_nodes = vec![ghost.id];
    let mut j2 = make_journal("still pending", 20);
    j2.linked_nodes = vec![ghost.id];
    // These are 10 and 20 days old; focus events are < 24h old → no overlap
    let journal = vec![j1, j2];

    let adj: std::collections::HashMap<Uuid, Vec<Uuid>> = std::collections::HashMap::new();
    let contracts = detector.detect(&node_refs, &events, &journal, &adj, Utc::now());
    // strength ≈ 1.0*0.4 + 1.0*0.4 + 1.0*0.2 = 1.0, well above threshold=0.35
    assert!(
        !contracts.is_empty(),
        "shallow-focused node with unfollowed journal refs should be a contract"
    );
    assert!(contracts[0].strength >= 0.35);
}

// ── ResonanceChamberEngine ────────────────────────────────────────────────────

#[test]
fn resonance_single_node_no_pairs() {
    let engine = ResonanceChamberEngine::new();
    let node = make_node(NodeType::Idea, "lone concept");
    let pairs = engine.find_resonances(&[&node]);
    assert!(pairs.is_empty());
}

#[test]
fn resonance_similar_nodes_produce_pairs() {
    let engine = ResonanceChamberEngine::new();
    let n1 = make_node(NodeType::Idea, "rust programming language systems engine");
    let n2 = make_node(NodeType::Idea, "rust language systems engine programming");
    let n3 = make_node(
        NodeType::Memory,
        "completely different topic cooking recipes",
    );

    let pairs = engine.find_resonances(&[&n1, &n2, &n3]);
    // n1 and n2 share most words — should have a resonance pair
    let has_n1_n2 = pairs.iter().any(|p| {
        (p.node_a == n1.id && p.node_b == n2.id) || (p.node_a == n2.id && p.node_b == n1.id)
    });
    assert!(
        has_n1_n2,
        "near-identical nodes should form a resonance pair; got {} pairs",
        pairs.len()
    );
}

#[test]
fn resonance_similarity_bounded() {
    let engine = ResonanceChamberEngine::new();
    let nodes: Vec<NodeData> = (0..5)
        .map(|i| {
            make_node(
                NodeType::Idea,
                &format!("concept topic domain knowledge {i}"),
            )
        })
        .collect();
    let node_refs: Vec<&NodeData> = nodes.iter().collect();

    let pairs = engine.find_resonances(&node_refs);
    for p in &pairs {
        assert!(
            p.similarity >= 0.0 && p.similarity <= 1.0,
            "similarity {} out of [0,1]",
            p.similarity
        );
    }
}

#[test]
fn resonance_pairs_sorted_descending() {
    let engine = ResonanceChamberEngine::new();
    let nodes: Vec<NodeData> = (0..6)
        .map(|i| {
            make_node(
                NodeType::Idea,
                &format!("rust systems programming knowledge engine concept {i}"),
            )
        })
        .collect();
    let node_refs: Vec<&NodeData> = nodes.iter().collect();

    let pairs = engine.find_resonances(&node_refs);
    for w in pairs.windows(2) {
        assert!(
            w[0].similarity >= w[1].similarity,
            "resonance pairs not sorted: {} < {}",
            w[0].similarity,
            w[1].similarity
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Phase 7 — Social Graph
// ═══════════════════════════════════════════════════════════════════════════════

// ── CivilizationDetector ──────────────────────────────────────────────────────

#[test]
fn civilization_empty_graph_no_civs() {
    let detector = CivilizationDetector::new();
    let civs = detector.detect(&[], &[]);
    assert!(civs.is_empty());
}

#[test]
fn civilization_isolated_nodes_below_min_size() {
    let detector = CivilizationDetector::new(); // min_size=3 by default
    let nodes: Vec<NodeData> = (0..2)
        .map(|i| make_node(NodeType::Idea, &format!("n{i}")))
        .collect();
    let node_refs: Vec<&NodeData> = nodes.iter().collect();
    let civs = detector.detect(&node_refs, &[]);
    assert!(
        civs.is_empty(),
        "two isolated nodes should not form a civilization"
    );
}

#[test]
fn civilization_detects_dense_cluster() {
    let detector = CivilizationDetector::new();
    // 5 nodes densely connected
    let nodes: Vec<NodeData> = (0..5)
        .map(|i| make_node(NodeType::Idea, &format!("cluster_node_{i}")))
        .collect();
    let node_refs: Vec<&NodeData> = nodes.iter().collect();

    // Connect them all pairwise (dense cluster)
    let mut edges: Vec<EdgeData> = Vec::new();
    for i in 0..5 {
        for j in (i + 1)..5 {
            edges.push(EdgeData::new(
                nodes[i].id,
                nodes[j].id,
                EdgeType::Connection,
                0.8,
            ));
        }
    }

    let civs = detector.detect(&node_refs, &edges);
    assert!(
        !civs.is_empty(),
        "dense 5-node cluster should form at least one civilization"
    );

    let total_members: usize = civs.iter().map(|c| c.member_nodes.len()).sum();
    assert!(
        total_members >= 3,
        "civilization should have at least 3 members"
    );
}

#[test]
fn civilization_internal_density_bounded() {
    let (ws, ids) = workspace_with_graph();
    let detector = CivilizationDetector::new();
    let nodes: Vec<&NodeData> = ws.graph.nodes().collect();
    let edges: Vec<EdgeData> = ws.graph.edges().cloned().collect();
    let civs = detector.detect(&nodes, &edges);

    for civ in &civs {
        assert!(
            civ.internal_density >= 0.0 && civ.internal_density <= 1.0,
            "internal_density {} out of [0,1]",
            civ.internal_density
        );
        assert!(!civ.member_nodes.is_empty());
        assert_eq!(civ.color.len(), 4);
    }
}

#[test]
fn civilization_color_golden_ratio_distinct() {
    // Consecutive civilization colors should be visually distinct (not equal)
    let c0 = civilization_color(0);
    let c1 = civilization_color(1);
    let c2 = civilization_color(2);
    assert_ne!(c0, c1, "consecutive civ colors should differ");
    assert_ne!(c1, c2, "consecutive civ colors should differ");
    // All alpha values should be 1.0
    assert_eq!(c0[3], 1.0);
    assert_eq!(c1[3], 1.0);
}

#[test]
fn civilization_color_deterministic() {
    // Same index always yields same color
    assert_eq!(civilization_color(0), civilization_color(0));
    assert_eq!(civilization_color(7), civilization_color(7));
    assert_eq!(civilization_color(100), civilization_color(100));
}

#[test]
fn civilization_detect_events_merge() {
    let detector = CivilizationDetector::new();
    let id_a = Uuid::new_v4();
    let id_b = Uuid::new_v4();

    let prev: Vec<Civilization> = vec![
        Civilization {
            id: id_a,
            member_nodes: vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()],
            dominant_node: None,
            internal_density: 0.5,
            age_days: 10.0,
            territory_radius: 1.5,
            color: [1.0, 0.5, 0.0, 1.0],
        },
        Civilization {
            id: id_b,
            member_nodes: vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()],
            dominant_node: None,
            internal_density: 0.5,
            age_days: 10.0,
            territory_radius: 1.5,
            color: [0.0, 0.5, 1.0, 1.0],
        },
    ];

    // Merged: both old sets now in one large civ
    let merged_members: Vec<Uuid> = prev[0]
        .member_nodes
        .iter()
        .chain(prev[1].member_nodes.iter())
        .cloned()
        .collect();

    let curr: Vec<Civilization> = vec![Civilization {
        id: Uuid::new_v4(),
        member_nodes: merged_members,
        dominant_node: None,
        internal_density: 0.7,
        age_days: 0.0,
        territory_radius: 2.5,
        color: [0.5, 0.5, 0.5, 1.0],
    }];

    let events = detector.detect_events(&prev, &curr, &[]);
    assert!(
        !events.is_empty(),
        "merging two civs into one should produce events"
    );
}

// ── CrystallizationEngine ─────────────────────────────────────────────────────

#[test]
fn crystallization_check_too_small_civ() {
    let engine = CrystallizationEngine::new(); // min_cluster_size=4
    let civ = Civilization {
        id: Uuid::new_v4(),
        member_nodes: vec![Uuid::new_v4(), Uuid::new_v4()], // only 2 members
        dominant_node: None,
        internal_density: 0.9,
        age_days: 30.0,
        territory_radius: 1.0,
        color: [1.0, 1.0, 0.5, 1.0],
    };
    let check = engine.check(&civ, &[], &[]);
    assert!(
        !check.qualifies,
        "civ with 2 members should not qualify for crystallization"
    );
}

#[test]
fn crystallization_check_qualifies_dense_stable() {
    let engine = CrystallizationEngine::new();
    let nodes: Vec<NodeData> = (0..6)
        .map(|i| make_node(NodeType::Idea, &format!("crystal_{i}")))
        .collect();

    // Build dense edges between all cluster members
    let mut edges: Vec<EdgeData> = Vec::new();
    for i in 0..6 {
        for j in (i + 1)..6 {
            edges.push(EdgeData::new(
                nodes[i].id,
                nodes[j].id,
                EdgeType::Connection,
                0.9,
            ));
        }
    }

    let civ = Civilization {
        id: Uuid::new_v4(),
        member_nodes: nodes.iter().map(|n| n.id).collect(),
        dominant_node: Some(nodes[0].id),
        internal_density: 0.9,
        age_days: 20.0, // > stability_days=14
        territory_radius: 2.0,
        color: [1.0, 0.9, 0.4, 1.0],
    };

    let node_refs: Vec<&NodeData> = nodes.iter().collect();
    let check = engine.check(&civ, &node_refs, &edges);
    assert!(
        check.qualifies,
        "dense 6-node stable civ should qualify for crystallization; density={:.3}",
        check.internal_density
    );
}

#[test]
fn crystallization_produces_crystal() {
    let engine = CrystallizationEngine::new();
    let nodes: Vec<NodeData> = (0..5)
        .map(|i| make_node(NodeType::Idea, &format!("know_{i}")))
        .collect();
    let node_refs: Vec<&NodeData> = nodes.iter().collect();

    let mut edges: Vec<EdgeData> = Vec::new();
    for i in 0..5 {
        for j in (i + 1)..5 {
            edges.push(EdgeData::new(
                nodes[i].id,
                nodes[j].id,
                EdgeType::Connection,
                0.85,
            ));
        }
    }

    let civ = Civilization {
        id: Uuid::new_v4(),
        member_nodes: nodes.iter().map(|n| n.id).collect(),
        dominant_node: Some(nodes[0].id),
        internal_density: 0.85,
        age_days: 25.0,
        territory_radius: 1.8,
        color: [1.0, 0.9, 0.4, 1.0],
    };

    let crystal = engine.crystallize(&civ, &node_refs, &edges);
    assert_eq!(crystal.source_civilization_id, civ.id);
    assert!(!crystal.member_nodes.is_empty());
    assert!(crystal.internal_density >= 0.0 && crystal.internal_density <= 1.0);
}

// ── VoidManager ───────────────────────────────────────────────────────────────

#[test]
fn void_manager_create_zone() {
    let ids: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();
    let zone = VoidManager::create_zone(ids.clone());
    assert_eq!(zone.entities, ids);
    assert!(!zone.id.is_nil());
}

#[test]
fn void_emergence_check_isolated_node() {
    let vm = VoidManager::new();
    let void_node = make_node(NodeType::Ghost, "isolated_void_concept");
    let active: Vec<NodeData> = (0..4)
        .map(|i| make_node(NodeType::Idea, &format!("active_concept_{i}")))
        .collect();
    let active_refs: Vec<&NodeData> = active.iter().collect();

    let resonance = ResonanceChamberEngine::new();
    let check = vm.check_emergence(&void_node, &active_refs, &resonance);
    assert!(check.resonance_score >= 0.0 && check.resonance_score <= 1.0);
    // bool field accessible
    let _ = check.emergence_likely;
}

#[test]
fn void_emergence_check_semantically_similar_node() {
    let vm = VoidManager::new();
    let void_node = make_node(NodeType::Ghost, "rust systems programming engine async");
    let active: Vec<NodeData> = vec![
        make_node(NodeType::Idea, "rust async systems engine programming"),
        make_node(NodeType::Idea, "rust programming language engine"),
        make_node(NodeType::Memory, "cooking recipes food pasta"),
    ];
    let active_refs: Vec<&NodeData> = active.iter().collect();

    let resonance = ResonanceChamberEngine::new();
    let check = vm.check_emergence(&void_node, &active_refs, &resonance);
    // Should find similar active nodes and have positive resonance
    assert!(
        !check.similar_active_nodes.is_empty(),
        "semantically similar void node should find similar active nodes"
    );
    assert!(check.resonance_score > 0.0);
}

#[test]
fn void_should_emerge_threshold() {
    let vm = VoidManager::new();
    let check_high = EmergenceCheck {
        node_id: Uuid::new_v4(),
        resonance_score: 0.9,
        emergence_likely: true,
        similar_active_nodes: vec![Uuid::new_v4()],
    };
    let check_low = EmergenceCheck {
        node_id: Uuid::new_v4(),
        resonance_score: 0.1,
        emergence_likely: false,
        similar_active_nodes: vec![],
    };
    assert!(vm.should_emerge(&check_high, 0.5));
    assert!(!vm.should_emerge(&check_low, 0.5));
}

// ── DigitalShadowDetector ─────────────────────────────────────────────────────

#[test]
fn shadow_detector_empty_no_shadows() {
    let detector = DigitalShadowDetector::new();
    let shadows = detector.detect(&[], &[], Utc::now());
    assert!(shadows.is_empty());
}

#[test]
fn shadow_detector_finds_high_revisit_node() {
    let detector = DigitalShadowDetector::new(); // min_revisits=3, revisit_gap_days=3, max_entropy=0.7

    let mut shadow_node = make_node(NodeType::Idea, "recurring_fixation");
    shadow_node.entropy = 0.4; // low entropy = stuck = shadow candidate
    shadow_node.created_at = Utc::now() - Duration::days(60);

    let nodes: Vec<NodeData> = vec![shadow_node.clone()];
    let node_refs: Vec<&NodeData> = nodes.iter().collect();

    // 8 visits spread over 28 days with 4-day gaps (safely > revisit_gap_days=3)
    let base = Utc::now();
    let events: Vec<FocusEvent> = (0..8)
        .map(|i| FocusEvent {
            node_id: shadow_node.id,
            timestamp: base - Duration::days(i as i64 * 4),
            duration_seconds: 300.0,
            depth: FocusDepth::Edit,
            session_id: Uuid::new_v4(),
        })
        .collect();

    let shadows = detector.detect(&node_refs, &events, Utc::now());
    assert!(
        !shadows.is_empty(),
        "node with 8 spaced revisits and low entropy should be a digital shadow"
    );
    assert_eq!(shadows[0].node_id, shadow_node.id);
    assert!(shadows[0].intensity >= 0.0 && shadows[0].intensity <= 1.0);
    assert!(shadows[0].revisit_count >= 3);
}

#[test]
fn shadow_detector_skips_high_entropy_node() {
    let detector = DigitalShadowDetector::new();

    let mut dynamic_node = make_node(NodeType::Idea, "evolving_concept");
    dynamic_node.entropy = 0.9; // high entropy → not a shadow

    let nodes = vec![dynamic_node.clone()];
    let node_refs: Vec<&NodeData> = nodes.iter().collect();

    let events: Vec<FocusEvent> = (0..10)
        .map(|i| {
            make_focus_event(
                dynamic_node.id,
                i as i64 * 4 * 86400,
                200.0,
                FocusDepth::Edit,
            )
        })
        .collect();

    let shadows = detector.detect(&node_refs, &events, Utc::now());
    assert!(
        shadows.is_empty(),
        "high-entropy node should not be a digital shadow"
    );
}

// ── workspace integration ─────────────────────────────────────────────────────

#[test]
fn workspace_detect_civilizations_returns_list() {
    let (ws, _) = workspace_with_graph();
    let civs = ws.detect_civilizations();
    // With 6 nodes and 5 edges, may or may not detect civs depending on density
    // Just verify it doesn't panic and returns a Vec
    let _ = civs.len();
}

#[test]
fn workspace_cognitive_season_returns_season() {
    let (ws, ids) = workspace_with_graph();
    let report = ws.cognitive_season();
    let _ = report.season; // accessible
    assert!(report.creation_rate >= 0.0);
}

#[test]
fn workspace_digital_shadows_on_fresh_workspace() {
    let (ws, _) = workspace_with_graph();
    // No focus events → no shadows
    let shadows = ws.digital_shadows();
    assert!(
        shadows.is_empty(),
        "fresh workspace without focus events should have no shadows"
    );
}

#[test]
fn workspace_resonant_pairs_on_similar_nodes() {
    let mut ws = SilentNodeWorkspace::new();
    ws.graph
        .add_node(make_node(
            NodeType::Idea,
            "rust programming systems engine memory",
        ))
        .unwrap();
    ws.graph
        .add_node(make_node(
            NodeType::Idea,
            "rust systems memory engine programming",
        ))
        .unwrap();
    ws.graph
        .add_node(make_node(
            NodeType::Memory,
            "baking bread recipes yeast flour",
        ))
        .unwrap();

    let pairs = ws.resonant_pairs();
    // The two rust nodes should be resonant
    assert!(
        !pairs.is_empty(),
        "similar rust nodes should produce resonance pairs"
    );
}

#[test]
fn workspace_thought_heatmap_returns_valid_structure() {
    let (mut ws, ids) = workspace_with_graph();
    // Add some focus events via public API equivalent
    // (directly construct as workspace doesn't expose add_focus_event in this context)
    let hm = ws.thought_heatmap(7);
    assert_eq!(hm.window_days, 7);
}

#[test]
fn workspace_oracle_signals_no_panic() {
    let (ws, _) = workspace_with_graph();
    let signals = ws.oracle_signals();
    // May or may not generate signals — just shouldn't panic
    for sig in &signals {
        assert!(sig.strength >= 0.0 && sig.strength <= 1.0);
    }
}
