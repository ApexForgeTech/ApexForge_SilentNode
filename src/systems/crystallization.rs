use crate::domain::{EdgeData, NodeData};
use crate::systems::civilization::Civilization;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeCrystal {
    pub id: Uuid,
    pub source_civilization_id: Uuid,
    pub formed_at: DateTime<Utc>,
    pub member_nodes: Vec<Uuid>,
    pub dominant_concept: Option<Uuid>,
    pub internal_density: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrystallizationCheck {
    pub civilization_id: Uuid,
    pub qualifies: bool,
    pub internal_density: f32,
    pub external_density: f32,
    pub stability_score: f32,
    pub size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrystallizationEngine {
    pub density_threshold: f32,
    pub stability_days: u32,
    pub min_cluster_size: usize,
}

impl Default for CrystallizationEngine {
    fn default() -> Self {
        Self {
            density_threshold: 0.65,
            stability_days: 14,
            min_cluster_size: 4,
        }
    }
}

impl CrystallizationEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check whether a civilization qualifies for crystallization.
    pub fn check(
        &self,
        civ: &Civilization,
        nodes: &[&NodeData],
        edges: &[EdgeData],
    ) -> CrystallizationCheck {
        let size = civ.member_nodes.len();
        let member_set: HashSet<Uuid> = civ.member_nodes.iter().copied().collect();

        // Count internal edges (both endpoints in civilization)
        let internal_edges = edges
            .iter()
            .filter(|e| member_set.contains(&e.source_id) && member_set.contains(&e.target_id))
            .count();

        let max_possible = if size > 1 { size * (size - 1) / 2 } else { 1 };
        let internal_density = (internal_edges as f32 / max_possible as f32).clamp(0.0, 1.0);

        // Count external edges (exactly one endpoint in civilization)
        let total_nodes = nodes.len();
        let external_node_count = (total_nodes.saturating_sub(size)).max(1);
        let external_edges = edges
            .iter()
            .filter(|e| member_set.contains(&e.source_id) != member_set.contains(&e.target_id))
            .count();
        let external_density = (external_edges as f32 / external_node_count as f32).clamp(0.0, 1.0);

        // Stability score: nodes that were previously active (high gravity) but have
        // now quieted (low velocity, low entropy) indicate crystallized understanding.
        // Vision.md: "a cluster has remained structurally stable across a significant time"
        let node_map: std::collections::HashMap<Uuid, &NodeData> =
            nodes.iter().map(|n| (n.id, *n)).collect();
        let stability_score = {
            let scores: Vec<f32> = civ
                .member_nodes
                .iter()
                .filter_map(|id| node_map.get(id))
                .map(|n| {
                    // A stable crystal node is:
                    //   • old enough (age > stability_days) — already checked at civilation level
                    //   • was important (gravity > 1.0) — means it was used
                    //   • has low current velocity (not churning)
                    //   • has low entropy (not decaying — crystal resists entropy)
                    let gravity_factor = (n.gravity / 2.0).clamp(0.0, 1.0);
                    let stillness_factor = 1.0 - n.velocity.clamp(0.0, 1.0);
                    let clarity_factor = (1.0 - n.entropy).clamp(0.0, 1.0);
                    // Were accessed before (not abandoned) — proxy via access_count
                    let usage_factor = (n.access_count as f32 / 5.0).clamp(0.0, 1.0).min(1.0);
                    // Stability = was used + now quiet + healthy + old
                    (gravity_factor * 0.30
                        + stillness_factor * 0.30
                        + clarity_factor * 0.25
                        + usage_factor * 0.15)
                        .clamp(0.0, 1.0)
                })
                .collect();
            if scores.is_empty() {
                0.0
            } else {
                scores.iter().sum::<f32>() / scores.len() as f32
            }
        };

        let qualifies = internal_density >= self.density_threshold
            && internal_density > external_density * 1.5
            && civ.age_days >= self.stability_days as f32
            && size >= self.min_cluster_size;

        CrystallizationCheck {
            civilization_id: civ.id,
            qualifies,
            internal_density,
            external_density,
            stability_score,
            size,
        }
    }

    /// Crystallize a civilization into a KnowledgeCrystal.
    pub fn crystallize(
        &self,
        civ: &Civilization,
        nodes: &[&NodeData],
        edges: &[EdgeData],
    ) -> KnowledgeCrystal {
        let check = self.check(civ, nodes, edges);
        KnowledgeCrystal {
            id: Uuid::new_v4(),
            source_civilization_id: civ.id,
            formed_at: Utc::now(),
            member_nodes: civ.member_nodes.clone(),
            dominant_concept: civ.dominant_node,
            internal_density: check.internal_density,
        }
    }
}
