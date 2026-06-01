// Phase 10: Rich Terminal UI Dashboard
//
// Tabs:  [0] Overview  [1] Nodes  [2] Journal  [3] Intelligence  [4] Oracle
// Keys:  Tab/←→ switch tab  ↑↓ navigate  Enter select  / search
//        R refresh  W save  Q quit  Esc deselect/clear
//
// Color scheme mirrors the GPU renderer: deep navy bg, neon cyan accents.

use crate::analytics::{
    AnalyticsEngine, BridgeEdge, CentralityEntry, GraphHealthReport, PageRankEntry,
};
use crate::domain::{ArcType, EdgeType, FocusDepth, LoreEntry, NodeData, NodeType};
use crate::dream::{DreamEngine, DreamProposal, ProposalKind};
use crate::entropy::EntropyEngine;
use crate::gravity::GravityEngine;
use crate::identity::{GeometryKind, LivingSignature, ShadowProject};
use crate::intelligence::{FocusSuggestion, SuggestionEngine};
use crate::materialize::MaterializationEngine;
use crate::storage::{SqliteWorkspaceStore, WorkspaceStore};
use crate::systems::{
    Civilization, CognitiveSeason, DetectedContract, OracleSignal, ResonancePair, Ritual,
    SeasonReport,
};
use crate::workspace::SilentNodeWorkspace;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        canvas::Canvas, Block, BorderType, Borders, Clear, Gauge, List, ListItem, ListState,
        Paragraph, Tabs, Wrap,
    },
    Frame, Terminal,
};
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use uuid::Uuid;

// ── App mode ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum AppMode {
    Normal,
    Search,
    AddNode,
    AddJournal,
    LinkMode { first: Uuid },
}

// ── Color palette ─────────────────────────────────────────────────────────────

#[allow(dead_code)]
const C_BG: Color = Color::Reset;
const C_BORDER: Color = Color::Rgb(30, 70, 140);
const C_BORDER_H: Color = Color::Rgb(64, 180, 255); // highlighted border
const C_TITLE: Color = Color::Rgb(64, 200, 255); // cyan
const C_TEXT: Color = Color::Rgb(180, 210, 255); // pale blue-white
const C_DIM: Color = Color::Rgb(80, 110, 160); // muted
const C_SELECT: Color = Color::Rgb(100, 220, 255); // selected item
const C_GOOD: Color = Color::Rgb(60, 220, 120); // green
const C_WARN: Color = Color::Rgb(230, 180, 50); // amber
const C_BAD: Color = Color::Rgb(220, 70, 70); // red
const C_PURPLE: Color = Color::Rgb(140, 80, 255);
const C_GHOST: Color = Color::Rgb(80, 100, 140);
const C_FOSSIL: Color = Color::Rgb(130, 110, 60);
const C_VOID: Color = Color::Rgb(100, 30, 120);

fn node_color(n: &NodeData) -> Color {
    if n.is_void {
        return C_VOID;
    }
    if n.is_fossil {
        return C_FOSSIL;
    }
    if n.is_ghost {
        return C_GHOST;
    }
    match n.node_type {
        NodeType::Idea => Color::Rgb(64, 200, 255),
        NodeType::Memory => Color::Rgb(200, 100, 255),
        NodeType::Project => Color::Rgb(60, 220, 120),
        NodeType::Person => Color::Rgb(255, 210, 60),
        NodeType::Artifact => Color::Rgb(100, 160, 255),
        NodeType::Media => Color::Rgb(60, 180, 200),
        NodeType::Process => Color::Rgb(120, 255, 160),
        NodeType::World => Color::Rgb(255, 255, 255),
        NodeType::Ghost => C_GHOST,
        NodeType::Fossil => C_FOSSIL,
        NodeType::Other => Color::Rgb(148, 163, 184),
    }
}

fn node_icon(n: &NodeData) -> &'static str {
    if n.is_void {
        return "◈";
    }
    if n.is_fossil {
        return "◫";
    }
    if n.is_ghost {
        return "◌";
    }
    match n.node_type {
        NodeType::Idea => "◆",
        NodeType::Memory => "◉",
        NodeType::Project => "▣",
        NodeType::Person => "◎",
        NodeType::Artifact => "◧",
        NodeType::Media => "◐",
        NodeType::Process => "◑",
        NodeType::World => "◯",
        NodeType::Ghost => "◌",
        NodeType::Fossil => "◫",
        NodeType::Other => "◇",
    }
}

fn season_color(s: CognitiveSeason) -> Color {
    match s {
        CognitiveSeason::Spring => Color::Rgb(80, 220, 120),
        CognitiveSeason::Summer => Color::Rgb(255, 210, 60),
        CognitiveSeason::Autumn => Color::Rgb(230, 120, 50),
        CognitiveSeason::Winter => Color::Rgb(100, 160, 255),
    }
}

fn entropy_color(e: f32) -> Color {
    if e > 0.7 {
        C_BAD
    } else if e > 0.4 {
        C_WARN
    } else {
        C_GOOD
    }
}

fn filled_bar(ratio: f32, width: usize) -> String {
    let filled = ((ratio.clamp(0.0, 1.0) * width as f32) as usize).min(width);
    let empty = width - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

// ── App state ─────────────────────────────────────────────────────────────────

pub struct TuiApp {
    pub workspace: SilentNodeWorkspace,
    db_path: PathBuf,

    // Mode
    mode: AppMode,

    // computed (refreshed on R or every 60s)
    suggestions: Vec<FocusSuggestion>,
    resonances: Vec<ResonancePair>,
    civs: Vec<Civilization>,
    oracle: Vec<OracleSignal>,
    rituals: Vec<Ritual>,
    contracts: Vec<DetectedContract>,
    season: Option<SeasonReport>,

    // UI navigation
    tab: usize,
    node_state: ListState,
    #[allow(dead_code)]
    journal_state: ListState,
    suggest_state: ListState,
    selected_node: Option<Uuid>,
    #[allow(dead_code)]
    node_scroll: usize,
    journal_scroll: usize,

    // search / filter
    search: String,
    node_type_filter: Option<NodeType>,

    // input buffer (add node / add journal)
    input_buf: String,

    // notification (shown for 3s in title)
    notification: Option<(String, Instant)>,

    // help overlay
    show_help: bool,

    // timing
    last_compute: Instant,
    pub frame: u64,
    needs_save: bool,

    // Analytics
    pagerank: Vec<PageRankEntry>,
    centrality: Vec<CentralityEntry>,
    bridges: Vec<BridgeEdge>,
    health: Option<GraphHealthReport>,
    pagerank_state: ListState,

    // Dream
    proposals: Vec<DreamProposal>,
    proposal_state: ListState,

    // Identity
    shadow_projects: Vec<ShadowProject>,
    living_sig: Option<LivingSignature>,
    lore_arcs: Vec<LoreEntry>,
}

impl TuiApp {
    pub fn new(workspace: SilentNodeWorkspace, db_path: PathBuf) -> Self {
        let mut app = Self {
            workspace,
            db_path,
            mode: AppMode::Normal,
            suggestions: Vec::new(),
            resonances: Vec::new(),
            civs: Vec::new(),
            oracle: Vec::new(),
            rituals: Vec::new(),
            contracts: Vec::new(),
            season: None,
            tab: 0,
            node_state: ListState::default(),
            journal_state: ListState::default(),
            suggest_state: ListState::default(),
            selected_node: None,
            node_scroll: 0,
            journal_scroll: 0,
            search: String::new(),
            node_type_filter: None,
            input_buf: String::new(),
            notification: None,
            show_help: false,
            last_compute: Instant::now() - Duration::from_secs(120),
            frame: 0,
            needs_save: false,
            pagerank: Vec::new(),
            centrality: Vec::new(),
            bridges: Vec::new(),
            health: None,
            pagerank_state: ListState::default(),
            proposals: Vec::new(),
            proposal_state: ListState::default(),
            shadow_projects: Vec::new(),
            living_sig: None,
            lore_arcs: Vec::new(),
        };
        app.recompute();
        app
    }

    fn notify(&mut self, msg: &str) {
        self.notification = Some((msg.to_string(), Instant::now()));
    }

    fn cycle_type_filter(&mut self) {
        self.node_type_filter = match self.node_type_filter {
            None => Some(NodeType::Idea),
            Some(NodeType::Idea) => Some(NodeType::Memory),
            Some(NodeType::Memory) => Some(NodeType::Project),
            Some(NodeType::Project) => Some(NodeType::Person),
            Some(NodeType::Person) => Some(NodeType::Artifact),
            Some(NodeType::Artifact) => Some(NodeType::Media),
            Some(NodeType::Media) => Some(NodeType::Process),
            Some(NodeType::Process) => Some(NodeType::World),
            Some(NodeType::World) => None,
            Some(_) => None,
        };
    }

    // ── Node actions ────────────────────────────────────────────────────────

    fn action_focus_node(&mut self) {
        let Some(nid) = self.selected_node else {
            self.notify("No node selected");
            return;
        };
        match self.workspace.record_focus(nid, 60.0, FocusDepth::DeepWork) {
            Ok(_) => {
                self.notify("Focus recorded (60s DeepWork)");
                self.needs_save = true;
            }
            Err(e) => self.notify(&format!("Error: {e}")),
        }
    }

    fn action_void_toggle(&mut self) {
        let Some(nid) = self.selected_node else {
            self.notify("No node selected");
            return;
        };
        let is_void = self
            .workspace
            .graph
            .get_node(nid)
            .map(|n| n.is_void)
            .unwrap_or(false);
        if is_void {
            match self.workspace.extract_from_void(nid) {
                Ok(_) => {
                    self.notify("Extracted from void");
                    self.needs_save = true;
                }
                Err(e) => self.notify(&format!("Error: {e}")),
            }
        } else {
            match self.workspace.send_to_void(nid) {
                Ok(_) => {
                    self.notify("Sent to void");
                    self.needs_save = true;
                }
                Err(e) => self.notify(&format!("Error: {e}")),
            }
        }
    }

    fn action_revive(&mut self) {
        let Some(nid) = self.selected_node else {
            self.notify("No node selected");
            return;
        };
        let engine = EntropyEngine::new();
        self.workspace.reverse_entropy(&engine, nid);
        let _ = self.workspace.revive_node(nid);
        self.notify("Node revived + entropy reversed");
        self.needs_save = true;
    }

    fn action_fossilize(&mut self) {
        let Some(nid) = self.selected_node else {
            self.notify("No node selected");
            return;
        };
        match self.workspace.fossilize_node(nid) {
            Ok(_) => {
                self.notify("Node fossilized");
                self.needs_save = true;
            }
            Err(e) => self.notify(&format!("Error: {e}")),
        }
    }

    fn action_excavate(&mut self) {
        let Some(nid) = self.selected_node else {
            self.notify("No node selected");
            return;
        };
        match self.workspace.excavate_node(nid) {
            Ok(_) => {
                self.notify("Node excavated");
                self.needs_save = true;
            }
            Err(e) => self.notify(&format!("Error: {e}")),
        }
    }

    fn action_link_confirm(&mut self, second: Uuid) {
        if let AppMode::LinkMode { first } = self.mode {
            match self
                .workspace
                .connect_nodes(first, second, EdgeType::Connection, 1.0)
            {
                Ok(_) => {
                    self.notify("Nodes connected!");
                    self.selected_node = Some(second);
                    self.needs_save = true;
                }
                Err(e) => self.notify(&format!("Error: {e}")),
            }
        }
        self.mode = AppMode::Normal;
    }

    fn do_save_node(&mut self) {
        let content = self.input_buf.trim().to_string();
        if content.is_empty() {
            self.mode = AppMode::Normal;
            return;
        }
        let engine = MaterializationEngine::default();
        match self.workspace.materialize_thought(&engine, &content) {
            Ok(result) => {
                let preview: String = content.chars().take(24).collect();
                self.notify(&format!("Node added: {preview}"));
                self.selected_node = Some(result.node_id);
                self.tab = 1;
                self.needs_save = true;
            }
            Err(e) => self.notify(&format!("Error: {e}")),
        }
        self.input_buf.clear();
        self.mode = AppMode::Normal;
    }

    fn do_save_journal(&mut self) {
        let content = self.input_buf.trim().to_string();
        if content.is_empty() {
            self.mode = AppMode::Normal;
            return;
        }
        let season_str = self
            .season
            .as_ref()
            .map(|s| format!("{:?}", s.season).to_lowercase());
        self.workspace.add_journal_entry(&content, season_str);
        self.notify("Journal entry saved");
        self.needs_save = true;
        self.input_buf.clear();
        self.mode = AppMode::Normal;
    }

    pub fn recompute(&mut self) {
        self.suggestions = SuggestionEngine::new().suggest_next_focus(&self.workspace, 12);
        self.resonances = self.workspace.resonant_pairs();
        self.civs = self.workspace.detect_civilizations();
        self.oracle = self.workspace.oracle_signals();
        self.rituals = self.workspace.detect_rituals();
        self.contracts = self.workspace.detect_contracts();
        self.season = Some(self.workspace.cognitive_season());
        let analytics = AnalyticsEngine::new();
        self.pagerank = analytics.pagerank(&self.workspace, 15);
        self.centrality = analytics.betweenness(&self.workspace, 10);
        self.bridges = analytics.find_bridges(&self.workspace);
        self.health = Some(analytics.health_report(&self.workspace));
        self.proposals = DreamEngine::new().generate(&self.workspace);
        // Phase 10 — Identity
        self.shadow_projects = self.workspace.detect_shadow_projects();
        self.workspace.derive_living_signature();
        self.living_sig = self.workspace.identity.current_signature.clone();
        self.lore_arcs = self.workspace.detect_lore(&[]);
        self.last_compute = Instant::now();
    }

    pub fn save(&mut self) {
        if let Ok(mut store) = SqliteWorkspaceStore::new(&self.db_path) {
            let snap = self.workspace.snapshot();
            let _ = store.save_snapshot(&snap);
        }
        self.needs_save = false;
    }

    pub fn filtered_nodes(&self) -> Vec<&NodeData> {
        let q = self.search.to_lowercase();
        let mut nodes: Vec<&NodeData> = self
            .workspace
            .graph
            .nodes()
            .filter(|n| {
                let type_ok = self.node_type_filter.map_or(true, |t| n.node_type == t);
                let search_ok = q.is_empty() || n.content.to_lowercase().contains(&q);
                type_ok && search_ok
            })
            .collect();
        nodes.sort_by(|a, b| {
            b.gravity
                .partial_cmp(&a.gravity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        nodes
    }

    // ── Input handling ──────────────────────────────────────────────────────

    pub fn on_key(&mut self, code: KeyCode) -> bool {
        // ── Input modes: AddNode / AddJournal ──────────────────────────────
        match &self.mode {
            AppMode::AddNode | AppMode::AddJournal => {
                match code {
                    KeyCode::Esc => {
                        self.input_buf.clear();
                        self.mode = AppMode::Normal;
                    }
                    KeyCode::Backspace => {
                        self.input_buf.pop();
                    }
                    KeyCode::Enter => {
                        if matches!(self.mode, AppMode::AddNode) {
                            self.do_save_node();
                        } else {
                            self.do_save_journal();
                        }
                    }
                    KeyCode::Char(c) => {
                        self.input_buf.push(c);
                    }
                    _ => {}
                }
                return false;
            }

            // ── Search mode ────────────────────────────────────────────────
            AppMode::Search => {
                match code {
                    KeyCode::Esc => {
                        self.mode = AppMode::Normal;
                        self.search.clear();
                    }
                    KeyCode::Enter => {
                        self.mode = AppMode::Normal;
                    }
                    KeyCode::Backspace => {
                        self.search.pop();
                    }
                    KeyCode::Char(c) => {
                        self.search.push(c);
                    }
                    _ => {}
                }
                return false;
            }

            // ── Link mode: waiting for 2nd node ────────────────────────────
            AppMode::LinkMode { .. } => {
                match code {
                    KeyCode::Esc => {
                        self.mode = AppMode::Normal;
                        self.notify("Link cancelled");
                    }
                    KeyCode::Enter => {
                        // confirm with currently highlighted node
                        let target = self
                            .node_state
                            .selected()
                            .and_then(|i| self.filtered_nodes().get(i).map(|n| n.id));
                        if let Some(tid) = target {
                            self.action_link_confirm(tid);
                        }
                    }
                    KeyCode::Up => self.navigate(-1),
                    KeyCode::Down => self.navigate(1),
                    _ => {}
                }
                return false;
            }

            AppMode::Normal => {}
        }

        // ── Normal mode ────────────────────────────────────────────────────
        match code {
            // Quit
            KeyCode::Char('q') | KeyCode::Char('Q') => return true,

            // Help overlay toggle
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
            }

            // Close help or deselect
            KeyCode::Esc => {
                if self.show_help {
                    self.show_help = false;
                } else {
                    self.selected_node = None;
                    self.search.clear();
                }
            }

            // Tab navigation
            KeyCode::Tab | KeyCode::Right => {
                self.tab = (self.tab + 1) % 8;
            }
            KeyCode::Left | KeyCode::BackTab => {
                self.tab = if self.tab == 0 { 7 } else { self.tab - 1 };
            }
            KeyCode::Char('1') => self.tab = 0,
            KeyCode::Char('2') => self.tab = 1,
            KeyCode::Char('3') => self.tab = 2,
            KeyCode::Char('4') => self.tab = 3,
            KeyCode::Char('5') => self.tab = 4,
            KeyCode::Char('6') => self.tab = 5,
            KeyCode::Char('7') => self.tab = 6,
            KeyCode::Char('8') => self.tab = 7,

            // List navigation
            KeyCode::Up => self.navigate(-1),
            KeyCode::Down => self.navigate(1),
            KeyCode::PageUp => self.navigate(-10),
            KeyCode::PageDown => self.navigate(10),

            // Select
            KeyCode::Enter => self.select_current(),

            // Search
            KeyCode::Char('/') => {
                self.mode = AppMode::Search;
                self.search.clear();
            }

            // Global actions
            KeyCode::Char('r') | KeyCode::Char('R') => self.recompute(),
            KeyCode::Char('w') | KeyCode::Char('W') => self.save(),
            KeyCode::Char('p') | KeyCode::Char('P') => {
                self.workspace.step_gravity(&GravityEngine::new(), 1.0);
                self.workspace.tick_entropy(&EntropyEngine::new());
                self.needs_save = true;
                self.notify("Physics step + entropy tick done");
            }

            // Add node (any tab)
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.input_buf.clear();
                self.mode = AppMode::AddNode;
            }

            // Add journal entry (any tab)
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.input_buf.clear();
                self.mode = AppMode::AddJournal;
            }

            // Type filter (Nodes tab only)
            KeyCode::Char('t') | KeyCode::Char('T') if self.tab == 1 => {
                self.cycle_type_filter();
            }

            // Node actions (require selected_node)
            KeyCode::Char('f') | KeyCode::Char('F') => self.action_focus_node(),
            KeyCode::Char('v') | KeyCode::Char('V') => self.action_void_toggle(),
            KeyCode::Char('g') | KeyCode::Char('G') => self.action_revive(),
            KeyCode::Char('z') | KeyCode::Char('Z') => self.action_fossilize(),
            KeyCode::Char('x') | KeyCode::Char('X') => self.action_excavate(),
            KeyCode::Char('l') | KeyCode::Char('L') => {
                if let Some(nid) = self.selected_node {
                    self.mode = AppMode::LinkMode { first: nid };
                    self.tab = 1; // switch to nodes tab for picking
                    self.notify(
                        "Link mode: select target node with ↑↓, Enter to connect, Esc to cancel",
                    );
                } else {
                    self.notify("Select a node first (Enter on Nodes tab)");
                }
            }

            _ => {}
        }
        false
    }

    fn navigate(&mut self, delta: i32) {
        match self.tab {
            1 => {
                let len = self.filtered_nodes().len();
                if len == 0 {
                    return;
                }
                let cur = self.node_state.selected().unwrap_or(0) as i32;
                let next = (cur + delta).rem_euclid(len as i32) as usize;
                self.node_state.select(Some(next));
            }
            2 => {
                let len = self.workspace.journal.entries().len();
                if len == 0 {
                    return;
                }
                let cur = self.journal_scroll as i32;
                self.journal_scroll =
                    (cur + delta).clamp(0, (len as i32).saturating_sub(1)) as usize;
            }
            3 => {
                let len = self.suggestions.len();
                if len == 0 {
                    return;
                }
                let cur = self.suggest_state.selected().unwrap_or(0) as i32;
                let next = (cur + delta).rem_euclid(len as i32) as usize;
                self.suggest_state.select(Some(next));
            }
            5 => {
                let len = self.pagerank.len();
                if len == 0 {
                    return;
                }
                let cur = self.pagerank_state.selected().unwrap_or(0) as i32;
                let next = (cur + delta).rem_euclid(len as i32) as usize;
                self.pagerank_state.select(Some(next));
            }
            6 => {
                let len = self.proposals.len();
                if len == 0 {
                    return;
                }
                let cur = self.proposal_state.selected().unwrap_or(0) as i32;
                let next = (cur + delta).rem_euclid(len as i32) as usize;
                self.proposal_state.select(Some(next));
            }
            _ => {}
        }
    }

    fn select_current(&mut self) {
        if self.tab == 1 {
            let nodes = self.filtered_nodes();
            if let Some(i) = self.node_state.selected() {
                if let Some(n) = nodes.get(i) {
                    self.selected_node = Some(n.id);
                }
            }
        }
    }

    // ── Render ──────────────────────────────────────────────────────────────

    pub fn render(&mut self, f: &mut Frame) {
        let area = f.area();

        // outer layout: title + tabs + content + status (2 lines)
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // title
                Constraint::Length(3), // tabs
                Constraint::Min(0),    // content
                Constraint::Length(2), // status (2 lines)
            ])
            .split(area);

        self.render_title(f, outer[0]);
        self.render_tabs(f, outer[1]);

        // Content + optional input overlay at bottom
        let content_area = outer[2];
        match self.tab {
            0 => self.render_overview(f, content_area),
            1 => self.render_nodes(f, content_area),
            2 => self.render_journal(f, content_area),
            3 => self.render_intelligence(f, content_area),
            4 => self.render_oracle(f, content_area),
            5 => self.render_analytics(f, content_area),
            6 => self.render_dream(f, content_area),
            7 => self.render_identity(f, content_area),
            _ => {}
        }

        // Input overlay (AddNode / AddJournal)
        if matches!(self.mode, AppMode::AddNode | AppMode::AddJournal) {
            self.render_input_overlay(f, content_area);
        }

        self.render_status(f, outer[3]);

        // Help overlay (floating, on top of everything)
        if self.show_help {
            self.render_help_overlay(f, area);
        }
    }

    fn render_input_overlay(&self, f: &mut Frame, area: Rect) {
        let label = if matches!(self.mode, AppMode::AddNode) {
            "ADD NODE"
        } else {
            "ADD JOURNAL ENTRY"
        };
        let h = 4u16;
        let overlay = Rect {
            x: area.x + 2,
            y: area.y + area.height.saturating_sub(h + 1),
            width: area.width.saturating_sub(4),
            height: h,
        };
        f.render_widget(Clear, overlay);

        let cursor_line = format!(" > {}█", self.input_buf);
        let hint_line = "   [Enter] save  [Esc] cancel  [Backspace] delete";
        let lines = vec![
            Line::from(Span::styled(
                cursor_line,
                Style::default().fg(C_SELECT).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(hint_line, Style::default().fg(C_DIM))),
        ];
        let block = Block::default()
            .title(Span::styled(
                format!(" {} ", label),
                Style::default().fg(C_TITLE).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(C_BORDER_H));
        f.render_widget(Paragraph::new(lines).block(block), overlay);
    }

    fn render_help_overlay(&self, f: &mut Frame, area: Rect) {
        let w = 58u16.min(area.width.saturating_sub(4));
        let h = 22u16.min(area.height.saturating_sub(2));
        let x = area.x + (area.width.saturating_sub(w)) / 2;
        let y = area.y + (area.height.saturating_sub(h)) / 2;
        let overlay = Rect {
            x,
            y,
            width: w,
            height: h,
        };
        f.render_widget(Clear, overlay);

        let lines = vec![
            Line::from(Span::styled(
                "  NAVIGATION                NODE ACTIONS (node selected)",
                Style::default().fg(C_DIM),
            )),
            Line::from(vec![
                Span::styled("  Tab/←→   ", Style::default().fg(C_SELECT)),
                Span::styled("Switch tabs    ", Style::default().fg(C_TEXT)),
                Span::styled("F  ", Style::default().fg(C_GOOD)),
                Span::styled("Record 60s focus event", Style::default().fg(C_TEXT)),
            ]),
            Line::from(vec![
                Span::styled("  ↑↓        ", Style::default().fg(C_SELECT)),
                Span::styled("Navigate       ", Style::default().fg(C_TEXT)),
                Span::styled("V  ", Style::default().fg(C_VOID)),
                Span::styled("Void / Un-void toggle", Style::default().fg(C_TEXT)),
            ]),
            Line::from(vec![
                Span::styled("  PgUp/PgDn ", Style::default().fg(C_SELECT)),
                Span::styled("Fast navigate  ", Style::default().fg(C_TEXT)),
                Span::styled("G  ", Style::default().fg(C_GHOST)),
                Span::styled("Revive ghost / entropy", Style::default().fg(C_TEXT)),
            ]),
            Line::from(vec![
                Span::styled("  Enter     ", Style::default().fg(C_SELECT)),
                Span::styled("Select item    ", Style::default().fg(C_TEXT)),
                Span::styled("L  ", Style::default().fg(C_BORDER_H)),
                Span::styled("Link mode (connect 2)", Style::default().fg(C_TEXT)),
            ]),
            Line::from(vec![
                Span::styled("  Esc       ", Style::default().fg(C_SELECT)),
                Span::styled("Cancel/deselect", Style::default().fg(C_TEXT)),
                Span::styled("Z  ", Style::default().fg(C_FOSSIL)),
                Span::styled("Fossilize node", Style::default().fg(C_TEXT)),
            ]),
            Line::from(vec![
                Span::styled("  /         ", Style::default().fg(C_SELECT)),
                Span::styled("Search nodes   ", Style::default().fg(C_TEXT)),
                Span::styled("X  ", Style::default().fg(C_FOSSIL)),
                Span::styled("Excavate (un-fossilize)", Style::default().fg(C_TEXT)),
            ]),
            Line::from(vec![
                Span::styled("  ?         ", Style::default().fg(C_SELECT)),
                Span::styled("Close this help", Style::default().fg(C_TEXT)),
            ]),
            Line::from(Span::styled("", Style::default())),
            Line::from(Span::styled(
                "  ADDING CONTENT            TABS",
                Style::default().fg(C_DIM),
            )),
            Line::from(vec![
                Span::styled("  N  ", Style::default().fg(C_GOOD)),
                Span::styled("Add new node       ", Style::default().fg(C_TEXT)),
                Span::styled("1 Overview  5 Oracle", Style::default().fg(C_DIM)),
            ]),
            Line::from(vec![
                Span::styled("  A  ", Style::default().fg(C_GOOD)),
                Span::styled("Add journal entry  ", Style::default().fg(C_TEXT)),
                Span::styled("2 Nodes     6 Analytics", Style::default().fg(C_DIM)),
            ]),
            Line::from(vec![
                Span::styled("     ", Style::default()),
                Span::styled("(works on any tab) ", Style::default().fg(C_DIM)),
                Span::styled("3 Journal   7 Dream", Style::default().fg(C_DIM)),
            ]),
            Line::from(Span::styled("", Style::default())),
            Line::from(Span::styled(
                "  GLOBAL KEYS               4 Intelligence  8 Identity",
                Style::default().fg(C_DIM),
            )),
            Line::from(vec![
                Span::styled("  T  ", Style::default().fg(C_WARN)),
                Span::styled("Type filter (Nodes tab)", Style::default().fg(C_TEXT)),
            ]),
            Line::from(vec![
                Span::styled("  R  ", Style::default().fg(C_SELECT)),
                Span::styled("Refresh computed data  ", Style::default().fg(C_TEXT)),
                Span::styled("P  ", Style::default().fg(C_SELECT)),
                Span::styled("Physics step", Style::default().fg(C_TEXT)),
            ]),
            Line::from(vec![
                Span::styled("  W  ", Style::default().fg(C_SELECT)),
                Span::styled("Save to database       ", Style::default().fg(C_TEXT)),
                Span::styled("Q  ", Style::default().fg(C_BAD)),
                Span::styled("Quit", Style::default().fg(C_TEXT)),
            ]),
            Line::from(Span::styled("", Style::default())),
            Line::from(Span::styled(
                "  Press ? to close                                    ",
                Style::default().fg(C_DIM),
            )),
        ];

        let block = Block::default()
            .title(Span::styled(
                " ◈ SILENTNODE KEY REFERENCE ",
                Style::default().fg(C_TITLE).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(C_BORDER_H));
        f.render_widget(Paragraph::new(lines).block(block), overlay);
    }

    fn render_title(&self, f: &mut Frame, area: Rect) {
        let pulse = (self.frame / 30) % 2 == 0;
        let title_col = if pulse {
            C_TITLE
        } else {
            Color::Rgb(40, 140, 200)
        };
        let save_ind = if self.needs_save { " ●" } else { "" };
        let stats = self.workspace.graph.stats();
        let time_str = chrono::Local::now().format("%H:%M:%S").to_string();

        let season_str = self
            .season
            .as_ref()
            .map(|s| format!("{:?}", s.season))
            .unwrap_or_default();

        let mode_str = match &self.mode {
            AppMode::Normal => "",
            AppMode::Search => " [SEARCH]",
            AppMode::AddNode => " [ADD NODE]",
            AppMode::AddJournal => " [ADD JOURNAL]",
            AppMode::LinkMode { .. } => " [LINK MODE]",
        };

        // notification overrides mode display if fresh
        let extra = if let Some((msg, t)) = &self.notification {
            if t.elapsed().as_secs() < 3 {
                format!("  ✓ {}", msg)
            } else {
                mode_str.to_string()
            }
        } else {
            mode_str.to_string()
        };

        let line = Line::from(vec![
            Span::styled(
                " ◈ SILENT NODE",
                Style::default().fg(title_col).add_modifier(Modifier::BOLD),
            ),
            Span::styled(save_ind, Style::default().fg(C_BAD)),
            Span::styled(
                format!(
                    "  n:{} e:{}  {}  {}",
                    stats.node_count, stats.edge_count, season_str, time_str
                ),
                Style::default().fg(C_DIM),
            ),
            Span::styled(
                extra,
                Style::default().fg(C_GOOD).add_modifier(Modifier::BOLD),
            ),
        ]);
        f.render_widget(Paragraph::new(line), area);
    }

    fn render_tabs(&self, f: &mut Frame, area: Rect) {
        let titles = vec![
            Line::from(Span::styled("1 Overview", Style::default().fg(C_TEXT))),
            Line::from(Span::styled("2 Nodes", Style::default().fg(C_TEXT))),
            Line::from(Span::styled("3 Journal", Style::default().fg(C_TEXT))),
            Line::from(Span::styled("4 Intelligence", Style::default().fg(C_TEXT))),
            Line::from(Span::styled("5 Oracle", Style::default().fg(C_TEXT))),
            Line::from(Span::styled("6 Analytics", Style::default().fg(C_TEXT))),
            Line::from(Span::styled("7 Dream", Style::default().fg(C_TEXT))),
            Line::from(Span::styled("8 Identity", Style::default().fg(C_TEXT))),
        ];
        let tabs = Tabs::new(titles)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(C_BORDER))
                    .title(Span::styled(
                        " SilentNode v0.1 ",
                        Style::default().fg(C_DIM),
                    )),
            )
            .select(self.tab)
            .highlight_style(
                Style::default()
                    .fg(C_SELECT)
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::UNDERLINED),
            )
            .divider(Span::styled(" │ ", Style::default().fg(C_BORDER)));
        f.render_widget(tabs, area);
    }

    fn render_status(&self, f: &mut Frame, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(area);

        // Line 1: tab-specific hints
        let tab_hint = match self.tab {
            0 => "  Overview — [N]ew-node  [A]journal  [P]physics  [R]refresh  [W]save",
            1 => {
                let filter = self
                    .node_type_filter
                    .map(|t| format!(" [filter:{:?}]", t))
                    .unwrap_or_default();
                &format!("  Nodes{filter} — [N]new  [F]focus  [V]void  [G]revive  [L]link  [Z]fossil  [X]excavate  [T]type-filter  [/]search")[..]
            }
            2 => "  Journal — [A]add-entry  [/]search  [↑↓]scroll",
            3 => "  Intelligence — [R]refresh",
            4 => "  Oracle — [R]refresh",
            5 => "  Analytics — [R]refresh",
            6 => "  Dream — [R]refresh",
            7 => "  Identity — [R]refresh  [N]new-node",
            _ => "",
        };
        let filter_str = if self.tab == 1 {
            self.node_type_filter
                .map(|t| format!(" [filter:{:?}]", t))
                .unwrap_or_default()
        } else {
            String::new()
        };
        let tab_hint_owned = if self.tab == 1 {
            format!("  Nodes{} — [N]new  [F]focus  [V]void  [G]revive  [L]link  [Z]fossil  [X]excavate  [T]type-filter  [/]search", filter_str)
        } else {
            tab_hint.to_string()
        };

        let search_prefix = if matches!(self.mode, AppMode::Search) {
            format!(" SEARCH: {}█  ", self.search)
        } else if !self.search.is_empty() {
            format!(" filter:\"{}\"  ", self.search)
        } else {
            String::new()
        };

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    search_prefix,
                    Style::default().fg(C_SELECT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(tab_hint_owned, Style::default().fg(C_DIM)),
            ])),
            rows[0],
        );

        // Line 2: universal keys
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  [Tab/←→] tabs  [↑↓/PgUp/PgDn] nav  [Enter] select  [1-8] jump  [?] help  [Q] quit",
                Style::default().fg(Color::Rgb(60, 80, 120)),
            ))),
            rows[1],
        );
    }

    // ── Tab 0: Overview ──────────────────────────────────────────────────────

    fn render_overview(&self, f: &mut Frame, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),  // stats + season
                Constraint::Min(8),     // gravity bars
                Constraint::Length(10), // focus trail + canvas
            ])
            .split(area);

        // row 0: stats | season+weather
        let top = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(rows[0]);
        self.render_stats_block(f, top[0]);
        self.render_season_block(f, top[1]);

        // row 1: gravity bar chart
        self.render_gravity_bars(f, rows[1]);

        // row 2: focus trail | mini graph canvas
        let bot = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[2]);
        self.render_focus_trail(f, bot[0]);
        self.render_mini_canvas(f, bot[1]);
    }

    fn render_stats_block(&self, f: &mut Frame, area: Rect) {
        let stats = self.workspace.graph.stats();
        let block = styled_block("Graph Stats", false);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let lines = vec![
            stat_line("Nodes", stats.node_count, C_TITLE),
            stat_line("Edges", stats.edge_count, C_TEXT),
            stat_line("Ghosts", stats.ghost_count, C_GHOST),
            stat_line("Fossils", stats.fossil_count, C_FOSSIL),
            stat_line("Void", stats.void_count, C_VOID),
            Line::from(vec![
                Span::styled("Focus events  ", Style::default().fg(C_DIM)),
                Span::styled(
                    self.workspace.focus.events().len().to_string(),
                    Style::default().fg(C_TEXT).add_modifier(Modifier::BOLD),
                ),
            ]),
        ];
        f.render_widget(Paragraph::new(lines), inner);
    }

    fn render_season_block(&self, f: &mut Frame, area: Rect) {
        let block = styled_block("Cognitive Season", false);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let lines = match &self.season {
            None => vec![Line::from(Span::styled(
                "computing…",
                Style::default().fg(C_DIM),
            ))],
            Some(s) => {
                let sc = season_color(s.season);
                vec![
                    Line::from(vec![
                        Span::styled("Season       ", Style::default().fg(C_DIM)),
                        Span::styled(
                            format!("{:?}", s.season),
                            Style::default().fg(sc).add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    gauge_line("Creation", s.creation_rate),
                    gauge_line("Focus", s.focus_density),
                    gauge_line("Explore", s.exploration_ratio),
                    gauge_line("Revisit", s.revisit_ratio),
                    gauge_line("Entropy", s.avg_entropy),
                ]
            }
        };
        f.render_widget(Paragraph::new(lines), inner);
    }

    fn render_gravity_bars(&self, f: &mut Frame, area: Rect) {
        let block = styled_block("Top Nodes — Gravity", false);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let mut nodes: Vec<&NodeData> = self.workspace.graph.nodes().collect();
        nodes.sort_by(|a, b| {
            b.gravity
                .partial_cmp(&a.gravity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let nodes: Vec<&NodeData> = nodes.into_iter().take(10).collect();

        let max_g = nodes.first().map(|n| n.gravity).unwrap_or(1.0).max(1.0);
        let bar_width = (inner.width as usize).saturating_sub(22).max(10);

        let lines: Vec<Line> = nodes
            .iter()
            .map(|n| {
                let label = if n.content.len() > 18 {
                    format!("{:.17}…", &n.content[..17])
                } else {
                    format!("{:<18}", n.content)
                };
                let ratio = n.gravity / max_g;
                let bar = filled_bar(ratio, bar_width);
                Line::from(vec![
                    Span::styled(
                        format!("{} ", node_icon(n)),
                        Style::default().fg(node_color(n)),
                    ),
                    Span::styled(label, Style::default().fg(C_TEXT)),
                    Span::styled(bar, Style::default().fg(Color::Rgb(40, 120, 220))),
                    Span::styled(format!(" {:.2}", n.gravity), Style::default().fg(C_DIM)),
                ])
            })
            .collect();

        f.render_widget(Paragraph::new(lines), inner);
    }

    fn render_focus_trail(&self, f: &mut Frame, area: Rect) {
        let block = styled_block("Recent Focus Trail", false);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let events = self.workspace.focus.events();
        if events.is_empty() {
            f.render_widget(
                Paragraph::new(Span::styled(
                    "no focus events yet",
                    Style::default().fg(C_DIM),
                )),
                inner,
            );
            return;
        }

        let lines: Vec<Line> = events
            .iter()
            .rev()
            .take(inner.height as usize)
            .map(|ev| {
                let label = self
                    .workspace
                    .graph
                    .get_node(ev.node_id)
                    .map(|n| n.content.as_str())
                    .unwrap_or("?");
                let label = if label.len() > 20 {
                    &label[..20]
                } else {
                    label
                };
                let ago = chrono::Utc::now().signed_duration_since(ev.timestamp);
                let ago_str = if ago.num_hours() > 0 {
                    format!("{:>3}h", ago.num_hours())
                } else {
                    format!("{:>3}m", ago.num_minutes())
                };
                Line::from(vec![
                    Span::styled(ago_str, Style::default().fg(C_DIM)),
                    Span::styled("  ", Style::default()),
                    Span::styled(format!("{:<20}", label), Style::default().fg(C_TEXT)),
                    Span::styled(
                        format!("{:>5}s ", ev.duration_seconds as u32),
                        Style::default().fg(C_DIM),
                    ),
                    Span::styled(format!("{:?}", ev.depth), Style::default().fg(C_GOOD)),
                ])
            })
            .collect();
        f.render_widget(Paragraph::new(lines), inner);
    }

    fn render_mini_canvas(&self, f: &mut Frame, area: Rect) {
        let nodes: Vec<(f32, f32, Color)> = self
            .workspace
            .graph
            .nodes()
            .map(|n| (n.position.x, n.position.z, node_color(n)))
            .collect();

        let edges: Vec<(f32, f32, f32, f32)> = self
            .workspace
            .graph
            .edges()
            .filter_map(|e| {
                let src = self.workspace.graph.get_node(e.source_id)?;
                let dst = self.workspace.graph.get_node(e.target_id)?;
                Some((
                    src.position.x,
                    src.position.z,
                    dst.position.x,
                    dst.position.z,
                ))
            })
            .collect();

        // compute bounds
        let (xmin, xmax, ymin, ymax) = bounds_of(&nodes);

        let canvas = Canvas::default()
            .block(styled_block("Graph View", false))
            .x_bounds([xmin as f64 - 2.0, xmax as f64 + 2.0])
            .y_bounds([ymin as f64 - 2.0, ymax as f64 + 2.0])
            .paint(move |ctx| {
                // draw edges first
                for (x1, y1, x2, y2) in &edges {
                    ctx.draw(&ratatui::widgets::canvas::Line {
                        x1: *x1 as f64,
                        y1: *y1 as f64,
                        x2: *x2 as f64,
                        y2: *y2 as f64,
                        color: Color::Rgb(30, 60, 120),
                    });
                }
                // draw nodes
                for (x, y, color) in &nodes {
                    ctx.print(
                        *x as f64,
                        *y as f64,
                        ratatui::text::Span::styled("●", Style::default().fg(*color)),
                    );
                }
            });
        f.render_widget(canvas, area);
    }

    // ── Tab 1: Nodes ─────────────────────────────────────────────────────────

    fn render_nodes(&mut self, f: &mut Frame, area: Rect) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(36), Constraint::Percentage(64)])
            .split(area);

        self.render_node_list(f, cols[0]);
        self.render_node_detail(f, cols[1]);
    }

    fn render_node_list(&mut self, f: &mut Frame, area: Rect) {
        let nodes = self.filtered_nodes();
        let link_first = if let AppMode::LinkMode { first } = self.mode {
            Some(first)
        } else {
            None
        };
        let filter_label = self
            .node_type_filter
            .map(|t| format!(":{:?}", t))
            .unwrap_or_default();
        let title = if let Some(fid) = link_first {
            let fname = self
                .workspace
                .graph
                .get_node(fid)
                .map(|n| n.content.chars().take(16).collect::<String>())
                .unwrap_or_default();
            format!("LINK FROM «{fname}» → select target (Enter=connect, Esc=cancel)")
        } else if self.search.is_empty() {
            format!("Nodes{filter_label} ({})", nodes.len())
        } else {
            format!(
                "Nodes{filter_label} — \"{}\" ({})",
                self.search,
                nodes.len()
            )
        };

        let items: Vec<ListItem> = nodes
            .iter()
            .map(|n| {
                let is_selected = self.selected_node == Some(n.id);
                let label = if n.content.len() > 26 {
                    format!("{:.25}…", &n.content[..25])
                } else {
                    n.content.clone()
                };
                let style = if is_selected {
                    Style::default().fg(C_SELECT).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(node_color(n))
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{} ", node_icon(n)), style),
                    Span::styled(label, style),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(styled_block(&title, self.tab == 1))
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(20, 50, 100))
                    .fg(C_SELECT)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        let mut state = self.node_state.clone();
        if state.selected().is_none() && !nodes.is_empty() {
            state.select(Some(0));
        }
        f.render_stateful_widget(list, area, &mut state);
        self.node_state = state;
    }

    fn render_node_detail(&self, f: &mut Frame, area: Rect) {
        let node = self
            .selected_node
            .and_then(|id| self.workspace.graph.get_node(id))
            .or_else(|| {
                self.node_state
                    .selected()
                    .and_then(|i| self.filtered_nodes().get(i).copied())
            });

        let block = styled_block("Node Detail", false);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let Some(n) = node else {
            f.render_widget(
                Paragraph::new(Span::styled(
                    "  ↑ select a node",
                    Style::default().fg(C_DIM),
                )),
                inner,
            );
            return;
        };

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(6), // basic info
                Constraint::Length(4), // entropy + gravity gauges
                Constraint::Length(4), // connections
                Constraint::Min(0),    // metadata / position
            ])
            .split(inner);

        // basic info
        let flags: Vec<&str> = [
            if n.is_ghost { Some("GHOST") } else { None },
            if n.is_fossil { Some("FOSSIL") } else { None },
            if n.is_void { Some("VOID") } else { None },
        ]
        .into_iter()
        .flatten()
        .collect();

        let info_lines = vec![
            Line::from(vec![
                Span::styled(
                    format!("{} ", node_icon(n)),
                    Style::default()
                        .fg(node_color(n))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    &n.content,
                    Style::default().fg(C_TEXT).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("id       ", Style::default().fg(C_DIM)),
                Span::styled(n.id.to_string(), Style::default().fg(C_DIM)),
            ]),
            Line::from(vec![
                Span::styled("type     ", Style::default().fg(C_DIM)),
                Span::styled(
                    format!("{:?}", n.node_type),
                    Style::default().fg(node_color(n)),
                ),
                if !flags.is_empty() {
                    Span::styled(
                        format!("  [{}]", flags.join(", ")),
                        Style::default().fg(C_WARN).add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::raw("")
                },
            ]),
            Line::from(vec![
                Span::styled("accesses ", Style::default().fg(C_DIM)),
                Span::styled(n.access_count.to_string(), Style::default().fg(C_TEXT)),
                Span::styled("   created ", Style::default().fg(C_DIM)),
                Span::styled(
                    n.created_at.format("%Y-%m-%d").to_string(),
                    Style::default().fg(C_DIM),
                ),
            ]),
            Line::from(vec![
                Span::styled("pos      ", Style::default().fg(C_DIM)),
                Span::styled(
                    format!(
                        "({:.1}, {:.1}, {:.1})",
                        n.position.x, n.position.y, n.position.z
                    ),
                    Style::default().fg(C_DIM),
                ),
            ]),
        ];
        f.render_widget(Paragraph::new(info_lines), rows[0]);

        // gauges
        let gauge_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Length(2)])
            .split(rows[1]);

        let entropy_pct = (n.entropy.clamp(0.0, 1.0) * 100.0) as u16;
        let gravity_pct = ((n.gravity / 5.0_f32.max(n.gravity)).clamp(0.0, 1.0) * 100.0) as u16;

        f.render_widget(
            Gauge::default()
                .block(Block::default().title(Span::styled("Entropy", Style::default().fg(C_DIM))))
                .gauge_style(
                    Style::default()
                        .fg(entropy_color(n.entropy))
                        .bg(Color::Rgb(15, 25, 50)),
                )
                .percent(entropy_pct)
                .label(Span::styled(
                    format!("{:.2}", n.entropy),
                    Style::default().fg(C_TEXT),
                )),
            gauge_rows[0],
        );
        f.render_widget(
            Gauge::default()
                .block(Block::default().title(Span::styled("Gravity", Style::default().fg(C_DIM))))
                .gauge_style(
                    Style::default()
                        .fg(Color::Rgb(60, 160, 255))
                        .bg(Color::Rgb(15, 25, 50)),
                )
                .percent(gravity_pct)
                .label(Span::styled(
                    format!("{:.2}", n.gravity),
                    Style::default().fg(C_TEXT),
                )),
            gauge_rows[1],
        );

        // connections
        let degree = self.workspace.graph.degree(n.id);
        let out_edges: Vec<_> = self
            .workspace
            .graph
            .outgoing_edges(n.id)
            .unwrap_or_default();
        let conn_lines: Vec<Line> = std::iter::once(Line::from(vec![
            Span::styled("connections  ", Style::default().fg(C_DIM)),
            Span::styled(degree.to_string(), Style::default().fg(C_TITLE)),
        ]))
        .chain(out_edges.iter().take(3).map(|e| {
            let dst = self
                .workspace
                .graph
                .get_node(e.target_id)
                .map(|nd| nd.content.as_str())
                .unwrap_or("?");
            let dst = if dst.len() > 22 { &dst[..22] } else { dst };
            Line::from(vec![
                Span::styled("  → ", Style::default().fg(C_BORDER_H)),
                Span::styled(format!("{:<22}", dst), Style::default().fg(C_TEXT)),
                Span::styled(
                    format!(" {:?} w={:.2}", e.edge_type, e.weight),
                    Style::default().fg(C_DIM),
                ),
            ])
        }))
        .collect();
        f.render_widget(Paragraph::new(conn_lines), rows[2]);

        // Action hints row
        let void_hint = if n.is_void { "V=un-void" } else { "V=void" };
        let revive_col = if n.is_ghost || n.entropy > 0.5 {
            C_WARN
        } else {
            C_DIM
        };
        let fossil_hint = if n.is_fossil {
            "X=excavate"
        } else {
            "Z=fossil"
        };
        let actions = Line::from(vec![
            Span::styled(" ACTIONS: ", Style::default().fg(C_DIM)),
            Span::styled("F=focus  ", Style::default().fg(C_GOOD)),
            Span::styled(format!("{void_hint}  "), Style::default().fg(C_VOID)),
            Span::styled("G=revive  ", Style::default().fg(revive_col)),
            Span::styled("L=link  ", Style::default().fg(C_BORDER_H)),
            Span::styled(fossil_hint, Style::default().fg(C_FOSSIL)),
        ]);
        f.render_widget(Paragraph::new(vec![Line::from(""), actions]), rows[3]);
    }

    // ── Tab 2: Journal ───────────────────────────────────────────────────────

    fn render_journal(&self, f: &mut Frame, area: Rect) {
        let entries = self.workspace.journal.entries();

        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
            .split(area);

        // Left: entry list
        let list_title = format!("Journal ({})  [A]=add", entries.len());
        let list_block = styled_block(&list_title, true);
        let list_inner = list_block.inner(cols[0]);
        f.render_widget(list_block, cols[0]);

        if entries.is_empty() {
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(Span::styled(
                        "  No journal entries.",
                        Style::default().fg(C_DIM),
                    )),
                    Line::from(Span::styled(
                        "  Press A to add one.",
                        Style::default().fg(C_DIM),
                    )),
                ]),
                list_inner,
            );
        } else {
            let items: Vec<Line> = entries
                .iter()
                .rev()
                .skip(self.journal_scroll)
                .take(list_inner.height as usize)
                .map(|e| {
                    let sc = e
                        .season
                        .as_deref()
                        .map(|s| match s {
                            "spring" => C_GOOD,
                            "summer" => C_WARN,
                            "autumn" => Color::Rgb(230, 120, 50),
                            "winter" => Color::Rgb(100, 160, 255),
                            _ => C_DIM,
                        })
                        .unwrap_or(C_DIM);
                    let date = e.timestamp.format("%m-%d %H:%M").to_string();
                    let preview: String = e.content.chars().take(22).collect();
                    Line::from(vec![
                        Span::styled(format!("{date} "), Style::default().fg(C_DIM)),
                        Span::styled("● ", Style::default().fg(sc)),
                        Span::styled(preview, Style::default().fg(C_TEXT)),
                    ])
                })
                .collect();
            f.render_widget(Paragraph::new(items), list_inner);
        }

        // Right: selected entry detail (most recent visible = scroll offset)
        let detail_block = styled_block("Entry Detail  [↑↓] scroll", false);
        let detail_inner = detail_block.inner(cols[1]);
        f.render_widget(detail_block, cols[1]);

        let visible: Vec<_> = entries.iter().rev().skip(self.journal_scroll).collect();
        if let Some(entry) = visible.first() {
            let sc = entry
                .season
                .as_deref()
                .map(|s| match s {
                    "spring" => C_GOOD,
                    "summer" => C_WARN,
                    "autumn" => Color::Rgb(230, 120, 50),
                    "winter" => Color::Rgb(100, 160, 255),
                    _ => C_DIM,
                })
                .unwrap_or(C_DIM);

            let mut lines: Vec<Line> = vec![
                Line::from(vec![
                    Span::styled(
                        entry.timestamp.format("%Y-%m-%d %H:%M  ").to_string(),
                        Style::default().fg(C_DIM),
                    ),
                    Span::styled(
                        format!("[{}]", entry.season.as_deref().unwrap_or("—")),
                        Style::default().fg(sc).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(Span::styled(
                    "─".repeat(detail_inner.width as usize),
                    Style::default().fg(C_BORDER),
                )),
            ];
            let width = detail_inner.width.saturating_sub(2) as usize;
            for chunk in entry
                .content
                .chars()
                .collect::<Vec<_>>()
                .chunks(width.max(1))
            {
                lines.push(Line::from(Span::styled(
                    chunk.iter().collect::<String>(),
                    Style::default().fg(C_TEXT),
                )));
            }
            if !entry.linked_nodes.is_empty() {
                lines.push(Line::from(Span::styled("", Style::default())));
                let names: Vec<String> = entry
                    .linked_nodes
                    .iter()
                    .filter_map(|id| self.workspace.graph.get_node(*id))
                    .map(|n| n.content.chars().take(18).collect::<String>())
                    .collect();
                lines.push(Line::from(vec![
                    Span::styled("⟳ ", Style::default().fg(C_BORDER_H)),
                    Span::styled(names.join(" • "), Style::default().fg(C_DIM)),
                ]));
            }
            f.render_widget(
                Paragraph::new(lines).wrap(Wrap { trim: false }),
                detail_inner,
            );
        } else {
            f.render_widget(
                Paragraph::new(Span::styled(
                    "  ↑↓ to scroll entries",
                    Style::default().fg(C_DIM),
                )),
                detail_inner,
            );
        }
    }

    // ── Tab 3: Intelligence ──────────────────────────────────────────────────

    fn render_intelligence(&mut self, f: &mut Frame, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(area);

        let top_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(rows[0]);

        self.render_suggestions(f, top_cols[0]);
        self.render_resonances(f, top_cols[1]);
        self.render_civilizations(f, rows[1]);
    }

    fn render_suggestions(&mut self, f: &mut Frame, area: Rect) {
        let title = format!("Focus Suggestions ({})", self.suggestions.len());
        let items: Vec<ListItem> = self
            .suggestions
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let label = if s.content_preview.len() > 24 {
                    format!("{:.23}…", &s.content_preview[..23])
                } else {
                    s.content_preview.clone()
                };
                let score_bar = filled_bar(s.score.min(1.0).max(0.0), 8);
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{:>2}. ", i + 1), Style::default().fg(C_DIM)),
                    Span::styled(format!("{:<25}", label), Style::default().fg(C_TEXT)),
                    Span::styled(score_bar, Style::default().fg(Color::Rgb(60, 160, 255))),
                    Span::styled(format!(" {:.2}", s.score), Style::default().fg(C_DIM)),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(styled_block(&title, false))
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(20, 40, 90))
                    .fg(C_SELECT)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        let mut state = self.suggest_state.clone();
        if state.selected().is_none() && !self.suggestions.is_empty() {
            state.select(Some(0));
        }
        f.render_stateful_widget(list, area, &mut state);
        self.suggest_state = state;
    }

    fn render_resonances(&self, f: &mut Frame, area: Rect) {
        let title = format!("Resonant Pairs ({})", self.resonances.len());
        let block = styled_block(&title, false);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let lines: Vec<Line> = self
            .resonances
            .iter()
            .take(inner.height as usize)
            .map(|p| {
                let la = self
                    .workspace
                    .graph
                    .get_node(p.node_a)
                    .map(|n| n.content.as_str())
                    .unwrap_or("?");
                let lb = self
                    .workspace
                    .graph
                    .get_node(p.node_b)
                    .map(|n| n.content.as_str())
                    .unwrap_or("?");
                let la = if la.len() > 14 { &la[..14] } else { la };
                let lb = if lb.len() > 14 { &lb[..14] } else { lb };
                let sim_bar = filled_bar(p.similarity, 6);
                let civ_mark = if p.same_civilization { " ★" } else { "" };
                Line::from(vec![
                    Span::styled(sim_bar, Style::default().fg(C_PURPLE)),
                    Span::styled(format!(" {:.2} ", p.similarity), Style::default().fg(C_DIM)),
                    Span::styled(format!("{:<14}", la), Style::default().fg(C_TEXT)),
                    Span::styled(" ↔ ", Style::default().fg(C_BORDER_H)),
                    Span::styled(format!("{:<14}", lb), Style::default().fg(C_TEXT)),
                    Span::styled(civ_mark, Style::default().fg(C_WARN)),
                ])
            })
            .collect();

        f.render_widget(Paragraph::new(lines), inner);
    }

    fn render_civilizations(&self, f: &mut Frame, area: Rect) {
        let title = format!("Civilizations ({})", self.civs.len());
        let block = styled_block(&title, false);
        let inner = block.inner(area);
        f.render_widget(block, area);

        if self.civs.is_empty() {
            f.render_widget(
                Paragraph::new(Span::styled(
                    "  no civilizations detected (need denser connected clusters)",
                    Style::default().fg(C_DIM),
                )),
                inner,
            );
            return;
        }

        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Ratio(1, self.civs.len().max(1) as u32);
                self.civs.len().min(4)
            ])
            .split(inner);

        for (i, (civ, col)) in self.civs.iter().zip(cols.iter()).enumerate() {
            let dominant = civ
                .dominant_node
                .and_then(|id| self.workspace.graph.get_node(id))
                .map(|n| n.content.as_str())
                .unwrap_or("?");
            let dominant = if dominant.len() > 16 {
                &dominant[..16]
            } else {
                dominant
            };
            let density_bar = filled_bar(civ.internal_density, 10);

            let lines = vec![
                Line::from(Span::styled(
                    format!("Civ {}", i + 1),
                    Style::default().fg(C_TITLE).add_modifier(Modifier::BOLD),
                )),
                Line::from(vec![
                    Span::styled("members  ", Style::default().fg(C_DIM)),
                    Span::styled(
                        civ.member_nodes.len().to_string(),
                        Style::default().fg(C_TEXT),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("density  ", Style::default().fg(C_DIM)),
                    Span::styled(density_bar, Style::default().fg(C_GOOD)),
                    Span::styled(
                        format!(" {:.2}", civ.internal_density),
                        Style::default().fg(C_DIM),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("age      ", Style::default().fg(C_DIM)),
                    Span::styled(format!("{:.1}d", civ.age_days), Style::default().fg(C_TEXT)),
                ]),
                Line::from(vec![
                    Span::styled("dominant ", Style::default().fg(C_DIM)),
                    Span::styled(dominant, Style::default().fg(C_SELECT)),
                ]),
            ];

            f.render_widget(
                Paragraph::new(lines).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(C_BORDER)),
                ),
                *col,
            );
        }
    }

    // ── Tab 4: Oracle ────────────────────────────────────────────────────────

    fn render_oracle(&self, f: &mut Frame, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(35), // oracle signals
                Constraint::Percentage(40), // rituals
                Constraint::Percentage(25), // contracts
            ])
            .split(area);

        self.render_oracle_signals(f, rows[0]);

        let bot = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(rows[1]);
        self.render_rituals(f, bot[0]);
        self.render_contracts(f, bot[1]);

        self.render_oracle_weight(f, rows[2]);
    }

    fn render_oracle_signals(&self, f: &mut Frame, area: Rect) {
        let title = format!("Oracle Signals ({})", self.oracle.len());
        let block = styled_block(&title, false);
        let inner = block.inner(area);
        f.render_widget(block, area);

        if self.oracle.is_empty() {
            f.render_widget(
                Paragraph::new(Span::styled(
                    "  silence — no signals",
                    Style::default().fg(C_DIM),
                )),
                inner,
            );
            return;
        }

        let lines: Vec<Line> = self
            .oracle
            .iter()
            .take(inner.height as usize)
            .map(|sig| {
                let bar = filled_bar(sig.strength, 10);
                let color = if sig.strength > 0.7 {
                    C_BAD
                } else if sig.strength > 0.4 {
                    C_WARN
                } else {
                    C_GOOD
                };
                Line::from(vec![
                    Span::styled(bar, Style::default().fg(color)),
                    Span::styled(
                        format!(" {:.2}  ", sig.strength),
                        Style::default().fg(C_DIM),
                    ),
                    Span::styled(&sig.description, Style::default().fg(C_TEXT)),
                ])
            })
            .collect();
        f.render_widget(Paragraph::new(lines), inner);
    }

    fn render_rituals(&self, f: &mut Frame, area: Rect) {
        let title = format!("Behavioral Rituals ({})", self.rituals.len());
        let block = styled_block(&title, false);
        let inner = block.inner(area);
        f.render_widget(block, area);

        if self.rituals.is_empty() {
            f.render_widget(
                Paragraph::new(Span::styled(
                    "  no rituals detected yet",
                    Style::default().fg(C_DIM),
                )),
                inner,
            );
            return;
        }

        let mut lines: Vec<Line> = Vec::new();
        for r in &self.rituals {
            lines.push(Line::from(vec![
                Span::styled(
                    &r.name,
                    Style::default().fg(C_TITLE).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  ×{}  str={:.2}", r.occurrence_count, r.strength),
                    Style::default().fg(C_DIM),
                ),
            ]));
            let steps: Vec<String> = r
                .sequence
                .iter()
                .take(5)
                .map(|id| {
                    self.workspace
                        .graph
                        .get_node(*id)
                        .map(|n| {
                            let c = &n.content;
                            if c.len() > 12 {
                                c[..12].to_string()
                            } else {
                                c.clone()
                            }
                        })
                        .unwrap_or_else(|| "?".to_string())
                })
                .collect();
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(steps.join(" → "), Style::default().fg(C_DIM)),
            ]));
        }
        f.render_widget(Paragraph::new(lines), inner);
    }

    fn render_contracts(&self, f: &mut Frame, area: Rect) {
        let title = format!("Silent Contracts ({})", self.contracts.len());
        let block = styled_block(&title, false);
        let inner = block.inner(area);
        f.render_widget(block, area);

        if self.contracts.is_empty() {
            f.render_widget(
                Paragraph::new(Span::styled("  no contracts", Style::default().fg(C_DIM))),
                inner,
            );
            return;
        }

        let lines: Vec<Line> = self
            .contracts
            .iter()
            .take(inner.height as usize)
            .map(|c| {
                let desc = if c.description.len() > inner.width as usize - 4 {
                    format!(
                        "{:.width$}…",
                        &c.description,
                        width = inner.width as usize - 5
                    )
                } else {
                    c.description.clone()
                };
                Line::from(vec![
                    Span::styled("⚠ ", Style::default().fg(C_WARN)),
                    Span::styled(desc, Style::default().fg(C_TEXT)),
                ])
            })
            .collect();
        f.render_widget(Paragraph::new(lines), inner);
    }

    fn render_oracle_weight(&self, f: &mut Frame, area: Rect) {
        let weight = self.workspace.cognitive_weight();
        let block = styled_block("Cognitive Weight", false);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let density_bar = filled_bar(weight.visual_density(), 20);
        let drag_bar = filled_bar(weight.particle_drag(), 20);

        let lines = vec![
            Line::from(vec![
                Span::styled("Visual density  ", Style::default().fg(C_DIM)),
                Span::styled(
                    &density_bar,
                    Style::default().fg(if weight.is_heavy() { C_BAD } else { C_GOOD }),
                ),
                Span::styled(
                    format!(" {:.3}", weight.visual_density()),
                    Style::default().fg(C_DIM),
                ),
            ]),
            Line::from(vec![
                Span::styled("Particle drag   ", Style::default().fg(C_DIM)),
                Span::styled(&drag_bar, Style::default().fg(Color::Rgb(100, 120, 200))),
                Span::styled(
                    format!(" {:.3}", weight.particle_drag()),
                    Style::default().fg(C_DIM),
                ),
            ]),
            Line::from(vec![
                Span::styled("Summary         ", Style::default().fg(C_DIM)),
                Span::styled(
                    weight.summary(),
                    Style::default().fg(if weight.is_heavy() { C_WARN } else { C_TEXT }),
                ),
            ]),
        ];
        f.render_widget(Paragraph::new(lines), inner);
    }
    // ── Tab 5: Analytics ─────────────────────────────────────────────────────

    fn render_analytics(&mut self, f: &mut Frame, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(5), Constraint::Min(0)])
            .split(area);

        self.render_health_strip(f, rows[0]);

        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40),
                Constraint::Percentage(30),
                Constraint::Percentage(30),
            ])
            .split(rows[1]);

        self.render_pagerank_list(f, cols[0]);
        self.render_centrality_bars(f, cols[1]);
        self.render_bridge_list(f, cols[2]);
    }

    fn render_health_strip(&self, f: &mut Frame, area: Rect) {
        let block = styled_block("Graph Health", false);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let Some(h) = &self.health else {
            f.render_widget(
                Paragraph::new(Span::styled("computing…", Style::default().fg(C_DIM))),
                inner,
            );
            return;
        };

        let score_color = if h.score > 0.75 {
            C_GOOD
        } else if h.score > 0.5 {
            C_WARN
        } else {
            C_BAD
        };
        let label_str = if h.score > 0.75 {
            "Thriving"
        } else if h.score > 0.5 {
            "Healthy"
        } else if h.score > 0.25 {
            "Fragile"
        } else {
            "Critical"
        };

        let score_bar = filled_bar(h.score as f32, 20);
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(inner);

        let left_lines = vec![
            Line::from(vec![
                Span::styled(score_bar, Style::default().fg(score_color)),
                Span::styled(
                    format!(" {:.3}  {label_str}", h.score),
                    Style::default()
                        .fg(score_color)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("density    ", Style::default().fg(C_DIM)),
                Span::styled(format!("{:.4}", h.density), Style::default().fg(C_TEXT)),
                Span::styled("   activity  ", Style::default().fg(C_DIM)),
                Span::styled(
                    format!("{:.0}%", h.activity_rate * 100.0),
                    Style::default().fg(C_GOOD),
                ),
            ]),
        ];
        let right_lines = vec![
            Line::from(vec![
                Span::styled("components  ", Style::default().fg(C_DIM)),
                Span::styled(h.component_count.to_string(), Style::default().fg(C_TEXT)),
                Span::styled("   bridges  ", Style::default().fg(C_DIM)),
                Span::styled(
                    h.bridge_count.to_string(),
                    Style::default().fg(if h.bridge_count > 3 { C_WARN } else { C_TEXT }),
                ),
            ]),
            Line::from(vec![
                Span::styled("decay      ", Style::default().fg(C_DIM)),
                Span::styled(
                    format!("{:.0}%", h.decay_ratio * 100.0),
                    Style::default().fg(if h.decay_ratio > 0.3 { C_BAD } else { C_TEXT }),
                ),
                Span::styled("   avg-η  ", Style::default().fg(C_DIM)),
                Span::styled(
                    format!("{:.3}", h.avg_entropy),
                    Style::default().fg(entropy_color(h.avg_entropy as f32)),
                ),
            ]),
        ];
        f.render_widget(Paragraph::new(left_lines), cols[0]);
        f.render_widget(Paragraph::new(right_lines), cols[1]);
    }

    fn render_pagerank_list(&mut self, f: &mut Frame, area: Rect) {
        let title = format!("PageRank ({})", self.pagerank.len());
        let max_score = self
            .pagerank
            .first()
            .map(|e| e.score)
            .unwrap_or(1.0)
            .max(0.001);

        let items: Vec<ListItem> = self
            .pagerank
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let bar = filled_bar((e.score / max_score) as f32, 10);
                let label = if e.content_preview.len() > 22 {
                    format!("{}…", &e.content_preview[..21])
                } else {
                    e.content_preview.clone()
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{:>2}. ", i + 1), Style::default().fg(C_DIM)),
                    Span::styled(format!("{:<23}", label), Style::default().fg(C_TEXT)),
                    Span::styled(bar, Style::default().fg(Color::Rgb(60, 180, 255))),
                    Span::styled(format!(" {:.4}", e.score), Style::default().fg(C_DIM)),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(styled_block(&title, self.tab == 5))
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(20, 50, 100))
                    .fg(C_SELECT)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        let mut state = self.pagerank_state.clone();
        if state.selected().is_none() && !self.pagerank.is_empty() {
            state.select(Some(0));
        }
        f.render_stateful_widget(list, area, &mut state);
        self.pagerank_state = state;
    }

    fn render_centrality_bars(&self, f: &mut Frame, area: Rect) {
        let title = format!("Betweenness ({})", self.centrality.len());
        let block = styled_block(&title, false);
        let inner = block.inner(area);
        f.render_widget(block, area);

        if self.centrality.is_empty() {
            f.render_widget(
                Paragraph::new(Span::styled("  no data", Style::default().fg(C_DIM))),
                inner,
            );
            return;
        }

        let max_bc = self
            .centrality
            .first()
            .map(|e| e.betweenness)
            .unwrap_or(1.0)
            .max(0.001);
        let bar_w = (inner.width as usize).saturating_sub(14).max(4);

        let lines: Vec<Line> = self
            .centrality
            .iter()
            .take(inner.height as usize)
            .map(|e| {
                let label = if e.content_preview.len() > 12 {
                    format!("{}…", &e.content_preview[..11])
                } else {
                    format!("{:<12}", e.content_preview)
                };
                let bar = filled_bar((e.betweenness / max_bc) as f32, bar_w);
                Line::from(vec![
                    Span::styled(format!("{label} "), Style::default().fg(C_TEXT)),
                    Span::styled(bar, Style::default().fg(C_PURPLE)),
                ])
            })
            .collect();

        f.render_widget(Paragraph::new(lines), inner);
    }

    fn render_bridge_list(&self, f: &mut Frame, area: Rect) {
        let title = format!("Bridges ({})", self.bridges.len());
        let block = styled_block(&title, false);
        let inner = block.inner(area);
        f.render_widget(block, area);

        if self.bridges.is_empty() {
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(Span::styled("  no bridges", Style::default().fg(C_GOOD))),
                    Line::from(Span::styled(
                        "  graph is robust",
                        Style::default().fg(C_DIM),
                    )),
                ]),
                inner,
            );
            return;
        }

        let mut lines: Vec<Line> = Vec::new();
        for b in &self.bridges {
            let src = if b.source_preview.len() > 16 {
                format!("{}…", &b.source_preview[..15])
            } else {
                b.source_preview.clone()
            };
            let tgt = if b.target_preview.len() > 16 {
                format!("{}…", &b.target_preview[..15])
            } else {
                b.target_preview.clone()
            };
            lines.push(Line::from(vec![
                Span::styled("⚡ ", Style::default().fg(C_WARN)),
                Span::styled(src, Style::default().fg(C_TEXT)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("   → ", Style::default().fg(C_BORDER_H)),
                Span::styled(tgt, Style::default().fg(C_TEXT)),
                Span::styled(format!(" w={:.2}", b.weight), Style::default().fg(C_DIM)),
            ]));
            if lines.len() >= inner.height as usize {
                break;
            }
        }
        f.render_widget(Paragraph::new(lines), inner);
    }

    // ── Tab 7: Identity ──────────────────────────────────────────────────────

    fn render_identity(&self, f: &mut Frame, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(area);

        let top = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
            .split(rows[0]);

        let bot = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[1]);

        self.render_living_signature(f, top[0]);
        self.render_shadow_projects(f, top[1]);
        self.render_lore_arcs(f, bot[0]);
        self.render_shift_history(f, bot[1]);
    }

    fn render_living_signature(&self, f: &mut Frame, area: Rect) {
        let block = styled_block("Living Signature", self.tab == 7);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let Some(sig) = &self.living_sig else {
            f.render_widget(
                Paragraph::new(Span::styled(
                    "  computing signature…",
                    Style::default().fg(C_DIM),
                )),
                inner,
            );
            return;
        };

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),    // ASCII art
                Constraint::Length(6), // metrics
            ])
            .split(inner);

        // ASCII art panel
        let art_rows = rows[0];
        let art_w = art_rows.width as usize;
        let art_h = art_rows.height as usize;
        let art_lines: Vec<Line> = sig_ascii_art(sig, art_w, art_h)
            .into_iter()
            .map(|row| {
                Line::from(Span::styled(
                    row,
                    Style::default().fg(sig_primary_color(sig)),
                ))
            })
            .collect();
        f.render_widget(Paragraph::new(art_lines), art_rows);

        // Metrics panel
        let pc = sig.primary_color;
        let sc = sig.secondary_color;
        let ac = sig.accent_color;

        let geo_col = match sig.geometry {
            GeometryKind::Circle => Color::Rgb(64, 200, 255),
            GeometryKind::Wave => Color::Rgb(140, 80, 255),
            GeometryKind::Spiral => Color::Rgb(60, 220, 120),
            GeometryKind::Fractal => Color::Rgb(230, 180, 50),
            GeometryKind::Line => Color::Rgb(180, 210, 255),
        };

        let metric_lines = vec![
            Line::from(vec![
                Span::styled(
                    format!(
                        "{} {} {} ",
                        sig.geometry.as_str(),
                        sig.symmetry.as_str(),
                        sig.motion.as_str()
                    ),
                    Style::default().fg(geo_col).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("v{}", sig.evolution_count),
                    Style::default().fg(C_DIM),
                ),
            ]),
            Line::from(vec![
                Span::styled("complexity ", Style::default().fg(C_DIM)),
                Span::styled(filled_bar(sig.complexity, 8), Style::default().fg(C_PURPLE)),
                Span::styled(
                    format!(" {:.2}", sig.complexity),
                    Style::default().fg(C_DIM),
                ),
            ]),
            Line::from(vec![
                Span::styled("vitality   ", Style::default().fg(C_DIM)),
                Span::styled(filled_bar(sig.vitality, 8), Style::default().fg(C_GOOD)),
                Span::styled(format!(" {:.2}", sig.vitality), Style::default().fg(C_DIM)),
            ]),
            Line::from(vec![
                Span::styled("depth      ", Style::default().fg(C_DIM)),
                Span::styled(filled_bar(sig.depth, 8), Style::default().fg(C_FOSSIL)),
                Span::styled(format!(" {:.2}", sig.depth), Style::default().fg(C_DIM)),
            ]),
            Line::from(vec![
                Span::styled("colors     ", Style::default().fg(C_DIM)),
                Span::styled(
                    "██",
                    Style::default().fg(Color::Rgb(
                        (pc[0] * 255.0) as u8,
                        (pc[1] * 255.0) as u8,
                        (pc[2] * 255.0) as u8,
                    )),
                ),
                Span::styled(
                    "██",
                    Style::default().fg(Color::Rgb(
                        (sc[0] * 255.0) as u8,
                        (sc[1] * 255.0) as u8,
                        (sc[2] * 255.0) as u8,
                    )),
                ),
                Span::styled(
                    "██",
                    Style::default().fg(Color::Rgb(
                        (ac[0] * 255.0) as u8,
                        (ac[1] * 255.0) as u8,
                        (ac[2] * 255.0) as u8,
                    )),
                ),
            ]),
        ];
        f.render_widget(Paragraph::new(metric_lines), rows[1]);
    }

    fn render_shadow_projects(&self, f: &mut Frame, area: Rect) {
        let title = format!(
            "Shadow Projects ({}) — \"what you chose not to build\"",
            self.shadow_projects.len()
        );
        let block = styled_block(&title, false);
        let inner = block.inner(area);
        f.render_widget(block, area);

        if self.shadow_projects.is_empty() {
            let lines = vec![
                Line::from(Span::styled(
                    "  No shadow projects detected.",
                    Style::default().fg(C_DIM),
                )),
                Line::from(Span::styled(
                    "  Shadow projects emerge from abandoned high-gravity",
                    Style::default().fg(C_DIM),
                )),
                Line::from(Span::styled(
                    "  nodes, long-incubating void ideas, or released",
                    Style::default().fg(C_DIM),
                )),
                Line::from(Span::styled(
                    "  digital shadows.",
                    Style::default().fg(C_DIM),
                )),
            ];
            f.render_widget(Paragraph::new(lines), inner);
            return;
        }

        let mut lines: Vec<Line> = Vec::new();
        for sp in self.shadow_projects.iter().take(inner.height as usize / 3) {
            let lum_bar = filled_bar(sp.luminescence, 6);
            let label = if sp.label.len() > 28 {
                format!("{}…", &sp.label[..27])
            } else {
                format!("{:<28}", sp.label)
            };
            lines.push(Line::from(vec![
                Span::styled("◈ ", Style::default().fg(C_VOID)),
                Span::styled(lum_bar, Style::default().fg(Color::Rgb(100, 30, 120))),
                Span::styled(
                    format!(" {:.2} ", sp.luminescence),
                    Style::default().fg(C_DIM),
                ),
                Span::styled(
                    label,
                    Style::default().fg(C_TEXT).add_modifier(Modifier::BOLD),
                ),
            ]));
            // description
            let desc = if sp.description.len() > inner.width as usize - 6 {
                format!("{}…", &sp.description[..inner.width as usize - 7])
            } else {
                sp.description.clone()
            };
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(desc, Style::default().fg(C_DIM)),
            ]));
            lines.push(Line::from(Span::styled(
                "  ─────────────────────────────────────────────",
                Style::default().fg(C_BORDER),
            )));
        }
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    }

    fn render_lore_arcs(&self, f: &mut Frame, area: Rect) {
        let title = format!("Personal Lore ({} arcs)", self.lore_arcs.len());
        let block = styled_block(&title, false);
        let inner = block.inner(area);
        f.render_widget(block, area);

        if self.lore_arcs.is_empty() {
            let lines = vec![
                Line::from(Span::styled(
                    "  No lore arcs detected yet.",
                    Style::default().fg(C_DIM),
                )),
                Line::from(Span::styled(
                    "  Lore emerges from long-term patterns:",
                    Style::default().fg(C_DIM),
                )),
                Line::from(Span::styled(
                    "  tectonic events, seasonal shifts,",
                    Style::default().fg(C_DIM),
                )),
                Line::from(Span::styled(
                    "  crystallizations, and old fossils.",
                    Style::default().fg(C_DIM),
                )),
                Line::from(Span::styled("", Style::default())),
                Line::from(Span::styled(
                    "  Run: cargo run -- lore-chronicle",
                    Style::default().fg(C_DIM),
                )),
            ];
            f.render_widget(Paragraph::new(lines), inner);
            return;
        }

        let mut lines: Vec<Line> = Vec::new();
        for entry in self.lore_arcs.iter().take(inner.height as usize) {
            let (icon, col) = arc_icon_color(&entry.arc_type);
            let title_s = if entry.title.len() > inner.width as usize - 14 {
                format!("{}…", &entry.title[..inner.width as usize - 15])
            } else {
                entry.title.clone()
            };
            let sig_bar = filled_bar(entry.significance, 5);
            lines.push(Line::from(vec![
                Span::styled(format!("{icon} "), Style::default().fg(col)),
                Span::styled(
                    format!("{:<8} ", format!("{:?}", entry.arc_type).to_lowercase()),
                    Style::default().fg(C_DIM),
                ),
                Span::styled(sig_bar, Style::default().fg(col)),
                Span::styled(format!(" {}", title_s), Style::default().fg(C_TEXT)),
            ]));
        }
        f.render_widget(Paragraph::new(lines), inner);
    }

    fn render_shift_history(&self, f: &mut Frame, area: Rect) {
        let block = styled_block("Signature Evolution", false);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let Some(sig) = &self.living_sig else {
            f.render_widget(
                Paragraph::new(Span::styled(
                    "  no signature yet",
                    Style::default().fg(C_DIM),
                )),
                inner,
            );
            return;
        };

        let mut lines: Vec<Line> = Vec::new();

        // Current state header
        lines.push(Line::from(vec![
            Span::styled("Current  ", Style::default().fg(C_DIM)),
            Span::styled(
                format!(
                    "{} {} {}",
                    sig.geometry.as_str(),
                    sig.symmetry.as_str(),
                    sig.motion.as_str()
                ),
                Style::default().fg(C_TITLE).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("         ", Style::default()),
            Span::styled(sig.geometry.description(), Style::default().fg(C_DIM)),
        ]));
        lines.push(Line::from(Span::styled(
            "  ─────────────────────────────────────────────",
            Style::default().fg(C_BORDER),
        )));

        if self.workspace.identity.shift_history.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No signature shifts recorded yet.",
                Style::default().fg(C_DIM),
            )));
            lines.push(Line::from(Span::styled(
                "  Shifts occur when geometry, symmetry, or motion",
                Style::default().fg(C_DIM),
            )));
            lines.push(Line::from(Span::styled(
                "  change significantly between recomputations.",
                Style::default().fg(C_DIM),
            )));
        } else {
            for shift in self
                .workspace
                .identity
                .shift_history
                .iter()
                .rev()
                .take(inner.height as usize - 4)
            {
                let mag_bar = filled_bar(shift.magnitude.clamp(0.0, 1.0), 6);
                lines.push(Line::from(vec![
                    Span::styled(
                        shift.at.format("%Y-%m-%d ").to_string(),
                        Style::default().fg(C_DIM),
                    ),
                    Span::styled(mag_bar, Style::default().fg(C_PURPLE)),
                    Span::styled(
                        format!(" {}", shift.description),
                        Style::default().fg(C_TEXT),
                    ),
                ]));
            }
        }

        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    }

    // ── Tab 6: Dream ─────────────────────────────────────────────────────────

    fn render_dream(&mut self, f: &mut Frame, area: Rect) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
            .split(area);

        self.render_proposal_list(f, cols[0]);
        self.render_proposal_detail(f, cols[1]);
    }

    fn render_proposal_list(&mut self, f: &mut Frame, area: Rect) {
        let title = format!("Dream Proposals ({})", self.proposals.len());

        let items: Vec<ListItem> = self
            .proposals
            .iter()
            .map(|p| {
                let (icon, color) = proposal_icon_color(&p.kind);
                let bar = filled_bar(p.confidence, 8);
                let desc = if p.description.len() > 36 {
                    format!("{}…", &p.description[..35])
                } else {
                    p.description.clone()
                };
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{icon} "),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!("{:<37}", desc), Style::default().fg(C_TEXT)),
                    Span::styled(bar, Style::default().fg(color)),
                    Span::styled(format!(" {:.2}", p.confidence), Style::default().fg(C_DIM)),
                ]))
            })
            .collect();

        let list = if items.is_empty() {
            List::new(vec![ListItem::new(Line::from(Span::styled(
                "  No proposals — graph may be sparse",
                Style::default().fg(C_DIM),
            )))])
            .block(styled_block(&title, self.tab == 6))
        } else {
            List::new(items)
                .block(styled_block(&title, self.tab == 6))
                .highlight_style(
                    Style::default()
                        .bg(Color::Rgb(20, 20, 60))
                        .fg(C_SELECT)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▶ ")
        };

        let mut state = self.proposal_state.clone();
        if state.selected().is_none() && !self.proposals.is_empty() {
            state.select(Some(0));
        }
        f.render_stateful_widget(list, area, &mut state);
        self.proposal_state = state;
    }

    fn render_proposal_detail(&self, f: &mut Frame, area: Rect) {
        let block = styled_block("Proposal Detail", false);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let idx = self.proposal_state.selected().unwrap_or(0);
        let Some(p) = self.proposals.get(idx) else {
            f.render_widget(
                Paragraph::new(Span::styled(
                    "  ↑ select a proposal",
                    Style::default().fg(C_DIM),
                )),
                inner,
            );
            return;
        };

        let (icon, color) = proposal_icon_color(&p.kind);
        let kind_name = match &p.kind {
            ProposalKind::SuggestEdge { .. } => "Suggest Edge",
            ProposalKind::ReviveGhost { .. } => "Revive Ghost",
            ProposalKind::MergeNodes { .. } => "Merge Nodes",
            ProposalKind::EntropyAlert { .. } => "Entropy Alert",
        };

        let conf_bar = filled_bar(p.confidence, 12);
        let mut lines = vec![
            Line::from(Span::styled(
                format!("{icon} {kind_name}"),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled("confidence  ", Style::default().fg(C_DIM)),
                Span::styled(&conf_bar, Style::default().fg(color)),
                Span::styled(format!(" {:.3}", p.confidence), Style::default().fg(C_DIM)),
            ]),
            Line::from(Span::styled(
                "────────────────────",
                Style::default().fg(C_BORDER),
            )),
        ];

        match &p.kind {
            ProposalKind::SuggestEdge {
                from,
                to,
                similarity,
            } => {
                let a = node_label(&self.workspace, *from);
                let b = node_label(&self.workspace, *to);
                lines.push(detail_line("from", a, C_TITLE));
                lines.push(detail_line("to  ", b, C_TITLE));
                lines.push(detail_line("sim ", format!("{similarity:.3}"), C_TEXT));
                lines.push(Line::from(Span::styled(
                    "  → add connection",
                    Style::default().fg(C_DIM),
                )));
            }
            ProposalKind::ReviveGhost { node_id } => {
                let label = node_label(&self.workspace, *node_id);
                lines.push(detail_line("node", label, C_GHOST));
                if let Some(n) = self.workspace.graph.get_node(*node_id) {
                    lines.push(detail_line(
                        "η   ",
                        format!("{:.3}", n.entropy),
                        entropy_color(n.entropy),
                    ));
                    lines.push(detail_line("G   ", format!("{:.3}", n.gravity), C_TEXT));
                }
                lines.push(Line::from(Span::styled(
                    "  → revive-node <id>",
                    Style::default().fg(C_DIM),
                )));
            }
            ProposalKind::MergeNodes { a, b, similarity } => {
                let la = node_label(&self.workspace, *a);
                let lb = node_label(&self.workspace, *b);
                lines.push(detail_line("node A", la, C_WARN));
                lines.push(detail_line("node B", lb, C_WARN));
                lines.push(detail_line("sim   ", format!("{similarity:.3}"), C_TEXT));
                lines.push(Line::from(Span::styled(
                    "  → review + merge manually",
                    Style::default().fg(C_DIM),
                )));
            }
            ProposalKind::EntropyAlert { node_id, entropy } => {
                let label = node_label(&self.workspace, *node_id);
                lines.push(detail_line("node", label, C_BAD));
                lines.push(detail_line(
                    "η   ",
                    format!("{entropy:.3}"),
                    entropy_color(*entropy),
                ));
                lines.push(Line::from(Span::styled(
                    "  → focus or restructure",
                    Style::default().fg(C_DIM),
                )));
            }
        }

        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    }
}

// ── Widget helpers ────────────────────────────────────────────────────────────

fn styled_block(title: &str, highlighted: bool) -> Block<'_> {
    let border_color = if highlighted { C_BORDER_H } else { C_BORDER };
    Block::default()
        .title(Span::styled(
            format!(" {} ", title),
            Style::default().fg(C_TITLE).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
}

fn stat_line(label: &'static str, value: usize, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{:<10}", label), Style::default().fg(C_DIM)),
        Span::styled(
            value.to_string(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ])
}

fn gauge_line(label: &str, value: f32) -> Line<'_> {
    let bar = filled_bar(value, 10);
    Line::from(vec![
        Span::styled(format!("{:<12}", label), Style::default().fg(C_DIM)),
        Span::styled(bar, Style::default().fg(Color::Rgb(60, 160, 220))),
        Span::styled(format!(" {:.2}", value), Style::default().fg(C_DIM)),
    ])
}

fn proposal_icon_color(kind: &ProposalKind) -> (&'static str, Color) {
    match kind {
        ProposalKind::SuggestEdge { .. } => ("⟿", Color::Rgb(64, 200, 255)),
        ProposalKind::ReviveGhost { .. } => ("◉", Color::Rgb(140, 80, 255)),
        ProposalKind::MergeNodes { .. } => ("⊕", Color::Rgb(230, 180, 50)),
        ProposalKind::EntropyAlert { .. } => ("⚠", Color::Rgb(220, 70, 70)),
    }
}

fn node_label(workspace: &crate::workspace::SilentNodeWorkspace, id: uuid::Uuid) -> String {
    workspace
        .graph
        .get_node(id)
        .map(|n| {
            if n.content.len() > 22 {
                format!("{}…", &n.content[..21])
            } else {
                n.content.clone()
            }
        })
        .unwrap_or_else(|| id.to_string())
}

fn detail_line(label: &'static str, value: String, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<6}  "), Style::default().fg(C_DIM)),
        Span::styled(
            value,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ])
}

// ── Identity helpers ──────────────────────────────────────────────────────────

fn sig_primary_color(sig: &LivingSignature) -> Color {
    let c = sig.primary_color;
    Color::Rgb(
        (c[0] * 255.0) as u8,
        (c[1] * 255.0) as u8,
        (c[2] * 255.0) as u8,
    )
}

fn arc_icon_color(arc: &ArcType) -> (&'static str, Color) {
    match arc {
        ArcType::Origin => ("◉", Color::Rgb(64, 200, 255)),
        ArcType::Conflict => ("⚔", Color::Rgb(220, 70, 70)),
        ArcType::Resolution => ("◇", Color::Rgb(60, 220, 120)),
        ArcType::Revelation => ("✦", Color::Rgb(255, 210, 60)),
        ArcType::Transformation => ("⟳", Color::Rgb(140, 80, 255)),
        ArcType::Legacy => ("◆", Color::Rgb(100, 160, 255)),
        ArcType::Tectonic => ("⚡", Color::Rgb(230, 180, 50)),
    }
}

/// Render the Living Signature as a simple ASCII art grid.
/// Uses the geometry type to determine the pattern.
fn sig_ascii_art(sig: &LivingSignature, width: usize, height: usize) -> Vec<String> {
    if width < 4 || height < 4 {
        return vec!["◈".to_string()];
    }

    let w = width.min(40);
    let h = height.min(16);
    let cx = w as f64 / 2.0;
    let cy = h as f64 / 2.0;
    let r = (w.min(h * 2) as f64 / 2.0) * 0.82;
    let folds = sig.symmetry.fold_count();
    let complexity = sig.complexity as f64;

    let mut grid: Vec<Vec<char>> = vec![vec![' '; w]; h];

    let plot = |grid: &mut Vec<Vec<char>>, x: f64, y: f64, ch: char| {
        let xi = x.round() as i32;
        let yi = y.round() as i32;
        if xi >= 0 && xi < w as i32 && yi >= 0 && yi < h as i32 {
            grid[yi as usize][xi as usize] = ch;
        }
    };

    match sig.geometry {
        GeometryKind::Circle => {
            let rings = 1 + (complexity * 3.0) as usize;
            for ring in 0..rings {
                let rr = r * (ring + 1) as f64 / (rings + 1) as f64;
                for fold in 0..folds {
                    let steps = (rr * std::f64::consts::PI * 2.0 * 3.0) as usize + 16;
                    for i in 0..steps {
                        let a = fold as f64 * 2.0 * std::f64::consts::PI / folds as f64
                            + i as f64 * 2.0 * std::f64::consts::PI / steps as f64;
                        let x = cx + rr * a.cos() * 2.2;
                        let y = cy + rr * a.sin();
                        let ch = if ring == 0 { '○' } else { '·' };
                        plot(&mut grid, x, y, ch);
                    }
                }
            }
            // Radial spokes
            for fold in 0..folds {
                let a = fold as f64 * 2.0 * std::f64::consts::PI / folds as f64;
                let steps = (r * 1.5) as usize + 4;
                for s in 0..steps {
                    let frac = 0.15 + 0.75 * s as f64 / steps as f64;
                    let x = cx + r * frac * a.cos() * 2.2;
                    let y = cy + r * frac * a.sin();
                    plot(&mut grid, x, y, '─');
                }
            }
        }
        GeometryKind::Spiral => {
            let turns = 2.0 + complexity * 2.0;
            for fold in 0..folds {
                let ao = fold as f64 * 2.0 * std::f64::consts::PI / folds as f64;
                let steps = 120;
                for i in 0..steps {
                    let frac = i as f64 / (steps - 1) as f64;
                    let rr = r * 0.1 + r * 0.85 * frac;
                    let a = ao + turns * 2.0 * std::f64::consts::PI * frac;
                    let x = cx + rr * a.cos() * 2.2;
                    let y = cy + rr * a.sin();
                    let ch = if frac < 0.6 {
                        '·'
                    } else if frac < 0.85 {
                        '○'
                    } else {
                        '◆'
                    };
                    plot(&mut grid, x, y, ch);
                }
            }
        }
        GeometryKind::Wave => {
            for fold in 0..folds {
                let ao = fold as f64 * std::f64::consts::PI / folds as f64;
                let freq = 3.0 + complexity * 3.0;
                let amp = r * 0.3 * (0.5 + complexity * 0.5);
                let steps = 100;
                for i in 0..steps {
                    let t = i as f64 / (steps - 1) as f64;
                    let ba = ao + t * 2.0 * std::f64::consts::PI;
                    let wave = amp * (freq * t * 2.0 * std::f64::consts::PI + fold as f64).sin();
                    let rr = r * 0.55 + wave;
                    let x = cx + rr * ba.cos() * 2.2;
                    let y = cy + rr * ba.sin();
                    let ch = if wave.abs() < r * 0.05 { '─' } else { '~' };
                    plot(&mut grid, x, y, ch);
                }
            }
        }
        GeometryKind::Fractal => {
            fn branch(
                grid: &mut Vec<Vec<char>>,
                cx: f64,
                cy: f64,
                x: f64,
                y: f64,
                angle: f64,
                len: f64,
                depth: u32,
                complexity: f64,
                w: usize,
                h: usize,
            ) {
                if depth == 0 || len < 1.0 {
                    return;
                }
                let x2 = x + len * angle.cos() * 2.0;
                let y2 = y + len * angle.sin();
                let steps = (len * 2.0) as usize + 2;
                for s in 0..steps {
                    let t = s as f64 / steps as f64;
                    let px = x + (x2 - x) * t;
                    let py = y + (y2 - y) * t;
                    let xi = px.round() as i32;
                    let yi = py.round() as i32;
                    if xi >= 0 && xi < w as i32 && yi >= 0 && yi < h as i32 {
                        let ch = if angle.cos().abs() < 0.3 {
                            '│'
                        } else if angle.sin().abs() < 0.3 {
                            '─'
                        } else {
                            '·'
                        };
                        grid[yi as usize][xi as usize] = ch;
                    }
                }
                let spread = std::f64::consts::PI / 4.0 + complexity * std::f64::consts::PI / 8.0;
                branch(
                    grid,
                    cx,
                    cy,
                    x2,
                    y2,
                    angle - spread,
                    len * 0.62,
                    depth - 1,
                    complexity,
                    w,
                    h,
                );
                branch(
                    grid,
                    cx,
                    cy,
                    x2,
                    y2,
                    angle + spread,
                    len * 0.62,
                    depth - 1,
                    complexity,
                    w,
                    h,
                );
            }
            let max_depth = 2 + (complexity * 2.0) as u32;
            for fold in 0..folds {
                let start_a = fold as f64 * 2.0 * std::f64::consts::PI / folds as f64
                    - std::f64::consts::PI / 2.0;
                let sx = cx + r * 0.25 * (start_a + std::f64::consts::PI).cos() * 2.0;
                let sy = cy + r * 0.25 * (start_a + std::f64::consts::PI).sin();
                branch(
                    &mut grid,
                    cx,
                    cy,
                    sx,
                    sy,
                    start_a,
                    r * 0.48,
                    max_depth,
                    complexity,
                    w,
                    h,
                );
            }
        }
        GeometryKind::Line => {
            let n_lines = 3 + (complexity * 3.0) as usize;
            for i in 0..n_lines {
                let frac = i as f64 / (n_lines - 1).max(1) as f64;
                let y0 = cy - r * 0.65 + r * 1.3 * frac;
                let steps = w - 4;
                for j in 0..steps {
                    let t = j as f64 / (steps - 1) as f64;
                    let x = 2.0 + j as f64;
                    let wave = r
                        * 0.15
                        * (std::f64::consts::PI * 3.0 * t + i as f64 * std::f64::consts::PI / 4.0)
                            .sin();
                    let y = y0 + wave;
                    let ch = if wave.abs() < 0.3 { '─' } else { '~' };
                    plot(&mut grid, x, y, ch);
                }
            }
        }
    }

    // Always place center marker
    let vitality = sig.vitality;
    let center_ch = if vitality > 0.6 { '✦' } else { '◇' };
    let cxi = cx.round() as i32;
    let cyi = cy.round() as i32;
    if cxi >= 0 && cxi < w as i32 && cyi >= 0 && cyi < h as i32 {
        grid[cyi as usize][cxi as usize] = center_ch;
    }

    grid.into_iter()
        .map(|row| row.into_iter().collect::<String>())
        .collect()
}

fn bounds_of(nodes: &[(f32, f32, Color)]) -> (f32, f32, f32, f32) {
    if nodes.is_empty() {
        return (-10.0, 10.0, -10.0, 10.0);
    }
    let xs: Vec<f32> = nodes.iter().map(|(x, _, _)| *x).collect();
    let ys: Vec<f32> = nodes.iter().map(|(_, y, _)| *y).collect();
    let xmin = xs.iter().cloned().fold(f32::INFINITY, f32::min);
    let xmax = xs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let ymin = ys.iter().cloned().fold(f32::INFINITY, f32::min);
    let ymax = ys.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    (xmin, xmax, ymin, ymax)
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn run_tui(workspace: SilentNodeWorkspace, db_path: PathBuf) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = TuiApp::new(workspace, db_path);
    let tick = Duration::from_millis(80);

    let result = (|| {
        loop {
            terminal.draw(|f| app.render(f))?;

            if event::poll(tick)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        if app.on_key(key.code) {
                            break;
                        }
                    }
                }
            }

            app.frame += 1;

            // auto-recompute every 60 seconds
            if app.last_compute.elapsed() > Duration::from_secs(60) {
                app.recompute();
            }
        }
        Ok::<(), io::Error>(())
    })();

    // always restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
    )?;
    terminal.show_cursor()?;

    // save on quit if there are unsaved changes
    if app.needs_save {
        app.save();
    }

    result
}
