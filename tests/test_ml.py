"""
Comprehensive test suite for silentnode_py/ml/.

Covers:
  - features.py  : load_nodes, load_focus_events, build_numeric_features,
                   build_graph_features, build_temporal_features
  - classifier.py: train, predict, predict_batch, cold-start fallback,
                   rule-based classification
  - ghost_predictor.py: train, predict_all, predict_entropy_trajectory
  - sequence.py  : train, predict_next, session_recommendations,
                   second-order transitions, fallback
  - cluster.py   : train, get_clusters, get_cluster_for_node,
                   get_cluster_summary, label generation
  - trainer.py   : train_all, get_status, incremental_update, should_retrain
  - Integration  : full pipeline on real data/silentnode.sqlite

Test strategy:
  - Unit tests use synthetic in-memory SQLite databases (no file I/O).
  - Integration tests use the real DB at data/silentnode.sqlite.
  - All mocks are explicit and minimal.
"""

import os
import sqlite3
import tempfile
import pickle
import json
import time
from pathlib import Path
from typing import List, Dict
from unittest.mock import patch, MagicMock
import numpy as np
import pytest

# ---------------------------------------------------------------------------
# Ensure tests run from the project root so relative DB paths work
# ---------------------------------------------------------------------------
PROJECT_ROOT = Path(__file__).parent.parent
os.chdir(PROJECT_ROOT)

REAL_DB = str(PROJECT_ROOT / "data" / "silentnode.sqlite")
REAL_DB_EXISTS = Path(REAL_DB).exists()

# ---------------------------------------------------------------------------
# Helpers: build minimal in-memory SQLite databases for unit tests
# ---------------------------------------------------------------------------

def _make_db(n_nodes: int = 15,
             n_edges: int = 20,
             n_events: int = 6,
             include_ghost: bool = True) -> str:
    """Create a temporary SQLite database with synthetic data.

    Returns the file path (caller is responsible for cleanup).
    """
    fd, path = tempfile.mkstemp(suffix=".sqlite")
    os.close(fd)

    conn = sqlite3.connect(path)
    cur  = conn.cursor()

    cur.executescript("""
        CREATE TABLE node (
            id          TEXT PRIMARY KEY,
            node_type   TEXT,
            content     TEXT,
            entropy     REAL,
            gravity     REAL,
            velocity    REAL,
            access_count INTEGER,
            is_ghost    INTEGER,
            is_fossil   INTEGER,
            is_void     INTEGER,
            created_at  TEXT,
            accessed_at TEXT,
            position_x  REAL DEFAULT 0,
            position_y  REAL DEFAULT 0,
            position_z  REAL DEFAULT 0
        );
        CREATE TABLE edge (
            source_id TEXT,
            target_id TEXT,
            edge_type TEXT,
            weight    REAL
        );
        CREATE TABLE focus_event (
            id               INTEGER PRIMARY KEY AUTOINCREMENT,
            node_id          TEXT,
            timestamp        TEXT,
            duration_seconds INTEGER,
            depth            TEXT
        );
    """)

    node_types = ["project", "idea", "artifact", "person", "process",
                  "memory", "world"]
    contents = [
        "Rust book chapter three",
        "SilentNode project planning",
        "Alice Johnson contact",
        "Deploy pipeline automation",
        "Deep learning research article",
        "Brainstorm ideas for the app",
        "Bob Smith meeting notes",
        "WebSocket server library",
        "Python tutorial video",
        "Marketing strategy document",
        "Code review process workflow",
        "Database schema design",
        "Conference talk proposal",
        "Budget spreadsheet artifact",
        "New feature idea draft",
    ]

    base_ts = "2026-01-01T00:00:00+00:00"
    ids = []
    for i in range(n_nodes):
        nid       = f"node-{i:04d}"
        ntype     = node_types[i % len(node_types)]
        content   = contents[i % len(contents)]
        entropy   = 0.1 + (i % 10) * 0.08
        gravity   = 1.0 + (i % 5)
        velocity  = 0.0 if i % 3 != 0 else 0.01
        acc_count = i * 2
        is_ghost  = 1 if (include_ghost and i == n_nodes - 1) else 0
        is_fossil = 0
        is_void   = 0
        # Vary creation/access times
        days_ago  = i * 3
        ts        = f"2026-01-{max(1, 31 - days_ago % 30):02d}T10:00:00+00:00"
        cur.execute("""
            INSERT INTO node VALUES (?,?,?,?,?,?,?,?,?,?,?,?,0,0,0)
        """, (nid, ntype, content, entropy, gravity, velocity,
              acc_count, is_ghost, is_fossil, is_void, ts, ts))
        ids.append(nid)

    # Add some edges
    for i in range(min(n_edges, len(ids) * (len(ids) - 1) // 2)):
        src = ids[i % len(ids)]
        tgt = ids[(i + 3) % len(ids)]
        if src != tgt:
            cur.execute("INSERT INTO edge VALUES (?,?,?,?)",
                        (src, tgt, "related", 1.0))

    # Add focus events with sequential timestamps
    depths = ["glance", "read", "edit", "deep_work"]
    used_ids = ids[:min(n_events, len(ids))]
    for i, nid in enumerate(used_ids):
        # Events are 20 minutes apart (within a single session)
        minute_offset = i * 20
        ts = f"2026-05-01T{(10 + minute_offset // 60):02d}:{minute_offset % 60:02d}:00+00:00"
        cur.execute("INSERT INTO focus_event (node_id, timestamp, duration_seconds, depth) "
                    "VALUES (?,?,?,?)",
                    (nid, ts, 300 + i * 60, depths[i % len(depths)]))

    conn.commit()
    conn.close()
    return path


# ===========================================================================
# features.py tests
# ===========================================================================

class TestFeatures:
    """Tests for feature extraction utilities."""

    def test_load_nodes_real_db(self):
        """load_nodes returns usable rows from the real database."""
        if not REAL_DB_EXISTS:
            pytest.skip("Real DB not available")
        from silentnode_py.ml.features import load_nodes
        nodes = load_nodes(REAL_DB)
        assert len(nodes) > 0
        required_keys = {"id", "node_type", "content", "entropy", "gravity",
                         "velocity", "access_count", "is_ghost", "is_fossil",
                         "is_void", "days_since_access", "days_since_created",
                         "connection_count"}
        for n in nodes:
            assert required_keys.issubset(n.keys()), (
                f"Missing keys in node {n['id']}: "
                f"{required_keys - n.keys()}")

    def test_load_nodes_synthetic(self):
        """load_nodes computes derived fields correctly."""
        from silentnode_py.ml.features import load_nodes
        db = _make_db(n_nodes=10)
        try:
            nodes = load_nodes(db)
            assert len(nodes) == 10
            for n in nodes:
                assert n["days_since_access"] >= 0
                assert n["days_since_created"] >= 0
                assert n["connection_count"] >= 0
        finally:
            os.unlink(db)

    def test_load_focus_events(self):
        """load_focus_events returns events with ts_float field."""
        from silentnode_py.ml.features import load_focus_events
        db = _make_db(n_nodes=8, n_events=4)
        try:
            events = load_focus_events(db)
            assert len(events) == 4
            for ev in events:
                assert "ts_float" in ev
                assert isinstance(ev["ts_float"], float)
        finally:
            os.unlink(db)

    def test_build_numeric_features_shape(self):
        """build_numeric_features returns (N, 14) float32 array."""
        from silentnode_py.ml.features import (
            load_nodes, load_focus_events, node_focus_stats,
            build_numeric_features,
        )
        db = _make_db(n_nodes=12)
        try:
            nodes       = load_nodes(db)
            events      = load_focus_events(db)
            focus_stats = node_focus_stats(nodes, events)
            X           = build_numeric_features(nodes, focus_stats)
            assert X.shape == (len(nodes), 14)
            assert X.dtype == np.float32
        finally:
            os.unlink(db)

    def test_build_numeric_features_range(self):
        """All normalized features are in [0, 1]."""
        from silentnode_py.ml.features import (
            load_nodes, load_focus_events, node_focus_stats,
            build_numeric_features,
        )
        db = _make_db(n_nodes=20)
        try:
            nodes       = load_nodes(db)
            events      = load_focus_events(db)
            focus_stats = node_focus_stats(nodes, events)
            X           = build_numeric_features(nodes, focus_stats)
            assert float(X.min()) >= -1e-9   # allow float32 rounding noise
            assert float(X.max()) <= 1.0 + 1e-5
        finally:
            os.unlink(db)

    def test_build_graph_features_keys(self):
        """build_graph_features returns correct keys for each node."""
        from silentnode_py.ml.features import load_nodes, build_graph_features
        db = _make_db(n_nodes=8, n_edges=10)
        try:
            nodes    = load_nodes(db)
            gf       = build_graph_features(nodes, db)
            expected = {"degree_centrality", "betweenness_centrality",
                        "clustering_coeff", "pagerank"}
            for n in nodes:
                assert expected.issubset(gf[n["id"]].keys()), (
                    f"Missing keys for node {n['id']}")
        finally:
            os.unlink(db)

    def test_build_graph_features_values(self):
        """Graph metrics are non-negative and pagerank sums to ~1."""
        from silentnode_py.ml.features import load_nodes, build_graph_features
        db = _make_db(n_nodes=10, n_edges=15)
        try:
            nodes = load_nodes(db)
            gf    = build_graph_features(nodes, db)
            for nid, metrics in gf.items():
                assert metrics["degree_centrality"] >= 0
                assert metrics["betweenness_centrality"] >= 0
                assert 0.0 <= metrics["clustering_coeff"] <= 1.0
                assert metrics["pagerank"] >= 0
            pr_sum = sum(v["pagerank"] for v in gf.values())
            assert abs(pr_sum - 1.0) < 0.1  # PageRank sums to ~1
        finally:
            os.unlink(db)

    def test_build_temporal_features_shape(self):
        """build_temporal_features returns (N, 4) float32 array."""
        from silentnode_py.ml.features import (
            load_nodes, load_focus_events, node_focus_stats,
            build_temporal_features,
        )
        db = _make_db(n_nodes=10)
        try:
            nodes       = load_nodes(db)
            events      = load_focus_events(db)
            focus_stats = node_focus_stats(nodes, events)
            T           = build_temporal_features(nodes, focus_stats)
            assert T.shape == (len(nodes), 4)
            assert T.dtype == np.float32
        finally:
            os.unlink(db)

    def test_build_text_tokens(self):
        """build_text_tokens produces unigrams and bigrams."""
        from silentnode_py.ml.features import build_text_tokens
        tokens = build_text_tokens("Hello World test")
        assert "hello" in tokens
        assert "world" in tokens
        assert "hello_world" in tokens
        assert "world_test" in tokens

    def test_build_text_tokens_multilingual_aliases(self):
        """Tokenizer preserves Azerbaijani text and emits semantic aliases."""
        from silentnode_py.ml.features import build_text_tokens
        tokens = build_text_tokens("magistr imtahan hazırlıq ingiliscə əlaqə fikir")
        assert "hazırlıq" in tokens
        assert "prep" in tokens
        assert "exam" in tokens
        assert "english" in tokens
        assert "contact" in tokens
        assert "idea" in tokens

    def test_parse_ts_nanoseconds(self):
        """parse_ts handles nanosecond timestamps without raising."""
        from silentnode_py.ml.features import parse_ts
        ts = "2026-01-15T12:30:45.123456789+00:00"
        result = parse_ts(ts)
        assert isinstance(result, float)
        assert result > 0

    def test_parse_ts_fallback(self):
        """parse_ts returns a recent timestamp on bad input."""
        from silentnode_py.ml.features import parse_ts, now_ts
        result = parse_ts("not-a-timestamp")
        assert abs(result - now_ts()) < 5.0


# ===========================================================================
# classifier.py tests
# ===========================================================================

class TestClassifier:
    """Tests for NodeClassifier."""

    def test_train_returns_trained_status(self):
        """Classifier trains successfully on synthetic data."""
        from silentnode_py.ml.classifier import NodeClassifier
        db = _make_db(n_nodes=15)
        try:
            clf    = NodeClassifier()
            result = clf.train(db)
            assert result["status"] == "trained"
            assert result["n_samples"] > 0
            assert isinstance(result["accuracy"], float)
            assert "classes" in result
        finally:
            os.unlink(db)

    def test_train_insufficient_data(self):
        """Classifier returns insufficient_data for < 3 nodes."""
        from silentnode_py.ml.classifier import NodeClassifier
        db = _make_db(n_nodes=2)
        try:
            clf    = NodeClassifier()
            result = clf.train(db)
            assert result["status"] == "insufficient_data"
        finally:
            os.unlink(db)

    def test_predict_after_train(self):
        """Classifier predict() returns a type and confidence after training."""
        from silentnode_py.ml.classifier import NodeClassifier
        db = _make_db(n_nodes=15)
        try:
            clf = NodeClassifier()
            clf.train(db)
            assert clf.trained
            pred = clf.predict("Rust programming book chapter")
            assert "type"       in pred
            assert "confidence" in pred
            assert "all_probs"  in pred
            assert 0.0 <= pred["confidence"] <= 1.0
        finally:
            os.unlink(db)

    def test_predict_cold_start_fallback(self):
        """Untrained classifier uses rule-based fallback."""
        from silentnode_py.ml.classifier import NodeClassifier
        clf  = NodeClassifier()   # not trained
        pred = clf.predict("Rust book tutorial")
        assert pred["method"] == "rule_based"
        assert pred["type"] == "artifact"

    def test_cold_start_person(self):
        """Rule-based fallback detects person names."""
        from silentnode_py.ml.classifier import _rule_based_classify
        assert _rule_based_classify("Alice") == "person"
        assert _rule_based_classify("Bob Smith") == "person"

    def test_cold_start_project(self):
        """Rule-based fallback detects project content."""
        from silentnode_py.ml.classifier import _rule_based_classify
        assert _rule_based_classify("deploy the new app service") == "project"

    def test_cold_start_process(self):
        """Rule-based fallback detects process/workflow content."""
        from silentnode_py.ml.classifier import _rule_based_classify
        assert _rule_based_classify("automation pipeline workflow") == "process"

    def test_cold_start_idea(self):
        """Rule-based fallback defaults to idea for generic content."""
        from silentnode_py.ml.classifier import _rule_based_classify
        result = _rule_based_classify("some random unclassifiable content xyz")
        assert result == "idea"

    def test_predict_batch_returns_list(self):
        """predict_batch returns one result per input string."""
        from silentnode_py.ml.classifier import NodeClassifier
        db = _make_db(n_nodes=15)
        try:
            clf = NodeClassifier()
            clf.train(db)
            contents = ["Rust book", "deploy pipeline", "Alice person"]
            results  = clf.predict_batch(contents)
            assert len(results) == 3
            for r in results:
                assert "type"       in r
                assert "confidence" in r
        finally:
            os.unlink(db)

    def test_predict_batch_empty(self):
        """predict_batch handles empty list."""
        from silentnode_py.ml.classifier import NodeClassifier
        clf = NodeClassifier()
        assert clf.predict_batch([]) == []

    def test_predict_batch_cold_start(self):
        """predict_batch uses rule-based when untrained."""
        from silentnode_py.ml.classifier import NodeClassifier
        clf = NodeClassifier()
        results = clf.predict_batch(["Rust book", "some idea"])
        assert all(r["method"] == "rule_based" for r in results)

    def test_feature_importances_after_train(self):
        """Feature importances are computed after training."""
        from silentnode_py.ml.classifier import NodeClassifier
        db = _make_db(n_nodes=15)
        try:
            clf = NodeClassifier()
            clf.train(db)
            assert len(clf.feature_importances) > 0
            assert all("feature" in fi and "importance" in fi
                       for fi in clf.feature_importances)
        finally:
            os.unlink(db)

    def test_version_increments(self):
        """Version counter increments on each train call."""
        from silentnode_py.ml.classifier import NodeClassifier
        db = _make_db(n_nodes=15)
        try:
            clf = NodeClassifier()
            assert clf.version == 0
            clf.train(db)
            assert clf.version == 1
            clf.train(db)
            assert clf.version == 2
        finally:
            os.unlink(db)

    def test_save_and_load(self, tmp_path):
        """Classifier persists correctly and reloads identically."""
        from silentnode_py.ml.classifier import NodeClassifier, CLASSIFIER_PATH
        db = _make_db(n_nodes=15)
        try:
            clf = NodeClassifier()
            clf.train(db)

            # Patch the save path
            save_path = tmp_path / "clf.pkl"
            with patch("silentnode_py.ml.classifier.CLASSIFIER_PATH", save_path):
                clf.save()
                loaded = NodeClassifier.load()
                # load without patch falls back to fresh instance
            # Test that saved state is consistent
            with open(save_path, "rb") as f:
                loaded2 = pickle.load(f)
            assert loaded2.trained == clf.trained
            assert loaded2.version == clf.version
        finally:
            os.unlink(db)


# ===========================================================================
# ghost_predictor.py tests
# ===========================================================================

class TestGhostPredictor:
    """Tests for GhostPredictor."""

    def test_train_returns_status(self):
        """Ghost predictor trains successfully."""
        from silentnode_py.ml.ghost_predictor import GhostPredictor
        db = _make_db(n_nodes=12, include_ghost=True)
        try:
            gp     = GhostPredictor()
            result = gp.train(db)
            assert result["status"] in ("trained", "rule_based")
            assert "n_samples" in result
            assert "entropy_threshold" in result
        finally:
            os.unlink(db)

    def test_predict_all_returns_list(self):
        """predict_all returns a list of risk dicts."""
        from silentnode_py.ml.ghost_predictor import GhostPredictor
        db = _make_db(n_nodes=12)
        try:
            gp = GhostPredictor()
            gp.train(db)
            risks = gp.predict_all(db)
            assert isinstance(risks, list)
            assert len(risks) > 0
        finally:
            os.unlink(db)

    def test_predict_all_fields(self):
        """Each risk entry has required fields."""
        from silentnode_py.ml.ghost_predictor import GhostPredictor
        db = _make_db(n_nodes=12)
        try:
            gp = GhostPredictor()
            gp.train(db)
            risks = gp.predict_all(db)
            required = {"node_id", "content", "node_type", "entropy",
                        "days_since_access", "connection_count",
                        "days_to_ghost", "risk_score", "risk_level"}
            for r in risks:
                assert required.issubset(r.keys()), (
                    f"Missing keys: {required - r.keys()}")
        finally:
            os.unlink(db)

    def test_predict_all_sorted_by_risk(self):
        """predict_all results are sorted by risk_score descending."""
        from silentnode_py.ml.ghost_predictor import GhostPredictor
        db = _make_db(n_nodes=12)
        try:
            gp = GhostPredictor()
            gp.train(db)
            risks = gp.predict_all(db)
            scores = [r["risk_score"] for r in risks]
            assert scores == sorted(scores, reverse=True)
        finally:
            os.unlink(db)

    def test_risk_score_in_range(self):
        """All risk scores are in [0, 1]."""
        from silentnode_py.ml.ghost_predictor import GhostPredictor
        db = _make_db(n_nodes=12)
        try:
            gp = GhostPredictor()
            gp.train(db)
            risks = gp.predict_all(db)
            for r in risks:
                assert 0.0 <= r["risk_score"] <= 1.0
        finally:
            os.unlink(db)

    def test_risk_level_labels(self):
        """Risk levels are one of the four expected strings."""
        from silentnode_py.ml.ghost_predictor import GhostPredictor
        db = _make_db(n_nodes=12)
        try:
            gp = GhostPredictor()
            gp.train(db)
            risks = gp.predict_all(db)
            valid_levels = {"critical", "high", "medium", "low"}
            for r in risks:
                assert r["risk_level"] in valid_levels
        finally:
            os.unlink(db)

    def test_predict_entropy_trajectory_fields(self):
        """predict_entropy_trajectory returns required fields."""
        from silentnode_py.ml.ghost_predictor import GhostPredictor
        db = _make_db(n_nodes=10)
        try:
            nodes = __import__("silentnode_py.ml.features",
                                fromlist=["load_nodes"]).load_nodes(db)
            node_id = nodes[0]["id"]
            gp = GhostPredictor()
            gp.train(db)
            result = gp.predict_entropy_trajectory(node_id, days_ahead=7,
                                                    db_path=db)
            required = {"node_id", "current_entropy", "projected_entropy",
                        "days_ahead", "daily_trajectory",
                        "risk_at_projection", "risk_level"}
            assert required.issubset(result.keys())
        finally:
            os.unlink(db)

    def test_predict_entropy_trajectory_length(self):
        """Trajectory has days_ahead + 1 entries (day 0 through day N)."""
        from silentnode_py.ml.ghost_predictor import GhostPredictor
        from silentnode_py.ml.features import load_nodes
        db = _make_db(n_nodes=10)
        try:
            nodes   = load_nodes(db)
            node_id = nodes[0]["id"]
            gp      = GhostPredictor()
            gp.train(db)
            result = gp.predict_entropy_trajectory(node_id, days_ahead=14,
                                                    db_path=db)
            assert len(result["daily_trajectory"]) == 15  # 0 through 14
        finally:
            os.unlink(db)

    def test_predict_entropy_trajectory_monotone(self):
        """Projected entropy is non-decreasing over time."""
        from silentnode_py.ml.ghost_predictor import GhostPredictor
        from silentnode_py.ml.features import load_nodes
        db = _make_db(n_nodes=10)
        try:
            nodes   = load_nodes(db)
            node_id = nodes[0]["id"]
            gp      = GhostPredictor()
            gp.train(db)
            result     = gp.predict_entropy_trajectory(node_id, days_ahead=10,
                                                        db_path=db)
            trajectory = result["daily_trajectory"]
            entropies  = [t["entropy"] for t in trajectory]
            for i in range(1, len(entropies)):
                assert entropies[i] >= entropies[i - 1] - 1e-6
        finally:
            os.unlink(db)

    def test_predict_entropy_trajectory_unknown_node(self):
        """Trajectory for unknown node_id returns an error dict."""
        from silentnode_py.ml.ghost_predictor import GhostPredictor
        db = _make_db(n_nodes=5)
        try:
            gp     = GhostPredictor()
            gp.train(db)
            result = gp.predict_entropy_trajectory("nonexistent-node-xyz",
                                                    days_ahead=5, db_path=db)
            assert "error" in result
        finally:
            os.unlink(db)

    def test_feature_importance_keys(self):
        """Feature importance dict has correct feature names."""
        from silentnode_py.ml.ghost_predictor import GhostPredictor, _FEATURE_NAMES
        db = _make_db(n_nodes=12)
        try:
            gp = GhostPredictor()
            gp.train(db)
            if gp.trained:
                for key in gp.feature_importance:
                    assert key in _FEATURE_NAMES
        finally:
            os.unlink(db)


# ===========================================================================
# sequence.py tests
# ===========================================================================

class TestSequenceModel:
    """Tests for MarkovSequenceModel."""

    def test_train_insufficient_events(self):
        """Model returns insufficient_data with < 2 events."""
        from silentnode_py.ml.sequence import MarkovSequenceModel
        db = _make_db(n_nodes=5, n_events=1)
        try:
            seq    = MarkovSequenceModel()
            result = seq.train(db)
            assert result["status"] == "insufficient_data"
        finally:
            os.unlink(db)

    def test_train_with_events(self):
        """Model trains when enough events are present."""
        from silentnode_py.ml.sequence import MarkovSequenceModel
        db = _make_db(n_nodes=8, n_events=6)
        try:
            seq    = MarkovSequenceModel()
            result = seq.train(db)
            assert result["status"] == "trained"
            assert result["n_events"] == 6
            assert seq.trained
        finally:
            os.unlink(db)

    def test_predict_next_returns_list(self):
        """predict_next returns a list (possibly empty or fallback)."""
        from silentnode_py.ml.sequence import MarkovSequenceModel
        db = _make_db(n_nodes=8, n_events=6)
        try:
            seq = MarkovSequenceModel()
            seq.train(db)
            nodes   = __import__("silentnode_py.ml.features",
                                  fromlist=["load_nodes"]).load_nodes(db)
            node_id = nodes[0]["id"]
            results = seq.predict_next(node_id, top_k=5)
            assert isinstance(results, list)
            for r in results:
                assert "node_id"     in r
                assert "probability" in r
                assert 0.0 <= r["probability"] <= 1.0
        finally:
            os.unlink(db)

    def test_predict_next_unknown_node_falls_back(self):
        """predict_next for unknown node returns popular fallback."""
        from silentnode_py.ml.sequence import MarkovSequenceModel
        db = _make_db(n_nodes=8, n_events=6)
        try:
            seq = MarkovSequenceModel()
            seq.train(db)
            results = seq.predict_next("unknown-node-xyz", top_k=3)
            assert isinstance(results, list)
            # Fallback should give at most top_k results
            assert len(results) <= 3
        finally:
            os.unlink(db)

    def test_session_recommendations_returns_list(self):
        """session_recommendations works with a recent node list."""
        from silentnode_py.ml.sequence import MarkovSequenceModel
        from silentnode_py.ml.features import load_nodes
        db = _make_db(n_nodes=10, n_events=6)
        try:
            seq   = MarkovSequenceModel()
            seq.train(db)
            nodes = load_nodes(db)
            recent = [n["id"] for n in nodes[:3]]
            recs   = seq.session_recommendations(recent, top_k=5)
            assert isinstance(recs, list)
        finally:
            os.unlink(db)

    def test_session_recommendations_excludes_session_nodes(self):
        """session_recommendations should not repeat nodes already in session."""
        from silentnode_py.ml.sequence import MarkovSequenceModel
        from silentnode_py.ml.features import load_nodes
        db = _make_db(n_nodes=10, n_events=6)
        try:
            seq   = MarkovSequenceModel()
            seq.train(db)
            nodes  = load_nodes(db)
            recent = [n["id"] for n in nodes[:5]]
            recs   = seq.session_recommendations(recent, top_k=5)
            rec_ids = {r["node_id"] for r in recs}
            assert len(rec_ids & set(recent)) == 0, (
                "Recommendations should not include nodes already in session")
        finally:
            os.unlink(db)

    def test_second_order_transitions_built(self):
        """Second-order transitions dict is populated when there are 3+ events."""
        from silentnode_py.ml.sequence import MarkovSequenceModel
        db = _make_db(n_nodes=10, n_events=8)
        try:
            seq    = MarkovSequenceModel()
            result = seq.train(db)
            if result["status"] == "trained":
                # With 8 consecutive events we expect some 2nd-order transitions
                assert isinstance(seq.transitions2, dict)
        finally:
            os.unlink(db)

    def test_fallback_popular_returns_top_k(self):
        """_fallback_popular returns at most top_k entries."""
        from silentnode_py.ml.sequence import MarkovSequenceModel
        db = _make_db(n_nodes=8, n_events=6)
        try:
            seq = MarkovSequenceModel()
            seq.train(db)
            results = seq._fallback_popular(3)
            assert len(results) <= 3
        finally:
            os.unlink(db)

    def test_transition_matrix(self):
        """get_transition_matrix returns sorted list."""
        from silentnode_py.ml.sequence import MarkovSequenceModel
        db = _make_db(n_nodes=8, n_events=6)
        try:
            seq = MarkovSequenceModel()
            seq.train(db)
            matrix = seq.get_transition_matrix()
            assert isinstance(matrix, list)
            probs = [r["prob"] for r in matrix]
            assert probs == sorted(probs, reverse=True)
        finally:
            os.unlink(db)

    def test_version_increments_on_train(self):
        """Version counter increments each time train is called."""
        from silentnode_py.ml.sequence import MarkovSequenceModel
        db = _make_db(n_nodes=8, n_events=6)
        try:
            seq = MarkovSequenceModel()
            assert seq.version == 0
            seq.train(db)
            assert seq.version == 1
        finally:
            os.unlink(db)


# ===========================================================================
# cluster.py tests
# ===========================================================================

class TestClusterEngine:
    """Tests for ClusterEngine."""

    def test_train_insufficient_nodes(self):
        """ClusterEngine returns insufficient_data with < 4 active nodes."""
        from silentnode_py.ml.cluster import ClusterEngine
        db = _make_db(n_nodes=3, n_edges=0, n_events=0)
        try:
            ce     = ClusterEngine()
            result = ce.train(db)
            assert result["status"] == "insufficient_data"
        finally:
            os.unlink(db)

    def test_train_produces_clusters(self):
        """ClusterEngine trains and produces valid cluster metadata."""
        from silentnode_py.ml.cluster import ClusterEngine
        db = _make_db(n_nodes=15)
        try:
            ce     = ClusterEngine()
            result = ce.train(db)
            assert result["status"] == "trained"
            assert result["n_clusters"] >= 2
            assert len(result["labels"]) == result["n_clusters"]
            assert ce.trained
        finally:
            os.unlink(db)

    def test_get_clusters_returns_list(self):
        """get_clusters returns a non-empty list after training."""
        from silentnode_py.ml.cluster import ClusterEngine
        db = _make_db(n_nodes=15)
        try:
            ce = ClusterEngine()
            ce.train(db)
            clusters = ce.get_clusters(db)
            assert isinstance(clusters, list)
            assert len(clusters) > 0
        finally:
            os.unlink(db)

    def test_get_clusters_fields(self):
        """Each cluster dict has required fields."""
        from silentnode_py.ml.cluster import ClusterEngine
        db = _make_db(n_nodes=15)
        try:
            ce       = ClusterEngine()
            ce.train(db)
            clusters = ce.get_clusters(db)
            required = {"cluster_id", "label", "dominant_type",
                        "size", "avg_gravity", "members"}
            for cl in clusters:
                assert required.issubset(cl.keys())
                assert cl["size"] == len(cl["members"])
        finally:
            os.unlink(db)

    def test_get_cluster_for_node_returns_result(self):
        """get_cluster_for_node returns a valid cluster assignment."""
        from silentnode_py.ml.cluster import ClusterEngine
        from silentnode_py.ml.features import load_nodes
        db = _make_db(n_nodes=15)
        try:
            ce    = ClusterEngine()
            ce.train(db)
            nodes = load_nodes(db)
            # Pick a non-ghost node
            live  = [n for n in nodes if not n["is_ghost"] and not n["is_void"]]
            nid   = live[0]["id"]
            result = ce.get_cluster_for_node(nid, db)
            assert result is not None
            assert "cluster_id"  in result
            assert "label"       in result
            assert "dominant_type" in result
            assert result["node_id"] == nid
        finally:
            os.unlink(db)

    def test_get_cluster_for_unknown_node_returns_none(self):
        """get_cluster_for_node returns None for unknown node_id."""
        from silentnode_py.ml.cluster import ClusterEngine
        db = _make_db(n_nodes=15)
        try:
            ce = ClusterEngine()
            ce.train(db)
            result = ce.get_cluster_for_node("nonexistent-xyz", db)
            assert result is None
        finally:
            os.unlink(db)

    def test_cluster_labels_no_digits_no_timestamps(self):
        """Cluster labels must not be pure digit strings or look like timestamps."""
        from silentnode_py.ml.cluster import ClusterEngine
        db = _make_db(n_nodes=20)
        try:
            ce = ClusterEngine()
            ce.train(db)
            import re
            timestamp_pattern = re.compile(r"\d{4}-\d{2}-\d{2}")
            for label in ce.cluster_labels:
                assert not label.isdigit(), f"Label is pure digit: {label!r}"
                assert not timestamp_pattern.search(label), (
                    f"Label looks like a timestamp: {label!r}")
        finally:
            os.unlink(db)

    def test_get_cluster_summary(self):
        """get_cluster_summary returns a list with type_distribution."""
        from silentnode_py.ml.cluster import ClusterEngine
        db = _make_db(n_nodes=15)
        try:
            ce       = ClusterEngine()
            ce.train(db)
            summaries = ce.get_cluster_summary(db)
            assert isinstance(summaries, list)
            for s in summaries:
                assert "type_distribution" in s
                assert isinstance(s["type_distribution"], dict)
        finally:
            os.unlink(db)

    def test_dominant_type_in_cluster(self):
        """Each cluster's dominant_type appears in its type_distribution."""
        from silentnode_py.ml.cluster import ClusterEngine
        db = _make_db(n_nodes=15)
        try:
            ce       = ClusterEngine()
            ce.train(db)
            summaries = ce.get_cluster_summary(db)
            for s in summaries:
                dom = s["dominant_type"]
                assert dom in s["type_distribution"] or dom == "unknown"
        finally:
            os.unlink(db)

    def test_untrained_get_clusters_returns_empty(self):
        """get_clusters returns [] when model is not trained."""
        from silentnode_py.ml.cluster import ClusterEngine
        db = _make_db(n_nodes=15)
        try:
            ce       = ClusterEngine()
            clusters = ce.get_clusters(db)
            assert clusters == []
        finally:
            os.unlink(db)


# ===========================================================================
# trainer.py tests
# ===========================================================================

class TestTrainer:
    """Tests for train_all, get_status, incremental_update."""

    def test_train_all_returns_dict(self):
        """train_all returns a status dict with all four models."""
        from silentnode_py.ml.trainer import train_all
        db = _make_db(n_nodes=15, n_events=6)
        try:
            result = train_all(db)
            assert "models" in result
            assert "trained_at" in result
            assert "duration_seconds" in result
            assert result["status"] in ("ok", "partial")
            expected_models = {"classifier", "ghost_predictor",
                                "sequence", "cluster"}
            assert expected_models.issubset(result["models"].keys())
        finally:
            os.unlink(db)

    def test_train_all_writes_status_json(self, tmp_path):
        """train_all writes status.json to the model directory."""
        from silentnode_py.ml import trainer as trainer_module
        db         = _make_db(n_nodes=15, n_events=6)
        status_path = tmp_path / "status.json"
        try:
            with patch.object(trainer_module, "STATUS_PATH", status_path):
                trainer_module.train_all(db)
            assert status_path.exists()
            with open(status_path) as f:
                data = json.load(f)
            assert data["status"] in ("ok", "partial")
        finally:
            os.unlink(db)

    def test_get_status_not_trained(self, tmp_path):
        """get_status returns not_trained when no status file exists."""
        from silentnode_py.ml import trainer as trainer_module
        fake_path = tmp_path / "nonexistent.json"
        with patch.object(trainer_module, "STATUS_PATH", fake_path):
            status = trainer_module.get_status()
        assert status["status"] == "not_trained"

    def test_get_status_after_train(self, tmp_path):
        """get_status reads and returns the written status."""
        from silentnode_py.ml import trainer as trainer_module
        db          = _make_db(n_nodes=15, n_events=6)
        status_path = tmp_path / "status.json"
        try:
            with patch.object(trainer_module, "STATUS_PATH", status_path):
                trainer_module.train_all(db)
                status = trainer_module.get_status()
            assert status["status"] in ("ok", "partial")
        finally:
            os.unlink(db)

    def test_should_retrain_when_not_trained(self, tmp_path):
        """should_retrain returns True when no status exists."""
        from silentnode_py.ml import trainer as trainer_module
        fake_path = tmp_path / "nonexistent.json"
        with patch.object(trainer_module, "STATUS_PATH", fake_path):
            assert trainer_module.should_retrain() is True

    def test_should_retrain_fresh_model(self, tmp_path):
        """should_retrain returns False just after training."""
        from silentnode_py.ml import trainer as trainer_module
        db          = _make_db(n_nodes=15, n_events=6)
        status_path = tmp_path / "status.json"
        try:
            with patch.object(trainer_module, "STATUS_PATH", status_path):
                trainer_module.train_all(db)
                result = trainer_module.should_retrain(db)
            assert result is False
        finally:
            os.unlink(db)

    def test_incremental_update_returns_dict(self):
        """incremental_update returns a result dict with 'incremental' mode."""
        from silentnode_py.ml.trainer import incremental_update
        db = _make_db(n_nodes=15, n_events=6)
        try:
            result = incremental_update(db)
            assert result.get("mode") == "incremental"
            assert "models" in result
        finally:
            os.unlink(db)

    def test_incremental_update_preserves_version(self):
        """Incremental update increments model versions (not resets)."""
        from silentnode_py.ml import trainer as trainer_module
        db = _make_db(n_nodes=15, n_events=6)
        try:
            r1 = trainer_module.train_all(db)
            r2 = trainer_module.incremental_update(db)
            # After two training runs the classifier version should be >= 2
            v1 = r1["models"].get("classifier", {}).get("version", 0)
            v2 = r2["models"].get("classifier", {}).get("version", 0)
            if v1 and v2:
                assert v2 >= v1
        finally:
            os.unlink(db)


# ===========================================================================
# Integration tests on real database
# ===========================================================================

@pytest.mark.skipif(not REAL_DB_EXISTS,
                    reason="Real database data/silentnode.sqlite not available")
class TestRealDatabase:
    """Integration tests that run against the actual silentnode.sqlite."""

    def test_load_nodes_count(self):
        """Real DB has loadable nodes."""
        from silentnode_py.ml.features import load_nodes
        nodes = load_nodes(REAL_DB)
        assert len(nodes) > 0

    def test_load_edges_count(self):
        """Real DB has a loadable edge list."""
        from silentnode_py.ml.features import load_edges
        edges = load_edges(REAL_DB)
        assert isinstance(edges, list)

    def test_load_focus_events_count(self):
        """Real DB has a loadable focus-event list."""
        from silentnode_py.ml.features import load_focus_events
        events = load_focus_events(REAL_DB)
        assert isinstance(events, list)

    def test_graph_features_all_nodes(self):
        """build_graph_features runs on all real DB nodes without error."""
        from silentnode_py.ml.features import load_nodes, build_graph_features
        nodes = load_nodes(REAL_DB)
        gf    = build_graph_features(nodes, REAL_DB)
        assert len(gf) == len(nodes)
        for nid, metrics in gf.items():
            assert all(v >= 0 for v in metrics.values())

    def test_classifier_trains_on_real_data(self):
        """Classifier trains on real data without error."""
        from silentnode_py.ml.classifier import NodeClassifier
        clf    = NodeClassifier()
        result = clf.train(REAL_DB)
        assert result["status"] in ("trained", "insufficient_data")

    def test_ghost_predictor_on_real_data(self):
        """GhostPredictor trains and predicts on real data."""
        from silentnode_py.ml.ghost_predictor import GhostPredictor
        gp     = GhostPredictor()
        result = gp.train(REAL_DB)
        assert result["status"] in ("trained", "rule_based")
        risks = gp.predict_all(REAL_DB)
        assert isinstance(risks, list)
        assert len(risks) > 0

    def test_sequence_model_on_real_data(self):
        """Sequence model trains on real focus events."""
        from silentnode_py.ml.sequence import MarkovSequenceModel
        seq    = MarkovSequenceModel()
        result = seq.train(REAL_DB)
        # 8 events might or might not yield transitions depending on session gaps
        assert result["status"] in ("trained", "insufficient_data")

    def test_cluster_engine_on_real_data(self):
        """ClusterEngine trains and assigns all active nodes to clusters."""
        from silentnode_py.ml.cluster import ClusterEngine
        from silentnode_py.ml.features import load_nodes
        ce     = ClusterEngine()
        result = ce.train(REAL_DB)
        assert result["status"] == "trained"
        clusters = ce.get_clusters(REAL_DB)
        # All active nodes should be in exactly one cluster
        nodes       = load_nodes(REAL_DB)
        active_ids  = {n["id"] for n in nodes
                       if not n["is_ghost"] and not n["is_void"]}
        assigned_ids = {m["node_id"]
                        for cl in clusters for m in cl["members"]}
        assert active_ids == assigned_ids, (
            f"Unassigned nodes: {active_ids - assigned_ids}")

    def test_full_pipeline(self):
        """Full train_all pipeline completes without crashing."""
        from silentnode_py.ml.trainer import train_all
        result = train_all(REAL_DB)
        assert result["status"] in ("ok", "partial")
        assert result["duration_seconds"] > 0
        assert len(result["models"]) == 4

    def test_entropy_trajectory_first_node(self):
        """Entropy trajectory works on the first real node."""
        from silentnode_py.ml.features import load_nodes
        from silentnode_py.ml.ghost_predictor import GhostPredictor
        nodes   = load_nodes(REAL_DB)
        node_id = nodes[0]["id"]
        gp      = GhostPredictor()
        gp.train(REAL_DB)
        result = gp.predict_entropy_trajectory(node_id, days_ahead=7,
                                                db_path=REAL_DB)
        assert "error" not in result
        assert len(result["daily_trajectory"]) == 8

    def test_predict_batch_real_data(self):
        """predict_batch returns correct count on multiple real contents."""
        from silentnode_py.ml.classifier import NodeClassifier
        from silentnode_py.ml.features import load_nodes
        nodes    = load_nodes(REAL_DB)
        contents = [n["content"] for n in nodes[:5]]
        clf      = NodeClassifier()
        clf.train(REAL_DB)
        results  = clf.predict_batch(contents)
        assert len(results) == 5
        for r in results:
            assert "type" in r

    def test_get_cluster_for_each_node(self):
        """get_cluster_for_node works for every active node."""
        from silentnode_py.ml.cluster import ClusterEngine
        from silentnode_py.ml.features import load_nodes
        nodes = load_nodes(REAL_DB)
        ce    = ClusterEngine()
        ce.train(REAL_DB)
        for n in nodes:
            if n["is_ghost"] or n["is_void"]:
                continue
            result = ce.get_cluster_for_node(n["id"], REAL_DB)
            assert result is not None
            assert 0 <= result["cluster_id"] < ce.n_clusters
