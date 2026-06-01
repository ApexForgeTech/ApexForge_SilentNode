"""
Phase 6.6 — Silent Contract Detector
Surfaces invisible obligations implied by behavioral patterns —
goals stated but never acted on, ideas approached but never opened.
"""

import json
import math
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


def _days_ago(ts: float) -> float:
    return max((_now() - ts) / 86400.0, 0.0)


# ── Signal calculators ────────────────────────────────────────────────────────

def _approach_score(
    node_id: str,
    events: list[dict],
    window_days: int = 60,
) -> float:
    """
    Score based on visit frequency with low depth (glance/read) — approaching but not engaging.
    High glance count + low deep_work = avoidance signal.
    """
    cutoff = _now() - window_days * 86400
    shallow = 0
    deep = 0
    for ev in events:
        if ev.get("node_id") != node_id:
            continue
        if _ts(ev.get("timestamp", "")) < cutoff:
            continue
        if ev.get("depth") in ("deep_work", "edit", "think"):
            deep += 1
        else:
            shallow += 1

    total = shallow + deep
    if total == 0:
        return 0.0
    # High approach score = many visits but mostly shallow
    ratio = shallow / total
    frequency_factor = min(total / 10.0, 1.0)
    return round(ratio * frequency_factor, 3)


def _journal_score(
    node_id: str,
    journal_entries: list[dict],
    events: list[dict],
    window_days: int = 90,
) -> float:
    """
    Score based on journal mentions of a node without subsequent action.
    Proxy: journal entry exists that references node + no deep work after journal.
    """
    cutoff = _now() - window_days * 86400
    linked_entries = [
        j for j in journal_entries
        if node_id in j.get("linked_nodes", [])
        and _ts(j.get("timestamp", "")) >= cutoff
    ]
    if not linked_entries:
        return 0.0

    # For each journal entry, check if there was deep work within 7 days after
    unacted = 0
    for j in linked_entries:
        j_ts = _ts(j.get("timestamp", ""))
        window_end = j_ts + 7 * 86400
        deep_after = any(
            ev.get("node_id") == node_id
            and ev.get("depth") in ("deep_work", "edit", "think")
            and j_ts <= _ts(ev.get("timestamp", "")) <= window_end
            for ev in events
        )
        if not deep_after:
            unacted += 1

    return round(min(unacted / max(len(linked_entries), 1), 1.0), 3)


def _gravity_output_gap(node: dict, events: list[dict]) -> float:
    """High gravity + low total output (depth-weighted time) = gap."""
    gravity = float(node.get("gravity", 0))
    if gravity < 0.5:
        return 0.0
    total_output = sum(
        float(ev.get("duration_seconds", 0))
        for ev in events
        if ev.get("node_id") == node["id"]
        and ev.get("depth") in ("deep_work", "edit", "think")
    )
    # Gap score: high gravity, low deep work time (cap at 1hr = 3600s)
    output_factor = 1.0 - min(total_output / 3600.0, 1.0)
    return round(gravity * output_factor / 2.0, 3)


def _estimate_age_days(node_id: str, events: list[dict], nodes: list[dict]) -> float:
    """Age of the contract = days since first visit or node creation."""
    node_map = {n["id"]: n for n in nodes}
    node = node_map.get(node_id)
    if node:
        created = _ts(node.get("created_at", ""))
        if created > 0:
            return round(_days_ago(created))
    visits = [_ts(e.get("timestamp", "")) for e in events if e.get("node_id") == node_id]
    if visits:
        return round(_days_ago(min(visits)))
    return 0.0


# ── Public API ────────────────────────────────────────────────────────────────

CONTRACT_THRESHOLD = 0.18


def detect_contracts(workspace_json: str) -> str:
    """
    Input:  JSON workspace snapshot
    Output: JSON list of SilentContract {node_id, label, strength, age_days, state, signals}
    """
    try:
        ws = json.loads(workspace_json)
        nodes: list[dict] = ws.get("nodes", [])
        events: list[dict] = ws.get("focus_events", [])
        journals: list[dict] = ws.get("journal_entries", [])

        contracts = []
        for n in nodes:
            if n.get("is_ghost") or n.get("is_fossil") or n.get("is_void"):
                continue
            nid = n["id"]

            a_score = _approach_score(nid, events)
            j_score = _journal_score(nid, journals, events)
            gg_score = _gravity_output_gap(n, events)

            strength = round(
                a_score * 0.40 + j_score * 0.40 + gg_score * 0.20,
                3,
            )
            if strength < CONTRACT_THRESHOLD:
                continue

            contracts.append({
                "node_id": nid,
                "label": n.get("content", "")[:50],
                "strength": strength,
                "age_days": _estimate_age_days(nid, events, nodes),
                "state": "pending",
                "signals": {
                    "approach_without_engagement": a_score,
                    "journal_without_action": j_score,
                    "gravity_output_gap": gg_score,
                },
                "description": (
                    f"Unresolved obligation detected: "
                    f"approached {a_score:.0%}, journaled {j_score:.0%}, "
                    f"gravity gap {gg_score:.0%}"
                ),
            })

        contracts.sort(key=lambda c: -c["strength"])
        return json.dumps(contracts)
    except Exception as exc:
        return json.dumps({"error": str(exc)})
