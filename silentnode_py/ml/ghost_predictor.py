"""
Ghost Predictor for SilentNode.

Estimates the 'ghost risk' for each live node — i.e., how likely a node
is to transition to ghost state due to inactivity and rising entropy.

Models:
  1. GradientBoostingRegressor  — predicts days_to_ghost from features
  2. Rule-based fallback        — used when < 5 training samples
  3. predict_entropy_trajectory — projects entropy N days into the future
     given current entropy and days_since_access

Features used:
  entropy, entropy^2, days_since_access, normalized_days, inverse_connection,
  inverse_gravity, inverse_focus, inverse_seconds, velocity, access_count,
  entropy*days (interaction), focus_recency_decay, pagerank (if available)
"""

import pickle
import numpy as np
from pathlib import Path
from typing import List, Dict

from sklearn.ensemble import GradientBoostingRegressor
from sklearn.preprocessing import StandardScaler

from .features import (
    load_nodes, load_focus_events, node_focus_stats,
    build_temporal_features, DB_PATH,
)

MODEL_DIR = Path("data/ml_models")
GHOST_MODEL_PATH = MODEL_DIR / "ghost_predictor.pkl"

# Feature names matching the _features() output order
_FEATURE_NAMES = [
    "entropy",
    "entropy_sq",
    "days_since_access",
    "days_access_norm",
    "inv_connection",
    "inv_gravity",
    "inv_focus_count",
    "inv_focus_seconds",
    "velocity",
    "access_count",
    "entropy_x_days",      # interaction term
    "focus_recency_decay", # from temporal features
    "connection_count",
]


class GhostPredictor:
    """Predict ghost risk and days-to-ghost for live nodes.

    Attributes:
        trained                — whether the ML model has been fitted
        n_samples              — total nodes seen during training
        feature_importance     — dict mapping feature name -> importance score
        ghost_entropy_threshold — 85th-percentile entropy of live nodes
        avg_days_to_ghost       — mean days_since_access of known ghost nodes
        version                 — integer version counter
    """

    def __init__(self):
        self.scaler = StandardScaler()
        self.model = GradientBoostingRegressor(
            n_estimators=100,
            max_depth=4,
            learning_rate=0.05,
            subsample=0.8,
            min_samples_leaf=2,
            random_state=42,
        )
        self.trained = False
        self.n_samples = 0
        self.feature_importance: Dict[str, float] = {}
        self.ghost_entropy_threshold = 0.75
        self.avg_days_to_ghost = 14.0
        self.version: int = 0

    # ------------------------------------------------------------------
    # Feature construction
    # ------------------------------------------------------------------

    def _features(self, nodes: List[Dict],
                  focus_stats: Dict[str, Dict]) -> np.ndarray:
        """Build the ghost-risk feature matrix for a list of nodes.

        Shape: (N, len(_FEATURE_NAMES))
        """
        temp_feats = build_temporal_features(nodes, focus_stats)
        rows = []
        for i, n in enumerate(nodes):
            fs = focus_stats.get(n["id"], {})
            entropy   = float(n["entropy"])
            days_acc  = float(n["days_since_access"])
            conn      = float(n["connection_count"])
            gravity   = float(n["gravity"])
            focus_cnt = float(fs.get("focus_count", 0))
            total_sec = float(fs.get("total_seconds", 0))
            velocity  = float(n["velocity"])
            acc_count = float(n["access_count"])

            # Recency decay from temporal features column 1
            recency_decay = float(temp_feats[i, 1]) if i < len(temp_feats) else 0.0

            rows.append([
                entropy,
                entropy ** 2,
                days_acc,
                min(days_acc / 30.0, 1.0),
                1.0 / (conn + 1.0),          # isolated nodes are riskier
                1.0 / (gravity + 0.1),
                1.0 / (focus_cnt + 1.0),
                1.0 / (total_sec + 1.0),
                velocity,
                acc_count,
                entropy * days_acc,           # joint risk signal
                recency_decay,
                conn,
            ])
        return np.array(rows, dtype=np.float32)

    # ------------------------------------------------------------------
    # Training
    # ------------------------------------------------------------------

    def train(self, db_path: str = DB_PATH) -> Dict:
        """Fit the ghost predictor from the database.

        The training target is an estimated 'days_to_ghost' derived from
        each node's entropy and days_since_access.  Nodes with actual
        is_ghost=1 inform the rule-based thresholds.
        """
        nodes  = load_nodes(db_path)
        events = load_focus_events(db_path)
        focus_stats = node_focus_stats(nodes, events)

        ghost_nodes = [n for n in nodes if n["is_ghost"]]
        live_nodes  = [n for n in nodes if not n["is_ghost"]]
        self.n_samples = len(nodes)

        # Update rule-based thresholds from observed data
        if live_nodes:
            entropies = [float(n["entropy"]) for n in live_nodes]
            self.ghost_entropy_threshold = float(np.percentile(entropies, 85))

        if ghost_nodes:
            self.avg_days_to_ghost = float(
                np.mean([float(n["days_since_access"]) for n in ghost_nodes])
            )

        # Build training set — prefer live nodes, fall back to all nodes
        train_nodes = live_nodes if len(live_nodes) >= 5 else nodes
        if not train_nodes:
            return {"status": "no_data", "n_samples": 0}

        X = self._features(train_nodes, focus_stats)

        # Construct target: estimated days remaining before ghost transition
        # based on entropy buckets calibrated from the observed threshold
        y = []
        for n in train_nodes:
            e = float(n["entropy"])
            d = float(n["days_since_access"])
            conn = float(n["connection_count"])
            # Well-connected nodes get a bonus
            conn_bonus = min(conn * 0.5, 5.0)
            if e > 0.85:
                base_days = 3.0
            elif e > 0.70:
                base_days = 7.0
            elif e > 0.50:
                base_days = 14.0
            elif e > 0.30:
                base_days = 21.0
            else:
                base_days = 30.0
            y.append(max(0.0, base_days + conn_bonus - d))

        if len(train_nodes) >= 5:
            X_scaled = self.scaler.fit_transform(X)
            self.model.fit(X_scaled, np.array(y, dtype=np.float32))
            self.trained = True
            self.version += 1

            importances = self.model.feature_importances_
            self.feature_importance = {
                _FEATURE_NAMES[i]: round(float(importances[i]), 4)
                for i in range(min(len(_FEATURE_NAMES), len(importances)))
            }

        self.save()
        return {
            "status":             "trained" if self.trained else "rule_based",
            "version":            self.version,
            "n_samples":          self.n_samples,
            "ghost_count":        len(ghost_nodes),
            "entropy_threshold":  round(self.ghost_entropy_threshold, 3),
            "avg_days_to_ghost":  round(self.avg_days_to_ghost, 1),
            "feature_importance": self.feature_importance,
        }

    # ------------------------------------------------------------------
    # Prediction
    # ------------------------------------------------------------------

    def predict_all(self, db_path: str = DB_PATH) -> List[Dict]:
        """Compute ghost risk scores for all live, non-void nodes.

        Returns a list sorted by risk_score descending.  Each entry has:
            node_id, content, node_type, entropy, days_since_access,
            connection_count, days_to_ghost, risk_score, risk_level
        """
        nodes  = load_nodes(db_path)
        events = load_focus_events(db_path)
        focus_stats = node_focus_stats(nodes, events)

        live_nodes = [n for n in nodes
                      if not n["is_ghost"] and not n["is_void"]]
        if not live_nodes:
            return []

        X = self._features(live_nodes, focus_stats)

        results = []
        for i, n in enumerate(live_nodes):
            entropy = float(n["entropy"])
            days    = float(n["days_since_access"])
            conn    = float(n["connection_count"])
            fs      = focus_stats.get(n["id"], {})
            last_focus = float(fs.get("last_focus_days", 30.0))

            # ML or rule-based days_to_ghost
            if self.trained:
                X_scaled   = self.scaler.transform(X[i:i + 1])
                days_left  = float(max(0.0, self.model.predict(X_scaled)[0]))
            else:
                conn_bonus = min(conn * 0.5, 5.0)
                if entropy > 0.85:
                    days_left = max(0.0, 3.0  + conn_bonus - days)
                elif entropy > 0.70:
                    days_left = max(0.0, 7.0  + conn_bonus - days)
                elif entropy > 0.50:
                    days_left = max(0.0, 14.0 + conn_bonus - days)
                else:
                    days_left = max(0.0, 30.0 + conn_bonus - days)

            # Risk score: combines entropy, inactivity, low connectivity,
            # and focus recency — all capped at 1.0
            focus_penalty = min(last_focus / 30.0, 1.0)
            conn_factor   = max(0.1, 1.0 - min(conn / 10.0, 0.9))
            risk = min(1.0,
                       entropy * 0.4
                       + min(days / 30.0, 1.0) * 0.3
                       + focus_penalty * 0.2
                       + conn_factor * 0.1)

            results.append({
                "node_id":           n["id"],
                "content":           n["content"][:60],
                "node_type":         n["node_type"],
                "entropy":           round(entropy, 3),
                "days_since_access": round(days, 1),
                "connection_count":  int(conn),
                "days_to_ghost":     round(days_left, 1),
                "risk_score":        round(risk, 3),
                "risk_level":        ("critical" if risk > 0.75 else
                                      "high"     if risk > 0.50 else
                                      "medium"   if risk > 0.25 else "low"),
            })

        results.sort(key=lambda x: x["risk_score"], reverse=True)
        return results

    def predict_entropy_trajectory(self, node_id: str,
                                   days_ahead: int = 7,
                                   db_path: str = DB_PATH) -> Dict:
        """Project a node's entropy N days into the future.

        Uses a logistic growth model bounded at [0, 1]:
            entropy(t) = 1 / (1 + exp(-k * (entropy_0 + rate * t - 0.5)))

        Where 'rate' is derived from the node's current inactivity and
        connectivity: isolated, rarely-accessed nodes decay faster.

        Returns a dict with:
            node_id, current_entropy, projected_entropy, days_ahead,
            daily_trajectory (list of {day, entropy}), risk_at_projection
        """
        nodes = load_nodes(db_path)
        node = next((n for n in nodes if n["id"] == node_id), None)
        if node is None:
            return {"error": f"Node '{node_id}' not found"}

        entropy_0 = float(node["entropy"])
        days_acc  = float(node["days_since_access"])
        conn      = float(node["connection_count"])
        gravity   = float(node["gravity"])

        # Decay rate per day: higher for isolated/inactive nodes
        # Gravity slows entropy growth; connectivity provides anchor
        k_decay = 0.02 + 0.03 * max(0.0, 1.0 - conn / 10.0)
        if gravity > 5.0:
            k_decay *= 0.7    # high-gravity nodes decay more slowly
        if days_acc > 14.0:
            k_decay *= 1.5    # already inactive nodes accelerate

        trajectory = []
        for day in range(days_ahead + 1):
            projected = min(1.0, entropy_0 + k_decay * day)
            trajectory.append({
                "day":     day,
                "entropy": round(projected, 4),
            })

        final_entropy = trajectory[-1]["entropy"]
        # Risk at projection
        risk = min(1.0,
                   final_entropy * 0.5
                   + min((days_acc + days_ahead) / 30.0, 1.0) * 0.5)

        return {
            "node_id":           node_id,
            "content":           node["content"][:60],
            "current_entropy":   round(entropy_0, 4),
            "projected_entropy": round(final_entropy, 4),
            "days_ahead":        days_ahead,
            "decay_rate_per_day": round(k_decay, 5),
            "daily_trajectory":  trajectory,
            "risk_at_projection": round(risk, 3),
            "risk_level":        ("critical" if risk > 0.75 else
                                  "high"     if risk > 0.50 else
                                  "medium"   if risk > 0.25 else "low"),
        }

    # ------------------------------------------------------------------
    # Persistence
    # ------------------------------------------------------------------

    def save(self):
        """Serialize this instance to disk."""
        MODEL_DIR.mkdir(parents=True, exist_ok=True)
        with open(GHOST_MODEL_PATH, "wb") as f:
            pickle.dump(self, f)

    @classmethod
    def load(cls) -> "GhostPredictor":
        """Load a persisted predictor, or return a fresh instance."""
        if GHOST_MODEL_PATH.exists():
            with open(GHOST_MODEL_PATH, "rb") as f:
                return pickle.load(f)
        return cls()
