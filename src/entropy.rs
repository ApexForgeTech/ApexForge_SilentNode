use crate::domain::NodeType;
use crate::graph::CognitiveGraph;
use chrono::{DateTime, Utc};
use rayon::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct EntropyEngine {
    decay_rates: HashMap<NodeType, f32>,
    ghost_threshold: f32,
    fossil_age_days: i64,
}

impl Default for EntropyEngine {
    fn default() -> Self {
        let mut decay_rates = HashMap::new();
        decay_rates.insert(NodeType::Idea, 0.08);
        decay_rates.insert(NodeType::Memory, 0.03);
        decay_rates.insert(NodeType::Project, 0.02);
        decay_rates.insert(NodeType::Person, 0.01);
        decay_rates.insert(NodeType::Artifact, 0.015);
        decay_rates.insert(NodeType::Media, 0.02);
        decay_rates.insert(NodeType::Process, 0.06);
        decay_rates.insert(NodeType::World, 0.01);
        decay_rates.insert(NodeType::Ghost, 0.0);
        decay_rates.insert(NodeType::Fossil, 0.0);

        Self {
            decay_rates,
            ghost_threshold: 0.92,
            fossil_age_days: 365,
        }
    }
}

struct EntropyUpdate {
    node_id: Uuid,
    new_entropy: f32,
    becomes_ghost: bool,
    becomes_fossil: bool,
}

impl EntropyEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parallel entropy tick over all nodes.
    /// Phase 1: compute updates in parallel (read-only snapshot).
    /// Phase 2: apply updates sequentially.
    pub fn tick(&self, graph: &mut CognitiveGraph, now: DateTime<Utc>) {
        let snapshot: Vec<(Uuid, NodeType, DateTime<Utc>, DateTime<Utc>, bool, bool)> = graph
            .nodes()
            .map(|n| {
                (
                    n.id,
                    n.node_type,
                    n.accessed_at,
                    n.created_at,
                    n.is_fossil,
                    n.is_void,
                )
            })
            .collect();

        let updates: Vec<EntropyUpdate> = snapshot
            .par_iter()
            .filter(|(_, _, _, _, is_fossil, is_void)| !is_fossil && !is_void)
            .map(|(id, node_type, accessed_at, created_at, _, _)| {
                let days = (now - *accessed_at).num_seconds().max(0) as f32 / 86_400.0;
                let rate = *self.decay_rates.get(node_type).unwrap_or(&0.02);
                let new_entropy = 1.0 - (-rate * days).exp();
                let age_days = (now - *created_at).num_days();
                EntropyUpdate {
                    node_id: *id,
                    new_entropy,
                    becomes_ghost: new_entropy >= self.ghost_threshold,
                    becomes_fossil: age_days >= self.fossil_age_days && new_entropy < 0.3,
                }
            })
            .collect();

        for update in updates {
            if let Some(node) = graph.get_node_mut(update.node_id) {
                node.entropy = update.new_entropy;
                if update.becomes_ghost {
                    node.is_ghost = true;
                }
                if update.becomes_fossil {
                    node.is_fossil = true;
                }
            }
        }
    }

    /// Instantly reverse entropy for a node — restores it from ghost/fossil state.
    pub fn reverse_entropy(&self, graph: &mut CognitiveGraph, node_id: Uuid, now: DateTime<Utc>) {
        if let Some(node) = graph.get_node_mut(node_id) {
            node.accessed_at = now;
            node.entropy *= 0.1;
            node.is_ghost = false;
        }
    }
}
