import type {
  SNode, SEdge, Status, JournalEntry, SeasonReport, OracleSignal,
  Civilization, ResonancePair, Suggestion, HealthReport, PageRankEntry,
  BridgeEdge, DreamProposal,
  ContractData, WeightData, RitualData, LoreEntry,
  SignatureData, ShadowProject,
  HeatmapData, MirrorData, WeatherData, SoulData, SilenceData, TrailEvent,
  ArchaeologyData, ResurrectedNode,
  DayReconstructionData, DayComparisonData,
  VoidZoneNode, ResonanceChamber,
  CalendarEvent, FocusWindow,
  DailyTask,
  MembraneStatus, MembraneRule,
  ProcessData, CrystallizationResult,
  CivilizationEvent, TectonicData,
  SystemModeData, AtmosphereData, ForgeArtifactData, TerminalContextData,
  ConstellationData, VisionCoverageData, ObsidianPreviewData, ObsidianImportResult,
  NotificationSettings, NotificationSettingsUpdate,
} from './types'

const BASE = '/api'

async function request<T>(path: string, opts?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    headers: { 'Content-Type': 'application/json' },
    ...opts,
  })
  if (!res.ok) {
    const msg = await res.text().catch(() => res.statusText)
    throw new Error(`${res.status}: ${msg}`)
  }
  if (res.status === 204) return undefined as T
  return res.json()
}

export const api = {
  // ── Status ─────────────────────────────────────────────────────────────
  status: (): Promise<Status> =>
    request('/status'),

  // ── Nodes ──────────────────────────────────────────────────────────────
  nodes: (): Promise<SNode[]> =>
    request('/nodes'),

  node: (id: string): Promise<SNode> =>
    request(`/nodes/${id}`),

  createNode: (
    content: string,
    node_type = 'idea',
    nickname?: string,
    custom_type?: string,
    custom_color?: string,
    schedule?: unknown,
  ): Promise<SNode> =>
    request('/nodes', { method: 'POST', body: JSON.stringify({ content, node_type, nickname, custom_type, custom_color, schedule }) }),

  deleteNode: (id: string): Promise<void> =>
    request(`/nodes/${id}`, { method: 'DELETE' }),

  updateNode: (id: string, data: {
    content?: string; node_type?: string; nickname?: string; aura_color?: string;
    custom_type?: string; custom_color?: string;
    schedule?: unknown;
  }): Promise<SNode> =>
    request(`/nodes/${id}`, { method: 'PUT', body: JSON.stringify(data) }),

  related: (id: string): Promise<SNode[]> =>
    request(`/nodes/${id}/related`),

  // ── Edges ──────────────────────────────────────────────────────────────
  edges: (): Promise<SEdge[]> =>
    request('/edges'),

  // ── Cognitive interaction ───────────────────────────────────────────────
  addThought: (text: string, nickname?: string): Promise<{ node_id: string }> =>
    request('/thought', { method: 'POST', body: JSON.stringify({ text, nickname }) }),

  recordFocus: (node_id: string, seconds: number, depth = 'DeepWork'): Promise<void> =>
    request('/focus', { method: 'POST', body: JSON.stringify({ node_id, seconds, depth }) }),

  // ── Journal ────────────────────────────────────────────────────────────
  journal: (): Promise<JournalEntry[]> =>
    request('/journal'),

  addJournal: (text: string, season?: string): Promise<JournalEntry> =>
    request('/journal', { method: 'POST', body: JSON.stringify({ text, season }) }),

  tasks: (date?: string): Promise<DailyTask[]> =>
    request(`/tasks${date ? `?date=${encodeURIComponent(date)}` : ''}`),

  addTask: (data: {
    title: string; date?: string; tags?: string[]; due_at?: string; notes?: string;
  }): Promise<DailyTask> =>
    request('/tasks', { method: 'POST', body: JSON.stringify(data) }),

  completeTask: (id: string, done: boolean): Promise<void> =>
    request(`/tasks/${id}/complete`, { method: 'POST', body: JSON.stringify({ done }) }),

  // ── Intelligence ───────────────────────────────────────────────────────
  season: (): Promise<SeasonReport> =>
    request('/season'),

  oracle: (): Promise<OracleSignal[]> =>
    request('/oracle'),

  civilizations: (): Promise<Civilization[]> =>
    request('/civilizations'),

  civilizationEvents: (): Promise<CivilizationEvent[]> =>
    request('/civilization-events'),

  resonances: (threshold?: number): Promise<ResonancePair[]> =>
    request(`/resonances${threshold !== undefined ? `?threshold=${threshold}` : ''}`),

  suggestions: (limit = 12): Promise<Suggestion[]> =>
    request(`/suggestions?limit=${limit}`),

  // ── Analytics ──────────────────────────────────────────────────────────
  health: (): Promise<HealthReport> =>
    request('/analytics'),

  pagerank: (limit = 20): Promise<PageRankEntry[]> =>
    request(`/analytics/pagerank?limit=${limit}`),

  bridges: (): Promise<BridgeEdge[]> =>
    request('/analytics/bridges'),

  // ── Dream ──────────────────────────────────────────────────────────────
  dreamProposals: (): Promise<DreamProposal[]> =>
    request('/dream/proposals'),

  synthesize: (query: string): Promise<{ narrative: string; related_nodes: string[] }> =>
    request('/synthesize', { method: 'POST', body: JSON.stringify({ query }) }),

  knowledgeGaps: (topic?: string): Promise<{ gaps: string[] }> =>
    request(`/synthesis/gaps${topic ? `?topic=${encodeURIComponent(topic)}` : ''}`),

  // ── Node actions (new) ─────────────────────────────────────────────────
  voidToggle:  (id: string): Promise<{ voided: boolean }> =>
    request(`/nodes/${id}/void`, { method: 'POST' }),

  fossilize: (id: string): Promise<void> =>
    request(`/nodes/${id}/fossilize`, { method: 'POST' }),

  excavate: (id: string): Promise<void> =>
    request(`/nodes/${id}/excavate`, { method: 'POST' }),

  revive: (id: string): Promise<void> =>
    request(`/nodes/${id}/revive`, { method: 'POST' }),

  connect: (source_id: string, target_id: string, weight = 1.0): Promise<void> =>
    request('/connect', { method: 'POST', body: JSON.stringify({ source_id, target_id, weight }) }),

  deleteNodes: (ids: string[]): Promise<{ deleted: number }> =>
    request('/nodes/bulk-delete', { method: 'DELETE', body: JSON.stringify(ids) }),

  // ── Missing intelligence ─────────────────────────────────────────────
  contracts: (): Promise<ContractData[]> =>
    request('/contracts'),

  weight: (): Promise<WeightData> =>
    request('/weight'),

  rituals: (): Promise<RitualData[]> =>
    request('/rituals'),

  lore: (): Promise<LoreEntry[]> =>
    request('/lore'),

  signature: (): Promise<SignatureData> =>
    request('/signature'),

  shadowProjects: (): Promise<ShadowProject[]> =>
    request('/shadow-projects'),

  // ── New endpoints ────────────────────────────────────────────────────────
  heatmap: (days = 30): Promise<HeatmapData> =>
    request(`/heatmap?days=${days}`),

  mirror: (days = 30): Promise<MirrorData> =>
    request(`/mirror?days=${days}`),

  weather: (): Promise<WeatherData> =>
    request('/weather'),

  souls: (): Promise<SoulData[]> =>
    request('/souls'),

  silence: (): Promise<SilenceData> =>
    request('/silence'),

  trail: (hours = 48): Promise<TrailEvent[]> =>
    request(`/trail?hours=${hours}`),

  tectonics: (): Promise<TectonicData> =>
    request('/tectonics'),

  modes: (): Promise<SystemModeData[]> =>
    request('/modes'),

  setMode: (mode: string | null): Promise<{ mode: string | null; source: string }> =>
    request('/modes', { method: 'POST', body: JSON.stringify({ mode }) }),

  atmospheres: (): Promise<AtmosphereData[]> =>
    request('/atmospheres'),

  forgeGenealogy: (): Promise<ForgeArtifactData[]> =>
    request('/forge/genealogy'),

  terminalContext: (): Promise<TerminalContextData> =>
    request('/terminal/context'),

  constellations: (): Promise<ConstellationData[]> =>
    request('/constellations'),

  visionCoverage: (): Promise<VisionCoverageData> =>
    request('/vision/coverage'),

  // ── Archaeology ─────────────────────────────────────────────────────────
  archaeology: (nodeId: string): Promise<ArchaeologyData> =>
    request(`/archaeology/${nodeId}`),

  archaeologyResurrect: (nodeId: string, index: number): Promise<ResurrectedNode> =>
    request(`/archaeology/${nodeId}/resurrect/${index}`, { method: 'POST' }),

  // ── Memory Reconstruction ────────────────────────────────────────────────
  reconstructDay: (date: string): Promise<DayReconstructionData> =>
    request(`/temporal/day/${date}`),

  compareDays: (from: string, to: string): Promise<DayComparisonData> =>
    request(`/temporal/compare?from=${from}&to=${to}`),

  takeSnapshot: (): Promise<{ ok: boolean; total_snapshots: number }> =>
    request('/temporal/snapshot', { method: 'POST' }),

  // ── Void Zones ───────────────────────────────────────────────────────────
  voidZones: (): Promise<VoidZoneNode[]> =>
    request('/void-zones'),

  // ── Contract actions ─────────────────────────────────────────────────────
  fulfillContract: (nodeId: string): Promise<void> =>
    request(`/contracts/${nodeId}/fulfill`, { method: 'POST' }),

  releaseContract: (nodeId: string): Promise<void> =>
    request(`/contracts/${nodeId}/release`, { method: 'POST' }),

  // ── Crystallization ──────────────────────────────────────────────────────
  crystallize: (civId: string): Promise<CrystallizationResult> =>
    request(`/crystallize/${civId}`, { method: 'POST' }),

  // ── Resonance Chambers ───────────────────────────────────────────────────
  resonanceChambers: (threshold?: number): Promise<ResonanceChamber[]> =>
    request(`/resonance-chambers${threshold !== undefined ? `?threshold=${threshold}` : ''}`),

  // ── Shadow actions ───────────────────────────────────────────────────────
  illuminateShadow: (nodeId: string): Promise<void> =>
    request(`/shadows/${nodeId}/illuminate`, { method: 'POST' }),

  nameShadow: (nodeId: string, name: string): Promise<void> =>
    request(`/shadows/${nodeId}/name`, { method: 'POST', body: JSON.stringify({ name }) }),

  releaseShadow: (nodeId: string): Promise<void> =>
    request(`/shadows/${nodeId}/release`, { method: 'POST' }),

  // ── Calendar ─────────────────────────────────────────────────────────────
  calendarEvents: (): Promise<CalendarEvent[]> =>
    request('/calendar'),

  addCalendarEvent: (data: {
    title: string; description?: string; category?: string;
    start_at: string; end_at?: string; linked_node_id?: string;
  }): Promise<{ id: string }> =>
    request('/calendar', { method: 'POST', body: JSON.stringify(data) }),

  deleteCalendarEvent: (id: string): Promise<void> =>
    request(`/calendar/${id}`, { method: 'DELETE' }),

  calendarFocusWindows: (): Promise<FocusWindow[]> =>
    request('/calendar/focus-windows'),

  // ── Membrane ─────────────────────────────────────────────────────────────
  membrane: (): Promise<MembraneStatus> =>
    request('/membrane'),

  addMembraneRule: (data: {
    pattern: string; direction?: string; allow?: boolean; description?: string;
  }): Promise<{ id: string }> =>
    request('/membrane/rules', { method: 'POST', body: JSON.stringify(data) }),

  deleteMembraneRule: (id: string): Promise<void> =>
    request(`/membrane/rules/${id}`, { method: 'DELETE' }),

  // ── Processes ────────────────────────────────────────────────────────────
  processes: (): Promise<ProcessData[]> =>
    request('/processes'),

  linkProcess: (pid: number, nodeId: string): Promise<void> =>
    request(`/processes/${pid}/link`, { method: 'POST', body: JSON.stringify({ node_id: nodeId }) }),

  // ── Export ─────────────────────────────────────────────────────────────
  exportDot: (): Promise<string> =>
    fetch(`${BASE}/export/dot`).then(r => r.text()),

  exportCsv: (): Promise<string> =>
    fetch(`${BASE}/export/csv`).then(r => r.text()),

  exportMarkdown: (): Promise<string> =>
    fetch(`${BASE}/export/markdown`).then(r => r.text()),

  // ── Vaults ─────────────────────────────────────────────────────────────
  vaults: (): Promise<{ vaults: { name: string; path: string }[]; current: string }> =>
    request('/vaults'),

  createVault: (name: string, path?: string): Promise<{ name: string; path: string }> =>
    request('/vaults', { method: 'POST', body: JSON.stringify({ name, path }) }),

  switchVault: (name: string): Promise<void> =>
    request('/vaults/switch', { method: 'POST', body: JSON.stringify({ name }) }),

  deleteVault: (name: string): Promise<void> =>
    request(`/vaults/${encodeURIComponent(name)}`, { method: 'DELETE' }),

  obsidianPreview: (path: string, max_files = 500): Promise<ObsidianPreviewData> =>
    request('/obsidian/preview', { method: 'POST', body: JSON.stringify({ path, max_files }) }),

  obsidianImport: (path: string, include_completed = true, max_files = 500): Promise<ObsidianImportResult> =>
    request('/obsidian/import', { method: 'POST', body: JSON.stringify({ path, include_completed, max_files }) }),

  // ── Settings ─────────────────────────────────────────────────────────────
  notificationSettings: (): Promise<NotificationSettings> =>
    request('/settings/notifications'),

  saveNotificationSettings: (data: NotificationSettingsUpdate): Promise<NotificationSettings> =>
    request('/settings/notifications', { method: 'PUT', body: JSON.stringify(data) }),

  testTelegramNotification: (message?: string): Promise<{ ok: boolean; channel: string }> =>
    request('/settings/notifications/test', { method: 'POST', body: JSON.stringify({ message }) }),

  // ── ML endpoints ──────────────────────────────────────────────────────────
  mlStatus: (): Promise<any> =>
    request('/ml/status'),

  mlClassify: (content: string, nickname?: string): Promise<{
    type: string
    confidence: number
    all_probs: Record<string, number>
    alternatives?: { type: string; confidence: number }[]
    uncertain?: boolean
    method?: string
  }> =>
    request('/ml/classify', { method: 'POST', body: JSON.stringify({ content, nickname }) }),

  mlFeedback: (data: {
    node_id?: string
    content: string
    nickname?: string
    predicted_type?: string
    selected_type: string
    confidence?: number
    source?: string
  }): Promise<{ ok: boolean }> =>
    request('/ml/feedback', { method: 'POST', body: JSON.stringify(data) }),

  mlGhostRisk: (): Promise<any[]> =>
    request('/ml/ghost-risk'),

  mlNextFocus: (nodeId: string): Promise<any[]> =>
    request(`/ml/next-focus/${nodeId}`),

  mlClusters: (): Promise<any[]> =>
    request('/ml/clusters'),

  mlTrain: (): Promise<any> =>
    request('/ml/train', { method: 'POST' }),
}
