// FossilEngine — node fossilization lifecycle.

use super::TemporalEngine;
use crate::domain::{ChangeType, NodeData, NodeType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Criteria result for a potential fossilization check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FossilizationCheck {
    pub node_id: uuid::Uuid,
    pub qualifies: bool,
    pub reasons: Vec<String>,
    /// Composite score 0..1 — higher = more fossilizable.
    pub score: f32,
}

impl FossilizationCheck {
    pub fn summary(&self) -> String {
        if self.qualifies {
            format!(
                "QUALIFIES (score={:.3}): {}",
                self.score,
                self.reasons.join("; ")
            )
        } else {
            format!(
                "does not qualify (score={:.3}): {}",
                self.score,
                self.reasons.join("; ")
            )
        }
    }
}

/// Governs the fossilization and excavation lifecycle of nodes.
pub struct FossilEngine {
    /// Minimum entropy required for fossilization.
    pub entropy_threshold: f32,
    /// Node must be at least this many days old.
    pub min_age_days: f64,
    /// Node must not have been accessed within this many days.
    pub silence_days: f64,
    /// Fossilization score cutoff (0..1).
    pub score_threshold: f32,
}

impl Default for FossilEngine {
    fn default() -> Self {
        Self {
            entropy_threshold: 0.5,
            min_age_days: 30.0,
            silence_days: 14.0,
            score_threshold: 0.55,
        }
    }
}

impl FossilEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check whether `node` qualifies for fossilization.
    ///
    /// Criteria (all weighted):
    ///   1. Entropy ≥ threshold (+0.30)
    ///   2. Node age ≥ min_age_days (+0.25)
    ///   3. No access in last silence_days (+0.25)
    ///   4. Node is already ghosted (+0.20)
    ///   5. Zero outgoing temporal-snapshot changes in last 30 days (+0.10 bonus)
    pub fn check_fossilization(
        &self,
        node: &NodeData,
        engine: &TemporalEngine,
        degree: usize,
        now: DateTime<Utc>,
    ) -> FossilizationCheck {
        if node.is_fossil {
            return FossilizationCheck {
                node_id: node.id,
                qualifies: false,
                reasons: vec!["already a fossil".into()],
                score: 0.0,
            };
        }

        let mut score = 0.0_f32;
        let mut reasons = Vec::new();

        // Entropy criterion
        if node.entropy >= self.entropy_threshold {
            score += 0.30;
            reasons.push(format!(
                "entropy={:.3} ≥ {:.3}",
                node.entropy, self.entropy_threshold
            ));
        }

        // Age criterion
        let age_days = (now - node.created_at).num_seconds() as f64 / 86_400.0;
        if age_days >= self.min_age_days {
            score += 0.25;
            reasons.push(format!("age={:.1}d ≥ {:.1}d", age_days, self.min_age_days));
        }

        // Silence criterion
        let silence_days = (now - node.accessed_at).num_seconds() as f64 / 86_400.0;
        if silence_days >= self.silence_days {
            score += 0.25;
            reasons.push(format!(
                "silent={:.1}d ≥ {:.1}d",
                silence_days, self.silence_days
            ));
        }

        // Ghost criterion
        if node.is_ghost {
            score += 0.20;
            reasons.push("already ghosted".into());
        }

        // Temporal quietude: no change events in last 30 days
        let recent_cutoff = now - chrono::Duration::days(30);
        let recent_changes = engine
            .snapshots_for(node.id)
            .into_iter()
            .filter(|s| s.timestamp >= recent_cutoff)
            .count();
        if recent_changes == 0 {
            score += 0.10;
            reasons.push("no recent temporal changes".into());
        }

        // Low-connectivity bonus
        if degree == 0 {
            score += 0.05;
            reasons.push("isolated node".into());
        }

        let qualifies = score >= self.score_threshold;

        FossilizationCheck {
            node_id: node.id,
            qualifies,
            reasons,
            score,
        }
    }

    /// Fossilize `node` in place: set `is_fossil`, change type to `Fossil`,
    /// mark as ghost (visual state), record the transition in `engine`.
    pub fn fossilize(&self, node: &mut NodeData, engine: &mut TemporalEngine) {
        if node.is_fossil {
            return;
        }
        node.is_fossil = true;
        node.is_ghost = true;
        node.node_type = NodeType::Fossil;
        node.aura_color = "#6b7280".to_string(); // muted slate crystalline
        engine.record_change(node, ChangeType::StateChanged);
    }

    /// Excavate a fossil: restore it to an active Ghost node ready to be
    /// revived or re-examined. Records the transition in `engine`.
    pub fn excavate(
        &self,
        node: &mut NodeData,
        engine: &mut TemporalEngine,
        restored_type: Option<NodeType>,
    ) {
        if !node.is_fossil {
            return;
        }
        node.is_fossil = false;
        node.node_type = restored_type.unwrap_or(NodeType::Ghost);
        node.is_ghost = node.node_type == NodeType::Ghost;
        node.aura_color = "#a78bfa".to_string(); // violet revived
        node.entropy *= 0.5; // partial entropy reset on excavation
        engine.record_change(node, ChangeType::StateChanged);
    }

    /// Scan `nodes` and return all that currently qualify for fossilization.
    pub fn candidates<'a>(
        &self,
        nodes: impl Iterator<Item = &'a NodeData>,
        engine: &TemporalEngine,
        degree_fn: impl Fn(uuid::Uuid) -> usize,
        now: DateTime<Utc>,
    ) -> Vec<FossilizationCheck> {
        nodes
            .map(|n| self.check_fossilization(n, engine, degree_fn(n.id), now))
            .filter(|c| c.qualifies)
            .collect()
    }
}
