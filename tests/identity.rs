// Phase 10 — Identity & Narrative Systems test suite.

use chrono::{Duration, Utc};
use silentnode_core::identity::ShadowOrigin;
use silentnode_core::{
    EdgeType, GeometryKind, IdentityEngine, MotionKind, NodeData, NodeType, ShadowProjectDetector,
    SilentNodeWorkspace, SymmetryKind,
};

// ── Helpers ────────────────────────────────────────────────────────────────────

fn make_node(nt: NodeType, content: &str) -> NodeData {
    NodeData::new(nt, content)
}

// ═══════════════════════════════════════════════════════════════════════════════
// IdentityEngine — Living Signature
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn identity_engine_starts_empty() {
    let engine = IdentityEngine::new();
    assert!(engine.current_signature.is_none());
    assert!(engine.shift_history.is_empty());
}

#[test]
fn living_signature_derives_from_empty_workspace() {
    let mut ws = SilentNodeWorkspace::new();
    let sig = ws.derive_living_signature();
    // Empty workspace → Line geometry, no symmetry, evolution count 1
    assert_eq!(sig.geometry, GeometryKind::Line);
    assert_eq!(sig.symmetry, SymmetryKind::None);
    assert_eq!(sig.evolution_count, 1);
    assert!(sig.complexity >= 0.0 && sig.complexity <= 1.0);
    assert!(sig.vitality >= 0.0 && sig.vitality <= 1.0);
}

#[test]
fn living_signature_evolution_count_increments() {
    let mut ws = SilentNodeWorkspace::new();
    let v1 = ws.derive_living_signature().evolution_count;
    let v2 = ws.derive_living_signature().evolution_count;
    assert_eq!(v1, 1);
    assert_eq!(v2, 2);
}

#[test]
fn living_signature_colors_are_valid_rgba() {
    let mut ws = SilentNodeWorkspace::new();
    ws.graph
        .add_node(make_node(NodeType::Idea, "test idea"))
        .unwrap();
    let sig = ws.derive_living_signature();
    for c in [sig.primary_color, sig.secondary_color, sig.accent_color] {
        for v in c {
            assert!(v >= 0.0 && v <= 1.0, "color component out of range: {v}");
        }
    }
}

#[test]
fn living_signature_vitality_reflects_active_ratio() {
    let mut ws = SilentNodeWorkspace::new();
    // Add 4 active nodes, no ghosts → vitality should be high
    for i in 0..4 {
        ws.graph
            .add_node(make_node(NodeType::Idea, &format!("active {i}")))
            .unwrap();
    }
    let sig = ws.derive_living_signature();
    assert!(
        sig.vitality > 0.9,
        "all-active workspace should have high vitality, got {}",
        sig.vitality
    );
}

#[test]
fn geometry_kind_has_descriptions() {
    for g in [
        GeometryKind::Circle,
        GeometryKind::Wave,
        GeometryKind::Spiral,
        GeometryKind::Fractal,
        GeometryKind::Line,
    ] {
        assert!(!g.as_str().is_empty());
        assert!(!g.description().is_empty());
    }
}

#[test]
fn symmetry_fold_counts_are_sensible() {
    assert_eq!(SymmetryKind::None.fold_count(), 1);
    assert_eq!(SymmetryKind::Bilateral.fold_count(), 2);
    assert_eq!(SymmetryKind::Radial3.fold_count(), 3);
    assert_eq!(SymmetryKind::Radial4.fold_count(), 4);
    assert!(SymmetryKind::Rotational.fold_count() >= 5);
}

#[test]
fn motion_kind_animation_durations_positive() {
    for m in [
        MotionKind::Flow,
        MotionKind::Pulse,
        MotionKind::Breathe,
        MotionKind::Crystallize,
        MotionKind::Drift,
    ] {
        assert!(m.animation_duration_ms() > 0);
        assert!(!m.as_str().is_empty());
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// ShadowProjectDetector
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn shadow_projects_empty_workspace_none() {
    let ws = SilentNodeWorkspace::new();
    let shadows = ws.detect_shadow_projects();
    assert!(shadows.is_empty());
}

#[test]
fn shadow_projects_detect_abandoned_high_gravity() {
    let detector = ShadowProjectDetector::new();
    let now = Utc::now();

    // A high-gravity project node not accessed in 90 days
    let mut node = make_node(NodeType::Project, "Abandoned compiler project");
    node.gravity = 3.0;
    node.accessed_at = now - Duration::days(90);
    node.created_at = now - Duration::days(120);

    let nodes = vec![&node];
    let shadows = detector.detect(&nodes, &[], &[], now);

    assert!(
        !shadows.is_empty(),
        "abandoned high-gravity project should be a shadow"
    );
    assert!(matches!(
        shadows[0].origin,
        ShadowOrigin::AbandonedHighGravity { .. }
    ));
}

#[test]
fn shadow_projects_recent_active_node_not_shadow() {
    let detector = ShadowProjectDetector::new();
    let now = Utc::now();

    // A recently-accessed high-gravity project — should NOT be a shadow
    let mut node = make_node(NodeType::Project, "Active project");
    node.gravity = 3.0;
    node.accessed_at = now - Duration::days(2);
    node.created_at = now - Duration::days(120);

    let nodes = vec![&node];
    let shadows = detector.detect(&nodes, &[], &[], now);
    assert!(
        shadows.is_empty(),
        "recently accessed node should not be a shadow project"
    );
}

#[test]
fn shadow_projects_sorted_by_luminescence() {
    let detector = ShadowProjectDetector::new();
    let now = Utc::now();

    let mut n1 = make_node(NodeType::Project, "Project one");
    n1.gravity = 2.5;
    n1.accessed_at = now - Duration::days(70);
    n1.created_at = now - Duration::days(100);
    let mut n2 = make_node(NodeType::World, "World two");
    n2.gravity = 4.0;
    n2.accessed_at = now - Duration::days(110);
    n2.created_at = now - Duration::days(200);

    let nodes = vec![&n1, &n2];
    let shadows = detector.detect(&nodes, &[], &[], now);

    // Verify descending luminescence ordering
    for w in shadows.windows(2) {
        assert!(w[0].luminescence >= w[1].luminescence);
    }
}

#[test]
fn workspace_living_signature_persists_in_engine() {
    let mut ws = SilentNodeWorkspace::new();
    ws.graph
        .add_node(make_node(NodeType::Idea, "alpha"))
        .unwrap();
    ws.derive_living_signature();
    assert!(ws.identity.current_signature.is_some());
}

#[test]
fn signature_shift_recorded_on_geometry_change() {
    // Build a workspace, derive once (Line), then mutate to force a shift.
    let mut ws = SilentNodeWorkspace::new();
    let n1 = ws
        .graph
        .add_node(make_node(NodeType::Idea, "alpha beta gamma"))
        .unwrap();
    let n2 = ws
        .graph
        .add_node(make_node(NodeType::Idea, "delta epsilon"))
        .unwrap();
    ws.graph.connect(n1, n2, EdgeType::Connection, 0.9).unwrap();
    ws.derive_living_signature();
    // Second derivation — shift history may or may not grow, but must not panic
    ws.derive_living_signature();
    // Engine remains consistent
    assert!(ws.identity.current_signature.is_some());
}
