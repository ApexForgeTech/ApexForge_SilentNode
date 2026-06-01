import { useState, useEffect } from 'react'
import type { VoidZoneNode, ResonanceChamber } from '../types'
import { api } from '../api'
import { toast } from './Toast'

export default function VoidResonanceView() {
  const [tab, setTab] = useState<'void' | 'resonance'>('void')
  const [voids, setVoids]       = useState<VoidZoneNode[]>([])
  const [chambers, setChambers] = useState<ResonanceChamber[]>([])
  const [threshold, setThreshold] = useState(0.45)
  const [loading, setLoading]   = useState(true)

  useEffect(() => { load() }, [tab, threshold])

  async function load() {
    setLoading(true)
    if (tab === 'void') {
      try { setVoids(await api.voidZones()) } catch {}
    } else {
      try { setChambers(await api.resonanceChambers(threshold)) } catch {}
    }
    setLoading(false)
  }

  async function extractFromVoid(id: string) {
    try {
      await api.voidToggle(id)
      toast('Node extracted from Void')
      load()
    } catch (e) { toast(String(e), 'error') }
  }

  return (
    <div className="col" style={{ height: '100%', gap: 10 }}>
      {/* Tab strip */}
      <div className="tabs" style={{ flexShrink: 0 }}>
        <button className={`tab${tab === 'void' ? ' active' : ''}`} onClick={() => setTab('void')}>
          ◌ Void Zones
        </button>
        <button className={`tab${tab === 'resonance' ? ' active' : ''}`} onClick={() => setTab('resonance')}>
          ◈ Resonance Chambers
        </button>
      </div>

      {tab === 'void' && (
        <div className="col fill" style={{ gap: 10, overflow: 'hidden' }}>
          {/* Summary */}
          <div className="panel" style={{ flexShrink: 0, padding: '10px 14px' }}>
            <div style={{ display: 'flex', gap: 16, alignItems: 'center' }}>
              <div style={{ textAlign: 'center' }}>
                <div style={{ fontSize: 24, fontWeight: 700, color: 'var(--lavender-text)', fontFamily: 'var(--font-mono)' }}>
                  {voids.length}
                </div>
                <div style={{ fontSize: 9, color: 'var(--t4)' }}>IN VOID</div>
              </div>
              <div style={{ textAlign: 'center' }}>
                <div style={{ fontSize: 24, fontWeight: 700, color: 'var(--green)', fontFamily: 'var(--font-mono)' }}>
                  {voids.filter(v => v.is_mature).length}
                </div>
                <div style={{ fontSize: 9, color: 'var(--t4)' }}>MATURE</div>
              </div>
              <div style={{ flex: 1, fontSize: 11, color: 'var(--t3)', lineHeight: 1.6 }}>
                The Void is a protected incubation zone. Nodes sent here rest undisturbed, growing in resonance readiness until they're ready to re-emerge.
              </div>
            </div>
          </div>

          {/* Void node list */}
          <div className="panel fill scroll">
            <div className="sec-head">
              <span style={{ color: 'var(--lavender-text)' }}>◌</span>
              Void Zones
            </div>
            {loading && <div style={{ padding: 16, color: 'var(--t4)', fontSize: 11 }}>Loading void zones…</div>}
            {!loading && voids.length === 0 && (
              <div style={{ padding: 16, color: 'var(--t4)', fontSize: 11 }}>No nodes in the Void</div>
            )}
            {voids.map(v => {
              const maturity = v.is_mature ? 'var(--green)' : v.resonance_readiness > 0.5 ? 'var(--amber)' : 'var(--t4)'
              return (
                <div key={v.node_id} style={{
                  padding: '10px 14px',
                  borderBottom: '1px solid rgba(255,255,255,0.04)',
                  display: 'flex', flexDirection: 'column', gap: 6,
                }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                    <span style={{
                      width: 8, height: 8, borderRadius: '50%', flexShrink: 0,
                      background: maturity,
                      boxShadow: v.is_mature ? `0 0 8px ${maturity}` : 'none',
                    }} />
                    <span style={{ flex: 1, fontSize: 12, color: 'var(--t1)' }}>
                      {v.content_preview}
                    </span>
                    {v.is_mature && (
                      <span className="badge badge-gn" style={{ fontSize: 9 }}>ready</span>
                    )}
                  </div>
                  <div style={{ display: 'flex', gap: 12, alignItems: 'center', paddingLeft: 16 }}>
                    <div style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
                      <span style={{ fontSize: 9, color: 'var(--t4)' }}>incubation</span>
                      <span style={{ fontSize: 10, color: 'var(--lavender-text)', fontFamily: 'var(--font-mono)' }}>
                        {v.incubation_days.toFixed(0)}d
                      </span>
                    </div>
                    <div style={{ flex: 1, display: 'flex', gap: 6, alignItems: 'center' }}>
                      <span style={{ fontSize: 9, color: 'var(--t4)' }}>resonance</span>
                      <div className="bar fill">
                        <div className="bar-fill" style={{
                          width: `${(v.resonance_readiness * 100).toFixed(0)}%`,
                          background: maturity,
                        }} />
                      </div>
                      <span style={{ fontSize: 9, color: maturity, fontFamily: 'var(--font-mono)' }}>
                        {(v.resonance_readiness * 100).toFixed(0)}%
                      </span>
                    </div>
                    <button
                      className="btn-xs btn-primary"
                      onClick={() => extractFromVoid(v.node_id)}
                      title="Extract from Void"
                    >
                      ↑ Extract
                    </button>
                  </div>
                </div>
              )
            })}
          </div>
        </div>
      )}

      {tab === 'resonance' && (
        <div className="col fill" style={{ gap: 10, overflow: 'hidden' }}>
          {/* Threshold control */}
          <div className="panel" style={{ flexShrink: 0, padding: '10px 14px' }}>
            <div style={{ display: 'flex', gap: 12, alignItems: 'center' }}>
              <span style={{ fontSize: 11, color: 'var(--t3)' }}>Similarity threshold</span>
              <input
                type="range" min="0.1" max="0.9" step="0.05"
                value={threshold}
                onChange={e => setThreshold(parseFloat(e.target.value))}
                style={{ flex: 1, accentColor: 'var(--lavender)' }}
              />
              <span style={{
                color: 'var(--lavender-text)', fontSize: 12, fontFamily: 'var(--font-mono)',
                width: 38, textAlign: 'right',
              }}>
                {(threshold * 100).toFixed(0)}%
              </span>
            </div>
            <div style={{ marginTop: 6, fontSize: 10, color: 'var(--t4)' }}>
              {chambers.length} resonant pairs found above {(threshold * 100).toFixed(0)}% similarity
            </div>
          </div>

          {/* Chamber list */}
          <div className="panel fill scroll">
            <div className="sec-head">
              <span style={{ color: 'var(--lavender-text)' }}>◈</span>
              Open Chambers
              <span style={{ marginLeft: 'auto', color: 'var(--t4)', fontSize: 10 }}>{chambers.length}</span>
            </div>
            {loading && <div style={{ padding: 16, color: 'var(--t4)', fontSize: 11 }}>Opening chambers…</div>}
            {!loading && chambers.length === 0 && (
              <div style={{ padding: 16, color: 'var(--t4)', fontSize: 11 }}>
                No resonant pairs at this threshold.
                Lower the threshold to find more distant connections.
              </div>
            )}
            {chambers.map(ch => {
              const simColor = ch.similarity > 0.75 ? 'var(--green)'
                             : ch.similarity > 0.55 ? 'var(--amber)' : 'var(--sky)'
              return (
                <div key={ch.id} style={{
                  padding: '10px 14px',
                  borderBottom: '1px solid rgba(255,255,255,0.04)',
                  display: 'flex', flexDirection: 'column', gap: 6,
                }}>
                  <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
                    <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 3 }}>
                      <span style={{ fontSize: 11, color: 'var(--t1)' }}>
                        {ch.preview_a}
                      </span>
                      <span style={{ fontSize: 9, color: 'var(--t4)', fontFamily: 'var(--font-mono)' }}>
                        ⟿ resonates with
                      </span>
                      <span style={{ fontSize: 11, color: 'var(--t1)' }}>
                        {ch.preview_b}
                      </span>
                    </div>
                    <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-end', gap: 4, flexShrink: 0 }}>
                      <span style={{
                        fontSize: 14, fontWeight: 700, color: simColor, fontFamily: 'var(--font-mono)',
                      }}>
                        {(ch.similarity * 100).toFixed(0)}%
                      </span>
                      <div className="bar" style={{ width: 50, height: 3 }}>
                        <div className="bar-fill" style={{ width: `${(ch.similarity * 100).toFixed(0)}%`, background: simColor }} />
                      </div>
                    </div>
                  </div>
                  <div style={{ display: 'flex', gap: 5 }}>
                    <button
                      className="btn-xs btn-primary"
                      onClick={async () => {
                        try {
                          await api.connect(ch.node_a, ch.node_b, ch.similarity)
                          toast('Resonance accepted — nodes connected')
                          load()
                        } catch (e) { toast(String(e), 'error') }
                      }}
                    >
                      ✓ Accept
                    </button>
                    <button
                      className="btn-xs btn-ghost"
                      onClick={() => toast(`Noted: ${ch.preview_a.slice(0, 20)} ↔ ${ch.preview_b.slice(0, 20)}`)}
                    >
                      ◌ Note
                    </button>
                  </div>
                </div>
              )
            })}
          </div>
        </div>
      )}
    </div>
  )
}
