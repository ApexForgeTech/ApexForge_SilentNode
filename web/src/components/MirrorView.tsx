import { useState, useEffect } from 'react'
import type { MirrorData } from '../types'
import { api } from '../api'

const WEEKDAYS = ['Mon','Tue','Wed','Thu','Fri','Sat','Sun']
const HOURS    = Array.from({ length: 24 }, (_, i) => `${i}:00`)

function Bar({ val, color = 'var(--lavender-text)' }: { val: number; color?: string }) {
  return (
    <div className="bar fill">
      <div className="bar-fill" style={{ width: `${Math.min(val * 100, 100).toFixed(0)}%`, background: color }} />
    </div>
  )
}

function GapIndicator({ gap }: { gap: number }) {
  const abs = Math.abs(gap)
  const col = gap > 0 ? 'var(--red)' : 'var(--green)'
  const label = gap > 0 ? 'neglected' : 'over-focused'
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
      <span style={{ color: col, fontSize: 10 }}>{gap > 0 ? '↓' : '↑'} {abs} rank {label}</span>
    </div>
  )
}

export default function MirrorView() {
  const [data,    setData]    = useState<MirrorData | null>(null)
  const [days,    setDays]    = useState(30)
  const [loading, setLoading] = useState(true)
  const [tab,     setTab]     = useState<'gaps'|'blind'|'obsessions'|'creative'|'evolution'>('gaps')

  useEffect(() => {
    setLoading(true)
    api.mirror(days).then(d => { setData(d); setLoading(false) }).catch(() => setLoading(false))
  }, [days])

  if (loading) return (
    <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--t4)' }}>
      Generating your cognitive portrait…
    </div>
  )

  if (!data) return (
    <div style={{ padding: 20, color: 'var(--t4)' }}>Failed to load mirror data.</div>
  )

  return (
    <div className="col" style={{ height: '100%', gap: 10 }}>

      {/* Header quote */}
      <div style={{ padding: '10px 14px', borderBottom: '1px solid var(--line)', flexShrink: 0 }}>
        <div style={{ fontSize: 11, color: 'var(--t3)', fontStyle: 'italic', lineHeight: 1.6 }}>
          "SilentNode sees you more clearly than you see yourself. The mirror reflects. It does not judge."
        </div>
      </div>

      {/* Controls */}
      <div style={{ display: 'flex', gap: 10, alignItems: 'center', flexShrink: 0, flexWrap: 'wrap' }}>
        <div className="tabs">
          {([
            ['gaps',      'Priority Gaps'],
            ['blind',     'Blind Spots'],
            ['obsessions','Obsessions'],
            ['creative',  'Creative Pattern'],
            ['evolution', 'Evolution'],
          ] as const).map(([t, l]) => (
            <button key={t} className={`tab${tab === t ? ' active' : ''}`} onClick={() => setTab(t)}>
              {l}
            </button>
          ))}
        </div>
        <div style={{ display: 'flex', gap: 5, marginLeft: 'auto', alignItems: 'center' }}>
          <span style={{ color: 'var(--t4)', fontSize: 11 }}>Window:</span>
          {[7, 30, 90].map(d => (
            <button key={d} className={`btn-xs${days === d ? ' btn-primary' : ''}`}
              onClick={() => setDays(d)}>{d}d</button>
          ))}
        </div>
      </div>

      {/* ── Priority Gaps ── */}
      {tab === 'gaps' && (
        <div className="panel fill scroll">
          <div className="sec-head">
            <span style={{ color: 'var(--amber)' }}>◈</span>
            Stated vs Actual Priorities
            <span style={{ marginLeft: 4, color: 'var(--t4)', fontSize: 10 }}>
              gravity rank vs focus attention rank
            </span>
          </div>
          {data.priority_gaps.length === 0 && (
            <div style={{ padding: '16px 14px', color: 'var(--t4)', fontSize: 12 }}>
              No priority gaps detected — your attention matches your stated priorities.
            </div>
          )}
          {data.priority_gaps.slice(0, 20).map((g, i) => (
            <div key={g.node_id} style={{
              padding: '9px 12px', borderBottom: '1px solid rgba(255,255,255,0.04)',
              display: 'flex', gap: 10, alignItems: 'center',
            }}>
              <span style={{ color: 'var(--t4)', fontSize: 10, width: 20 }}>{i+1}</span>
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ color: 'var(--t1)', fontSize: 12, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                  {g.content_preview}
                </div>
                <div style={{ display: 'flex', gap: 12, marginTop: 3 }}>
                  <span style={{ color: 'var(--t4)', fontSize: 10 }}>Gravity rank #{g.stated_rank + 1}</span>
                  <span style={{ color: 'var(--t4)', fontSize: 10 }}>Focus rank #{g.actual_rank + 1}</span>
                </div>
              </div>
              <GapIndicator gap={g.gap} />
            </div>
          ))}
        </div>
      )}

      {/* ── Blind Spots ── */}
      {tab === 'blind' && (
        <div className="panel fill scroll">
          <div className="sec-head">
            <span style={{ color: 'var(--red)' }}>◌</span>
            Cognitive Blind Spots
            <span style={{ marginLeft: 4, color: 'var(--t4)', fontSize: 10 }}>
              connected to active work but never visited
            </span>
          </div>
          {data.blind_spots.length === 0 && (
            <div style={{ padding: '16px 14px', color: 'var(--t4)', fontSize: 12 }}>
              No blind spots detected — all relevant areas are receiving attention.
            </div>
          )}
          {data.blind_spots.map((b, i) => (
            <div key={b.node_id} style={{
              padding: '9px 12px', borderBottom: '1px solid rgba(255,255,255,0.04)',
              display: 'flex', gap: 10, alignItems: 'center',
            }}>
              <div style={{ width: 7, height: 7, borderRadius: '50%', background: 'var(--red)', opacity: 0.6, flexShrink: 0 }} />
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ color: 'var(--t1)', fontSize: 12, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                  {b.content_preview}
                </div>
              </div>
              <span style={{ color: 'var(--red)', fontSize: 11, fontFamily: 'var(--font-mono)', flexShrink: 0 }}>
                {b.last_accessed_days_ago.toFixed(0)}d silent
              </span>
            </div>
          ))}
        </div>
      )}

      {/* ── Obsessions ── */}
      {tab === 'obsessions' && (
        <div className="panel fill scroll">
          <div className="sec-head">
            <span style={{ color: 'var(--amber)' }}>⟳</span>
            Obsession Map
            <span style={{ marginLeft: 4, color: 'var(--t4)', fontSize: 10 }}>
              disproportionate attention relative to output
            </span>
          </div>
          {data.obsessions.length === 0 && (
            <div style={{ padding: '16px 14px', color: 'var(--t4)', fontSize: 12 }}>
              No obsessions detected.
            </div>
          )}
          {data.obsessions.map((o, i) => {
            const eCol = o.entropy > 0.6 ? 'var(--red)' : o.entropy > 0.3 ? 'var(--amber)' : 'var(--green)'
            return (
              <div key={o.node_id} style={{ padding: '10px 12px', borderBottom: '1px solid rgba(255,255,255,0.04)' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 6 }}>
                  <span style={{ color: 'var(--t1)', fontSize: 12, fontWeight: 500 }}>{o.content_preview}</span>
                  <span className="badge badge-am">{o.revisit_count}× visits</span>
                </div>
                <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
                  <span style={{ color: 'var(--t4)', fontSize: 10, width: 60 }}>Focus</span>
                  <Bar val={o.focus_score} color="var(--amber)" />
                  <span style={{ color: 'var(--amber)', fontSize: 10, fontFamily: 'var(--font-mono)', width: 36, textAlign: 'right' }}>
                    {(o.focus_score * 100).toFixed(0)}%
                  </span>
                </div>
                <div style={{ display: 'flex', gap: 8, alignItems: 'center', marginTop: 3 }}>
                  <span style={{ color: 'var(--t4)', fontSize: 10, width: 60 }}>Entropy</span>
                  <Bar val={o.entropy} color={eCol} />
                  <span style={{ color: eCol, fontSize: 10, fontFamily: 'var(--font-mono)', width: 36, textAlign: 'right' }}>
                    {(o.entropy * 100).toFixed(0)}%
                  </span>
                </div>
              </div>
            )
          })}
        </div>
      )}

      {/* ── Creative Pattern ── */}
      {tab === 'creative' && (
        <div style={{ display: 'flex', gap: 10, height: '100%' }}>
          <div className="panel" style={{ flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
            <div className="sec-head">
              <span style={{ color: 'var(--green)' }}>◉</span>
              When You Actually Create
            </div>
            <div style={{ padding: '16px 14px', display: 'flex', flexDirection: 'column', gap: 14 }}>
              {data.peak_hour !== null ? (
                <>
                  <div>
                    <div style={{ color: 'var(--t4)', fontSize: 10, marginBottom: 6 }}>PEAK CREATIVE HOUR</div>
                    <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap' }}>
                      {Array.from({ length: 24 }, (_, h) => (
                        <div key={h} style={{
                          width: 26, height: 26, borderRadius: 4,
                          background: h === data.peak_hour
                            ? 'var(--lavender-soft)'
                            : 'rgba(255,255,255,0.03)',
                          border: `1px solid ${h === data.peak_hour ? 'rgba(167,139,250,0.4)' : 'var(--line)'}`,
                          display: 'flex', alignItems: 'center', justifyContent: 'center',
                          fontSize: 9, color: h === data.peak_hour ? 'var(--lavender-text)' : 'var(--t4)',
                          fontFamily: 'var(--font-mono)',
                        }}>
                          {h}
                        </div>
                      ))}
                    </div>
                    <div style={{ marginTop: 6, color: 'var(--lavender-text)', fontSize: 12 }}>
                      Peak: {HOURS[data.peak_hour]}
                    </div>
                  </div>

                  <div>
                    <div style={{ color: 'var(--t4)', fontSize: 10, marginBottom: 6 }}>PEAK DAY OF WEEK</div>
                    <div style={{ display: 'flex', gap: 4 }}>
                      {WEEKDAYS.map((d, i) => (
                        <div key={d} style={{
                          flex: 1, height: 40, borderRadius: 4,
                          background: i === data.peak_weekday
                            ? 'var(--lavender-soft)'
                            : 'rgba(255,255,255,0.03)',
                          border: `1px solid ${i === data.peak_weekday ? 'rgba(167,139,250,0.4)' : 'var(--line)'}`,
                          display: 'flex', alignItems: 'center', justifyContent: 'center',
                          fontSize: 10, color: i === data.peak_weekday ? 'var(--lavender-text)' : 'var(--t4)',
                        }}>
                          {d}
                        </div>
                      ))}
                    </div>
                  </div>
                </>
              ) : (
                <div style={{ color: 'var(--t4)', fontSize: 12 }}>
                  Insufficient focus data. Record DeepWork focus events to generate your creative pattern.
                </div>
              )}

              <div className="divider" />
              <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
                {[
                  ['Focus Period',        data.focus_period,                            'var(--lavender-text)'],
                  ['Deep Work Sessions',  String(data.deep_work_event_count),           'var(--green)'],
                ].map(([l, v, c]) => (
                  <div key={l} className="m-row" style={{ padding: '4px 0' }}>
                    <span className="m-label">{l}</span>
                    <span className="m-val" style={{ color: c }}>{v}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      )}

      {/* ── Evolution Portrait ── */}
      {tab === 'evolution' && (
        <div className="panel fill scroll">
          <div className="sec-head">
            <span style={{ color: 'var(--sky)' }}>⟳</span>
            Evolution Portrait
            <span style={{ marginLeft: 4, color: 'var(--t4)', fontSize: 10 }}>how your thinking has changed</span>
          </div>
          {data.evolution.length === 0 && (
            <div style={{ padding: '16px 14px', color: 'var(--t4)', fontSize: 12 }}>
              No evolution data yet. Requires temporal snapshots — run `cargo run -- snapshot-all` to begin recording.
            </div>
          )}
          {data.evolution.map(e => {
            const tCol = e.trajectory === 'rising'  ? 'var(--green)'
                       : e.trajectory === 'decaying' ? 'var(--red)'
                       : 'var(--t3)'
            const tIcon = e.trajectory === 'rising' ? '↑' : e.trajectory === 'decaying' ? '↓' : '→'
            return (
              <div key={e.node_id} style={{ padding: '10px 12px', borderBottom: '1px solid rgba(255,255,255,0.04)' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 6 }}>
                  <span style={{ color: 'var(--t1)', fontSize: 12, fontWeight: 500 }}>{e.label}</span>
                  <div style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
                    {e.was_central && <span className="badge badge-am">was central</span>}
                    <span style={{ color: tCol, fontSize: 12 }}>{tIcon} {e.trajectory}</span>
                  </div>
                </div>
                <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
                  <span style={{ color: 'var(--t4)', fontSize: 10, width: 60 }}>Then</span>
                  <Bar val={e.entropy_start} color="var(--t3)" />
                  <span style={{ color: 'var(--t3)', fontSize: 10, fontFamily: 'var(--font-mono)', width: 36, textAlign: 'right' }}>
                    {(e.entropy_start * 100).toFixed(0)}%
                  </span>
                </div>
                <div style={{ display: 'flex', gap: 8, alignItems: 'center', marginTop: 3 }}>
                  <span style={{ color: 'var(--t4)', fontSize: 10, width: 60 }}>Now</span>
                  <Bar val={e.entropy_now} color={tCol} />
                  <span style={{ color: tCol, fontSize: 10, fontFamily: 'var(--font-mono)', width: 36, textAlign: 'right' }}>
                    {(e.entropy_now * 100).toFixed(0)}%
                  </span>
                </div>
                {e.state_changes > 0 && (
                  <div style={{ marginTop: 4, color: 'var(--t4)', fontSize: 10 }}>
                    {e.state_changes} recorded state changes
                  </div>
                )}
              </div>
            )
          })}
        </div>
      )}
    </div>
  )
}
