/// Phase 10 — Shadow Projects
///
/// A Shadow Project is a project defined entirely by its absence —
/// a direction deliberately not taken, a concept consciously refused,
/// a system intentionally not constructed.
///
/// Vision.md:
///   "The most revealing thing about a creator is not what they built.
///    It is what they chose not to build."
///
/// Shadow Projects are identified from:
///   1. Long-incubation Void Zone entities (never extracted, >14 days)
///   2. Abandoned high-gravity nodes (important once, neglected now)
///   3. Released Digital Shadows (formally dissolved without becoming real)
///   4. Orphaned architectures (Project nodes with former connections, now isolated)
use crate::domain::{NodeData, NodeType};
use crate::systems::{DigitalShadow, VoidZone};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── ShadowProject ─────────────────────────────────────────────────────────────

/// The origin story of a Shadow Project — why it became a shadow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShadowOrigin {
    /// An idea placed in the Void Zone that was never extracted.
    /// Incubation exceeded the natural threshold.
    LongIncubation {
        void_zone_id: Uuid,
        incubation_days: f32,
        resonance_readiness: f32,
    },
    /// A Digital Shadow that was formally Released (dissolved without becoming real).
    ReleasedShadow {
        shadow_node_id: Uuid,
        revisit_count: usize,
    },
    /// A Project/World node that once had high gravity but has been abandoned.
    /// Significant attention invested, no follow-through.
    AbandonedHighGravity {
        node_id: Uuid,
        peak_gravity: f32,
        days_since_access: f32,
    },
    /// A Project node that had connections (architecture existed) but is now
    /// isolated and stagnant — the structure was built but not inhabited.
    OrphanedArchitecture {
        node_id: Uuid,
        former_connection_proxy: f32, // based on gravity as proxy for past connections
    },
}

/// A Shadow Project — the shape of something that never fully materialized.
///
/// Vision.md:
///   "The constellation of Shadow Projects surrounding a user's universe reveals:
///    their creative boundaries, their fears, their values, their aesthetic."
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowProject {
    pub id: Uuid,
    pub origin: ShadowOrigin,
    /// Node IDs associated with this shadow.
    pub related_nodes: Vec<Uuid>,
    /// Short label derived from node content.
    pub label: String,
    pub description: String,
    pub detected_at: DateTime<Utc>,
    /// How old is this shadow?
    pub age_days: f32,
    /// Visual intensity — how strongly it should glow in the negative space.
    /// 0.0 = barely perceptible, 1.0 = vivid structural outline
    pub luminescence: f32,
}

impl ShadowProject {
    pub fn summary(&self) -> String {
        let origin_kind = match &self.origin {
            ShadowOrigin::LongIncubation {
                incubation_days, ..
            } => format!("void {:.0}d", incubation_days),
            ShadowOrigin::ReleasedShadow { revisit_count, .. } => {
                format!("released ({}× revisited)", revisit_count)
            }
            ShadowOrigin::AbandonedHighGravity {
                days_since_access, ..
            } => format!("abandoned {:.0}d ago", days_since_access),
            ShadowOrigin::OrphanedArchitecture { .. } => "orphaned architecture".to_string(),
        };
        format!(
            "[{:.2}] {:40} — {} | age={:.0}d",
            self.luminescence,
            self.label.chars().take(40).collect::<String>(),
            origin_kind,
            self.age_days,
        )
    }
}

// ── ShadowProjectDetector ─────────────────────────────────────────────────────

/// Detects Shadow Projects from the current workspace state.
pub struct ShadowProjectDetector {
    /// Minimum days in void before it becomes a shadow project.
    pub void_incubation_threshold_days: f32,
    /// Minimum gravity for a node to be "high gravity".
    pub high_gravity_threshold: f32,
    /// Days without access before abandonment is declared.
    pub abandonment_days: f32,
}

impl Default for ShadowProjectDetector {
    fn default() -> Self {
        Self {
            void_incubation_threshold_days: 14.0,
            high_gravity_threshold: 2.0,
            abandonment_days: 60.0,
        }
    }
}

impl ShadowProjectDetector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Detect all Shadow Projects from workspace state.
    pub fn detect(
        &self,
        nodes: &[&NodeData],
        void_zones: &[VoidZone],
        shadows: &[DigitalShadow],
        now: DateTime<Utc>,
    ) -> Vec<ShadowProject> {
        let mut projects: Vec<ShadowProject> = Vec::new();

        // ── 1. Long-incubation Void Zones ─────────────────────────────────────
        for zone in void_zones {
            let days = zone.incubation_days(now);
            if days < self.void_incubation_threshold_days {
                continue;
            }
            if zone.resonance_readiness > 0.5 {
                continue; // Ready to emerge — not a shadow project yet
            }

            // Find node labels for related entities
            let label = zone
                .entities
                .first()
                .and_then(|id| nodes.iter().find(|n| n.id == *id))
                .map(|n| n.content.chars().take(40).collect::<String>())
                .unwrap_or_else(|| format!("Void Zone {}", &zone.id.to_string()[..8]));

            let luminescence =
                ((days - self.void_incubation_threshold_days) / 30.0).clamp(0.1, 0.8);

            projects.push(ShadowProject {
                id: Uuid::new_v4(),
                origin: ShadowOrigin::LongIncubation {
                    void_zone_id: zone.id,
                    incubation_days: days,
                    resonance_readiness: zone.resonance_readiness,
                },
                related_nodes: zone.entities.clone(),
                label,
                description: format!(
                    "Incubating in the Void for {:.0} days — never extracted, \
                     resonance={:.2}. A path not yet committed to.",
                    days, zone.resonance_readiness
                ),
                detected_at: now,
                age_days: days,
                luminescence,
            });
        }

        // ── 2. Released Digital Shadows ───────────────────────────────────────
        // A shadow that was formally dissolved (is_void + high entropy) represents
        // a conscious "I will not build this" decision.
        for node in nodes.iter().filter(|n| n.is_void && n.entropy > 0.05) {
            // Cross-reference with digital shadows list
            if let Some(shadow) = shadows.iter().find(|s| s.node_id == node.id) {
                if shadow.revisit_count < 2 {
                    continue;
                }

                let age_days = (now - node.created_at).num_seconds().max(0) as f32 / 86400.0;
                let luminescence = (shadow.revisit_count as f32 / 6.0).clamp(0.1, 0.9);

                projects.push(ShadowProject {
                    id: Uuid::new_v4(),
                    origin: ShadowOrigin::ReleasedShadow {
                        shadow_node_id: node.id,
                        revisit_count: shadow.revisit_count,
                    },
                    related_nodes: vec![node.id],
                    label: node.content.chars().take(40).collect(),
                    description: format!(
                        "Revisited {} times, then consciously released to the Void. \
                         A direction you looked at and chose not to take.",
                        shadow.revisit_count
                    ),
                    detected_at: now,
                    age_days,
                    luminescence,
                });
            }
        }

        // ── 3. Abandoned High-Gravity Nodes ───────────────────────────────────
        // Project/World nodes that were once important (gravity > threshold)
        // but haven't been accessed in a long time.
        for node in nodes.iter().filter(|n| {
            !n.is_ghost
                && !n.is_fossil
                && !n.is_void
                && n.gravity >= self.high_gravity_threshold
                && matches!(
                    n.node_type,
                    NodeType::Project | NodeType::World | NodeType::Artifact
                )
        }) {
            let days_since = (now - node.accessed_at).num_seconds().max(0) as f32 / 86400.0;
            if days_since < self.abandonment_days {
                continue;
            }

            let age_days = (now - node.created_at).num_seconds().max(0) as f32 / 86400.0;
            // Luminescence: higher gravity + longer silence = more vivid shadow
            let luminescence =
                ((node.gravity / 4.0) * (days_since / 120.0).min(1.0)).clamp(0.1, 1.0);

            projects.push(ShadowProject {
                id: Uuid::new_v4(),
                origin: ShadowOrigin::AbandonedHighGravity {
                    node_id: node.id,
                    peak_gravity: node.gravity,
                    days_since_access: days_since,
                },
                related_nodes: vec![node.id],
                label: node.content.chars().take(40).collect(),
                description: format!(
                    "Once held gravity={:.2} but has not been accessed in {:.0} days. \
                     Significant attention invested — the work was never completed.",
                    node.gravity, days_since
                ),
                detected_at: now,
                age_days,
                luminescence,
            });
        }

        // ── 4. Orphaned Architectures ─────────────────────────────────────────
        // Project nodes with high gravity (had connections once, implied by gravity)
        // but now have zero connections and are old.
        for node in nodes.iter().filter(|n| {
            !n.is_ghost && !n.is_fossil && !n.is_void
            && n.gravity > 1.5
            && n.access_count > 3  // was visited before
            && matches!(n.node_type, NodeType::Project)
        }) {
            let age_days = (now - node.created_at).num_seconds().max(0) as f32 / 86400.0;
            if age_days < 30.0 {
                continue; // Too young to be orphaned
            }

            // Entropy rising but not ghost yet → structure without momentum
            if node.entropy < 0.35 || node.entropy > 0.80 {
                continue;
            }

            let luminescence = (node.gravity / 5.0 * (node.entropy / 0.6)).clamp(0.1, 0.7);

            projects.push(ShadowProject {
                id: Uuid::new_v4(),
                origin: ShadowOrigin::OrphanedArchitecture {
                    node_id: node.id,
                    former_connection_proxy: node.gravity,
                },
                related_nodes: vec![node.id],
                label: node.content.chars().take(40).collect(),
                description: format!(
                    "A project architecture that was built but never fully inhabited. \
                     Age={:.0}d, entropy={:.2}. The structure remains, momentum is gone.",
                    age_days, node.entropy
                ),
                detected_at: now,
                age_days,
                luminescence,
            });
        }

        // Sort by luminescence (most vivid shadows first)
        projects.sort_by(|a, b| {
            b.luminescence
                .partial_cmp(&a.luminescence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        projects
    }
}
