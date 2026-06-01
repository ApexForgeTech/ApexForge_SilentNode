use crate::graph::CognitiveGraph;
use rayon::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

const THETA: f32 = 0.5;

#[derive(Debug, Clone)]
pub struct GravityEngine {
    repulsion: f32,
    attraction: f32,
    damping: f32,
    center_pull: f32,
    max_step: f32,
}

impl Default for GravityEngine {
    fn default() -> Self {
        Self {
            repulsion: 2.0,
            attraction: 0.04,
            damping: 0.85,
            center_pull: 0.01,
            max_step: 8.0,
        }
    }
}

// Barnes-Hut quadtree node
struct QtNode {
    cx: f32,
    cy: f32,
    half: f32,
    total_mass: f32,
    com_x: f32,
    com_y: f32,
    // None = internal node, Some = leaf with a single body
    body: Option<(f32, f32, f32)>,      // (x, y, mass)
    children: [Option<Box<QtNode>>; 4], // [sw, nw, se, ne] indexed by (right | top<<1)
}

impl QtNode {
    fn new(cx: f32, cy: f32, half: f32) -> Self {
        Self {
            cx,
            cy,
            half,
            total_mass: 0.0,
            com_x: 0.0,
            com_y: 0.0,
            body: None,
            children: [None, None, None, None],
        }
    }

    fn quadrant(&self, x: f32, y: f32) -> usize {
        let right = (x >= self.cx) as usize;
        let top = (y >= self.cy) as usize;
        right | (top << 1)
    }

    fn child_center(&self, q: usize) -> (f32, f32) {
        let h = self.half * 0.5;
        let dx = if q & 1 != 0 { h } else { -h };
        let dy = if q & 2 != 0 { h } else { -h };
        (self.cx + dx, self.cy + dy)
    }

    fn insert(&mut self, x: f32, y: f32, mass: f32) {
        // Update center of mass and total mass
        self.com_x = (self.com_x * self.total_mass + x * mass) / (self.total_mass + mass);
        self.com_y = (self.com_y * self.total_mass + y * mass) / (self.total_mass + mass);
        self.total_mass += mass;

        // Stop subdividing when cells become tiny — treat as single aggregate body.
        // This also prevents infinite recursion when two bodies share the same position.
        if self.half < 1e-6 {
            self.body = None; // treat as internal aggregate
            return;
        }

        match self.body {
            None if self.children.iter().all(|c| c.is_none()) => {
                // Empty leaf — store body here
                self.body = Some((x, y, mass));
            }
            Some((bx, by, bm)) => {
                // Occupied leaf — subdivide: push existing body down then insert new
                self.body = None;
                let q = self.quadrant(bx, by);
                let (ccx, ccy) = self.child_center(q);
                let child = self.children[q]
                    .get_or_insert_with(|| Box::new(QtNode::new(ccx, ccy, self.half * 0.5)));
                child.insert(bx, by, bm);

                let q2 = self.quadrant(x, y);
                let (ccx2, ccy2) = self.child_center(q2);
                let child2 = self.children[q2]
                    .get_or_insert_with(|| Box::new(QtNode::new(ccx2, ccy2, self.half * 0.5)));
                child2.insert(x, y, mass);
            }
            None => {
                // Internal node — push into child
                let q = self.quadrant(x, y);
                let (ccx, ccy) = self.child_center(q);
                let child = self.children[q]
                    .get_or_insert_with(|| Box::new(QtNode::new(ccx, ccy, self.half * 0.5)));
                child.insert(x, y, mass);
            }
        }
    }

    // Iterative DFS force computation — avoids stack overflow for large trees
    fn compute_force(
        &self,
        x: f32,
        y: f32,
        repulsion: f32,
        attraction_factor: f32,
        my_mass: f32,
    ) -> (f32, f32) {
        let mut fx = 0.0f32;
        let mut fy = 0.0f32;

        let mut stack: Vec<&QtNode> = vec![self];

        while let Some(node) = stack.pop() {
            if node.total_mass == 0.0 {
                continue;
            }
            let dx = node.com_x - x;
            let dy = node.com_y - y;
            let dist_sq = (dx * dx + dy * dy).max(0.01);
            let dist = dist_sq.sqrt();

            let is_leaf_body = node.body.is_some();
            let is_far = (node.half * 2.0) / dist < THETA;

            if is_leaf_body || is_far {
                // Treat as single body
                let rep = repulsion / dist_sq;
                let att = attraction_factor * (my_mass * node.total_mass).sqrt() / (1.0 + dist);
                let net = att - rep;
                fx += (dx / dist) * net;
                fy += (dy / dist) * net;
            } else {
                for child in node.children.iter().flatten() {
                    stack.push(child);
                }
            }
        }

        (fx, fy)
    }
}

impl GravityEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn recalculate_masses(
        &self,
        graph: &mut CognitiveGraph,
        focus_heatmap: &HashMap<Uuid, f32>,
    ) {
        let node_ids = graph.node_ids();
        for node_id in node_ids {
            let degree = graph.degree(node_id) as f32;
            if let Some(node) = graph.get_node_mut(node_id) {
                let recency_days =
                    (chrono::Utc::now() - node.accessed_at).num_seconds().max(0) as f32 / 86_400.0;
                let recency_score = 1.0 / (1.0 + recency_days);
                let focus_score = focus_heatmap.get(&node_id).copied().unwrap_or(0.0);
                node.gravity =
                    1.0 + node.access_count as f32 * 0.2 + degree + recency_score + focus_score;
            }
        }
    }

    pub fn step(&self, graph: &mut CognitiveGraph, delta_time: f32) {
        let node_ids = graph.node_ids();
        if node_ids.is_empty() {
            return;
        }

        let snapshot: Vec<(Uuid, f32, f32, f32)> = node_ids
            .iter()
            .filter_map(|&id| {
                graph
                    .get_node(id)
                    .map(|n| (id, n.position.x, n.position.y, n.gravity))
            })
            .collect();

        // Compute bounding box to size the quadtree
        let min_x = snapshot.iter().map(|n| n.1).fold(f32::INFINITY, f32::min);
        let max_x = snapshot
            .iter()
            .map(|n| n.1)
            .fold(f32::NEG_INFINITY, f32::max);
        let min_y = snapshot.iter().map(|n| n.2).fold(f32::INFINITY, f32::min);
        let max_y = snapshot
            .iter()
            .map(|n| n.2)
            .fold(f32::NEG_INFINITY, f32::max);

        let cx = (min_x + max_x) * 0.5;
        let cy = (min_y + max_y) * 0.5;
        let half = ((max_x - min_x).max(max_y - min_y) * 0.5 + 1.0) * 1.05;

        let mut root = QtNode::new(cx, cy, half);
        for &(_, x, y, mass) in &snapshot {
            root.insert(x, y, mass);
        }

        // Parallel force computation using rayon
        let forces: Vec<(Uuid, f32, f32)> = snapshot
            .par_iter()
            .map(|&(id, x, y, mass)| {
                let center_fx = -x * self.center_pull;
                let center_fy = -y * self.center_pull;
                let (tree_fx, tree_fy) =
                    root.compute_force(x, y, self.repulsion, self.attraction, mass);
                (id, center_fx + tree_fx, center_fy + tree_fy)
            })
            .collect();

        for (id, fx, fy) in forces {
            if let Some(node) = graph.get_node_mut(id) {
                let step_x = (fx * delta_time * self.damping).clamp(-self.max_step, self.max_step);
                let step_y = (fy * delta_time * self.damping).clamp(-self.max_step, self.max_step);
                node.position.x += step_x;
                node.position.y += step_y;
                node.velocity = (step_x * step_x + step_y * step_y).sqrt();
            }
        }
    }
}
