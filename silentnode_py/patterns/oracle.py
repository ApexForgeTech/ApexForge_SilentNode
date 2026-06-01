"""
Phase 6.3 — Oracle Layer
Generates predictive cognitive signals from long-term behavioral patterns.
No AI — pure statistical pattern recognition over focus history.
"""

import json
import math
from collections import defaultdict
from datetime import datetime, timezone
from typing import Any

import numpy as np


# ── Helpers ───────────────────────────────────────────────────────────────────

def _ts(s: str) -> float:
    try:
        return datetime.fromisoformat(s.replace("Z", "+00:00")).timestamp()
    except Exception:
        return 0.0


def _now_hour() -> int:
    return datetime.now(tz=timezone.utc).hour


def _days_ago(ts: float) -> float:
    return (_now() - ts) / 86400.0


def _now() -> float:
    return datetime.now(tz=timezone.utc).timestamp()


# ── Node anticipation ─────────────────────────────────────────────────────────

def _predict_next_navigation(
    events: list[dict],
    history_days: int = 90,
) -> list[dict[str, Any]]:
    """
    Which nodes are statistically likely to be visited in the current hour?
    Uses a 2-hour time window: events from ±1 hour of current hour across history.
    """
    current_hour = _now_hour()
    cutoff = _now() - history_days * 86400

    hour_node_counts: dict[str, int] = defaultdict(int)
    total_in_window = 0

    for ev in events:
        ts = _ts(ev.get("timestamp", ""))
        if ts < cutoff:
            continue
        ev_hour = datetime.fromtimestamp(ts, tz=timezone.utc).hour
        hour_dist = min(abs(ev_hour - current_hour), 24 - abs(ev_hour - current_hour))
        if hour_dist <= 1:
            nid = ev.get("node_id", "")
            if nid:
                hour_node_counts[nid] += 1
                total_in_window += 1

    if not hour_node_counts:
        return []

    signals = []
    for nid, count in sorted(hour_node_counts.items(), key=lambda x: -x[1])[:5]:
        confidence = round(count / max(total_in_window, 1), 3)
        if confidence >= 0.05:
            signals.append({
                "type": "NodeAnticipation",
                "node_id": nid,
                "confidence": confidence,
                "reason": f"Visited ~{count}x during hour {current_hour} in past {history_days} days",
            })
    return signals


# ── Ghost return prediction ───────────────────────────────────────────────────

def _predict_ghost_returns(
    nodes: list[dict],
    events: list[dict],
    history_days: int = 90,
) -> list[dict[str, Any]]:
    """
    Ghosts that previously re-emerged after similar conditions.
    Heuristic: ghost nodes whose gravity > 0.3 and that have been visited
    at least once in history are candidates for return.
    """
    ghost_ids = {n["id"] for n in nodes if n.get("is_ghost", False)}
    if not ghost_ids:
        return []

    cutoff = _now() - history_days * 86400
    ghost_visits: dict[str, int] = defaultdict(int)
    for ev in events:
        ts = _ts(ev.get("timestamp", ""))
        if ts >= cutoff and ev.get("node_id", "") in ghost_ids:
            ghost_visits[ev["node_id"]] += 1

    node_gravity = {n["id"]: float(n.get("gravity", 0)) for n in nodes}

    signals = []
    for gid, visits in ghost_visits.items():
        g = node_gravity.get(gid, 0)
        if g < 0.2:
            continue
        confidence = round(min((g * 0.5 + visits * 0.1), 1.0), 3)
        signals.append({
            "type": "GhostReturn",
            "node_id": gid,
            "confidence": confidence,
            "reason": f"Ghost with gravity {g:.2f}, visited {visits}x recently",
        })
    signals.sort(key=lambda s: -s["confidence"])
    return signals[:3]


# ── Season transition warning ─────────────────────────────────────────────────

def _season_transition_signal(season_json: str) -> list[dict[str, Any]]:
    """Emit a warning if season transition is estimated within 14 days."""
    try:
        season_data = json.loads(season_json)
        if "error" in season_data:
            return []
        tp = season_data.get("transition_prediction", {})
        days = tp.get("estimated_days", 999)
        if days <= 14:
            return [{
                "type": "SeasonTransition",
                "node_id": None,
                "confidence": round(1.0 - days / 14.0, 3),
                "reason": (
                    f"Cognitive season may shift to {tp.get('next_season', '?')} "
                    f"in ~{days} days ({tp.get('trend_direction', '')})"
                ),
            }]
    except Exception:
        pass
    return []


# ── High-resonance pair signal ────────────────────────────────────────────────

def _resonance_signals(resonances_json: str) -> list[dict[str, Any]]:
    """Surface top resonance pairs as oracle signals."""
    try:
        pairs = json.loads(resonances_json)
        if not isinstance(pairs, list):
            return []
        signals = []
        for p in pairs[:3]:
            sim = float(p.get("similarity", 0))
            if sim >= 0.70:
                signals.append({
                    "type": "HighResonancePair",
                    "node_id": p.get("node_a"),
                    "node_id_b": p.get("node_b"),
                    "confidence": round(sim, 3),
                    "reason": f"Semantic similarity {sim:.0%} — potential hidden connection",
                })
        return signals
    except Exception:
        return []


# ── Public API ────────────────────────────────────────────────────────────────

def generate_signals(
    workspace_json: str,
    season_json: str = "{}",
    resonances_json: str = "[]",
) -> str:
    """
    Input:  workspace JSON, optional pre-computed season + resonance JSON
    Output: JSON list of OracleSignal objects sorted by confidence
    """
    try:
        ws = json.loads(workspace_json)
        nodes: list[dict] = ws.get("nodes", [])
        events: list[dict] = ws.get("focus_events", [])

        signals: list[dict] = []
        signals.extend(_predict_next_navigation(events))
        signals.extend(_predict_ghost_returns(nodes, events))
        signals.extend(_season_transition_signal(season_json))
        signals.extend(_resonance_signals(resonances_json))

        signals.sort(key=lambda s: -s["confidence"])
        return json.dumps(signals)
    except Exception as exc:
        return json.dumps({"error": str(exc)})
