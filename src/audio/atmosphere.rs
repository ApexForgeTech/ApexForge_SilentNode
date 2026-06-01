/// Phase 9 — Atmosphere Kinds and Parameter Presets
///
/// Each atmosphere maps a cognitive/environmental state to a complete set of
/// synthesis parameters. All values are tuned for musical coherence and
/// psychological resonance with the state they represent.
///
/// Memory Atmospheres (vision.md):
///   Research, Creative, Technical, Personal, Ghost, Void, Crystal, HighEntropy
///
/// Cognitive Season Atmospheres (vision.md):
///   Spring, Summer, Autumn, Winter
use crate::audio::synth::AtmosphereTarget;

// ── AtmosphereKind ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AtmosphereKind {
    // ── Memory region atmospheres ──
    /// Ambient hum, cool harmonic resonance, depth and layered complexity.
    Research,
    /// Expansive, melodic, warm amber tones, organic breathing motion.
    Creative,
    /// Rhythmic, mechanical texture, precise and dense.
    Technical,
    /// Soft, intimate soundscape — warmth and emotional weight.
    Personal,
    /// Deep silence with occasional distant resonance, cavernous reverb.
    Ghost,
    /// Absolute silence — felt absence of sound.
    Void,
    /// Clear, sustained crystalline tones — pure harmonic series.
    Crystal,
    /// Dissonant, fragmented — turbulent cognitive overload.
    HighEntropy,

    // ── Cognitive season atmospheres ──
    /// Light, ascending harmonic movement — idea generation accelerating.
    Spring,
    /// Full, warm, sustained resonance — peak creative output.
    Summer,
    /// Fading harmonic complexity, descending — consolidation, reflection.
    Autumn,
    /// Near-silence, occasional deep resonance, long decay — incubation.
    Winter,

    /// Neutral ambient baseline.
    Ambient,
}

impl AtmosphereKind {
    /// Returns the complete synthesis parameter preset for this atmosphere.
    pub fn to_params(&self) -> AtmosphereTarget {
        match self {
            // ── Research ─────────────────────────────────────────────────────
            // 60 Hz fundamental — sub-bass drone felt more than heard.
            // Heavy Schroeder reverb creates cavernous, layered depth.
            // Very slow LFO (7-second cycle) for natural breathing.
            // Pink noise at 18% adds textural complexity without harshness.
            Self::Research => AtmosphereTarget {
                base_freq: 60.0,
                osc_amp1: 0.26,
                osc_amp2: 0.13,
                osc_amp3: 0.07,
                lfo_rate: 0.07,
                lfo_depth: 0.08,
                pitch_drift_rate: 0.03,
                pitch_drift_depth: 0.001,
                noise_mix: 0.18,
                reverb: 0.72,
                volume: 0.36,
                pulse_rate_hz: 0.0,
                pulse_depth: 0.0,
            },

            // ── Creative ─────────────────────────────────────────────────────
            // 216 Hz (A sub-3 — subharmonic of 432 Hz).
            // Gentle vibrato (0.3 Hz) creates organic warmth.
            // Moderate reverb — spacious but not overwhelming.
            // Minimal noise — clarity of harmonic content.
            Self::Creative => AtmosphereTarget {
                base_freq: 216.0,
                osc_amp1: 0.22,
                osc_amp2: 0.14,
                osc_amp3: 0.09,
                lfo_rate: 0.30,
                lfo_depth: 0.13,
                pitch_drift_rate: 0.15,
                pitch_drift_depth: 0.0045,
                noise_mix: 0.05,
                reverb: 0.55,
                volume: 0.43,
                pulse_rate_hz: 0.0,
                pulse_depth: 0.0,
            },

            // ── Technical ────────────────────────────────────────────────────
            // 80 Hz — punchy low fundamental.
            // 2 Hz pulse (120 BPM) with 35% depth: rhythmic, mechanical.
            // Tight room reverb (22% wet) — precise and close.
            // Minimal LFO — stability over motion.
            Self::Technical => AtmosphereTarget {
                base_freq: 80.0,
                osc_amp1: 0.28,
                osc_amp2: 0.14,
                osc_amp3: 0.05,
                lfo_rate: 0.05,
                lfo_depth: 0.03,
                pitch_drift_rate: 0.02,
                pitch_drift_depth: 0.0005,
                noise_mix: 0.10,
                reverb: 0.22,
                volume: 0.40,
                pulse_rate_hz: 2.0,
                pulse_depth: 0.35,
            },

            // ── Personal ─────────────────────────────────────────────────────
            // A3 (220 Hz) — warm, intimate register.
            // Heavy reverb (82%) with soft LFO — intimate but spacious.
            // Very low noise, very quiet — presence without intrusion.
            Self::Personal => AtmosphereTarget {
                base_freq: 220.0,
                osc_amp1: 0.18,
                osc_amp2: 0.09,
                osc_amp3: 0.04,
                lfo_rate: 0.12,
                lfo_depth: 0.07,
                pitch_drift_rate: 0.08,
                pitch_drift_depth: 0.003,
                noise_mix: 0.03,
                reverb: 0.82,
                volume: 0.28,
                pulse_rate_hz: 0.0,
                pulse_depth: 0.0,
            },

            // ── Ghost ────────────────────────────────────────────────────────
            // 85 Hz — barely present, like a memory.
            // 92% reverb — near-infinite tail, the sound of distance.
            // Slow deep LFO with significant depth (15%) — uncertain, wavering.
            // Volume 12% — more felt than heard.
            Self::Ghost => AtmosphereTarget {
                base_freq: 85.0,
                osc_amp1: 0.12,
                osc_amp2: 0.05,
                osc_amp3: 0.02,
                lfo_rate: 0.04,
                lfo_depth: 0.16,
                pitch_drift_rate: 0.02,
                pitch_drift_depth: 0.006,
                noise_mix: 0.14,
                reverb: 0.92,
                volume: 0.12,
                pulse_rate_hz: 0.0,
                pulse_depth: 0.0,
            },

            // ── Void ─────────────────────────────────────────────────────────
            // Absolute silence — all oscillators and noise at zero.
            // The felt absence of sound. Not muted — genuinely empty.
            Self::Void => AtmosphereTarget::silence(),

            // ── Crystal ──────────────────────────────────────────────────────
            // C4 just-intonation (264 Hz) — clear, mathematically pure.
            // No LFO, no pitch drift — stability as a defining property.
            // Near-zero noise, 28% reverb — clear without being cold.
            // Harmonics ring like crystal facets.
            Self::Crystal => AtmosphereTarget {
                base_freq: 264.0,
                osc_amp1: 0.22,
                osc_amp2: 0.11,
                osc_amp3: 0.06,
                lfo_rate: 0.0,
                lfo_depth: 0.0,
                pitch_drift_rate: 0.0,
                pitch_drift_depth: 0.0,
                noise_mix: 0.01,
                reverb: 0.28,
                volume: 0.35,
                pulse_rate_hz: 0.0,
                pulse_depth: 0.0,
            },

            // ── HighEntropy ──────────────────────────────────────────────────
            // A3 (220 Hz) with amplified 2nd and 3rd harmonics — dissonant.
            // Fast LFO (0.8 Hz) with deep amplitude swings — turbulent.
            // 38% noise and 60% reverb — dense, unstable, fragmented.
            Self::HighEntropy => AtmosphereTarget {
                base_freq: 220.0,
                osc_amp1: 0.20,
                osc_amp2: 0.17,
                osc_amp3: 0.13,
                lfo_rate: 0.80,
                lfo_depth: 0.26,
                pitch_drift_rate: 0.60,
                pitch_drift_depth: 0.013,
                noise_mix: 0.38,
                reverb: 0.60,
                volume: 0.32,
                pulse_rate_hz: 0.0,
                pulse_depth: 0.0,
            },

            // ── Spring ───────────────────────────────────────────────────────
            // C3 (130.5 Hz) — bright, ascending energy.
            // Fast-ish LFO (0.4 Hz) — lively, quick breathing.
            // Pitch drift adds forward momentum.
            Self::Spring => AtmosphereTarget {
                base_freq: 130.5,
                osc_amp1: 0.20,
                osc_amp2: 0.12,
                osc_amp3: 0.07,
                lfo_rate: 0.40,
                lfo_depth: 0.11,
                pitch_drift_rate: 0.26,
                pitch_drift_depth: 0.006,
                noise_mix: 0.09,
                reverb: 0.44,
                volume: 0.40,
                pulse_rate_hz: 0.0,
                pulse_depth: 0.0,
            },

            // ── Summer ───────────────────────────────────────────────────────
            // E3 (165 Hz) — warm, full, confident.
            // Gentle LFO — present but not restless.
            // Loudest season — peak presence and vitality.
            Self::Summer => AtmosphereTarget {
                base_freq: 165.0,
                osc_amp1: 0.24,
                osc_amp2: 0.14,
                osc_amp3: 0.09,
                lfo_rate: 0.20,
                lfo_depth: 0.06,
                pitch_drift_rate: 0.05,
                pitch_drift_depth: 0.002,
                noise_mix: 0.04,
                reverb: 0.48,
                volume: 0.52,
                pulse_rate_hz: 0.0,
                pulse_depth: 0.0,
            },

            // ── Autumn ───────────────────────────────────────────────────────
            // A2 (110 Hz) — deeper, slower, receding.
            // Moderate reverb and noise — warmth fading to dust.
            // Slower LFO than Spring — the world is winding down.
            Self::Autumn => AtmosphereTarget {
                base_freq: 110.0,
                osc_amp1: 0.20,
                osc_amp2: 0.10,
                osc_amp3: 0.05,
                lfo_rate: 0.15,
                lfo_depth: 0.13,
                pitch_drift_rate: 0.08,
                pitch_drift_depth: 0.004,
                noise_mix: 0.15,
                reverb: 0.66,
                volume: 0.33,
                pulse_rate_hz: 0.0,
                pulse_depth: 0.0,
            },

            // ── Winter ───────────────────────────────────────────────────────
            // A1 (55 Hz) — the deepest drone, sub-bass incubation.
            // Extremely slow LFO (50-second cycle) — deep breath.
            // 90% reverb — infinite decay, the sound of frozen time.
            // Volume 18% — an almost inaudible presence.
            Self::Winter => AtmosphereTarget {
                base_freq: 55.0,
                osc_amp1: 0.14,
                osc_amp2: 0.05,
                osc_amp3: 0.02,
                lfo_rate: 0.02,
                lfo_depth: 0.22,
                pitch_drift_rate: 0.01,
                pitch_drift_depth: 0.009,
                noise_mix: 0.04,
                reverb: 0.90,
                volume: 0.18,
                pulse_rate_hz: 0.0,
                pulse_depth: 0.0,
            },

            // ── Ambient ──────────────────────────────────────────────────────
            // Neutral, unobtrusive — suitable when state is undetermined.
            Self::Ambient => AtmosphereTarget {
                base_freq: 60.0,
                osc_amp1: 0.15,
                osc_amp2: 0.07,
                osc_amp3: 0.03,
                lfo_rate: 0.10,
                lfo_depth: 0.06,
                pitch_drift_rate: 0.05,
                pitch_drift_depth: 0.002,
                noise_mix: 0.08,
                reverb: 0.50,
                volume: 0.26,
                pulse_rate_hz: 0.0,
                pulse_depth: 0.0,
            },
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "research" | "r" => Self::Research,
            "creative" | "c" => Self::Creative,
            "technical" | "t" => Self::Technical,
            "personal" | "p" => Self::Personal,
            "ghost" | "g" => Self::Ghost,
            "void" | "v" => Self::Void,
            "crystal" => Self::Crystal,
            "entropy" | "e" => Self::HighEntropy,
            "spring" => Self::Spring,
            "summer" => Self::Summer,
            "autumn" | "fall" => Self::Autumn,
            "winter" => Self::Winter,
            _ => Self::Ambient,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Research => "research",
            Self::Creative => "creative",
            Self::Technical => "technical",
            Self::Personal => "personal",
            Self::Ghost => "ghost",
            Self::Void => "void",
            Self::Crystal => "crystal",
            Self::HighEntropy => "entropy",
            Self::Spring => "spring",
            Self::Summer => "summer",
            Self::Autumn => "autumn",
            Self::Winter => "winter",
            Self::Ambient => "ambient",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Research => "ambient hum + harmonic resonance, cavernous reverb",
            Self::Creative => "warm 432Hz tone, organic vibrato, expansive atmosphere",
            Self::Technical => "rhythmic 80Hz pulse, 120 BPM gate, tight room",
            Self::Personal => "A3 intimate drone, heavy reverb, emotional warmth",
            Self::Ghost => "barely-present 85Hz, near-infinite reverb tail",
            Self::Void => "absolute silence — felt absence of sound",
            Self::Crystal => "pure C4 harmonics, no drift, crystalline clarity",
            Self::HighEntropy => "dissonant harmonics, turbulent fast LFO, fragmented",
            Self::Spring => "C3 ascending energy, lively breathing, forward motion",
            Self::Summer => "E3 full warm resonance, peak vitality, sustained",
            Self::Autumn => "A2 descending warmth, fading complexity, dust",
            Self::Winter => "A1 sub-bass incubation, near-silent, infinite decay",
            Self::Ambient => "neutral unobtrusive baseline",
        }
    }

    /// All available atmosphere names for display / CLI help.
    pub fn all() -> &'static [&'static str] {
        &[
            "research",
            "creative",
            "technical",
            "personal",
            "ghost",
            "void",
            "crystal",
            "entropy",
            "spring",
            "summer",
            "autumn",
            "winter",
            "ambient",
        ]
    }
}

// ── Atmosphere derivation from workspace state ────────────────────────────────

/// Derive the appropriate AtmosphereKind from a cognitive season string.
pub fn atmosphere_from_season(season: &str) -> AtmosphereKind {
    match season.to_lowercase().as_str() {
        "spring" => AtmosphereKind::Spring,
        "summer" => AtmosphereKind::Summer,
        "autumn" => AtmosphereKind::Autumn,
        "winter" => AtmosphereKind::Winter,
        _ => AtmosphereKind::Ambient,
    }
}

/// Derive atmosphere from graph entropy level.
/// High entropy → dissonant; ghost-level entropy → ghost atmosphere.
pub fn atmosphere_from_entropy(entropy: f32) -> AtmosphereKind {
    if entropy >= 0.92 {
        AtmosphereKind::Ghost
    } else if entropy >= 0.60 {
        AtmosphereKind::HighEntropy
    } else {
        AtmosphereKind::Ambient
    }
}

/// Blend two atmospheres by weight (0.0 = all a, 1.0 = all b).
/// Returns a new AtmosphereTarget that interpolates both presets.
pub fn blend_atmospheres(a: &AtmosphereKind, b: &AtmosphereKind, blend: f32) -> AtmosphereTarget {
    let pa = a.to_params();
    let pb = b.to_params();
    pa.lerp_toward(&pb, blend.clamp(0.0, 1.0))
}
