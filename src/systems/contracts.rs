use crate::domain::{FocusDepth, FocusEvent, JournalEntry, NodeData, NodeType};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedContract {
    pub node_id: Uuid,
    pub strength: f32,
    /// Focus events with shallow depth but no DeepWork, normalized
    pub approach_score: f32,
    /// Journal mentions with no follow-up action within 48h
    pub journal_score: f32,
    /// High gravity vs low access count
    pub gravity_gap: f32,
    /// Isolated high-gravity node: connected to nothing (orphaned project structure)
    pub isolation_score: f32,
    pub age_days: f32,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SilentContractDetector {
    pub threshold: f32,
}

impl Default for SilentContractDetector {
    fn default() -> Self {
        Self { threshold: 0.35 }
    }
}

impl SilentContractDetector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn detect(
        &self,
        nodes: &[&NodeData],
        focus_events: &[FocusEvent],
        journal_entries: &[JournalEntry],
        adjacency: &HashMap<Uuid, Vec<Uuid>>,
        now: DateTime<Utc>,
    ) -> Vec<DetectedContract> {
        // Group focus events by node
        let mut events_by_node: HashMap<Uuid, Vec<&FocusEvent>> = HashMap::new();
        for e in focus_events.iter() {
            events_by_node.entry(e.node_id).or_default().push(e);
        }

        // Max gravity for normalization
        let max_gravity = nodes
            .iter()
            .map(|n| n.gravity)
            .fold(0.0_f32, f32::max)
            .max(1.0);

        let mut contracts: Vec<DetectedContract> = Vec::new();

        for node in nodes.iter() {
            let node_events = events_by_node
                .get(&node.id)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            let total_events = node_events.len();

            // ── approach_score ──────────────────────────────────────────────────
            // Events with shallow depth (Glance or Read) and no DeepWork at all
            let has_deep_work = node_events.iter().any(|e| e.depth == FocusDepth::DeepWork);
            let shallow_count = node_events
                .iter()
                .filter(|e| matches!(e.depth, FocusDepth::Glance | FocusDepth::Read))
                .count();
            let approach_score = if total_events == 0 || has_deep_work {
                0.0
            } else {
                (shallow_count as f32 / total_events as f32).clamp(0.0, 1.0)
            };

            // ── journal_score ───────────────────────────────────────────────────
            // Journal entries mentioning this node with no focus event within 48h after
            let mentions: Vec<&JournalEntry> = journal_entries
                .iter()
                .filter(|j| j.linked_nodes.contains(&node.id))
                .collect();
            let mention_count = mentions.len();
            let unfollowed = mentions
                .iter()
                .filter(|j| {
                    let deadline = j.timestamp + Duration::hours(48);
                    // No focus event on this node between mention and +48h
                    !node_events
                        .iter()
                        .any(|e| e.timestamp >= j.timestamp && e.timestamp <= deadline)
                })
                .count();
            let journal_score = if mention_count == 0 {
                0.0
            } else {
                (unfollowed as f32 / mention_count as f32).clamp(0.0, 1.0)
            };

            // ── gravity_gap ─────────────────────────────────────────────────────
            // High gravity but low access count
            let raw_gap = node.gravity / (node.access_count as f32 + 1.0);
            // Normalize by max_gravity (a rough upper bound)
            let gravity_gap = (raw_gap / max_gravity).clamp(0.0, 1.0);

            // ── isolation_score ─────────────────────────────────────────────────
            // Vision.md: "Orphaned project structures — projects that have architecture
            // and nodes but whose activity trails have gone cold."
            // High gravity + no connections (or all neighbors are ghosts) = strong orphan signal.
            let neighbors = adjacency.get(&node.id).cloned().unwrap_or_default();
            let is_project_type = matches!(
                node.node_type,
                NodeType::Project | NodeType::World | NodeType::Artifact
            );
            let isolation_score = if is_project_type && node.gravity > 1.2 {
                let active_neighbors = neighbors.iter().count(); // we'd filter out ghosts if we had node state, but adjacency is enough
                if active_neighbors == 0 {
                    // Completely isolated high-gravity project node
                    (node.gravity / 3.0).clamp(0.0, 1.0)
                } else if active_neighbors <= 1 {
                    // Nearly isolated
                    (node.gravity / 5.0).clamp(0.0, 0.6)
                } else {
                    0.0
                }
            } else {
                0.0
            };

            // ── strength ────────────────────────────────────────────────────────
            let strength = approach_score * 0.35
                + journal_score * 0.35
                + gravity_gap * 0.15
                + isolation_score * 0.15;

            if strength < self.threshold {
                continue;
            }

            let age_days = (now - node.created_at).num_seconds().max(0) as f32 / 86400.0;

            let description = if isolation_score > 0.3 {
                format!(
                    "Orphaned project contract on '{}': isolated high-gravity node \
                     (approach={:.2}, journal={:.2}, isolation={:.2})",
                    node.content.chars().take(40).collect::<String>(),
                    approach_score,
                    journal_score,
                    isolation_score
                )
            } else {
                format!(
                    "Silent contract on '{}': approach={:.2}, journal_gap={:.2}, \
                     gravity_gap={:.2}",
                    node.content.chars().take(40).collect::<String>(),
                    approach_score,
                    journal_score,
                    gravity_gap
                )
            };

            contracts.push(DetectedContract {
                node_id: node.id,
                strength,
                approach_score,
                journal_score,
                gravity_gap,
                isolation_score,
                age_days,
                description,
            });
        }

        contracts.sort_by(|a, b| {
            b.strength
                .partial_cmp(&a.strength)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        contracts
    }
}
