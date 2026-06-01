"""
Phase 6.2 — Cognitive Season Detector
Detects the user's current cognitive season (Spring/Summer/Autumn/Winter)
using time-windowed statistical analysis of focus and creation behavior.
"""

import json
import math
from collections import defaultdict
from datetime import datetime, timezone, timedelta
from typing import Any

import numpy as np
from scipy import stats


# ── Time helpers ──────────────────────────────────────────────────────────────

def _ts(s: str) -> float:
    try:
        return datetime.fromisoformat(s.replace("Z", "+00:00")).timestamp()
    except Exception:
        return 0.0


def _now() -> float:
    return datetime.now(tz=timezone.utc).timestamp()


# ── Signal extraction ─────────────────────────────────────────────────────────

def _creation_rate(nodes: list[dict], window_days: int = 30) -> float:
    """Nodes created per day in window, normalised to [0,1] (cap = 5/day)."""
    cutoff = _now() - window_days * 86400
    count = sum(1 for n in nodes if _ts(n.get("created_at", "")) >= cutoff)
    rate = count / max(window_days, 1)
    return min(rate / 5.0, 1.0)


def _focus_density(events: list[dict], window_days: int = 30) -> float:
    """Average daily focus events in window, normalised (cap = 20/day)."""
    cutoff = _now() - window_days * 86400
    recent = [e for e in events if _ts(e.get("timestamp", "")) >= cutoff]
    return min(len(recent) / max(window_days * 20, 1), 1.0)


def _exploration_ratio(events: list[dict], window_days: int = 30) -> float:
    """Unique nodes / total visits in window. High = exploratory."""
    cutoff = _now() - window_days * 86400
    recent = [e for e in events if _ts(e.get("timestamp", "")) >= cutoff]
    if not recent:
        return 0.0
    unique = len({e.get("node_id", "") for e in recent})
    return min(unique / len(recent), 1.0)


def _revisit_ratio(events: list[dict], window_days: int = 30) -> float:
    """Fraction of visits that go to nodes visited before. High = consolidating."""
    cutoff = _now() - window_days * 86400
    recent = sorted(
        [e for e in events if _ts(e.get("timestamp", "")) >= cutoff],
        key=lambda e: _ts(e["timestamp"]),
    )
    if len(recent) < 2:
        return 0.0
    seen: set[str] = set()
    revisits = 0
    for ev in recent:
        nid = ev.get("node_id", "")
        if nid in seen:
            revisits += 1
        seen.add(nid)
    return revisits / max(len(recent) - 1, 1)


def _avg_entropy(nodes: list[dict]) -> float:
    entropies = [float(n.get("entropy", 0)) for n in nodes if not n.get("is_ghost")]
    return float(np.mean(entropies)) if entropies else 0.0


def _creation_trend(nodes: list[dict], window_days: int = 60) -> float:
    """
    Slope of daily creation count over last window_days.
    Positive = accelerating, negative = decelerating.
    Returns value in [-1, 1].
    """
    cutoff = _now() - window_days * 86400
    daily: dict[int, int] = defaultdict(int)
    for n in nodes:
        ts = _ts(n.get("created_at", ""))
        if ts >= cutoff:
            day = int((ts - cutoff) // 86400)
            daily[day] += 1
    if len(daily) < 4:
        return 0.0
    xs = np.array(list(daily.keys()), dtype=float)
    ys = np.array([daily[k] for k in daily.keys()], dtype=float)
    slope, _, _, _, _ = stats.linregress(xs, ys)
    return float(np.clip(slope / 2.0, -1.0, 1.0))


# ── Season classification ─────────────────────────────────────────────────────

def _classify_season(
    creation_rate: float,
    focus_density: float,
    exploration_ratio: float,
    revisit_ratio: float,
    avg_entropy: float,
    creation_trend: float,
) -> tuple[str, float, dict[str, float]]:
    """
    Returns (season_name, confidence, signal_scores).

    Spring  — high creation + high exploration + positive trend
    Summer  — high creation + high focus + low exploration (deep work)
    Autumn  — declining creation + high revisit + moderate entropy
    Winter  — low creation + low focus + low exploration
    """
    signals = {
        "creation_rate": creation_rate,
        "focus_density": focus_density,
        "exploration_ratio": exploration_ratio,
        "revisit_ratio": revisit_ratio,
        "avg_entropy": avg_entropy,
        "creation_trend": (creation_trend + 1) / 2,  # normalise to [0,1]
    }

    spring_score = (
        creation_rate * 0.30
        + exploration_ratio * 0.25
        + max(creation_trend, 0) * 0.25
        + (1 - avg_entropy) * 0.20
    )
    summer_score = (
        creation_rate * 0.25
        + focus_density * 0.35
        + (1 - exploration_ratio) * 0.25
        + (1 - avg_entropy) * 0.15
    )
    autumn_score = (
        (1 - creation_rate) * 0.20
        + revisit_ratio * 0.35
        + (1 - max(creation_trend, 0)) * 0.25
        + avg_entropy * 0.20
    )
    winter_score = (
        (1 - creation_rate) * 0.35
        + (1 - focus_density) * 0.30
        + (1 - exploration_ratio) * 0.20
        + avg_entropy * 0.15
    )

    scores = {
        "Spring": spring_score,
        "Summer": summer_score,
        "Autumn": autumn_score,
        "Winter": winter_score,
    }
    season = max(scores, key=lambda k: scores[k])
    total = sum(scores.values())
    confidence = round(scores[season] / max(total, 1e-9), 3)
    return season, confidence, signals


# ── Transition prediction ─────────────────────────────────────────────────────

_NEXT_SEASON = {"Spring": "Summer", "Summer": "Autumn", "Autumn": "Winter", "Winter": "Spring"}

TYPICAL_SEASON_DAYS = {"Spring": 45, "Summer": 60, "Autumn": 40, "Winter": 50}


def _predict_transition(season: str, nodes: list[dict], creation_trend: float) -> dict[str, Any]:
    """Estimate days until next season based on trend."""
    typical = TYPICAL_SEASON_DAYS[season]
    # If creation_trend strongly positive and season is Spring → Summer sooner
    adjustment = int(-creation_trend * typical * 0.3)
    days_remaining = max(typical + adjustment, 5)
    return {
        "next_season": _NEXT_SEASON[season],
        "estimated_days": days_remaining,
        "trend_direction": "accelerating" if creation_trend > 0.1 else (
            "decelerating" if creation_trend < -0.1 else "stable"
        ),
    }


# ── Public API ────────────────────────────────────────────────────────────────

def detect_season(workspace_json: str) -> str:
    """
    Input:  JSON workspace snapshot
    Output: JSON {season, confidence, signals, transition_prediction, aura_colors}
    """
    try:
        ws = json.loads(workspace_json)
        nodes: list[dict] = ws.get("nodes", [])
        events: list[dict] = ws.get("focus_events", [])

        cr = _creation_rate(nodes)
        fd = _focus_density(events)
        er = _exploration_ratio(events)
        rr = _revisit_ratio(events)
        ae = _avg_entropy(nodes)
        ct = _creation_trend(nodes)

        season, confidence, signals = _classify_season(cr, fd, er, rr, ae, ct)
        transition = _predict_transition(season, nodes, ct)

        aura_colors = {
            "Spring": {"primary": "#7EE8A2", "secondary": "#80FF72", "particle": "#A8FF78"},
            "Summer": {"primary": "#FFD700", "secondary": "#FFA500", "particle": "#FFEC6E"},
            "Autumn": {"primary": "#FF7043", "secondary": "#BF360C", "particle": "#FF8A65"},
            "Winter": {"primary": "#90CAF9", "secondary": "#5C6BC0", "particle": "#B3E5FC"},
        }

        return json.dumps({
            "season": season,
            "confidence": confidence,
            "signals": {k: round(v, 3) for k, v in signals.items()},
            "transition_prediction": transition,
            "aura_colors": aura_colors[season],
            "description": _season_description(season, confidence, signals),
        })
    except Exception as exc:
        return json.dumps({"error": str(exc)})


def _season_description(season: str, confidence: float, signals: dict) -> str:
    descs = {
        "Spring": (
            f"Spring — ideas are germinating. Creation rate {signals['creation_rate']:.0%}, "
            f"exploration {signals['exploration_ratio']:.0%}. The graph is expanding."
        ),
        "Summer": (
            f"Summer — peak focus mode. Deep work density {signals['focus_density']:.0%}. "
            f"You are building, not exploring."
        ),
        "Autumn": (
            f"Autumn — harvesting and consolidating. Revisit ratio {signals['revisit_ratio']:.0%}. "
            f"Old ideas are being refined."
        ),
        "Winter": (
            f"Winter — silence and incubation. Low creation ({signals['creation_rate']:.0%}), "
            f"low focus. The universe breathes slowly."
        ),
    }
    return descs.get(season, "")
