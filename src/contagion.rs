use crate::graph::CognitiveGraph;
use std::collections::{HashSet, VecDeque};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ContagionEngine {
    decay: f32,
    min_strength: f32,
}

impl Default for ContagionEngine {
    fn default() -> Self {
        Self {
            decay: 0.6,
            min_strength: 0.01,
        }
    }
}

impl ContagionEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// BFS propagation of energy from source_id outward through edges.
    /// Strength decays as: next = current * edge_weight * decay.
    /// Stops when strength drops below min_strength.
    pub fn propagate(&self, graph: &mut CognitiveGraph, source_id: Uuid, strength: f32) {
        // Phase 1: BFS to collect (node_id, strength) — read-only pass
        let mut queue: VecDeque<(Uuid, f32)> = VecDeque::new();
        let mut visited: HashSet<Uuid> = HashSet::new();
        let mut to_apply: Vec<(Uuid, f32)> = Vec::new();

        queue.push_back((source_id, strength));
        visited.insert(source_id);

        while let Some((current_id, current_strength)) = queue.pop_front() {
            let neighbors: Vec<(Uuid, f32)> = graph
                .neighbors(current_id)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|n| graph.get_edge(current_id, n.id).map(|e| (n.id, e.weight)))
                .collect();

            for (neighbor_id, edge_weight) in neighbors {
                if visited.contains(&neighbor_id) {
                    continue;
                }
                let next_strength = current_strength * edge_weight * self.decay;
                if next_strength < self.min_strength {
                    continue;
                }
                visited.insert(neighbor_id);
                to_apply.push((neighbor_id, next_strength));
                queue.push_back((neighbor_id, next_strength));
            }
        }

        // Phase 2: apply effects — write pass
        for (node_id, s) in to_apply {
            if let Some(node) = graph.get_node_mut(node_id) {
                if node.is_ghost || node.is_fossil || node.is_void {
                    continue;
                }
                node.entropy = (node.entropy * (1.0 - s * 0.3)).max(0.0);
                node.gravity += s * 0.1;
                node.velocity = (node.velocity + s * 2.0).min(10.0);
            }
        }
    }
}
