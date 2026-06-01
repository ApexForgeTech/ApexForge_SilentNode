import { useState, useEffect, useCallback } from 'react'
import type { OracleSignal, ResonancePair, Suggestion, Civilization, SNode } from '../types'
import { NODE_COLORS } from '../types'
import { api } from '../api'
import { toast } from './Toast'

function Section({ title, accent, count, children }: {
  title: string; accent: string; count?: number; children: React.ReactNode
}) {
  return (
    <div className="glass" style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
      <div className="section-head">
        <span style={{ color: accent }}>{accent === 'var(--cyan)' ? '◈' : '●'}</span>
        {title}
        {count !== undefined && (
          <span style={{ marginLeft: 4, color: 'var(--text-muted)' }}>({count})</span>
        )}
      </div>
      <div className="scroll-y" style={{ flex: 1 }}>{children}</div>
    </div>
  )
}

function Bar({ val, color = 'var(--cyan)' }: { val: number; color?: string }) {
  return (
    <div className="prog-track" style={{ width: 60 }}>
      <div className="prog-fill" style={{ width: `${(val*100).toFixed(0)}%`, background: color }} />
    </div>
  )
}

interface Props {
  oracle: OracleSignal[]
  civs: Civilization[]
  nodes: SNode[]
  onSelectNode: (id: string) => void
  season?: string
}

// Derive a cognitive synthesis narrative from available signals
function synthesize(oracle: OracleSignal[], nodes: SNode[], resonances: ResonancePair[], civs: Civilization[], season?: string): string[] {
  const insights: string[] = []

  const season_ctx = season === 'Spring' ? 'high creative emergence'
    : season === 'Summer' ? 'peak output phase'
    : season === 'Autumn' ? 'consolidation and reflection'
    : season === 'Winter' ? 'incubation and silence'
    : 'transitional state'

  insights.push(`Cognitive season: ${season ?? 'unknown'} — ${season_ctx}.`)

  const ghostCount = nodes.filter(n => n.is_ghost).length
  if (ghostCount > 0) insights.push(`${ghostCount} ghost node${ghostCount > 1 ? 's' : ''} detected — abandoned ideas awaiting resurrection.`)

  const highEntropy = nodes.filter(n => n.entropy > 0.7).length
  if (highEntropy > 2) insights.push(`${highEntropy} nodes in high entropy — attention fading in multiple regions.`)

  const topOracle = oracle.find(s => s.strength > 0.7)
  if (topOracle) insights.push(`Strong oracle signal: ${topOracle.description}`)

  if (resonances.length > 3) insights.push(`${resonances.length} unconnected resonances detected — your knowledge clusters are ready to bridge.`)

  const topCiv = civs.reduce<Civilization | null>((best, c) => (!best || c.member_count > best.member_count) ? c : best, null)
  if (topCiv && topCiv.member_count > 4) insights.push(`Dominant civilization with ${topCiv.member_count} nodes — crystallization may be near.`)

  if (insights.length === 0) insights.push('Universe is quiet. No significant cognitive patterns detected.')

  return insights
}

export default function IntelligenceView({ oracle, civs, nodes, onSelectNode, season }: Props) {
  const [resonances,   setResonances]   = useState<ResonancePair[]>([])
  const [suggestions,  setSuggestions]  = useState<Suggestion[]>([])
  const [loading,      setLoading]      = useState(true)

  const nodeMap = new Map(nodes.map(n => [n.id, n]))

  useEffect(() => {
    setLoading(true)
    Promise.allSettled([
      api.resonances(),
      api.suggestions(16),
    ]).then(([r, s]) => {
      if (r.status === 'fulfilled') setResonances(r.value)
      if (s.status === 'fulfilled') setSuggestions(s.value)
      setLoading(false)
    })
  }, [])

  const focusNode = async (id: string) => {
    try {
      await api.recordFocus(id, 30, 'Read')
      toast('Focus recorded')
      onSelectNode(id)
    } catch (e) { toast(String(e), 'error') }
  }

  const dismissResonance = useCallback((idxA: string, idxB: string) => {
    setResonances(prev => prev.filter(r => !(r.node_a === idxA && r.node_b === idxB)))
    toast('Resonance dismissed')
  }, [])

  const connectResonance = useCallback(async (r: ResonancePair) => {
    try {
      await api.connect(r.node_a, r.node_b, Math.round(r.similarity * 100) / 100)
      setResonances(prev => prev.filter(x => !(x.node_a === r.node_a && x.node_b === r.node_b)))
      toast('Nodes connected via resonance')
    } catch (e) { toast(String(e), 'error') }
  }, [])

  const noteResonance = useCallback((r: ResonancePair) => {
    const a = nodeMap.get(r.node_a)
    const b = nodeMap.get(r.node_b)
    toast(`Noted: "${a?.content ?? r.node_a}" ↔ "${b?.content ?? r.node_b}"`)
    setResonances(prev => prev.filter(x => !(x.node_a === r.node_a && x.node_b === r.node_b)))
  }, [nodeMap])

  const synthesis = synthesize(oracle, nodes, resonances, civs, season)

  return (
    <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gridTemplateRows: 'auto 1fr 1fr', gap: 10, height: '100%' }}>

      {/* Cognitive Synthesis */}
      <div className="glass" style={{ gridColumn: '1 / -1', padding: '10px 14px' }}>
        <div className="section-head" style={{ marginBottom: 8 }}>
          <span style={{ color: 'var(--cyan)' }}>◎</span> Cognitive Synthesis
        </div>
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: '6px 16px' }}>
          {synthesis.map((insight, i) => (
            <span key={i} style={{
              fontSize: 11, color: i === 0 ? 'var(--text-secondary)' : 'var(--text-muted)',
              borderLeft: `2px solid ${i === 0 ? 'var(--cyan)' : 'rgba(64,200,255,0.2)'}`,
              paddingLeft: 8, lineHeight: 1.5,
            }}>
              {insight}
            </span>
          ))}
        </div>
      </div>

      {/* Oracle signals */}
      <div className="glass" style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
        <div className="section-head">
          <span style={{ color: 'var(--amber)' }}>⚡</span> Oracle Signals
          <span style={{ marginLeft: 4, color: 'var(--text-muted)' }}>({oracle.length})</span>
        </div>
        <div className="scroll-y" style={{ flex: 1 }}>
          {oracle.length === 0 && (
            <div style={{ padding: '16px', color: 'var(--text-muted)', fontSize: 11 }}>
              Silence — no oracle signals
            </div>
          )}
          {oracle.map((sig, i) => {
            const col = sig.strength > 0.7 ? 'var(--crimson)' : sig.strength > 0.4 ? 'var(--amber)' : 'var(--emerald)'
            return (
              <div key={i} style={{
                padding: '8px 12px', borderBottom: '1px solid rgba(40,70,120,0.25)',
                display: 'flex', gap: 10, alignItems: 'flex-start',
              }}>
                <div style={{ paddingTop: 3 }}>
                  <Bar val={sig.strength} color={col} />
                  <div style={{ color: col, fontSize: 9, textAlign: 'right', marginTop: 1 }}>
                    {sig.strength.toFixed(2)}
                  </div>
                </div>
                <div>
                  <div style={{ color: col, fontSize: 9, fontFamily: 'var(--font-head)', letterSpacing: '0.08em', marginBottom: 2 }}>
                    {sig.kind.replace(/([A-Z])/g, ' $1').trim()}
                  </div>
                  <div style={{ color: 'var(--text-secondary)', fontSize: 11, lineHeight: 1.4 }}>
                    {sig.description}
                  </div>
                </div>
              </div>
            )
          })}
        </div>
      </div>

      {/* Suggestions */}
      <div className="glass" style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
        <div className="section-head">
          <span style={{ color: 'var(--emerald)' }}>→</span> Focus Suggestions
        </div>
        <div className="scroll-y" style={{ flex: 1 }}>
          {loading && <div style={{ padding: 12, color: 'var(--text-muted)', fontSize: 10 }}>Loading…</div>}
          {suggestions.map((s, i) => {
            const n = nodeMap.get(s.node_id)
            const col = NODE_COLORS[n?.node_type ?? ''] ?? 'var(--cyan)'
            return (
              <div key={s.node_id} style={{
                padding: '8px 12px', borderBottom: '1px solid rgba(40,70,120,0.25)',
                display: 'flex', gap: 8, alignItems: 'center',
              }}>
                <span style={{ color: 'var(--text-muted)', fontSize: 10, width: 18 }}>{i + 1}.</span>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ color: col, fontSize: 11, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                    {s.content_preview}
                  </div>
                  <div style={{ color: 'var(--text-muted)', fontSize: 9, marginTop: 2 }}>{s.reason}</div>
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 6, flexShrink: 0 }}>
                  <Bar val={s.score} color="var(--cyan)" />
                  <button className="btn-xs" onClick={() => focusNode(s.node_id)}>Focus</button>
                </div>
              </div>
            )
          })}
        </div>
      </div>

      {/* Resonances */}
      <div className="glass" style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
        <div className="section-head">
          <span style={{ color: 'var(--violet)' }}>↔</span> Resonances
          <span style={{ marginLeft: 4, color: 'var(--text-muted)' }}>({resonances.length})</span>
        </div>
        <div className="scroll-y" style={{ flex: 1 }}>
          {loading && <div style={{ padding: 12, color: 'var(--text-muted)', fontSize: 10 }}>Loading…</div>}
          {resonances.slice(0, 20).map((r, i) => {
            const a = nodeMap.get(r.node_a)
            const b = nodeMap.get(r.node_b)
            const col = r.same_civilization ? 'var(--amber)' : 'var(--violet)'
            return (
              <div key={i} style={{
                padding: '7px 12px', borderBottom: '1px solid rgba(40,70,120,0.25)',
              }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 3 }}>
                  <Bar val={r.similarity} color={col} />
                  <span style={{ color: col, fontSize: 10 }}>{r.similarity.toFixed(2)}</span>
                  {r.same_civilization && <span className="badge badge-amber" style={{ fontSize: 8 }}>same civ</span>}
                </div>
                <div style={{ display: 'flex', gap: 6, alignItems: 'center', fontSize: 11, marginBottom: 5 }}>
                  <span style={{ color: NODE_COLORS[a?.node_type ?? ''] ?? 'var(--text-secondary)', overflow:'hidden', textOverflow:'ellipsis', whiteSpace:'nowrap', maxWidth: 95 }}>
                    {a?.content ?? r.node_a.slice(0,10)}
                  </span>
                  <span style={{ color: 'var(--text-muted)' }}>↔</span>
                  <span style={{ color: NODE_COLORS[b?.node_type ?? ''] ?? 'var(--text-secondary)', overflow:'hidden', textOverflow:'ellipsis', whiteSpace:'nowrap', maxWidth: 95 }}>
                    {b?.content ?? r.node_b.slice(0,10)}
                  </span>
                </div>
                <div style={{ display: 'flex', gap: 4 }}>
                  <button className="btn-xs btn-emerald" style={{ fontSize: 9, padding: '2px 6px' }} onClick={() => connectResonance(r)}>Connect</button>
                  <button className="btn-xs" style={{ fontSize: 9, padding: '2px 6px', color: 'var(--violet)', border: '1px solid rgba(139,92,246,0.3)' }} onClick={() => noteResonance(r)}>Note</button>
                  <button className="btn-xs" style={{ fontSize: 9, padding: '2px 6px', color: 'var(--text-muted)', border: '1px solid rgba(100,116,139,0.2)' }} onClick={() => dismissResonance(r.node_a, r.node_b)}>Dismiss</button>
                </div>
              </div>
            )
          })}
        </div>
      </div>

      {/* Civilizations */}
      <div className="glass" style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
        <div className="section-head">
          <span style={{ color: 'var(--cyan)' }}>⬡</span> Civilizations
          <span style={{ marginLeft: 4, color: 'var(--text-muted)' }}>({civs.length})</span>
        </div>
        <div className="scroll-y" style={{ flex: 1 }}>
          {civs.length === 0 && (
            <div style={{ padding: 16, color: 'var(--text-muted)', fontSize: 11 }}>
              No civilizations detected yet — need denser clusters
            </div>
          )}
          {civs.map((civ, i) => {
            const dom = nodeMap.get(civ.dominant_node ?? '')
            return (
              <div key={civ.id} style={{
                padding: '10px 12px', borderBottom: '1px solid rgba(40,70,120,0.25)',
              }}>
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 6 }}>
                  <div style={{ fontFamily: 'var(--font-head)', fontSize: 9, color: 'var(--cyan)', letterSpacing: '0.1em' }}>
                    CIV {i + 1}
                  </div>
                  <div style={{ display: 'flex', gap: 6 }}>
                    <span className="badge badge-cyan">{civ.member_count} nodes</span>
                    <span className="badge badge-emerald">{civ.age_days.toFixed(0)}d</span>
                  </div>
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 4 }}>
                  <span style={{ color: 'var(--text-muted)', fontSize: 10, width: 60 }}>density</span>
                  <div className="prog-track" style={{ flex: 1 }}>
                    <div className="prog-fill" style={{ width: `${(civ.internal_density*100).toFixed(0)}%`, background: 'var(--cyan)' }} />
                  </div>
                  <span style={{ color: 'var(--text-secondary)', fontSize: 10 }}>{civ.internal_density.toFixed(2)}</span>
                </div>
                {dom && (
                  <div style={{ fontSize: 11, color: 'var(--text-secondary)' }}>
                    ◆ {dom.content.slice(0, 36)}{dom.content.length > 36 ? '…' : ''}
                  </div>
                )}
              </div>
            )
          })}
        </div>
      </div>
    </div>
  )
}
