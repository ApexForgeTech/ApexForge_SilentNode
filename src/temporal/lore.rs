// Personal Lore System — narrative arc detection from temporal patterns.

use super::TemporalEngine;
use crate::domain::{ArcType, ChangeType, LoreEntry, NodeData};
use crate::systems::TectonicEvent;
use chrono::{DateTime, Utc};

/// Detects narrative arcs from temporal data and generates LoreEntry records.
pub struct LoreArcDetector {
    /// Minimum significance to include an arc (0..1).
    pub min_significance: f32,
}

impl Default for LoreArcDetector {
    fn default() -> Self {
        Self {
            min_significance: 0.25,
        }
    }
}

impl LoreArcDetector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Run all arc detectors and return discovered lore entries.
    pub fn detect_arcs(
        &self,
        nodes: &[&NodeData],
        engine: &TemporalEngine,
        tectonic_events: &[TectonicEvent],
        now: DateTime<Utc>,
    ) -> Vec<LoreEntry> {
        let mut lore = Vec::new();

        lore.extend(self.detect_origins(nodes, engine));
        lore.extend(self.detect_conflicts(nodes, engine));
        lore.extend(self.detect_resolutions(nodes, engine));
        lore.extend(self.detect_revelations(nodes, engine));
        lore.extend(self.detect_transformations(nodes, engine));
        lore.extend(self.detect_legacies(nodes, engine));
        lore.extend(self.detect_tectonic_lore(tectonic_events, now));

        lore.retain(|entry| entry.significance >= self.min_significance);
        lore.sort_by(|a, b| b.significance.total_cmp(&a.significance));
        lore
    }

    // ── Arc detectors ────────────────────────────────────────────────────────

    /// Origin arc: nodes born with many connections quickly (rapid crystallisation).
    fn detect_origins(&self, nodes: &[&NodeData], engine: &TemporalEngine) -> Vec<LoreEntry> {
        let mut lore = Vec::new();
        for node in nodes {
            let snaps = engine.snapshots_for(node.id);
            if snaps.len() < 2 {
                continue;
            }
            // Count Created + Connected events in first snapshot window
            let created_count = snaps
                .iter()
                .take(5)
                .filter(|s| matches!(s.change_type, ChangeType::Created | ChangeType::Connected))
                .count();
            if created_count < 2 {
                continue;
            }
            let significance = (created_count as f32 / 5.0).min(1.0) * node.gravity * 0.5;
            if significance < self.min_significance {
                continue;
            }
            lore.push(LoreEntry::new(
                format!("Origin of '{}'", node.content),
                ArcType::Origin,
                generate_origin_narrative(node, created_count),
                vec![node.id],
                significance,
            ));
        }
        lore
    }

    /// Conflict arc: nodes with high entropy + many StateChanged events.
    fn detect_conflicts(&self, nodes: &[&NodeData], engine: &TemporalEngine) -> Vec<LoreEntry> {
        let mut lore = Vec::new();
        for node in nodes {
            if node.entropy < 0.5 {
                continue;
            }
            let state_changes = engine
                .snapshots_for(node.id)
                .iter()
                .filter(|s| s.change_type == ChangeType::StateChanged)
                .count();
            if state_changes < 2 {
                continue;
            }
            let significance = (node.entropy * 0.6 + state_changes as f32 * 0.08).min(1.0);
            lore.push(LoreEntry::new(
                format!("Conflict within '{}'", node.content),
                ArcType::Conflict,
                generate_conflict_narrative(node, state_changes),
                vec![node.id],
                significance,
            ));
        }
        lore
    }

    /// Resolution arc: nodes that had high entropy but were touched and recovered.
    fn detect_resolutions(&self, nodes: &[&NodeData], engine: &TemporalEngine) -> Vec<LoreEntry> {
        let mut lore = Vec::new();
        for node in nodes {
            if node.entropy >= 0.3 {
                continue; // Still in conflict
            }
            let snaps = engine.snapshots_for(node.id);
            // Find if there was a prior high-entropy snapshot
            let peak_entropy = snaps
                .iter()
                .filter_map(|s| {
                    s.snapshot
                        .get("entropy")
                        .and_then(|v| v.as_f64())
                        .map(|v| v as f32)
                })
                .fold(0.0_f32, f32::max);
            if peak_entropy < 0.55 {
                continue;
            }
            let delta = peak_entropy - node.entropy;
            let significance = (delta * 0.8).min(1.0);
            if significance < self.min_significance {
                continue;
            }
            lore.push(LoreEntry::new(
                format!("Resolution of '{}'", node.content),
                ArcType::Resolution,
                generate_resolution_narrative(node, peak_entropy),
                vec![node.id],
                significance,
            ));
        }
        lore
    }

    /// Revelation arc: isolated nodes that gained connections (sudden connectivity spike).
    fn detect_revelations(&self, nodes: &[&NodeData], engine: &TemporalEngine) -> Vec<LoreEntry> {
        let mut lore = Vec::new();
        for node in nodes {
            let snaps = engine.snapshots_for(node.id);
            // Count Connected events
            let connections = snaps
                .iter()
                .filter(|s| s.change_type == ChangeType::Connected)
                .count();
            if connections < 3 {
                continue;
            }
            // Only if it started as isolated (first snapshot was Created with access_count=0)
            let started_alone = snaps
                .first()
                .map(|s| {
                    s.change_type == ChangeType::Created
                        && s.snapshot
                            .get("access_count")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(1)
                            == 0
                })
                .unwrap_or(false);
            if !started_alone {
                continue;
            }
            let significance = (connections as f32 * 0.15).min(1.0);
            lore.push(LoreEntry::new(
                format!("Revelation of '{}'", node.content),
                ArcType::Revelation,
                generate_revelation_narrative(node, connections),
                vec![node.id],
                significance,
            ));
        }
        lore
    }

    /// Transformation arc: nodes that changed type (Ghost revived, Idea upgraded).
    fn detect_transformations(
        &self,
        nodes: &[&NodeData],
        engine: &TemporalEngine,
    ) -> Vec<LoreEntry> {
        let mut lore = Vec::new();
        for node in nodes {
            let snaps = engine.snapshots_for(node.id);
            if snaps.len() < 2 {
                continue;
            }
            // Find type transitions
            let types: Vec<&str> = snaps
                .iter()
                .filter_map(|s| s.snapshot.get("node_type").and_then(|v| v.as_str()))
                .collect();
            let first_type = types.first().copied().unwrap_or("");
            let last_type = types.last().copied().unwrap_or("");
            if first_type == last_type {
                continue;
            }
            let significance = 0.7 * node.gravity.min(1.0);
            lore.push(LoreEntry::new(
                format!("Transformation of '{}'", node.content),
                ArcType::Transformation,
                generate_transformation_narrative(node, first_type, last_type),
                vec![node.id],
                significance,
            ));
        }
        lore
    }

    /// Legacy arc: fossilised nodes that had high connectivity before crystallising.
    fn detect_legacies(&self, nodes: &[&NodeData], engine: &TemporalEngine) -> Vec<LoreEntry> {
        let mut lore = Vec::new();
        for node in nodes.iter().filter(|n| n.is_fossil) {
            let connections_before = engine
                .snapshots_for(node.id)
                .iter()
                .filter(|s| s.change_type == ChangeType::Connected)
                .count();
            if connections_before < 2 {
                continue;
            }
            let significance = (connections_before as f32 * 0.12 + 0.3).min(1.0);
            lore.push(LoreEntry::new(
                format!("Legacy of '{}'", node.content),
                ArcType::Legacy,
                generate_legacy_narrative(node, connections_before),
                vec![node.id],
                significance,
            ));
        }
        lore
    }

    /// Tectonic arcs: each structural shift becomes a lore event.
    fn detect_tectonic_lore(
        &self,
        events: &[TectonicEvent],
        _now: DateTime<Utc>,
    ) -> Vec<LoreEntry> {
        events
            .iter()
            .filter(|e| e.magnitude >= self.min_significance)
            .map(|e| {
                LoreEntry::new(
                    format!("Tectonic Shift — {}", e.description),
                    ArcType::Tectonic,
                    format!(
                        "The cognitive universe underwent a structural reorganisation. {}  Magnitude: {:.2}.",
                        e.description, e.magnitude
                    ),
                    vec![],
                    e.magnitude.min(1.0),
                )
            })
            .collect()
    }
}

// ── Narrative generators ─────────────────────────────────────────────────────

fn generate_origin_narrative(node: &NodeData, event_count: usize) -> String {
    format!(
        "The node '{}' emerged rapidly into the cognitive universe, crystallising {} connections within its first moments of existence. Born as {:?}, it quickly became anchored in the graph's topology.",
        node.content, event_count, node.node_type
    )
}

fn generate_conflict_narrative(node: &NodeData, state_changes: usize) -> String {
    format!(
        "The node '{}' entered a period of turbulence — entropy climbed to {:.2} across {} state transitions. Something within this concept became unstable, caught between competing forces.",
        node.content, node.entropy, state_changes
    )
}

fn generate_resolution_narrative(node: &NodeData, peak_entropy: f32) -> String {
    format!(
        "After reaching a peak entropy of {:.2}, the node '{}' found resolution. Entropy fell to {:.2}, suggesting renewed focus or structural clarity restored its coherence.",
        peak_entropy, node.content, node.entropy
    )
}

fn generate_revelation_narrative(node: &NodeData, connections: usize) -> String {
    format!(
        "Once isolated and unknown, '{}' underwent a sudden revelation — {} connections emerged, weaving it into the broader cognitive fabric. Isolation transformed into centrality.",
        node.content, connections
    )
}

fn generate_transformation_narrative(node: &NodeData, from: &str, to: &str) -> String {
    format!(
        "The node '{}' underwent a fundamental transformation — its nature shifted from {} to {}. This metamorphosis marks a pivotal moment in the cognitive chronology.",
        node.content, from, to
    )
}

fn generate_legacy_narrative(node: &NodeData, connections: usize) -> String {
    format!(
        "The node '{}' crystallised into fossil form after forging {} connections. Though now dormant, its legacy persists in the edges it leaves behind — a monument in the cognitive graph.",
        node.content, connections
    )
}
