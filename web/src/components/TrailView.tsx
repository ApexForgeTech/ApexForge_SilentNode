import { useState, useEffect } from 'react'
import type { TrailEvent } from '../types'
import { api } from '../api'

const DEPTH_COLORS: Record<string, string> = {
  Glance:   'var(--t4)',
  Read:     'var(--sky)',
  Edit:     'var(--amber)',
  DeepWork: 'var(--green)',
}
const DEPTH_ICONS: Record<string, string> = {
  Glance:   '◌',
  Read:     '◎',
  Edit:     '◆',
  DeepWork: '⬟',
}

function durationLabel(s: number): string {
  if (s < 60) return `${s.toFixed(0)}s`
  if (s < 3600) return `${(s / 60).toFixed(0)}m`
  return `${(s / 3600).toFixed(1)}h`
}

function timeAgo(ts: string): string {
  const diff = Date.now() - new Date(ts).getTime()
  const m = Math.floor(diff / 60000)
  if (m < 1) return 'just now'
  if (m < 60) return `${m}m ago`
  const h = Math.floor(m / 60)
  if (h < 24) return `${h}h ago`
  return `${Math.floor(h / 24)}d ago`
}

export default function TrailView() {
  const [trail, setTrail]   = useState<TrailEvent[]>([])
  const [hours, setHours]   = useState(48)
  const [loading, setLoading] = useState(true)
  const [selected, setSelected] = useState<TrailEvent | null>(null)

  useEffect(() => {
    setLoading(true)
    api.trail(hours)
      .then(t => { setTrail(t); setSelected(t[0] ?? null) })
      .catch(() => {})
      .finally(() => setLoading(false))
  }, [hours])

  const totalTime = trail.reduce((s, e) => s + e.duration_seconds, 0)
  const deepWorkTime = trail.filter(e => e.depth === 'DeepWork').reduce((s, e) => s + e.duration_seconds, 0)

  return (
    <div className="split">
      {/* Left: trail list */}
      <div className="split-list panel">
        <div className="sec-head">
          <span style={{ color: 'var(--sky)' }}>◎</span>
          Focus Trail
          <span style={{ marginLeft: 'auto', color: 'var(--t4)', fontSize: 10 }}>{trail.length}</span>
        </div>

        {/* Window selector */}
        <div style={{ padding: '6px 10px', borderBottom: '1px solid var(--line)', display: 'flex', gap: 4 }}>
          {[12, 24, 48, 168].map(h => (
            <button key={h}
              className={`btn-xs${hours === h ? ' btn-primary' : ''}`}
              onClick={() => setHours(h)}
            >
              {h < 24 ? `${h}h` : `${h/24}d`}
            </button>
          ))}
        </div>

        {/* Summary row */}
        {trail.length > 0 && (
          <div style={{
            padding: '8px 12px', display: 'flex', gap: 12,
            borderBottom: '1px solid var(--line)',
            background: 'rgba(255,255,255,0.02)',
          }}>
            <div style={{ textAlign: 'center' }}>
              <div style={{ fontSize: 14, fontWeight: 700, color: 'var(--sky)', fontFamily: 'var(--font-mono)' }}>
                {durationLabel(totalTime)}
              </div>
              <div style={{ fontSize: 9, color: 'var(--t4)' }}>total</div>
            </div>
            <div style={{ textAlign: 'center' }}>
              <div style={{ fontSize: 14, fontWeight: 700, color: 'var(--green)', fontFamily: 'var(--font-mono)' }}>
                {durationLabel(deepWorkTime)}
              </div>
              <div style={{ fontSize: 9, color: 'var(--t4)' }}>deep work</div>
            </div>
            <div style={{ textAlign: 'center' }}>
              <div style={{ fontSize: 14, fontWeight: 700, color: 'var(--lavender-text)', fontFamily: 'var(--font-mono)' }}>
                {new Set(trail.map(e => e.node_id)).size}
              </div>
              <div style={{ fontSize: 9, color: 'var(--t4)' }}>nodes</div>
            </div>
          </div>
        )}

        <div className="scroll fill">
          {loading && <div style={{ padding: 16, color: 'var(--t4)', fontSize: 11 }}>Loading trail…</div>}
          {!loading && trail.length === 0 && (
            <div style={{ padding: 16, color: 'var(--t4)', fontSize: 11 }}>
              No focus events in the last {hours < 24 ? `${hours}h` : `${hours/24}d`}
            </div>
          )}
          {trail.map((ev, i) => {
            const col  = DEPTH_COLORS[ev.depth] ?? 'var(--t3)'
            const icon = DEPTH_ICONS[ev.depth] ?? '◌'
            const isSel = selected === ev
            return (
              <div key={i}
                className="list-row"
                style={isSel ? {
                  background: 'rgba(167,139,250,0.07)',
                  borderLeft: '2px solid rgba(167,139,250,0.5)',
                } : {}}
                onClick={() => setSelected(ev)}
              >
                <span style={{ color: col, fontSize: 12, marginTop: 1, flexShrink: 0 }}>{icon}</span>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{
                    fontSize: 11, color: 'var(--t1)',
                    overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                    marginBottom: 2,
                  }}>
                    {ev.content_preview || ev.node_id.slice(0, 12)}
                  </div>
                  <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
                    <span style={{ fontSize: 9, color: col, fontWeight: 600, letterSpacing: '0.04em' }}>
                      {ev.depth}
                    </span>
                    <span style={{ fontSize: 9, color: 'var(--t4)', fontFamily: 'var(--font-mono)' }}>
                      {durationLabel(ev.duration_seconds)}
                    </span>
                    <span style={{ fontSize: 9, color: 'var(--t4)', marginLeft: 'auto' }}>
                      {timeAgo(ev.timestamp)}
                    </span>
                  </div>
                </div>
                <button
                  className="btn-xs btn-danger"
                  style={{ padding: '1px 5px', fontSize: 9, flexShrink: 0, opacity: 0.6 }}
                  onClick={async e => {
                    e.stopPropagation()
                    await fetch(`/api/focus/${ev.session_id}`, { method: 'DELETE' })
                    setTrail(prev => prev.filter(x => x.session_id !== ev.session_id))
                    if (selected === ev) setSelected(null)
                  }}
                >✕</button>
              </div>
            )
          })}
        </div>
      </div>

      {/* Right: detail + heatmap */}
      <div className="split-detail" style={{ gap: 10 }}>

        {/* Selected event detail */}
        {selected && (
          <div className="panel anim-in" style={{ flexShrink: 0 }}>
            <div className="sec-head">
              <span style={{ color: DEPTH_COLORS[selected.depth] ?? 'var(--sky)' }}>
                {DEPTH_ICONS[selected.depth] ?? '◎'}
              </span>
              Focus Session
            </div>
            <div style={{ padding: '12px 14px', display: 'flex', flexDirection: 'column', gap: 8 }}>
              <div style={{ color: 'var(--t1)', fontSize: 13, lineHeight: 1.5 }}>
                {selected.content_preview}
              </div>
              <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap' }}>
                {[
                  ['Depth',    selected.depth,                      DEPTH_COLORS[selected.depth] ?? 'var(--t3)'],
                  ['Duration', durationLabel(selected.duration_seconds), 'var(--sky)'],
                  ['Time',     new Date(selected.timestamp).toLocaleString(), 'var(--t3)'],
                ].map(([l, v, c]) => (
                  <div key={String(l)}>
                    <div style={{ fontSize: 9, color: 'var(--t4)', letterSpacing: '0.06em', marginBottom: 2 }}>
                      {l}
                    </div>
                    <div style={{ fontSize: 12, color: String(c), fontWeight: 500 }}>{v}</div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}

        {/* Depth breakdown */}
        <div className="panel fill scroll">
          <div className="sec-head">
            <span style={{ color: 'var(--amber)' }}>◐</span>
            Depth Distribution
          </div>
          <div style={{ padding: '12px 14px', display: 'flex', flexDirection: 'column', gap: 10 }}>
            {(['DeepWork', 'Edit', 'Read', 'Glance'] as const).map(depth => {
              const events = trail.filter(e => e.depth === depth)
              const time = events.reduce((s, e) => s + e.duration_seconds, 0)
              const pct = totalTime > 0 ? (time / totalTime) * 100 : 0
              const col = DEPTH_COLORS[depth]
              return (
                <div key={depth}>
                  <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 4 }}>
                    <div style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
                      <span style={{ color: col }}>{DEPTH_ICONS[depth]}</span>
                      <span style={{ color: 'var(--t2)', fontSize: 12 }}>{depth}</span>
                      <span style={{ color: 'var(--t4)', fontSize: 10 }}>{events.length} sessions</span>
                    </div>
                    <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
                      <span style={{ color: col, fontSize: 11, fontFamily: 'var(--font-mono)' }}>
                        {durationLabel(time)}
                      </span>
                      <span style={{ color: 'var(--t4)', fontSize: 10 }}>{pct.toFixed(0)}%</span>
                    </div>
                  </div>
                  <div className="bar">
                    <div className="bar-fill" style={{ width: `${pct}%`, background: col }} />
                  </div>
                </div>
              )
            })}
          </div>

          {/* Top nodes */}
          {trail.length > 0 && (() => {
            const byNode = new Map<string, { preview: string; time: number; count: number }>()
            trail.forEach(e => {
              const cur = byNode.get(e.node_id) ?? { preview: e.content_preview, time: 0, count: 0 }
              cur.time += e.duration_seconds
              cur.count++
              byNode.set(e.node_id, cur)
            })
            const sorted = [...byNode.entries()]
              .sort((a, b) => b[1].time - a[1].time)
              .slice(0, 8)
            return (
              <div style={{ marginTop: 12, paddingTop: 12, borderTop: '1px solid var(--line)' }}>
                <div style={{ fontSize: 9, color: 'var(--t4)', letterSpacing: '0.08em', textTransform: 'uppercase', marginBottom: 8 }}>
                  Most Focused Nodes
                </div>
                {sorted.map(([id, data], i) => (
                  <div key={id} style={{
                    display: 'flex', alignItems: 'center', gap: 8,
                    padding: '5px 0', borderBottom: '1px solid rgba(255,255,255,0.03)',
                  }}>
                    <span style={{ color: 'var(--t4)', fontSize: 9, width: 16, textAlign: 'right', flexShrink: 0 }}>
                      #{i + 1}
                    </span>
                    <span style={{ flex: 1, fontSize: 11, color: 'var(--t2)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                      {data.preview || id.slice(0, 12)}
                    </span>
                    <span style={{ color: 'var(--sky)', fontSize: 10, fontFamily: 'var(--font-mono)', flexShrink: 0 }}>
                      {durationLabel(data.time)}
                    </span>
                    <span style={{ color: 'var(--t4)', fontSize: 9, flexShrink: 0 }}>{data.count}×</span>
                  </div>
                ))}
              </div>
            )
          })()}
        </div>
      </div>
    </div>
  )
}
