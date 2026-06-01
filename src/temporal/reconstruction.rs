// Memory Reconstruction — full-day cognitive state snapshots.

use super::archaeology::snapshot_to_node;
use super::TemporalEngine;
use crate::domain::{FocusEvent, JournalEntry, NodeData};
use crate::focus::FocusTrailEngine;
use crate::journal::JournalEngine;
use crate::systems::WeatherSystem;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// The ambient aura signature of a day, derived from the weather system at
/// end-of-day. Stored as a compact visual fingerprint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuraSignature {
    pub state_name: String,
    pub intensity: f32,
    pub turbulence: f32,
    pub pulse_rate: f32,
    pub primary_color: [f32; 4],
    pub secondary_color: [f32; 4],
}

/// A day's worth of lore: which nodes were active, the journal, the weather.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayReconstruction {
    pub date: NaiveDate,
    /// Node states at the end of the day (last known snapshot for each node).
    pub node_states: HashMap<Uuid, NodeData>,
    /// Focus trail for that calendar day.
    pub focus_events: Vec<FocusEvent>,
    /// Journal entries for that calendar day.
    pub journal_entries: Vec<JournalEntry>,
    /// Ambient aura state approximated from node states.
    pub aura_signature: AuraSignature,
    /// Total unique nodes touched.
    pub nodes_touched: usize,
    /// Total focus time (seconds).
    pub total_focus_seconds: f32,
    /// Dominant node ID (most focused).
    pub dominant_node: Option<Uuid>,
}

impl DayReconstruction {
    pub fn summary(&self) -> String {
        format!(
            "{} | {} nodes touched | {:.0}s focus | state: {} | journal: {} entries",
            self.date,
            self.nodes_touched,
            self.total_focus_seconds,
            self.aura_signature.state_name,
            self.journal_entries.len()
        )
    }
}

/// Field-level change between two DayReconstructions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeChangeSummary {
    pub node_id: Uuid,
    pub entropy_delta: f32,
    pub access_delta: i64,
    pub became_ghost: bool,
    pub became_fossil: bool,
    pub content_changed: bool,
}

/// Comparison between two reconstructed days.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayComparison {
    pub day_a: NaiveDate,
    pub day_b: NaiveDate,
    /// Nodes that were present on day_b but not day_a (newly appeared).
    pub new_nodes: Vec<Uuid>,
    /// Nodes that existed on day_a but are gone by day_b.
    pub removed_nodes: Vec<Uuid>,
    /// Per-node field changes.
    pub changed_nodes: Vec<NodeChangeSummary>,
    pub focus_delta_seconds: f32,
    pub journal_entry_delta: i32,
}

impl DayComparison {
    pub fn summary(&self) -> String {
        format!(
            "{} → {} | +{} -{} changed:{} focus_Δ{:+.0}s journal_Δ{:+}",
            self.day_a,
            self.day_b,
            self.new_nodes.len(),
            self.removed_nodes.len(),
            self.changed_nodes.len(),
            self.focus_delta_seconds,
            self.journal_entry_delta
        )
    }
}

/// Reconstructs point-in-time cognitive states for any calendar day.
pub struct MemoryReconstructor<'a> {
    engine: &'a TemporalEngine,
    focus: &'a FocusTrailEngine,
    journal: &'a JournalEngine,
}

impl<'a> MemoryReconstructor<'a> {
    pub fn new(
        engine: &'a TemporalEngine,
        focus: &'a FocusTrailEngine,
        journal: &'a JournalEngine,
    ) -> Self {
        Self {
            engine,
            focus,
            journal,
        }
    }

    /// Reconstruct the full cognitive state for `date`.
    pub fn reconstruct_day(&self, date: NaiveDate) -> DayReconstruction {
        let day_start: DateTime<Utc> = Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).unwrap());
        let day_end: DateTime<Utc> = Utc.from_utc_datetime(&date.and_hms_opt(23, 59, 59).unwrap());

        // Node states at end of day
        let universe_map = self.engine.reconstruct_universe_at(day_end);
        let node_states: HashMap<Uuid, NodeData> = universe_map
            .into_iter()
            .map(|(id, map)| (id, snapshot_to_node(&map, id)))
            .collect();

        // Focus events for the day
        let focus_events: Vec<FocusEvent> = self.focus.trail_between(day_start, day_end);

        // Journal entries for the day
        let journal_entries: Vec<JournalEntry> = self.journal.between(day_start, day_end);

        // Derive aura from reconstructed node states
        let nodes_ref: Vec<&NodeData> = node_states.values().collect();
        let mut weather = WeatherSystem::new();
        weather.derive(&nodes_ref, &focus_events, day_end);
        let aura_signature = aura_from_weather(&weather);

        // Compute stats
        let nodes_touched: std::collections::HashSet<Uuid> =
            focus_events.iter().map(|e| e.node_id).collect();
        let total_focus_seconds: f32 = focus_events.iter().map(|e| e.duration_seconds).sum();
        let dominant_node = {
            let mut acc: HashMap<Uuid, f32> = HashMap::new();
            for e in &focus_events {
                *acc.entry(e.node_id).or_default() += e.duration_seconds;
            }
            acc.into_iter()
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(id, _)| id)
        };

        DayReconstruction {
            date,
            node_states,
            focus_events,
            journal_entries,
            aura_signature,
            nodes_touched: nodes_touched.len(),
            total_focus_seconds,
            dominant_node,
        }
    }

    /// Compare two calendar days and produce a structural diff.
    pub fn compare_days(&self, day_a: NaiveDate, day_b: NaiveDate) -> DayComparison {
        let rec_a = self.reconstruct_day(day_a);
        let rec_b = self.reconstruct_day(day_b);

        let ids_a: std::collections::HashSet<Uuid> = rec_a.node_states.keys().cloned().collect();
        let ids_b: std::collections::HashSet<Uuid> = rec_b.node_states.keys().cloned().collect();

        let new_nodes: Vec<Uuid> = ids_b.difference(&ids_a).cloned().collect();
        let removed_nodes: Vec<Uuid> = ids_a.difference(&ids_b).cloned().collect();

        let mut changed_nodes = Vec::new();
        for id in ids_a.intersection(&ids_b) {
            let na = &rec_a.node_states[id];
            let nb = &rec_b.node_states[id];
            let content_changed = na.content != nb.content;
            let entropy_delta = nb.entropy - na.entropy;
            let access_delta = nb.access_count as i64 - na.access_count as i64;
            let became_ghost = !na.is_ghost && nb.is_ghost;
            let became_fossil = !na.is_fossil && nb.is_fossil;
            if content_changed
                || entropy_delta.abs() > 0.001
                || access_delta != 0
                || became_ghost
                || became_fossil
            {
                changed_nodes.push(NodeChangeSummary {
                    node_id: *id,
                    entropy_delta,
                    access_delta,
                    became_ghost,
                    became_fossil,
                    content_changed,
                });
            }
        }

        DayComparison {
            day_a,
            day_b,
            new_nodes,
            removed_nodes,
            changed_nodes,
            focus_delta_seconds: rec_b.total_focus_seconds - rec_a.total_focus_seconds,
            journal_entry_delta: rec_b.journal_entries.len() as i32
                - rec_a.journal_entries.len() as i32,
        }
    }
}

fn aura_from_weather(weather: &WeatherSystem) -> AuraSignature {
    let s = &weather.current;
    AuraSignature {
        state_name: s.name().to_string(),
        intensity: s.intensity(),
        turbulence: s.turbulence(),
        pulse_rate: s.pulse_rate(),
        primary_color: s.primary_color(),
        secondary_color: s.secondary_color(),
    }
}
