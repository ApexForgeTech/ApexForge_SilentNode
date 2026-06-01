use crate::domain::FocusEvent;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Derive a human-readable ritual name from a sequence of node IDs and content.
///
/// Vision.md: rituals are "behavioral patterns that occur regularly across time
/// and serve as cognitive transitions — marking entry into or exit from a mental state."
///
/// Name examples:
///   "2-step: Rust compiler → Error analysis"
///   "3-step: Morning nodes → Deep work → Review"
pub fn name_ritual(sequence: &[Uuid], content_map: &HashMap<Uuid, String>) -> String {
    if sequence.is_empty() {
        return "Empty ritual".to_string();
    }

    let label = |id: &Uuid| -> String {
        content_map
            .get(id)
            .map(|s| s.chars().take(22).collect::<String>())
            .unwrap_or_else(|| id.to_string()[..8].to_string())
    };

    match sequence.len() {
        1 => format!("Solo: {}", label(&sequence[0])),
        2 => format!("{} → {}", label(&sequence[0]), label(&sequence[1])),
        3 => format!(
            "{} → {} → {}",
            label(&sequence[0]),
            label(&sequence[1]),
            label(&sequence[2])
        ),
        n => format!(
            "{} → … → {} ({}-step)",
            label(&sequence[0]),
            label(&sequence[n - 1]),
            n
        ),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ritual {
    pub id: Uuid,
    pub name: String,
    pub sequence: Vec<Uuid>,
    pub occurrence_count: usize,
    pub avg_interval_hours: f32,
    pub strength: f32,
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RitualEngine {
    pub min_occurrences: usize,
    pub window_hours: usize,
}

impl Default for RitualEngine {
    fn default() -> Self {
        Self {
            min_occurrences: 3,
            window_hours: 4,
        }
    }
}

impl RitualEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Split focus events into sessions; events are grouped if the gap between
    /// consecutive events is within `window_hours`.
    fn extract_sessions(&self, events: &[FocusEvent]) -> Vec<Vec<Uuid>> {
        if events.is_empty() {
            return Vec::new();
        }

        // Sort by timestamp
        let mut sorted: Vec<&FocusEvent> = events.iter().collect();
        sorted.sort_by_key(|e| e.timestamp);

        let window = Duration::hours(self.window_hours as i64);
        let mut sessions: Vec<Vec<Uuid>> = Vec::new();
        let mut current_session: Vec<Uuid> = vec![sorted[0].node_id];
        let mut last_ts = sorted[0].timestamp;

        for event in sorted.iter().skip(1) {
            if event.timestamp - last_ts <= window {
                current_session.push(event.node_id);
            } else {
                if !current_session.is_empty() {
                    sessions.push(current_session.clone());
                }
                current_session = vec![event.node_id];
            }
            last_ts = event.timestamp;
        }
        if !current_session.is_empty() {
            sessions.push(current_session);
        }
        sessions
    }

    /// Detect repeating node sequences of length 2..=5 across sessions.
    pub fn detect_rituals(&self, events: &[FocusEvent]) -> Vec<Ritual> {
        let sessions = self.extract_sessions(events);
        let total_sessions = sessions.len().max(1);

        // Map sequence -> list of (session_index, last_seen_timestamp)
        let mut seq_occurrences: HashMap<Vec<Uuid>, Vec<usize>> = HashMap::new();
        let mut seq_last_seen: HashMap<Vec<Uuid>, DateTime<Utc>> = HashMap::new();

        // Also track timestamps per session for interval calculation
        // Sort events once so we can get timestamps
        let mut sorted_events: Vec<&FocusEvent> = events.iter().collect();
        sorted_events.sort_by_key(|e| e.timestamp);

        // Build per-session timestamp of last event
        let mut session_timestamps: Vec<DateTime<Utc>> = Vec::new();
        {
            let mut sorted2: Vec<&FocusEvent> = events.iter().collect();
            sorted2.sort_by_key(|e| e.timestamp);
            let window = Duration::hours(self.window_hours as i64);
            if !sorted2.is_empty() {
                let mut last_ts = sorted2[0].timestamp;
                let mut current_last = sorted2[0].timestamp;
                for event in sorted2.iter().skip(1) {
                    if event.timestamp - last_ts <= window {
                        current_last = event.timestamp;
                    } else {
                        session_timestamps.push(current_last);
                        current_last = event.timestamp;
                    }
                    last_ts = event.timestamp;
                }
                session_timestamps.push(current_last);
            }
        }

        for (sess_idx, session) in sessions.iter().enumerate() {
            for len in 2usize..=5 {
                if session.len() < len {
                    continue;
                }
                for start in 0..=(session.len() - len) {
                    let subseq: Vec<Uuid> = session[start..start + len].to_vec();
                    seq_occurrences
                        .entry(subseq.clone())
                        .or_default()
                        .push(sess_idx);
                    let ts = session_timestamps
                        .get(sess_idx)
                        .copied()
                        .unwrap_or_else(Utc::now);
                    seq_last_seen
                        .entry(subseq)
                        .and_modify(|t| {
                            if ts > *t {
                                *t = ts;
                            }
                        })
                        .or_insert(ts);
                }
            }
        }

        // Filter by min_occurrences, avoiding duplicates (sub-sequences of larger ones
        // that already dominate are kept — we report all qualifying sequences).
        let mut rituals: Vec<Ritual> = Vec::new();

        for (seq, sess_indices) in &seq_occurrences {
            let count = sess_indices.len();
            if count < self.min_occurrences {
                continue;
            }

            // Compute average interval between occurrences (in hours)
            let mut intervals: Vec<f32> = Vec::new();
            let mut sorted_indices = sess_indices.clone();
            sorted_indices.sort_unstable();
            for pair in sorted_indices.windows(2) {
                let ts_a = session_timestamps
                    .get(pair[0])
                    .copied()
                    .unwrap_or_else(Utc::now);
                let ts_b = session_timestamps
                    .get(pair[1])
                    .copied()
                    .unwrap_or_else(Utc::now);
                let diff = (ts_b - ts_a).num_seconds().abs() as f32 / 3600.0;
                intervals.push(diff);
            }
            let avg_interval = if intervals.is_empty() {
                0.0
            } else {
                intervals.iter().sum::<f32>() / intervals.len() as f32
            };

            let strength = (count as f32 / total_sessions as f32).clamp(0.0, 1.0);
            let last_seen = seq_last_seen.get(seq).copied().unwrap_or_else(Utc::now);

            // Name placeholder — caller can enrich with node content via `name_ritual()`
            let name = format!("{}-step ritual", seq.len());

            rituals.push(Ritual {
                id: Uuid::new_v4(),
                name,
                sequence: seq.clone(),
                occurrence_count: count,
                avg_interval_hours: avg_interval,
                strength,
                last_seen,
            });
        }

        // Sort by strength descending
        rituals.sort_by(|a, b| {
            b.strength
                .partial_cmp(&a.strength)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        rituals
    }

    /// Given the current partial session, predict the next node based on a ritual pattern.
    pub fn predict_next_step(session: &[Uuid], ritual: &Ritual) -> Option<Uuid> {
        if ritual.sequence.is_empty() || session.is_empty() {
            return None;
        }
        let seq = &ritual.sequence;
        let n = session.len();

        // Find the longest suffix of `session` that matches a prefix of ritual.sequence
        for len in (1..=n.min(seq.len() - 1)).rev() {
            let suffix = &session[n - len..];
            if suffix == &seq[..len] {
                return seq.get(len).copied();
            }
        }
        None
    }
}
