/// Phase 9 — Core Audio Synthesis Engine
///
/// Oscillators, pink noise, LFO, Schroeder reverb, and the master
/// AudioSynthesizer that combines all voices into a single PCM sample stream.
/// Designed for real-time audio callback — zero allocation after construction.
use std::f32::consts::PI;

// ── Oscillator ────────────────────────────────────────────────────────────────

/// Single sine-wave voice.
pub struct Oscillator {
    pub frequency: f32,
    pub amplitude: f32,
    pub phase: f32,
    pub phase_step: f32,
    sample_rate: f32,
}

impl Oscillator {
    pub fn new(frequency: f32, amplitude: f32, sample_rate: f32) -> Self {
        Self {
            frequency,
            amplitude,
            phase: 0.0,
            phase_step: frequency / sample_rate,
            sample_rate,
        }
    }

    pub fn set_frequency(&mut self, freq: f32) {
        self.frequency = freq;
        self.phase_step = freq / self.sample_rate;
    }

    #[inline]
    pub fn next_sample(&mut self) -> f32 {
        let s = (self.phase * 2.0 * PI).sin() * self.amplitude;
        self.phase = (self.phase + self.phase_step).fract();
        s
    }
}

// ── Pink Noise Generator ──────────────────────────────────────────────────────

/// Paul Kellett's 6-filter pink noise approximation.
/// Pink noise (-3 dB/octave) sounds more natural than white noise for ambience.
pub struct PinkNoiseGen {
    b: [f32; 7],
    seed: u64,
}

impl PinkNoiseGen {
    pub fn new() -> Self {
        Self {
            b: [0.0; 7],
            seed: 0xDEAD_BEEF_1234_5678,
        }
    }

    #[inline]
    fn white(&mut self) -> f32 {
        self.seed = self
            .seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.seed >> 33) as i32) as f32 / i32::MAX as f32
    }

    #[inline]
    pub fn next_sample(&mut self) -> f32 {
        let w = self.white();
        self.b[0] = 0.998_860 * self.b[0] + w * 0.055_517_9;
        self.b[1] = 0.993_320 * self.b[1] + w * 0.075_075_9;
        self.b[2] = 0.969_000 * self.b[2] + w * 0.153_852_0;
        self.b[3] = 0.866_500 * self.b[3] + w * 0.310_485_6;
        self.b[4] = 0.550_000 * self.b[4] + w * 0.532_952_2;
        self.b[5] = -0.761_600 * self.b[5] + w * 0.016_898_0;
        let pink = self.b[0]
            + self.b[1]
            + self.b[2]
            + self.b[3]
            + self.b[4]
            + self.b[5]
            + self.b[6]
            + w * 0.536_2;
        self.b[6] = w * 0.115_926;
        pink * 0.11 // normalize to roughly ±1
    }
}

// ── LFO ──────────────────────────────────────────────────────────────────────

/// Low-frequency oscillator for amplitude breathing and pitch drift.
pub struct Lfo {
    phase: f32,
    phase_step: f32,
    pub depth: f32, // 0–1: output = 1.0 ± depth
}

impl Lfo {
    pub fn new(frequency: f32, depth: f32, sample_rate: f32) -> Self {
        Self {
            phase: 0.0,
            phase_step: frequency / sample_rate,
            depth,
        }
    }

    pub fn set_frequency(&mut self, freq: f32, sample_rate: f32) {
        self.phase_step = freq / sample_rate;
    }

    #[inline]
    pub fn next_value(&mut self) -> f32 {
        let v = (self.phase * 2.0 * PI).sin();
        self.phase = (self.phase + self.phase_step).fract();
        1.0 + v * self.depth
    }
}

// ── Schroeder Reverb ──────────────────────────────────────────────────────────

struct CombFilter {
    buffer: Vec<f32>,
    pos: usize,
    feedback: f32,
    damp1: f32,
    damp2: f32,
    filterstore: f32,
}

impl CombFilter {
    fn new(delay_samples: usize, feedback: f32, damp: f32) -> Self {
        Self {
            buffer: vec![0.0; delay_samples.max(1)],
            pos: 0,
            feedback,
            damp1: damp,
            damp2: 1.0 - damp,
            filterstore: 0.0,
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let output = self.buffer[self.pos];
        self.filterstore = output * self.damp2 + self.filterstore * self.damp1;
        self.buffer[self.pos] = input + self.filterstore * self.feedback;
        self.pos = (self.pos + 1) % self.buffer.len();
        output
    }
}

struct AllpassFilter {
    buffer: Vec<f32>,
    pos: usize,
    feedback: f32,
}

impl AllpassFilter {
    fn new(delay_samples: usize) -> Self {
        Self {
            buffer: vec![0.0; delay_samples.max(1)],
            pos: 0,
            feedback: 0.5,
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let buffered = self.buffer[self.pos];
        let output = -input + buffered;
        self.buffer[self.pos] = input + buffered * self.feedback;
        self.pos = (self.pos + 1) % self.buffer.len();
        output
    }
}

/// Classic Schroeder reverb: 4 comb + 2 allpass filters.
/// Produces a spacious, natural-sounding tail without coloration.
pub struct SchroederReverb {
    combs: [CombFilter; 4],
    allpasses: [AllpassFilter; 2],
    pub wet: f32,
    pub dry: f32,
}

impl SchroederReverb {
    pub fn new(sample_rate: f32, wet: f32) -> Self {
        // Delay times are tuned for 44100 Hz; scale proportionally.
        let s = |d: usize| ((d as f32 * sample_rate / 44100.0) as usize).max(1);
        Self {
            combs: [
                CombFilter::new(s(1_557), 0.84, 0.20),
                CombFilter::new(s(1_617), 0.84, 0.20),
                CombFilter::new(s(1_491), 0.84, 0.20),
                CombFilter::new(s(1_422), 0.84, 0.20),
            ],
            allpasses: [AllpassFilter::new(s(225)), AllpassFilter::new(s(556))],
            wet,
            dry: 1.0 - wet,
        }
    }

    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        let comb_sum = self.combs[0].process(input)
            + self.combs[1].process(input)
            + self.combs[2].process(input)
            + self.combs[3].process(input);
        let ap1 = self.allpasses[0].process(comb_sum * 0.25);
        let wet = self.allpasses[1].process(ap1);
        input * self.dry + wet * self.wet
    }
}

// ── AtmosphereTarget ──────────────────────────────────────────────────────────

/// All synthesis parameters for one atmosphere state.
/// Supports linear interpolation for smooth crossfades.
#[derive(Clone, Debug)]
pub struct AtmosphereTarget {
    pub base_freq: f32,
    pub osc_amp1: f32,          // fundamental
    pub osc_amp2: f32,          // 2nd harmonic
    pub osc_amp3: f32,          // 3rd harmonic
    pub lfo_rate: f32,          // Hz — amplitude breathing
    pub lfo_depth: f32,         // 0–1
    pub pitch_drift_rate: f32,  // Hz — slow pitch wander
    pub pitch_drift_depth: f32, // fraction of base_freq
    pub noise_mix: f32,         // 0–1 pink noise blend
    pub reverb: f32,            // 0–1 wet level
    pub volume: f32,            // 0–1 master volume
    pub pulse_rate_hz: f32,     // rhythmic pulse frequency (0 = off)
    pub pulse_depth: f32,       // 0–1 pulse amplitude depth
}

impl AtmosphereTarget {
    pub fn silence() -> Self {
        Self {
            base_freq: 60.0,
            osc_amp1: 0.0,
            osc_amp2: 0.0,
            osc_amp3: 0.0,
            lfo_rate: 0.1,
            lfo_depth: 0.0,
            pitch_drift_rate: 0.05,
            pitch_drift_depth: 0.0,
            noise_mix: 0.0,
            reverb: 0.0,
            volume: 0.0,
            pulse_rate_hz: 0.0,
            pulse_depth: 0.0,
        }
    }

    /// Per-parameter linear interpolation — used for smooth atmosphere transitions.
    pub fn lerp_toward(&self, target: &Self, t: f32) -> Self {
        let l = |a: f32, b: f32| a + (b - a) * t;
        Self {
            base_freq: l(self.base_freq, target.base_freq),
            osc_amp1: l(self.osc_amp1, target.osc_amp1),
            osc_amp2: l(self.osc_amp2, target.osc_amp2),
            osc_amp3: l(self.osc_amp3, target.osc_amp3),
            lfo_rate: l(self.lfo_rate, target.lfo_rate),
            lfo_depth: l(self.lfo_depth, target.lfo_depth),
            pitch_drift_rate: l(self.pitch_drift_rate, target.pitch_drift_rate),
            pitch_drift_depth: l(self.pitch_drift_depth, target.pitch_drift_depth),
            noise_mix: l(self.noise_mix, target.noise_mix),
            reverb: l(self.reverb, target.reverb),
            volume: l(self.volume, target.volume),
            pulse_rate_hz: l(self.pulse_rate_hz, target.pulse_rate_hz),
            pulse_depth: l(self.pulse_depth, target.pulse_depth),
        }
    }
}

// ── AudioSynthesizer ──────────────────────────────────────────────────────────

/// Master synthesizer: 3 oscillators + pink noise + LFOs + reverb + event voice.
/// All state is pre-allocated; safe to call from a real-time audio callback.
pub struct AudioSynthesizer {
    pub sample_rate: f32,

    // Drone voices
    pub osc1: Oscillator,
    pub osc2: Oscillator,
    pub osc3: Oscillator,

    // Modulation
    pub amp_lfo: Lfo,
    pub pitch_lfo: Lfo,

    // Texture
    pub noise: PinkNoiseGen,
    pub noise_mix: f32,

    // Spatial
    pub reverb: SchroederReverb,

    // Dynamics
    pub volume: f32,

    // Rhythmic pulse (for Technical atmosphere)
    pulse_phase: f32,
    pub pulse_rate_hz: f32,
    pub pulse_depth: f32,

    // One-shot event voice (crystallization, tectonic, etc.)
    pub event_osc: Oscillator,
    pub event_env: f32,   // 1.0 → 0.0 envelope
    pub event_decay: f32, // envelope decay per sample
}

impl AudioSynthesizer {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            osc1: Oscillator::new(60.0, 0.3, sample_rate),
            osc2: Oscillator::new(120.0, 0.15, sample_rate),
            osc3: Oscillator::new(180.0, 0.08, sample_rate),
            amp_lfo: Lfo::new(0.10, 0.10, sample_rate),
            pitch_lfo: Lfo::new(0.05, 0.003, sample_rate),
            noise: PinkNoiseGen::new(),
            noise_mix: 0.1,
            reverb: SchroederReverb::new(sample_rate, 0.5),
            volume: 0.4,
            pulse_phase: 0.0,
            pulse_rate_hz: 0.0,
            pulse_depth: 0.0,
            event_osc: Oscillator::new(528.0, 0.0, sample_rate),
            event_env: 0.0,
            event_decay: 0.0001,
        }
    }

    /// Apply an AtmosphereTarget instantly (used during smooth interpolation).
    #[inline]
    pub fn apply_params(&mut self, p: &AtmosphereTarget) {
        self.osc1.set_frequency(p.base_freq);
        self.osc2.set_frequency(p.base_freq * 2.0);
        self.osc3.set_frequency(p.base_freq * 3.0);
        self.osc1.amplitude = p.osc_amp1;
        self.osc2.amplitude = p.osc_amp2;
        self.osc3.amplitude = p.osc_amp3;
        self.amp_lfo
            .set_frequency(p.lfo_rate.max(0.001), self.sample_rate);
        self.amp_lfo.depth = p.lfo_depth;
        self.pitch_lfo
            .set_frequency(p.pitch_drift_rate.max(0.001), self.sample_rate);
        self.pitch_lfo.depth = p.pitch_drift_depth;
        self.noise_mix = p.noise_mix;
        self.reverb.wet = p.reverb;
        self.reverb.dry = 1.0 - p.reverb;
        self.volume = p.volume;
        self.pulse_rate_hz = p.pulse_rate_hz;
        self.pulse_depth = p.pulse_depth;
    }

    /// Trigger a one-shot event tone that decays over `decay_secs`.
    pub fn trigger_event(&mut self, freq: f32, amp: f32, decay_secs: f32) {
        self.event_osc.set_frequency(freq);
        self.event_osc.amplitude = amp;
        self.event_env = 1.0;
        self.event_decay = 1.0 / (decay_secs * self.sample_rate).max(1.0);
    }

    /// Produce the next mono PCM sample. Called from the real-time audio thread.
    #[inline]
    pub fn next_sample(&mut self) -> f32 {
        // — Amplitude breathing —
        let amp_mod = self.amp_lfo.next_value();

        // — Subtle pitch drift (applied to osc1 only) —
        let pitch_mod = self.pitch_lfo.next_value();
        let drifted_f1 = self.osc1.frequency * pitch_mod;
        self.osc1.phase_step = drifted_f1 / self.sample_rate;

        // — Drone voices —
        let drone = self.osc1.next_sample() + self.osc2.next_sample() + self.osc3.next_sample();

        // — Pink noise texture —
        let noise_out = self.noise.next_sample() * self.noise_mix;

        // — Rhythmic pulse modulation (Technical atmosphere) —
        let pulse_mod = if self.pulse_rate_hz > 0.0 {
            self.pulse_phase += self.pulse_rate_hz / self.sample_rate;
            if self.pulse_phase >= 1.0 {
                self.pulse_phase -= 1.0;
            }
            // Soft-attack trapezoid pulse
            let p = self.pulse_phase;
            let gate = if p < 0.08 {
                p / 0.08
            } else if p < 0.42 {
                1.0
            } else if p < 0.50 {
                (0.50 - p) / 0.08
            } else {
                0.0
            };
            1.0 - gate * self.pulse_depth
        } else {
            1.0
        };

        // — One-shot event envelope —
        let event_out = if self.event_env > 0.0 {
            let s = self.event_osc.next_sample() * self.event_env;
            // Smooth exponential-ish decay
            self.event_env = (self.event_env - self.event_decay).max(0.0);
            s
        } else {
            0.0
        };

        // — Mix, reverb, master volume —
        let dry = (drone + noise_out) * amp_mod * pulse_mod + event_out;
        let out = self.reverb.process(dry);
        (out * self.volume).clamp(-1.0, 1.0)
    }
}
