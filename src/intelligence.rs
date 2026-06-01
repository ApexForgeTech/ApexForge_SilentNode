use crate::domain::NodeData;
use crate::workspace::SilentNodeWorkspace;
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

/// A node ranked by how urgently it deserves attention.
#[derive(Debug, Clone)]
pub struct FocusSuggestion {
    pub node_id: Uuid,
    pub score: f32,
    pub content_preview: String,
    pub reason: String,
}

/// A node that shares strong content similarity with a reference node.
#[derive(Debug, Clone)]
pub struct RelatedSuggestion {
    pub node_id: Uuid,
    pub similarity: f32,
    pub content_preview: String,
}

/// A group of nodes that share thematic content.
#[derive(Debug, Clone)]
pub struct ContentCluster {
    pub centroid_id: Uuid,
    pub member_ids: Vec<Uuid>,
    pub label: String,
}

pub struct SuggestionEngine;

impl SuggestionEngine {
    pub fn new() -> Self {
        Self
    }

    /// Rank all non-ghost, non-fossil nodes by neglect score:
    ///   neglect = time_since_access_hours / (1 + access_count * 0.1)
    ///   boosted by entropy and penalized by degree (well-connected nodes are less urgent).
    pub fn suggest_next_focus(
        &self,
        workspace: &SilentNodeWorkspace,
        limit: usize,
    ) -> Vec<FocusSuggestion> {
        let now = Utc::now();
        let mut scored: Vec<(f32, &NodeData, String)> = workspace
            .graph
            .nodes()
            .filter(|n| !n.is_ghost && !n.is_fossil && !n.is_void)
            .map(|n| {
                let hours_idle = (now - n.accessed_at).num_minutes() as f32 / 60.0;
                let neglect = hours_idle / (1.0 + n.access_count as f32 * 0.1);
                let degree_penalty = workspace.graph.degree(n.id) as f32 * 0.05;
                let score = neglect * (1.0 + n.entropy * 0.5) - degree_penalty;

                let reason = if n.entropy > 0.7 {
                    format!("high entropy ({:.2}) — needs attention", n.entropy)
                } else if hours_idle > 168.0 {
                    format!("not accessed in {:.0}h", hours_idle)
                } else if n.access_count == 0 {
                    "never focused".to_string()
                } else {
                    format!("idle {:.0}h, entropy {:.2}", hours_idle, n.entropy)
                };

                (score, n, reason)
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        scored
            .into_iter()
            .take(limit)
            .map(|(score, n, reason)| FocusSuggestion {
                node_id: n.id,
                score,
                content_preview: truncate(&n.content, 60),
                reason,
            })
            .collect()
    }

    /// Find nodes most similar to `node_id` using TF-IDF cosine similarity.
    pub fn suggest_related(
        &self,
        workspace: &SilentNodeWorkspace,
        node_id: Uuid,
        limit: usize,
    ) -> Vec<RelatedSuggestion> {
        let target = match workspace.graph.get_node(node_id) {
            Some(n) => n,
            None => return Vec::new(),
        };

        let nodes: Vec<&NodeData> = workspace
            .graph
            .nodes()
            .filter(|n| n.id != node_id)
            .collect();

        if nodes.is_empty() {
            return Vec::new();
        }

        // Build TF-IDF for all candidates + target together for shared IDF
        let mut all: Vec<&NodeData> = vec![target];
        all.extend_from_slice(&nodes);

        let tokenized: Vec<Vec<String>> = all.iter().map(|n| tokenize(&n.content)).collect();
        let n_docs = all.len() as f32;

        let mut df: HashMap<String, usize> = HashMap::new();
        for tokens in &tokenized {
            let unique: std::collections::HashSet<&String> = tokens.iter().collect();
            for term in unique {
                *df.entry(term.clone()).or_insert(0) += 1;
            }
        }

        let vectors: Vec<HashMap<String, f32>> = tokenized
            .iter()
            .map(|tokens| build_tfidf(tokens, &df, n_docs))
            .collect();

        let target_vec = &vectors[0];
        let mut results: Vec<RelatedSuggestion> = vectors[1..]
            .iter()
            .zip(nodes.iter())
            .map(|(vec, node)| {
                let sim = cosine_similarity(target_vec, vec);
                RelatedSuggestion {
                    node_id: node.id,
                    similarity: sim,
                    content_preview: truncate(&node.content, 60),
                }
            })
            .filter(|r| r.similarity > 0.01)
            .collect();

        results.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);
        results
    }

    /// Greedy k-means-style content clustering using TF-IDF centroids.
    /// Iterates until convergence or 20 iterations.
    pub fn cluster_content(
        &self,
        workspace: &SilentNodeWorkspace,
        k: usize,
    ) -> Vec<ContentCluster> {
        let nodes: Vec<&NodeData> = workspace.graph.nodes().collect();
        if nodes.len() < k || k == 0 {
            return nodes
                .into_iter()
                .enumerate()
                .map(|(_i, n)| ContentCluster {
                    centroid_id: n.id,
                    member_ids: vec![n.id],
                    label: truncate(&n.content, 30),
                })
                .take(k.max(1))
                .collect();
        }

        let tokenized: Vec<Vec<String>> = nodes.iter().map(|n| tokenize(&n.content)).collect();
        let n_docs = nodes.len() as f32;

        let mut df: HashMap<String, usize> = HashMap::new();
        for tokens in &tokenized {
            let unique: std::collections::HashSet<&String> = tokens.iter().collect();
            for term in unique {
                *df.entry(term.clone()).or_insert(0) += 1;
            }
        }

        let vectors: Vec<HashMap<String, f32>> = tokenized
            .iter()
            .map(|tokens| build_tfidf(tokens, &df, n_docs))
            .collect();

        // Seed centroids: evenly spaced by index (deterministic, no RNG dep)
        let step = nodes.len() / k;
        let mut centroid_indices: Vec<usize> = (0..k).map(|i| i * step).collect();

        let mut assignments = vec![0usize; nodes.len()];
        for _iter in 0..20 {
            // Assign each node to nearest centroid
            let mut changed = false;
            for (i, vec) in vectors.iter().enumerate() {
                let best = centroid_indices
                    .iter()
                    .enumerate()
                    .map(|(ci, &ci_idx)| (ci, cosine_similarity(vec, &vectors[ci_idx])))
                    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(ci, _)| ci)
                    .unwrap_or(0);

                if assignments[i] != best {
                    assignments[i] = best;
                    changed = true;
                }
            }

            // Update centroids: pick member with highest total similarity to others in cluster
            for ci in 0..k {
                let members: Vec<usize> = assignments
                    .iter()
                    .enumerate()
                    .filter(|(_, &a)| a == ci)
                    .map(|(i, _)| i)
                    .collect();

                if members.is_empty() {
                    continue;
                }

                let best_member = members
                    .iter()
                    .map(|&m| {
                        let total_sim: f32 = members
                            .iter()
                            .filter(|&&o| o != m)
                            .map(|&o| cosine_similarity(&vectors[m], &vectors[o]))
                            .sum();
                        (m, total_sim)
                    })
                    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(m, _)| m)
                    .unwrap_or(members[0]);

                centroid_indices[ci] = best_member;
            }

            if !changed {
                break;
            }
        }

        (0..k)
            .map(|ci| {
                let member_ids: Vec<Uuid> = assignments
                    .iter()
                    .enumerate()
                    .filter(|(_, &a)| a == ci)
                    .map(|(i, _)| nodes[i].id)
                    .collect();

                let centroid_id = nodes[centroid_indices[ci]].id;
                let label = truncate(&nodes[centroid_indices[ci]].content, 30);

                ContentCluster {
                    centroid_id,
                    member_ids,
                    label,
                }
            })
            .filter(|c| !c.member_ids.is_empty())
            .collect()
    }
}

impl Default for SuggestionEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

static STOP_WORDS: &[&str] = &[
    "the", "a", "an", "is", "in", "of", "and", "or", "to", "it", "its", "that", "this", "with",
    "for", "on", "at", "by", "from", "as", "be", "was", "are", "were", "has", "have", "had", "not",
    "but", "so", "if", "then", "than", "up", "do", "did", "will", "can", "may", "i", "you", "he",
    "she", "we", "they", "my", "your", "our", "their",
];

fn tokenize(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|w| {
            w.chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
                .to_lowercase()
        })
        .filter(|w| !w.is_empty() && !STOP_WORDS.contains(&w.as_str()) && w.len() > 1)
        .collect()
}

fn build_tfidf(
    tokens: &[String],
    df: &HashMap<String, usize>,
    n_docs: f32,
) -> HashMap<String, f32> {
    let total = tokens.len().max(1) as f32;
    let mut tf: HashMap<String, usize> = HashMap::new();
    for t in tokens {
        *tf.entry(t.clone()).or_insert(0) += 1;
    }
    tf.into_iter()
        .map(|(term, count)| {
            let tf_val = count as f32 / total;
            let df_val = df.get(&term).copied().unwrap_or(1) as f32;
            let idf = ((n_docs + 1.0) / (df_val + 1.0)).ln() + 1.0;
            (term, tf_val * idf)
        })
        .collect()
}

fn cosine_similarity(a: &HashMap<String, f32>, b: &HashMap<String, f32>) -> f32 {
    let dot: f32 = a
        .iter()
        .filter_map(|(term, &va)| b.get(term).map(|&vb| va * vb))
        .sum();
    let mag_a: f32 = a.values().map(|v| v * v).sum::<f32>().sqrt();
    let mag_b: f32 = b.values().map(|v| v * v).sum::<f32>().sqrt();
    if mag_a <= 0.0 || mag_b <= 0.0 {
        0.0
    } else {
        (dot / (mag_a * mag_b)).clamp(0.0, 1.0)
    }
}
