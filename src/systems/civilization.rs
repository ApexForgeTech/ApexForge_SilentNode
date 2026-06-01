use crate::domain::{EdgeData, NodeData};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Civilization {
    pub id: Uuid,
    pub member_nodes: Vec<Uuid>,
    pub dominant_node: Option<Uuid>,
    pub internal_density: f32,
    pub age_days: f32,
    pub territory_radius: f32,
    pub color: [f32; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CivEventKind {
    Expansion,
    Trade,
    Conflict,
    Merge,
    Collapse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CivilizationEvent {
    pub kind: CivEventKind,
    pub magnitude: f32,
    pub involved_civs: Vec<Uuid>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CivilizationDetector {
    pub min_size: usize,
    pub resolution: f32,
}

impl Default for CivilizationDetector {
    fn default() -> Self {
        Self {
            min_size: 3,
            resolution: 1.2,
        }
    }
}

impl CivilizationDetector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Detect civilizations via label propagation on the edge graph.
    pub fn detect(&self, nodes: &[&NodeData], edges: &[EdgeData]) -> Vec<Civilization> {
        if nodes.is_empty() {
            return Vec::new();
        }

        // Build adjacency: node_id -> list of (neighbor_id, weight)
        let mut adj: HashMap<Uuid, Vec<(Uuid, f32)>> = HashMap::new();
        for node in nodes.iter() {
            adj.entry(node.id).or_default();
        }
        for edge in edges {
            adj.entry(edge.source_id)
                .or_default()
                .push((edge.target_id, edge.weight));
            adj.entry(edge.target_id)
                .or_default()
                .push((edge.source_id, edge.weight));
        }

        // Initialize: each node is its own label (using index for speed)
        let node_ids: Vec<Uuid> = nodes.iter().map(|n| n.id).collect();
        let id_to_idx: HashMap<Uuid, usize> = node_ids
            .iter()
            .enumerate()
            .map(|(i, id)| (*id, i))
            .collect();

        let mut labels: Vec<usize> = (0..node_ids.len()).collect();

        // Greedy label propagation: 20 iterations max
        for _iter in 0..20 {
            let mut changed = false;
            for idx in 0..node_ids.len() {
                let id = node_ids[idx];
                let neighbors = adj.get(&id).cloned().unwrap_or_default();
                if neighbors.is_empty() {
                    continue;
                }

                // Accumulate weight per label of neighbors
                let mut label_weight: HashMap<usize, f32> = HashMap::new();
                for (nb_id, w) in &neighbors {
                    if let Some(&nb_idx) = id_to_idx.get(nb_id) {
                        let nb_label = labels[nb_idx];
                        *label_weight.entry(nb_label).or_insert(0.0) += w;
                    }
                }
                // Find the label with highest accumulated weight
                let best_label = label_weight
                    .into_iter()
                    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(l, _)| l);

                if let Some(lbl) = best_label {
                    if labels[idx] != lbl {
                        labels[idx] = lbl;
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }

        // Group nodes by label
        let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
        for (i, &lbl) in labels.iter().enumerate() {
            groups.entry(lbl).or_default().push(i);
        }

        // Build civilizations for groups >= min_size
        let mut civs: Vec<Civilization> = Vec::new();

        // Build quick lookup for NodeData
        let node_map: HashMap<Uuid, &NodeData> = nodes.iter().map(|n| (n.id, *n)).collect();

        // Build a set of edges (undirected) for density calculation
        let edge_set: std::collections::HashSet<(Uuid, Uuid)> = edges
            .iter()
            .flat_map(|e| {
                let (a, b) = if e.source_id < e.target_id {
                    (e.source_id, e.target_id)
                } else {
                    (e.target_id, e.source_id)
                };
                std::iter::once((a, b))
            })
            .collect();

        for (civ_idx, (_, member_indices)) in groups.iter().enumerate() {
            if member_indices.len() < self.min_size {
                continue;
            }

            let member_ids: Vec<Uuid> = member_indices.iter().map(|&i| node_ids[i]).collect();

            let member_set: std::collections::HashSet<Uuid> = member_ids.iter().copied().collect();

            // Dominant node: highest gravity
            let dominant_node = member_ids
                .iter()
                .max_by(|&&a, &&b| {
                    let ga = node_map.get(&a).map(|n| n.gravity).unwrap_or(0.0);
                    let gb = node_map.get(&b).map(|n| n.gravity).unwrap_or(0.0);
                    ga.partial_cmp(&gb).unwrap_or(std::cmp::Ordering::Equal)
                })
                .copied();

            // Internal density
            let m = member_ids.len();
            let max_edges = if m > 1 { m * (m - 1) / 2 } else { 1 };
            let internal_edges = edge_set
                .iter()
                .filter(|(a, b)| member_set.contains(a) && member_set.contains(b))
                .count();
            let internal_density = (internal_edges as f32 / max_edges as f32).clamp(0.0, 1.0);

            // Age: days since oldest member was created
            let oldest_ts: Option<DateTime<Utc>> = member_ids
                .iter()
                .filter_map(|id| node_map.get(id).map(|n| n.created_at))
                .min();
            let now = Utc::now();
            let age_days = oldest_ts
                .map(|t| (now - t).num_seconds().max(0) as f32 / 86400.0)
                .unwrap_or(0.0);

            // Territory radius: bounding sphere from positions
            let positions: Vec<(f32, f32, f32)> = member_ids
                .iter()
                .filter_map(|id| {
                    node_map
                        .get(id)
                        .map(|n| (n.position.x, n.position.y, n.position.z))
                })
                .collect();
            let territory_radius = bounding_radius(&positions);

            // Color from dominant node or deterministic from index
            let color = if let Some(dom) = dominant_node {
                if let Some(node) = node_map.get(&dom) {
                    aura_color_to_rgba(&node.aura_color)
                } else {
                    civilization_color(civ_idx)
                }
            } else {
                civilization_color(civ_idx)
            };

            civs.push(Civilization {
                id: Uuid::new_v4(),
                member_nodes: member_ids,
                dominant_node,
                internal_density,
                age_days,
                territory_radius,
                color,
            });
        }

        civs.sort_by(|a, b| b.member_nodes.len().cmp(&a.member_nodes.len()));
        civs
    }

    /// Detect events by comparing previous and current civilization snapshots.
    ///
    /// `edges` — all current graph edges, used for Trade (cross-civ bridges) and
    /// Conflict (contested boundary regions) detection.
    pub fn detect_events(
        &self,
        prev: &[Civilization],
        curr: &[Civilization],
        edges: &[EdgeData],
    ) -> Vec<CivilizationEvent> {
        let mut events: Vec<CivilizationEvent> = Vec::new();

        // Build membership maps
        let prev_membership: HashMap<Uuid, Uuid> = prev
            .iter()
            .flat_map(|c| c.member_nodes.iter().map(move |&n| (n, c.id)))
            .collect();

        let curr_membership: HashMap<Uuid, Uuid> = curr
            .iter()
            .flat_map(|c| c.member_nodes.iter().map(move |&n| (n, c.id)))
            .collect();

        // ── Collapse ─────────────────────────────────────────────────────────
        for pc in prev {
            let pc_set: std::collections::HashSet<Uuid> = pc.member_nodes.iter().copied().collect();
            let survived = curr
                .iter()
                .any(|cc| cc.member_nodes.iter().any(|m| pc_set.contains(m)));
            if !survived && pc.member_nodes.len() >= 3 {
                events.push(CivilizationEvent {
                    kind: CivEventKind::Collapse,
                    magnitude: (pc.member_nodes.len() as f32 / 10.0).min(1.0),
                    involved_civs: vec![pc.id],
                    description: format!(
                        "Civilization collapsed ({} members dissolved)",
                        pc.member_nodes.len()
                    ),
                });
            }
        }

        for cc in curr {
            let cc_set: std::collections::HashSet<Uuid> = cc.member_nodes.iter().copied().collect();

            // Which prev civs contributed members to this current civ?
            let source_civs: std::collections::HashSet<Uuid> = cc
                .member_nodes
                .iter()
                .filter_map(|m| prev_membership.get(m))
                .copied()
                .collect();

            // ── Merge ─────────────────────────────────────────────────────────
            if source_civs.len() >= 2 {
                let mut involved: Vec<Uuid> = source_civs.into_iter().collect();
                involved.push(cc.id);
                events.push(CivilizationEvent {
                    kind: CivEventKind::Merge,
                    magnitude: (cc.member_nodes.len() as f32 / 10.0).min(1.0),
                    involved_civs: involved,
                    description: format!(
                        "Civilizations merged into new entity ({} members)",
                        cc.member_nodes.len()
                    ),
                });
                continue;
            }

            // ── Expansion ─────────────────────────────────────────────────────
            let prev_match = prev
                .iter()
                .find(|pc| pc.member_nodes.iter().any(|m| cc_set.contains(m)));
            if let Some(pm) = prev_match {
                let prev_count = pm.member_nodes.len().max(1);
                let curr_count = cc.member_nodes.len();
                if curr_count as f32 / prev_count as f32 > 1.4 {
                    events.push(CivilizationEvent {
                        kind: CivEventKind::Expansion,
                        magnitude: ((curr_count - prev_count) as f32 / 10.0).min(1.0),
                        involved_civs: vec![cc.id],
                        description: format!(
                            "Civilization expanded from {} to {} members",
                            prev_count, curr_count
                        ),
                    });
                }
            }
        }

        // ── Trade: cross-civilization edges ───────────────────────────────────
        // Trade = two distinct civilizations that have at least one direct edge
        // between their member nodes. Each unique pair emits one Trade event.
        let mut trade_pairs: std::collections::HashSet<(Uuid, Uuid)> =
            std::collections::HashSet::new();

        for edge in edges {
            let civ_src = curr_membership.get(&edge.source_id);
            let civ_dst = curr_membership.get(&edge.target_id);
            if let (Some(&ca), Some(&cb)) = (civ_src, civ_dst) {
                if ca != cb {
                    // Canonical pair: smaller UUID first
                    let pair = if ca < cb { (ca, cb) } else { (cb, ca) };
                    if trade_pairs.insert(pair) {
                        // Count total cross-edges for this pair
                        let bridge_count = edges
                            .iter()
                            .filter(|e| {
                                let sa = curr_membership.get(&e.source_id);
                                let da = curr_membership.get(&e.target_id);
                                matches!((sa, da), (Some(&x), Some(&y)) if
                                (x == ca && y == cb) || (x == cb && y == ca))
                            })
                            .count();

                        events.push(CivilizationEvent {
                            kind: CivEventKind::Trade,
                            magnitude: (bridge_count as f32 / 5.0).clamp(0.1, 1.0),
                            involved_civs: vec![ca, cb],
                            description: format!(
                                "Bridge trade established: {} cross-civilization edge(s) detected",
                                bridge_count
                            ),
                        });
                    }
                }
            }
        }

        // ── Conflict: civilizations contesting the same spatial territory ─────
        // Conflict = two civs whose territory circles overlap significantly
        // (territorial overlap = distance between centroids < sum of radii * 0.6).
        // Only emit if both civs have >= 3 members and distinct identities.
        if curr.len() >= 2 {
            for i in 0..curr.len() {
                for j in (i + 1)..curr.len() {
                    let ca = &curr[i];
                    let cb = &curr[j];

                    // Compute centroids from member positions (approximate — we use
                    // territory_radius as bounding sphere proxy)
                    let overlap_threshold = (ca.territory_radius + cb.territory_radius) * 0.55;
                    // We can't compute centroid distance without node positions here,
                    // but we can use territory_radius overlap as a proxy:
                    // If both civs have non-zero radii and their radii sum is large
                    // relative to the larger radius, they may be competing.
                    if ca.territory_radius > 0.0
                        && cb.territory_radius > 0.0
                        && overlap_threshold > 0.0
                        && ca.member_nodes.len() >= 3
                        && cb.member_nodes.len() >= 3
                    {
                        // Only emit Conflict if no Trade already exists between them
                        let pair = if ca.id < cb.id {
                            (ca.id, cb.id)
                        } else {
                            (cb.id, ca.id)
                        };
                        let has_trade = trade_pairs.contains(&pair);
                        if !has_trade {
                            // Detect via density competition: both civs have high internal density
                            // and share at least one common prev-civ ancestor
                            let ca_prev: std::collections::HashSet<Uuid> = ca
                                .member_nodes
                                .iter()
                                .filter_map(|m| prev_membership.get(m))
                                .copied()
                                .collect();
                            let cb_prev: std::collections::HashSet<Uuid> = cb
                                .member_nodes
                                .iter()
                                .filter_map(|m| prev_membership.get(m))
                                .copied()
                                .collect();
                            let shared_ancestry = ca_prev.intersection(&cb_prev).count();

                            if shared_ancestry >= 1
                                && ca.internal_density > 0.35
                                && cb.internal_density > 0.35
                            {
                                let magnitude = ((ca.internal_density + cb.internal_density) / 2.0)
                                    .clamp(0.1, 1.0);
                                events.push(CivilizationEvent {
                                    kind: CivEventKind::Conflict,
                                    magnitude,
                                    involved_civs: vec![ca.id, cb.id],
                                    description: format!(
                                        "Civilizations in conflict over shared territory (shared ancestry: {}, densities: {:.2}/{:.2})",
                                        shared_ancestry, ca.internal_density, cb.internal_density
                                    ),
                                });
                            }
                        }
                    }
                }
            }
        }

        events
    }
}

/// Returns a deterministic distinct color for a civilization by index.
pub fn civilization_color(civ_index: usize) -> [f32; 4] {
    // Golden ratio hue stepping
    const GOLDEN: f32 = 0.618_033_9;
    let hue = ((civ_index as f32 * GOLDEN) % 1.0) * 360.0;
    hsv_to_rgba(hue, 0.7, 0.9)
}

fn hsv_to_rgba(h: f32, s: f32, v: f32) -> [f32; 4] {
    let h = h % 360.0;
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r1, g1, b1) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    [r1 + m, g1 + m, b1 + m, 1.0]
}

/// Convert a hex aura color string (#rrggbb) to linear RGBA.
fn aura_color_to_rgba(hex: &str) -> [f32; 4] {
    let hex = hex.trim_start_matches('#');
    if hex.len() < 6 {
        return civilization_color(0);
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(127) as f32 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(127) as f32 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(127) as f32 / 255.0;
    [r, g, b, 1.0]
}

/// Compute the bounding sphere radius from a list of 3-D positions.
fn bounding_radius(positions: &[(f32, f32, f32)]) -> f32 {
    if positions.is_empty() {
        return 0.0;
    }
    let n = positions.len() as f32;
    let cx = positions.iter().map(|p| p.0).sum::<f32>() / n;
    let cy = positions.iter().map(|p| p.1).sum::<f32>() / n;
    let cz = positions.iter().map(|p| p.2).sum::<f32>() / n;
    positions
        .iter()
        .map(|p| {
            let dx = p.0 - cx;
            let dy = p.1 - cy;
            let dz = p.2 - cz;
            (dx * dx + dy * dy + dz * dz).sqrt()
        })
        .fold(0.0_f32, f32::max)
}
