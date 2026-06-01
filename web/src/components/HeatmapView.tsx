import { useState, useEffect } from 'react'
import type { HeatmapData, TrailEvent } from '../types'
import { api } from '../api'
import { toast } from './Toast'

const DEPTH_COL: Record<string, string> = {
  DeepWork: 'var(--lavender-text)',
  Edit:     'var(--green)',
  Read:     'var(--sky)',
  Glance:   'var(--t3)',
}
const WEEKDAYS = ['Mon','Tue','Wed','Thu','Fri','Sat','Sun']

function Bar({ val, color = 'var(--lavender-text)' }: { val: number; color?: string }) {
  return (
    <div className="bar fill">
      <div className="bar-fill" style={{ width: `${(val * 100).toFixed(0)}%`, background: color }} />
    </div>
  )
}

export default function HeatmapView() {
  const [heatmap, setHeatmap] = useState<HeatmapData | null>(null)
  const [trail,   setTrail]   = useState<TrailEvent[]>([])
  const [days,    setDays]    = useState(30)
  const [hours,   setHours]   = useState(72)
  const [loading, setLoading] = useState(true)
  const [tab,     setTab]     = useState<'heatmap'|'trail'|'loops'|'neglected'>('heatmap')

  async function load() {
    setLoading(true)
    const [h, t] = await Promise.allSettled([api.heatmap(days), api.trail(hours)])
    if (h.status === 'fulfilled') setHeatmap(h.value)
    if (t.status === 'fulfilled') setTrail(t.value)
    setLoading(false)
  }

  useEffect(() => { load() }, [days, hours])

  function formatDuration(secs: number) {
    if (secs < 60) return `${secs.toFixed(0)}s`
    return `${(secs / 60).toFixed(1)}m`
  }

  function relTime(ts: string) {
    const diff = Date.now() - new Date(ts).getTime()
    const h = Math.floor(diff / 3600000)
    const m = Math.floor((diff % 3600000) / 60000)
    if (h > 0) return `${h}h ago`
    return `${m}m ago`
  }

  const topEntries = heatmap?.entries.slice(0, 30) ?? []
  const maxEnergy  = topEntries[0]?.energy ?? 1

  return (
    <div className="col" style={{ height: '100%', gap: 10 }}>

      {/* Controls */}
      <div className="panel" style={{ padding: '8px 12px', display: 'flex', gap: 12, alignItems: 'center', flexWrap: 'wrap' }}>
        <div className="tabs">
          {(['heatmap','trail','loops','neglected'] as const).map(t => (
            <button key={t} className={`tab${tab === t ? ' active' : ''}`} onClick={() => setTab(t)}>
              {t === 'heatmap'   && '◆ Heatmap'}
              {t === 'trail'     && '→ Focus Trail'}
              {t === 'loops'     && '⟳ Obsessive Loops'}
              {t === 'neglected' && '◌ Neglected'}
            </button>
          ))}
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginLeft: 'auto' }}>
          {tab !== 'trail' && (
            <>
              <span style={{ color: 'var(--t4)', fontSize: 11 }}>Window:</span>
              {[7, 14, 30, 90].map(d => (
                <button key={d} className={`btn-xs${days === d ? ' btn-primary' : ''}`}
                  onClick={() => setDays(d)}>{d}d</button>
              ))}
            </>
          )}
          {tab === 'trail' && (
            <>
              <span style={{ color: 'var(--t4)', fontSize: 11 }}>Show:</span>
              {[24, 48, 168].map(h => (
                <button key={h} className={`btn-xs${hours === h ? ' btn-primary' : ''}`}
                  onClick={() => setHours(h)}>{h < 48 ? `${h}h` : `${h/24}d`}</button>
              ))}
            </>
          )}
          <button className="btn-sm" onClick={load}>↺</button>
        </div>
      </div>

      {/* Content */}
      {loading ? (
        <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--t4)' }}>
          Loading…
        </div>
      ) : (
        <div className="fill" style={{ overflow: 'hidden' }}>

          {/* ── Heatmap ── */}
          {tab === 'heatmap' && (
            <div style={{ display: 'flex', gap: 10, height: '100%' }}>
              {/* Bar chart */}
              <div className="panel fill scroll">
                <div className="sec-head">
                  <span style={{ color: 'var(--lavender-text)' }}>◆</span>
                  Thought Energy Distribution
                  <span style={{ marginLeft: 'auto', color: 'var(--t4)', fontSize: 10 }}>
                    {heatmap?.entries.length ?? 0} nodes · {days}d window
                  </span>
                </div>
                {topEntries.length === 0 && (
                  <div style={{ padding: '16px 14px', color: 'var(--t4)', fontSize: 12 }}>
                    No focus events in this window. Record focus events to populate the heatmap.
                  </div>
                )}
                {topEntries.map((e, i) => {
                  const rel = e.energy / maxEnergy
                  const col = rel > 0.7 ? 'var(--lavender-text)'
                            : rel > 0.4 ? 'var(--sky)'
                            : rel > 0.2 ? 'var(--green)'
                            : 'var(--t3)'
                  return (
                    <div key={e.node_id} style={{
                      display: 'flex', alignItems: 'center', gap: 10,
                      padding: '6px 12px', borderBottom: '1px solid rgba(255,255,255,0.04)',
                    }}>
                      <span style={{ color: 'var(--t4)', fontSize: 10, width: 20, textAlign: 'right' }}>
                        {i + 1}
                      </span>
                      <div style={{
                        width: 8, height: 8, borderRadius: '50%',
                        background: col, flexShrink: 0,
                        opacity: 0.5 + rel * 0.5,
                      }} />
                      <span style={{ color: 'var(--t2)', fontSize: 12, flex: 1, minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                        {e.content_preview}
                      </span>
                      <div style={{ width: 120, display: 'flex', alignItems: 'center', gap: 6, flexShrink: 0 }}>
                        <Bar val={rel} color={col} />
                        <span style={{ color: col, fontSize: 10, fontFamily: 'var(--font-mono)', width: 36, textAlign: 'right' }}>
                          {(e.energy * 100).toFixed(0)}%
                        </span>
                      </div>
                    </div>
                  )
                })}
              </div>

              {/* Summary panel */}
              <div style={{ width: 240, display: 'flex', flexDirection: 'column', gap: 10, flexShrink: 0 }}>
                <div className="panel">
                  <div className="sec-head"><span style={{ color: 'var(--lavender-text)' }}>◈</span> Summary</div>
                  <div style={{ padding: '10px 12px', display: 'flex', flexDirection: 'column', gap: 6 }}>
                    {[
                      ['Active nodes',  heatmap?.entries.filter(e => e.energy > 0.1).length ?? 0, 'var(--green)'],
                      ['Silent nodes',  heatmap?.entries.filter(e => e.energy === 0).length ?? 0, 'var(--t4)'],
                      ['Loops',         heatmap?.obsessive_loops.length ?? 0, 'var(--amber)'],
                      ['Neglected',     heatmap?.neglected_regions.length ?? 0, 'var(--red)'],
                    ].map(([l, v, c]) => (
                      <div key={String(l)} className="m-row" style={{ padding: '3px 0' }}>
                        <span className="m-label">{l}</span>
                        <span className="m-val" style={{ color: String(c) }}>{v}</span>
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* ── Focus Trail ── */}
          {tab === 'trail' && (
            <div className="panel fill scroll">
              <div className="sec-head">
                <span style={{ color: 'var(--green)' }}>→</span>
                Focus Trail
                <span style={{ marginLeft: 'auto', color: 'var(--t4)', fontSize: 10 }}>
                  {trail.length} events · last {hours}h
                </span>
              </div>
              {trail.length === 0 && (
                <div style={{ padding: '16px 14px', color: 'var(--t4)', fontSize: 12 }}>
                  No focus events in this window.
                </div>
              )}
              {trail.map((ev, i) => {
                const col = DEPTH_COL[ev.depth] ?? 'var(--t3)'
                return (
                  <div key={i} style={{
                    display: 'flex', alignItems: 'center', gap: 10,
                    padding: '7px 12px', borderBottom: '1px solid rgba(255,255,255,0.04)',
                  }}>
                    {/* Time */}
                    <span style={{ color: 'var(--t4)', fontSize: 10, width: 50, textAlign: 'right', fontFamily: 'var(--font-mono)', flexShrink: 0 }}>
                      {relTime(ev.timestamp)}
                    </span>
                    {/* Depth indicator */}
                    <div style={{ width: 6, height: 6, borderRadius: '50%', background: col, flexShrink: 0 }} />
                    {/* Node */}
                    <span style={{ color: 'var(--t1)', fontSize: 12, flex: 1, minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                      {ev.content_preview}
                    </span>
                    {/* Duration */}
                    <span style={{ color: 'var(--t3)', fontSize: 10, fontFamily: 'var(--font-mono)', width: 40, textAlign: 'right', flexShrink: 0 }}>
                      {formatDuration(ev.duration_seconds)}
                    </span>
                    {/* Depth badge */}
                    <span className="badge" style={{
                      color: col, borderColor: col + '44',
                      background: col + '15', fontSize: 9, flexShrink: 0,
                    }}>
                      {ev.depth}
                    </span>
                  </div>
                )
              })}
            </div>
          )}

          {/* ── Obsessive Loops ── */}
          {tab === 'loops' && (
            <div className="panel fill scroll">
              <div className="sec-head">
                <span style={{ color: 'var(--amber)' }}>⟳</span>
                Obsessive Loops
                <span style={{ marginLeft: 4, color: 'var(--t4)', fontSize: 10 }}>
                  High attention, low output
                </span>
              </div>
              {(heatmap?.obsessive_loops.length ?? 0) === 0 && (
                <div style={{ padding: '16px 14px', color: 'var(--t4)', fontSize: 12 }}>
                  No obsessive loops detected. This is a sign of healthy cognitive balance.
                </div>
              )}
              {heatmap?.obsessive_loops.map((l, i) => (
                <div key={i} style={{ padding: '10px 12px', borderBottom: '1px solid rgba(255,255,255,0.04)' }}>
                  <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 6 }}>
                    <span style={{ color: 'var(--t1)', fontSize: 12, fontWeight: 500 }}>{l.content_preview}</span>
                    <span className="badge badge-am">{l.revisit_count}× revisited</span>
                  </div>
                  <div style={{ display: 'flex', gap: 12 }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                      <span style={{ color: 'var(--t4)', fontSize: 10 }}>avg session</span>
                      <span style={{ color: 'var(--amber)', fontSize: 11, fontFamily: 'var(--font-mono)' }}>
                        {formatDuration(l.avg_session_seconds)}
                      </span>
                    </div>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                      <span style={{ color: 'var(--t4)', fontSize: 10 }}>entropy</span>
                      <span style={{ color: l.entropy > 0.5 ? 'var(--red)' : 'var(--amber)', fontSize: 11, fontFamily: 'var(--font-mono)' }}>
                        {(l.entropy * 100).toFixed(0)}%
                      </span>
                    </div>
                  </div>
                  <div style={{ marginTop: 6, padding: '5px 8px', background: 'rgba(251,191,36,0.06)', borderRadius: 4, fontSize: 11, color: 'var(--t3)', lineHeight: 1.5 }}>
                    You return here often but progress is stalled. Consider committing to a decision or releasing this as a Silent Contract.
                  </div>
                </div>
              ))}
            </div>
          )}

          {/* ── Neglected ── */}
          {tab === 'neglected' && (
            <div className="panel fill scroll">
              <div className="sec-head">
                <span style={{ color: 'var(--t3)' }}>◌</span>
                Neglected Regions
                <span style={{ marginLeft: 4, color: 'var(--t4)', fontSize: 10 }}>
                  Connected to active areas but unvisited
                </span>
              </div>
              {(heatmap?.neglected_regions.length ?? 0) === 0 && (
                <div style={{ padding: '16px 14px', color: 'var(--t4)', fontSize: 12 }}>
                  No neglected regions — all connected areas are being attended to.
                </div>
              )}
              {heatmap?.neglected_regions.map((r, i) => (
                <div key={i} style={{ padding: '10px 12px', borderBottom: '1px solid rgba(255,255,255,0.04)' }}>
                  <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 5 }}>
                    <span style={{ color: 'var(--t2)', fontSize: 12 }}>{r.content_preview}</span>
                    <span style={{ color: 'var(--red)', fontSize: 11, fontFamily: 'var(--font-mono)' }}>
                      {r.days_since_access.toFixed(0)}d silent
                    </span>
                  </div>
                  <span style={{ color: 'var(--t4)', fontSize: 11 }}>
                    {r.connected_active_nodes} active neighbor{r.connected_active_nodes !== 1 ? 's' : ''}
                  </span>
                </div>
              ))}
            </div>
          )}

        </div>
      )}
    </div>
  )
}
