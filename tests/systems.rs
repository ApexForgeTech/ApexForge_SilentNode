use silentnode_core::{
    derive_all_souls, CognitiveWeightSystem, EdgeType, EntropyEngine, FocusDepth, GlowPattern,
    GraphSnapshot, GravityEngine, NodeData, NodeType, ParticleStyle, ProjectSoul,
    SilentNodeWorkspace, TectonicDetector, WeatherState, WeatherSystem, WeightReport,
};
use uuid::Uuid;

// ─── Helper ──────────────────────────────────────────────────────────────────

fn make_node(nt: NodeType, content: &str) -> NodeData {
    NodeData::new(nt, content)
}

fn make_workspace_with_nodes() -> SilentNodeWorkspace {
    let mut ws = SilentNodeWorkspace::new();
    let n1 = ws
        .graph
        .add_node(make_node(NodeType::Idea, "Alpha"))
        .unwrap();
    let n2 = ws
        .graph
        .add_node(make_node(NodeType::Memory, "Beta"))
        .unwrap();
    let n3 = ws
        .graph
        .add_node(make_node(NodeType::Project, "Gamma Project"))
        .unwrap();
    ws.graph.connect(n1, n2, EdgeType::Connection, 0.8).unwrap();
    ws.graph.connect(n2, n3, EdgeType::Causal, 0.6).unwrap();
    ws
}

// ─── WeatherSystem ───────────────────────────────────────────────────────────

#[test]
fn weather_default_is_calm() {
    let w = WeatherSystem::new();
    assert_eq!(w.current.name(), "Calm");
    assert_eq!(w.transition_progress, 1.0);
}

#[test]
fn weather_tick_advances_transition() {
    let mut w = WeatherSystem::new();
    // Force a transition by manually setting transition_progress < 1
    w.transition_progress = 0.0;
    w.tick(0.5);
    assert!(w.transition_progress > 0.0);
    assert!(w.transition_progress <= 1.0);
}

#[test]
fn weather_blended_values_are_finite() {
    let w = WeatherSystem::new();
    assert!(w.blended_intensity().is_finite());
    assert!(w.blended_turbulence().is_finite());
    assert!(w.blended_pulse_rate().is_finite());
    assert!(w.blended_particle_speed().is_finite());
    let p = w.blended_primary();
    let s = w.blended_secondary();
    assert!(p.iter().all(|v| v.is_finite()));
    assert!(s.iter().all(|v| v.is_finite()));
}

#[test]
fn weather_derive_from_high_entropy_graph_goes_fading() {
    let mut ws = SilentNodeWorkspace::new();
    // Create a graph with many ghost nodes (high entropy → Fading state)
    for i in 0..10 {
        let mut n = make_node(NodeType::Ghost, &format!("ghost {i}"));
        n.is_ghost = true;
        n.entropy = 0.85;
        ws.graph.add_node(n).unwrap();
    }
    for _ in 0..4 {
        let n = make_node(NodeType::Idea, "normal");
        ws.graph.add_node(n).unwrap();
    }

    let mut weather = WeatherSystem::new();
    ws.derive_weather(&mut weather);
    // With >25% ghost ratio and high entropy, should derive Fading
    assert_eq!(
        weather.current.name(),
        "Fading",
        "expected Fading, got {}",
        weather.current.name()
    );
}

#[test]
fn weather_derives_with_empty_graph() {
    let ws = SilentNodeWorkspace::new();
    let mut weather = WeatherSystem::new();
    ws.derive_weather(&mut weather);
    // Empty graph → Calm (special case)
    assert_eq!(weather.current.name(), "Calm");
}

#[test]
fn weather_state_parameters_in_range() {
    let states = [
        WeatherState::Energetic {
            pulse_rate: 0.8,
            expansion: 0.6,
        },
        WeatherState::Calm {
            clarity: 0.7,
            stillness: 0.8,
        },
        WeatherState::Fading {
            dim_factor: 0.7,
            decay_speed: 0.4,
        },
        WeatherState::Reflective {
            ghost_visibility: 0.3,
            warmth: 0.5,
        },
        WeatherState::Turbulent {
            intensity: 0.6,
            chaos_frequency: 1.2,
        },
    ];
    for state in &states {
        assert!(
            state.intensity() >= 0.0 && state.intensity() <= 1.5,
            "{} intensity out of range: {}",
            state.name(),
            state.intensity()
        );
        assert!(
            state.turbulence() >= 0.0,
            "{} turbulence negative: {}",
            state.name(),
            state.turbulence()
        );
        assert!(
            state.pulse_rate() >= 0.0,
            "{} pulse_rate negative: {}",
            state.name(),
            state.pulse_rate()
        );
        assert!(
            state.particle_speed() >= 0.0,
            "{} particle_speed negative: {}",
            state.name(),
            state.particle_speed()
        );
    }
}

// ─── CognitiveWeightSystem ───────────────────────────────────────────────────

#[test]
fn weight_empty_graph_is_zero() {
    let ws = SilentNodeWorkspace::new();
    let report = ws.cognitive_weight();
    assert_eq!(report.total, 0.0);
    assert!(!report.is_heavy());
}

#[test]
fn weight_ghost_nodes_increase_total() {
    let mut ws = SilentNodeWorkspace::new();
    let mut n = make_node(NodeType::Ghost, "ghost");
    n.is_ghost = true;
    ws.graph.add_node(n).unwrap();

    let report = ws.cognitive_weight();
    assert!(report.total > 0.0, "ghosts must add weight");
    assert_eq!(report.ghost_nodes, 1);
}

#[test]
fn weight_isolated_nodes_count_correctly() {
    let sys = CognitiveWeightSystem::new();
    let isolated_ids = vec![Uuid::new_v4(), Uuid::new_v4()];
    let report = sys.calculate(&[], &[], &isolated_ids);
    assert_eq!(report.isolated_nodes, 2);
    assert!(report.total > 0.0);
}

#[test]
fn weight_caps_at_100() {
    let mut nodes: Vec<NodeData> = Vec::new();
    for _ in 0..100 {
        let mut n = make_node(NodeType::Ghost, "ghost");
        n.is_ghost = true;
        n.is_fossil = true;
        n.is_void = true;
        n.entropy = 1.0;
        nodes.push(n);
    }
    let refs: Vec<&NodeData> = nodes.iter().collect();
    let sys = CognitiveWeightSystem::new();
    let report = sys.calculate(&refs, &[], &[]);
    assert!(
        report.total <= 100.0,
        "weight must be capped at 100, got {}",
        report.total
    );
}

#[test]
fn weight_visual_density_above_one_when_heavy() {
    let mut nodes: Vec<NodeData> = Vec::new();
    for _ in 0..30 {
        let mut n = make_node(NodeType::Ghost, "g");
        n.is_ghost = true;
        nodes.push(n);
    }
    let refs: Vec<&NodeData> = nodes.iter().collect();
    let sys = CognitiveWeightSystem::new();
    let report = sys.calculate(&refs, &[], &[]);
    if report.is_heavy() {
        assert!(report.visual_density() > 1.0);
        assert!(report.particle_drag() > 1.0);
    }
}

#[test]
fn weight_summary_contains_key_fields() {
    let report = WeightReport {
        total: 42.5,
        ghost_nodes: 3,
        fossil_nodes: 1,
        void_nodes: 2,
        pending_contracts: 4,
        active_contracts: 1,
        isolated_nodes: 5,
    };
    let s = report.summary();
    assert!(s.contains("42.5"), "summary must contain total weight");
    assert!(s.contains("ghosts=3"));
    assert!(s.contains("isolated=5"));
}

// ─── ProjectSoul ─────────────────────────────────────────────────────────────

#[test]
fn soul_derived_from_project_node() {
    let node = make_node(NodeType::Project, "Phoenix OS");
    let soul = ProjectSoul::derive(&node, &[], 5, 20);
    assert_eq!(soul.project_id, node.id);
    assert!(soul.activity_level >= 0.0 && soul.activity_level <= 1.0);
    assert!(soul.maturity >= 0.0 && soul.maturity <= 1.0);
    assert!(soul.social_density >= 0.0 && soul.social_density <= 1.0);
}

#[test]
fn soul_colors_are_valid_rgba() {
    let node = make_node(NodeType::World, "NightOS");
    let soul = ProjectSoul::derive(&node, &[], 3, 10);
    for &v in soul.primary_color.iter().chain(soul.secondary_color.iter()) {
        assert!(v >= 0.0 && v <= 1.0, "color component out of [0,1]: {v}");
    }
}

#[test]
fn soul_pulse_params_non_negative() {
    let node = make_node(NodeType::Project, "ActiveProject");
    let soul = ProjectSoul::derive(&node, &[], 8, 20);
    assert!(soul.pulse_amplitude() >= 0.0);
    assert!(soul.pulse_frequency() >= 0.0);
}

#[test]
fn soul_deterministic_same_content() {
    let node = make_node(NodeType::Project, "DeterministicNode");
    let soul1 = ProjectSoul::derive(&node, &[], 3, 10);
    let soul2 = ProjectSoul::derive(&node, &[], 3, 10);
    assert_eq!(soul1.primary_color, soul2.primary_color);
    assert_eq!(soul1.secondary_color, soul2.secondary_color);
    assert_eq!(soul1.particle_style, soul2.particle_style);
    assert_eq!(soul1.glow_pattern, soul2.glow_pattern);
}

#[test]
fn soul_different_content_different_colors() {
    let n1 = make_node(NodeType::Project, "ProjectAlpha");
    let n2 = make_node(NodeType::Project, "ProjectBeta");
    let s1 = ProjectSoul::derive(&n1, &[], 3, 10);
    let s2 = ProjectSoul::derive(&n2, &[], 3, 10);
    // Two distinct content strings should almost certainly produce different hues
    assert_ne!(s1.primary_color, s2.primary_color);
}

#[test]
fn derive_all_souls_only_project_world() {
    let nodes = vec![
        make_node(NodeType::Project, "P1"),
        make_node(NodeType::Idea, "I1"), // excluded
        make_node(NodeType::World, "W1"),
        make_node(NodeType::Memory, "M1"), // excluded
        make_node(NodeType::Project, "P2"),
    ];
    let souls = derive_all_souls(nodes.into_iter(), &[], 5, None);
    assert_eq!(souls.len(), 3, "expected 3 souls (2 Project + 1 World)");
}

#[test]
fn fossil_node_soul_has_high_maturity() {
    let mut node = make_node(NodeType::Project, "Ancient");
    node.is_fossil = true;
    let soul = ProjectSoul::derive(&node, &[], 2, 10);
    assert_eq!(
        soul.maturity, 1.0,
        "fossil nodes must have maximum maturity"
    );
}

#[test]
fn workspace_derive_souls_returns_project_souls() {
    let ws = make_workspace_with_nodes();
    let souls = ws.derive_souls();
    // The workspace has 1 Project node
    assert_eq!(souls.len(), 1);
    let soul = &souls[0];
    assert!(matches!(
        soul.particle_style,
        ParticleStyle::Aggressive
            | ParticleStyle::Crystalline
            | ParticleStyle::Fluid
            | ParticleStyle::Organic
    ));
    assert!(matches!(
        soul.glow_pattern,
        GlowPattern::Pulse
            | GlowPattern::Radiate
            | GlowPattern::Breathe
            | GlowPattern::Flicker
            | GlowPattern::Steady
    ));
}

// ─── TectonicDetector ────────────────────────────────────────────────────────

#[test]
fn tectonic_self_comparison_is_zero() {
    let ws = make_workspace_with_nodes();
    let snapshot = ws.tectonic_snapshot();
    let detector = TectonicDetector::new();
    // Comparing a snapshot to the exact same graph should yield no event
    let result = ws.check_tectonics(&snapshot, &detector);
    assert!(
        result.is_none(),
        "self-comparison must not trigger a tectonic event"
    );
}

#[test]
fn tectonic_large_addition_triggers_event() {
    let ws_before = make_workspace_with_nodes();
    let snapshot = ws_before.tectonic_snapshot();

    let mut ws_after = ws_before.clone();
    // Add many nodes and edges to create a large structural shift
    let mut prev_id = None;
    for i in 0..20 {
        let n = ws_after
            .graph
            .add_node(make_node(NodeType::Idea, &format!("new {i}")))
            .unwrap();
        if let Some(prev) = prev_id {
            ws_after
                .graph
                .connect(prev, n, EdgeType::Connection, 0.5)
                .unwrap();
        }
        prev_id = Some(n);
    }

    let detector = TectonicDetector::new();
    let result = ws_after.check_tectonics(&snapshot, &detector);
    assert!(
        result.is_some(),
        "large node addition must trigger a tectonic event"
    );
    let event = result.unwrap();
    assert!(event.magnitude > 0.0);
    assert!(event.affected_node_count > 0);
    assert!(!event.description.is_empty());
}

#[test]
fn tectonic_event_magnitude_in_range() {
    let ws_before = SilentNodeWorkspace::new();
    let snapshot = ws_before.tectonic_snapshot();

    let mut ws_after = SilentNodeWorkspace::new();
    for i in 0..15 {
        ws_after
            .graph
            .add_node(make_node(NodeType::Memory, &format!("m{i}")))
            .unwrap();
    }

    let detector = TectonicDetector::new();
    if let Some(event) = ws_after.check_tectonics(&snapshot, &detector) {
        assert!(
            event.magnitude > 0.0 && event.magnitude <= 1.0,
            "magnitude must be in [0,1], got {}",
            event.magnitude
        );
    }
}

#[test]
fn tectonic_snapshot_captures_node_count() {
    let ws = make_workspace_with_nodes();
    let snapshot = ws.tectonic_snapshot();
    assert_eq!(snapshot.nodes.len(), ws.graph.node_count());
    assert_eq!(snapshot.edges.len(), ws.graph.edge_count());
}

#[test]
fn tectonic_detector_custom_threshold() {
    let detector = TectonicDetector::with_threshold(0.0);
    // A zero threshold means any change triggers an event
    let before = GraphSnapshot {
        nodes: Vec::new(),
        edges: Vec::new(),
        taken_at: chrono::Utc::now(),
    };
    let mut ws = SilentNodeWorkspace::new();
    ws.graph.add_node(make_node(NodeType::Idea, "x")).unwrap();
    // With threshold=0, even 1 node addition should trigger
    let result = ws.check_tectonics(&before, &detector);
    assert!(result.is_some(), "zero threshold must detect any change");
}

// ─── Integration: full Phase 4 pipeline ──────────────────────────────────────

#[test]
fn phase4_full_pipeline_on_seeded_workspace() {
    let mut ws = SilentNodeWorkspace::new();

    // Seed a richer graph
    let n1 = ws
        .graph
        .add_node(make_node(NodeType::Project, "CoreProject"))
        .unwrap();
    let n2 = ws
        .graph
        .add_node(make_node(NodeType::Idea, "BigIdea"))
        .unwrap();
    let n3 = ws
        .graph
        .add_node(make_node(NodeType::World, "Universe"))
        .unwrap();
    let mut ghost = make_node(NodeType::Ghost, "GhostTrace");
    ghost.is_ghost = true;
    ghost.entropy = 0.7;
    let _n4 = ws.graph.add_node(ghost).unwrap();
    ws.graph.connect(n1, n2, EdgeType::Connection, 0.9).unwrap();
    ws.graph.connect(n2, n3, EdgeType::Causal, 0.7).unwrap();

    // Add some focus events
    ws.record_focus(n1, 300.0, FocusDepth::DeepWork).unwrap();
    ws.record_focus(n2, 120.0, FocusDepth::Read).unwrap();

    // Phase 4: weather
    let mut weather = WeatherSystem::new();
    ws.derive_weather(&mut weather);
    assert!(!weather.current.name().is_empty());

    // Phase 4: cognitive weight
    let report = ws.cognitive_weight();
    assert!(report.ghost_nodes >= 1);
    assert!(report.total > 0.0);

    // Phase 4: souls
    let souls = ws.derive_souls();
    assert_eq!(souls.len(), 2, "expected souls for Project + World nodes");

    // Phase 4: tectonics — snapshot before, add more, compare
    let snapshot = ws.tectonic_snapshot();
    let gravity = GravityEngine::new();
    ws.step_gravity(&gravity, 1.0);
    ws.tick_entropy(&EntropyEngine::new());
    let detector = TectonicDetector::new();
    // After entropy/gravity tick, there may or may not be a tectonic event
    // — just verify it doesn't panic
    let _ = ws.check_tectonics(&snapshot, &detector);
}
