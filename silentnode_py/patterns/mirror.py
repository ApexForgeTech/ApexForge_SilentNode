"""
Phase 6.4 — Cognitive Mirror
Generates an objective self-portrait from behavioral data.
No judgement — only honest visibility.
"""

import json
import math
from collections import defaultdict
from datetime import datetime, timezone
from typing import Any

import numpy as np
from scipy import stats


# ── Helpers ───────────────────────────────────────────────────────────────────

def _ts(s: str) -> float:
    try:
        return datetime.fromisoformat(s.replace("Z", "+00:00")).timestamp()
    except Exception:
        return 0.0


def _now() -> float:
    return datetime.now(tz=timezone.utc).timestamp()


def _days_ago(ts: float) -> float:
    return (_now() - ts) / 86400.0


# ── Priority gap ──────────────────────────────────────────────────────────────

def _calc_priority_gap(
    nodes: list[dict],
    events: list[dict],
    top_n: int = 10,
) -> list[dict[str, Any]]:
    """
    Compare node gravity rank (stated importance) vs focus trail rank (actual attention).
    Returns nodes where the two ranks diverge most significantly.
    """
    # Gravity rank (high gravity = user considers important)
    sorted_by_gravity = sorted(nodes, key=lambda n: -float(n.get("gravity", 0)))
    gravity_rank = {n["id"]: i for i, n in enumerate(sorted_by_gravity)}

    # Focus rank (high focus time = actual attention)
    focus_time: dict[str, float] = defaultdict(float)
    for ev in events:
        nid = ev.get("node_id", "")
        if nid:
            focus_time[nid] += float(ev.get("duration_seconds", 0))
    sorted_by_focus = sorted(nodes, key=lambda n: -focus_time.get(n["id"], 0))
    focus_rank = {n["id"]: i for i, n in enumerate(sorted_by_focus)}

    gaps = []
    for n in nodes:
        nid = n["id"]
        gr = gravity_rank.get(nid, len(nodes))
        fr = focus_rank.get(nid, len(nodes))
        gap = fr - gr  # positive = getting less focus than gravity implies
        gaps.append({
            "node_id": nid,
            "label": n.get("content", "")[:40],
            "gravity_rank": gr,
            "focus_rank": fr,
            "gap": gap,
            "gap_direction": "under-focused" if gap > 0 else ("over-focused" if gap < 0 else "aligned"),
        })

    gaps.sort(key=lambda g: abs(g["gap"]), reverse=True)
    return gaps[:top_n]


# ── Creative patterns ─────────────────────────────────────────────────────────

def _analyze_creative_patterns(events: list[dict]) -> dict[str, Any]:
    """
    When is the user actually most creative?
    Measure: focus events where depth == 'deep_work', 'edit', or legacy 'think', by hour and weekday.
    """
    deep_events = [e for e in events if e.get("depth") in ("deep_work", "edit", "think")]
    if not deep_events:
        return {"peak_hour": -1, "peak_weekday": -1, "hourly_distribution": {}, "weekday_distribution": {}}

    hourly: dict[int, float] = defaultdict(float)
    weekday: dict[int, float] = defaultdict(float)
    for ev in deep_events:
        ts = _ts(ev.get("timestamp", ""))
        if ts <= 0:
            continue
        dt = datetime.fromtimestamp(ts, tz=timezone.utc)
        dur = float(ev.get("duration_seconds", 60))
        hourly[dt.hour] += dur
        weekday[dt.weekday()] += dur

    peak_hour = max(hourly, key=lambda h: hourly[h]) if hourly else -1
    peak_weekday = max(weekday, key=lambda w: weekday[w]) if weekday else -1

    weekday_names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
    return {
        "peak_hour": peak_hour,
        "peak_hour_label": f"{peak_hour:02d}:00" if peak_hour >= 0 else "unknown",
        "peak_weekday": peak_weekday,
        "peak_weekday_label": weekday_names[peak_weekday] if 0 <= peak_weekday <= 6 else "unknown",
        "hourly_distribution": {str(h): round(t / 3600, 2) for h, t in sorted(hourly.items())},
        "weekday_distribution": {weekday_names[w]: round(t / 3600, 2) for w, t in sorted(weekday.items())},
    }


# ── Blind spots ───────────────────────────────────────────────────────────────

def _find_blind_spots(
    nodes: list[dict],
    edges: list[dict],
    events: list[dict],
    silence_days: int = 21,
) -> list[dict[str, Any]]:
    """
    Nodes connected to highly-active nodes but untouched for silence_days.
    """
    cutoff = _now() - silence_days * 86400

    # Recent focus
    recently_focused = {
        e.get("node_id", "")
        for e in events
        if _ts(e.get("timestamp", "")) >= cutoff
    }

    # Adjacency
    adj: dict[str, set[str]] = defaultdict(set)
    for edge in edges:
        s, t = edge.get("source_id", ""), edge.get("target_id", "")
        adj[s].add(t)
        adj[t].add(s)

    blind_spots = []
    for n in nodes:
        nid = n["id"]
        if nid in recently_focused:
            continue
        if n.get("is_ghost") or n.get("is_fossil"):
            continue
        # Count how many of its neighbours are recently active
        active_neighbours = len(adj.get(nid, set()) & recently_focused)
        if active_neighbours >= 2:
            blind_spots.append({
                "node_id": nid,
                "label": n.get("content", "")[:40],
                "active_neighbours": active_neighbours,
                "days_silent": round(_days_ago(_ts(n.get("accessed_at", "")))),
                "gravity": round(float(n.get("gravity", 0)), 3),
            })

    blind_spots.sort(key=lambda b: -b["active_neighbours"])
    return blind_spots[:8]


# ── Obsession mapping ─────────────────────────────────────────────────────────

def _map_obsessions(
    nodes: list[dict],
    events: list[dict],
    window_days: int = 30,
) -> list[dict[str, Any]]:
    """
    Nodes with high focus time but low entropy reversal (revisited, not evolved).
    High attention / low output = obsession.
    """
    cutoff = _now() - window_days * 86400
    focus_time: dict[str, float] = defaultdict(float)
    visit_count: dict[str, int] = defaultdict(int)
    for ev in events:
        ts = _ts(ev.get("timestamp", ""))
        if ts >= cutoff:
            nid = ev.get("node_id", "")
            focus_time[nid] += float(ev.get("duration_seconds", 0))
            visit_count[nid] += 1

    if not focus_time:
        return []

    max_time = max(focus_time.values(), default=1.0)
    node_map = {n["id"]: n for n in nodes}

    obsessions = []
    for nid, time in focus_time.items():
        n = node_map.get(nid)
        if not n:
            continue
        entropy = float(n.get("entropy", 0))
        # Obsession = high focus + medium/high entropy (not improving)
        focus_score = time / max_time
        if focus_score >= 0.4 and entropy >= 0.3:
            obsessions.append({
                "node_id": nid,
                "label": n.get("content", "")[:40],
                "focus_hours": round(time / 3600, 2),
                "visit_count": visit_count[nid],
                "entropy": round(entropy, 3),
                "obsession_score": round(focus_score * entropy, 3),
                "description": f"High attention ({time / 3600:.1f}h) but entropy still {entropy:.0%}",
            })

    obsessions.sort(key=lambda o: -o["obsession_score"])
    return obsessions[:5]


# ── Evolution portrait ────────────────────────────────────────────────────────

def _trace_evolution(
    nodes: list[dict],
    events: list[dict],
) -> dict[str, Any]:
    """How has the center of gravity shifted over time?"""
    if not nodes or not events:
        return {}

    # Divide time into quarters
    all_ts = [_ts(e.get("timestamp", "")) for e in events if _ts(e.get("timestamp", "")) > 0]
    if len(all_ts) < 4:
        return {"insufficient_data": True}

    all_ts.sort()
    quarter_size = len(all_ts) // 4
    quarters = [all_ts[i * quarter_size:(i + 1) * quarter_size] for i in range(4)]

    node_map = {n["id"]: n for n in nodes}

    def top_nodes_in_period(ts_list: list[float]) -> list[str]:
        ts_set = set(ts_list)
        counts: dict[str, int] = defaultdict(int)
        for ev in events:
            if _ts(ev.get("timestamp", "")) in ts_set:
                counts[ev.get("node_id", "")] += 1
        return [nid for nid, _ in sorted(counts.items(), key=lambda x: -x[1])[:3]]

    quarters_data = []
    for i, q_ts in enumerate(quarters):
        top = top_nodes_in_period(q_ts)
        quarters_data.append({
            "period": f"Q{i+1}",
            "top_nodes": [
                {"id": nid, "label": node_map.get(nid, {}).get("content", "")[:30]}
                for nid in top
            ],
        })

    return {"quarters": quarters_data}


# ── Public API ────────────────────────────────────────────────────────────────

def generate_portrait(workspace_json: str) -> str:
    """
    Input:  JSON workspace snapshot
    Output: JSON CognitivePortrait {priority_gap, creative_patterns, blind_spots, obsessions, evolution}
    """
    try:
        ws = json.loads(workspace_json)
        nodes = ws.get("nodes", [])
        edges = ws.get("edges", [])
        events = ws.get("focus_events", [])

        portrait = {
            "priority_gap": _calc_priority_gap(nodes, events),
            "creative_patterns": _analyze_creative_patterns(events),
            "blind_spots": _find_blind_spots(nodes, edges, events),
            "obsessions": _map_obsessions(nodes, events),
            "evolution": _trace_evolution(nodes, events),
        }
        return json.dumps(portrait)
    except Exception as exc:
        return json.dumps({"error": str(exc)})
