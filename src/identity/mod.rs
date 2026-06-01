/// Phase 10 — Identity & Narrative Systems
///
/// The Identity Engine tracks the evolving cognitive fingerprint of a user.
/// The Living Signature is a continuously evolving visual symbol — unique to
/// each user — that represents the complete accumulated pattern of their
/// cognitive existence within SilentNode.
///
/// Vision.md:
///   "No two signatures are alike. No signature is ever finished."
///   "Month by month the signature shifts imperceptibly.
///    Year by year the transformation becomes visible."
pub mod shadow_projects;

pub use shadow_projects::{ShadowOrigin, ShadowProject, ShadowProjectDetector};

use crate::domain::{ArcType, LoreEntry, NodeData};
use crate::systems::{CognitiveSeason, KnowledgeCrystal, Ritual};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Geometry / Symmetry / Motion kinds ───────────────────────────────────────

/// The base geometric form of the Living Signature.
/// Derived from the distribution of Lore Arc types over the user's history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeometryKind {
    /// Stable, cyclical, foundational — Origin + Legacy arcs dominant.
    Circle,
    /// Oscillatory tension and release — Conflict + Resolution arcs dominant.
    Wave,
    /// Continuous outward growth — Transformation + Revelation arcs dominant.
    Spiral,
    /// Complex self-similar structure — Tectonic arcs dominant.
    Fractal,
    /// Sequential progression — mixed or insufficient arcs.
    Line,
}

impl GeometryKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Circle => "circle",
            Self::Wave => "wave",
            Self::Spiral => "spiral",
            Self::Fractal => "fractal",
            Self::Line => "line",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Circle => "Cyclic and foundational — you return to and build on core truths",
            Self::Wave => "Oscillatory — you move through cycles of tension and resolution",
            Self::Spiral => "Expanding outward — each revolution is a higher turn of the same path",
            Self::Fractal => "Self-similar complexity — major shifts echo at every scale",
            Self::Line => "Sequential — you move forward, leaving what came before behind",
        }
    }
}

/// The symmetry pattern of the Living Signature.
/// Derived from the structure and step-count of detected behavioral rituals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymmetryKind {
    /// No recurring patterns detected.
    None,
    /// Two-part rituals — mirrored halves.
    Bilateral,
    /// Three-point rituals — triangular balance.
    Radial3,
    /// Four-part rituals — quadrant structure.
    Radial4,
    /// Five or more — complex radial pattern.
    Rotational,
}

impl SymmetryKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Bilateral => "bilateral",
            Self::Radial3 => "radial-3",
            Self::Radial4 => "radial-4",
            Self::Rotational => "rotational",
        }
    }

    pub fn fold_count(&self) -> usize {
        match self {
            Self::None => 1,
            Self::Bilateral => 2,
            Self::Radial3 => 3,
            Self::Radial4 => 4,
            Self::Rotational => 6,
        }
    }
}

/// The animation character of the Living Signature.
/// Derived from the current Cognitive Season and historical season distribution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MotionKind {
    /// Spring — expanding outward, forward momentum.
    Flow,
    /// Summer — strong, regular heartbeat pulse.
    Pulse,
    /// Autumn — slow breathing in and out.
    Breathe,
    /// Winter — still, precise, crystalline.
    Crystallize,
    /// Balanced — no dominant season, gentle drift.
    Drift,
}

impl MotionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Flow => "flow",
            Self::Pulse => "pulse",
            Self::Breathe => "breathe",
            Self::Crystallize => "crystallize",
            Self::Drift => "drift",
        }
    }

    pub fn animation_duration_ms(&self) -> u32 {
        match self {
            Self::Flow => 3_000,
            Self::Pulse => 1_500,
            Self::Breathe => 6_000,
            Self::Crystallize => 12_000,
            Self::Drift => 8_000,
        }
    }
}

// ── LivingSignature ───────────────────────────────────────────────────────────

/// The Living Signature — the complete visual identity of a SilentNode user.
///
/// Vision.md:
///   "The Living Signature is a continuously evolving visual symbol — unique to
///    each user — that represents the complete accumulated pattern of their
///    cognitive existence within SilentNode."
///
/// Derived from:
///   - the total shape of the universe over time
///   - the dominant Civilizations that have formed
///   - the Cognitive Seasons experienced
///   - the Lore Arcs completed
///   - the Knowledge Crystals formed
///   - the artifacts created in The Forge
///   - the rituals maintained
///   - the Shadows carried
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivingSignature {
    pub id: Uuid,
    pub computed_at: DateTime<Utc>,

    // ── Visual identity ──────────────────────────────────────────────────────
    /// Primary color — from dominant civilization palette.
    pub primary_color: [f32; 4],
    /// Secondary color — from second civilization or complementary hue.
    pub secondary_color: [f32; 4],
    /// Accent color — from Knowledge Crystals or high-entropy nodes.
    pub accent_color: [f32; 4],

    // ── Form ─────────────────────────────────────────────────────────────────
    pub geometry: GeometryKind,
    pub symmetry: SymmetryKind,
    pub motion: MotionKind,

    // ── Complexity ───────────────────────────────────────────────────────────
    /// 0–1 — how intricate the signature is (driven by civilization count).
    pub complexity: f32,
    /// 0–1 — how saturated/vivid (driven by recent activity level).
    pub vitality: f32,
    /// 0–1 — how much historical depth (driven by fossil + lore count).
    pub depth: f32,

    // ── Evolution ────────────────────────────────────────────────────────────
    /// How many times this signature has been recomputed.
    pub evolution_count: u64,
    /// When did the last significant form-change occur?
    pub last_major_shift_at: Option<DateTime<Utc>>,
    /// Human-readable description of the current signature.
    pub description: String,
}

impl LivingSignature {
    pub fn summary(&self) -> String {
        format!(
            "Living Signature v{} | {} {} {} | complexity={:.2} vitality={:.2} | {}",
            self.evolution_count,
            self.geometry.as_str(),
            self.symmetry.as_str(),
            self.motion.as_str(),
            self.complexity,
            self.vitality,
            self.description
        )
    }

    /// Has this signature shifted significantly relative to a previous version?
    pub fn has_shifted_from(&self, prev: &LivingSignature) -> bool {
        self.geometry != prev.geometry
            || self.symmetry != prev.symmetry
            || self.motion != prev.motion
            || (self.complexity - prev.complexity).abs() > 0.15
            || (self.vitality - prev.vitality).abs() > 0.20
    }
}

// ── IdentityEngine ────────────────────────────────────────────────────────────

/// The Identity Engine derives and tracks the user's Living Signature.
///
/// Vision.md: "SilentNode evolves with the individual. Over time the system
/// becomes psychologically unique, visually unique, behaviorally unique,
/// cognitively unique."
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IdentityEngine {
    pub current_signature: Option<LivingSignature>,
    /// History of signature shifts (kept small — only major shifts stored).
    pub shift_history: Vec<SignatureShift>,
}

/// A recorded moment where the Living Signature changed significantly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureShift {
    pub at: DateTime<Utc>,
    pub from_geometry: GeometryKind,
    pub to_geometry: GeometryKind,
    pub magnitude: f32,
    pub description: String,
}

impl IdentityEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Derive a new Living Signature from the current workspace state.
    /// Returns a reference to the newly computed signature.
    pub fn derive(
        &mut self,
        nodes: &[&NodeData],
        rituals: &[Ritual],
        lore: &[LoreEntry],
        crystals: &[KnowledgeCrystal],
        civs_colors: &[[f32; 4]],
        season: CognitiveSeason,
        evolution_count: u64,
    ) -> &LivingSignature {
        let now = Utc::now();

        // ── Colors ───────────────────────────────────────────────────────────
        let (primary, secondary, accent) = derive_colors(nodes, civs_colors, crystals);

        // ── Geometry — from lore arc distribution ────────────────────────────
        let geometry = derive_geometry(lore);

        // ── Symmetry — from ritual step patterns ─────────────────────────────
        let symmetry = derive_symmetry(rituals);

        // ── Motion — from cognitive season ───────────────────────────────────
        let motion = derive_motion(season);

        // ── Complexity = civilization count / 10 ─────────────────────────────
        let complexity = (civs_colors.len() as f32 / 10.0).clamp(0.0, 1.0);

        // ── Vitality = active nodes / total nodes ─────────────────────────────
        let active = nodes
            .iter()
            .filter(|n| !n.is_ghost && !n.is_fossil && !n.is_void)
            .count();
        let vitality = (active as f32 / nodes.len().max(1) as f32).clamp(0.0, 1.0);

        // ── Depth = (fossils + lore entries) / 20 ────────────────────────────
        let fossils = nodes.iter().filter(|n| n.is_fossil).count();
        let depth = ((fossils + lore.len()) as f32 / 20.0).clamp(0.0, 1.0);

        let description = format!(
            "{} {} {} — {} civs, {} lore arcs, {} crystals",
            geometry.description(),
            symmetry.as_str(),
            motion.as_str(),
            civs_colors.len(),
            lore.len(),
            crystals.len(),
        );

        // ── Check for major shift ────────────────────────────────────────────
        let last_major_shift_at = if let Some(prev) = &self.current_signature {
            let new_sig = LivingSignature {
                id: Uuid::new_v4(),
                computed_at: now,
                primary_color: primary,
                secondary_color: secondary,
                accent_color: accent,
                geometry: geometry.clone(),
                symmetry: symmetry.clone(),
                motion: motion.clone(),
                complexity,
                vitality,
                depth,
                evolution_count,
                last_major_shift_at: prev.last_major_shift_at,
                description: description.clone(),
            };
            if new_sig.has_shifted_from(prev) {
                self.shift_history.push(SignatureShift {
                    at: now,
                    from_geometry: prev.geometry.clone(),
                    to_geometry: geometry.clone(),
                    magnitude: (complexity - prev.complexity).abs()
                        + (vitality - prev.vitality).abs(),
                    description: format!("{} → {}", prev.geometry.as_str(), geometry.as_str()),
                });
                if self.shift_history.len() > 50 {
                    self.shift_history.remove(0);
                }
                Some(now)
            } else {
                prev.last_major_shift_at
            }
        } else {
            None
        };

        self.current_signature = Some(LivingSignature {
            id: Uuid::new_v4(),
            computed_at: now,
            primary_color: primary,
            secondary_color: secondary,
            accent_color: accent,
            geometry,
            symmetry,
            motion,
            complexity,
            vitality,
            depth,
            evolution_count,
            last_major_shift_at,
            description,
        });

        self.current_signature.as_ref().unwrap()
    }
}

// ── Derivation helpers ────────────────────────────────────────────────────────

fn derive_colors(
    nodes: &[&NodeData],
    civs_colors: &[[f32; 4]],
    crystals: &[KnowledgeCrystal],
) -> ([f32; 4], [f32; 4], [f32; 4]) {
    // Primary: average of top-2 civilization colors (weighted by implied size)
    let primary = if civs_colors.is_empty() {
        [0.25, 0.55, 0.95, 1.0] // Default: deep blue
    } else {
        let c = civs_colors[0];
        [c[0] * 0.7 + 0.15, c[1] * 0.7 + 0.15, c[2] * 0.7 + 0.15, 1.0]
    };

    // Secondary: second civilization or complementary rotation
    let secondary = if civs_colors.len() >= 2 {
        let c = civs_colors[1];
        [c[0], c[1], c[2], 1.0]
    } else {
        // Complementary: rotate hue by 0.5
        [1.0 - primary[0], 1.0 - primary[1], primary[2], 1.0]
    };

    // Accent: from crystals (crystallized knowledge = gold/amber) or high-entropy nodes
    let accent = if !crystals.is_empty() {
        [0.95, 0.82, 0.28, 1.0] // Gold — crystallized knowledge
    } else {
        let ghost_count = nodes.iter().filter(|n| n.is_ghost).count();
        let total = nodes.len().max(1);
        if ghost_count as f32 / total as f32 > 0.2 {
            [0.65, 0.40, 0.90, 1.0] // Violet — ghost-heavy past
        } else {
            [0.28, 0.95, 0.68, 1.0] // Teal — active and growing
        }
    };

    (primary, secondary, accent)
}

fn derive_geometry(lore: &[LoreEntry]) -> GeometryKind {
    if lore.is_empty() {
        return GeometryKind::Line;
    }

    let mut origin_legacy = 0usize;
    let mut conflict_resolution = 0usize;
    let mut transform_revelation = 0usize;
    let mut tectonic = 0usize;

    for entry in lore {
        match entry.arc_type {
            ArcType::Origin | ArcType::Legacy => origin_legacy += 1,
            ArcType::Conflict | ArcType::Resolution => conflict_resolution += 1,
            ArcType::Transformation | ArcType::Revelation => transform_revelation += 1,
            ArcType::Tectonic => tectonic += 1,
        }
    }

    let max = [
        origin_legacy,
        conflict_resolution,
        transform_revelation,
        tectonic,
    ]
    .into_iter()
    .max()
    .unwrap_or(0);

    if max == 0 {
        return GeometryKind::Line;
    }
    if origin_legacy == max {
        GeometryKind::Circle
    } else if conflict_resolution == max {
        GeometryKind::Wave
    } else if transform_revelation == max {
        GeometryKind::Spiral
    } else {
        GeometryKind::Fractal
    }
}

fn derive_symmetry(rituals: &[Ritual]) -> SymmetryKind {
    if rituals.is_empty() {
        return SymmetryKind::None;
    }

    // Most common ritual sequence length
    let mut len_counts = [0usize; 8]; // index = sequence length
    for r in rituals {
        let len = r.sequence.len().min(7);
        len_counts[len] += 1;
    }
    let dominant_len = len_counts
        .iter()
        .enumerate()
        .skip(2)
        .max_by_key(|(_, &c)| c)
        .map(|(i, _)| i)
        .unwrap_or(0);

    match dominant_len {
        2 => SymmetryKind::Bilateral,
        3 => SymmetryKind::Radial3,
        4 => SymmetryKind::Radial4,
        5..=7 => SymmetryKind::Rotational,
        _ => SymmetryKind::None,
    }
}

fn derive_motion(season: CognitiveSeason) -> MotionKind {
    match season {
        CognitiveSeason::Spring => MotionKind::Flow,
        CognitiveSeason::Summer => MotionKind::Pulse,
        CognitiveSeason::Autumn => MotionKind::Breathe,
        CognitiveSeason::Winter => MotionKind::Crystallize,
    }
}
