"""
Phase 7.4 — Digital Shadow Detector
Identifies ideas in a state of permanent becoming — started but never completed,
revisited cyclically, never formally closed or released.
"""

import json
from collections import defaultdict
from datetime import datetime, timezone
from typing import Any


def _ts(s: str) -> float:
    try:
        return datetime.fromisoformat(s.replace("Z", "+00:00")).timestamp()
    except Exception:
        return 0.0


def _now() -> float:
    return datetime.now(tz=timezone.utc).timestamp()


# ── Revisit detection ─────────────────────────────────────────────────────────

def _count_revisits(node_id: str, events: list[dict], gap_days: float = 7.0) -> int:
    """
    Count how many times the user returned to this node after a gap of >= gap_days.
    A 'return' = a focus event after gap_days silence.
    """
    node_events = sorted(
        [e for e in events if e.get("node_id") == node_id],
        key=lambda e: _ts(e.get("timestamp", "")),
    )
    if len(node_events) < 2:
        return 0
    gap_secs = gap_days * 86400
    revisits = 0
    prev_ts = _ts(node_events[0].get("timestamp", ""))
    for ev in node_events[1:]:
        curr_ts = _ts(ev.get("timestamp", ""))
        if curr_ts - prev_ts >= gap_secs:
            revisits += 1
        prev_ts = curr_ts
    return revisits


def _is_incomplete(node: dict, events: list[dict]) -> bool:
    """
    Heuristic: node has been visited but shows no 'deep completion' signal.
    Deep completion proxy: at least 2 deep_work sessions and entropy < 0.2.
    """
    node_id = node["id"]
    deep_sessions = sum(
        1 for e in events
        if e.get("node_id") == node_id and e.get("depth") in ("deep_work", "edit")
    )
    entropy = float(node.get("entropy", 0))
    # If it has deep work but entropy is still high → not converging
    if deep_sessions >= 2 and entropy > 0.4:
        return True
    # If it has been visited many times but never gone deep → incomplete
    total_visits = sum(1 for e in events if e.get("node_id") == node_id)
    if total_visits >= 5 and deep_sessions == 0:
        return True
    return False


def _cyclical_revisit(node_id: str, events: list[dict]) -> bool:
    """True if node has been revisited at least 3 times with 7+ day gaps."""
    return _count_revisits(node_id, events, gap_days=7) >= 3


def _never_formally_closed(node: dict) -> bool:
    """A node is considered open if it's not a fossil, not archived, not void."""
    return (
        not node.get("is_fossil", False)
        and not node.get("is_void", False)
        and float(node.get("entropy", 0)) < 0.90  # not about to ghost
    )


# ── Shadow intensity ──────────────────────────────────────────────────────────

def _shadow_intensity(
    node: dict,
    events: list[dict],
    revisit_count: int,
) -> float:
    """
    Intensity = how persistent and unresolved this shadow is.
    High revisit + high gravity + medium entropy = strong shadow.
    """
    gravity = float(node.get("gravity", 0))
    entropy = float(node.get("entropy", 0))
    total_visits = sum(1 for e in events if e.get("node_id") == node["id"])
    deep_visits = sum(
        1 for e in events
        if e.get("node_id") == node["id"] and e.get("depth") in ("deep_work", "edit")
    )

    revisit_factor = min(revisit_count / 10.0, 1.0)
    gravity_factor = min(gravity / 3.0, 1.0)
    # Entropy in middle range (0.3-0.6) = maximum shadow — not dying but not alive
    entropy_factor = 1.0 - abs(entropy - 0.45) / 0.55
    approach_factor = (total_visits - deep_visits) / max(total_visits, 1)

    intensity = (
        revisit_factor * 0.35
        + gravity_factor * 0.25
        + entropy_factor * 0.25
        + approach_factor * 0.15
    )
    return round(min(intensity, 1.0), 4)


# ── Public API ────────────────────────────────────────────────────────────────

def detect_shadows(workspace_json: str) -> str:
    """
    Input:  JSON workspace snapshot
    Output: JSON list of DigitalShadow {node_id, label, revisit_count, age_days, intensity, description}
    """
    try:
        ws = json.loads(workspace_json)
        nodes: list[dict] = ws.get("nodes", [])
        events: list[dict] = ws.get("focus_events", [])

        shadows = []
        for n in nodes:
            if n.get("is_ghost") or n.get("is_fossil") or n.get("is_void"):
                continue

            nid = n["id"]
            total_visits = sum(1 for e in events if e.get("node_id") == nid)
            if total_visits < 2:
                continue

            incomplete = _is_incomplete(n, events)
            cyclical = _cyclical_revisit(nid, events)
            open_ = _never_formally_closed(n)

            if not (incomplete and open_):
                continue
            if not cyclical and total_visits < 6:
                continue

            revisit_count = _count_revisits(nid, events)
            created_ts = _ts(n.get("created_at", ""))
            age_days = round((_now() - created_ts) / 86400.0) if created_ts > 0 else 0
            intensity = _shadow_intensity(n, events, revisit_count)

            if intensity < 0.15:
                continue

            deep_visits = sum(
                1 for e in events
                if e.get("node_id") == nid and e.get("depth") in ("deep_work", "edit")
            )

            shadows.append({
                "node_id": nid,
                "label": n.get("content", "")[:50],
                "revisit_count": revisit_count,
                "total_visits": total_visits,
                "deep_visits": deep_visits,
                "age_days": age_days,
                "entropy": round(float(n.get("entropy", 0)), 3),
                "gravity": round(float(n.get("gravity", 0)), 3),
                "intensity": intensity,
                "state": "shadow",
                "description": (
                    f"Revisited {revisit_count}x with 7+ day gaps over {age_days} days. "
                    f"Visits: {total_visits} total, {deep_visits} deep. "
                    f"Still unresolved (entropy {n.get('entropy', 0):.0%})."
                ),
            })

        shadows.sort(key=lambda s: -s["intensity"])
        return json.dumps(shadows)
    except Exception as exc:
        return json.dumps({"error": str(exc)})
