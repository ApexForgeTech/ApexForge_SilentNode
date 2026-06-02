// ── Node ─────────────────────────────────────────────────────────────────────
export type NodeType =
  | 'idea' | 'memory' | 'project' | 'person'
  | 'artifact' | 'media' | 'process' | 'world'
  | 'ghost' | 'fossil' | 'other'

export interface SNode {
  id: string
  node_type: string          // matches API: snake_case
  custom_type?: string | null
  custom_color?: string | null
  schedule?: NodeSchedule | null
  nickname: string
  content: string
  entropy: number
  gravity: number
  velocity: number
  access_count: number
  is_ghost: boolean
  is_fossil: boolean
  is_void: boolean
  position: { x: number; y: number; z: number }
  created_at: string
  accessed_at: string
  aura_color: string
  entropy_state?: string
  velocity_state?: string
  visual_weight?: number
  contagion_heat?: number
}

export interface NodeSchedule {
  mode: 'once' | 'daily' | 'weekly' | 'interval' | 'custom_days'
  status: 'active' | 'paused' | 'completed'
  start_at?: string | null
  end_at?: string | null
  time_of_day?: string | null
  interval_minutes?: number | null
  days_of_week: number[]
  reminder_enabled: boolean
  reminder_minutes_before: number
}

export interface NotificationSettings {
  telegram_enabled: boolean
  telegram_token_set: boolean
  telegram_token_preview?: string | null
  telegram_chat_id?: string | null
  default_channel: 'app' | 'telegram' | 'both' | string
}

export interface NotificationSettingsUpdate {
  telegram_enabled?: boolean
  telegram_bot_token?: string
  telegram_chat_id?: string
  default_channel?: 'app' | 'telegram' | 'both'
}

export interface ActiveFocus {
  active: boolean
  session_id?: string | null
  node_id?: string | null
  node_nickname?: string | null
  node_preview?: string | null
  depth?: string | null
  started_at?: string | null
  elapsed_seconds: number
  timeout_seconds?: number | null
  remaining_seconds?: number | null
}

// ── Edge ─────────────────────────────────────────────────────────────────────
export interface SEdge {
  source_id: string
  target_id: string
  edge_type: string
  weight: number
  created_at: string
}

// ── Status ───────────────────────────────────────────────────────────────────
export interface Status {
  node_count: number
  edge_count: number
  ghost_count: number
  fossil_count: number
  void_count: number
  focus_events: number
  journal_entries: number
}

// ── Journal ───────────────────────────────────────────────────────────────────
export interface JournalEntry {
  id: string
  content: string
  timestamp: string
  season: string | null
  linked_nodes: string[]
}

// ── Season ───────────────────────────────────────────────────────────────────
export interface SeasonReport {
  season: string
  creation_rate: number
  focus_density: number
  exploration_ratio: number
  revisit_ratio: number
  avg_entropy: number
}

// ── Oracle ───────────────────────────────────────────────────────────────────
export interface OracleSignal {
  kind: string
  strength: number
  description: string
}

// ── Civilization ──────────────────────────────────────────────────────────────
export interface Civilization {
  id: string
  member_count: number
  internal_density: number
  age_days: number
  territory_radius?: number
  color?: [number, number, number, number]
  dominant_node: string | null
  dominant_preview?: string
}

export interface CivilizationEvent {
  kind: string
  magnitude: number
  involved_civs: string[]
  description: string
}

// ── Resonance ─────────────────────────────────────────────────────────────────
export interface ResonancePair {
  node_a: string
  node_b: string
  similarity: number
  same_civilization: boolean
}

// ── Suggestion ────────────────────────────────────────────────────────────────
export interface Suggestion {
  node_id: string
  score: number
  content_preview: string
  reason: string
}

// ── Analytics ─────────────────────────────────────────────────────────────────
export interface HealthReport {
  score: number
  label: string
  avg_entropy: number
  density: number
  activity_rate: number
  decay_ratio: number
  component_count: number
  node_count: number
  edge_count: number
  bridge_count: number
  summary: string
}

export interface PageRankEntry {
  node_id: string
  score: number
  content_preview: string
}

export interface BridgeEdge {
  source_id: string
  target_id: string
  weight: number
  source_preview: string
  target_preview: string
}

// ── Dream ─────────────────────────────────────────────────────────────────────
export interface DreamProposal {
  id: string
  kind: string
  confidence: number
  description: string
}

// ── New types (missing endpoints) ────────────────────────────────────────────
export interface ContractData {
  node_id: string
  description: string
  strength: number
  age_days: number
}

export interface WeightData {
  ghost_nodes: number
  fossil_nodes: number
  void_nodes: number
  isolated_nodes: number
  pending_contracts: number
  total: number
  summary: string
}

export interface RitualData {
  name: string
  occurrence_count: number
  strength: number
  sequence_len: number
}

export interface LoreEntry {
  id: string
  title: string
  arc_type: string
  narrative: string
  significance: number
  timestamp: string
}

export interface SignatureData {
  geometry: string
  symmetry: string
  motion: string
  complexity: number
  vitality: number
  depth: number
  description: string
  evolution_count: number
}

export interface ShadowProject {
  label: string
  description: string
  age_days: number
  luminescence: number
  origin_kind: string
}

// ── Heatmap ──────────────────────────────────────────────────────────────────
export interface HeatmapEntry {
  node_id: string
  content_preview: string
  energy: number
  raw_score: number
}
export interface ObsessiveLoop {
  node_id: string
  content_preview: string
  revisit_count: number
  avg_session_seconds: number
  entropy: number
}
export interface NeglectedRegion {
  node_id: string
  content_preview: string
  connected_active_nodes: number
  days_since_access: number
}
export interface HeatmapData {
  entries: HeatmapEntry[]
  window_days: number
  obsessive_loops: ObsessiveLoop[]
  neglected_regions: NeglectedRegion[]
}

// ── Mirror ───────────────────────────────────────────────────────────────────
export interface PriorityGap {
  node_id: string
  content_preview: string
  stated_rank: number
  actual_rank: number
  gap: number
}
export interface BlindSpot {
  node_id: string
  content_preview: string
  last_accessed_days_ago: number
}
export interface Obsession {
  node_id: string
  content_preview: string
  focus_score: number
  entropy: number
  revisit_count: number
}
export interface Evolution {
  node_id: string
  label: string
  entropy_start: number
  entropy_now: number
  trajectory: string
  was_central: boolean
  state_changes: number
}
export interface MirrorData {
  priority_gaps: PriorityGap[]
  blind_spots: BlindSpot[]
  obsessions: Obsession[]
  peak_hour: number | null
  peak_weekday: number | null
  focus_period: string
  deep_work_event_count: number
  evolution: Evolution[]
}

// ── Weather ──────────────────────────────────────────────────────────────────
export interface WeatherData {
  state: string
  intensity: number
  color_r: number
  color_g: number
  color_b: number
  description: string
}

// ── Souls ────────────────────────────────────────────────────────────────────
export interface SoulData {
  project_id: string
  content_preview: string
  primary_color: [number, number, number, number]
  secondary_color: [number, number, number, number]
  particle_style: string
  glow_pattern: string
  activity_level: number
  maturity: number
  social_density: number
}

// ── Silence ──────────────────────────────────────────────────────────────────
export interface BridgeGap {
  node_a: string
  node_b: string
  preview_a: string
  preview_b: string
  similarity: number
  reason: string
}
export interface ImpliedConcept {
  suggested_content: string
  confidence: number
  implied_by_previews: string[]
}
export interface SilenceData {
  missing_bridges: BridgeGap[]
  implied_concepts: ImpliedConcept[]
}

// ── Focus Trail ──────────────────────────────────────────────────────────────
export interface TrailEvent {
  session_id: string
  node_id: string
  content_preview: string
  timestamp: string
  duration_seconds: number
  depth: string
  order?: number
}

export interface TectonicNode {
  node_id: string
  content_preview: string
  stress: number
  velocity: number
  entropy: number
}
export interface TectonicData {
  magnitude: number
  epicenter_id: string | null
  epicenter_preview: string
  affected_node_count: number
  stress_nodes: TectonicNode[]
  description: string
}

export interface SystemModeData {
  id: string
  label: string
  active: boolean
  intensity: number
  description: string
  source?: string
}

export interface AtmosphereData {
  id: string
  label: string
  region: string
  intensity: number
  color: string
  audio_signature: string
  visual_signature: string
}

export interface ForgeArtifactData {
  node_id: string
  label: string
  artifact_type: string
  parent_ids: string[]
  child_ids: string[]
  generation: number
  heat: number
}

export interface TerminalContextData {
  active_processes: number
  linked_processes: number
  dominant_process: string
  suggested_node_id: string | null
  suggested_node_preview: string
  lines: string[]
}

export interface ConstellationData {
  id: string
  label: string
  kind: string
  member_ids: string[]
  member_previews: string[]
  gravity: number
  emotional_weight: number
}

export interface VisionCoverageItem {
  concept: string
  area: string
  status: 'live' | 'partial' | 'stub' | string
  confidence: number
  backend_evidence: string[]
  web_evidence: string[]
  gap: string
}

export interface VisionCoverageData {
  generated_from: string
  summary: string
  completion_ratio: number
  items: VisionCoverageItem[]
}

export interface ObsidianTaskPreview {
  source_file: string
  line: number
  text: string
  completed: boolean
  tags: string[]
  date: string | null
  duplicate: boolean
}

export interface ObsidianPreviewData {
  vault_path: string
  files_scanned: number
  tasks: ObsidianTaskPreview[]
  tags: string[]
  warnings: string[]
}

export interface ObsidianImportResult {
  files_scanned: number
  tasks_created: number
  tasks_skipped: number
  tag_nodes_created: number
  day_nodes_created: number
  edges_created: number
  warnings: string[]
}

// ── Node colors ───────────────────────────────────────────────────────────────
export const NODE_COLORS: Record<string, string> = {
  idea:     '#40c8ff',
  memory:   '#c864ff',
  project:  '#3cdc78',
  person:   '#ffd23c',
  artifact: '#64a0ff',
  media:    '#3cb4c8',
  process:  '#78ffa0',
  world:    '#ffffff',
  ghost:    '#506080',
  fossil:   '#82723c',
  other:    '#94a3b8',
}

export const NODE_ICONS: Record<string, string> = {
  idea:     '◆',
  memory:   '◉',
  project:  '▣',
  person:   '◎',
  artifact: '◧',
  media:    '◐',
  process:  '◑',
  world:    '◯',
  ghost:    '◌',
  fossil:   '◫',
  other:    '◇',
}

export const SEASON_COLORS: Record<string, string> = {
  Spring: '#3cdc78',
  Summer: '#ffd23c',
  Autumn: '#e6783c',
  Winter: '#64a0ff',
}

// ── Archaeology ──────────────────────────────────────────────────────────────
export interface ArchaeologyTimelineEntry {
  index: number
  timestamp: string
  change_type: string
}
export interface ArchaeologyData {
  node_id: string
  snapshot_count: number
  cursor: number
  current_timestamp: string
  current_content: string
  current_entropy: number
  timeline: ArchaeologyTimelineEntry[]
}
export interface ResurrectedNode {
  node_id: string
  snapshot_index: number
  timestamp: string
  content: string
  entropy: number
  gravity: number
  is_ghost: boolean
  is_fossil: boolean
}

// ── Memory Reconstruction ────────────────────────────────────────────────────
export interface DayReconstructionData {
  date: string
  nodes_touched: number
  total_focus_seconds: number
  journal_entries_count: number
  dominant_node_id: string | null
  dominant_node_preview: string
  aura_state: string
  aura_intensity: number
  primary_color: [number, number, number, number]
  focus_events_count: number
}
export interface DayComparisonEntry {
  field: string
  day_a_value: string
  day_b_value: string
}
export interface DayComparisonData {
  day_a: string
  day_b: string
  entries: DayComparisonEntry[]
}

// ── Void Zones (enhanced) ────────────────────────────────────────────────────
export interface VoidZoneNode {
  node_id: string
  content_preview: string
  incubation_days: number
  is_mature: boolean
  resonance_readiness: number
  entropy: number
}

// ── Resonance Chambers ───────────────────────────────────────────────────────
export interface ResonanceChamber {
  id: string
  node_a: string
  preview_a: string
  node_b: string
  preview_b: string
  similarity: number
  state: string
}

// ── Calendar ─────────────────────────────────────────────────────────────────
export interface CalendarEvent {
  id: string
  title: string
  description: string
  category: string
  start_at: string
  end_at: string | null
  linked_node_id: string | null
  computed_gravity: number
  hours_until: number
  is_approaching: boolean
}

export interface DailyTask {
  id: string
  node_id: string
  title: string
  status: string
  date: string | null
  tags: string[]
  source: string
  calendar_event_id: string | null
  gravity: number
  entropy: number
}

export interface FocusWindow {
  start_hour: number
  end_hour: number
  score: number
  reason: string
}

// ── Membrane ─────────────────────────────────────────────────────────────────
export interface MembraneRule {
  id: string
  pattern: string
  direction: string
  allow: boolean
  description: string
}
export interface MembraneStatus {
  integrity_score: number
  rule_count: number
  blocked_count: number
  rules: MembraneRule[]
}

// ── Process ──────────────────────────────────────────────────────────────────
export interface ProcessData {
  pid: number
  name: string
  command: string
  cpu_usage: number
  memory_mb: number
  uptime_seconds: number
  status: string
  linked_node_id: string | null
}

// ── Crystallization ──────────────────────────────────────────────────────────
export interface CrystallizationResult {
  civ_id: string
  qualifies: boolean
  internal_density: number
  stability_score: number
  size: number
  crystal_id: string | null
}
