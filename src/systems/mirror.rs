use crate::domain::{FocusEvent, NodeData, TemporalSnapshot};
use chrono::{DateTime, Datelike, Duration, Timelike, Utc, Weekday};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityGap {
    pub node_id: Uuid,
    /// Rank by gravity (stated priority), 0 = highest gravity
    pub stated_rank: usize,
    /// Rank by focus score (actual attention), 0 = most focused
    pub actual_rank: usize,
    /// actual_rank - stated_rank; positive means neglected relative to stated importance
    pub gap: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindSpot {
    pub node_id: Uuid,
    pub connected_to_active: Vec<Uuid>,
    pub last_accessed_days_ago: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObsessionEntry {
    pub node_id: Uuid,
    pub focus_score: f32,
    pub entropy: f32,
    pub revisit_count: usize,
}

/// A single node's trajectory across time — part of the Evolution Portrait.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionEntry {
    pub node_id: Uuid,
    pub label: String,
    /// Entropy at the oldest recorded snapshot.
    pub entropy_start: f32,
    /// Entropy now.
    pub entropy_now: f32,
    /// Was this node once high-gravity (> 2.0) but is now neglected (< 0.5)?
    pub was_central: bool,
    /// Node type at first snapshot → current type (only present if changed).
    pub type_evolution: Option<(String, String)>,
    /// Total recorded state-change events for this node.
    pub state_changes: usize,
    /// Direction: "rising" | "stable" | "decaying"
    pub trajectory: String,
}

/// When the user is actually most cognitively productive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreativePattern {
    /// Hour of day (0–23) with the most deep_work sessions. None = insufficient data.
    pub peak_hour: Option<u8>,
    /// Day of week (0=Mon … 6=Sun) with the most deep_work sessions.
    pub peak_weekday: Option<u8>,
    /// Average deep_work session length in seconds.
    pub avg_deep_session_secs: f32,
    /// Human-readable period label: "Morning" / "Afternoon" / "Evening" / "Night" / "Distributed"
    pub focus_period: String,
    /// Total deep_work events in the analysis window.
    pub deep_work_event_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitivePortrait {
    pub priority_gaps: Vec<PriorityGap>,
    pub blind_spots: Vec<BlindSpot>,
    pub obsessions: Vec<ObsessionEntry>,
    pub most_neglected: Option<Uuid>,
    pub most_obsessed: Option<Uuid>,
    /// How the user's thinking has evolved over time (requires temporal snapshots).
    pub evolution: Vec<EvolutionEntry>,
    /// When the user is actually most cognitively productive.
    pub creative_pattern: Option<CreativePattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveMirror;

impl Default for CognitiveMirror {
    fn default() -> Self {
        Self
    }
}

impl CognitiveMirror {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_portrait(
        &self,
        nodes: &[&NodeData],
        focus_events: &[FocusEvent],
        adjacency: &HashMap<Uuid, Vec<Uuid>>,
        now: DateTime<Utc>,
        days: u32,
        snapshots: &[TemporalSnapshot],
    ) -> CognitivePortrait {
        if nodes.is_empty() {
            return CognitivePortrait {
                priority_gaps: Vec::new(),
                blind_spots: Vec::new(),
                obsessions: Vec::new(),
                most_neglected: None,
                most_obsessed: None,
                evolution: Vec::new(),
                creative_pattern: None,
            };
        }

        let window_start = now - Duration::days(days as i64);

        // Compute per-node focus score (sum of duration * depth_weight within window)
        let mut focus_scores: HashMap<Uuid, f32> = HashMap::new();
        let mut revisit_counts: HashMap<Uuid, usize> = HashMap::new();

        for event in focus_events.iter().filter(|e| e.timestamp >= window_start) {
            let score = event.duration_seconds * event.depth.weight();
            *focus_scores.entry(event.node_id).or_insert(0.0) += score;
            *revisit_counts.entry(event.node_id).or_insert(0) += 1;
        }

        // Normalize focus scores to [0, 1]
        let max_focus = focus_scores.values().cloned().fold(0.0_f32, f32::max);
        let norm_focus: HashMap<Uuid, f32> = focus_scores
            .iter()
            .map(|(id, &s)| (*id, if max_focus > 0.0 { s / max_focus } else { 0.0 }))
            .collect();

        // ── Priority Gaps ───────────────────────────────────────────────────────
        // stated rank: sorted by gravity descending
        let mut by_gravity: Vec<Uuid> = nodes.iter().map(|n| n.id).collect();
        by_gravity.sort_by(|a, b| {
            let ga = nodes
                .iter()
                .find(|n| n.id == *a)
                .map(|n| n.gravity)
                .unwrap_or(0.0);
            let gb = nodes
                .iter()
                .find(|n| n.id == *b)
                .map(|n| n.gravity)
                .unwrap_or(0.0);
            gb.partial_cmp(&ga).unwrap_or(std::cmp::Ordering::Equal)
        });

        // actual rank: sorted by focus score descending
        let mut by_focus: Vec<Uuid> = nodes.iter().map(|n| n.id).collect();
        by_focus.sort_by(|a, b| {
            let fa = norm_focus.get(a).copied().unwrap_or(0.0);
            let fb = norm_focus.get(b).copied().unwrap_or(0.0);
            fb.partial_cmp(&fa).unwrap_or(std::cmp::Ordering::Equal)
        });

        let stated_rank_map: HashMap<Uuid, usize> = by_gravity
            .iter()
            .enumerate()
            .map(|(i, id)| (*id, i))
            .collect();
        let actual_rank_map: HashMap<Uuid, usize> = by_focus
            .iter()
            .enumerate()
            .map(|(i, id)| (*id, i))
            .collect();

        let mut priority_gaps: Vec<PriorityGap> = nodes
            .iter()
            .map(|n| {
                let stated_rank = stated_rank_map.get(&n.id).copied().unwrap_or(0);
                let actual_rank = actual_rank_map.get(&n.id).copied().unwrap_or(0);
                let gap = actual_rank as i32 - stated_rank as i32;
                PriorityGap {
                    node_id: n.id,
                    stated_rank,
                    actual_rank,
                    gap,
                }
            })
            .collect();
        // Sort by absolute gap descending
        priority_gaps.sort_by_key(|g| -g.gap.abs());

        // ── Blind Spots ─────────────────────────────────────────────────────────
        // Nodes that haven't been accessed for > 21 days but have active neighbors (< 30 days)
        let active_cutoff = now - Duration::days(30);
        let blind_cutoff = now - Duration::days(21);

        let active_nodes: std::collections::HashSet<Uuid> = nodes
            .iter()
            .filter(|n| n.accessed_at >= active_cutoff)
            .map(|n| n.id)
            .collect();

        let mut blind_spots: Vec<BlindSpot> = Vec::new();
        for node in nodes.iter() {
            if node.accessed_at >= blind_cutoff {
                continue;
            }
            let neighbors = adjacency.get(&node.id).cloned().unwrap_or_default();
            let active_neighbors: Vec<Uuid> = neighbors
                .into_iter()
                .filter(|nb| active_nodes.contains(nb))
                .collect();
            if !active_neighbors.is_empty() {
                let last_accessed_days_ago =
                    (now - node.accessed_at).num_seconds().max(0) as f32 / 86400.0;
                blind_spots.push(BlindSpot {
                    node_id: node.id,
                    connected_to_active: active_neighbors,
                    last_accessed_days_ago,
                });
            }
        }
        blind_spots.sort_by(|a, b| {
            b.last_accessed_days_ago
                .partial_cmp(&a.last_accessed_days_ago)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // ── Obsessions ──────────────────────────────────────────────────────────
        let mut obsessions: Vec<ObsessionEntry> = Vec::new();
        for node in nodes.iter() {
            let focus = norm_focus.get(&node.id).copied().unwrap_or(0.0);
            if focus > 0.6 && node.entropy > 0.5 {
                let revisit_count = revisit_counts.get(&node.id).copied().unwrap_or(0);
                obsessions.push(ObsessionEntry {
                    node_id: node.id,
                    focus_score: focus,
                    entropy: node.entropy,
                    revisit_count,
                });
            }
        }
        obsessions.sort_by(|a, b| {
            b.focus_score
                .partial_cmp(&a.focus_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // ── Most neglected / most obsessed ─────────────────────────────────────
        let most_neglected =
            priority_gaps
                .first()
                .and_then(|g| if g.gap > 0 { Some(g.node_id) } else { None });
        let most_obsessed = obsessions.first().map(|o| o.node_id);

        // ── Evolution Portrait ───────────────────────────────────────────────
        let evolution = build_evolution(nodes, snapshots);

        // ── Creative Pattern Analysis ────────────────────────────────────────
        let creative_pattern = build_creative_pattern(focus_events, now, days);

        CognitivePortrait {
            priority_gaps,
            blind_spots,
            obsessions,
            most_neglected,
            most_obsessed,
            evolution,
            creative_pattern,
        }
    }
}

// ── Evolution Portrait helpers ────────────────────────────────────────────────

fn build_evolution(nodes: &[&NodeData], snapshots: &[TemporalSnapshot]) -> Vec<EvolutionEntry> {
    use crate::domain::ChangeType;

    // Build per-node snapshot history
    let mut by_node: HashMap<Uuid, Vec<&TemporalSnapshot>> = HashMap::new();
    for s in snapshots {
        by_node.entry(s.node_id).or_default().push(s);
    }
    for snaps in by_node.values_mut() {
        snaps.sort_by_key(|s| s.timestamp);
    }

    let mut entries: Vec<EvolutionEntry> = Vec::new();

    for node in nodes {
        let snaps = match by_node.get(&node.id) {
            Some(s) if s.len() >= 2 => s,
            _ => continue,
        };

        let first = snaps.first().unwrap();
        let entropy_start = first
            .snapshot
            .get("entropy")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32)
            .unwrap_or(0.0);
        let entropy_now = node.entropy;

        let gravity_start = first
            .snapshot
            .get("gravity")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32)
            .unwrap_or(1.0);
        let was_central = gravity_start > 2.0 && node.gravity < 0.8;

        let type_start = first
            .snapshot
            .get("node_type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let type_now = format!("{:?}", node.node_type);
        let type_evolution = if !type_start.is_empty() && type_start != type_now {
            Some((type_start, type_now))
        } else {
            None
        };

        let state_changes = snaps
            .iter()
            .filter(|s| s.change_type == ChangeType::StateChanged)
            .count();

        let delta = entropy_now - entropy_start;
        let trajectory = if delta > 0.15 {
            "decaying".to_string()
        } else if delta < -0.15 {
            "rising".to_string()
        } else {
            "stable".to_string()
        };

        entries.push(EvolutionEntry {
            node_id: node.id,
            label: node.content.chars().take(40).collect(),
            entropy_start,
            entropy_now,
            was_central,
            type_evolution,
            state_changes,
            trajectory,
        });
    }

    // Sort: was_central first, then by abs(entropy_delta) descending
    entries.sort_by(|a, b| {
        let rank_a = if a.was_central { 1u8 } else { 0 };
        let rank_b = if b.was_central { 1u8 } else { 0 };
        rank_b.cmp(&rank_a).then_with(|| {
            let da = (a.entropy_now - a.entropy_start).abs();
            let db = (b.entropy_now - b.entropy_start).abs();
            db.partial_cmp(&da).unwrap_or(std::cmp::Ordering::Equal)
        })
    });

    entries.truncate(20);
    entries
}

// ── Creative Pattern helpers ───────────────────────────────────────────────────

fn build_creative_pattern(
    focus_events: &[FocusEvent],
    now: DateTime<Utc>,
    days: u32,
) -> Option<CreativePattern> {
    use crate::domain::FocusDepth;

    let window_start = now - Duration::days(days as i64);
    let deep_events: Vec<&FocusEvent> = focus_events
        .iter()
        .filter(|e| e.timestamp >= window_start && e.depth == FocusDepth::DeepWork)
        .collect();

    if deep_events.len() < 3 {
        return None;
    }

    // Hour distribution (0–23)
    let mut hour_counts = [0u32; 24];
    let mut weekday_counts = [0u32; 7];
    let mut total_secs = 0.0f32;

    for e in &deep_events {
        let local = e.timestamp;
        let h = local.hour() as usize;
        hour_counts[h] += 1;
        // Weekday: Mon=0 … Sun=6
        let wd = match local.weekday() {
            Weekday::Mon => 0,
            Weekday::Tue => 1,
            Weekday::Wed => 2,
            Weekday::Thu => 3,
            Weekday::Fri => 4,
            Weekday::Sat => 5,
            Weekday::Sun => 6,
        };
        weekday_counts[wd] += 1;
        total_secs += e.duration_seconds;
    }

    let peak_hour = hour_counts
        .iter()
        .enumerate()
        .max_by_key(|(_, &c)| c)
        .filter(|(_, &c)| c > 0)
        .map(|(h, _)| h as u8);

    let peak_weekday = weekday_counts
        .iter()
        .enumerate()
        .max_by_key(|(_, &c)| c)
        .filter(|(_, &c)| c > 0)
        .map(|(d, _)| d as u8);

    let avg_deep_session_secs = total_secs / deep_events.len() as f32;

    // Focus period: determine dominant block (morning/afternoon/evening/night)
    let morning: u32 = hour_counts[5..12].iter().sum();
    let afternoon: u32 = hour_counts[12..18].iter().sum();
    let evening: u32 = hour_counts[18..23].iter().sum();
    let night: u32 = hour_counts[23..].iter().sum::<u32>() + hour_counts[..5].iter().sum::<u32>();
    let max_period = morning.max(afternoon).max(evening).max(night);
    let focus_period = if max_period == 0 || {
        let ratio = max_period as f32 / deep_events.len() as f32;
        ratio < 0.35
    } {
        "Distributed"
    } else if morning == max_period {
        "Morning"
    } else if afternoon == max_period {
        "Afternoon"
    } else if evening == max_period {
        "Evening"
    } else {
        "Night"
    }
    .to_string();

    Some(CreativePattern {
        peak_hour,
        peak_weekday,
        avg_deep_session_secs,
        focus_period,
        deep_work_event_count: deep_events.len(),
    })
}
