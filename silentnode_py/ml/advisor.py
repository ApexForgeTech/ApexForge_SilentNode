"""
Daily ML advisor for SilentNode.

This layer turns the learned graph into operational recommendations: what to
touch today, which focus depth to use, and which habits need reinforcement.
It is intentionally deterministic on top of the learned features so the user
can trust and inspect the decision.
"""

from __future__ import annotations

from typing import Dict, List

from .features import build_text_tokens, load_nodes, load_focus_events, node_focus_stats


_ROUTINE_KEYWORDS = {
    "daily", "routine", "habit", "prayer", "quran", "namaz", "tahajjud",
    "listening", "english", "work", "review", "gundelik", "gündəlik",
    "rutin", "dinleme", "dinləmə",
}
_STUDY_KEYWORDS = {
    "study", "test", "tests", "exam", "magistr", "academy", "informatics",
    "admission", "prep", "preparation", "imtahan", "hazirliq", "hazırlıq",
    "qebul", "qəbul", "informatika",
}
_PROJECT_KEYWORDS = {
    "project", "sprint", "system", "app", "build", "release", "layihe",
    "layihə", "proyekt",
}
_SPIRITUAL_KEYWORDS = {"prayer", "quran", "namaz", "tahajjud", "dua"}
_LANGUAGE_KEYWORDS = {"english", "listening", "ingilis", "ingilisce", "ingiliscə"}
_WORK_KEYWORDS = {"work", "iş", "gomruk", "gömrük", "customs"}


def _tokens(node: Dict) -> set:
    text = f"{node.get('nickname', '')} {node.get('content', '')}"
    return set(build_text_tokens(text))


def _preview(node: Dict, limit: int = 90) -> str:
    nick = str(node.get("nickname") or "").strip()
    content = str(node.get("content") or "").strip().splitlines()[0]
    text = nick or content
    return text[:limit]


def _focus_depth_for(node: Dict, toks: set) -> str:
    node_type = str(node.get("node_type", "")).lower()
    if toks & _STUDY_KEYWORDS:
        return "deep_work"
    if node_type == "project" or toks & _PROJECT_KEYWORDS:
        return "edit"
    if toks & _ROUTINE_KEYWORDS or node_type in {"process", "artifact", "media"}:
        return "read"
    if node_type == "idea":
        return "edit"
    return "glance"


def _minutes_for(depth: str, toks: set) -> int:
    if depth == "deep_work":
        return 90 if toks & {"magistr", "informatics", "admission", "informatika"} else 60
    if depth == "edit":
        return 25
    if depth == "read":
        return 20 if toks & _ROUTINE_KEYWORDS else 30
    return 5


def _priority_score(node: Dict, focus: Dict) -> float:
    entropy = float(node.get("entropy") or 0.0)
    gravity = float(node.get("gravity") or 1.0)
    velocity = float(node.get("velocity") or 0.0)
    access = float(node.get("access_count") or 0.0)
    last_focus_days = float(focus.get("last_focus_days", 30.0))
    total_seconds = float(focus.get("total_seconds", 0.0))

    neglect = min(last_focus_days / 14.0, 1.0)
    effort_gap = 1.0 if total_seconds <= 0 else max(0.0, 1.0 - total_seconds / 7200.0)
    access_stability = min(access / 10.0, 1.0)
    toks = _tokens(node)
    domain_boost = 0.0
    if toks & _STUDY_KEYWORDS:
        domain_boost += 0.22
    if toks & _ROUTINE_KEYWORDS:
        domain_boost += 0.12
    if toks & _SPIRITUAL_KEYWORDS:
        domain_boost += 0.08
    if toks & _LANGUAGE_KEYWORDS:
        domain_boost += 0.05

    score = (
        gravity * 0.22
        + entropy * 0.18
        + velocity * 0.08
        + neglect * 0.24
        + effort_gap * 0.18
        + access_stability * 0.10
        + domain_boost
    )
    return round(score, 4)


def _reason(node: Dict, focus: Dict, toks: set) -> str:
    parts = []
    if toks & _ROUTINE_KEYWORDS:
        parts.append("recurring routine")
    if toks & _STUDY_KEYWORDS:
        parts.append("exam/study pressure")
    if toks & _PROJECT_KEYWORDS:
        parts.append("project outcome")
    if float(focus.get("total_seconds", 0.0)) <= 0:
        parts.append("no focus logged yet")
    elif float(focus.get("last_focus_days", 30.0)) >= 3:
        parts.append("not focused recently")
    return ", ".join(parts) or "balanced graph priority"


def _domains(toks: set) -> List[str]:
    domains = []
    if toks & _SPIRITUAL_KEYWORDS:
        domains.append("spiritual")
    if toks & _STUDY_KEYWORDS:
        domains.append("exam")
    if toks & _LANGUAGE_KEYWORDS:
        domains.append("language")
    if toks & _WORK_KEYWORDS:
        domains.append("work")
    if toks & _PROJECT_KEYWORDS:
        domains.append("project")
    return domains or ["general"]


def daily_plan(db_path: str = None, limit: int = 8) -> Dict:
    """Return a daily operating plan derived from vault nodes and focus events."""
    nodes = [
        n for n in load_nodes(db_path)
        if not n.get("is_ghost") and not n.get("is_fossil") and not n.get("is_void")
    ]
    events = load_focus_events(db_path)
    focus_stats = node_focus_stats(nodes, events)

    candidates: List[Dict] = []
    for node in nodes:
        toks = _tokens(node)
        focus = focus_stats.get(node["id"], {})
        depth = _focus_depth_for(node, toks)
        score = _priority_score(node, focus)
        candidates.append({
            "node_id": node["id"],
            "title": _preview(node),
            "node_type": str(node.get("node_type", "")).lower(),
            "recommended_depth": depth,
            "recommended_minutes": _minutes_for(depth, toks),
            "priority": score,
            "reason": _reason(node, focus, toks),
            "domains": _domains(toks),
            "focus_count": int(focus.get("focus_count", 0)),
            "total_focus_minutes": round(float(focus.get("total_seconds", 0.0)) / 60.0, 1),
        })

    ranked = sorted(candidates, key=lambda item: item["priority"], reverse=True)
    routines = sorted(
        [
            item for item in ranked
            if "recurring routine" in item["reason"]
            and "exam/study pressure" not in item["reason"]
        ],
        key=lambda item: item["priority"],
        reverse=True,
    )[:4]
    deep_work = [
        item for item in ranked
        if item["recommended_depth"] == "deep_work"
    ][:3]

    return {
        "status": "ok",
        "summary": {
            "nodes": len(nodes),
            "focus_events": len(events),
            "message": "Use routines as daily anchors and deep_work for real progress blocks.",
        },
        "today": {
            "anchors": routines,
            "deep_work_targets": deep_work,
            "next_best": ranked[:limit],
        },
        "metrics": {
            "routine_count": len(routines),
            "deep_work_count": len(deep_work),
            "unfocused_count": sum(1 for item in ranked if item["focus_count"] == 0),
        },
        "focus_depths": [
            {
                "depth": "glance",
                "meaning": "Quick touch/check. Use for 2-5 minutes to keep a node alive.",
                "best_for": "reviewing status, finding what to do next",
            },
            {
                "depth": "read",
                "meaning": "Input/consumption. Use when reading, listening, Quran/English review, or checking notes.",
                "best_for": "Quran reading, English listening, reviewing material",
            },
            {
                "depth": "edit",
                "meaning": "Changing the node. Use when writing, planning, refining, or logging progress.",
                "best_for": "journal updates, improving plans, adding test results",
            },
            {
                "depth": "deep_work",
                "meaning": "Protected serious work. Use for 45-120 minute blocks with one target.",
                "best_for": "Magistr preparation, informatics tests, exam sprint work",
            },
        ],
        "operating_rules": [
            "Create one stable node per long-lived area; repeat focus events on that node instead of creating a new node every day.",
            "Create temporary project/sprint nodes only for bounded goals, such as an exam phase.",
            "For daily habits, record focus on the existing process node every day.",
            "Use edit when the content changes; use read when you consume/review; use deep_work when you seriously study.",
            "Retrain ML after a batch of new nodes or corrections so feedback becomes part of the model.",
        ],
    }
