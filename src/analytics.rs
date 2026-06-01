// Phase 12: Advanced Graph Analytics
// PageRank influence scoring, betweenness centrality, bridge detection,
// and a composite graph health report.

use crate::workspace::SilentNodeWorkspace;
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;

// ── Output types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PageRankEntry {
    pub node_id: Uuid,
    pub score: f64,
    pub content_preview: String,
}

#[derive(Debug, Clone)]
pub struct CentralityEntry {
    pub node_id: Uuid,
    pub betweenness: f64,
    pub content_preview: String,
}

#[derive(Debug, Clone)]
pub struct BridgeEdge {
    pub source_id: Uuid,
    pub target_id: Uuid,
    pub weight: f32,
    pub source_preview: String,
    pub target_preview: String,
}

#[derive(Debug, Clone)]
pub struct GraphHealthReport {
    /// Overall health score 0..1 (higher = healthier).
    pub score: f64,
    pub avg_entropy: f64,
    /// Ratio of edges to max possible (density).
    pub density: f64,
    /// Fraction of nodes accessed in last 30 days.
    pub activity_rate: f64,
    /// Fraction of nodes that are ghost or fossil.
    pub decay_ratio: f64,
    /// Number of weakly-connected components.
    pub component_count: usize,
    pub node_count: usize,
    pub edge_count: usize,
    pub bridge_count: usize,
}

impl GraphHealthReport {
    pub fn summary(&self) -> String {
        let label = if self.score > 0.75 {
            "Thriving"
        } else if self.score > 0.5 {
            "Healthy"
        } else if self.score > 0.25 {
            "Fragile"
        } else {
            "Critical"
        };
        format!(
            "{label} (score={:.3}): {} nodes / {} edges  density={:.3}  activity={:.0}%  components={}",
            self.score,
            self.node_count,
            self.edge_count,
            self.density,
            self.activity_rate * 100.0,
            self.component_count,
        )
    }
}

// ── Engine ────────────────────────────────────────────────────────────────────

pub struct AnalyticsEngine;

impl AnalyticsEngine {
    pub fn new() -> Self {
        Self
    }

    // ── PageRank ─────────────────────────────────────────────────────────────

    pub fn pagerank(&self, workspace: &SilentNodeWorkspace, top_n: usize) -> Vec<PageRankEntry> {
        let ids: Vec<Uuid> = workspace.graph.node_ids();
        let n = ids.len();
        if n == 0 {
            return vec![];
        }

        let id_to_idx: HashMap<Uuid, usize> =
            ids.iter().enumerate().map(|(i, &id)| (id, i)).collect();

        let mut successors: Vec<Vec<usize>> = vec![vec![]; n];
        for edge in workspace.graph.edges() {
            if let (Some(&si), Some(&ti)) = (
                id_to_idx.get(&edge.source_id),
                id_to_idx.get(&edge.target_id),
            ) {
                successors[si].push(ti);
            }
        }

        let d = 0.85_f64;
        let base = (1.0 - d) / n as f64;
        let mut rank: Vec<f64> = vec![1.0 / n as f64; n];

        for _ in 0..50 {
            let mut new_rank: Vec<f64> = vec![base; n];
            // dangling nodes (no out-edges) contribute rank / n to everyone
            let dangling_sum: f64 = rank
                .iter()
                .enumerate()
                .filter(|(i, _)| successors[*i].is_empty())
                .map(|(_, r)| r * d / n as f64)
                .sum();
            for val in new_rank.iter_mut() {
                *val += dangling_sum;
            }
            for i in 0..n {
                if !successors[i].is_empty() {
                    let contrib = d * rank[i] / successors[i].len() as f64;
                    for &j in &successors[i] {
                        new_rank[j] += contrib;
                    }
                }
            }
            rank = new_rank;
        }

        let mut entries: Vec<PageRankEntry> = ids
            .iter()
            .zip(rank.iter())
            .map(|(&id, &score)| {
                let preview = workspace
                    .graph
                    .get_node(id)
                    .map(|nd| {
                        if nd.content.len() > 30 {
                            format!("{}…", &nd.content[..29])
                        } else {
                            nd.content.clone()
                        }
                    })
                    .unwrap_or_default();
                PageRankEntry {
                    node_id: id,
                    score,
                    content_preview: preview,
                }
            })
            .collect();

        entries.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        entries.truncate(top_n);
        entries
    }

    // ── Betweenness Centrality (Brandes, unweighted BFS) ─────────────────────

    pub fn betweenness(
        &self,
        workspace: &SilentNodeWorkspace,
        top_n: usize,
    ) -> Vec<CentralityEntry> {
        let ids: Vec<Uuid> = workspace.graph.node_ids();
        let n = ids.len();
        if n == 0 {
            return vec![];
        }

        let id_to_idx: HashMap<Uuid, usize> =
            ids.iter().enumerate().map(|(i, &id)| (id, i)).collect();

        // undirected adjacency
        let mut adj: Vec<Vec<usize>> = vec![vec![]; n];
        for edge in workspace.graph.edges() {
            if let (Some(&si), Some(&ti)) = (
                id_to_idx.get(&edge.source_id),
                id_to_idx.get(&edge.target_id),
            ) {
                adj[si].push(ti);
                adj[ti].push(si);
            }
        }

        let mut centrality: Vec<f64> = vec![0.0; n];
        let limit = n.min(80); // sample cap for large graphs

        for s in 0..limit {
            let mut stack: Vec<usize> = Vec::new();
            let mut pred: Vec<Vec<usize>> = vec![vec![]; n];
            let mut sigma: Vec<f64> = vec![0.0; n];
            sigma[s] = 1.0;
            let mut dist: Vec<i64> = vec![-1; n];
            dist[s] = 0;
            let mut queue: VecDeque<usize> = VecDeque::new();
            queue.push_back(s);

            while let Some(v) = queue.pop_front() {
                stack.push(v);
                for &w in &adj[v] {
                    if dist[w] < 0 {
                        queue.push_back(w);
                        dist[w] = dist[v] + 1;
                    }
                    if dist[w] == dist[v] + 1 {
                        sigma[w] += sigma[v];
                        pred[w].push(v);
                    }
                }
            }

            let mut delta: Vec<f64> = vec![0.0; n];
            while let Some(w) = stack.pop() {
                for &v in &pred[w] {
                    if sigma[w] > 0.0 {
                        delta[v] += (sigma[v] / sigma[w]) * (1.0 + delta[w]);
                    }
                }
                if w != s {
                    centrality[w] += delta[w];
                }
            }
        }

        // normalize to [0, 1]
        let norm = if n > 2 {
            ((n - 1) * (n - 2)) as f64
        } else {
            1.0
        };
        for c in &mut centrality {
            *c /= norm;
        }

        let mut entries: Vec<CentralityEntry> = ids
            .iter()
            .zip(centrality.iter())
            .map(|(&id, &bc)| {
                let preview = workspace
                    .graph
                    .get_node(id)
                    .map(|nd| {
                        if nd.content.len() > 30 {
                            format!("{}…", &nd.content[..29])
                        } else {
                            nd.content.clone()
                        }
                    })
                    .unwrap_or_default();
                CentralityEntry {
                    node_id: id,
                    betweenness: bc,
                    content_preview: preview,
                }
            })
            .collect();

        entries.sort_by(|a, b| {
            b.betweenness
                .partial_cmp(&a.betweenness)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        entries.truncate(top_n);
        entries
    }

    // ── Bridge Detection (Tarjan, iterative) ──────────────────────────────────

    pub fn find_bridges(&self, workspace: &SilentNodeWorkspace) -> Vec<BridgeEdge> {
        let ids: Vec<Uuid> = workspace.graph.node_ids();
        let n = ids.len();
        if n == 0 {
            return vec![];
        }

        let id_to_idx: HashMap<Uuid, usize> =
            ids.iter().enumerate().map(|(i, &id)| (id, i)).collect();

        let mut adj: Vec<Vec<usize>> = vec![vec![]; n];
        for edge in workspace.graph.edges() {
            if let (Some(&si), Some(&ti)) = (
                id_to_idx.get(&edge.source_id),
                id_to_idx.get(&edge.target_id),
            ) {
                adj[si].push(ti);
                adj[ti].push(si);
            }
        }

        let mut disc: Vec<i32> = vec![-1; n];
        let mut low: Vec<i32> = vec![0; n];
        let mut bridges_set: HashSet<(usize, usize)> = HashSet::new();
        let mut timer = 0i32;

        for start in 0..n {
            if disc[start] == -1 {
                bridge_dfs(
                    start,
                    usize::MAX,
                    &adj,
                    &mut disc,
                    &mut low,
                    &mut timer,
                    &mut bridges_set,
                );
            }
        }

        let mut result: Vec<BridgeEdge> = Vec::new();
        for edge in workspace.graph.edges() {
            if let (Some(&si), Some(&ti)) = (
                id_to_idx.get(&edge.source_id),
                id_to_idx.get(&edge.target_id),
            ) {
                if bridges_set.contains(&(si.min(ti), si.max(ti))) {
                    let src_preview = workspace
                        .graph
                        .get_node(edge.source_id)
                        .map(|nd| {
                            if nd.content.len() > 22 {
                                format!("{}…", &nd.content[..21])
                            } else {
                                nd.content.clone()
                            }
                        })
                        .unwrap_or_default();
                    let tgt_preview = workspace
                        .graph
                        .get_node(edge.target_id)
                        .map(|nd| {
                            if nd.content.len() > 22 {
                                format!("{}…", &nd.content[..21])
                            } else {
                                nd.content.clone()
                            }
                        })
                        .unwrap_or_default();
                    result.push(BridgeEdge {
                        source_id: edge.source_id,
                        target_id: edge.target_id,
                        weight: edge.weight,
                        source_preview: src_preview,
                        target_preview: tgt_preview,
                    });
                    bridges_set.remove(&(si.min(ti), si.max(ti)));
                }
            }
        }

        result
    }

    // ── Graph Health ──────────────────────────────────────────────────────────

    pub fn health_report(&self, workspace: &SilentNodeWorkspace) -> GraphHealthReport {
        let node_count = workspace.graph.node_count();
        let edge_count = workspace.graph.edge_count();

        if node_count == 0 {
            return GraphHealthReport {
                score: 0.0,
                avg_entropy: 0.0,
                density: 0.0,
                activity_rate: 0.0,
                decay_ratio: 0.0,
                component_count: 0,
                node_count: 0,
                edge_count: 0,
                bridge_count: 0,
            };
        }

        let avg_entropy = workspace
            .graph
            .nodes()
            .map(|nd| nd.entropy as f64)
            .sum::<f64>()
            / node_count as f64;

        let density = if node_count > 1 {
            edge_count as f64 / (node_count as f64 * (node_count as f64 - 1.0))
        } else {
            0.0
        };

        let thirty_days_ago =
            chrono::Utc::now() - chrono::Duration::try_days(30).unwrap_or_default();
        let active = workspace
            .graph
            .nodes()
            .filter(|nd| nd.accessed_at > thirty_days_ago)
            .count();
        let activity_rate = active as f64 / node_count as f64;

        let decay = workspace
            .graph
            .nodes()
            .filter(|nd| nd.is_ghost || nd.is_fossil)
            .count();
        let decay_ratio = decay as f64 / node_count as f64;

        // weakly-connected components via BFS on undirected adjacency
        let ids: Vec<Uuid> = workspace.graph.node_ids();
        let id_to_idx: HashMap<Uuid, usize> =
            ids.iter().enumerate().map(|(i, &id)| (id, i)).collect();
        let mut adj: Vec<Vec<usize>> = vec![vec![]; ids.len()];
        for edge in workspace.graph.edges() {
            if let (Some(&si), Some(&ti)) = (
                id_to_idx.get(&edge.source_id),
                id_to_idx.get(&edge.target_id),
            ) {
                adj[si].push(ti);
                adj[ti].push(si);
            }
        }
        let mut visited = vec![false; ids.len()];
        let mut component_count = 0usize;
        for start in 0..ids.len() {
            if !visited[start] {
                component_count += 1;
                let mut queue = VecDeque::from([start]);
                visited[start] = true;
                while let Some(v) = queue.pop_front() {
                    for &w in &adj[v] {
                        if !visited[w] {
                            visited[w] = true;
                            queue.push_back(w);
                        }
                    }
                }
            }
        }

        let bridge_count = self.find_bridges(workspace).len();

        // composite health score
        let entropy_ok = (1.0 - avg_entropy).clamp(0.0, 1.0);
        let density_ok = (density * 6.0).clamp(0.0, 1.0);
        let activity_ok = activity_rate;
        let decay_ok = (1.0 - decay_ratio).clamp(0.0, 1.0);
        let conn_ok = if node_count == 0 {
            0.0
        } else {
            1.0 - ((component_count.saturating_sub(1)) as f64 / node_count as f64).clamp(0.0, 1.0)
        };
        let score = (entropy_ok * 0.20
            + density_ok * 0.20
            + activity_ok * 0.25
            + decay_ok * 0.15
            + conn_ok * 0.20)
            .clamp(0.0, 1.0);

        GraphHealthReport {
            score,
            avg_entropy,
            density,
            activity_rate,
            decay_ratio,
            component_count,
            node_count,
            edge_count,
            bridge_count,
        }
    }
}

impl Default for AnalyticsEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tarjan bridge DFS (recursive, safe for graph sizes used here) ─────────────

fn bridge_dfs(
    u: usize,
    parent: usize,
    adj: &[Vec<usize>],
    disc: &mut Vec<i32>,
    low: &mut Vec<i32>,
    timer: &mut i32,
    bridges: &mut HashSet<(usize, usize)>,
) {
    disc[u] = *timer;
    low[u] = *timer;
    *timer += 1;

    for &v in &adj[u] {
        if disc[v] == -1 {
            bridge_dfs(v, u, adj, disc, low, timer, bridges);
            low[u] = low[u].min(low[v]);
            if low[v] > disc[u] {
                bridges.insert((u.min(v), u.max(v)));
            }
        } else if v != parent {
            low[u] = low[u].min(disc[v]);
        }
    }
}
