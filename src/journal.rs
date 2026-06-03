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

    pub fn entries_mut(&mut self) -> &mut [JournalEntry] {
        &mut self.entries
    }

    pub fn get(&self, id: Uuid) -> Option<&JournalEntry> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    pub fn update_entry(
        &mut self,
        id: Uuid,
        content: String,
        linked_nodes: Vec<Uuid>,
        season: Option<String>,
    ) -> Option<JournalEntry> {
        let entry = self.entries.iter_mut().find(|entry| entry.id == id)?;
        entry.content = content;
        entry.linked_nodes = linked_nodes;
        entry.season = season;
        Some(entry.clone())
    }

    pub fn remove_entry(&mut self, id: Uuid) -> Option<JournalEntry> {
        let index = self.entries.iter().position(|entry| entry.id == id)?;
        Some(self.entries.remove(index))
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
