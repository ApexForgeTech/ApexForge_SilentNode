/// Phase 5 — Temporal Systems test suite.
use chrono::{Duration, NaiveDate, TimeZone, Utc};
use silentnode_core::{
    ArchaeologySession, ChangeType, DayComparison, DayReconstruction, FossilEngine,
    FossilizationCheck, LoreArcDetector, NodeData, NodeType, Position3, SilentNodeWorkspace,
    TemporalEngine, TemporalMarker,
};
use uuid::Uuid;

// ── helpers ──────────────────────────────────────────────────────────────────

fn fresh_node(nt: NodeType, content: &str) -> NodeData {
    NodeData::new(nt, content)
}

fn aged_node(nt: NodeType, content: &str, days_ago: i64, entropy: f32) -> NodeData {
    let mut n = NodeData::new(nt, content);
    n.created_at = Utc::now() - Duration::days(days_ago);
    n.accessed_at = Utc::now() - Duration::days(days_ago / 2);
    n.entropy = entropy;
    n
}

// ── TemporalEngine ────────────────────────────────────────────────────────────

#[test]
fn temporal_engine_starts_empty() {
    let engine = TemporalEngine::new();
    assert_eq!(engine.snapshot_count(), 0);
    assert_eq!(engine.tracked_node_count(), 0);
}

#[test]
fn record_change_increments_count() {
    let mut engine = TemporalEngine::new();
    let node = fresh_node(NodeType::Idea, "alpha");
    engine.record_change(&node, ChangeType::Created);
    assert_eq!(engine.snapshot_count(), 1);
    assert_eq!(engine.tracked_node_count(), 1);
}

#[test]
fn record_multiple_changes_same_node() {
    let mut engine = TemporalEngine::new();
    let mut node = fresh_node(NodeType::Idea, "beta");
    engine.record_change(&node, ChangeType::Created);
    node.entropy = 0.5;
    engine.record_change(&node, ChangeType::StateChanged);
    node.entropy = 0.1;
    engine.record_change(&node, ChangeType::Accessed);

    assert_eq!(engine.snapshot_count(), 3);
    assert_eq!(engine.tracked_node_count(), 1);
    let snaps = engine.snapshots_for(node.id);
    assert_eq!(snaps.len(), 3);
}

#[test]
fn reconstruct_node_at_returns_last_before_time() {
    let mut engine = TemporalEngine::new();
    let node = fresh_node(NodeType::Project, "gamma");
    engine.record_change(&node, ChangeType::Created);

    let at = Utc::now() + Duration::hours(1);
    let map = engine.reconstruct_node_at(node.id, at);
    assert!(map.is_some());
    let map = map.unwrap();
    assert_eq!(map["content"].as_str().unwrap(), "gamma");
}

#[test]
fn reconstruct_node_at_returns_none_before_creation() {
    let mut engine = TemporalEngine::new();
    let node = fresh_node(NodeType::Idea, "delta");
    engine.record_change(&node, ChangeType::Created);

    let before = Utc::now() - Duration::days(999);
    assert!(engine.reconstruct_node_at(node.id, before).is_none());
}

#[test]
fn reconstruct_universe_at_covers_all_nodes() {
    let mut engine = TemporalEngine::new();
    let n1 = fresh_node(NodeType::Idea, "node-1");
    let n2 = fresh_node(NodeType::Memory, "node-2");
    engine.record_change(&n1, ChangeType::Created);
    engine.record_change(&n2, ChangeType::Created);

    let universe = engine.reconstruct_universe_at(Utc::now() + Duration::hours(1));
    assert_eq!(universe.len(), 2);
}

#[test]
fn from_snapshots_round_trips() {
    let mut engine = TemporalEngine::new();
    let node = fresh_node(NodeType::Person, "alice");
    engine.record_change(&node, ChangeType::Created);
    let snaps = engine.all_snapshots().to_vec();

    let restored = TemporalEngine::from_snapshots(snaps);
    assert_eq!(restored.snapshot_count(), 1);
}

// ── ArchaeologySession ────────────────────────────────────────────────────────

#[test]
fn archaeology_open_none_when_no_history() {
    let engine = TemporalEngine::new();
    let session = ArchaeologySession::open(&engine, Uuid::new_v4());
    assert!(session.is_none());
}

#[test]
fn archaeology_open_some_with_history() {
    let mut engine = TemporalEngine::new();
    let node = fresh_node(NodeType::Idea, "test");
    engine.record_change(&node, ChangeType::Created);
    let session = ArchaeologySession::open(&engine, node.id);
    assert!(session.is_some());
}

#[test]
fn archaeology_depth_matches_snapshot_count() {
    let mut engine = TemporalEngine::new();
    let mut node = fresh_node(NodeType::Idea, "depth-test");
    engine.record_change(&node, ChangeType::Created);
    node.entropy = 0.3;
    engine.record_change(&node, ChangeType::StateChanged);
    node.entropy = 0.6;
    engine.record_change(&node, ChangeType::StateChanged);

    let session = ArchaeologySession::open(&engine, node.id).unwrap();
    assert_eq!(session.depth(), 3);
}

#[test]
fn archaeology_cursor_starts_at_latest() {
    let mut engine = TemporalEngine::new();
    let mut node = fresh_node(NodeType::Idea, "cursor");
    engine.record_change(&node, ChangeType::Created);
    node.entropy = 0.5;
    engine.record_change(&node, ChangeType::StateChanged);

    let session = ArchaeologySession::open(&engine, node.id).unwrap();
    assert_eq!(session.cursor(), 1);
}

#[test]
fn archaeology_descend_moves_cursor_back() {
    let mut engine = TemporalEngine::new();
    let mut node = fresh_node(NodeType::Idea, "descend");
    for i in 0..5 {
        node.entropy = i as f32 * 0.2;
        engine.record_change(&node, ChangeType::StateChanged);
    }
    let mut session = ArchaeologySession::open(&engine, node.id).unwrap();
    let steps = session.descend(3);
    assert_eq!(steps, 3);
    assert_eq!(session.cursor(), 1);
}

#[test]
fn archaeology_descend_clamps_at_zero() {
    let mut engine = TemporalEngine::new();
    let mut node = fresh_node(NodeType::Idea, "clamp");
    for _ in 0..3 {
        engine.record_change(&node, ChangeType::StateChanged);
        node.entropy += 0.1;
    }
    let mut session = ArchaeologySession::open(&engine, node.id).unwrap();
    let steps = session.descend(100);
    assert_eq!(session.cursor(), 0);
    assert!(steps <= 2);
}

#[test]
fn archaeology_resurrect_returns_node_data() {
    let mut engine = TemporalEngine::new();
    let node = fresh_node(NodeType::Project, "resurrect-me");
    engine.record_change(&node, ChangeType::Created);
    let session = ArchaeologySession::open(&engine, node.id).unwrap();
    let resurrected = session.resurrect();
    assert_eq!(resurrected.content, "resurrect-me");
    assert_eq!(resurrected.node_type, NodeType::Project);
}

#[test]
fn archaeology_mark_records_marker() {
    let mut engine = TemporalEngine::new();
    let node = fresh_node(NodeType::Idea, "mark-test");
    engine.record_change(&node, ChangeType::Created);
    let mut session = ArchaeologySession::open(&engine, node.id).unwrap();
    let marker = session.mark("interesting point");
    assert_eq!(marker.label, "interesting point");
    assert_eq!(session.markers().len(), 1);
}

#[test]
fn archaeology_compare_forward_produces_diff() {
    let mut engine = TemporalEngine::new();
    let mut node = fresh_node(NodeType::Idea, "diff-test");
    node.entropy = 0.1;
    engine.record_change(&node, ChangeType::Created);
    node.entropy = 0.8;
    engine.record_change(&node, ChangeType::StateChanged);

    let mut session = ArchaeologySession::open(&engine, node.id).unwrap();
    session.seek(0);
    let diff = session.compare_forward(1).unwrap();
    assert!(diff.entropy_delta > 0.0);
    assert!(!diff.changes.is_empty());
}

#[test]
fn archaeology_timeline_length_matches_depth() {
    let mut engine = TemporalEngine::new();
    let mut node = fresh_node(NodeType::Memory, "timeline");
    for _ in 0..4 {
        engine.record_change(&node, ChangeType::Accessed);
        node.access_count += 1;
    }
    let session = ArchaeologySession::open(&engine, node.id).unwrap();
    assert_eq!(session.timeline().len(), 4);
}

// ── FossilEngine ──────────────────────────────────────────────────────────────

#[test]
fn fossil_check_already_fossil_returns_false() {
    let engine = FossilEngine::new();
    let temporal = TemporalEngine::new();
    let mut node = fresh_node(NodeType::Fossil, "already");
    node.is_fossil = true;
    let check = engine.check_fossilization(&node, &temporal, 0, Utc::now());
    assert!(!check.qualifies);
}

#[test]
fn fossil_check_fresh_node_does_not_qualify() {
    let engine = FossilEngine::new();
    let temporal = TemporalEngine::new();
    let node = fresh_node(NodeType::Idea, "fresh");
    let check = engine.check_fossilization(&node, &temporal, 3, Utc::now());
    assert!(!check.qualifies);
}

#[test]
fn fossil_check_old_silent_high_entropy_qualifies() {
    let engine = FossilEngine::new();
    let temporal = TemporalEngine::new();
    let mut node = fresh_node(NodeType::Ghost, "ancient");
    node.is_ghost = true;
    node.entropy = 0.9;
    node.created_at = Utc::now() - Duration::days(60);
    node.accessed_at = Utc::now() - Duration::days(30);
    let check = engine.check_fossilization(&node, &temporal, 0, Utc::now());
    assert!(check.qualifies, "score={:.3}", check.score);
}

#[test]
fn fossil_check_score_increases_with_entropy() {
    let engine = FossilEngine::new();
    let temporal = TemporalEngine::new();
    let mut lo = fresh_node(NodeType::Idea, "lo");
    lo.entropy = 0.0;
    lo.created_at = Utc::now() - Duration::days(60);
    lo.accessed_at = Utc::now() - Duration::days(20);
    let mut hi = fresh_node(NodeType::Idea, "hi");
    hi.entropy = 0.9;
    hi.created_at = Utc::now() - Duration::days(60);
    hi.accessed_at = Utc::now() - Duration::days(20);

    let score_lo = engine
        .check_fossilization(&lo, &temporal, 3, Utc::now())
        .score;
    let score_hi = engine
        .check_fossilization(&hi, &temporal, 3, Utc::now())
        .score;
    assert!(score_hi > score_lo);
}

#[test]
fn fossil_engine_fossilize_mutates_node() {
    let engine = FossilEngine::new();
    let mut temporal = TemporalEngine::new();
    let mut node = fresh_node(NodeType::Idea, "candidate");
    engine.fossilize(&mut node, &mut temporal);
    assert!(node.is_fossil);
    assert!(node.is_ghost);
    assert_eq!(node.node_type, NodeType::Fossil);
}

#[test]
fn fossil_engine_excavate_restores_node() {
    let engine = FossilEngine::new();
    let mut temporal = TemporalEngine::new();
    let mut node = fresh_node(NodeType::Fossil, "fossil");
    node.is_fossil = true;
    node.is_ghost = true;
    let old_entropy = 0.8;
    node.entropy = old_entropy;
    engine.excavate(&mut node, &mut temporal, None);
    assert!(!node.is_fossil);
    assert!(node.is_ghost);
    assert_eq!(node.node_type, NodeType::Ghost);
    assert!(node.entropy < old_entropy);
}

#[test]
fn fossil_candidates_filter_correctly() {
    let engine = FossilEngine::new();
    let temporal = TemporalEngine::new();
    let mut old_ghost = fresh_node(NodeType::Ghost, "old");
    old_ghost.is_ghost = true;
    old_ghost.entropy = 0.9;
    old_ghost.created_at = Utc::now() - Duration::days(60);
    old_ghost.accessed_at = Utc::now() - Duration::days(30);

    let fresh = fresh_node(NodeType::Idea, "fresh");

    let nodes = vec![&old_ghost, &fresh];
    let candidates = engine.candidates(nodes.into_iter(), &temporal, |_| 0, Utc::now());
    assert!(!candidates.is_empty());
    assert!(candidates.iter().all(|c| c.qualifies));
}

// ── LoreArcDetector ───────────────────────────────────────────────────────────

#[test]
fn lore_empty_graph_produces_no_arcs() {
    let detector = LoreArcDetector::new();
    let engine = TemporalEngine::new();
    let lore = detector.detect_arcs(&[], &engine, &[], Utc::now());
    assert!(lore.is_empty());
}

#[test]
fn lore_detects_origin_arc_with_rapid_creation() {
    let mut engine = TemporalEngine::new();
    let mut node = fresh_node(NodeType::Project, "origin-node");
    engine.record_change(&node, ChangeType::Created);
    engine.record_change(&node, ChangeType::Connected);
    engine.record_change(&node, ChangeType::Connected);
    node.gravity = 0.9;
    engine.record_change(&node, ChangeType::Accessed);

    let detector = LoreArcDetector {
        min_significance: 0.1,
    };
    let lore = detector.detect_arcs(&[&node], &engine, &[], Utc::now());
    // At least one origin arc should be found
    let origins: Vec<_> = lore
        .iter()
        .filter(|e| matches!(e.arc_type, silentnode_core::ArcType::Origin))
        .collect();
    assert!(
        !origins.is_empty(),
        "expected origin arc, got {:?}",
        lore.iter()
            .map(|e| format!("{:?}", e.arc_type))
            .collect::<Vec<_>>()
    );
}

#[test]
fn lore_detects_conflict_arc_for_high_entropy_node() {
    let mut engine = TemporalEngine::new();
    let mut node = fresh_node(NodeType::Idea, "turbulent");
    node.entropy = 0.8;
    engine.record_change(&node, ChangeType::StateChanged);
    node.entropy = 0.85;
    engine.record_change(&node, ChangeType::StateChanged);
    node.entropy = 0.9;
    engine.record_change(&node, ChangeType::StateChanged);

    let detector = LoreArcDetector {
        min_significance: 0.1,
    };
    let lore = detector.detect_arcs(&[&node], &engine, &[], Utc::now());
    let conflicts: Vec<_> = lore
        .iter()
        .filter(|e| matches!(e.arc_type, silentnode_core::ArcType::Conflict))
        .collect();
    assert!(!conflicts.is_empty());
}

#[test]
fn lore_detects_transformation_arc_on_type_change() {
    let mut engine = TemporalEngine::new();
    let mut node = fresh_node(NodeType::Ghost, "morph");
    engine.record_change(&node, ChangeType::Created);
    node.node_type = NodeType::Idea;
    engine.record_change(&node, ChangeType::StateChanged);

    let detector = LoreArcDetector {
        min_significance: 0.1,
    };
    let lore = detector.detect_arcs(&[&node], &engine, &[], Utc::now());
    let transforms: Vec<_> = lore
        .iter()
        .filter(|e| matches!(e.arc_type, silentnode_core::ArcType::Transformation))
        .collect();
    assert!(!transforms.is_empty());
}

#[test]
fn lore_detects_legacy_arc_for_fossil() {
    let mut engine = TemporalEngine::new();
    let mut node = fresh_node(NodeType::Fossil, "legacy");
    node.is_fossil = true;
    engine.record_change(&node, ChangeType::Connected);
    engine.record_change(&node, ChangeType::Connected);
    engine.record_change(&node, ChangeType::Connected);
    engine.record_change(&node, ChangeType::StateChanged);

    let detector = LoreArcDetector {
        min_significance: 0.1,
    };
    let lore = detector.detect_arcs(&[&node], &engine, &[], Utc::now());
    let legacies: Vec<_> = lore
        .iter()
        .filter(|e| matches!(e.arc_type, silentnode_core::ArcType::Legacy))
        .collect();
    assert!(!legacies.is_empty());
}

#[test]
fn lore_sorted_by_significance_descending() {
    let mut engine = TemporalEngine::new();
    let mut high = fresh_node(NodeType::Project, "h");
    high.entropy = 0.9;
    high.gravity = 1.0;
    engine.record_change(&high, ChangeType::StateChanged);
    engine.record_change(&high, ChangeType::StateChanged);
    engine.record_change(&high, ChangeType::StateChanged);

    let mut low = fresh_node(NodeType::Fossil, "l");
    low.is_fossil = true;
    engine.record_change(&low, ChangeType::Connected);
    engine.record_change(&low, ChangeType::Connected);

    let detector = LoreArcDetector {
        min_significance: 0.0,
    };
    let lore = detector.detect_arcs(&[&high, &low], &engine, &[], Utc::now());
    for pair in lore.windows(2) {
        assert!(pair[0].significance >= pair[1].significance);
    }
}

// ── Workspace temporal integration ───────────────────────────────────────────

#[test]
fn workspace_snapshot_all_nodes_fills_temporal() {
    let mut ws = SilentNodeWorkspace::new();
    let n1 = ws
        .graph
        .add_node(NodeData::new(NodeType::Idea, "n1"))
        .unwrap();
    let n2 = ws
        .graph
        .add_node(NodeData::new(NodeType::Memory, "n2"))
        .unwrap();
    ws.snapshot_all_nodes();
    assert_eq!(ws.temporal_snapshot_count(), 2);
}

#[test]
fn workspace_record_temporal_change() {
    let mut ws = SilentNodeWorkspace::new();
    let id = ws
        .graph
        .add_node(NodeData::new(NodeType::Idea, "x"))
        .unwrap();
    ws.record_temporal_change(id, ChangeType::Accessed);
    assert_eq!(ws.temporal_snapshot_count(), 1);
}

#[test]
fn workspace_open_archaeology_none_without_history() {
    let ws = SilentNodeWorkspace::new();
    let session = ws.open_archaeology(Uuid::new_v4());
    assert!(session.is_none());
}

#[test]
fn workspace_open_archaeology_some_after_snapshot() {
    let mut ws = SilentNodeWorkspace::new();
    let id = ws
        .graph
        .add_node(NodeData::new(NodeType::Idea, "y"))
        .unwrap();
    ws.record_temporal_change(id, ChangeType::Created);
    let session = ws.open_archaeology(id);
    assert!(session.is_some());
}

#[test]
fn workspace_fossil_check_returns_some_for_existing_node() {
    let mut ws = SilentNodeWorkspace::new();
    let id = ws
        .graph
        .add_node(NodeData::new(NodeType::Idea, "old"))
        .unwrap();
    let check = ws.check_fossilization(id);
    assert!(check.is_some());
}

#[test]
fn workspace_fossil_check_returns_none_for_missing() {
    let ws = SilentNodeWorkspace::new();
    let check = ws.check_fossilization(Uuid::new_v4());
    assert!(check.is_none());
}

#[test]
fn workspace_fossilize_sets_fossil_flag() {
    let mut ws = SilentNodeWorkspace::new();
    let id = ws
        .graph
        .add_node(NodeData::new(NodeType::Idea, "candidate"))
        .unwrap();
    ws.fossilize_node(id).unwrap();
    let node = ws.graph.get_node(id).unwrap();
    assert!(node.is_fossil);
    assert_eq!(node.node_type, NodeType::Fossil);
}

#[test]
fn workspace_excavate_clears_fossil_flag() {
    let mut ws = SilentNodeWorkspace::new();
    let mut node = NodeData::new(NodeType::Fossil, "fossil");
    node.is_fossil = true;
    node.is_ghost = true;
    let id = ws.graph.add_node(node).unwrap();
    ws.excavate_node(id).unwrap();
    let node = ws.graph.get_node(id).unwrap();
    assert!(!node.is_fossil);
}

#[test]
fn workspace_detect_lore_runs_without_error() {
    let mut ws = SilentNodeWorkspace::new();
    let id = ws
        .graph
        .add_node(NodeData::new(NodeType::Project, "root"))
        .unwrap();
    ws.record_temporal_change(id, ChangeType::Created);
    let _lore = ws.detect_lore(&[]);
}

#[test]
fn workspace_reconstruct_day_returns_reconstruction() {
    let ws = SilentNodeWorkspace::new();
    let today = chrono::Utc::now().date_naive();
    let rec = ws.reconstruct_day(today);
    assert_eq!(rec.date, today);
}

#[test]
fn workspace_compare_days_produces_comparison() {
    let ws = SilentNodeWorkspace::new();
    let today = chrono::Utc::now().date_naive();
    let yesterday = today - chrono::Duration::days(1);
    let cmp = ws.compare_days(yesterday, today);
    assert_eq!(cmp.day_a, yesterday);
    assert_eq!(cmp.day_b, today);
}

#[test]
fn workspace_snapshot_persists_temporal_snapshots() {
    let mut ws = SilentNodeWorkspace::new();
    let id = ws
        .graph
        .add_node(NodeData::new(NodeType::Idea, "persist"))
        .unwrap();
    ws.record_temporal_change(id, ChangeType::Created);
    let snap = ws.snapshot();
    assert_eq!(snap.temporal_snapshots.len(), 1);
}

#[test]
fn workspace_from_snapshot_restores_temporal() {
    let mut ws = SilentNodeWorkspace::new();
    let id = ws
        .graph
        .add_node(NodeData::new(NodeType::Idea, "restore"))
        .unwrap();
    ws.record_temporal_change(id, ChangeType::Created);
    let snap = ws.snapshot();
    let ws2 = SilentNodeWorkspace::from_snapshot(snap).unwrap();
    assert_eq!(ws2.temporal_snapshot_count(), 1);
}
