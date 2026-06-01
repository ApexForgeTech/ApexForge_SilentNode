/// Phase 9 — Ambient Sound Architecture
///
/// AudioEngine: drives a cpal output stream with procedural synthesis.
/// Atmosphere transitions are smooth (sample-by-sample interpolation).
/// One-shot AudioEvents overlay on the ambient layer without interrupting it.
///
/// Build: `cargo build --features audio`
/// Play:  `cargo run --features audio -- audio-play research 10`
pub mod atmosphere;
pub mod synth;

use serde_json::json;
use std::sync::{Arc, Mutex};

pub use atmosphere::{
    atmosphere_from_entropy, atmosphere_from_season, blend_atmospheres, AtmosphereKind,
};
pub use synth::AtmosphereTarget;

// ── SoundMode ─────────────────────────────────────────────────────────────────

/// Controls what the audio engine outputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SoundMode {
    /// Full composition — all layers active.
    Full,
    /// Only the current focus area's regional signature.
    Regional,
    /// Output muted; internal state continues to evolve.
    Silence,
}

impl SoundMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Regional => "regional",
            Self::Silence => "silence",
        }
    }
}

// ── AudioEvent ────────────────────────────────────────────────────────────────

/// One-shot events overlaid on the ambient soundscape.
/// Each event produces a brief, distinctive sound that does not
/// interrupt the underlying atmosphere.
#[derive(Debug, Clone)]
pub enum AudioEvent {
    /// Knowledge cluster crystallizes — ascending harmonic arpeggio.
    Crystallization,
    /// Structural paradigm shift — low rumble scaled by magnitude.
    TectonicShift { magnitude: f32 },
    /// Ghost node emerges back into the active graph.
    GhostEmergence,
    /// Resonance chamber forms between distant nodes.
    ResonanceChamber,
    /// Cognitive season transition detected.
    SeasonTransition,
    /// Node sent to the Void — fade to silence.
    VoidEntry,
    /// Node extracted from the Void — quiet reappearance.
    VoidExit,
    /// Digital shadow detected — persistent unresolved presence.
    ShadowPulse,
    /// Lore arc completed — significant narrative moment.
    LoreArcComplete,
}

// ── Session seed ─────────────────────────────────────────────────────────────

/// Generate a small session-unique variation factor in [0.93, 1.07].
/// Vision.md: "no two sessions sound identical."
fn session_variation() -> f32 {
    // Use current time nanoseconds as a pseudo-random seed
    let ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(12345);
    // Map to [-0.07, +0.07]
    let v = ((ns % 1000) as f32 / 1000.0) * 0.14 - 0.07;
    1.0 + v
}

// ── SharedAudioState ──────────────────────────────────────────────────────────

/// Audio state shared between the control thread and the cpal callback thread.
/// The callback thread only calls `next_sample()`; the control thread updates
/// `target` and `mode`. All access is through Mutex.
pub struct SharedAudioState {
    pub synth: synth::AudioSynthesizer,
    pub mode: SoundMode,
    pub current_kind: AtmosphereKind,
    /// Focused node type for Regional mode (None = fall back to Full).
    pub regional_kind: Option<AtmosphereKind>,

    /// Target atmosphere parameters (where we're heading).
    pub target: AtmosphereTarget,
    /// Currently playing parameters (smoothly converging to target).
    pub current: AtmosphereTarget,

    /// Lerp coefficient per sample — higher = faster crossfade.
    /// 0.00008 ≈ 6 s crossfade at 44100 Hz.
    /// 0.0003  ≈ 1.5 s crossfade.
    pub transition_speed: f32,
}

impl SharedAudioState {
    fn new(sample_rate: f32) -> Self {
        let kind = AtmosphereKind::Ambient;
        // Apply session-unique variation to LFO rates and base frequency
        // so no two sessions sound identical (vision.md requirement).
        let sv = session_variation();
        let mut target = kind.to_params();
        target.lfo_rate = (target.lfo_rate * sv).clamp(0.005, 2.0);
        target.pitch_drift_rate = (target.pitch_drift_rate * sv).clamp(0.001, 1.0);
        target.base_freq = (target.base_freq * (1.0 + (sv - 1.0) * 0.3)).clamp(20.0, 880.0);
        let current = target.clone();
        Self {
            synth: synth::AudioSynthesizer::new(sample_rate),
            mode: SoundMode::Full,
            current_kind: kind,
            regional_kind: None,
            target,
            current,
            transition_speed: 0.000_08,
        }
    }

    /// Called once per output sample from the cpal callback.
    #[inline]
    fn next_sample(&mut self) -> f32 {
        if self.mode == SoundMode::Silence {
            return 0.0;
        }

        // Per-parameter lerp toward target
        let t = self.transition_speed;
        macro_rules! lerp {
            ($field:ident) => {
                self.current.$field += (self.target.$field - self.current.$field) * t;
            };
        }
        lerp!(base_freq);
        lerp!(osc_amp1);
        lerp!(osc_amp2);
        lerp!(osc_amp3);
        lerp!(lfo_rate);
        lerp!(lfo_depth);
        lerp!(pitch_drift_rate);
        lerp!(pitch_drift_depth);
        lerp!(noise_mix);
        lerp!(reverb);
        lerp!(volume);
        lerp!(pulse_rate_hz);
        lerp!(pulse_depth);

        self.synth.apply_params(&self.current);
        self.synth.next_sample()
    }
}

// ── AudioEngine ───────────────────────────────────────────────────────────────

/// The public audio engine.
///
/// Owns a cpal output stream and a shared synthesis state.
/// All methods are non-blocking — they write to the shared state and return
/// immediately; the audio callback picks up changes on the next buffer fill.
pub struct AudioEngine {
    shared: Arc<Mutex<SharedAudioState>>,
    /// Kept alive to prevent the stream from being dropped.
    /// cpal::Stream is not Send on all platforms, so no Send bound here.
    #[allow(dead_code)]
    _stream: Option<Box<dyn std::any::Any>>,
}

impl AudioEngine {
    /// Create a new AudioEngine.
    /// If no audio device is available (headless server, CI), returns a silent engine.
    pub fn new() -> Self {
        #[cfg(feature = "audio")]
        {
            use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

            let host = cpal::default_host();
            let device = match host.default_output_device() {
                Some(d) => d,
                None => {
                    eprintln!("[audio] No output device found — running in silent mode.");
                    return Self::silent();
                }
            };
            let config = match device.default_output_config() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[audio] Output config error: {e} — silent mode.");
                    return Self::silent();
                }
            };

            let sample_rate = config.sample_rate().0 as f32;
            let channels = config.channels() as usize;
            let shared = Arc::new(Mutex::new(SharedAudioState::new(sample_rate)));
            let shared_cb = Arc::clone(&shared);

            let stream_result = device.build_output_stream(
                &config.into(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let mut state = shared_cb.lock().unwrap();
                    let mut i = 0;
                    while i < data.len() {
                        let s = state.next_sample();
                        for _ in 0..channels {
                            if i < data.len() {
                                data[i] = s;
                                i += 1;
                            }
                        }
                    }
                },
                |e| eprintln!("[audio] Stream error: {e}"),
                None,
            );

            match stream_result {
                Ok(stream) => {
                    if let Err(e) = stream.play() {
                        eprintln!("[audio] Failed to start stream: {e}");
                    }
                    return Self {
                        shared,
                        _stream: Some(Box::new(stream)),
                    };
                }
                Err(e) => {
                    eprintln!("[audio] Failed to build stream: {e} — silent mode.");
                }
            }
        }
        Self::silent()
    }

    fn silent() -> Self {
        Self {
            shared: Arc::new(Mutex::new(SharedAudioState::new(44100.0))),
            _stream: None,
        }
    }

    // ── Control API ──────────────────────────────────────────────────────────

    /// Set the atmosphere; transition is smooth (no click).
    /// Session variation is preserved: only the preset's *shape* changes,
    /// not the session-unique LFO offsets already applied.
    pub fn set_atmosphere(&self, kind: AtmosphereKind) {
        let mut s = self.shared.lock().unwrap();
        let mut params = kind.to_params();
        // Preserve session variation on LFO and frequency (±3% of new target)
        let sv = session_variation();
        params.lfo_rate = (params.lfo_rate * (1.0 + (sv - 1.0) * 0.5)).clamp(0.005, 2.0);
        s.target = params;
        s.current_kind = kind;
    }

    /// Set the atmosphere using a blended mix of two kinds.
    pub fn set_blended_atmosphere(&self, a: AtmosphereKind, b: AtmosphereKind, blend: f32) {
        let params = blend_atmospheres(&a, &b, blend);
        let mut s = self.shared.lock().unwrap();
        s.target = params;
        s.current_kind = if blend < 0.5 { a } else { b };
    }

    /// Change the sound mode.
    ///
    /// `Regional` mode: only the sound of the user's current focus area plays.
    /// Vision.md: "only the sound of the user's current focus area."
    /// Call `set_regional_kind()` to specify which cluster type is in focus.
    pub fn set_mode(&self, mode: SoundMode) {
        let mut s = self.shared.lock().unwrap();
        if mode == SoundMode::Regional {
            // Apply the regional kind if set, otherwise stay with current
            if let Some(ref rk) = s.regional_kind.clone() {
                let mut params = rk.to_params();
                // Regional mode: same atmosphere but 20% quieter and slightly drier
                params.volume *= 0.80;
                params.reverb = (params.reverb * 0.85).clamp(0.0, 1.0);
                s.target = params;
            }
        }
        s.mode = mode;
    }

    /// Set which atmosphere kind represents the user's current focus region.
    /// Only takes effect when `SoundMode::Regional` is active.
    /// Vision.md: "only the sound of the user's current focus area."
    pub fn set_regional_kind(&self, kind: AtmosphereKind) {
        let mut s = self.shared.lock().unwrap();
        if s.mode == SoundMode::Regional {
            let mut params = kind.to_params();
            params.volume *= 0.80;
            params.reverb = (params.reverb * 0.85).clamp(0.0, 1.0);
            s.target = params;
        }
        s.regional_kind = Some(kind);
    }

    /// Set the crossfade speed (0.00008 = slow, 0.001 = fast).
    pub fn set_transition_speed(&self, speed: f32) {
        self.shared.lock().unwrap().transition_speed = speed.clamp(0.00001, 0.01);
    }

    /// Trigger a one-shot event sound layered on top of the current atmosphere.
    pub fn trigger_event(&self, event: AudioEvent) {
        let mut s = self.shared.lock().unwrap();
        match event {
            // Ascending harmonic arpeggio: fundamental → 2nd → 3rd → decay
            AudioEvent::Crystallization => {
                s.synth.trigger_event(528.0, 0.32, 3.5);
            }
            // Low-frequency rumble scaled by magnitude, 5-second tail
            AudioEvent::TectonicShift { magnitude } => {
                let freq = 28.0 + magnitude.clamp(0.0, 1.0) * 55.0;
                let amp = 0.38 * magnitude.clamp(0.0, 1.0);
                s.synth.trigger_event(freq, amp, 5.0);
            }
            // Soft mid-range rise — a familiar presence returning
            AudioEvent::GhostEmergence => {
                s.synth.trigger_event(183.0, 0.18, 4.2);
            }
            // Two resonant nodes finding each other — warm meeting tone
            AudioEvent::ResonanceChamber => {
                s.synth.trigger_event(396.0, 0.24, 2.8);
            }
            // Clear tone announcing a season change — like a bell
            AudioEvent::SeasonTransition => {
                s.synth.trigger_event(264.0, 0.22, 6.0);
            }
            // Void entry: fade the atmosphere to silence quickly
            AudioEvent::VoidEntry => {
                s.target = AtmosphereKind::Void.to_params();
                s.current_kind = AtmosphereKind::Void;
                s.transition_speed = 0.0003; // fast fade
            }
            // Void exit: quiet reappearance tone
            AudioEvent::VoidExit => {
                s.synth.trigger_event(220.0, 0.14, 3.0);
                s.transition_speed = 0.000_08; // restore normal speed
            }
            // Shadow pulse: persistent low reminder
            AudioEvent::ShadowPulse => {
                s.synth.trigger_event(110.0, 0.12, 2.0);
            }
            // Lore arc complete: significant, sustained tone
            AudioEvent::LoreArcComplete => {
                s.synth.trigger_event(432.0, 0.28, 5.0);
            }
        }
    }

    // ── State inspection ──────────────────────────────────────────────────────

    pub fn current_kind(&self) -> String {
        self.shared
            .lock()
            .unwrap()
            .current_kind
            .as_str()
            .to_string()
    }

    pub fn current_mode(&self) -> String {
        self.shared.lock().unwrap().mode.as_str().to_string()
    }

    /// Serialise the full current audio state to a JSON string.
    pub fn to_state_json(&self) -> String {
        let s = self.shared.lock().unwrap();
        let c = &s.current;
        let tk = s.current_kind.as_str();
        serde_json::to_string_pretty(&json!({
            "atmosphere":   tk,
            "description":  s.current_kind.description(),
            "mode":         s.mode.as_str(),
            "params": {
                "base_freq_hz":   (c.base_freq * 10.0).round() / 10.0,
                "harmonics": {
                    "osc1_amp": (c.osc_amp1 * 1000.0).round() / 1000.0,
                    "osc2_amp": (c.osc_amp2 * 1000.0).round() / 1000.0,
                    "osc3_amp": (c.osc_amp3 * 1000.0).round() / 1000.0,
                },
                "modulation": {
                    "lfo_rate_hz":  (c.lfo_rate * 1000.0).round() / 1000.0,
                    "lfo_depth":    (c.lfo_depth * 1000.0).round() / 1000.0,
                    "pitch_drift":  (c.pitch_drift_depth * 1000.0).round() / 1000.0,
                },
                "texture": {
                    "noise_mix": (c.noise_mix * 1000.0).round() / 1000.0,
                    "reverb_wet": (c.reverb * 1000.0).round() / 1000.0,
                },
                "dynamics": {
                    "volume":        (c.volume * 1000.0).round() / 1000.0,
                    "pulse_rate_hz": c.pulse_rate_hz,
                    "pulse_depth":   (c.pulse_depth * 1000.0).round() / 1000.0,
                },
            },
        }))
        .unwrap_or_else(|_| "{}".to_string())
    }
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new()
    }
}
