import { lazy, Suspense, useState, useEffect, useCallback, useMemo } from 'react'
import NodeDetail from './components/NodeDetail'
import AddNodeDialog from './components/AddNodeDialog'
import { ToastContainer } from './components/Toast'
import { api } from './api'
import type {
  SNode, SEdge, Status, JournalEntry, SeasonReport, OracleSignal, Civilization,
  SoulData, TrailEvent, TectonicData, CivilizationEvent,
  ActiveFocus,
} from './types'
import './styles/global.css'

const Graph3D = lazy(() => import('./components/Graph3D'))
const JournalView = lazy(() => import('./components/JournalView'))
const IntelligenceView = lazy(() => import('./components/IntelligenceView'))
const AnalyticsView = lazy(() => import('./components/AnalyticsView'))
const NodesView = lazy(() => import('./components/NodesView'))
const DreamView = lazy(() => import('./components/DreamView'))
const IdentityView = lazy(() => import('./components/IdentityView'))
const HeatmapView = lazy(() => import('./components/HeatmapView'))
const MirrorView = lazy(() => import('./components/MirrorView'))
const WeatherView = lazy(() => import('./components/WeatherView'))
const SilenceView = lazy(() => import('./components/SilenceView'))
const SoulsView = lazy(() => import('./components/SoulsView'))
const ForgeView = lazy(() => import('./components/ForgeView'))
const TrailView = lazy(() => import('./components/TrailView'))
const ArchaeologyView = lazy(() => import('./components/ArchaeologyView'))
const ReconstructionView = lazy(() => import('./components/ReconstructionView'))
const CalendarView = lazy(() => import('./components/CalendarView'))
const MembraneView = lazy(() => import('./components/MembraneView'))
const ProcessView = lazy(() => import('./components/ProcessView'))
const VoidResonanceView = lazy(() => import('./components/VoidResonanceView'))
const SystemModesView = lazy(() => import('./components/SystemModesView'))
const LivingTerminalView = lazy(() => import('./components/LivingTerminalView'))
const ConstellationsView = lazy(() => import('./components/ConstellationsView'))
const DeepFocusOverlay = lazy(() => import('./components/DeepFocusOverlay'))
const ForgeGenealogyView = lazy(() => import('./components/ForgeGenealogyView'))
const VisionCoverageView = lazy(() => import('./components/VisionCoverageView'))
const VaultView = lazy(() => import('./components/VaultView'))
const SettingsView = lazy(() => import('./components/SettingsView'))

type Space = 'today' | 'universe' | 'memory' | 'systems' | 'dream'
type Panel =
  | 'overview' | 'nodes' | 'journal' | 'forge' | 'intelligence'
  | 'heatmap' | 'mirror' | 'weather' | 'silence' | 'souls'
  | 'analytics' | 'trail' | 'archaeology' | 'reconstruction'
  | 'calendar' | 'membrane' | 'processes' | 'void' | 'dream' | 'identity'
  | 'modes' | 'terminal' | 'constellations' | 'genealogy' | 'vision' | 'vault' | 'settings'

const SPACES: { id: Space; label: string }[] = [
  { id: 'today', label: 'Command' },
  { id: 'universe', label: 'Universe' },
  { id: 'memory', label: 'Memory' },
  { id: 'systems', label: 'Systems' },
  { id: 'dream', label: 'Dream' },
]

const PANELS: Record<Space, { id: Panel; label: string }[]> = {
  today: [
    { id: 'overview', label: 'Pulse' },
    { id: 'journal', label: 'Journal' },
    { id: 'forge', label: 'Forge' },
    { id: 'genealogy', label: 'Lineage' },
    { id: 'terminal', label: 'Terminal' },
    { id: 'intelligence', label: 'Next' },
  ],
  universe: [
    { id: 'overview', label: 'Graph' },
    { id: 'nodes', label: 'Nodes' },
    { id: 'souls', label: 'Souls' },
    { id: 'constellations', label: 'Life' },
    { id: 'void', label: 'Void' },
    { id: 'silence', label: 'Silence' },
  ],
  memory: [
    { id: 'trail', label: 'Trail' },
    { id: 'heatmap', label: 'Heatmap' },
    { id: 'mirror', label: 'Mirror' },
    { id: 'archaeology', label: 'Archaeology' },
    { id: 'reconstruction', label: 'Days' },
  ],
  systems: [
    { id: 'settings', label: 'Settings' },
    { id: 'vault', label: 'Vaults' },
    { id: 'vision', label: 'Vision' },
    { id: 'modes', label: 'Modes' },
    { id: 'weather', label: 'Weather' },
    { id: 'calendar', label: 'Calendar' },
    { id: 'membrane', label: 'Membrane' },
    { id: 'processes', label: 'Processes' },
    { id: 'analytics', label: 'Health' },
  ],
  dream: [
    { id: 'dream', label: 'Dream' },
    { id: 'identity', label: 'Identity' },
  ],
}

function pct(v: number) {
  return `${Math.round(v * 100)}%`
}

function Metric({ label, value, tone = 'var(--t1)' }: { label: string; value: string | number; tone?: string }) {
  return (
    <div className="metric-card">
      <span>{label}</span>
      <strong style={{ color: tone }}>{value}</strong>
    </div>
  )
}

function PanelFallback() {
  return (
    <div className="panel-loading">
      <span />
      <strong>Loading surface</strong>
    </div>
  )
}

function GraphFallback() {
  return <div className="graph-loading" />
}

function InsightList({
  title,
  items,
  empty,
}: {
  title: string
  items: { label: string; value?: string; tone?: string }[]
  empty: string
}) {
  return (
    <section className="sn-panel">
      <div className="sn-panel-head">
        <span>{title}</span>
        <b>{items.length}</b>
      </div>
      <div className="sn-list">
        {items.length === 0 && <div className="empty-state">{empty}</div>}
        {items.slice(0, 7).map((item, index) => (
          <div className="sn-row" key={`${item.label}-${index}`}>
            <span>{item.label}</span>
            {item.value && <em style={{ color: item.tone }}>{item.value}</em>}
          </div>
        ))}
      </div>
    </section>
  )
}

export default function App() {
  const [nodes, setNodes] = useState<SNode[]>([])
  const [edges, setEdges] = useState<SEdge[]>([])
  const [status, setStatus] = useState<Status | null>(null)
  const [journal, setJournal] = useState<JournalEntry[]>([])
  const [season, setSeason] = useState<SeasonReport | null>(null)
  const [oracle, setOracle] = useState<OracleSignal[]>([])
  const [civs, setCivs] = useState<Civilization[]>([])
  const [civEvents, setCivEvents] = useState<CivilizationEvent[]>([])
  const [souls, setSouls] = useState<SoulData[]>([])
  const [trail, setTrail] = useState<TrailEvent[]>([])
  const [tectonics, setTectonics] = useState<TectonicData | null>(null)
  const [space, setSpace] = useState<Space>('today')
  const [panel, setPanel] = useState<Panel>('overview')
  const [selectedNode, setSelectedNode] = useState<SNode | null>(null)
  const [addingNode, setAddingNode] = useState(false)
  const [deepFocus, setDeepFocus] = useState(false)
  const [apiOk, setApiOk] = useState<boolean | null>(null)
  const [currentVault, setCurrentVault] = useState<string>('Default')
  const [auraClass, setAuraClass] = useState<string>('')
  const [activeFocus, setActiveFocus] = useState<ActiveFocus | null>(null)

  const refresh = useCallback(async () => {
    const results = await Promise.allSettled([
      api.nodes(), api.edges(), api.status(), api.journal(), api.season(), api.oracle(),
      api.civilizations(), api.souls(), api.trail(72), api.tectonics(), api.civilizationEvents(),
      api.vaults(), api.weather(), api.activeFocus(),
    ])
    setApiOk(results[0].status === 'fulfilled')
    if (results[11].status === 'fulfilled') setCurrentVault((results[11].value as any).current)
    if (results[0].status === 'fulfilled') setNodes(results[0].value)
    if (results[1].status === 'fulfilled') setEdges(results[1].value)
    if (results[2].status === 'fulfilled') setStatus(results[2].value)
    if (results[3].status === 'fulfilled') setJournal(results[3].value)
    if (results[4].status === 'fulfilled') setSeason(results[4].value)
    if (results[5].status === 'fulfilled') setOracle(results[5].value)
    if (results[6].status === 'fulfilled') setCivs(results[6].value)
    if (results[7].status === 'fulfilled') setSouls(results[7].value)
    if (results[8].status === 'fulfilled') setTrail(results[8].value)
    if (results[9].status === 'fulfilled') setTectonics(results[9].value)
    if (results[10].status === 'fulfilled') setCivEvents(results[10].value)
    if (results[12].status === 'fulfilled') {
      const w = results[12].value as any
      const cls = w.state === 'Energetic' ? 'aura-energetic'
        : w.state === 'Turbulent' ? 'aura-turbulent'
        : w.state === 'Fading'    ? 'aura-fading'
        : w.state === 'Reflective' ? 'aura-reflective'
        : 'aura-calm'
      setAuraClass(cls)
    }
    if (results[13].status === 'fulfilled') setActiveFocus(results[13].value as ActiveFocus)
  }, [])

  useEffect(() => {
    refresh()
    const id = setInterval(refresh, 9000)
    return () => clearInterval(id)
  }, [refresh])

  useEffect(() => {
    const load = () => api.activeFocus().then(setActiveFocus).catch(() => {})
    load()
    const id = setInterval(load, 1000)
    return () => clearInterval(id)
  }, [])

  useEffect(() => {
    if (!selectedNode) return
    const updated = nodes.find(n => n.id === selectedNode.id)
    if (updated) setSelectedNode(updated)
  }, [nodes, selectedNode])

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return
      if (e.key === 'n' || e.key === 'N') setAddingNode(true)
      if (e.key === 'r' || e.key === 'R') refresh()
      if (e.key === 'Escape') { setSelectedNode(null); setAddingNode(false) }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [refresh])

  const activePanels = PANELS[space]
  const currentPanel = activePanels.some(p => p.id === panel) ? panel : activePanels[0].id

  function chooseSpace(next: Space) {
    setSpace(next)
    setPanel(PANELS[next][0].id)
    if (next === 'universe') setSelectedNode(null)
  }

  const hotNodes = useMemo(
    () => [...nodes].sort((a, b) => (b.contagion_heat ?? b.velocity) - (a.contagion_heat ?? a.velocity)).slice(0, 7),
    [nodes],
  )
  const fadingNodes = useMemo(
    () => nodes.filter(n => n.entropy_state === 'Fading' || n.entropy_state === 'Crystallizing' || n.is_ghost).slice(0, 7),
    [nodes],
  )

  const graphVisible = space === 'universe' && currentPanel === 'overview'

  return (
    <div className={`sn-app ${auraClass}`}>
      <div className="sn-bg">
        <Suspense fallback={<GraphFallback />}>
          <Graph3D
            nodes={nodes}
            edges={edges}
            selectedId={selectedNode?.id ?? null}
            onSelect={node => {
              setSelectedNode(node)
              setSpace('universe')
              setPanel('overview')
            }}
            dim={!graphVisible}
            trail={trail}
            souls={souls}
          />
        </Suspense>
      </div>

      <header className="sn-top">
        <div className="sn-brand">
          <span className="sn-mark" />
          <div>
            <strong>SilentNode</strong>
            <small>
              {currentVault} · {season?.season ?? 'No season'} · {apiOk === false ? 'offline' : 'local'}
              {activeFocus?.active && ` · focus: ${activeFocus.node_nickname || activeFocus.node_preview || 'active'}`}
            </small>
          </div>
        </div>
        <nav className="sn-nav">
          {SPACES.map(item => (
            <button key={item.id} className={space === item.id ? 'active' : ''} onClick={() => chooseSpace(item.id)}>
              {item.label}
            </button>
          ))}
        </nav>
        <button className="sn-primary" onClick={() => setAddingNode(true)}>New thought</button>
      </header>

      <main className={graphVisible ? 'sn-main graph-active' : 'sn-main'}>
        <aside className="sn-rail">
          <Metric label="Nodes" value={status?.node_count ?? nodes.length} />
          <Metric label="Edges" value={status?.edge_count ?? edges.length} />
          <Metric label="Ghosts" value={status?.ghost_count ?? 0} tone="var(--t3)" />
          <Metric label="Tectonics" value={tectonics ? pct(tectonics.magnitude) : '0%'} tone={tectonics && tectonics.magnitude > 0.5 ? 'var(--amber)' : 'var(--sky)'} />

          <div className="sn-subnav">
            {activePanels.map(item => (
              <button key={item.id} className={currentPanel === item.id ? 'active' : ''} onClick={() => setPanel(item.id)}>
                {item.label}
              </button>
            ))}
          </div>
        </aside>

        <section className={graphVisible ? 'sn-stage graph-mode' : 'sn-stage'}>
          {graphVisible ? (
            <>
              <div className="sn-float left">
                <InsightList
                  title="Live Pulse"
                  empty="No movement yet."
                  items={[
                    ...(tectonics ? [{ label: tectonics.description, value: pct(tectonics.magnitude), tone: 'var(--amber)' }] : []),
                    ...hotNodes.map(n => ({ label: n.content, value: n.velocity_state ?? 'moving', tone: 'var(--teal)' })),
                  ]}
                />
              </div>
              {selectedNode && (
                <div className="sn-detail">
                  <NodeDetail node={selectedNode} onClose={() => setSelectedNode(null)} onRefresh={refresh} />
                </div>
              )}
            </>
          ) : currentPanel === 'overview' ? (
            <div className="sn-dashboard">
              <section className="hero-panel">
                <div>
                  <small>{season?.season ?? 'Silent'} season</small>
                  <h1>{tectonics?.epicenter_preview || 'Your cognitive workspace is ready'}</h1>
                  <p>{tectonics?.description || 'Add thoughts, focus events, and journal entries to make the graph start speaking back.'}</p>
                </div>
                <div className="hero-metrics">
                  <Metric label="Focus trail" value={trail.length} />
                  <Metric label="Project souls" value={souls.length} />
                  <Metric label="Civilizations" value={civs.length} />
                </div>
              </section>

              <div className="sn-grid">
                <InsightList
                  title="Oracle"
                  empty="No oracle signals."
                  items={oracle.map(o => ({ label: o.description, value: pct(o.strength), tone: 'var(--lavender-text)' }))}
                />
                <InsightList
                  title="Needs Attention"
                  empty="No fading nodes."
                  items={fadingNodes.map(n => ({ label: n.content, value: n.entropy_state, tone: n.is_ghost ? 'var(--t3)' : 'var(--amber)' }))}
                />
                <InsightList
                  title="Civilization Dynamics"
                  empty={civs.length ? 'No active trade or conflict events.' : 'No civilizations detected.'}
                  items={[
                    ...civEvents.map(e => ({ label: e.description, value: e.kind, tone: 'var(--sky)' })),
                    ...civs.slice(0, 5).map(c => ({ label: c.dominant_preview || c.id.slice(0, 8), value: `${c.member_count} nodes`, tone: 'var(--green)' })),
                  ]}
                />
              </div>
            </div>
          ) : (
            <div className="sn-panel-wrap">
              <Suspense fallback={<PanelFallback />}>
                {currentPanel === 'nodes' && <NodesView nodes={nodes} onRefresh={refresh} onAddThought={() => setAddingNode(true)} />}
                {currentPanel === 'journal' && <JournalView entries={journal} season={season} onRefresh={refresh} />}
                {currentPanel === 'forge' && <ForgeView nodes={nodes} onRefresh={refresh} />}
                {currentPanel === 'genealogy' && <ForgeGenealogyView />}
                {currentPanel === 'terminal' && <LivingTerminalView />}
                {currentPanel === 'intelligence' && (
                  <IntelligenceView oracle={oracle} civs={civs} nodes={nodes} season={season?.season} onSelectNode={id => {
                    const n = nodes.find(nd => nd.id === id)
                    if (n) { setSelectedNode(n); setSpace('universe'); setPanel('overview') }
                  }} />
                )}
                {currentPanel === 'heatmap' && <HeatmapView />}
                {currentPanel === 'mirror' && <MirrorView />}
                {currentPanel === 'weather' && <WeatherView />}
                {currentPanel === 'silence' && <SilenceView />}
                {currentPanel === 'souls' && <SoulsView />}
                {currentPanel === 'constellations' && <ConstellationsView />}
                {currentPanel === 'analytics' && <AnalyticsView nodes={nodes} />}
                {currentPanel === 'settings' && <SettingsView />}
                {currentPanel === 'vision' && <VisionCoverageView />}
                {currentPanel === 'vault' && <VaultView onSwitch={name => { setCurrentVault(name); refresh() }} />}
                {currentPanel === 'modes' && <SystemModesView onDeepFocus={() => setDeepFocus(true)} />}
                {currentPanel === 'trail' && <TrailView />}
                {currentPanel === 'archaeology' && <ArchaeologyView nodes={nodes} />}
                {currentPanel === 'reconstruction' && <ReconstructionView />}
                {currentPanel === 'calendar' && <CalendarView nodes={nodes} />}
                {currentPanel === 'membrane' && <MembraneView />}
                {currentPanel === 'processes' && <ProcessView nodes={nodes} />}
                {currentPanel === 'void' && <VoidResonanceView />}
                {currentPanel === 'dream' && <DreamView nodes={nodes} />}
                {currentPanel === 'identity' && <IdentityView />}
              </Suspense>
            </div>
          )}
        </section>
      </main>

      {addingNode && (
        <AddNodeDialog onClose={() => setAddingNode(false)} onAdded={() => {
          setAddingNode(false)
          refresh()
        }} />
      )}
      {deepFocus && (
        <Suspense fallback={null}>
          <DeepFocusOverlay season={season} tectonics={tectonics} onExit={() => setDeepFocus(false)} />
        </Suspense>
      )}
      <ToastContainer />
    </div>
  )
}
