"""
Focus Sequence Model for SilentNode.

Learns which nodes a user tends to visit in sequence during focus sessions.
Implements both a first-order and second-order Markov chain.

Key concepts:
  - A 'session' is a sequence of focus events separated by < SESSION_GAP seconds.
  - First-order transitions: P(next | current)
  - Second-order transitions: P(next | prev, current)
  - Session recommendations: weighted prediction from recent node history
  - Time-of-day weighting: if timestamps are available, recent-hour transitions
    receive a slight boost when predicting within the same hour bucket.
"""

import pickle
import numpy as np
from pathlib import Path
from typing import List, Dict, Optional
from collections import defaultdict
from datetime import datetime, timezone

from .features import load_nodes, load_focus_events, DB_PATH

MODEL_DIR = Path("data/ml_models")
SEQUENCE_PATH = MODEL_DIR / "sequence_model.pkl"

# Events within this gap (seconds) belong to the same session
SESSION_GAP = 30 * 60  # 30 minutes

# Hour buckets: 0–5 night, 6–11 morning, 12–17 afternoon, 18–23 evening
_HOUR_BUCKET = {h: h // 6 for h in range(24)}   # 4 buckets: 0,1,2,3


class MarkovSequenceModel:
    """First + second order Markov chain over focus-event sequences.

    Attributes:
        transitions         — first-order: {from_id: {to_id: count}}
        transitions2        — second-order: {(prev_id, curr_id): {to_id: count}}
        hour_transitions    — {hour_bucket: {from_id: {to_id: count}}}
        node_access_count   — {node_id: total_visits}
        node_info           — {node_id: {content, node_type}}
        trained             — whether train() has been called successfully
        n_transitions       — total first-order transitions observed
        n_transitions2      — total second-order transitions observed
        n_unique_nodes      — number of distinct nodes seen
        version             — integer version counter
    """

    def __init__(self):
        self.transitions: Dict[str, Dict[str, int]] = {}
        self.transitions2: Dict[tuple, Dict[str, int]] = {}
        self.hour_transitions: Dict[int, Dict[str, Dict[str, int]]] = {
            b: {} for b in range(4)
        }
        self.node_access_count: Dict[str, int] = {}
        self.node_info: Dict[str, Dict] = {}
        self.trained = False
        self.n_transitions = 0
        self.n_transitions2 = 0
        self.n_unique_nodes = 0
        self.version: int = 0

    # ------------------------------------------------------------------
    # Training
    # ------------------------------------------------------------------

    def train(self, db_path: str = DB_PATH) -> Dict:
        """Fit the Markov chain from focus events in the database.

        Sessions are segmented by SESSION_GAP.  Within each session,
        consecutive distinct node visits build first- and second-order
        transition tables.
        """
        nodes  = load_nodes(db_path)
        events = load_focus_events(db_path)

        # Store node metadata
        for n in nodes:
            self.node_info[n["id"]] = {
                "content":   n["content"][:60],
                "node_type": n["node_type"],
            }

        events_sorted = sorted(events, key=lambda e: e["ts_float"])
        if len(events_sorted) < 2:
            return {
                "status":   "insufficient_data",
                "n_events": len(events_sorted),
                "message":  "Need at least 2 focus events to learn sequences",
            }

        # Walk through events, segment into sessions, build transitions
        prev_event  = None
        prev2_event = None   # two steps back (for second-order chain)

        for ev in events_sorted:
            nid = ev["node_id"]
            self.node_access_count[nid] = self.node_access_count.get(nid, 0) + 1

            if prev_event is not None:
                time_diff = ev["ts_float"] - prev_event["ts_float"]
                in_session = time_diff < SESSION_GAP

                if in_session and prev_event["node_id"] != nid:
                    from_id = prev_event["node_id"]

                    # --- First-order ---
                    if from_id not in self.transitions:
                        self.transitions[from_id] = {}
                    self.transitions[from_id][nid] = (
                        self.transitions[from_id].get(nid, 0) + 1
                    )
                    self.n_transitions += 1

                    # --- Hour-bucket ---
                    try:
                        dt  = datetime.fromtimestamp(ev["ts_float"],
                                                      tz=timezone.utc)
                        hb  = _HOUR_BUCKET[dt.hour]
                        hbt = self.hour_transitions[hb]
                        if from_id not in hbt:
                            hbt[from_id] = {}
                        hbt[from_id][nid] = hbt[from_id].get(nid, 0) + 1
                    except Exception:
                        pass

                    # --- Second-order ---
                    if (prev2_event is not None
                            and prev2_event["node_id"] != from_id
                            and (ev["ts_float"] - prev2_event["ts_float"]) < SESSION_GAP):
                        key = (prev2_event["node_id"], from_id)
                        if key not in self.transitions2:
                            self.transitions2[key] = {}
                        self.transitions2[key][nid] = (
                            self.transitions2[key].get(nid, 0) + 1
                        )
                        self.n_transitions2 += 1

                else:
                    # Session boundary — reset second-order context
                    prev2_event = None

            prev2_event = prev_event
            prev_event  = ev

        self.n_unique_nodes = len(self.node_access_count)
        self.trained = True
        self.version += 1
        self.save()

        return {
            "status":         "trained",
            "version":        self.version,
            "n_events":       len(events_sorted),
            "n_transitions":  self.n_transitions,
            "n_transitions2": self.n_transitions2,
            "n_unique_nodes": self.n_unique_nodes,
        }

    # ------------------------------------------------------------------
    # Prediction helpers
    # ------------------------------------------------------------------

    def _probs_from_counts(self, counts: Dict[str, int],
                           top_k: int) -> List[Dict]:
        """Convert a {node_id: count} dict into a ranked probability list."""
        total = sum(counts.values()) or 1
        results = []
        for node_id, count in sorted(counts.items(),
                                      key=lambda x: x[1], reverse=True)[:top_k]:
            info = self.node_info.get(node_id,
                                      {"content": node_id[:16], "node_type": "idea"})
            results.append({
                "node_id":     node_id,
                "content":     info["content"],
                "node_type":   info["node_type"],
                "probability": round(count / total, 4),
                "count":       count,
            })
        return results

    def _hour_boost(self, scores: Dict[str, float],
                    hour_bucket: Optional[int]) -> Dict[str, float]:
        """Apply a small multiplicative boost to transitions observed in
        the same hour bucket as the current time."""
        if hour_bucket is None:
            return scores
        hbt = self.hour_transitions.get(hour_bucket, {})
        # Aggregate all to-nodes seen in this hour bucket
        hour_counts: Dict[str, int] = defaultdict(int)
        for to_map in hbt.values():
            for to_id, cnt in to_map.items():
                hour_counts[to_id] += cnt
        if not hour_counts:
            return scores
        max_cnt = max(hour_counts.values()) or 1
        boosted = {}
        for nid, score in scores.items():
            boost = 1.0 + 0.15 * (hour_counts.get(nid, 0) / max_cnt)
            boosted[nid] = score * boost
        return boosted

    # ------------------------------------------------------------------
    # Public prediction API
    # ------------------------------------------------------------------

    def predict_next(self, current_node_id: str,
                     top_k: int = 5,
                     use_time_of_day: bool = True) -> List[Dict]:
        """Predict the most likely next nodes after visiting current_node_id.

        Uses first-order transitions; falls back to popular nodes if none exist.
        Applies optional time-of-day weighting.
        """
        if not self.trained:
            return self._fallback_popular(top_k)

        counts = self.transitions.get(current_node_id, {})
        if not counts:
            return self._fallback_popular(top_k)

        # Convert to float scores for optional boosting
        total = sum(counts.values()) or 1
        scores: Dict[str, float] = {nid: cnt / total
                                     for nid, cnt in counts.items()}

        if use_time_of_day:
            try:
                hb = _HOUR_BUCKET[datetime.now(timezone.utc).hour]
                scores = self._hour_boost(scores, hb)
            except Exception:
                pass

        # Re-rank by boosted scores
        ranked = sorted(scores.items(), key=lambda x: x[1], reverse=True)[:top_k]
        results = []
        for node_id, score in ranked:
            info = self.node_info.get(node_id,
                                       {"content": node_id[:16], "node_type": "idea"})
            results.append({
                "node_id":     node_id,
                "content":     info["content"],
                "node_type":   info["node_type"],
                "probability": round(min(score, 1.0), 4),
            })
        return results

    def session_recommendations(self, recent_node_ids: List[str],
                                 top_k: int = 5,
                                 use_time_of_day: bool = True) -> List[Dict]:
        """Recommend next nodes given a list of recently visited node IDs.

        Strategy:
          1. Try second-order Markov for the last 2 nodes (highest weight).
          2. Blend first-order predictions for each recent node, weighted by
             recency (most-recent node has weight 1.0, older nodes decay by 0.5^i).
          3. Optionally apply time-of-day boost.

        Args:
            recent_node_ids: list of node IDs, most-recent last
            top_k: number of recommendations to return
            use_time_of_day: whether to apply hour-bucket weighting

        Returns:
            List of recommendation dicts sorted by score descending.
        """
        if not self.trained or not recent_node_ids:
            return self._fallback_popular(top_k)

        scores: Dict[str, float] = defaultdict(float)

        # --- Second-order contribution (strong signal) ---
        if len(recent_node_ids) >= 2:
            key = (recent_node_ids[-2], recent_node_ids[-1])
            second_counts = self.transitions2.get(key, {})
            if second_counts:
                s_total = sum(second_counts.values()) or 1
                for nid, cnt in second_counts.items():
                    scores[nid] += 1.5 * cnt / s_total   # weight > first-order

        # --- First-order contributions (recency-weighted) ---
        decay_weights = [0.5 ** i for i in range(len(recent_node_ids))]
        # most-recent node gets weight index 0
        for nid, w in zip(reversed(recent_node_ids), decay_weights):
            trans = self.transitions.get(nid, {})
            total = sum(trans.values()) or 1
            for to_id, cnt in trans.items():
                scores[to_id] += w * cnt / total

        # --- Time-of-day boost ---
        if use_time_of_day:
            try:
                hb = _HOUR_BUCKET[datetime.now(timezone.utc).hour]
                scores = dict(self._hour_boost(dict(scores), hb))
            except Exception:
                pass

        # Exclude nodes already in the session
        session_set = set(recent_node_ids)
        results = []
        for node_id, score in sorted(scores.items(),
                                      key=lambda x: x[1], reverse=True):
            if node_id in session_set:
                continue
            info = self.node_info.get(node_id,
                                       {"content": node_id[:16], "node_type": "idea"})
            results.append({
                "node_id":     node_id,
                "content":     info["content"],
                "node_type":   info["node_type"],
                "probability": round(min(score, 1.0), 4),
            })
            if len(results) >= top_k:
                break

        return results if results else self._fallback_popular(top_k)

    def _fallback_popular(self, top_k: int) -> List[Dict]:
        """Return the globally most-accessed nodes as a fallback."""
        popular = sorted(self.node_access_count.items(),
                         key=lambda x: x[1], reverse=True)[:top_k]
        total = sum(c for _, c in popular) or 1
        results = []
        for nid, cnt in popular:
            info = self.node_info.get(nid,
                                       {"content": nid[:16], "node_type": "idea"})
            results.append({
                "node_id":     nid,
                "content":     info["content"],
                "node_type":   info["node_type"],
                "probability": round(cnt / total, 4),
                "count":       cnt,
            })
        return results

    def get_transition_matrix(self) -> List[Dict]:
        """Return all first-order transitions for debugging/inspection."""
        rows = []
        for from_id, to_map in self.transitions.items():
            total = sum(to_map.values())
            from_info = self.node_info.get(from_id, {"content": from_id[:16]})
            for to_id, count in sorted(to_map.items(),
                                        key=lambda x: x[1], reverse=True):
                to_info = self.node_info.get(to_id, {"content": to_id[:16]})
                rows.append({
                    "from":  from_info["content"],
                    "to":    to_info["content"],
                    "prob":  round(count / total, 4),
                    "count": count,
                })
        return sorted(rows, key=lambda x: x["prob"], reverse=True)

    # ------------------------------------------------------------------
    # Persistence
    # ------------------------------------------------------------------

    def save(self):
        """Serialize this instance to disk."""
        MODEL_DIR.mkdir(parents=True, exist_ok=True)
        with open(SEQUENCE_PATH, "wb") as f:
            pickle.dump(self, f)

    @classmethod
    def load(cls) -> "MarkovSequenceModel":
        """Load a persisted model, or return a fresh instance."""
        if SEQUENCE_PATH.exists():
            with open(SEQUENCE_PATH, "rb") as f:
                return pickle.load(f)
        return cls()
