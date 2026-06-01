/// Phase 8.1 — Digital Membrane
///
/// The architectural boundary between SilentNode's internal universe and the
/// external digital world. Nothing enters or exits without passing through here.
///
/// Vision.md guarantees:
///   • permeable by choice  — the user determines what can pass through
///   • transparent          — all crossings are logged and visible
///   • bidirectional        — governing both inbound and outbound traffic
///   • non-negotiable       — no external service can bypass the membrane
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Rule types ────────────────────────────────────────────────────────────────

/// What type of crossing a rule governs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrossingDirection {
    Inbound,
    Outbound,
    Both,
}

/// Protocol category for matching.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Protocol {
    Http,
    Https,
    WebSocket,
    Filesystem,
    Process,
    Any,
    Custom(String),
}

/// Content / data type classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataType {
    Text,
    Media,
    Code,
    Archive,
    Credential,
    Binary,
    Any,
}

/// A single membrane rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MembraneRule {
    pub id: Uuid,
    /// Pattern to match against the source/destination (domain, path, or `*`).
    pub pattern: String,
    pub direction: CrossingDirection,
    pub protocol: Protocol,
    pub data_types: Vec<DataType>,
    /// If true: this rule **allows** crossings. If false: it **blocks** them.
    pub allow: bool,
    /// If true: requires explicit user confirmation before allowing.
    pub requires_approval: bool,
    pub description: String,
}

impl MembraneRule {
    pub fn allow(pattern: impl Into<String>, direction: CrossingDirection) -> Self {
        Self {
            id: Uuid::new_v4(),
            pattern: pattern.into(),
            direction,
            protocol: Protocol::Any,
            data_types: vec![DataType::Any],
            allow: true,
            requires_approval: false,
            description: String::new(),
        }
    }

    pub fn block(pattern: impl Into<String>, direction: CrossingDirection) -> Self {
        Self {
            id: Uuid::new_v4(),
            pattern: pattern.into(),
            direction,
            protocol: Protocol::Any,
            data_types: vec![DataType::Any],
            allow: false,
            requires_approval: false,
            description: String::new(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Returns true if this rule matches the given target/direction.
    pub fn matches(&self, target: &str, dir: &CrossingDirection) -> bool {
        let dir_match = match (&self.direction, dir) {
            (CrossingDirection::Both, _) => true,
            (CrossingDirection::Inbound, CrossingDirection::Inbound) => true,
            (CrossingDirection::Outbound, CrossingDirection::Outbound) => true,
            _ => false,
        };
        if !dir_match {
            return false;
        }
        if self.pattern == "*" {
            return true;
        }
        target.contains(self.pattern.as_str())
    }
}

// ── Decision & log ────────────────────────────────────────────────────────────

/// The membrane's decision for a crossing request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MembraneDecision {
    /// Crossing is permitted by an explicit whitelist rule.
    Allow,
    /// Crossing is blocked by a blacklist rule.
    Block,
    /// Crossing requires explicit user confirmation.
    RequiresApproval,
    /// No rule matched — default behaviour (configurable; here: Allow with logging).
    DefaultAllow,
}

impl MembraneDecision {
    pub fn is_permitted(&self) -> bool {
        matches!(self, Self::Allow | Self::DefaultAllow)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Block => "block",
            Self::RequiresApproval => "requires_approval",
            Self::DefaultAllow => "default_allow",
        }
    }
}

/// A single recorded crossing event (inbound or outbound).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossingEvent {
    pub id: Uuid,
    pub direction: CrossingDirection,
    /// Source or destination — URL, path, process name, etc.
    pub target: String,
    pub data_type: DataType,
    pub size_bytes: u64,
    pub decision: MembraneDecision,
    /// The rule that triggered the decision (None = default).
    pub matched_rule_id: Option<Uuid>,
    pub timestamp: DateTime<Utc>,
    pub description: String,
}

impl CrossingEvent {
    pub fn summary(&self) -> String {
        format!(
            "[{}] {} {} {} ({} bytes) → {}",
            self.timestamp.format("%H:%M:%S"),
            match self.direction {
                CrossingDirection::Inbound => "IN ",
                CrossingDirection::Outbound => "OUT",
                CrossingDirection::Both => "BI ",
            },
            self.decision.as_str(),
            self.target,
            self.size_bytes,
            self.description,
        )
    }
}

// ── DigitalMembrane ───────────────────────────────────────────────────────────

/// The Digital Membrane — the sovereign boundary of the SilentNode universe.
///
/// Rules are evaluated in insertion order; the first match wins.
/// If no rule matches, `default_policy` applies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigitalMembrane {
    pub rules: Vec<MembraneRule>,
    /// Maximum log entries kept in memory (FIFO rotation).
    pub log_capacity: usize,
    inbound_log: Vec<CrossingEvent>,
    outbound_log: Vec<CrossingEvent>,
    /// Default decision when no rule matches.
    pub default_policy: MembraneDecision,
}

impl Default for DigitalMembrane {
    fn default() -> Self {
        Self::new()
    }
}

impl DigitalMembrane {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            log_capacity: 1_000,
            inbound_log: Vec::new(),
            outbound_log: Vec::new(),
            default_policy: MembraneDecision::DefaultAllow,
        }
    }

    /// Add a rule. Rules are evaluated in insertion order; first match wins.
    pub fn add_rule(&mut self, rule: MembraneRule) {
        self.rules.push(rule);
    }

    /// Remove a rule by its UUID.
    pub fn remove_rule(&mut self, rule_id: Uuid) {
        self.rules.retain(|r| r.id != rule_id);
    }

    /// Evaluate a crossing request and record it in the appropriate log.
    pub fn check(
        &mut self,
        target: &str,
        direction: CrossingDirection,
        data_type: DataType,
        size_bytes: u64,
        description: impl Into<String>,
    ) -> MembraneDecision {
        let decision = self.evaluate(target, &direction);

        let event = CrossingEvent {
            id: Uuid::new_v4(),
            direction: direction.clone(),
            target: target.to_string(),
            data_type,
            size_bytes,
            decision: decision.clone(),
            matched_rule_id: self.find_matching_rule(target, &direction).map(|r| r.id),
            timestamp: Utc::now(),
            description: description.into(),
        };

        let log = match direction {
            CrossingDirection::Inbound => &mut self.inbound_log,
            CrossingDirection::Outbound => &mut self.outbound_log,
            CrossingDirection::Both => &mut self.inbound_log,
        };
        log.push(event);
        if log.len() > self.log_capacity {
            log.remove(0);
        }

        decision
    }

    fn evaluate(&self, target: &str, direction: &CrossingDirection) -> MembraneDecision {
        for rule in &self.rules {
            if rule.matches(target, direction) {
                if !rule.allow {
                    return MembraneDecision::Block;
                }
                if rule.requires_approval {
                    return MembraneDecision::RequiresApproval;
                }
                return MembraneDecision::Allow;
            }
        }
        self.default_policy.clone()
    }

    fn find_matching_rule(
        &self,
        target: &str,
        direction: &CrossingDirection,
    ) -> Option<&MembraneRule> {
        self.rules.iter().find(|r| r.matches(target, direction))
    }

    /// Full inbound crossing log.
    pub fn inbound_log(&self) -> &[CrossingEvent] {
        &self.inbound_log
    }

    /// Full outbound crossing log.
    pub fn outbound_log(&self) -> &[CrossingEvent] {
        &self.outbound_log
    }

    /// Combined log sorted by timestamp descending (most recent first).
    pub fn combined_log(&self) -> Vec<&CrossingEvent> {
        let mut all: Vec<&CrossingEvent> = self
            .inbound_log
            .iter()
            .chain(self.outbound_log.iter())
            .collect();
        all.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        all
    }

    /// Count of blocked crossings (inbound + outbound).
    pub fn blocked_count(&self) -> usize {
        self.inbound_log
            .iter()
            .chain(self.outbound_log.iter())
            .filter(|e| e.decision == MembraneDecision::Block)
            .count()
    }

    /// Integrity health: fraction of crossings that were explicitly allowed
    /// (vs default-allowed or blocked). Higher = more precisely governed.
    pub fn integrity_score(&self) -> f32 {
        let total = self.inbound_log.len() + self.outbound_log.len();
        if total == 0 {
            return 1.0;
        }
        let explicit = self
            .inbound_log
            .iter()
            .chain(self.outbound_log.iter())
            .filter(|e| {
                e.decision == MembraneDecision::Allow || e.decision == MembraneDecision::Block
            })
            .count();
        explicit as f32 / total as f32
    }

    /// Print a summary of membrane state to stdout.
    pub fn print_status(&self) {
        println!("══ Digital Membrane ═══════════════════════════════════════");
        println!("  Rules:        {}", self.rules.len());
        println!("  Default:      {}", self.default_policy.as_str());
        println!("  Inbound log:  {}", self.inbound_log.len());
        println!("  Outbound log: {}", self.outbound_log.len());
        println!("  Blocked:      {}", self.blocked_count());
        println!("  Integrity:    {:.1}%", self.integrity_score() * 100.0);
        if !self.rules.is_empty() {
            println!("  Active rules:");
            for r in &self.rules {
                let kind = if r.allow { "ALLOW" } else { "BLOCK" };
                let approval = if r.requires_approval {
                    " [approval]"
                } else {
                    ""
                };
                println!(
                    "    {} {:?} → {}{}  {}",
                    kind, r.direction, r.pattern, approval, r.description
                );
            }
        }
    }
}
