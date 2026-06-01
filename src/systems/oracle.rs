use crate::domain::{FocusEvent, JournalEntry, NodeData};
use crate::systems::seasons::{CognitiveSeason, CognitiveSeasonDetector};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OracleSignalKind {
    /// A node is likely to be visited next based on temporal pattern.
    NodeAnticipation { node_id: Uuid, confidence: f32 },
    /// A ghost node's content still resonates — it may re-emerge soon.
    GhostReturn { node_id: Uuid, days_inactive: f32 },
    /// The current cognitive season is shifting toward a new state.
    SeasonTransition { incoming: CognitiveSeason },
    /// Two nodes share deep semantic similarity across different clusters.
    HighResonancePair { node_a: Uuid, node_b: Uuid },
    /// A node is approaching critical entropy — it may become a ghost soon.
    /// Vision.md: "entropy prediction — identifying which entities are
    /// approaching critical decay before the visual changes are obvious."
    EntropyWarning {
        node_id: Uuid,
        current_entropy: f32,
        estimated_days_to_ghost: f32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleSignal {
    pub id: Uuid,
    pub kind: OracleSignalKind,
    pub strength: f32,
    pub generated_at: DateTime<Utc>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleLayer {
    pub history_days: u32,
}

impl Default for OracleLayer {
    fn default() -> Self {
        Self { history_days: 30 }
    }
}

impl OracleLayer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn generate_signals(
        &self,
        nodes: &[&NodeData],
        focus_events: &[FocusEvent],
        journal_entries: &[JournalEntry],
        now: DateTime<Utc>,
    ) -> Vec<OracleSignal> {
        let mut signals: Vec<OracleSignal> = Vec::new();

        let history_start = now - Duration::days(self.history_days as i64);

        // ── 1. Node Anticipation ────────────────────────────────────────────────
        // Group events by node_id, sorted by timestamp
        let mut events_by_node: HashMap<Uuid, Vec<&FocusEvent>> = HashMap::new();
        for e in focus_events.iter().filter(|e| e.timestamp >= history_start) {
            events_by_node.entry(e.node_id).or_default().push(e);
        }
        for (node_id, mut visits) in events_by_node {
            visits.sort_by_key(|e| e.timestamp);
            if visits.len() < 3 {
                continue;
            }
            // Compute gaps between last 3 visits
            let last_three = &visits[visits.len().saturating_sub(3)..];
            let gaps: Vec<f32> = last_three
                .windows(2)
                .map(|w| (w[1].timestamp - w[0].timestamp).num_seconds().abs() as f32 / 3600.0)
                .collect();
            let median_gap = if gaps.is_empty() {
                continue;
            } else if gaps.len() == 1 {
                gaps[0]
            } else {
                let mut gs = gaps.clone();
                gs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                gs[gs.len() / 2]
            };

            if median_gap <= 0.0 {
                continue;
            }

            let last_visit = last_three.last().map(|e| e.timestamp).unwrap_or(now);
            let hours_since_last = (now - last_visit).num_seconds().max(0) as f32 / 3600.0;

            // Due if hours_since_last is between 1x and 3x the median_gap
            if hours_since_last >= median_gap && hours_since_last < 3.0 * median_gap {
                let confidence =
                    (1.0 - (hours_since_last - median_gap) / (2.0 * median_gap)).clamp(0.0, 1.0);
                signals.push(OracleSignal {
                    id: Uuid::new_v4(),
                    strength: confidence,
                    generated_at: now,
                    description: format!(
                        "Node {} due for a visit (median gap {:.1}h, last seen {:.1}h ago)",
                        node_id, median_gap, hours_since_last
                    ),
                    kind: OracleSignalKind::NodeAnticipation {
                        node_id,
                        confidence,
                    },
                });
            }
        }

        // ── 2. Ghost Return ─────────────────────────────────────────────────────
        for node in nodes.iter().filter(|n| n.is_ghost) {
            let days_inactive = (now - node.accessed_at).num_seconds().max(0) as f32 / 86400.0;
            // Must have had meaningful access history
            if node.access_count < 3 {
                continue;
            }
            // Entropy dropping is proxied as entropy < 0.5 (low/medium — not still climbing)
            if node.entropy < 0.5 {
                let strength = (node.access_count as f32 / 10.0).clamp(0.0, 1.0);
                signals.push(OracleSignal {
                    id: Uuid::new_v4(),
                    strength,
                    generated_at: now,
                    description: format!(
                        "Ghost node {} may be ready to return (inactive {:.1} days, {} prior accesses)",
                        node.id, days_inactive, node.access_count
                    ),
                    kind: OracleSignalKind::GhostReturn {
                        node_id: node.id,
                        days_inactive,
                    },
                });
            }
        }

        // ── 3. Season Transition ────────────────────────────────────────────────
        {
            let detector = CognitiveSeasonDetector::new();
            let report = detector.detect_season(nodes, focus_events, journal_entries, now);
            let current_season = report.season;

            // Find when the current season started by scanning 21+ days back
            let check_point = now - Duration::days(21);
            let past_report =
                detector.detect_season(nodes, focus_events, journal_entries, check_point);

            if past_report.season == current_season {
                // Been in same season for > 21 days — check if metrics suggest shift
                let shift_score = match current_season {
                    CognitiveSeason::Spring => {
                        // Spring shifts to Summer if focus_density rises
                        (report.focus_density - 0.5).max(0.0) * 2.0
                    }
                    CognitiveSeason::Summer => {
                        // Summer shifts to Autumn if revisit_ratio rises
                        (report.revisit_ratio - 0.4).max(0.0) * 2.5
                    }
                    CognitiveSeason::Autumn => {
                        // Autumn shifts to Winter if everything drops
                        (0.3 - report.creation_rate).max(0.0)
                            + (0.3 - report.focus_density).max(0.0)
                    }
                    CognitiveSeason::Winter => {
                        // Winter shifts to Spring if creation picks up
                        (report.creation_rate - 0.25).max(0.0) * 3.0
                    }
                };

                if shift_score > 0.3 {
                    let incoming = next_season(current_season);
                    let strength = shift_score.clamp(0.0, 1.0);
                    signals.push(OracleSignal {
                        id: Uuid::new_v4(),
                        strength,
                        generated_at: now,
                        description: format!(
                            "Cognitive season transition approaching: {} → {}",
                            current_season.name(),
                            incoming.name()
                        ),
                        kind: OracleSignalKind::SeasonTransition { incoming },
                    });
                }
            }
        }

        // ── 4. High Resonance Pairs — TF-IDF semantic similarity ──────────────
        // Vision.md: "Resonance Chamber pre-detection — identifying structural
        // similarities before they reach chamber-formation threshold."
        // Pre-resonance window: similarity 0.18–0.42 (below chamber threshold).
        {
            let pre_resonance_pairs = find_pre_resonance_pairs(nodes, 0.18, 0.42);
            for (a, b, sim) in pre_resonance_pairs {
                let strength = (sim / 0.42).clamp(0.0, 1.0) * 0.7;
                signals.push(OracleSignal {
                    id: Uuid::new_v4(),
                    strength,
                    generated_at: now,
                    description: format!(
                        "Semantic pre-resonance between nodes {} and {} (similarity={:.2}) — \
                         a Resonance Chamber may form soon",
                        a, b, sim
                    ),
                    kind: OracleSignalKind::HighResonancePair {
                        node_a: a,
                        node_b: b,
                    },
                });
            }

            // Also use journal co-mention as a weaker corroborating signal
            let mut co_mentions: HashMap<(Uuid, Uuid), usize> = HashMap::new();
            for entry in journal_entries {
                let linked = &entry.linked_nodes;
                for i in 0..linked.len() {
                    for j in (i + 1)..linked.len() {
                        let key = order_pair(linked[i], linked[j]);
                        *co_mentions.entry(key).or_insert(0) += 1;
                    }
                }
            }
            for ((a, b), count) in &co_mentions {
                if *count > 3 {
                    // Only emit if not already covered by semantic signal
                    let already_present = signals.iter().any(|s| {
                        matches!(
                            &s.kind,
                            OracleSignalKind::HighResonancePair { node_a, node_b }
                            if (*node_a == *a && *node_b == *b) || (*node_a == *b && *node_b == *a)
                        )
                    });
                    if !already_present {
                        let strength = (*count as f32 / 12.0).clamp(0.0, 0.6);
                        signals.push(OracleSignal {
                            id: Uuid::new_v4(),
                            strength,
                            generated_at: now,
                            description: format!(
                                "Journal co-mention resonance: nodes {} and {} mentioned together \
                                 {} times",
                                a, b, count
                            ),
                            kind: OracleSignalKind::HighResonancePair {
                                node_a: *a,
                                node_b: *b,
                            },
                        });
                    }
                }
            }
        }

        // ── 5. Entropy Warning ──────────────────────────────────────────────────
        // Vision.md: "entropy prediction — identifying which entities are
        // approaching critical decay before the visual changes are obvious."
        // Ghost threshold = 0.92. Warn when entropy is rising into 0.65–0.92 range.
        {
            let ghost_threshold = 0.92_f32;
            for node in nodes
                .iter()
                .filter(|n| !n.is_ghost && !n.is_fossil && !n.is_void)
            {
                let entropy = node.entropy;
                if entropy < 0.55 || entropy >= ghost_threshold {
                    continue;
                }
                // Estimate days until entropy reaches ghost threshold
                // entropy = 1 - exp(-k*t) → t = -ln(1-entropy)/k
                // We don't know k, but we can estimate rate from recent access pattern.
                let days_since = (now - node.accessed_at).num_seconds().max(0) as f32 / 86400.0;
                if days_since < 1.0 {
                    continue; // Recently accessed, not at risk
                }
                // Approximate k from observed entropy growth
                let k = if entropy > 0.0 && days_since > 0.0 {
                    (-((1.0 - entropy).ln())) / days_since.max(0.001)
                } else {
                    0.02 // default decay rate
                };
                let days_to_ghost = if k > 0.0 {
                    (-((1.0 - ghost_threshold).ln())) / k - days_since
                } else {
                    f32::MAX
                };
                if days_to_ghost < 0.0 || days_to_ghost > 30.0 {
                    continue; // Too far or already past
                }
                let urgency = 1.0 - (days_to_ghost / 30.0).clamp(0.0, 1.0);
                let strength = urgency * (entropy - 0.55) / (ghost_threshold - 0.55);
                signals.push(OracleSignal {
                    id: Uuid::new_v4(),
                    strength: strength.clamp(0.0, 1.0),
                    generated_at: now,
                    description: format!(
                        "Node {} (entropy={:.2}) approaching ghost threshold — \
                         estimated {:.1} days remaining",
                        node.id,
                        entropy,
                        days_to_ghost.max(0.0)
                    ),
                    kind: OracleSignalKind::EntropyWarning {
                        node_id: node.id,
                        current_entropy: entropy,
                        estimated_days_to_ghost: days_to_ghost.max(0.0),
                    },
                });
            }
        }

        // Sort by strength descending
        signals.sort_by(|a, b| {
            b.strength
                .partial_cmp(&a.strength)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        signals
    }
}

// ── Pre-resonance pair detection (TF-IDF) ────────────────────────────────────

/// Find node pairs with semantic similarity in [min_sim, max_sim).
/// This is the "pre-resonance" window — below formal chamber threshold.
fn find_pre_resonance_pairs(
    nodes: &[&NodeData],
    min_sim: f32,
    max_sim: f32,
) -> Vec<(Uuid, Uuid, f32)> {
    if nodes.len() < 2 {
        return Vec::new();
    }

    let tokenized: Vec<Vec<String>> = nodes.iter().map(|n| tokenize_oracle(&n.content)).collect();
    let n = nodes.len();

    // Document frequency
    let mut df: HashMap<String, usize> = HashMap::new();
    for tokens in &tokenized {
        let unique: std::collections::HashSet<&String> = tokens.iter().collect();
        for t in unique {
            *df.entry(t.clone()).or_insert(0) += 1;
        }
    }

    // TF-IDF vectors
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
                    let df_val = *df.get(&term).unwrap_or(&1) as f32;
                    let idf = ((n as f32 + 1.0) / (df_val + 1.0)).ln() + 1.0;
                    (term, tf_val * idf)
                })
                .collect()
        })
        .collect();

    let mut pairs: Vec<(Uuid, Uuid, f32)> = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            // Skip same civilization
            let same_civ = nodes[i].civilization_id.is_some()
                && nodes[i].civilization_id == nodes[j].civilization_id;
            if same_civ {
                continue;
            }
            let sim = cosine_sim_oracle(&vectors[i], &vectors[j]);
            if sim >= min_sim && sim < max_sim {
                pairs.push((nodes[i].id, nodes[j].id, sim));
            }
        }
    }
    pairs.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    pairs.truncate(8);
    pairs
}

static ORACLE_STOP_WORDS: &[&str] = &[
    "the", "a", "an", "is", "in", "of", "and", "or", "to", "it", "that", "this", "with", "for",
    "on", "at", "by", "from", "as", "be", "was", "are", "were", "has", "have", "had", "not", "but",
    "so", "if", "then", "i", "you", "we",
];

fn tokenize_oracle(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|w| {
            w.chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
                .to_lowercase()
        })
        .filter(|w| w.len() > 2 && !ORACLE_STOP_WORDS.contains(&w.as_str()))
        .collect()
}

fn cosine_sim_oracle(a: &HashMap<String, f32>, b: &HashMap<String, f32>) -> f32 {
    let dot: f32 = a
        .iter()
        .filter_map(|(t, &v)| b.get(t).map(|&bv| v * bv))
        .sum();
    let ma = a.values().map(|v| v * v).sum::<f32>().sqrt();
    let mb = b.values().map(|v| v * v).sum::<f32>().sqrt();
    if ma <= 0.0 || mb <= 0.0 {
        0.0
    } else {
        (dot / (ma * mb)).clamp(0.0, 1.0)
    }
}

fn next_season(season: CognitiveSeason) -> CognitiveSeason {
    match season {
        CognitiveSeason::Spring => CognitiveSeason::Summer,
        CognitiveSeason::Summer => CognitiveSeason::Autumn,
        CognitiveSeason::Autumn => CognitiveSeason::Winter,
        CognitiveSeason::Winter => CognitiveSeason::Spring,
    }
}

fn order_pair(a: Uuid, b: Uuid) -> (Uuid, Uuid) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}
