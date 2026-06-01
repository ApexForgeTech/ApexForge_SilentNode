use crate::domain::{FocusDepth, FocusEvent};
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct FocusTrailEngine {
    events: Vec<FocusEvent>,
}

impl FocusTrailEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_events(events: Vec<FocusEvent>) -> Self {
        Self { events }
    }

    pub fn record(
        &mut self,
        node_id: Uuid,
        duration_seconds: f32,
        depth: FocusDepth,
    ) -> FocusEvent {
        let event = FocusEvent {
            node_id,
            timestamp: Utc::now(),
            duration_seconds,
            depth,
            session_id: Uuid::new_v4(),
        };
        self.events.push(event.clone());
        event
    }

    pub fn events(&self) -> &[FocusEvent] {
        &self.events
    }

    pub fn trail_between(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<FocusEvent> {
        self.events
            .iter()
            .filter(|event| event.timestamp >= from && event.timestamp <= to)
            .cloned()
            .collect()
    }

    pub fn active_nodes_since(&self, since: DateTime<Utc>) -> Vec<Uuid> {
        let mut seen = HashSet::new();
        let mut nodes = Vec::new();

        for event in self.events.iter().rev() {
            if event.timestamp < since {
                break;
            }
            if seen.insert(event.node_id) {
                nodes.push(event.node_id);
            }
        }

        nodes.reverse();
        nodes
    }

    pub fn heatmap(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> HashMap<Uuid, f32> {
        let mut values = HashMap::<Uuid, f32>::new();

        for event in self
            .events
            .iter()
            .filter(|event| event.timestamp >= from && event.timestamp <= to)
        {
            let age_seconds = (to - event.timestamp).num_seconds().max(0) as f32;
            let recency = 1.0 / (1.0 + age_seconds / 3600.0);
            let entry = values.entry(event.node_id).or_insert(0.0);
            *entry += event.duration_seconds * event.depth.weight() * recency;
        }

        let max_value = values.values().copied().fold(0.0, f32::max);
        if max_value > 0.0 {
            for value in values.values_mut() {
                *value /= max_value;
            }
        }

        values
    }
}
