"""
Phase 10.3 — Personal Lore Chronicle + Hero's Journey Mapping

Vision.md:
  "The Chronicle is the readable narrative form of the Lore System.
   It is not a journal. It is not a log. It is a living document
   that evolves as new arcs complete and new chapters open —
   written in the language of pattern, not prose."

  "Hero's Journey Mapping: the system optionally maps your behavioral
   patterns onto universal narrative structures: the call to a new
   project, the threshold of commitment, the trials and failures,
   the moment of transformation, the return with new capability.
   No mythology is imposed. The patterns are observed and reflected."
"""

from __future__ import annotations

import json
from datetime import datetime, timezone
from typing import Any


# ── Arc type definitions ───────────────────────────────────────────────────────

# Maps arc_type → (hero_journey_stage, stage_description)
_HERO_MAP: dict[str, tuple[str, str]] = {
    "origin":         ("THE CALL",          "The universe calls — a new project or idea is born"),
    "conflict":       ("THE TRIALS",         "Friction and resistance — the project faces adversity"),
    "resolution":     ("THE RETURN",         "Integration — conflict resolves and understanding consolidates"),
    "revelation":     ("TRANSFORMATION",     "A sudden insight shifts the entire trajectory"),
    "transformation": ("TRANSFORMATION",     "The protagonist evolves — the project reaches maturity"),
    "legacy":         ("THE RETURN",         "What endures — the lasting imprint of completed work"),
    "tectonic":       ("THE THRESHOLD",      "The point of no return — a fundamental paradigm shift"),
}

_ARC_ICONS: dict[str, str] = {
    "origin":         "◉",
    "conflict":       "⚔",
    "resolution":     "◇",
    "revelation":     "✦",
    "transformation": "⟳",
    "legacy":         "◆",
    "tectonic":       "⚡",
}

_STAGE_ORDER = [
    "THE CALL",
    "THE THRESHOLD",
    "THE TRIALS",
    "TRANSFORMATION",
    "THE RETURN",
]


def _ts_to_str(s: str) -> str:
    try:
        dt = datetime.fromisoformat(s.replace("Z", "+00:00"))
        return dt.strftime("%Y-%m-%d")
    except Exception:
        return s[:10] if len(s) >= 10 else s


def _sig_bar(v: float, width: int = 10) -> str:
    filled = max(0, min(width, round(v * width)))
    return "█" * filled + "░" * (width - filled)


# ── Chronicle generator ────────────────────────────────────────────────────────

class PersonalChronicle:
    """
    Generates the readable Chronicle narrative from workspace lore arcs,
    cognitive seasons, and tectonic events.

    Vision.md: "A living document that evolves as new arcs complete and
    new chapters open — written in the language of pattern, not prose."
    """

    def generate(self, workspace_json: str) -> str:
        try:
            ws = json.loads(workspace_json)
        except Exception as exc:
            return f"[Chronicle Error: {exc}]"

        lore_entries  = ws.get("lore_entries", [])
        nodes         = ws.get("nodes", [])
        season        = ws.get("season", "Summer")
        civ_count     = len(ws.get("civilizations", []))
        crystal_count = len(ws.get("crystals", []))
        ghost_count   = sum(1 for n in nodes if n.get("is_ghost"))
        fossil_count  = sum(1 for n in nodes if n.get("is_fossil"))
        total_nodes   = len(nodes)

        lines: list[str] = []

        lines.append("━" * 60)
        lines.append("  THE CHRONICLE")
        lines.append('  "The living document of who you are becoming."')
        lines.append("━" * 60)
        lines.append("")

        if not lore_entries:
            lines.append("  The Chronicle has not yet begun.")
            lines.append("")
            lines.append("  Arcs form through time and attention —")
            lines.append("  continue building, and the narrative will emerge.")
            lines.append("")
            lines.append("  Current universe state:")
            lines.append(f"    Season   : {season}")
            lines.append(f"    Nodes    : {total_nodes}")
            lines.append(f"    Ghosts   : {ghost_count}")
            lines.append(f"    Fossils  : {fossil_count}")
            lines.append(f"    Civs     : {civ_count}")
            lines.append(f"    Crystals : {crystal_count}")
            return "\n".join(lines)

        # Group arcs by type for chapter ordering
        arc_order = ["origin", "tectonic", "conflict", "revelation",
                     "transformation", "resolution", "legacy"]
        sorted_entries: list[dict] = sorted(
            lore_entries,
            key=lambda e: (
                arc_order.index(e.get("arc_type", "origin"))
                if e.get("arc_type", "origin") in arc_order else 99,
                e.get("timestamp", "")
            )
        )

        chapter_num = 0
        for entry in sorted_entries:
            arc_type  = entry.get("arc_type", "origin")
            title     = entry.get("title", "Unnamed Arc")
            ts        = entry.get("timestamp", "")
            narrative = entry.get("narrative", "")
            sig       = entry.get("significance", 0.5)
            linked    = entry.get("linked_nodes", [])

            # Resolve linked node labels
            node_map  = {n["id"]: n["content"] for n in nodes if "id" in n}
            node_names = [node_map.get(nid, "?")[:20] for nid in linked[:4]]

            icon = _ARC_ICONS.get(arc_type, "◉")
            hero_stage, _ = _HERO_MAP.get(arc_type, ("—", ""))
            chapter_num += 1
            date_str = _ts_to_str(ts) if ts else "—"

            lines.append(f"  Chapter {_roman(chapter_num)} — {arc_type.upper()}")
            lines.append(f"  {icon} {title}  [{date_str}]")
            lines.append(f"  Hero's Journey: {hero_stage}")
            lines.append(f"  Significance: {_sig_bar(sig)} {sig:.2f}")
            if narrative:
                # Word-wrap narrative at 56 chars
                words = narrative.split()
                current = "  "
                for word in words:
                    if len(current) + len(word) + 1 > 58:
                        lines.append(current)
                        current = "    " + word
                    else:
                        current += (" " if current.strip() else "") + word
                if current.strip():
                    lines.append(current)
            if node_names:
                lines.append(f"  Entities: {', '.join(node_names)}")
            lines.append("")

        # Chronicle footer: universe state summary
        lines.append("━" * 60)
        lines.append("  CURRENT STATE OF THE UNIVERSE")
        lines.append("━" * 60)
        lines.append(f"  Season    : {season}")
        lines.append(f"  Nodes     : {total_nodes}  (ghosts: {ghost_count}, fossils: {fossil_count})")
        lines.append(f"  Civs      : {civ_count}  Crystals: {crystal_count}")
        lines.append(f"  Arcs      : {len(lore_entries)}")
        lines.append("")

        return "\n".join(lines)


# ── Hero's Journey mapper ──────────────────────────────────────────────────────

class HeroJourneyMapper:
    """
    Maps lore arcs onto the universal Hero's Journey narrative structure.

    Vision.md: "The system optionally maps your behavioral patterns onto
    universal narrative structures: the call to a new project, the threshold
    of commitment, the trials and failures, the moment of transformation,
    the return with new capability. No mythology is imposed."
    """

    def analyze(self, workspace_json: str) -> dict[str, Any]:
        try:
            ws = json.loads(workspace_json)
        except Exception as exc:
            return {"error": str(exc)}

        lore_entries = ws.get("lore_entries", [])
        nodes        = ws.get("nodes", [])
        season       = ws.get("season", "Summer")

        # Map arcs to stages
        stage_arcs: dict[str, list[dict]] = {s: [] for s in _STAGE_ORDER}
        for entry in lore_entries:
            arc_type = entry.get("arc_type", "origin")
            stage, _ = _HERO_MAP.get(arc_type, ("THE CALL", ""))
            stage_arcs[stage].append(entry)

        # Determine current stage (last stage with content, or first empty stage)
        current_stage = "THE CALL"
        for stage in _STAGE_ORDER:
            if stage_arcs[stage]:
                current_stage = stage

        # Determine completeness
        stages_with_content = [s for s in _STAGE_ORDER if stage_arcs[s]]
        completion = len(stages_with_content) / len(_STAGE_ORDER)

        # Build stage summaries
        stage_summaries = {}
        for stage in _STAGE_ORDER:
            arcs = stage_arcs[stage]
            if arcs:
                avg_sig = sum(a.get("significance", 0.5) for a in arcs) / len(arcs)
                titles  = [a.get("title", "?")[:30] for a in arcs[:3]]
                stage_summaries[stage] = {
                    "arc_count":      len(arcs),
                    "avg_significance": round(avg_sig, 3),
                    "arc_titles":     titles,
                    "status":         "complete" if stage != current_stage else "active",
                }
            else:
                stage_summaries[stage] = {
                    "arc_count":   0,
                    "status":      "not_yet_begun",
                    "arc_titles":  [],
                }

        # Insight: what the journey pattern reveals
        insights = _derive_insights(stage_arcs, season, nodes)

        return {
            "current_stage":    current_stage,
            "completion":       round(completion, 3),
            "stages":           stage_summaries,
            "stage_order":      _STAGE_ORDER,
            "total_arcs":       len(lore_entries),
            "insights":         insights,
        }

    def narrative(self, workspace_json: str) -> str:
        data = self.analyze(workspace_json)
        if "error" in data:
            return f"[Hero's Journey Error: {data['error']}]"

        lines: list[str] = []
        lines.append("━" * 60)
        lines.append("  HERO'S JOURNEY MAPPING")
        lines.append('  "No mythology is imposed. The patterns are observed."')
        lines.append("━" * 60)
        lines.append("")

        total = data["total_arcs"]
        if total == 0:
            lines.append("  The journey has not yet begun.")
            lines.append("  Add lore arcs to see your narrative structure emerge.")
            lines.append("")
            return "\n".join(lines)

        current_stage = data["current_stage"]
        completion    = data["completion"]

        lines.append(f"  Current Stage : {current_stage}")
        lines.append(f"  Completion    : {_sig_bar(completion)} {completion:.0%}")
        lines.append(f"  Total Arcs    : {total}")
        lines.append("")
        lines.append("  ─── STAGES ──────────────────────────────────────────")
        lines.append("")

        for stage in _STAGE_ORDER:
            info = data["stages"][stage]
            is_current = (stage == current_stage)
            status_sym = "▶" if is_current else ("✓" if info["arc_count"] > 0 else "○")
            style = f"[{stage}]" if is_current else f" {stage} "
            lines.append(f"  {status_sym} {style}")

            _, stage_desc = _HERO_MAP.get(
                next((k for k, v in _HERO_MAP.items() if v[0] == stage), "origin"),
                (stage, "")
            )

            if info["arc_count"] > 0:
                lines.append(f"    {info['arc_count']} arc(s) — avg significance {info['avg_significance']:.2f}")
                for title in info["arc_titles"]:
                    lines.append(f"      · {title}")
            else:
                lines.append("    (not yet begun)")
            lines.append("")

        if data["insights"]:
            lines.append("  ─── INSIGHTS ─────────────────────────────────────────")
            lines.append("")
            for insight in data["insights"]:
                lines.append(f"  ◈ {insight}")
            lines.append("")

        return "\n".join(lines)


# ── Insight derivation ─────────────────────────────────────────────────────────

def _derive_insights(
    stage_arcs: dict[str, list[dict]],
    season: str,
    nodes: list[dict],
) -> list[str]:
    insights: list[str] = []

    has_call          = bool(stage_arcs.get("THE CALL"))
    has_threshold     = bool(stage_arcs.get("THE THRESHOLD"))
    has_trials        = bool(stage_arcs.get("THE TRIALS"))
    has_transformation = bool(stage_arcs.get("TRANSFORMATION"))
    has_return        = bool(stage_arcs.get("THE RETURN"))

    ghost_count  = sum(1 for n in nodes if n.get("is_ghost"))
    fossil_count = sum(1 for n in nodes if n.get("is_fossil"))
    total_nodes  = max(len(nodes), 1)

    if has_call and not has_threshold:
        insights.append(
            "You have answered the call but not yet crossed the threshold. "
            "The real commitment has not been made."
        )

    if has_trials and not has_transformation:
        insights.append(
            "You are deep in the trials — friction and resistance are present. "
            "Transformation typically follows sustained conflict."
        )

    if has_transformation and not has_return:
        insights.append(
            "Transformation is underway. The return — integration and legacy — "
            "is the final stage awaiting completion."
        )

    if has_return:
        insights.append(
            "A complete arc has been traversed. What has been learned is now "
            "part of the foundation."
        )

    if fossil_count / total_nodes > 0.1:
        insights.append(
            f"{fossil_count} fossilized ideas form the bedrock of your universe — "
            "ancient beliefs that underpin all current work."
        )

    if ghost_count / total_nodes > 0.2:
        insights.append(
            "A significant portion of your universe exists as ghosts. "
            "The past is present but not yet integrated."
        )

    # Seasonal insight
    season_insights = {
        "Spring": "You are in a period of emergence — the journey accelerates with new creation.",
        "Summer": "Peak intensity — this is the heart of the trials or transformation.",
        "Autumn": "Reflection and harvest — the return is near or already begun.",
        "Winter": "Silence and incubation — the journey continues beneath the surface.",
    }
    if season in season_insights:
        insights.append(season_insights[season])

    return insights


# ── Roman numerals ─────────────────────────────────────────────────────────────

def _roman(n: int) -> str:
    vals = [
        (1000, "M"), (900, "CM"), (500, "D"), (400, "CD"),
        (100, "C"),  (90, "XC"), (50, "L"),  (40, "XL"),
        (10, "X"),   (9, "IX"),  (5, "V"),   (4, "IV"), (1, "I"),
    ]
    result = ""
    for value, numeral in vals:
        while n >= value:
            result += numeral
            n -= value
    return result


# ── Module-level shortcuts ────────────────────────────────────────────────────

def generate_chronicle(workspace_json: str) -> str:
    """Generate the full Personal Lore Chronicle from workspace JSON."""
    return PersonalChronicle().generate(workspace_json)


def heroes_journey_narrative(workspace_json: str) -> str:
    """Generate the Hero's Journey narrative from workspace JSON."""
    return HeroJourneyMapper().narrative(workspace_json)


def heroes_journey_analysis(workspace_json: str) -> str:
    """Return Hero's Journey analysis as JSON string."""
    return json.dumps(HeroJourneyMapper().analyze(workspace_json), indent=2)
