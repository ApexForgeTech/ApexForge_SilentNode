/// Phase 8.5 — Calendar & Temporal Intelligence Layer
///
/// Time is not external to SilentNode. Time is woven into the fabric of the universe.
///
/// Calendar events exist as Temporal Nodes within the graph:
///   • they carry gravity — approaching events increase in visual mass,
///   • they cast shadows backward — preparation activity is linked to the event,
///   • they leave impressions forward — post-event reflections are linked.
///
/// Vision.md: SilentNode does not interrupt. Events cause visual shifts,
/// not alerts. The user's attention is drawn — never grabbed.
use crate::domain::FocusEvent;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── CalendarEvent ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventCategory {
    Meeting,
    Deadline,
    Task,
    Review,
    Personal,
    Recurring,
    Milestone,
}

impl EventCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Meeting => "meeting",
            Self::Deadline => "deadline",
            Self::Task => "task",
            Self::Review => "review",
            Self::Personal => "personal",
            Self::Recurring => "recurring",
            Self::Milestone => "milestone",
        }
    }

    /// Gravity multiplier: deadlines pull harder than personal events.
    pub fn gravity_multiplier(&self) -> f32 {
        match self {
            Self::Deadline => 3.0,
            Self::Milestone => 2.5,
            Self::Meeting => 2.0,
            Self::Task => 1.9,
            Self::Review => 1.8,
            Self::Recurring => 1.2,
            Self::Personal => 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub category: EventCategory,
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    /// Cognitive nodes this event relates to.
    pub linked_nodes: Vec<Uuid>,
    /// True if this event repeats on a regular cadence.
    pub is_recurring: bool,
    /// How many days before the event the system begins signalling it.
    pub anticipation_days: u32,
    pub created_at: DateTime<Utc>,
}

impl CalendarEvent {
    pub fn new(
        title: impl Into<String>,
        category: EventCategory,
        start_at: DateTime<Utc>,
        end_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            description: String::new(),
            category,
            start_at,
            end_at,
            linked_nodes: Vec::new(),
            is_recurring: false,
            anticipation_days: 3,
            created_at: Utc::now(),
        }
    }

    /// Duration of the event in minutes.
    pub fn duration_minutes(&self) -> i64 {
        (self.end_at - self.start_at).num_minutes().max(0)
    }

    /// How many hours until this event starts (negative if in the past).
    pub fn hours_until(&self, now: DateTime<Utc>) -> f32 {
        (self.start_at - now).num_seconds() as f32 / 3600.0
    }

    /// Gravity at `now`: increases as the event approaches, peaks at start.
    /// Returns 0.0 if more than `anticipation_days` away.
    pub fn computed_gravity(&self, now: DateTime<Utc>) -> f32 {
        let hours = self.hours_until(now);
        if hours < 0.0 {
            return 0.0; // past event
        }
        let total_anticipation_hours = self.anticipation_days as f32 * 24.0;
        if hours > total_anticipation_hours {
            return 0.0; // too far away
        }
        let fraction = 1.0 - (hours / total_anticipation_hours);
        (fraction * self.category.gravity_multiplier()).min(5.0)
    }

    pub fn is_approaching(&self, now: DateTime<Utc>, within_hours: f32) -> bool {
        let h = self.hours_until(now);
        h >= 0.0 && h <= within_hours
    }

    pub fn is_past(&self, now: DateTime<Utc>) -> bool {
        self.end_at < now
    }
}

// ── PreparationAnalysis ───────────────────────────────────────────────────────

/// Analysis of how the user prepares for an event type based on history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparationAnalysis {
    pub event_id: Uuid,
    /// Average hours before event when preparation focus activity peaks.
    pub avg_prep_start_hours_before: f32,
    /// Fraction of similar past events that had detectable preparation.
    pub prep_rate: f32,
    /// Whether preparation activity was detected for this specific event.
    pub has_prep_activity: bool,
    /// Procrastination score 0–1 (1 = no prep detected, event is close).
    pub procrastination_score: f32,
    /// Linked nodes that were focused on during prep windows for similar events.
    pub prep_nodes: Vec<Uuid>,
}

// ── CalendarIntelligence ──────────────────────────────────────────────────────

/// Derives temporal intelligence from calendar events and focus history.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CalendarIntelligence;

impl CalendarIntelligence {
    pub fn new() -> Self {
        Self
    }

    /// Analyse preparation behavior for `event` by looking at focus events
    /// in the window `[event.start_at - lookback_days, event.start_at]`.
    pub fn analyze_preparation(
        &self,
        event: &CalendarEvent,
        focus_events: &[FocusEvent],
        lookback_days: u32,
    ) -> PreparationAnalysis {
        let prep_window_start = event.start_at - Duration::days(lookback_days as i64);

        // Focus events in the preparation window on linked nodes
        let prep_events: Vec<&FocusEvent> = focus_events
            .iter()
            .filter(|e| {
                e.timestamp >= prep_window_start
                    && e.timestamp < event.start_at
                    && (event.linked_nodes.is_empty() || event.linked_nodes.contains(&e.node_id))
            })
            .collect();

        let has_prep_activity = !prep_events.is_empty();

        // Find earliest prep activity
        let earliest_prep_hours = prep_events
            .iter()
            .map(|e| (event.start_at - e.timestamp).num_seconds() as f32 / 3600.0)
            .fold(f32::NEG_INFINITY, f32::max);

        let avg_prep_start_hours_before = if has_prep_activity {
            earliest_prep_hours
        } else {
            0.0
        };

        let prep_nodes: Vec<Uuid> = prep_events
            .iter()
            .map(|e| e.node_id)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        // Procrastination: if event is within 24h and no prep detected
        let hours_until = event.hours_until(Utc::now());
        let procrastination_score =
            if !has_prep_activity && hours_until >= 0.0 && hours_until <= 24.0 {
                (1.0 - hours_until / 24.0).clamp(0.0, 1.0)
            } else {
                0.0
            };

        PreparationAnalysis {
            event_id: event.id,
            avg_prep_start_hours_before,
            prep_rate: if has_prep_activity { 1.0 } else { 0.0 },
            has_prep_activity,
            procrastination_score,
            prep_nodes,
        }
    }

    /// Suggest optimal focus windows based on historical trail activity.
    /// Returns `(date, hour, reason)` tuples representing suggested windows.
    pub fn suggest_focus_windows(
        &self,
        focus_events: &[FocusEvent],
        upcoming: &[CalendarEvent],
        now: DateTime<Utc>,
    ) -> Vec<(NaiveDate, u8, String)> {
        use chrono::Timelike;

        // Find hours of day with the most deep work in the last 30 days
        let window_start = now - Duration::days(30);
        let mut hour_counts = [0u32; 24];
        for e in focus_events.iter().filter(|e| e.timestamp >= window_start) {
            hour_counts[e.timestamp.hour() as usize] += 1;
        }

        let peak_hour = hour_counts
            .iter()
            .enumerate()
            .max_by_key(|(_, &c)| c)
            .map(|(h, _)| h as u8)
            .unwrap_or(9);

        // Suggest windows for the next 7 days
        let mut suggestions = Vec::new();
        for days_ahead in 1u32..=7 {
            let date = (now + Duration::days(days_ahead as i64)).date_naive();

            // Check if any event is that day
            let has_event = upcoming.iter().any(|ev| ev.start_at.date_naive() == date);

            let reason = if has_event {
                format!("Event scheduled — prepare at peak hour {}", peak_hour)
            } else {
                format!("Open day — recommended focus window at hour {}", peak_hour)
            };

            suggestions.push((date, peak_hour, reason));
        }
        suggestions
    }
}

// ── CalendarEngine ────────────────────────────────────────────────────────────

/// Stores and queries the local calendar.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CalendarEngine {
    events: Vec<CalendarEvent>,
    pub intelligence: CalendarIntelligence,
}

impl CalendarEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_events(mut events: Vec<CalendarEvent>) -> Self {
        events.sort_by_key(|e| e.start_at);
        Self {
            events,
            intelligence: CalendarIntelligence::new(),
        }
    }

    pub fn add_event(&mut self, event: CalendarEvent) {
        self.events.push(event);
        self.events.sort_by_key(|e| e.start_at);
    }

    pub fn remove_event(&mut self, event_id: Uuid) {
        self.events.retain(|e| e.id != event_id);
    }

    pub fn get_event(&self, event_id: Uuid) -> Option<&CalendarEvent> {
        self.events.iter().find(|e| e.id == event_id)
    }

    pub fn get_event_mut(&mut self, event_id: Uuid) -> Option<&mut CalendarEvent> {
        self.events.iter_mut().find(|e| e.id == event_id)
    }

    /// All events, sorted chronologically.
    pub fn all_events(&self) -> &[CalendarEvent] {
        &self.events
    }

    /// Upcoming events (start_at >= now), sorted soonest first.
    pub fn upcoming(&self, now: DateTime<Utc>) -> Vec<&CalendarEvent> {
        let mut v: Vec<&CalendarEvent> = self.events.iter().filter(|e| e.start_at >= now).collect();
        v.sort_by_key(|e| e.start_at);
        v
    }

    /// Events happening today.
    pub fn today(&self, now: DateTime<Utc>) -> Vec<&CalendarEvent> {
        let today = now.date_naive();
        self.events
            .iter()
            .filter(|e| e.start_at.date_naive() == today)
            .collect()
    }

    /// Events within `hours` from now.
    pub fn approaching(&self, now: DateTime<Utc>, within_hours: f32) -> Vec<&CalendarEvent> {
        self.events
            .iter()
            .filter(|e| e.is_approaching(now, within_hours))
            .collect()
    }

    /// The highest-gravity event right now (None if no events are in anticipation range).
    pub fn dominant_event(&self, now: DateTime<Utc>) -> Option<&CalendarEvent> {
        self.events
            .iter()
            .filter(|e| e.computed_gravity(now) > 0.0)
            .max_by(|a, b| {
                a.computed_gravity(now)
                    .partial_cmp(&b.computed_gravity(now))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// All events linked to a specific cognitive node.
    pub fn events_for_node(&self, node_id: Uuid) -> Vec<&CalendarEvent> {
        self.events
            .iter()
            .filter(|e| e.linked_nodes.contains(&node_id))
            .collect()
    }

    /// Analyze preparation for an event.
    pub fn analyze_preparation(
        &self,
        event_id: Uuid,
        focus_events: &[FocusEvent],
        lookback_days: u32,
    ) -> Option<PreparationAnalysis> {
        let event = self.get_event(event_id)?;
        Some(
            self.intelligence
                .analyze_preparation(event, focus_events, lookback_days),
        )
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn print_status(&self, now: DateTime<Utc>) {
        println!("══ Calendar Engine ═════════════════════════════════════════");
        println!("  Total events: {}", self.events.len());
        let upcoming = self.upcoming(now);
        println!("  Upcoming: {}", upcoming.len());
        for ev in upcoming.iter().take(5) {
            let hours = ev.hours_until(now);
            let grav = ev.computed_gravity(now);
            println!(
                "  [{:7}] {:40} in {:.1}h  gravity={:.2}",
                ev.category.as_str(),
                ev.title.chars().take(40).collect::<String>(),
                hours,
                grav
            );
        }
        if let Some(dominant) = self.dominant_event(now) {
            println!(
                "  Dominant event: {} (gravity={:.2})",
                dominant.title,
                dominant.computed_gravity(now)
            );
        }
    }
}
