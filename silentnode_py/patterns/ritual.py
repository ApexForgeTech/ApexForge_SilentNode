"""
Phase 6.1 — Ritual Engine
Detects recurring behavioral sequences (rituals) in focus session data using
DBSCAN clustering on session-sequence feature vectors.
"""

import json
import math
from collections import defaultdict
from datetime import datetime, timezone
from typing import Any

import numpy as np
from sklearn.cluster import DBSCAN
from sklearn.preprocessing import normalize


# ── Session segmentation ──────────────────────────────────────────────────────

SESSION_GAP_SECONDS = 1800  # 30 minutes gap = new session


def _parse_ts(s: str) -> float:
    """ISO timestamp → unix epoch float."""
    try:
        dt = datetime.fromisoformat(s.replace("Z", "+00:00"))
        return dt.timestamp()
    except Exception:
        return 0.0


def _segment_sessions(events: list[dict]) -> list[list[dict]]:
    """Group sorted focus events into sessions by gap threshold."""
    if not events:
        return []
    sorted_ev = sorted(events, key=lambda e: _parse_ts(e["timestamp"]))
    sessions: list[list[dict]] = []
    current: list[dict] = [sorted_ev[0]]
    for ev in sorted_ev[1:]:
        gap = _parse_ts(ev["timestamp"]) - _parse_ts(current[-1]["timestamp"])
        if gap > SESSION_GAP_SECONDS:
            sessions.append(current)
            current = [ev]
        else:
            current.append(ev)
    sessions.append(current)
    return sessions


# ── Feature extraction ────────────────────────────────────────────────────────

DEPTH_WEIGHTS = {"glance": 0.25, "read": 0.5, "edit": 0.75, "think": 0.75, "deep_work": 1.0}

_DEPTH_IDX = {"glance": 0, "read": 1, "edit": 2, "think": 2, "deep_work": 3}


def _session_to_vector(session: list[dict], all_node_ids: list[str]) -> np.ndarray:
    """
    Represent a session as a fixed-length feature vector:
      - Slot 0-3:  depth distribution (normalized counts per depth)
      - Slot 4:    total duration (log-scaled, capped)
      - Slot 5:    session length (number of events, log-scaled)
      - Slot 6:    unique nodes ratio
      - Slot 7:    opening depth index (what depth started the session)
      - Slot 8:    closing depth index
      - Slot 9-12: pairwise transition counts (depth→depth normalized)
    """
    depth_counts = np.zeros(4, dtype=float)
    total_duration = 0.0
    unique_nodes: set[str] = set()

    for ev in session:
        d = ev.get("depth", "glance")
        depth_counts[_DEPTH_IDX.get(d, 0)] += 1
        total_duration += float(ev.get("duration_seconds", 0))
        unique_nodes.add(ev.get("node_id", ""))

    n = len(session)
    depth_dist = depth_counts / max(depth_counts.sum(), 1.0)

    open_depth = _DEPTH_IDX.get(session[0].get("depth", "glance"), 0) / 3.0
    close_depth = _DEPTH_IDX.get(session[-1].get("depth", "glance"), 0) / 3.0

    # Transition matrix (4×4 → 16 features, but only 4 diagonal neighbors used)
    transitions = np.zeros(4, dtype=float)
    for i in range(len(session) - 1):
        a = _DEPTH_IDX.get(session[i].get("depth", "glance"), 0)
        b = _DEPTH_IDX.get(session[i + 1].get("depth", "glance"), 0)
        transitions[b] += 1
    transitions /= max(transitions.sum(), 1.0)

    vec = np.concatenate([
        depth_dist,
        [math.log1p(total_duration) / 10.0],
        [math.log1p(n) / 5.0],
        [len(unique_nodes) / max(n, 1)],
        [open_depth],
        [close_depth],
        transitions,
    ])
    return vec


# ── DBSCAN clustering ─────────────────────────────────────────────────────────

def _cluster_sessions(
    sessions: list[list[dict]],
    all_node_ids: list[str],
    eps: float = 0.28,
    min_samples: int = 3,
) -> dict[int, list[int]]:
    """Return {cluster_id: [session_indices]} (label -1 = noise, excluded)."""
    if len(sessions) < min_samples:
        return {}
    vectors = np.array([_session_to_vector(s, all_node_ids) for s in sessions])
    vectors = normalize(vectors)
    labels = DBSCAN(eps=eps, min_samples=min_samples, metric="euclidean").fit_predict(vectors)
    clusters: dict[int, list[int]] = defaultdict(list)
    for idx, label in enumerate(labels):
        if label >= 0:
            clusters[label].append(idx)
    return dict(clusters)


# ── Ritual summarisation ──────────────────────────────────────────────────────

def _summarise_ritual(
    cluster_sessions: list[list[dict]],
    cluster_id: int,
    all_sessions: list[list[dict]],
) -> dict[str, Any]:
    """Build a human-readable ritual record from grouped sessions."""
    all_events: list[dict] = []
    for s in cluster_sessions:
        all_events.extend(s)

    # Most common opening depth
    open_depths = [s[0].get("depth", "glance") for s in cluster_sessions]
    open_depth = max(set(open_depths), key=open_depths.count)

    # Most common closing depth
    close_depths = [s[-1].get("depth", "glance") for s in cluster_sessions]
    close_depth = max(set(close_depths), key=close_depths.count)

    # Avg session length
    avg_len = sum(len(s) for s in cluster_sessions) / len(cluster_sessions)

    # Avg total duration
    avg_dur = sum(
        sum(float(e.get("duration_seconds", 0)) for e in s)
        for s in cluster_sessions
    ) / len(cluster_sessions)

    # Depth distribution
    depth_counts: dict[str, int] = defaultdict(int)
    for ev in all_events:
        depth_counts[ev.get("depth", "glance")] += 1
    total_ev = max(sum(depth_counts.values()), 1)
    depth_dist = {k: round(v / total_ev, 3) for k, v in depth_counts.items()}

    # Top nodes (by frequency)
    node_freq: dict[str, int] = defaultdict(int)
    for ev in all_events:
        nid = ev.get("node_id", "")
        if nid:
            node_freq[nid] += 1
    top_nodes = sorted(node_freq, key=lambda k: -node_freq[k])[:5]

    # Typical time-of-day
    hours = []
    for s in cluster_sessions:
        ts = _parse_ts(s[0]["timestamp"])
        if ts > 0:
            hours.append(datetime.fromtimestamp(ts, tz=timezone.utc).hour)
    typical_hour = int(np.median(hours)) if hours else -1

    # Confidence: ratio of sessions in this cluster vs total sessions
    confidence = round(len(cluster_sessions) / max(len(all_sessions), 1), 3)

    # Predict next step from most common first-event depth
    next_predicted = open_depth

    return {
        "id": f"ritual_{cluster_id}",
        "frequency": len(cluster_sessions),
        "confidence": confidence,
        "opening_depth": open_depth,
        "closing_depth": close_depth,
        "avg_session_length": round(avg_len, 1),
        "avg_duration_seconds": round(avg_dur, 1),
        "depth_distribution": depth_dist,
        "top_nodes": top_nodes,
        "typical_hour": typical_hour,
        "next_predicted": next_predicted,
        "description": (
            f"Ritual begins with '{open_depth}', ends with '{close_depth}'. "
            f"Avg {avg_len:.0f} events, {avg_dur / 60:.0f} min. "
            f"Seen {len(cluster_sessions)}x."
        ),
    }


# ── Public API ────────────────────────────────────────────────────────────────

def detect_rituals(workspace_json: str) -> str:
    """
    Input:  JSON workspace snapshot (nodes, focus_events, ...)
    Output: JSON list of detected rituals
    """
    try:
        ws = json.loads(workspace_json)
        events: list[dict] = ws.get("focus_events", [])
        nodes: list[dict] = ws.get("nodes", [])
        all_node_ids = [n["id"] for n in nodes]

        sessions = _segment_sessions(events)
        if len(sessions) < 3:
            return json.dumps([])

        clusters = _cluster_sessions(sessions, all_node_ids)
        rituals = [
            _summarise_ritual([sessions[i] for i in indices], cid, sessions)
            for cid, indices in clusters.items()
        ]
        rituals.sort(key=lambda r: -r["confidence"])
        return json.dumps(rituals)
    except Exception as exc:
        return json.dumps({"error": str(exc)})


def predict_next_step(workspace_json: str, current_session_json: str) -> str:
    """Given current session events, predict the next depth/node action."""
    try:
        rituals = json.loads(detect_rituals(workspace_json))
        if not rituals or isinstance(rituals, dict):
            return json.dumps({"predicted_depth": "deep_work", "confidence": 0.0})
        cur_events = json.loads(current_session_json)
        if not cur_events:
            return json.dumps({"predicted_depth": rituals[0]["opening_depth"], "confidence": rituals[0]["confidence"]})
        cur_depth = cur_events[-1].get("depth", "glance")
        for ritual in rituals:
            if ritual["opening_depth"] == cur_depth or ritual["closing_depth"] == cur_depth:
                return json.dumps({
                    "predicted_depth": ritual["next_predicted"],
                    "confidence": ritual["confidence"],
                    "ritual_id": ritual["id"],
                })
        return json.dumps({"predicted_depth": rituals[0]["next_predicted"], "confidence": rituals[0]["confidence"]})
    except Exception as exc:
        return json.dumps({"error": str(exc)})
