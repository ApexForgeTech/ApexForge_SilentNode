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

from .features import (
    load_nodes,
    load_focus_events,
    load_edges,
    node_focus_stats,
    build_text_tokens,
    DB_PATH,
)

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
        self.node_order: List[str] = []
        self.focus_stats: Dict[str, Dict] = {}
        self.graph_links: Dict[str, Dict[str, float]] = {}
        self.trained = False
        self.n_transitions = 0
        self.n_transitions2 = 0
        self.n_unique_nodes = 0
        self.version: int = 0

    def _reset_learned_state(self):
        """Clear train-derived state while preserving the model version."""
        self.transitions = {}
        self.transitions2 = {}
        self.hour_transitions = {b: {} for b in range(4)}
        self.node_access_count = {}
        self.node_info = {}
        self.node_order = []
        self.focus_stats = {}
        self.graph_links = {}
        self.trained = False
        self.n_transitions = 0
        self.n_transitions2 = 0
        self.n_unique_nodes = 0

    # ------------------------------------------------------------------
    # Training
    # ------------------------------------------------------------------

    def train(self, db_path: str = DB_PATH) -> Dict:
        """Fit the Markov chain from focus events in the database.

        Sessions are segmented by SESSION_GAP.  Within each session,
        consecutive distinct node visits build first- and second-order
        transition tables.
        """
        self._reset_learned_state()
        nodes  = load_nodes(db_path)
        events = load_focus_events(db_path)
        edges  = load_edges(db_path)
        self.focus_stats = node_focus_stats(nodes, events)
        self.node_order = [n["id"] for n in nodes]

        # Store node metadata
        for n in nodes:
            self.node_info[n["id"]] = {
                "content":   n["content"][:60],
                "node_type": n["node_type"],
                "nickname":  n.get("nickname", ""),
                "metadata":  n.get("metadata", {}) or {},
                "entropy":   float(n.get("entropy") or 0.0),
                "gravity":   float(n.get("gravity") or 1.0),
                "velocity":  float(n.get("velocity") or 0.0),
                "is_ghost":  bool(n.get("is_ghost")),
                "is_fossil": bool(n.get("is_fossil")),
                "is_void":   bool(n.get("is_void")),
            }
        self.graph_links = self._build_graph_links(edges)

        events_sorted = sorted(events, key=lambda e: e["ts_float"])
        for ev in events_sorted:
            nid = ev["node_id"]
            self.node_access_count[nid] = self.node_access_count.get(nid, 0) + 1

        if len(events_sorted) < 2:
            self.n_unique_nodes = len(self.node_access_count)
            self.trained = True
            self.version += 1
            self.save()
            return {
                "status":   "insufficient_data",
                "version":  self.version,
                "n_events": len(events_sorted),
                "message":  "Need at least 2 focus events to learn sequences",
            }

        # Walk through events, segment into sessions, build transitions
        prev_event  = None
        prev2_event = None   # two steps back (for second-order chain)

        for ev in events_sorted:
            nid = ev["node_id"]

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

    def _build_graph_links(self, edges: List[Dict]) -> Dict[str, Dict[str, float]]:
        """Directed adjacency plus sibling hints used for contextual fallback."""
        links: Dict[str, Dict[str, float]] = defaultdict(dict)
        children_by_parent: Dict[str, List[tuple]] = defaultdict(list)
        for edge in edges:
            src = edge.get("source_id")
            dst = edge.get("target_id")
            if not src or not dst:
                continue
            try:
                weight = float(edge.get("weight") or 1.0)
            except Exception:
                weight = 1.0
            weight = max(0.05, min(weight, 5.0))
            links[src][dst] = max(links[src].get(dst, 0.0), weight)
            links[dst][src] = max(links[dst].get(src, 0.0), weight * 0.25)
            children_by_parent[src].append((dst, weight))

        # Parent/root nodes often connect a routine group. From a child node,
        # siblings are usually a better "next focus" than jumping back to the
        # broad parent hub, so add a moderate sibling signal.
        for children in children_by_parent.values():
            if len(children) < 2:
                continue
            for src, src_weight in children:
                for dst, dst_weight in children:
                    if src == dst:
                        continue
                    sibling_weight = min(src_weight, dst_weight) * 0.55
                    links[src][dst] = max(links[src].get(dst, 0.0), sibling_weight)
        return dict(links)

    def _node_tokens(self, node_id: str) -> set:
        info = self.node_info.get(node_id, {})
        text = f"{info.get('nickname', '')} {info.get('content', '')}"
        return set(build_text_tokens(text))

    def _text_similarity(self, current_id: Optional[str], candidate_id: str) -> float:
        if not current_id or current_id not in self.node_info:
            return 0.0
        a = self._node_tokens(current_id)
        b = self._node_tokens(candidate_id)
        if not a or not b:
            return 0.0
        return len(a & b) / max(len(a | b), 1)

    def _schedule_boost(self, node_id: str) -> float:
        sched = self.node_info.get(node_id, {}).get("metadata", {}).get("schedule") or {}
        if not isinstance(sched, dict) or sched.get("status", "active") != "active":
            return 0.0
        mode = str(sched.get("mode") or "").lower()
        if mode in {"daily", "custom_days", "weekly"}:
            return 0.35
        if mode == "interval":
            return 0.25
        if mode == "once":
            return 0.18
        return 0.0

    def _candidate_reason(self, current_id: Optional[str], node_id: str, parts: Dict[str, float]) -> str:
        labels = []
        if parts.get("transition", 0.0) > 0:
            labels.append("learned sequence")
        if parts.get("graph", 0.0) > 0.05:
            labels.append("linked in graph")
        if parts.get("text", 0.0) > 0.08:
            labels.append("similar content")
        if parts.get("schedule", 0.0) > 0:
            labels.append("scheduled routine")
        if parts.get("neglect", 0.0) > 0.25:
            labels.append("needs attention")
        if parts.get("focus_gap", 0.0) > 0.2:
            labels.append("little focus logged")
        if current_id and current_id not in self.node_info:
            labels.append("popular fallback")
        return ", ".join(labels[:3]) or "contextual next focus"

    def _contextual_scores(
        self,
        current_node_id: Optional[str],
        transition_scores: Optional[Dict[str, float]] = None,
        exclude: Optional[set] = None,
    ) -> List[tuple]:
        """Rank candidates when direct sequence data is sparse.

        This deliberately blends several weak signals instead of letting the
        global most-focused node dominate every private vault.
        """
        transition_scores = transition_scores or {}
        exclude = exclude or set()
        max_access = max(self.node_access_count.values(), default=1)
        current_links = self.graph_links.get(current_node_id or "", {})
        max_link = max(current_links.values(), default=1.0)

        rows = []
        for node_id in self.node_order:
            if node_id in exclude:
                continue
            info = self.node_info.get(node_id, {})
            if info.get("is_ghost") or info.get("is_fossil") or info.get("is_void"):
                continue

            focus = self.focus_stats.get(node_id, {})
            total_seconds = float(focus.get("total_seconds", 0.0))
            last_days = float(focus.get("last_focus_days", 30.0))
            access_count = float(self.node_access_count.get(node_id, 0))

            graph = current_links.get(node_id, 0.0) / max_link if current_links else 0.0
            text = self._text_similarity(current_node_id, node_id)
            schedule = self._schedule_boost(node_id)
            neglect = min(last_days / 14.0, 1.0)
            focus_gap = 1.0 if total_seconds <= 0 else max(0.0, 1.0 - total_seconds / 7200.0)
            access = access_count / max_access if max_access else 0.0
            gravity = min(float(info.get("gravity") or 1.0) / 5.0, 1.0)
            entropy = min(float(info.get("entropy") or 0.0), 1.0)
            transition = transition_scores.get(node_id, 0.0)

            parts = {
                "transition": transition,
                "graph": graph,
                "text": text,
                "schedule": schedule,
                "neglect": neglect,
                "focus_gap": focus_gap,
                "access": access,
                "gravity": gravity,
                "entropy": entropy,
            }
            score = (
                transition * 0.42
                + graph * 0.18
                + text * 0.16
                + schedule * 0.12
                + neglect * 0.07
                + focus_gap * 0.06
                + access * 0.04
                + gravity * 0.03
                + entropy * 0.02
            )
            if score <= 0:
                continue
            rows.append((node_id, score, parts))

        rows.sort(key=lambda item: item[1], reverse=True)
        return rows

    def _ranked_results(
        self,
        rows: List[tuple],
        top_k: int,
        source: str,
        current_node_id: Optional[str] = None,
    ) -> List[Dict]:
        rows = rows[:max(0, top_k)]
        total = sum(score for _, score, _ in rows) or 1.0
        results = []
        for node_id, score, parts in rows:
            info = self.node_info.get(node_id, {"content": node_id[:16], "node_type": "idea"})
            results.append({
                "node_id":     node_id,
                "content":     info["content"],
                "node_type":   info["node_type"],
                "probability": round(max(0.0, min(score / total, 1.0)), 4),
                "score":       round(score, 4),
                "source":      source,
                "reason":      self._candidate_reason(current_node_id, node_id, parts),
            })
        return results

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
            return self._contextual_fallback(top_k, current_node_id, exclude={current_node_id})

        counts = self.transitions.get(current_node_id, {})
        if not counts:
            return self._contextual_fallback(top_k, current_node_id, exclude={current_node_id})

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
        rows = self._contextual_scores(
            current_node_id,
            transition_scores=scores,
            exclude={current_node_id},
        )
        return self._ranked_results(rows, top_k, "sequence_context", current_node_id)

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
            return self._contextual_fallback(top_k)

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
        rows = self._contextual_scores(
            recent_node_ids[-1] if recent_node_ids else None,
            transition_scores=dict(scores),
            exclude=session_set,
        )
        current_id = recent_node_ids[-1] if recent_node_ids else None
        return (
            self._ranked_results(rows, top_k, "session_context", current_id)
            if rows
            else self._contextual_fallback(top_k, current_id, exclude=session_set)
        )

    def _contextual_fallback(
        self,
        top_k: int,
        current_node_id: Optional[str] = None,
        exclude: Optional[set] = None,
    ) -> List[Dict]:
        exclude = exclude or set()
        rows = self._contextual_scores(current_node_id, exclude=exclude)
        if rows:
            return self._ranked_results(rows, top_k, "contextual_fallback", current_node_id)
        return self._fallback_popular(top_k, exclude=exclude)

    def _fallback_popular(self, top_k: int, exclude: Optional[set] = None) -> List[Dict]:
        """Return the globally most-accessed nodes as a fallback."""
        exclude = exclude or set()
        popular = [
            item for item in sorted(self.node_access_count.items(),
                                    key=lambda x: x[1], reverse=True)
            if item[0] not in exclude
        ][:top_k]
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
                "source":      "popular_fallback",
                "reason":      "popular fallback",
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
