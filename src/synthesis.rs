// Phase 13: Thought Synthesis Engine
// Synthesizes narratives from graph structure, finds knowledge gaps,
// and traces causal chains via BFS.

use crate::domain::EdgeType;
use crate::workspace::SilentNodeWorkspace;
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;

pub struct SynthesisEngine;

impl SynthesisEngine {
    pub fn new() -> Self {
        Self
    }

    /// Build a textual narrative about `query` by ranking nodes via TF-IDF overlap.
    pub fn synthesize_topic(&self, workspace: &SilentNodeWorkspace, query: &str) -> String {
        let q_terms: HashSet<String> = tokenize(query);
        if q_terms.is_empty() {
            return "Query is empty — nothing to synthesize.".into();
        }

        // Score each node: TF-IDF overlap with query + gravity bonus
        let mut scored: Vec<(f32, Uuid)> = workspace
            .graph
            .nodes()
            .map(|n| {
                let node_terms: HashSet<String> = tokenize(&n.content);
                let overlap = q_terms.intersection(&node_terms).count() as f32;
                let score = overlap / (q_terms.len() as f32).max(1.0) + n.gravity * 0.05;
                (score, n.id)
            })
            .filter(|(s, _)| *s > 0.0)
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(8);

        if scored.is_empty() {
            return format!("No nodes found relating to «{query}».");
        }

        let mut parts: Vec<String> = Vec::new();
        parts.push(format!("Synthesis for «{query}»:\n"));

        for (rank, (score, id)) in scored.iter().enumerate() {
            let node = match workspace.graph.get_node(*id) {
                Some(n) => n,
                None => continue,
            };
            let label = if node.content.len() > 40 {
                &node.content[..40]
            } else {
                &node.content
            };
            parts.push(format!(
                "  {}. {} (relevance={:.2}  entropy={:.2})",
                rank + 1,
                label,
                score,
                node.entropy
            ));

            // List outgoing connections for context
            let out_edges = workspace.graph.outgoing_edges(*id).unwrap_or_default();
            for edge in out_edges.iter().take(3) {
                if let Some(tgt) = workspace.graph.get_node(edge.target_id) {
                    let tl = if tgt.content.len() > 28 {
                        &tgt.content[..28]
                    } else {
                        tgt.content.as_str()
                    };
                    parts.push(format!("     → [{:?}] {}", edge.edge_type, tl));
                }
            }
        }

        // Closing remark
        let total = workspace.graph.node_count();
        parts.push(format!(
            "\n  Graph has {total} nodes — {} matched this query.",
            scored.len()
        ));
        parts.join("\n")
    }

    /// Return terms from `topic` that appear in no node's content.
    pub fn find_knowledge_gaps(&self, workspace: &SilentNodeWorkspace, topic: &str) -> Vec<String> {
        let query_terms: Vec<String> = tokenize(topic).into_iter().collect();
        if query_terms.is_empty() {
            return vec![];
        }

        // Collect all terms present in the graph
        let mut graph_terms: HashSet<String> = HashSet::new();
        for node in workspace.graph.nodes() {
            for t in tokenize(&node.content) {
                graph_terms.insert(t);
            }
        }

        let mut gaps: Vec<String> = query_terms
            .into_iter()
            .filter(|t| !graph_terms.contains(t))
            .collect();
        gaps.sort();
        gaps.dedup();
        gaps
    }

    /// BFS path from `from` to `to`, preferring Causal edges.
    /// Returns `None` if no path exists.
    pub fn trace_causal_chain(
        &self,
        workspace: &SilentNodeWorkspace,
        from: Uuid,
        to: Uuid,
    ) -> Option<Vec<Uuid>> {
        if from == to {
            return Some(vec![from]);
        }

        let mut visited: HashSet<Uuid> = HashSet::new();
        let mut queue: VecDeque<Vec<Uuid>> = VecDeque::new();
        queue.push_back(vec![from]);
        visited.insert(from);

        while let Some(path) = queue.pop_front() {
            let current = *path.last().unwrap();

            // Collect outgoing edges, causal first
            let mut edges: Vec<(Uuid, bool)> = workspace
                .graph
                .outgoing_edges(current)
                .unwrap_or_default()
                .iter()
                .map(|e| (e.target_id, e.edge_type == EdgeType::Causal))
                .collect();
            edges.sort_by_key(|(_, is_causal)| if *is_causal { 0u8 } else { 1u8 });

            for (next, _) in edges {
                if visited.contains(&next) {
                    continue;
                }
                let mut new_path = path.clone();
                new_path.push(next);
                if next == to {
                    return Some(new_path);
                }
                visited.insert(next);
                queue.push_back(new_path);
            }
        }
        None
    }

    /// Return a formatted summary of the causal chain path.
    pub fn format_chain(&self, workspace: &SilentNodeWorkspace, path: &[Uuid]) -> String {
        let labels: Vec<String> = path
            .iter()
            .map(|&id| {
                workspace
                    .graph
                    .get_node(id)
                    .map(|n| {
                        if n.content.len() > 20 {
                            format!("{}…", &n.content[..20])
                        } else {
                            n.content.clone()
                        }
                    })
                    .unwrap_or_else(|| id.to_string())
            })
            .collect();
        labels.join("  →  ")
    }
}

impl Default for SynthesisEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ── Term frequency helpers ────────────────────────────────────────────────────

fn tokenize(text: &str) -> HashSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 3)
        .map(|t| t.to_lowercase())
        .collect()
}

/// Build a simple TF map for a document.
#[allow(dead_code)]
pub fn term_frequencies(text: &str) -> HashMap<String, f32> {
    let tokens: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 3)
        .map(|t| t.to_lowercase())
        .collect();
    let total = tokens.len() as f32;
    if total == 0.0 {
        return HashMap::new();
    }
    let mut counts: HashMap<String, f32> = HashMap::new();
    for t in tokens {
        *counts.entry(t).or_insert(0.0) += 1.0;
    }
    for v in counts.values_mut() {
        *v /= total;
    }
    counts
}
