use crate::domain::NodeData;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResonancePair {
    pub node_a: Uuid,
    pub node_b: Uuid,
    pub similarity: f32,
    pub same_civilization: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResonanceChamberEngine {
    pub min_similarity: f32,
    pub cross_civ_only: bool,
}

impl Default for ResonanceChamberEngine {
    fn default() -> Self {
        Self {
            min_similarity: 0.35,
            cross_civ_only: false,
        }
    }
}

impl ResonanceChamberEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build TF-IDF vectors for each node and compute pairwise cosine similarity.
    pub fn find_resonances(&self, nodes: &[&NodeData]) -> Vec<ResonancePair> {
        if nodes.len() < 2 {
            return Vec::new();
        }

        // Tokenize content for each node
        let tokenized: Vec<Vec<String>> = nodes.iter().map(|n| tokenize(&n.content)).collect();

        let n = nodes.len();

        // Collect vocabulary and document frequencies
        let mut df: HashMap<String, usize> = HashMap::new();
        for tokens in &tokenized {
            let unique: std::collections::HashSet<&String> = tokens.iter().collect();
            for term in unique {
                *df.entry(term.clone()).or_insert(0) += 1;
            }
        }

        let n_docs = n as f32;

        // Build TF-IDF vectors
        let vectors: Vec<HashMap<String, f32>> = tokenized
            .iter()
            .map(|tokens| {
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
            })
            .collect();

        // Compute cosine similarities for all pairs
        let mut pairs: Vec<ResonancePair> = Vec::new();

        for i in 0..n {
            for j in (i + 1)..n {
                let sim = cosine_similarity(&vectors[i], &vectors[j]);
                if sim < self.min_similarity {
                    continue;
                }
                let same_civ = nodes[i].civilization_id.is_some()
                    && nodes[i].civilization_id == nodes[j].civilization_id;

                if self.cross_civ_only && same_civ {
                    continue;
                }

                pairs.push(ResonancePair {
                    node_a: nodes[i].id,
                    node_b: nodes[j].id,
                    similarity: sim,
                    same_civilization: same_civ,
                });
            }
        }

        pairs.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        pairs
    }
}

// ── Resonance Chamber lifecycle ───────────────────────────────────────────────

/// The lifecycle state of a resonance chamber.
///
/// Vision.md: "the chamber persists for a limited time before dissolving —
/// the resonance is offered, not imposed."
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChamberState {
    /// Open — awaiting user decision.
    Open,
    /// User accepted: a permanent edge has been created between node_a and node_b.
    Connected,
    /// User noted: resonance recorded as a lore entry; no edge created.
    Noted,
    /// User dismissed: chamber dissolved without record.
    Dismissed,
    /// Auto-expired after TTL without user interaction.
    Expired,
}

/// A temporary spatial event where two semantically resonant nodes from
/// different civilizations meet. The user can Connect, Note, or Dismiss.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResonanceChamber {
    pub id: Uuid,
    pub node_a: Uuid,
    pub node_b: Uuid,
    pub similarity: f32,
    pub same_civilization: bool,
    pub opened_at: DateTime<Utc>,
    pub state: ChamberState,
}

impl ResonanceChamber {
    pub fn open(pair: &ResonancePair) -> Self {
        Self {
            id: Uuid::new_v4(),
            node_a: pair.node_a,
            node_b: pair.node_b,
            similarity: pair.similarity,
            same_civilization: pair.same_civilization,
            opened_at: Utc::now(),
            state: ChamberState::Open,
        }
    }

    pub fn is_open(&self) -> bool {
        self.state == ChamberState::Open
    }

    pub fn accept(&mut self) {
        self.state = ChamberState::Connected;
    }
    pub fn note(&mut self) {
        self.state = ChamberState::Noted;
    }
    pub fn dismiss(&mut self) {
        self.state = ChamberState::Dismissed;
    }
    pub fn expire(&mut self) {
        self.state = ChamberState::Expired;
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
