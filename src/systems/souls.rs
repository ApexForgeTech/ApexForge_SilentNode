use crate::domain::{NodeData, NodeType, TemporalSnapshot};
use chrono::Utc;
use uuid::Uuid;

/// Visual style for particles emitted by a Project node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleStyle {
    Aggressive,  // Fast, sharp, high-velocity
    Crystalline, // Slow, geometric, precise
    Fluid,       // Smooth, flowing, curved
    Organic,     // Random, biological, warm
}

/// Animated glow pattern around the project node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlowPattern {
    Pulse,   // Regular heartbeat pulse
    Radiate, // Continuous outward radiation
    Breathe, // Slow in/out like breathing
    Flicker, // Chaotic energy sparks
    Steady,  // Constant calm glow
}

/// Each Project/World node develops a unique visual personality over time.
#[derive(Debug, Clone)]
pub struct ProjectSoul {
    pub project_id: Uuid,
    pub primary_color: [f32; 4],
    pub secondary_color: [f32; 4],
    pub particle_style: ParticleStyle,
    pub glow_pattern: GlowPattern,
    /// 0–1 — how active the project currently is
    pub activity_level: f32,
    /// 0–1 — how old/mature the project is
    pub maturity: f32,
    /// 0–1 — how connected to other clusters
    pub social_density: f32,
}

impl ProjectSoul {
    /// Derive a soul from a node's content hash + history metrics.
    pub fn derive(
        node: &NodeData,
        snapshots: &[TemporalSnapshot],
        connection_count: usize,
        total_nodes: usize,
    ) -> Self {
        // Deterministic palette from content hash
        let hash = simple_hash(node.content.as_bytes());
        let hue1 = (hash & 0xFFFF) as f32 / 65535.0;
        let hue2 = ((hash >> 16) & 0xFFFF) as f32 / 65535.0;

        let primary = hsl_to_rgba(hue1, 0.7, 0.55);
        let secondary = hsl_to_rgba((hue1 + hue2 * 0.3 + 0.5) % 1.0, 0.6, 0.65);

        // Activity: recent snapshot activity
        let recent_changes = snapshots.iter().filter(|s| s.node_id == node.id).count();
        let snapshot_activity = (recent_changes as f32 / 20.0).clamp(0.0, 1.0);
        let access_activity = ((node.access_count as f32).ln_1p() / 6.0).clamp(0.0, 1.0);
        let recency_days =
            (Utc::now() - node.accessed_at).num_seconds().max(0) as f32 / 86_400.0;
        let recency_activity = (1.0 / (1.0 + recency_days / 7.0)).clamp(0.0, 1.0);
        let activity =
            (snapshot_activity * 0.55 + access_activity * 0.25 + recency_activity * 0.20)
                .clamp(0.0, 1.0);

        // Maturity: normalized entropy inverse (fossil = very mature)
        let maturity = if node.is_fossil {
            1.0
        } else {
            (1.0 - node.entropy).clamp(0.0, 1.0) * 0.8
        };

        let social = (connection_count as f32 / total_nodes.max(1) as f32 * 5.0).clamp(0.0, 1.0);

        // Particle style: driven by node_type + activity
        let particle_style = match node.node_type {
            NodeType::Project if activity > 0.6 => ParticleStyle::Aggressive,
            NodeType::Project => ParticleStyle::Fluid,
            NodeType::World => ParticleStyle::Organic,
            NodeType::Idea if maturity < 0.3 => ParticleStyle::Fluid,
            NodeType::Idea => ParticleStyle::Crystalline,
            _ => ParticleStyle::Organic,
        };

        // Glow: driven by activity + connection density
        let glow_pattern = if activity > 0.7 {
            GlowPattern::Flicker
        } else if social > 0.5 {
            GlowPattern::Radiate
        } else if activity > 0.3 {
            GlowPattern::Pulse
        } else if maturity > 0.7 {
            GlowPattern::Breathe
        } else {
            GlowPattern::Steady
        };

        Self {
            project_id: node.id,
            primary_color: primary,
            secondary_color: secondary,
            particle_style,
            glow_pattern,
            activity_level: activity,
            maturity,
            social_density: social,
        }
    }

    /// Pulse amplitude for shader [0,1].
    pub fn pulse_amplitude(&self) -> f32 {
        match self.glow_pattern {
            GlowPattern::Pulse => 0.4 + self.activity_level * 0.4,
            GlowPattern::Radiate => 0.6 + self.social_density * 0.3,
            GlowPattern::Breathe => 0.2,
            GlowPattern::Flicker => 0.7 + self.activity_level * 0.2,
            GlowPattern::Steady => 0.1,
        }
    }

    /// Pulse frequency Hz.
    pub fn pulse_frequency(&self) -> f32 {
        match self.glow_pattern {
            GlowPattern::Pulse => 0.8 + self.activity_level * 1.2,
            GlowPattern::Radiate => 0.3,
            GlowPattern::Breathe => 0.15,
            GlowPattern::Flicker => 4.0 + self.activity_level * 6.0,
            GlowPattern::Steady => 0.0,
        }
    }
}

/// Derive Project/World souls from a CognitiveGraph.
///
/// `degree_map` — maps node UUID to its connection count in the graph.
/// Pass `None` to fall back to degree=0 (social_density will be 0 for all).
pub fn derive_all_souls(
    nodes: impl Iterator<Item = NodeData>,
    snapshots: &[TemporalSnapshot],
    total_node_count: usize,
    degree_map: Option<&std::collections::HashMap<uuid::Uuid, usize>>,
) -> Vec<ProjectSoul> {
    nodes
        .filter(|n| matches!(n.node_type, NodeType::Project | NodeType::World))
        .map(|n| {
            let degree = degree_map.and_then(|m| m.get(&n.id)).copied().unwrap_or(0);
            ProjectSoul::derive(&n, snapshots, degree, total_node_count)
        })
        .collect()
}

// ─── helpers ─────────────────────────────────────────────────────────────────

fn simple_hash(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Minimal HSL→RGBA (sRGB approximation, L controls lightness).
fn hsl_to_rgba(h: f32, s: f32, l: f32) -> [f32; 4] {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let h6 = h * 6.0;
    let x = c * (1.0 - (h6 % 2.0 - 1.0).abs());
    let (r1, g1, b1) = if h6 < 1.0 {
        (c, x, 0.0)
    } else if h6 < 2.0 {
        (x, c, 0.0)
    } else if h6 < 3.0 {
        (0.0, c, x)
    } else if h6 < 4.0 {
        (0.0, x, c)
    } else if h6 < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    let m = l - c * 0.5;
    [r1 + m, g1 + m, b1 + m, 1.0]
}
