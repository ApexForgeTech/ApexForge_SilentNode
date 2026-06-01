use crate::domain::{EdgeData, NodeData};
use crate::graph::CognitiveGraph;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// A frozen view of the graph used to detect structural shifts.
#[derive(Debug, Clone)]
pub struct GraphSnapshot {
    pub nodes: Vec<NodeData>,
    pub edges: Vec<EdgeData>,
    pub taken_at: DateTime<Utc>,
}

impl GraphSnapshot {
    pub fn from_graph(graph: &CognitiveGraph) -> Self {
        let view = graph.export();
        Self {
            nodes: view.nodes,
            edges: view.edges,
            taken_at: Utc::now(),
        }
    }
}

/// A significant structural shift detected between two graph states.
#[derive(Debug, Clone)]
pub struct TectonicEvent {
    pub magnitude: f32,             // 0.0–1.0 normalised intensity
    pub epicenter_id: Option<Uuid>, // The node where change originated
    pub affected_node_count: usize,
    pub detected_at: DateTime<Utc>,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct TectonicDetector {
    /// Minimum structural change score to emit an event.
    pub threshold: f32,
}

impl Default for TectonicDetector {
    fn default() -> Self {
        Self { threshold: 0.18 }
    }
}

impl TectonicDetector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_threshold(threshold: f32) -> Self {
        Self { threshold }
    }

    /// Compare a previous snapshot to the current live graph.
    /// Returns `Some(TectonicEvent)` when structural change exceeds the threshold.
    pub fn check(&self, before: &GraphSnapshot, after: &CognitiveGraph) -> Option<TectonicEvent> {
        let score = self.measure_structural_change(before, after);
        if score < self.threshold {
            return None;
        }

        let epicenter_id = find_epicenter(before, after);
        let affected = count_affected(before, after);
        let description = describe(score, &before, after);

        Some(TectonicEvent {
            magnitude: score,
            epicenter_id,
            affected_node_count: affected,
            detected_at: Utc::now(),
            description,
        })
    }

    /// Returns a [0,1] structural change score.
    pub fn measure_structural_change(&self, before: &GraphSnapshot, after: &CognitiveGraph) -> f32 {
        let prev_nodes = before.nodes.len() as f32;
        let prev_edges = before.edges.len() as f32;
        let curr_nodes = after.node_count() as f32;
        let curr_edges = after.edge_count() as f32;

        if prev_nodes == 0.0 && curr_nodes == 0.0 {
            return 0.0;
        }

        let node_delta = (curr_nodes - prev_nodes).abs() / (prev_nodes + curr_nodes + 1.0);
        let edge_delta = (curr_edges - prev_edges).abs() / (prev_edges + curr_edges + 1.0);

        // Entropy shift
        let prev_entropy: f32 = if before.nodes.is_empty() {
            0.0
        } else {
            before.nodes.iter().map(|n| n.entropy).sum::<f32>() / prev_nodes
        };
        let curr_entropy: f32 = if after.node_count() == 0 {
            0.0
        } else {
            after.nodes().map(|n| n.entropy).sum::<f32>() / curr_nodes
        };
        let entropy_delta = (curr_entropy - prev_entropy).abs();

        // Ghost appearance rate
        let prev_ghost = before.nodes.iter().filter(|n| n.is_ghost).count() as f32;
        let curr_ghost = after.nodes().filter(|n| n.is_ghost).count() as f32;
        let ghost_delta = (curr_ghost - prev_ghost).abs() / (prev_nodes + 1.0);

        (node_delta * 0.30 + edge_delta * 0.35 + entropy_delta * 0.20 + ghost_delta * 0.15)
            .clamp(0.0, 1.0)
    }
}

fn find_epicenter(before: &GraphSnapshot, after: &CognitiveGraph) -> Option<Uuid> {
    // Node with the largest individual change in gravity (proxy for activity burst)
    let before_gravity: std::collections::HashMap<Uuid, f32> =
        before.nodes.iter().map(|n| (n.id, n.gravity)).collect();

    after
        .nodes()
        .filter_map(|n| {
            let prev_g = before_gravity.get(&n.id).copied().unwrap_or(n.gravity);
            let delta = (n.gravity - prev_g).abs();
            if delta > 0.01 {
                Some((n.id, delta))
            } else {
                None
            }
        })
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(id, _)| id)
}

fn count_affected(before: &GraphSnapshot, after: &CognitiveGraph) -> usize {
    let before_ids: std::collections::HashSet<Uuid> = before.nodes.iter().map(|n| n.id).collect();
    let after_ids: std::collections::HashSet<Uuid> = after.node_ids().into_iter().collect();

    before_ids.symmetric_difference(&after_ids).count()
        + after
            .nodes()
            .filter(|n| n.is_ghost && !before.nodes.iter().any(|b| b.id == n.id && b.is_ghost))
            .count()
}

fn describe(score: f32, before: &GraphSnapshot, after: &CognitiveGraph) -> String {
    let node_change = after.node_count() as i64 - before.nodes.len() as i64;
    let edge_change = after.edge_count() as i64 - before.edges.len() as i64;

    match score {
        s if s > 0.7 => format!(
            "Major tectonic shift (magnitude {:.2}): {} nodes, {} edges changed",
            score, node_change, edge_change
        ),
        s if s > 0.4 => format!(
            "Significant restructuring (magnitude {:.2}): {} nodes, {} edges changed",
            score, node_change, edge_change
        ),
        _ => format!(
            "Minor structural drift (magnitude {:.2}): {} nodes, {} edges changed",
            score, node_change, edge_change
        ),
    }
}
