use crate::domain::JournalEntry;
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct JournalEngine {
    entries: Vec<JournalEntry>,
}

impl JournalEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_entries(entries: Vec<JournalEntry>) -> Self {
        Self { entries }
    }

    pub fn add_entry(
        &mut self,
        content: impl Into<String>,
        linked_nodes: Vec<Uuid>,
        season: Option<String>,
    ) -> JournalEntry {
        let entry = JournalEntry::new(content, linked_nodes, season, BTreeMap::new());
        self.entries.push(entry.clone());
        entry
    }

    pub fn entries(&self) -> &[JournalEntry] {
        &self.entries
    }

    pub fn search(&self, query: &str) -> Vec<JournalEntry> {
        let needle = query.to_lowercase();
        self.entries
            .iter()
            .filter(|entry| entry.content.to_lowercase().contains(&needle))
            .cloned()
            .collect()
    }

    pub fn between(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<JournalEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.timestamp >= from && entry.timestamp <= to)
            .cloned()
            .collect()
    }
}
