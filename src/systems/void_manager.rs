use crate::domain::NodeData;
use crate::systems::resonance::ResonanceChamberEngine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A Void Zone — a region of intentional emptiness where ideas incubate.
///
/// Vision.md: "zero gravitational pull — no connection formation — no entropy —
/// no visibility from other regions — no classification pressure."
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoidZone {
    pub id: Uuid,
    /// Node IDs residing in this void zone.
    pub entities: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
    /// Resonance readiness score: how strongly the void entities resonate with
    /// the active graph. Updated by `check_emergence`. 0.0 = dormant, 1.0 = ready.
    pub resonance_readiness: f32,
    /// Timestamp of the last emergence check.
    pub last_checked_at: Option<DateTime<Utc>>,
}

impl VoidZone {
    /// How many days these ideas have been incubating in the void.
    pub fn incubation_days(&self, now: DateTime<Utc>) -> f32 {
        (now - self.created_at).num_seconds().max(0) as f32 / 86400.0
    }

    /// Whether the incubation has reached natural completion (> 7 days by default).
    pub fn is_mature(&self, now: DateTime<Utc>, min_days: f32) -> bool {
        self.incubation_days(now) >= min_days
    }

    /// Update the resonance readiness score from an emergence check.
    pub fn update_readiness(&mut self, score: f32, now: DateTime<Utc>) {
        self.resonance_readiness = score.clamp(0.0, 1.0);
        self.last_checked_at = Some(now);
    }

    pub fn summary(&self, now: DateTime<Utc>) -> String {
        format!(
            "VoidZone {} | {} entities | {:.1} days incubating | readiness={:.2}",
            &self.id.to_string()[..8],
            self.entities.len(),
            self.incubation_days(now),
            self.resonance_readiness,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergenceCheck {
    pub node_id: Uuid,
    pub resonance_score: f32,
    pub emergence_likely: bool,
    pub similar_active_nodes: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoidManager;

impl Default for VoidManager {
    fn default() -> Self {
        Self
    }
}

impl VoidManager {
    pub fn new() -> Self {
        Self
    }

    /// Create a new void zone containing the given node IDs.
    pub fn create_zone(node_ids: Vec<Uuid>) -> VoidZone {
        VoidZone {
            id: Uuid::new_v4(),
            entities: node_ids,
            created_at: Utc::now(),
            resonance_readiness: 0.0,
            last_checked_at: None,
        }
    }

    /// Check whether a void node has enough resonance with active nodes to emerge,
    /// and optionally update the owning VoidZone's readiness score.
    pub fn check_emergence_and_update(
        &self,
        void_node: &NodeData,
        active_nodes: &[&NodeData],
        resonance: &ResonanceChamberEngine,
        zone: Option<&mut VoidZone>,
    ) -> EmergenceCheck {
        let check = self.check_emergence(void_node, active_nodes, resonance);
        if let Some(z) = zone {
            z.update_readiness(check.resonance_score, Utc::now());
        }
        check
    }

    /// Check whether a void node has enough resonance with active nodes to emerge.
    pub fn check_emergence(
        &self,
        void_node: &NodeData,
        active_nodes: &[&NodeData],
        resonance: &ResonanceChamberEngine,
    ) -> EmergenceCheck {
        if active_nodes.is_empty() {
            return EmergenceCheck {
                node_id: void_node.id,
                resonance_score: 0.0,
                emergence_likely: false,
                similar_active_nodes: Vec::new(),
            };
        }

        // Build a temporary slice with void_node included
        let all_nodes: Vec<&NodeData> = std::iter::once(void_node)
            .chain(active_nodes.iter().copied())
            .collect();

        let pairs = resonance.find_resonances(&all_nodes);

        // Collect pairs involving void_node
        let mut top_matches: Vec<(Uuid, f32)> = pairs
            .iter()
            .filter_map(|p| {
                if p.node_a == void_node.id {
                    Some((p.node_b, p.similarity))
                } else if p.node_b == void_node.id {
                    Some((p.node_a, p.similarity))
                } else {
                    None
                }
            })
            .collect();

        top_matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Resonance score = average similarity to top 3 matches (or fewer)
        let top_k = top_matches.iter().take(3);
        let count = top_matches.len().min(3);
        let resonance_score = if count == 0 {
            0.0
        } else {
            top_matches.iter().take(3).map(|(_, s)| s).sum::<f32>() / count as f32
        };
        let _ = top_k; // suppress warning

        let similar_active_nodes: Vec<Uuid> =
            top_matches.iter().take(5).map(|(id, _)| *id).collect();

        let emergence_likely = self.should_emerge(
            &EmergenceCheck {
                node_id: void_node.id,
                resonance_score,
                emergence_likely: false,
                similar_active_nodes: similar_active_nodes.clone(),
            },
            resonance.min_similarity,
        );

        EmergenceCheck {
            node_id: void_node.id,
            resonance_score,
            emergence_likely,
            similar_active_nodes,
        }
    }

    /// Return true if the check's resonance score exceeds threshold.
    pub fn should_emerge(&self, check: &EmergenceCheck, threshold: f32) -> bool {
        check.resonance_score >= threshold
    }
}
