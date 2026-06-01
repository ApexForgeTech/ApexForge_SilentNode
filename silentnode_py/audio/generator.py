"""
Phase 9 — AudioStateMapper
Maps a SilentNode workspace snapshot to AudioParameters for the Rust synthesis engine.

The mapper analyses:
  - Cognitive season (Spring/Summer/Autumn/Winter)
  - Average entropy across the universe
  - Active civilization count and average density
  - Trail density (focus events in last 30 days)
  - Cognitive weight (unfinished things)
  - Node type distribution (research / creative / technical / personal)

All of these combine to produce a blended AtmosphereKind and its
derived AtmosphereParams, which the Rust AudioEngine applies.

No actual audio output — this module is analysis-only.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, asdict
from datetime import datetime, timezone, timedelta
from typing import Any


# ── Timestamp helpers ──────────────────────────────────────────────────────────

def _now() -> float:
    return datetime.now(tz=timezone.utc).timestamp()


def _ts(s: str) -> float:
    try:
        return datetime.fromisoformat(s.replace("Z", "+00:00")).timestamp()
    except Exception:
        return 0.0


# ── Atmosphere parameter tables ───────────────────────────────────────────────

# These mirror the Rust AtmosphereKind presets exactly so Python analysis
# can produce the same parameter values.
ATMOSPHERE_PRESETS: dict[str, dict[str, float]] = {
    "research":  dict(base_freq=60.0,  osc1=0.26, osc2=0.13, osc3=0.07,
                      lfo_rate=0.07,  lfo_depth=0.08,  pitch_rate=0.03, pitch_depth=0.001,
                      noise=0.18, reverb=0.72, volume=0.36, pulse_hz=0.0, pulse_depth=0.0),
    "creative":  dict(base_freq=216.0, osc1=0.22, osc2=0.14, osc3=0.09,
                      lfo_rate=0.30,  lfo_depth=0.13,  pitch_rate=0.15, pitch_depth=0.0045,
                      noise=0.05, reverb=0.55, volume=0.43, pulse_hz=0.0, pulse_depth=0.0),
    "technical": dict(base_freq=80.0,  osc1=0.28, osc2=0.14, osc3=0.05,
                      lfo_rate=0.05,  lfo_depth=0.03,  pitch_rate=0.02, pitch_depth=0.0005,
                      noise=0.10, reverb=0.22, volume=0.40, pulse_hz=2.0, pulse_depth=0.35),
    "personal":  dict(base_freq=220.0, osc1=0.18, osc2=0.09, osc3=0.04,
                      lfo_rate=0.12,  lfo_depth=0.07,  pitch_rate=0.08, pitch_depth=0.003,
                      noise=0.03, reverb=0.82, volume=0.28, pulse_hz=0.0, pulse_depth=0.0),
    "ghost":     dict(base_freq=85.0,  osc1=0.12, osc2=0.05, osc3=0.02,
                      lfo_rate=0.04,  lfo_depth=0.16,  pitch_rate=0.02, pitch_depth=0.006,
                      noise=0.14, reverb=0.92, volume=0.12, pulse_hz=0.0, pulse_depth=0.0),
    "void":      dict(base_freq=60.0,  osc1=0.0,  osc2=0.0,  osc3=0.0,
                      lfo_rate=0.1,   lfo_depth=0.0,   pitch_rate=0.05, pitch_depth=0.0,
                      noise=0.0,  reverb=0.0,  volume=0.0,  pulse_hz=0.0, pulse_depth=0.0),
    "crystal":   dict(base_freq=264.0, osc1=0.22, osc2=0.11, osc3=0.06,
                      lfo_rate=0.0,   lfo_depth=0.0,   pitch_rate=0.0,  pitch_depth=0.0,
                      noise=0.01, reverb=0.28, volume=0.35, pulse_hz=0.0, pulse_depth=0.0),
    "entropy":   dict(base_freq=220.0, osc1=0.20, osc2=0.17, osc3=0.13,
                      lfo_rate=0.80,  lfo_depth=0.26,  pitch_rate=0.60, pitch_depth=0.013,
                      noise=0.38, reverb=0.60, volume=0.32, pulse_hz=0.0, pulse_depth=0.0),
    "spring":    dict(base_freq=130.5, osc1=0.20, osc2=0.12, osc3=0.07,
                      lfo_rate=0.40,  lfo_depth=0.11,  pitch_rate=0.26, pitch_depth=0.006,
                      noise=0.09, reverb=0.44, volume=0.40, pulse_hz=0.0, pulse_depth=0.0),
    "summer":    dict(base_freq=165.0, osc1=0.24, osc2=0.14, osc3=0.09,
                      lfo_rate=0.20,  lfo_depth=0.06,  pitch_rate=0.05, pitch_depth=0.002,
                      noise=0.04, reverb=0.48, volume=0.52, pulse_hz=0.0, pulse_depth=0.0),
    "autumn":    dict(base_freq=110.0, osc1=0.20, osc2=0.10, osc3=0.05,
                      lfo_rate=0.15,  lfo_depth=0.13,  pitch_rate=0.08, pitch_depth=0.004,
                      noise=0.15, reverb=0.66, volume=0.33, pulse_hz=0.0, pulse_depth=0.0),
    "winter":    dict(base_freq=55.0,  osc1=0.14, osc2=0.05, osc3=0.02,
                      lfo_rate=0.02,  lfo_depth=0.22,  pitch_rate=0.01, pitch_depth=0.009,
                      noise=0.04, reverb=0.90, volume=0.18, pulse_hz=0.0, pulse_depth=0.0),
    "ambient":   dict(base_freq=60.0,  osc1=0.15, osc2=0.07, osc3=0.03,
                      lfo_rate=0.10,  lfo_depth=0.06,  pitch_rate=0.05, pitch_depth=0.002,
                      noise=0.08, reverb=0.50, volume=0.26, pulse_hz=0.0, pulse_depth=0.0),
}

ATMOSPHERE_DESCRIPTIONS: dict[str, str] = {
    "research":  "ambient hum + harmonic resonance, cavernous reverb",
    "creative":  "warm 432Hz tone, organic vibrato, expansive atmosphere",
    "technical": "rhythmic 80Hz pulse, 120 BPM gate, tight room",
    "personal":  "A3 intimate drone, heavy reverb, emotional warmth",
    "ghost":     "barely-present 85Hz, near-infinite reverb tail",
    "void":      "absolute silence — felt absence of sound",
    "crystal":   "pure C4 harmonics, no drift, crystalline clarity",
    "entropy":   "dissonant harmonics, turbulent fast LFO, fragmented",
    "spring":    "C3 ascending energy, lively breathing, forward motion",
    "summer":    "E3 full warm resonance, peak vitality, sustained",
    "autumn":    "A2 descending warmth, fading complexity",
    "winter":    "A1 sub-bass incubation, near-silent, infinite decay",
    "ambient":   "neutral unobtrusive baseline",
}


# ── Cognitive state extraction ────────────────────────────────────────────────

@dataclass
class CognitiveState:
    """Distilled workspace state used for audio mapping."""
    season:              str    # Spring/Summer/Autumn/Winter
    avg_entropy:         float  # 0–1
    ghost_ratio:         float  # fraction of nodes that are ghosts
    void_ratio:          float  # fraction of nodes that are voids
    crystal_count:       int    # crystallized civilizations
    civilization_count:  int
    avg_civ_density:     float  # internal edge density
    trail_density:       float  # focus events per node in last 30 days
    cognitive_weight:    float  # 0–100
    node_type_dist:      dict   # {type: count}
    deep_work_ratio:     float  # deep_work / total focus events
    creation_rate:       float  # new nodes per day (30-day window)


def _extract_state(ws: dict) -> CognitiveState:
    nodes:  list[dict] = ws.get("nodes", [])
    events: list[dict] = ws.get("focus_events", [])
    civs:   list[dict] = ws.get("civilizations", [])

    now = _now()
    n_total = max(len(nodes), 1)

    # Entropy + ghost/void ratios
    entropies   = [float(n.get("entropy", 0)) for n in nodes]
    avg_entropy = sum(entropies) / n_total
    ghost_ratio = sum(1 for n in nodes if n.get("is_ghost")) / n_total
    void_ratio  = sum(1 for n in nodes if n.get("is_void"))  / n_total

    # Crystallized civilizations (is_crystallized flag)
    crystal_count  = sum(1 for c in civs if c.get("is_crystallized"))
    avg_civ_density = (
        sum(float(c.get("internal_density", 0)) for c in civs) / len(civs)
        if civs else 0.0
    )

    # Trail density — focus events in last 30 days / node count
    cutoff = now - 30 * 86400
    recent = [e for e in events if _ts(e.get("timestamp", "")) >= cutoff]
    trail_density = len(recent) / n_total

    # Deep work ratio
    deep = sum(1 for e in events if e.get("depth") in ("deep_work", "edit"))
    deep_work_ratio = deep / max(len(events), 1)

    # Node type distribution
    type_dist: dict[str, int] = {}
    for n in nodes:
        t = str(n.get("node_type", n.get("type", "idea")))
        type_dist[t] = type_dist.get(t, 0) + 1

    # Creation rate: nodes created in last 30 days / 30
    cutoff_30 = now - 30 * 86400
    new_nodes = sum(1 for n in nodes if _ts(n.get("created_at", "")) >= cutoff_30)
    creation_rate = new_nodes / 30.0

    # Cognitive weight (incomplete + ghost + void nodes)
    incomplete = sum(
        1 for n in nodes
        if not n.get("is_fossil") and not n.get("is_void")
        and float(n.get("entropy", 0)) > 0.1
        and float(n.get("gravity", 1)) > 0.5
    )
    cognitive_weight = min(
        (incomplete * 1.5 + sum(1 for n in nodes if n.get("is_ghost")) * 0.5),
        100.0,
    )

    # Season
    season = ws.get("season", "")
    if not season:
        # Derive from creation_rate + entropy if not provided
        if creation_rate > 0.5 and avg_entropy < 0.4:
            season = "Spring"
        elif creation_rate > 0.3 and deep_work_ratio > 0.4:
            season = "Summer"
        elif avg_entropy > 0.5:
            season = "Autumn"
        else:
            season = "Winter"

    return CognitiveState(
        season=season,
        avg_entropy=round(avg_entropy, 4),
        ghost_ratio=round(ghost_ratio, 4),
        void_ratio=round(void_ratio, 4),
        crystal_count=crystal_count,
        civilization_count=len(civs),
        avg_civ_density=round(avg_civ_density, 4),
        trail_density=round(trail_density, 4),
        cognitive_weight=round(cognitive_weight, 2),
        node_type_dist=type_dist,
        deep_work_ratio=round(deep_work_ratio, 4),
        creation_rate=round(creation_rate, 4),
    )


# ── Atmosphere selection logic ────────────────────────────────────────────────

def _select_primary_atmosphere(state: CognitiveState) -> str:
    """
    Primary atmosphere is driven by cognitive season + entropy extremes.
    """
    # Ghost / void dominance overrides season
    if state.ghost_ratio > 0.40:
        return "ghost"
    if state.void_ratio > 0.30:
        return "void"
    # High entropy crisis
    if state.avg_entropy > 0.70:
        return "entropy"

    # Season mapping
    season_map = {
        "Spring": "spring",
        "Summer": "summer",
        "Autumn": "autumn",
        "Winter": "winter",
    }
    return season_map.get(state.season, "ambient")


def _select_secondary_atmosphere(state: CognitiveState) -> str | None:
    """
    Secondary atmosphere reflects the dominant cognitive activity type.
    Derived from node type distribution and behavioural signals.
    """
    types = state.node_type_dist
    total = max(sum(types.values()), 1)

    research_score  = types.get("research",  0) / total
    creative_score  = types.get("idea",      0) / total + types.get("memory",   0) / total
    technical_score = types.get("project",   0) / total + types.get("process",  0) / total
    personal_score  = types.get("person",    0) / total + types.get("journal",  0) / total

    # Crystallized civs → add crystal flavour
    if state.crystal_count >= 2 and state.avg_civ_density > 0.5:
        return "crystal"

    # Deep work sessions → technical
    if state.deep_work_ratio > 0.5 and technical_score > 0.2:
        return "technical"

    # High trail density in idea nodes → creative
    if state.trail_density > 2.0 and creative_score > 0.25:
        return "creative"

    # Research node majority
    if research_score > 0.30:
        return "research"

    # High personal/journal ratio
    if personal_score > 0.25:
        return "personal"

    return None


def _compute_blend(state: CognitiveState) -> float:
    """
    How much the secondary atmosphere contributes (0 = all primary, 1 = all secondary).
    Ranges from 0.0 to 0.45 — primary always dominates.
    """
    # More civilizations and higher density → stronger secondary signal
    civ_factor   = min(state.civilization_count / 8.0, 1.0) * 0.2
    trail_factor = min(state.trail_density / 4.0, 1.0) * 0.15
    deep_factor  = state.deep_work_ratio * 0.10
    return round(min(civ_factor + trail_factor + deep_factor, 0.45), 3)


def _blend_params(
    primary:   dict[str, float],
    secondary: dict[str, float] | None,
    blend:     float,
) -> dict[str, float]:
    if secondary is None or blend == 0.0:
        return dict(primary)
    result = {}
    for k in primary:
        a = primary[k]
        b = secondary.get(k, a)
        result[k] = round(a + (b - a) * blend, 5)
    return result


# ── Volume modifiers ──────────────────────────────────────────────────────────

def _apply_cognitive_weight_modifier(
    params: dict[str, float],
    weight: float,
) -> dict[str, float]:
    """
    Cognitive weight compresses the universe's sound — higher weight
    reduces clarity (more reverb, lower volume, denser noise).
    """
    w = weight / 100.0
    p = dict(params)
    p["volume"]  = round(p["volume"]  * (1.0 - w * 0.25), 4)
    p["reverb"]  = round(min(p["reverb"]  + w * 0.12, 0.95), 4)
    p["noise"]   = round(min(p["noise"]   + w * 0.08, 0.50), 4)
    return p


# ── Public API ────────────────────────────────────────────────────────────────

class AudioStateMapper:
    """
    Maps a workspace snapshot to audio parameters.

    Usage:
        mapper = AudioStateMapper()
        params_json = mapper.map_to_params(workspace_json_str)
    """

    def map_to_params(self, workspace_json: str) -> str:
        """
        Input:  JSON workspace snapshot
        Output: JSON AudioMapping {atmosphere, blend, params, derived_from, description}
        """
        try:
            ws    = json.loads(workspace_json)
            state = _extract_state(ws)

            primary_kind   = _select_primary_atmosphere(state)
            secondary_kind = _select_secondary_atmosphere(state)
            blend          = _compute_blend(state) if secondary_kind else 0.0

            primary_p   = ATMOSPHERE_PRESETS.get(primary_kind,   ATMOSPHERE_PRESETS["ambient"])
            secondary_p = ATMOSPHERE_PRESETS.get(secondary_kind, None) if secondary_kind else None

            blended = _blend_params(primary_p, secondary_p, blend)
            blended = _apply_cognitive_weight_modifier(blended, state.cognitive_weight)

            description = ATMOSPHERE_DESCRIPTIONS.get(primary_kind, "")
            if secondary_kind and blend > 0.15:
                desc2 = ATMOSPHERE_DESCRIPTIONS.get(secondary_kind, "")
                description = f"{description} + {desc2} ({blend:.0%} blend)"

            return json.dumps({
                "atmosphere":      primary_kind,
                "secondary":       secondary_kind,
                "blend":           blend,
                "description":     description,
                "derived_from": {
                    "season":             state.season,
                    "avg_entropy":        state.avg_entropy,
                    "ghost_ratio":        state.ghost_ratio,
                    "void_ratio":         state.void_ratio,
                    "crystal_count":      state.crystal_count,
                    "civilization_count": state.civilization_count,
                    "avg_civ_density":    state.avg_civ_density,
                    "trail_density":      state.trail_density,
                    "cognitive_weight":   state.cognitive_weight,
                    "deep_work_ratio":    state.deep_work_ratio,
                    "creation_rate_per_day": state.creation_rate,
                },
                "params": blended,
                "seasonal_audio": {
                    "Spring": "light ascending harmonic movement",
                    "Summer": "full warm sustained resonance",
                    "Autumn": "fading harmonic complexity, descending",
                    "Winter": "near-silence, occasional deep resonance, long decay",
                }.get(state.season, "ambient baseline"),
                "memory_atmosphere": _derive_memory_atmosphere(state),
            })
        except Exception as exc:
            return json.dumps({"error": str(exc)})

    def list_atmospheres(self) -> str:
        """Return JSON list of all available atmospheres with descriptions."""
        return json.dumps([
            {"name": k, "description": v}
            for k, v in ATMOSPHERE_DESCRIPTIONS.items()
        ])


def _derive_memory_atmosphere(state: CognitiveState) -> str:
    """
    Vision.md: different memory region types produce distinct atmospheres.
    Returns the name of the dominant memory atmosphere type.
    """
    types = state.node_type_dist
    total = max(sum(types.values()), 1)
    scores = {
        "research":     (types.get("research", 0) + types.get("lore", 0)) / total,
        "creative":     (types.get("idea", 0) + types.get("memory", 0)) / total,
        "technical":    (types.get("project", 0) + types.get("process", 0)) / total,
        "personal":     (types.get("person", 0) + types.get("journal", 0)) / total,
        "failure":      types.get("ghost", 0) / total,
    }
    best = max(scores, key=lambda k: scores[k])
    descriptions = {
        "research":  "ambient hum, cool blue-grey, slow deliberate particle motion",
        "creative":  "cinematic expansive audio, warm shifting colour, organic flow",
        "technical": "rhythmic mechanical texture, high-contrast, precise ordered motion",
        "personal":  "soft intimate soundscape, warm amber, gentle slow particles",
        "failure":   "dissonant low harmonic, desaturated, turbulent or still",
    }
    return descriptions.get(best, "ambient baseline")


# ── Seasonal modifiers (roadmap Phase 9.2 spec) ───────────────────────────────

SEASON_AUDIO_MODS: dict[str, dict[str, float]] = {
    "Spring": {"freq_mult": 1.15, "harmonic_boost": 0.10, "reverb_reduce": 0.08, "lfo_boost": 0.05},
    "Summer": {"freq_mult": 1.00, "harmonic_boost": 0.15, "reverb_reduce": 0.05, "lfo_boost": 0.00},
    "Autumn": {"freq_mult": 0.90, "harmonic_boost": -0.05, "reverb_reduce": -0.10, "lfo_boost": -0.02},
    "Winter": {"freq_mult": 0.75, "harmonic_boost": -0.15, "reverb_reduce": -0.20, "lfo_boost": -0.05},
}


class AudioStateMapper:
    """
    Maps a workspace snapshot to audio parameters.

    Two mapping modes:
    1. map_to_params()         — preset-based (atmosphere selection)
    2. map_to_parametric()     — continuous parametric (roadmap Phase 9.2 exact formulas)

    Usage:
        mapper = AudioStateMapper()
        params_json = mapper.map_to_params(workspace_json_str)
        parametric_json = mapper.map_to_parametric(workspace_json_str)
    """

    def map_to_params(self, workspace_json: str) -> str:
        """
        Input:  JSON workspace snapshot
        Output: JSON AudioMapping {atmosphere, blend, params, derived_from, description}
        """
        try:
            ws    = json.loads(workspace_json)
            state = _extract_state(ws)

            primary_kind   = _select_primary_atmosphere(state)
            secondary_kind = _select_secondary_atmosphere(state)
            blend          = _compute_blend(state) if secondary_kind else 0.0

            primary_p   = ATMOSPHERE_PRESETS.get(primary_kind,   ATMOSPHERE_PRESETS["ambient"])
            secondary_p = ATMOSPHERE_PRESETS.get(secondary_kind, None) if secondary_kind else None

            blended = _blend_params(primary_p, secondary_p, blend)
            blended = _apply_cognitive_weight_modifier(blended, state.cognitive_weight)

            description = ATMOSPHERE_DESCRIPTIONS.get(primary_kind, "")
            if secondary_kind and blend > 0.15:
                desc2 = ATMOSPHERE_DESCRIPTIONS.get(secondary_kind, "")
                description = f"{description} + {desc2} ({blend:.0%} blend)"

            return json.dumps({
                "atmosphere":      primary_kind,
                "secondary":       secondary_kind,
                "blend":           blend,
                "description":     description,
                "derived_from": {
                    "season":             state.season,
                    "avg_entropy":        state.avg_entropy,
                    "ghost_ratio":        state.ghost_ratio,
                    "void_ratio":         state.void_ratio,
                    "crystal_count":      state.crystal_count,
                    "civilization_count": state.civilization_count,
                    "avg_civ_density":    state.avg_civ_density,
                    "trail_density":      state.trail_density,
                    "cognitive_weight":   state.cognitive_weight,
                    "deep_work_ratio":    state.deep_work_ratio,
                    "creation_rate_per_day": state.creation_rate,
                },
                "params": blended,
                "seasonal_audio": {
                    "Spring": "light ascending harmonic movement",
                    "Summer": "full warm sustained resonance",
                    "Autumn": "fading harmonic complexity, descending",
                    "Winter": "near-silence, occasional deep resonance, long decay",
                }.get(state.season, "ambient baseline"),
                "memory_atmosphere": _derive_memory_atmosphere(state),
            })
        except Exception as exc:
            return json.dumps({"error": str(exc)})

    def map_to_parametric(self, workspace_json: str) -> str:
        """
        Roadmap Phase 9.2 exact parametric mapping:
        Maps continuous graph metrics directly to synthesis parameters.

        Formulas (from roadmap):
            base_frequency     = 220 + (1 - avg_entropy) * 220    # 220–440 Hz
            harmonic_complexity = min(trail_density * 2.0, 1.0)
            reverb_amount      = cognitive_weight / 100.0
            pulse_rate         = avg_velocity * 60.0              # BPM-like
            seasonal_modifier  = SEASON_AUDIO_MODS[season]

        Returns JSON {base_frequency, harmonic_complexity, reverb_amount,
                      pulse_rate, seasonal_modifier, derived_from}
        """
        try:
            ws    = json.loads(workspace_json)
            state = _extract_state(ws)
            nodes = ws.get("nodes", [])

            # ── Roadmap exact formulas ─────────────────────────────────────
            base_frequency = round(220.0 + (1.0 - state.avg_entropy) * 220.0, 2)

            harmonic_complexity = round(min(state.trail_density * 2.0, 1.0), 4)

            reverb_amount = round(state.cognitive_weight / 100.0, 4)

            # avg_velocity: mean node velocity from graph
            velocities = [float(n.get("velocity", 0.0)) for n in nodes if not n.get("is_ghost")]
            avg_velocity = sum(velocities) / max(len(velocities), 1)
            pulse_rate = round(min(avg_velocity * 60.0, 180.0), 2)  # cap at 180 BPM

            # Seasonal modifier
            season_mod = SEASON_AUDIO_MODS.get(state.season, SEASON_AUDIO_MODS["Summer"])

            # Apply seasonal modifier to base frequency
            base_frequency_with_season = round(base_frequency * season_mod["freq_mult"], 2)

            # Harmonic complexity boosted by season
            harmonic_final = round(
                _clamp(harmonic_complexity + season_mod["harmonic_boost"], 0.0, 1.0), 4
            )

            # Reverb with seasonal reduction
            reverb_final = round(
                _clamp(reverb_amount + season_mod["reverb_reduce"], 0.0, 1.0), 4
            )

            # LFO rate from trail density + season
            lfo_rate = round(
                _clamp(0.05 + state.trail_density * 0.25 + season_mod["lfo_boost"], 0.01, 0.8),
                4,
            )

            # Volume: inverse of cognitive weight (heavy universe = quieter to reflect compression)
            volume = round(_clamp(0.5 - reverb_amount * 0.2, 0.15, 0.6), 4)

            return json.dumps({
                "mapping_mode": "parametric",
                "base_frequency":      base_frequency_with_season,
                "base_frequency_raw":  base_frequency,
                "harmonic_complexity": harmonic_final,
                "reverb_amount":       reverb_final,
                "pulse_rate_bpm":      pulse_rate,
                "lfo_rate_hz":         lfo_rate,
                "volume":              volume,
                "seasonal_modifier":   season_mod,
                "derived_from": {
                    "avg_entropy":       state.avg_entropy,
                    "trail_density":     state.trail_density,
                    "cognitive_weight":  state.cognitive_weight,
                    "avg_velocity":      round(avg_velocity, 4),
                    "season":            state.season,
                },
                "description": (
                    f"Parametric: {base_frequency_with_season:.0f}Hz base, "
                    f"complexity={harmonic_final:.2f}, reverb={reverb_final:.2f}, "
                    f"pulse={pulse_rate:.0f}bpm — {state.season}"
                ),
            })
        except Exception as exc:
            return json.dumps({"error": str(exc)})

    def list_atmospheres(self) -> str:
        """Return JSON list of all available atmospheres with descriptions."""
        return json.dumps([
            {"name": k, "description": v}
            for k, v in ATMOSPHERE_DESCRIPTIONS.items()
        ])


def _clamp(v: float, lo: float, hi: float) -> float:
    return max(lo, min(hi, v))


def _derive_memory_atmosphere(state: CognitiveState) -> str:
    """
    Vision.md: different memory region types produce distinct atmospheres.
    Returns the name of the dominant memory atmosphere type.
    """
    types = state.node_type_dist
    total = max(sum(types.values()), 1)
    scores = {
        "research":     (types.get("research", 0) + types.get("lore", 0)) / total,
        "creative":     (types.get("idea", 0) + types.get("memory", 0)) / total,
        "technical":    (types.get("project", 0) + types.get("process", 0)) / total,
        "personal":     (types.get("person", 0) + types.get("journal", 0)) / total,
        "failure":      types.get("ghost", 0) / total,
    }
    best = max(scores, key=lambda k: scores[k])
    descriptions = {
        "research":  "ambient hum, cool blue-grey, slow deliberate particle motion",
        "creative":  "cinematic expansive audio, warm shifting colour, organic flow",
        "technical": "rhythmic mechanical texture, high-contrast, precise ordered motion",
        "personal":  "soft intimate soundscape, warm amber, gentle slow particles",
        "failure":   "dissonant low harmonic, desaturated, turbulent or still",
    }
    return descriptions.get(best, "ambient baseline")


# ── Module-level convenience functions ───────────────────────────────────────

def map_workspace_to_audio(workspace_json: str) -> str:
    """Preset-based mapping — module-level shorthand."""
    return AudioStateMapper().map_to_params(workspace_json)


def map_workspace_to_audio_parametric(workspace_json: str) -> str:
    """Parametric mapping (roadmap Phase 9.2 exact formulas) — module-level shorthand."""
    return AudioStateMapper().map_to_parametric(workspace_json)
