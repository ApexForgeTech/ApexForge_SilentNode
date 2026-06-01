import { useState, useEffect } from 'react'
import type { SignatureData, LoreEntry, ShadowProject, WeightData, RitualData, ContractData, Civilization } from '../types'
import { api } from '../api'
import { toast } from './Toast'

const ARC_COLORS: Record<string, string> = {
  origin: 'var(--lavender-text)', conflict: 'var(--red)', resolution: 'var(--green)',
  revelation: 'var(--amber)', transformation: 'var(--sky)', legacy: 'var(--teal)',
  tectonic: '#fb923c',
}
const ARC_ICONS: Record<string, string> = {
  origin: '◉', conflict: '⚔', resolution: '◇', revelation: '✦',
  transformation: '⟳', legacy: '◆', tectonic: '⚡',
}

function Bar({ val, color = 'var(--lavender-text)' }: { val: number; color?: string }) {
  return (
    <div className="bar fill">
      <div className="bar-fill" style={{ width: `${(val * 100).toFixed(0)}%`, background: color }} />
    </div>
  )
}

export default function IdentityView() {
  const [sig,       setSig]       = useState<SignatureData | null>(null)
  const [lore,      setLore]      = useState<LoreEntry[]>([])
  const [shadows,   setShadows]   = useState<ShadowProject[]>([])
  const [weight,    setWeight]    = useState<WeightData | null>(null)
  const [rituals,   setRituals]   = useState<RitualData[]>([])
  const [contracts, setContracts] = useState<ContractData[]>([])
  const [civs,      setCivs]      = useState<Civilization[]>([])
  const [loading,   setLoading]   = useState(true)

  async function load() {
    setLoading(true)
    const results = await Promise.allSettled([
      api.signature(), api.lore(), api.shadowProjects(),
      api.weight(), api.rituals(), api.contracts(), api.civilizations(),
    ])
    const [s, l, sh, w, r, c, cv] = results
    if (s.status  === 'fulfilled') setSig(s.value)
    if (l.status  === 'fulfilled') setLore(l.value)
    if (sh.status === 'fulfilled') setShadows(sh.value)
    if (w.status  === 'fulfilled') setWeight(w.value)
    if (r.status  === 'fulfilled') setRituals(r.value)
    if (c.status  === 'fulfilled') setContracts(c.value)
    if (cv.status === 'fulfilled') setCivs(cv.value)
    setLoading(false)
  }

  useEffect(() => { load() }, [])

  if (loading) {
    return <div style={{ padding: 24, color: 'var(--t4)' }}>Loading identity data…</div>
  }

  return (
    <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gridTemplateRows: '1fr 1fr', gap: 10, height: '100%' }}>

      {/* Living Signature */}
      <div className="panel" style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
        <div className="sec-head"><span style={{ color: 'var(--lavender-text)' }}>◈</span> Living Signature</div>
        <div className="scroll" style={{ flex: 1, padding: '12px 14px' }}>
          {!sig ? (
            <div style={{ color: 'var(--t4)', fontSize: 12 }}>No signature data</div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
              <div>
                <div style={{ fontSize: 11, color: 'var(--t3)', marginBottom: 4 }}>Form</div>
                <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
                  <span className="badge badge-lv">{sig.geometry}</span>
                  <span className="badge badge-mt">{sig.symmetry}</span>
                  <span className="badge badge-sky">{sig.motion}</span>
                  <span className="badge badge-mt">v{sig.evolution_count}</span>
                </div>
              </div>
              {[
                { l: 'Complexity', v: sig.complexity, c: 'var(--lavender-text)' },
                { l: 'Vitality',   v: sig.vitality,   c: 'var(--green)' },
                { l: 'Depth',      v: sig.depth,       c: 'var(--amber)' },
              ].map(({ l, v, c }) => (
                <div key={l} style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <span style={{ color: 'var(--t4)', fontSize: 10, width: 70, flexShrink: 0 }}>{l}</span>
                  <Bar val={v} color={c} />
                  <span style={{ color: c, fontSize: 10, width: 32, textAlign: 'right', fontFamily: 'var(--font-mono)', flexShrink: 0 }}>
                    {(v * 100).toFixed(0)}%
                  </span>
                </div>
              ))}
              <div style={{ color: 'var(--t2)', fontSize: 11, lineHeight: 1.6, paddingTop: 4, borderTop: '1px solid var(--line)' }}>
                {sig.description}
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Cognitive Weight */}
      <div className="panel" style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
        <div className="sec-head"><span style={{ color: 'var(--amber)' }}>◐</span> Cognitive Weight</div>
        <div className="scroll" style={{ flex: 1, padding: '12px 14px' }}>
          {!weight ? (
            <div style={{ color: 'var(--t4)', fontSize: 12 }}>No data</div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
                <span style={{
                  fontSize: 28, fontWeight: 700,
                  color: weight.total > 60 ? 'var(--red)' : weight.total > 30 ? 'var(--amber)' : 'var(--green)',
                }}>
                  {weight.total.toFixed(0)}
                </span>
                <span style={{ color: 'var(--t3)', fontSize: 11 }}>/100</span>
              </div>
              {[
                ['Ghosts',    weight.ghost_nodes,      'var(--t3)'],
                ['Fossils',   weight.fossil_nodes,     'var(--amber)'],
                ['Void',      weight.void_nodes,       'var(--lavender-text)'],
                ['Isolated',  weight.isolated_nodes,   'var(--red)'],
                ['Contracts', weight.pending_contracts, 'var(--sky)'],
              ].map(([l, v, c]) => (
                <div key={String(l)} className="m-row" style={{ padding: '3px 0' }}>
                  <span className="m-label">{l}</span>
                  <span className="m-val" style={{ color: String(c) }}>{v}</span>
                </div>
              ))}
              <div style={{ color: 'var(--t3)', fontSize: 11, marginTop: 4, lineHeight: 1.5, paddingTop: 8, borderTop: '1px solid var(--line)' }}>
                {weight.summary}
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Rituals */}
      <div className="panel" style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
        <div className="sec-head">
          <span style={{ color: 'var(--green)' }}>⟳</span> Behavioral Rituals
          <span style={{ marginLeft: 'auto', color: 'var(--t4)', fontSize: 10 }}>{rituals.length}</span>
        </div>
        <div className="scroll" style={{ flex: 1 }}>
          {rituals.length === 0 && (
            <div style={{ padding: '12px 14px', color: 'var(--t4)', fontSize: 11 }}>No rituals detected</div>
          )}
          {rituals.map((r, i) => (
            <div key={i} style={{ padding: '8px 12px', borderBottom: '1px solid rgba(255,255,255,0.04)' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 4 }}>
                <span style={{ color: 'var(--t1)', fontSize: 12, fontWeight: 500 }}>{r.name}</span>
                <span className="badge badge-gn">{r.occurrence_count}×</span>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <Bar val={r.strength} color="var(--green)" />
                <span style={{ color: 'var(--t3)', fontSize: 10, fontFamily: 'var(--font-mono)' }}>
                  {r.sequence_len} steps
                </span>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Lore Arcs */}
      <div className="panel" style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
        <div className="sec-head">
          <span style={{ color: 'var(--sky)' }}>◆</span> Personal Lore
          <span style={{ marginLeft: 'auto', color: 'var(--t4)', fontSize: 10 }}>{lore.length} arcs</span>
        </div>
        <div className="scroll" style={{ flex: 1 }}>
          {lore.length === 0 && (
            <div style={{ padding: '12px 14px', color: 'var(--t4)', fontSize: 11 }}>
              No lore arcs yet — emerge from long-term patterns
            </div>
          )}
          {lore.map(e => {
            const col  = ARC_COLORS[e.arc_type] ?? 'var(--t2)'
            const icon = ARC_ICONS[e.arc_type]  ?? '◉'
            return (
              <div key={e.id} style={{ padding: '8px 12px', borderBottom: '1px solid rgba(255,255,255,0.04)' }}>
                <div style={{ display: 'flex', gap: 6, alignItems: 'center', marginBottom: 3 }}>
                  <span style={{ color: col }}>{icon}</span>
                  <span style={{ color: 'var(--t1)', fontSize: 12, fontWeight: 500 }}>{e.title}</span>
                  <span className="badge badge-mt" style={{ marginLeft: 'auto', fontSize: 9 }}>{e.arc_type}</span>
                </div>
                {e.narrative && (
                  <div style={{ color: 'var(--t3)', fontSize: 11, lineHeight: 1.5 }}>
                    {e.narrative.slice(0, 100)}{e.narrative.length > 100 ? '…' : ''}
                  </div>
                )}
                <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginTop: 4 }}>
                  <div className="bar" style={{ flex: 1 }}>
                    <div className="bar-fill" style={{ width: `${(e.significance * 100).toFixed(0)}%`, background: col }} />
                  </div>
                  <span style={{ color: 'var(--t4)', fontSize: 10 }}>{e.timestamp.slice(0, 10)}</span>
                </div>
              </div>
            )
          })}
        </div>
      </div>

      {/* Shadow Projects */}
      <div className="panel" style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
        <div className="sec-head">
          <span style={{ color: 'var(--t3)' }}>◌</span> Shadow Projects
          <span style={{ marginLeft: 'auto', color: 'var(--t4)', fontSize: 10 }}>{shadows.length}</span>
        </div>
        <div className="scroll" style={{ flex: 1 }}>
          {shadows.length === 0 && (
            <div style={{ padding: '12px 14px', color: 'var(--t4)', fontSize: 11 }}>
              No shadow projects — roads not yet taken
            </div>
          )}
          {shadows.map((s, i) => {
            const nodeId = s.label
            return (
              <div key={i} style={{ padding: '8px 12px', borderBottom: '1px solid rgba(255,255,255,0.04)' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 4 }}>
                  <span style={{ color: 'var(--t2)', fontSize: 12 }}>{s.label}</span>
                  <span className="badge badge-mt" style={{ fontSize: 9 }}>{s.origin_kind}</span>
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <div className="bar" style={{ flex: 1 }}>
                    <div className="bar-fill" style={{ width: `${(s.luminescence * 100).toFixed(0)}%`, background: 'var(--t3)' }} />
                  </div>
                  <span style={{ color: 'var(--t4)', fontSize: 10 }}>{s.age_days.toFixed(0)}d</span>
                </div>
                <div style={{ color: 'var(--t4)', fontSize: 10, marginTop: 3, lineHeight: 1.4 }}>
                  {s.description.slice(0, 80)}{s.description.length > 80 ? '…' : ''}
                </div>
              </div>
            )
          })}
        </div>
      </div>

      {/* Silent Contracts */}
      <div className="panel" style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
        <div className="sec-head">
          <span style={{ color: 'var(--red)' }}>⚠</span> Silent Contracts
          <span style={{ marginLeft: 'auto', color: 'var(--t4)', fontSize: 10 }}>{contracts.length}</span>
        </div>
        <div className="scroll" style={{ flex: 1 }}>
          {contracts.length === 0 && (
            <div style={{ padding: '12px 14px', color: 'var(--t4)', fontSize: 11 }}>
              No silent contracts — no unresolved obligations
            </div>
          )}
          {contracts.map((c, i) => (
            <div key={i} style={{ padding: '8px 12px', borderBottom: '1px solid rgba(255,255,255,0.04)' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
                <div className="bar" style={{ flex: 1 }}>
                  <div className="bar-fill" style={{
                    width: `${(c.strength * 100).toFixed(0)}%`,
                    background: c.strength > 0.6 ? 'var(--red)' : 'var(--amber)',
                  }} />
                </div>
                <span style={{
                  color: c.strength > 0.6 ? 'var(--red)' : 'var(--amber)',
                  fontSize: 10, fontFamily: 'var(--font-mono)', flexShrink: 0,
                }}>
                  {c.strength.toFixed(2)}
                </span>
                <span style={{ color: 'var(--t4)', fontSize: 10, flexShrink: 0 }}>{c.age_days.toFixed(0)}d</span>
              </div>
              <div style={{ color: 'var(--t2)', fontSize: 11, lineHeight: 1.4, marginBottom: 6 }}>
                {c.description}
              </div>
              <div style={{ display: 'flex', gap: 5 }}>
                <button
                  className="btn-xs btn-emerald"
                  onClick={async () => {
                    try {
                      await api.fulfillContract(c.node_id)
                      toast('Contract fulfilled', 'success')
                      load()
                    } catch (e) { toast(String(e), 'error') }
                  }}
                >
                  ✓ Fulfill
                </button>
                <button
                  className="btn-xs btn-ghost"
                  onClick={async () => {
                    try {
                      await api.releaseContract(c.node_id)
                      toast('Contract released')
                      load()
                    } catch (e) { toast(String(e), 'error') }
                  }}
                >
                  ◌ Release
                </button>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Crystallization */}
      <div className="panel" style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden', gridColumn: '1 / -1' }}>
        <div className="sec-head">
          <span style={{ color: 'var(--sky)' }}>◆</span> Civilization Crystallization
          <span style={{ marginLeft: 'auto', color: 'var(--t4)', fontSize: 10 }}>{civs.length} civilizations</span>
        </div>
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8, padding: '10px 12px' }}>
          {civs.length === 0 && (
            <span style={{ color: 'var(--t4)', fontSize: 11 }}>
              No civilizations detected — connect more nodes to form clusters
            </span>
          )}
          {civs.map(civ => (
            <div key={civ.id} style={{
              background: 'var(--raised)', border: '1px solid var(--line)',
              borderRadius: 6, padding: '8px 10px',
              display: 'flex', flexDirection: 'column', gap: 4, minWidth: 180,
            }}>
              <div style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
                <span style={{ color: 'var(--sky)', fontSize: 10, fontWeight: 600 }}>
                  {civ.member_count} nodes
                </span>
                <span style={{ color: 'var(--t4)', fontSize: 9 }}>
                  {civ.age_days.toFixed(0)}d old
                </span>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                <span style={{ fontSize: 9, color: 'var(--t4)', width: 50 }}>density</span>
                <div className="bar" style={{ flex: 1 }}>
                  <div className="bar-fill" style={{
                    width: `${(civ.internal_density * 100).toFixed(0)}%`,
                    background: civ.internal_density > 0.65 ? 'var(--sky)' : 'var(--t4)',
                  }} />
                </div>
                <span style={{ fontSize: 9, color: 'var(--t3)', fontFamily: 'var(--font-mono)' }}>
                  {(civ.internal_density * 100).toFixed(0)}%
                </span>
              </div>
              <button
                className="btn-xs btn-primary"
                style={{ marginTop: 2 }}
                onClick={async () => {
                  try {
                    const r = await api.crystallize(civ.id)
                    if (r.qualifies && r.crystal_id) {
                      toast(`Crystal formed: ${r.crystal_id.slice(0, 8)}`, 'success')
                    } else {
                      toast(`Not ready — density: ${(r.internal_density * 100).toFixed(0)}%, stability: ${(r.stability_score * 100).toFixed(0)}%`, 'info')
                    }
                  } catch (e) { toast(String(e), 'error') }
                }}
              >
                ◆ Crystallize
              </button>
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}
