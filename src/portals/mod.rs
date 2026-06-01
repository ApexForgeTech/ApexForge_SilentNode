/// Phase 8.2 — Portal System
///
/// Portals are integrated access points within SilentNode that provide
/// contained, tracked, and graph-connected interfaces to external services.
///
/// A Portal is not a browser tab. It is a first-class entity within the
/// SilentNode universe. Every action inside a Portal generates graph data.
///
/// Vision.md portal types: Media, Knowledge, Code, Communication, Filesystem
use crate::domain::{FocusDepth, FocusEvent};
use crate::membrane::{CrossingDirection, DataType, DigitalMembrane};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

// ── Portal type & activity ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortalType {
    /// YouTube, video platforms, podcast feeds.
    Media,
    /// Documentation sites, research databases, wikis.
    Knowledge,
    /// GitHub, GitLab, package registries.
    Code,
    /// Email, messaging.
    Communication,
    /// Native filesystem access.
    Filesystem,
}

impl PortalType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Media => "media",
            Self::Knowledge => "knowledge",
            Self::Code => "code",
            Self::Communication => "communication",
            Self::Filesystem => "filesystem",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityKind {
    /// User viewed content.
    View,
    /// User navigated to a resource.
    Navigate,
    /// User submitted a form or message.
    Submit,
    /// User downloaded a resource.
    Download,
    /// User uploaded content.
    Upload,
    /// User searched.
    Search,
    /// User linked a resource to a cognitive node.
    Link,
    /// User played media.
    Play,
}

/// A single activity event recorded by a portal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalActivity {
    pub id: Uuid,
    pub portal_id: Uuid,
    pub portal_type: PortalType,
    pub kind: ActivityKind,
    /// URL, path, search query, or identifier.
    pub target: String,
    /// Content title or description (if available).
    pub title: String,
    /// Topics extracted from content (populated during ingestion).
    pub topics: Vec<String>,
    /// Cognitive node this activity was linked to (if any).
    pub linked_node: Option<Uuid>,
    pub duration_seconds: f32,
    pub timestamp: DateTime<Utc>,
}

impl PortalActivity {
    pub fn new(
        portal_id: Uuid,
        portal_type: PortalType,
        kind: ActivityKind,
        target: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            portal_id,
            portal_type,
            kind,
            target: target.into(),
            title: String::new(),
            topics: Vec::new(),
            linked_node: None,
            duration_seconds: 0.0,
            timestamp: Utc::now(),
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn with_duration(mut self, secs: f32) -> Self {
        self.duration_seconds = secs;
        self
    }

    pub fn link_to(mut self, node_id: Uuid) -> Self {
        self.linked_node = Some(node_id);
        self
    }

    /// Convert to a FocusEvent for the linked node (if any).
    pub fn to_focus_event(&self) -> Option<FocusEvent> {
        let node_id = self.linked_node?;
        let depth = match self.kind {
            ActivityKind::View | ActivityKind::Play => FocusDepth::Read,
            ActivityKind::Navigate | ActivityKind::Search => FocusDepth::Glance,
            ActivityKind::Submit | ActivityKind::Upload => FocusDepth::Edit,
            ActivityKind::Download | ActivityKind::Link => FocusDepth::Read,
        };
        Some(FocusEvent {
            node_id,
            timestamp: self.timestamp,
            duration_seconds: self.duration_seconds,
            depth,
            session_id: Uuid::new_v4(),
        })
    }
}

// ── Ingestion proposal ────────────────────────────────────────────────────────

/// A proposed change to the cognitive graph based on portal activity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IngestionProposalKind {
    /// Create a new World Node from external content.
    CreateWorldNode { content: String, source_url: String },
    /// Strengthen an existing node by increasing gravity.
    StrengthenNode { node_id: Uuid, gravity_boost: f32 },
    /// Link this activity to an existing node.
    LinkToNode { node_id: Uuid },
    /// Record focus time on a linked node.
    RecordFocus {
        node_id: Uuid,
        duration_seconds: f32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionProposal {
    pub id: Uuid,
    pub from_activity_id: Uuid,
    pub kind: IngestionProposalKind,
    pub confidence: f32,
    pub reason: String,
    pub accepted: Option<bool>,
}

impl IngestionProposal {
    pub fn create_world_node(
        activity: &PortalActivity,
        content: impl Into<String>,
        source: impl Into<String>,
        confidence: f32,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            from_activity_id: activity.id,
            kind: IngestionProposalKind::CreateWorldNode {
                content: content.into(),
                source_url: source.into(),
            },
            confidence,
            reason: reason.into(),
            accepted: None,
        }
    }

    pub fn strengthen(
        activity: &PortalActivity,
        node_id: Uuid,
        boost: f32,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            from_activity_id: activity.id,
            kind: IngestionProposalKind::StrengthenNode {
                node_id,
                gravity_boost: boost,
            },
            confidence: 0.7,
            reason: reason.into(),
            accepted: None,
        }
    }
}

// ── Portal implementations ────────────────────────────────────────────────────

/// Filesystem portal: tracks file access and modification activity in a directory.
/// Renders the filesystem as part of the cognitive universe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemPortal {
    pub id: Uuid,
    pub root_path: PathBuf,
    pub log: Vec<PortalActivity>,
    /// Maximum log entries.
    pub capacity: usize,
}

impl FilesystemPortal {
    pub fn new(root_path: impl AsRef<Path>) -> Self {
        Self {
            id: Uuid::new_v4(),
            root_path: root_path.as_ref().to_path_buf(),
            log: Vec::new(),
            capacity: 500,
        }
    }

    pub fn portal_type(&self) -> PortalType {
        PortalType::Filesystem
    }

    /// Record a file access event.
    pub fn record_access(&mut self, path: &Path, kind: ActivityKind, duration_secs: f32) {
        let activity = PortalActivity::new(
            self.id,
            PortalType::Filesystem,
            kind,
            path.display().to_string(),
        )
        .with_duration(duration_secs);
        self.push(activity);
    }

    fn push(&mut self, activity: PortalActivity) {
        self.log.push(activity);
        if self.log.len() > self.capacity {
            self.log.remove(0);
        }
    }

    /// Scan the root directory and return a list of entries with modification times.
    /// Returns (path, is_dir, modified_secs_ago).
    pub fn scan_entries(&self) -> Vec<(String, bool, f32)> {
        let now = std::time::SystemTime::now();
        let mut entries = Vec::new();
        if let Ok(read_dir) = std::fs::read_dir(&self.root_path) {
            for entry in read_dir.flatten() {
                let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                let modified_secs = entry
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| now.duration_since(t).ok())
                    .map(|d| d.as_secs_f32())
                    .unwrap_or(f32::MAX);
                entries.push((
                    entry.file_name().to_string_lossy().into_owned(),
                    is_dir,
                    modified_secs,
                ));
            }
        }
        entries.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
        entries
    }

    pub fn activity_log(&self) -> &[PortalActivity] {
        &self.log
    }
}

/// Generic external resource portal (for Media, Knowledge, Code, Communication).
/// Records activity against a named external service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalPortal {
    pub id: Uuid,
    pub name: String,
    pub portal_type: PortalType,
    pub base_url: String,
    pub log: Vec<PortalActivity>,
    pub capacity: usize,
}

impl ExternalPortal {
    pub fn new(
        name: impl Into<String>,
        portal_type: PortalType,
        base_url: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            portal_type,
            base_url: base_url.into(),
            log: Vec::new(),
            capacity: 500,
        }
    }

    /// Log an activity through this portal; also checks it against the membrane.
    pub fn record(
        &mut self,
        kind: ActivityKind,
        target: impl Into<String>,
        title: impl Into<String>,
        duration_secs: f32,
        membrane: Option<&mut DigitalMembrane>,
    ) -> PortalActivity {
        let target = target.into();

        if let Some(mb) = membrane {
            let dir = match kind {
                ActivityKind::Upload | ActivityKind::Submit => CrossingDirection::Outbound,
                _ => CrossingDirection::Inbound,
            };
            mb.check(
                &target,
                dir,
                DataType::Any,
                0,
                format!("portal:{}", self.name),
            );
        }

        let activity = PortalActivity::new(self.id, self.portal_type.clone(), kind, &target)
            .with_title(title)
            .with_duration(duration_secs);

        self.log.push(activity.clone());
        if self.log.len() > self.capacity {
            self.log.remove(0);
        }
        activity
    }

    pub fn activity_log(&self) -> &[PortalActivity] {
        &self.log
    }

    /// Total focus time logged through this portal (seconds).
    pub fn total_time_seconds(&self) -> f32 {
        self.log.iter().map(|a| a.duration_seconds).sum()
    }

    /// Most frequent targets (top-k by occurrence count).
    pub fn top_targets(&self, k: usize) -> Vec<(String, usize)> {
        let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for a in &self.log {
            *counts.entry(a.target.clone()).or_default() += 1;
        }
        let mut sorted: Vec<(String, usize)> = counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(k);
        sorted
    }
}

// ── PortalManager ─────────────────────────────────────────────────────────────

/// Manages all active portals and aggregates their activity for ingestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalManager {
    pub filesystem_portals: Vec<FilesystemPortal>,
    pub external_portals: Vec<ExternalPortal>,
}

impl Default for PortalManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PortalManager {
    pub fn new() -> Self {
        Self {
            filesystem_portals: Vec::new(),
            external_portals: Vec::new(),
        }
    }

    pub fn add_filesystem_portal(&mut self, portal: FilesystemPortal) {
        self.filesystem_portals.push(portal);
    }

    pub fn add_external_portal(&mut self, portal: ExternalPortal) {
        self.external_portals.push(portal);
    }

    /// Find an external portal by name.
    pub fn external_portal_mut(&mut self, name: &str) -> Option<&mut ExternalPortal> {
        self.external_portals.iter_mut().find(|p| p.name == name)
    }

    /// All activity across all portals, sorted by timestamp descending.
    pub fn all_activity(&self) -> Vec<&PortalActivity> {
        let mut all: Vec<&PortalActivity> = self
            .filesystem_portals
            .iter()
            .flat_map(|p| p.activity_log())
            .chain(self.external_portals.iter().flat_map(|p| p.activity_log()))
            .collect();
        all.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        all
    }

    /// Total tracked portals.
    pub fn count(&self) -> usize {
        self.filesystem_portals.len() + self.external_portals.len()
    }

    /// Total events logged across all portals.
    pub fn total_events(&self) -> usize {
        self.filesystem_portals
            .iter()
            .map(|p| p.log.len())
            .sum::<usize>()
            + self
                .external_portals
                .iter()
                .map(|p| p.log.len())
                .sum::<usize>()
    }

    /// Time distribution: total seconds per portal type.
    pub fn time_by_type(&self) -> std::collections::HashMap<String, f32> {
        let mut map: std::collections::HashMap<String, f32> = std::collections::HashMap::new();
        for p in &self.external_portals {
            *map.entry(p.portal_type.as_str().to_string()).or_default() += p.total_time_seconds();
        }
        map
    }

    pub fn print_status(&self) {
        println!("══ Portal Manager ══════════════════════════════════════════");
        println!(
            "  Portals: {} filesystem, {} external",
            self.filesystem_portals.len(),
            self.external_portals.len()
        );
        println!("  Total events: {}", self.total_events());
        for p in &self.external_portals {
            println!(
                "  [{}] {} — {} events, {:.0}s total",
                p.portal_type.as_str(),
                p.name,
                p.log.len(),
                p.total_time_seconds()
            );
        }
        for p in &self.filesystem_portals {
            println!(
                "  [filesystem] {} — {} events",
                p.root_path.display(),
                p.log.len()
            );
        }
    }
}
