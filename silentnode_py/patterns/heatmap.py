"""
Phase 6.5 — Thought Heatmap Engine
Computes exponential-decay cognitive energy per node across time windows.
"""

import json
import math
from collections import defaultdict
from datetime import datetime, timezone
from typing import Any

import numpy as np


DEPTH_WEIGHTS = {"glance": 0.25, "read": 0.50, "edit": 0.75, "think": 0.75, "deep_work": 1.00}
DECAY_HALF_LIFE_DAYS = 14.0  # energy halves every 14 days


def _ts(s: str) -> float:
    try:
        return datetime.fromisoformat(s.replace("Z", "+00:00")).timestamp()
    except Exception:
        return 0.0


def _now() -> float:
    return datetime.now(tz=timezone.utc).timestamp()


def _time_decay(event_ts: float, reference_ts: float) -> float:
    """Exponential decay: energy = exp(-lambda * days_ago)."""
    days = max((reference_ts - event_ts) / 86400.0, 0.0)
    lam = math.log(2) / DECAY_HALF_LIFE_DAYS
    return math.exp(-lam * days)


def _compute_energies(
    events: list[dict],
    window_days: int,
    reference_ts: float,
) -> dict[str, float]:
    cutoff = reference_ts - window_days * 86400
    energies: dict[str, float] = defaultdict(float)
    for ev in events:
        ts = _ts(ev.get("timestamp", ""))
        if ts < cutoff or ts > reference_ts:
            continue
        nid = ev.get("node_id", "")
        if not nid:
            continue
        depth_w = DEPTH_WEIGHTS.get(ev.get("depth", "glance"), 0.25)
        dur = float(ev.get("duration_seconds", 30))
        decay = _time_decay(ts, reference_ts)
        energies[nid] += dur * depth_w * decay
    return dict(energies)


def _normalize(energies: dict[str, float]) -> dict[str, float]:
    mx = max(energies.values(), default=1.0)
    return {k: round(v / mx, 4) for k, v in energies.items()}


def _classify_zone(energy: float) -> str:
    if energy >= 0.75:
        return "hot"
    if energy >= 0.50:
        return "warm"
    if energy >= 0.25:
        return "cool"
    if energy >= 0.05:
        return "cold"
    return "dead"


def _find_obsessive_loops(
    energies: dict[str, float],
    nodes: list[dict],
    events: list[dict],
    window_days: int = 30,
) -> list[dict[str, Any]]:
    """Nodes with high energy but high entropy — attention without progress."""
    node_map = {n["id"]: n for n in nodes}
    loops = []
    for nid, energy in energies.items():
        n = node_map.get(nid)
        if not n:
            continue
        entropy = float(n.get("entropy", 0))
        if energy >= 0.50 and entropy >= 0.35:
            loops.append({
                "node_id": nid,
                "label": n.get("content", "")[:40],
                "energy": round(energy, 4),
                "entropy": round(entropy, 3),
                "loop_score": round(energy * entropy, 4),
            })
    loops.sort(key=lambda l: -l["loop_score"])
    return loops[:5]


def _find_neglected_regions(
    energies: dict[str, float],
    nodes: list[dict],
    edges: list[dict],
) -> list[dict[str, Any]]:
    """Nodes connected to hot nodes but themselves cold/dead."""
    from collections import defaultdict as dd
    adj: dict[str, set[str]] = dd(set)
    for edge in edges:
        s, t = edge.get("source_id", ""), edge.get("target_id", "")
        adj[s].add(t)
        adj[t].add(s)

    hot_nodes = {nid for nid, e in energies.items() if e >= 0.50}
    node_map = {n["id"]: n for n in nodes}
    neglected = []
    for n in nodes:
        nid = n["id"]
        if n.get("is_ghost") or n.get("is_fossil"):
            continue
        energy = energies.get(nid, 0.0)
        if energy >= 0.10:
            continue
        hot_neighbors = len(adj.get(nid, set()) & hot_nodes)
        if hot_neighbors >= 1:
            neglected.append({
                "node_id": nid,
                "label": n.get("content", "")[:40],
                "energy": round(energy, 4),
                "hot_neighbors": hot_neighbors,
                "gravity": round(float(n.get("gravity", 0)), 3),
            })
    neglected.sort(key=lambda r: -r["hot_neighbors"])
    return neglected[:6]


def compute_heatmap(workspace_json: str, window_days: int = 30) -> str:
    """
    Input:  JSON workspace snapshot + optional window_days
    Output: JSON {energies, zones, obsessive_loops, neglected_regions, stats}
    """
    try:
        ws = json.loads(workspace_json)
        nodes: list[dict] = ws.get("nodes", [])
        edges: list[dict] = ws.get("edges", [])
        events: list[dict] = ws.get("focus_events", [])
        ref = _now()

        raw = _compute_energies(events, window_days, ref)
        if not raw:
            return json.dumps({"energies": {}, "zones": {}, "obsessive_loops": [], "neglected_regions": [], "stats": {}})

        norm = _normalize(raw)
        zones = {nid: _classify_zone(e) for nid, e in norm.items()}

        zone_counts = defaultdict(int)
        for z in zones.values():
            zone_counts[z] += 1

        loops = _find_obsessive_loops(norm, nodes, events, window_days)
        neglected = _find_neglected_regions(norm, nodes, edges)

        # Top hot nodes with labels
        node_map = {n["id"]: n for n in nodes}
        top_hot = sorted(
            [(nid, e) for nid, e in norm.items() if e >= 0.50],
            key=lambda x: -x[1],
        )[:10]
        top_hot_labeled = [
            {
                "node_id": nid,
                "label": node_map.get(nid, {}).get("content", "")[:40],
                "energy": e,
                "zone": zones[nid],
            }
            for nid, e in top_hot
        ]

        stats = {
            "total_nodes_tracked": len(norm),
            "zone_counts": dict(zone_counts),
            "avg_energy": round(float(np.mean(list(norm.values()))), 4),
            "median_energy": round(float(np.median(list(norm.values()))), 4),
        }

        return json.dumps({
            "energies": norm,
            "zones": zones,
            "top_hot": top_hot_labeled,
            "obsessive_loops": loops,
            "neglected_regions": neglected,
            "stats": stats,
            "window_days": window_days,
        })
    except Exception as exc:
        return json.dumps({"error": str(exc)})
