use crate::domain::{FocusEvent, NodeData};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigitalShadow {
    pub node_id: Uuid,
    pub revisit_count: usize,
    pub age_days: f32,
    pub intensity: f32,
    pub last_visited: DateTime<Utc>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigitalShadowDetector {
    pub min_revisits: usize,
    pub revisit_gap_days: u32,
    pub max_entropy: f32,
}

impl Default for DigitalShadowDetector {
    fn default() -> Self {
        Self {
            min_revisits: 3,
            revisit_gap_days: 3,
            max_entropy: 0.7,
        }
    }
}

impl DigitalShadowDetector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn detect(
        &self,
        nodes: &[&NodeData],
        focus_events: &[FocusEvent],
        now: DateTime<Utc>,
    ) -> Vec<DigitalShadow> {
        let gap = Duration::days(self.revisit_gap_days as i64);
        let mut shadows: Vec<DigitalShadow> = Vec::new();

        // Group events by node_id
        let mut events_by_node: std::collections::HashMap<Uuid, Vec<&FocusEvent>> =
            std::collections::HashMap::new();
        for e in focus_events {
            events_by_node.entry(e.node_id).or_default().push(e);
        }

        for node in nodes.iter() {
            // Skip ghost, fossil, void nodes
            if node.is_ghost || node.is_fossil || node.is_void {
                continue;
            }
            // Entropy gate
            if node.entropy > self.max_entropy || node.entropy <= 0.1 {
                continue;
            }

            let mut node_events: Vec<&FocusEvent> =
                events_by_node.get(&node.id).cloned().unwrap_or_default();
            if node_events.is_empty() {
                continue;
            }

            // Sort events by timestamp
            node_events.sort_by_key(|e| e.timestamp);

            // Count revisits: a new visit is a "revisit" if the previous event
            // on this node was more than revisit_gap_days ago
            let mut revisit_count = 0usize;
            let mut last_visit: Option<DateTime<Utc>> = None;
            let mut last_visited_ts = node_events[0].timestamp;

            for event in &node_events {
                if let Some(prev) = last_visit {
                    if event.timestamp - prev >= gap {
                        revisit_count += 1;
                    }
                }
                last_visit = Some(event.timestamp);
                if event.timestamp > last_visited_ts {
                    last_visited_ts = event.timestamp;
                }
            }

            if revisit_count < self.min_revisits {
                continue;
            }

            let age_days = (now - node.created_at).num_seconds().max(0) as f32 / 86400.0;

            // Intensity measures how persistent and unresolved this shadow is.
            // Vision.md: "high approach, low output — significant attention received,
            // minimal structural change."
            //
            // Components:
            //   revisit_frequency — saturates at 5 returns (min(revisits/5, 1.0))
            //   recency_factor    — shadow that was visited recently is more present
            //   entropy_midpoint  — mid-entropy (0.3-0.6) maximises shadow character:
            //                       not dead (ghost) and not thriving (spring node)
            //   shallow_ratio     — fraction of visits that were shallow (no deep work)
            let revisit_freq = (revisit_count as f32 / 5.0).clamp(0.0, 1.0);

            let days_since_last = (now - last_visited_ts).num_seconds().max(0) as f32 / 86400.0;
            let recency_factor = if days_since_last < 7.0 {
                1.0
            } else if days_since_last < 30.0 {
                1.0 - (days_since_last - 7.0) / 23.0
            } else {
                0.3
            };

            // Entropy midpoint: peaks at 0.45, falls off toward 0 and 1
            let entropy_midpoint = (1.0 - (node.entropy - 0.45).abs() / 0.55).max(0.0);

            let intensity = (revisit_freq * 0.45 + recency_factor * 0.25 + entropy_midpoint * 0.30)
                .clamp(0.0, 1.0);

            let description = format!(
                "Digital shadow: '{}' revisited {} times (intensity={:.2}, entropy={:.2})",
                node.content.chars().take(40).collect::<String>(),
                revisit_count,
                intensity,
                node.entropy
            );

            shadows.push(DigitalShadow {
                node_id: node.id,
                revisit_count,
                age_days,
                intensity,
                last_visited: last_visited_ts,
                description,
            });
        }

        shadows.sort_by(|a, b| {
            b.intensity
                .partial_cmp(&a.intensity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        shadows
    }
}
