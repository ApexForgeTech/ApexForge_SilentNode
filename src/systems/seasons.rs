use crate::domain::{FocusEvent, JournalEntry, NodeData};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CognitiveSeason {
    Spring,
    Summer,
    Autumn,
    Winter,
}

impl CognitiveSeason {
    pub fn name(self) -> &'static str {
        match self {
            Self::Spring => "Spring",
            Self::Summer => "Summer",
            Self::Autumn => "Autumn",
            Self::Winter => "Winter",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonReport {
    pub season: CognitiveSeason,
    pub creation_rate: f32,
    pub focus_density: f32,
    pub exploration_ratio: f32,
    pub avg_entropy: f32,
    pub revisit_ratio: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveSeasonDetector;

impl Default for CognitiveSeasonDetector {
    fn default() -> Self {
        Self
    }
}

impl CognitiveSeasonDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detect_season(
        &self,
        nodes: &[&NodeData],
        focus_events: &[FocusEvent],
        journal_entries: &[JournalEntry],
        now: DateTime<Utc>,
    ) -> SeasonReport {
        let window_start = now - Duration::days(30);
        let total = nodes.len().max(1);

        // ── Creation rate ─────────────────────────────────────────────────────
        let created_in_window = nodes
            .iter()
            .filter(|n| n.created_at >= window_start)
            .count();
        let creation_rate = (created_in_window as f32 / total as f32).clamp(0.0, 1.0);

        // ── Focus events in window ────────────────────────────────────────────
        let events_in_window: Vec<&FocusEvent> = focus_events
            .iter()
            .filter(|e| e.timestamp >= window_start)
            .collect();

        let focus_density = (events_in_window.len() as f32 / 20.0).clamp(0.0, 1.0);

        // ── Exploration ratio ─────────────────────────────────────────────────
        let unique_in_window: HashSet<uuid::Uuid> =
            events_in_window.iter().map(|e| e.node_id).collect();
        let exploration_ratio = (unique_in_window.len() as f32 / total as f32).clamp(0.0, 1.0);

        // ── Average entropy ───────────────────────────────────────────────────
        let avg_entropy = if nodes.is_empty() {
            0.0
        } else {
            nodes.iter().map(|n| n.entropy).sum::<f32>() / total as f32
        };

        // ── Revisit ratio ─────────────────────────────────────────────────────
        let revisit_count = events_in_window
            .iter()
            .filter(|e| {
                nodes
                    .iter()
                    .find(|n| n.id == e.node_id)
                    .map(|node| (e.timestamp - node.created_at).num_days() > 7)
                    .unwrap_or(false)
            })
            .count();
        let revisit_ratio = if events_in_window.is_empty() {
            0.0
        } else {
            (revisit_count as f32 / events_in_window.len() as f32).clamp(0.0, 1.0)
        };

        // ── Journal-based signals (vision.md: "journal activity patterns") ───
        let journal_in_window: Vec<&JournalEntry> = journal_entries
            .iter()
            .filter(|j| j.timestamp >= window_start)
            .collect();

        // Journal density: how active is journaling?
        let journal_density = (journal_in_window.len() as f32 / 10.0).clamp(0.0, 1.0);

        // Keyword scan for seasonal markers
        let (spring_keywords, winter_keywords, autumn_keywords) =
            scan_journal_keywords(&journal_in_window);

        // ── Connection formation rate ─────────────────────────────────────────
        // Proxy: average degree of nodes created in the last 30 days
        // (nodes created in window that have > 0 connections suggest connectivity burst)
        let new_nodes_connected = nodes
            .iter()
            .filter(|n| n.created_at >= window_start && n.gravity > 1.5)
            .count();
        let connectivity_burst =
            (new_nodes_connected as f32 / (created_in_window.max(1) as f32)).clamp(0.0, 1.0);

        // ── Season classification ─────────────────────────────────────────────
        //
        // Spring: new ideas, high exploration, new connections, journal mentions of beginnings
        // Summer: deep focus, high output, sustained activity on specific areas
        // Autumn: revisiting, consolidation, journal-heavy, past-oriented keywords
        // Winter: low activity across all signals, high void/journal introspection

        let spring_score = creation_rate * 0.35
            + exploration_ratio * 0.25
            + connectivity_burst * 0.20
            + spring_keywords * 0.20;

        let summer_score = focus_density * 0.45 + creation_rate * 0.25 + (1.0 - avg_entropy) * 0.30;

        let autumn_score = revisit_ratio * 0.40
            + journal_density * 0.25
            + autumn_keywords * 0.20
            + avg_entropy * 0.15;

        let winter_score = (1.0 - focus_density) * 0.35
            + (1.0 - creation_rate) * 0.25
            + winter_keywords * 0.25
            + avg_entropy * 0.15;

        let season = [
            (CognitiveSeason::Spring, spring_score),
            (CognitiveSeason::Summer, summer_score),
            (CognitiveSeason::Autumn, autumn_score),
            (CognitiveSeason::Winter, winter_score),
        ]
        .into_iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(s, _)| s)
        .unwrap_or(CognitiveSeason::Winter);

        SeasonReport {
            season,
            creation_rate,
            focus_density,
            exploration_ratio,
            avg_entropy,
            revisit_ratio,
        }
    }
}

// ── Journal keyword scanner ───────────────────────────────────────────────────

/// Scan journal entries for seasonal keyword clusters.
/// Returns (spring_score, winter_score, autumn_score) in [0, 1].
fn scan_journal_keywords(entries: &[&JournalEntry]) -> (f32, f32, f32) {
    if entries.is_empty() {
        return (0.0, 0.0, 0.0);
    }

    let spring_words: &[&str] = &[
        "start", "begin", "new", "launch", "idea", "explore", "discover", "plan", "create",
        "build", "design", "fresh", "excited", "energy",
    ];
    let winter_words: &[&str] = &[
        "rest", "pause", "silence", "reflect", "quiet", "slow", "empty", "incubate", "wait",
        "stuck", "tired", "burnout", "void", "nothing",
    ];
    let autumn_words: &[&str] = &[
        "review",
        "revisit",
        "harvest",
        "consolidate",
        "archive",
        "finish",
        "complete",
        "learn",
        "understand",
        "past",
        "history",
        "process",
    ];

    let total_words = entries
        .iter()
        .map(|j| j.content.split_whitespace().count())
        .sum::<usize>()
        .max(1) as f32;

    let count_matches = |words: &[&str]| -> f32 {
        entries
            .iter()
            .flat_map(|j| j.content.split_whitespace())
            .filter(|w| words.iter().any(|kw| w.to_lowercase().contains(kw)))
            .count() as f32
    };

    let spring_count = count_matches(spring_words);
    let winter_count = count_matches(winter_words);
    let autumn_count = count_matches(autumn_words);

    let norm = (total_words / 50.0).max(1.0);
    (
        (spring_count / norm).clamp(0.0, 1.0),
        (winter_count / norm).clamp(0.0, 1.0),
        (autumn_count / norm).clamp(0.0, 1.0),
    )
}

/// Returns (primary_color, secondary_color) as RGBA [f32; 4] for the given season.
pub fn season_aura_colors(season: CognitiveSeason) -> ([f32; 4], [f32; 4]) {
    match season {
        CognitiveSeason::Spring => ([0.02, 0.05, 0.10, 1.0], [0.20, 0.80, 0.45, 1.0]),
        CognitiveSeason::Summer => ([0.06, 0.04, 0.02, 1.0], [1.00, 0.75, 0.20, 1.0]),
        CognitiveSeason::Autumn => ([0.06, 0.02, 0.01, 1.0], [0.85, 0.40, 0.10, 1.0]),
        CognitiveSeason::Winter => ([0.02, 0.03, 0.08, 1.0], [0.45, 0.55, 0.95, 1.0]),
    }
}
