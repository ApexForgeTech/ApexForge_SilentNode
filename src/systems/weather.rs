use crate::domain::{FocusDepth, FocusEvent, NodeData};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum WeatherState {
    /// High creation rate + exploration: new connections forming rapidly.
    Energetic { pulse_rate: f32, expansion: f32 },
    /// Deep focus + high productivity: flow state.
    Calm { clarity: f32, stillness: f32 },
    /// High entropy + ghost accumulation: cognitive exhaustion / decay.
    Fading { dim_factor: f32, decay_speed: f32 },
    /// Revisiting dormant nodes: memory, retrospection.
    Reflective { ghost_visibility: f32, warmth: f32 },
    /// Mixed conflicting signals: transition, chaos.
    Turbulent {
        intensity: f32,
        chaos_frequency: f32,
    },
}

impl WeatherState {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Energetic { .. } => "Energetic",
            Self::Calm { .. } => "Calm",
            Self::Fading { .. } => "Fading",
            Self::Reflective { .. } => "Reflective",
            Self::Turbulent { .. } => "Turbulent",
        }
    }

    /// Linear-space sky background color (RGBA).
    pub fn primary_color(&self) -> [f32; 4] {
        match self {
            Self::Energetic { .. } => [0.04, 0.06, 0.16, 1.0],
            Self::Calm { .. } => [0.03, 0.05, 0.10, 1.0],
            Self::Fading { .. } => [0.06, 0.03, 0.08, 1.0],
            Self::Reflective { .. } => [0.04, 0.05, 0.09, 1.0],
            Self::Turbulent { .. } => [0.08, 0.03, 0.04, 1.0],
        }
    }

    /// Accent / glow color.
    pub fn secondary_color(&self) -> [f32; 4] {
        match self {
            Self::Energetic { .. } => [0.25, 0.65, 1.00, 1.0],
            Self::Calm { .. } => [0.35, 0.90, 0.55, 1.0],
            Self::Fading { .. } => [0.40, 0.28, 0.50, 1.0],
            Self::Reflective { .. } => [0.70, 0.50, 0.30, 1.0],
            Self::Turbulent { .. } => [1.00, 0.35, 0.18, 1.0],
        }
    }

    pub fn turbulence(&self) -> f32 {
        match self {
            Self::Turbulent { intensity, .. } => *intensity,
            Self::Energetic { expansion, .. } => expansion * 0.4,
            Self::Fading { decay_speed, .. } => decay_speed * 0.3,
            Self::Calm { .. } => 0.04,
            Self::Reflective { .. } => 0.08,
        }
    }

    pub fn intensity(&self) -> f32 {
        match self {
            Self::Energetic { pulse_rate, .. } => 0.45 + pulse_rate * 0.40,
            Self::Calm { clarity, .. } => 0.28 + clarity * 0.28,
            Self::Fading { dim_factor, .. } => 0.12 + (1.0 - dim_factor) * 0.18,
            Self::Reflective { warmth, .. } => 0.22 + warmth * 0.18,
            Self::Turbulent { intensity, .. } => 0.38 + intensity * 0.48,
        }
    }

    pub fn pulse_rate(&self) -> f32 {
        match self {
            Self::Energetic { pulse_rate, .. } => *pulse_rate,
            Self::Turbulent {
                chaos_frequency, ..
            } => *chaos_frequency,
            Self::Calm { .. } => 0.18,
            Self::Reflective { .. } => 0.28,
            Self::Fading { .. } => 0.08,
        }
    }

    pub fn particle_speed(&self) -> f32 {
        match self {
            Self::Energetic { expansion, .. } => 1.4 + expansion,
            Self::Turbulent { intensity, .. } => 1.1 + intensity * 0.8,
            Self::Calm { .. } => 0.55,
            Self::Reflective { .. } => 0.65,
            Self::Fading { decay_speed, .. } => 0.25 + decay_speed * 0.35,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WeatherSystem {
    pub current: WeatherState,
    pub previous: WeatherState,
    /// 0.0 = just transitioned, 1.0 = fully settled into current
    pub transition_progress: f32,
    transition_speed: f32,
}

impl Default for WeatherSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl WeatherSystem {
    pub fn new() -> Self {
        let initial = WeatherState::Calm {
            clarity: 0.6,
            stillness: 0.75,
        };
        Self {
            previous: initial.clone(),
            current: initial,
            transition_progress: 1.0,
            transition_speed: 0.25,
        }
    }

    /// Re-derive weather from current workspace metrics.
    pub fn derive(&mut self, nodes: &[&NodeData], focus_events: &[FocusEvent], now: DateTime<Utc>) {
        let new_state = compute_weather(nodes, focus_events, now);
        if new_state.name() != self.current.name() {
            self.previous = self.current.clone();
            self.current = new_state;
            self.transition_progress = 0.0;
        }
    }

    /// Advance blend animation (call each frame with delta_time in seconds).
    pub fn tick(&mut self, dt: f32) {
        if self.transition_progress < 1.0 {
            self.transition_progress =
                (self.transition_progress + dt * self.transition_speed).min(1.0);
        }
    }

    pub fn blended_primary(&self) -> [f32; 4] {
        lerp4(
            self.previous.primary_color(),
            self.current.primary_color(),
            self.transition_progress,
        )
    }
    pub fn blended_secondary(&self) -> [f32; 4] {
        lerp4(
            self.previous.secondary_color(),
            self.current.secondary_color(),
            self.transition_progress,
        )
    }
    pub fn blended_intensity(&self) -> f32 {
        lerp1(
            self.previous.intensity(),
            self.current.intensity(),
            self.transition_progress,
        )
    }
    pub fn blended_turbulence(&self) -> f32 {
        lerp1(
            self.previous.turbulence(),
            self.current.turbulence(),
            self.transition_progress,
        )
    }
    pub fn blended_pulse_rate(&self) -> f32 {
        lerp1(
            self.previous.pulse_rate(),
            self.current.pulse_rate(),
            self.transition_progress,
        )
    }
    pub fn blended_particle_speed(&self) -> f32 {
        lerp1(
            self.previous.particle_speed(),
            self.current.particle_speed(),
            self.transition_progress,
        )
    }
}

fn compute_weather(
    nodes: &[&NodeData],
    focus_events: &[FocusEvent],
    now: DateTime<Utc>,
) -> WeatherState {
    if nodes.is_empty() {
        return WeatherState::Calm {
            clarity: 0.8,
            stillness: 0.9,
        };
    }

    let total = nodes.len() as f32;
    let avg_entropy: f32 = nodes.iter().map(|n| n.entropy).sum::<f32>() / total;
    let ghost_ratio: f32 = nodes.iter().filter(|n| n.is_ghost).count() as f32 / total;

    let recent: Vec<&FocusEvent> = focus_events
        .iter()
        .filter(|e| e.timestamp > now - Duration::hours(24))
        .collect();

    let trail_density = (recent.len() as f32 / 20.0).clamp(0.0, 1.0);
    let unique: HashSet<Uuid> = recent.iter().map(|e| e.node_id).collect();
    let exploration = (unique.len() as f32 / total).clamp(0.0, 1.0);
    let deep_ratio = recent
        .iter()
        .filter(|e| e.depth == FocusDepth::DeepWork)
        .count() as f32
        / recent.len().max(1) as f32;

    if avg_entropy > 0.6 && ghost_ratio > 0.25 {
        return WeatherState::Fading {
            dim_factor: avg_entropy,
            decay_speed: ghost_ratio,
        };
    }
    if trail_density > 0.65 && exploration > 0.45 {
        return WeatherState::Energetic {
            pulse_rate: trail_density,
            expansion: exploration,
        };
    }
    if trail_density > 0.4 && deep_ratio > 0.45 {
        return WeatherState::Calm {
            clarity: deep_ratio,
            stillness: 1.0 - exploration,
        };
    }
    if trail_density < 0.2 && avg_entropy > 0.28 {
        return WeatherState::Reflective {
            ghost_visibility: ghost_ratio,
            warmth: 0.5,
        };
    }
    WeatherState::Turbulent {
        intensity: (avg_entropy + trail_density) * 0.5,
        chaos_frequency: exploration * 2.0,
    }
}

fn lerp1(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
fn lerp4(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        lerp1(a[0], b[0], t),
        lerp1(a[1], b[1], t),
        lerp1(a[2], b[2], t),
        lerp1(a[3], b[3], t),
    ]
}
