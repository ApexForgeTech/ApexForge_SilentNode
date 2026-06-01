use crate::graph::CognitiveGraph;
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct MissingBridge {
    pub node_a: Uuid,
    pub node_b: Uuid,
    pub similarity: f32,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct ImpliedConcept {
    pub implied_by: Vec<Uuid>,
    pub suggested_content: String,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct SilenceAnalyzer {
    similarity_threshold: f32,
}

impl Default for SilenceAnalyzer {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.3,
        }
    }
}

impl SilenceAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_threshold(threshold: f32) -> Self {
        Self {
            similarity_threshold: threshold,
        }
    }

    pub fn find_missing_bridges(&self, graph: &CognitiveGraph) -> Vec<MissingBridge> {
        let nodes: Vec<_> = graph.nodes().collect();
        if nodes.len() < 2 {
            return Vec::new();
        }

        // Build TF-IDF vectors for each node
        let documents: Vec<Vec<String>> = nodes.iter().map(|n| tokenize(&n.content)).collect();

        let tfidf_vectors = compute_tfidf(&documents);

        // Find connected components (bidirectional BFS)
        let components = find_components(graph);

        let mut bridges = Vec::new();
        let n = nodes.len();

        for i in 0..n {
            for j in (i + 1)..n {
                let id_a = nodes[i].id;
                let id_b = nodes[j].id;

                // Skip if in the same component (already reachable from each other)
                if components.get(&id_a) == components.get(&id_b) {
                    continue;
                }

                // Skip if there is already a direct edge in either direction
                if graph.get_edge(id_a, id_b).is_some() || graph.get_edge(id_b, id_a).is_some() {
                    continue;
                }

                let sim = cosine_similarity(&tfidf_vectors[i], &tfidf_vectors[j]);
                if sim >= self.similarity_threshold {
                    let reason = format!(
                        "Nodes share semantic content (similarity {:.2}) but belong to disconnected clusters",
                        sim
                    );
                    bridges.push(MissingBridge {
                        node_a: id_a,
                        node_b: id_b,
                        similarity: sim,
                        reason,
                    });
                }
            }
        }

        // Sort by descending similarity so the strongest suggestions come first
        bridges.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
        bridges
    }

    pub fn find_implied_concepts(&self, graph: &CognitiveGraph) -> Vec<ImpliedConcept> {
        let nodes: Vec<_> = graph.nodes().collect();
        if nodes.len() < 2 {
            return Vec::new();
        }

        let documents: Vec<Vec<String>> = nodes.iter().map(|n| tokenize(&n.content)).collect();

        let tfidf_vectors = compute_tfidf(&documents);
        let mut implied: Vec<ImpliedConcept> = Vec::new();

        let n = nodes.len();
        for i in 0..n {
            for j in (i + 1)..n {
                let sim = cosine_similarity(&tfidf_vectors[i], &tfidf_vectors[j]);
                if sim < self.similarity_threshold {
                    continue;
                }

                // Find terms present in one but not the other — these are the "gap"
                let tokens_i: HashSet<&str> = tfidf_vectors[i].keys().map(|s| s.as_str()).collect();
                let tokens_j: HashSet<&str> = tfidf_vectors[j].keys().map(|s| s.as_str()).collect();
                let exclusive_i: Vec<&str> = tokens_i.difference(&tokens_j).copied().collect();
                let exclusive_j: Vec<&str> = tokens_j.difference(&tokens_i).copied().collect();

                if !exclusive_i.is_empty() && !exclusive_j.is_empty() {
                    let mut bridge_terms = exclusive_i;
                    bridge_terms.extend(exclusive_j);
                    bridge_terms.sort();
                    let suggested = bridge_terms.join(" + ");

                    implied.push(ImpliedConcept {
                        implied_by: vec![nodes[i].id, nodes[j].id],
                        suggested_content: suggested,
                        confidence: sim,
                    });
                }
            }
        }

        implied.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        implied
    }
}

// ─── TF-IDF helpers ──────────────────────────────────────────────────────────

fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() > 2)
        .map(|t| t.to_string())
        .collect()
}

fn compute_tfidf(documents: &[Vec<String>]) -> Vec<HashMap<String, f32>> {
    let n = documents.len() as f32;

    // Document frequencies
    let mut df: HashMap<&str, usize> = HashMap::new();
    for doc in documents {
        let unique: HashSet<&str> = doc.iter().map(|s| s.as_str()).collect();
        for term in unique {
            *df.entry(term).or_insert(0) += 1;
        }
    }

    documents
        .iter()
        .map(|doc| {
            if doc.is_empty() {
                return HashMap::new();
            }
            let total = doc.len() as f32;
            let mut tf: HashMap<&str, f32> = HashMap::new();
            for term in doc {
                *tf.entry(term.as_str()).or_insert(0.0) += 1.0;
            }
            tf.into_iter()
                .map(|(term, count)| {
                    let tf_val = count / total;
                    // Smoothed IDF: ln((N+1)/(df+1)) + 1 — always positive, even when
                    // a term appears in every document.
                    let df_val = *df.get(term).unwrap_or(&1) as f32;
                    let idf_val = ((n + 1.0) / (df_val + 1.0)).ln() + 1.0;
                    (term.to_string(), tf_val * idf_val)
                })
                .collect()
        })
        .collect()
}

fn cosine_similarity(a: &HashMap<String, f32>, b: &HashMap<String, f32>) -> f32 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let dot: f32 = a
        .iter()
        .filter_map(|(term, &va)| b.get(term).map(|&vb| va * vb))
        .sum();

    let norm_a: f32 = a.values().map(|v| v * v).sum::<f32>().sqrt();
    let norm_b: f32 = b.values().map(|v| v * v).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

// ─── Connected components via bidirectional BFS ───────────────────────────────

fn find_components(graph: &CognitiveGraph) -> HashMap<Uuid, usize> {
    let mut component_id: HashMap<Uuid, usize> = HashMap::new();
    let mut next_component = 0usize;

    for node in graph.nodes() {
        if component_id.contains_key(&node.id) {
            continue;
        }

        let mut queue: VecDeque<Uuid> = VecDeque::new();
        queue.push_back(node.id);
        component_id.insert(node.id, next_component);

        while let Some(current) = queue.pop_front() {
            // Outgoing neighbors
            let out: Vec<Uuid> = graph
                .neighbors(current)
                .unwrap_or_default()
                .into_iter()
                .map(|n| n.id)
                .collect();
            // Incoming neighbors
            let inc: Vec<Uuid> = graph
                .incoming_edges(current)
                .unwrap_or_default()
                .into_iter()
                .map(|e| e.source_id)
                .collect();

            for neighbor_id in out.into_iter().chain(inc) {
                if !component_id.contains_key(&neighbor_id) {
                    component_id.insert(neighbor_id, next_component);
                    queue.push_back(neighbor_id);
                }
            }
        }

        next_component += 1;
    }

    component_id
}
