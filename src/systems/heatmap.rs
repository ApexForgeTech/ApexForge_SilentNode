use crate::domain::{FocusEvent, NodeData};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatmapEntry {
    pub node_id: Uuid,
    /// Normalized energy [0, 1]
    pub energy: f32,
    pub raw_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveHeatmap {
    pub entries: Vec<HeatmapEntry>,
    pub computed_at: DateTime<Utc>,
    pub window_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObsessiveLoop {
    pub node_id: Uuid,
    pub revisit_count: usize,
    pub avg_session_seconds: f32,
    pub entropy: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeglectedRegion {
    pub node_id: Uuid,
    pub connected_active_nodes: usize,
    pub days_since_access: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtHeatmapEngine {
    pub decay_halflife_days: f32,
}

impl Default for ThoughtHeatmapEngine {
    fn default() -> Self {
        Self {
            decay_halflife_days: 7.0,
        }
    }
}

impl ThoughtHeatmapEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate the cognitive heatmap from focus events within the window.
    pub fn calculate(
        &self,
        nodes: &[&NodeData],
        focus_events: &[FocusEvent],
        now: DateTime<Utc>,
        window_days: u32,
    ) -> CognitiveHeatmap {
        let window_start = now - Duration::days(window_days as i64);
        let halflife = self.decay_halflife_days;

        let mut raw_scores: HashMap<Uuid, f32> = HashMap::new();

        for event in focus_events.iter().filter(|e| e.timestamp >= window_start) {
            let age_days = (now - event.timestamp).num_seconds().max(0) as f32 / 86400.0;
            let decay = (-age_days / halflife).exp();
            let contribution = event.duration_seconds * event.depth.weight() * decay;
            *raw_scores.entry(event.node_id).or_insert(0.0) += contribution;
        }

        // Ensure all nodes appear, even with zero score
        for node in nodes.iter() {
            raw_scores.entry(node.id).or_insert(0.0);
        }

        let max_score = raw_scores.values().cloned().fold(0.0_f32, f32::max);

        let mut entries: Vec<HeatmapEntry> = raw_scores
            .into_iter()
            .map(|(node_id, raw_score)| {
                let energy = if max_score > 0.0 {
                    (raw_score / max_score).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                HeatmapEntry {
                    node_id,
                    energy,
                    raw_score,
                }
            })
            .collect();

        entries.sort_by(|a, b| {
            b.energy
                .partial_cmp(&a.energy)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        CognitiveHeatmap {
            entries,
            computed_at: now,
            window_days,
        }
    }

    /// Find nodes with high energy (top 20%) AND high entropy (> 0.4) — obsessive revisiting.
    pub fn find_obsessive_loops(
        &self,
        heatmap: &CognitiveHeatmap,
        nodes: &[&NodeData],
        min_revisits: usize,
        focus_events: &[FocusEvent],
    ) -> Vec<ObsessiveLoop> {
        if heatmap.entries.is_empty() {
            return Vec::new();
        }

        let cutoff_idx = (heatmap.entries.len() as f32 * 0.2).ceil() as usize;
        let top_20_pct: std::collections::HashSet<Uuid> = heatmap
            .entries
            .iter()
            .take(cutoff_idx.max(1))
            .map(|e| e.node_id)
            .collect();

        // Build revisit counts and avg session seconds per node
        let window_start = heatmap.computed_at - Duration::days(heatmap.window_days as i64);
        let mut event_groups: HashMap<Uuid, Vec<&FocusEvent>> = HashMap::new();
        for e in focus_events.iter().filter(|e| e.timestamp >= window_start) {
            event_groups.entry(e.node_id).or_default().push(e);
        }

        let mut loops: Vec<ObsessiveLoop> = Vec::new();
        for node in nodes.iter() {
            if !top_20_pct.contains(&node.id) || node.entropy <= 0.4 {
                continue;
            }
            let visits = event_groups
                .get(&node.id)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            let revisit_count = visits.len();
            if revisit_count < min_revisits {
                continue;
            }
            let avg_session_seconds = if revisit_count == 0 {
                0.0
            } else {
                visits.iter().map(|e| e.duration_seconds).sum::<f32>() / revisit_count as f32
            };
            loops.push(ObsessiveLoop {
                node_id: node.id,
                revisit_count,
                avg_session_seconds,
                entropy: node.entropy,
            });
        }

        loops.sort_by(|a, b| b.revisit_count.cmp(&a.revisit_count));
        loops
    }

    /// Find nodes in bottom 30% of heatmap that have neighbors in top 50%.
    pub fn find_neglected_regions(
        &self,
        heatmap: &CognitiveHeatmap,
        adjacency: &HashMap<Uuid, Vec<Uuid>>,
    ) -> Vec<NeglectedRegion> {
        if heatmap.entries.is_empty() {
            return Vec::new();
        }

        let total = heatmap.entries.len();
        let bottom_cutoff = (total as f32 * 0.7).floor() as usize;
        let top_cutoff = (total as f32 * 0.5).ceil() as usize;

        let bottom_30: std::collections::HashSet<Uuid> = heatmap
            .entries
            .iter()
            .skip(bottom_cutoff)
            .map(|e| e.node_id)
            .collect();
        let top_50: std::collections::HashSet<Uuid> = heatmap
            .entries
            .iter()
            .take(top_cutoff.max(1))
            .map(|e| e.node_id)
            .collect();

        let mut regions: Vec<NeglectedRegion> = Vec::new();
        for node_id in &bottom_30 {
            let neighbors = adjacency.get(node_id).cloned().unwrap_or_default();
            let active_count = neighbors.iter().filter(|nb| top_50.contains(nb)).count();
            if active_count > 0 {
                // Find days_since_access from heatmap (bottom nodes have low energy)
                // We approximate days_since_access via position in list
                let entry_pos = heatmap.entries.iter().position(|e| &e.node_id == node_id);
                let days_since_access = entry_pos
                    .map(|p| {
                        let normalized = p as f32 / total as f32;
                        normalized * heatmap.window_days as f32
                    })
                    .unwrap_or(heatmap.window_days as f32);
                regions.push(NeglectedRegion {
                    node_id: *node_id,
                    connected_active_nodes: active_count,
                    days_since_access,
                });
            }
        }

        regions.sort_by(|a, b| {
            b.days_since_access
                .partial_cmp(&a.days_since_access)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        regions
    }
}
