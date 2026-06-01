import { useState } from 'react'
import type { DayReconstructionData, DayComparisonData } from '../types'
import { api } from '../api'
import { toast } from './Toast'

const AURA_COLORS: Record<string, string> = {
  Energetic:  '#40c878',
  Calm:       '#38bdf8',
  Fading:     '#8b87aa',
  Reflective: '#a78bfa',
  Turbulent:  '#f87171',
}

function AuraOrb({ state, intensity }: { state: string; intensity: number }) {
  const col = AURA_COLORS[state] ?? '#8b87aa'
  return (
    <div style={{
      width: 64, height: 64, borderRadius: '50%',
      background: `radial-gradient(circle at 35% 35%, ${col}, ${col}44)`,
      boxShadow: `0 0 20px ${col}${Math.round(intensity * 80).toString(16).padStart(2, '0')}`,
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      fontSize: 20, flexShrink: 0,
      border: `1px solid ${col}44`,
    }}>
      {state === 'Energetic' ? '⚡' : state === 'Calm' ? '◇' : state === 'Fading' ? '◌' : state === 'Reflective' ? '◉' : '⚠'}
    </div>
  )
}

export default function ReconstructionView() {
  const [mode, setMode]           = useState<'day' | 'compare'>('day')
  const [dateA, setDateA]         = useState(() => new Date().toISOString().slice(0, 10))
  const [dateB, setDateB]         = useState(() => {
    const d = new Date(); d.setDate(d.getDate() - 1); return d.toISOString().slice(0, 10)
  })
  const [dayData, setDayData]     = useState<DayReconstructionData | null>(null)
  const [cmpData, setCmpData]     = useState<DayComparisonData | null>(null)
  const [loading, setLoading]     = useState(false)
  const [snapshotLoading, setSnapshotLoading] = useState(false)
  const [snapshotCount, setSnapshotCount]     = useState<number | null>(null)

  async function reconstruct() {
    setLoading(true); setDayData(null); setCmpData(null)
    try {
      if (mode === 'day') {
        const d = await api.reconstructDay(dateA)
        setDayData(d)
      } else {
        const d = await api.compareDays(dateB, dateA)
        setCmpData(d)
      }
    } catch (e) { toast(String(e), 'error') }
    setLoading(false)
  }

  async function takeSnapshot() {
    setSnapshotLoading(true)
    try {
      const r = await api.takeSnapshot()
      setSnapshotCount(r.total_snapshots)
      toast(`Snapshot taken — ${r.total_snapshots} total`, 'success')
    } catch (e) { toast(String(e), 'error') }
    setSnapshotLoading(false)
  }

  function secondsLabel(s: number) {
    if (s < 60) return `${s.toFixed(0)}s`
    if (s < 3600) return `${(s/60).toFixed(0)}m`
    return `${(s/3600).toFixed(1)}h`
  }

  return (
    <div className="col" style={{ height: '100%', gap: 10 }}>

      {/* Controls */}
      <div className="panel" style={{ flexShrink: 0, padding: '10px 14px' }}>
        <div style={{ display: 'flex', gap: 8, alignItems: 'center', flexWrap: 'wrap' }}>
          {/* Mode toggle */}
          <div className="tabs" style={{ flexShrink: 0 }}>
            <button className={`tab${mode === 'day' ? ' active' : ''}`} onClick={() => setMode('day')}>
              Single Day
            </button>
            <button className={`tab${mode === 'compare' ? ' active' : ''}`} onClick={() => setMode('compare')}>
              Compare
            </button>
          </div>

          {mode === 'compare' && (
            <>
              <input type="date" value={dateB} onChange={e => setDateB(e.target.value)}
                style={{ width: 130, fontFamily: 'var(--font-mono)', fontSize: 11 }} />
              <span style={{ color: 'var(--t4)', fontSize: 12 }}>→</span>
            </>
          )}
          <input type="date" value={dateA} onChange={e => setDateA(e.target.value)}
            style={{ width: 130, fontFamily: 'var(--font-mono)', fontSize: 11 }} />

          <button className="btn-primary btn-sm" onClick={reconstruct} disabled={loading}>
            {loading ? 'Reconstructing…' : mode === 'day' ? 'Reconstruct Day' : 'Compare Days'}
          </button>

          <div style={{ flex: 1 }} />

          {/* Snapshot button */}
          <button
            className="btn-sm btn-amber"
            onClick={takeSnapshot}
            disabled={snapshotLoading}
            title="Take a temporal snapshot of all nodes now"
          >
            {snapshotLoading ? 'Snapshotting…' : '⬡ Snapshot Now'}
          </button>
          {snapshotCount !== null && (
            <span style={{ fontSize: 10, color: 'var(--t4)', fontFamily: 'var(--font-mono)' }}>
              {snapshotCount} total
            </span>
          )}
        </div>
        <div style={{ marginTop: 6, fontSize: 10, color: 'var(--t4)' }}>
          Temporal snapshots let you reconstruct exact cognitive states on any past day.
          Take snapshots regularly to increase reconstruction fidelity.
        </div>
      </div>

      {/* Day reconstruction result */}
      {dayData && (
        <div className="col fill anim-glow-in" style={{ gap: 10, overflow: 'hidden' }}>
          {/* Header card */}
          <div className="panel" style={{ flexShrink: 0 }}>
            <div className="sec-head">
              <span style={{ color: 'var(--lavender-text)' }}>◈</span>
              {dayData.date}
            </div>
            <div style={{ padding: '14px', display: 'flex', gap: 20, alignItems: 'center', flexWrap: 'wrap' }}>
              <AuraOrb state={dayData.aura_state} intensity={dayData.aura_intensity} />
              <div style={{ display: 'flex', gap: 24, flexWrap: 'wrap' }}>
                {[
                  ['Nodes Touched',    dayData.nodes_touched,                         'var(--sky)'],
                  ['Focus Time',       secondsLabel(dayData.total_focus_seconds),      'var(--green)'],
                  ['Focus Events',     dayData.focus_events_count,                     'var(--amber)'],
                  ['Journal Entries',  dayData.journal_entries_count,                  'var(--lavender-text)'],
                ].map(([l, v, c]) => (
                  <div key={String(l)} style={{ textAlign: 'center' }}>
                    <div style={{ fontSize: 20, fontWeight: 700, color: String(c), fontFamily: 'var(--font-mono)' }}>
                      {v}
                    </div>
                    <div style={{ fontSize: 9, color: 'var(--t4)', textTransform: 'uppercase', letterSpacing: '0.06em' }}>
                      {l}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>

          {/* Aura + dominant node */}
          <div style={{ display: 'flex', gap: 10, flexShrink: 0 }}>
            <div className="panel" style={{ flex: 1, padding: '12px 14px' }}>
              <div style={{ fontSize: 9, color: 'var(--t4)', textTransform: 'uppercase', letterSpacing: '0.06em', marginBottom: 6 }}>
                Ambient Aura
              </div>
              <div style={{
                fontSize: 18, fontWeight: 700,
                color: AURA_COLORS[dayData.aura_state] ?? 'var(--t2)',
                marginBottom: 4,
              }}>
                {dayData.aura_state}
              </div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <div className="bar fill">
                  <div className="bar-fill" style={{
                    width: `${(dayData.aura_intensity * 100).toFixed(0)}%`,
                    background: AURA_COLORS[dayData.aura_state] ?? 'var(--t3)',
                  }} />
                </div>
                <span style={{ color: 'var(--t3)', fontSize: 10, fontFamily: 'var(--font-mono)' }}>
                  {(dayData.aura_intensity * 100).toFixed(0)}%
                </span>
              </div>
            </div>

            {dayData.dominant_node_id && (
              <div className="panel" style={{ flex: 1, padding: '12px 14px' }}>
                <div style={{ fontSize: 9, color: 'var(--t4)', textTransform: 'uppercase', letterSpacing: '0.06em', marginBottom: 6 }}>
                  Dominant Node
                </div>
                <div style={{ color: 'var(--t1)', fontSize: 12, lineHeight: 1.5 }}>
                  {dayData.dominant_node_preview || dayData.dominant_node_id.slice(0, 16)}
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Day comparison result */}
      {cmpData && (
        <div className="panel fill scroll anim-glow-in">
          <div className="sec-head">
            <span style={{ color: 'var(--sky)' }}>◈</span>
            {cmpData.day_a} → {cmpData.day_b}
            <span style={{ marginLeft: 'auto', color: 'var(--t4)', fontSize: 10 }}>
              {cmpData.entries.length} dimensions
            </span>
          </div>
          <div style={{ padding: '8px 0' }}>
            {cmpData.entries.map((entry, i) => (
              <div key={i} style={{
                display: 'flex', alignItems: 'center', gap: 12,
                padding: '9px 14px', borderBottom: '1px solid rgba(255,255,255,0.04)',
              }}>
                <div style={{ width: 160, fontSize: 11, color: 'var(--t3)', flexShrink: 0 }}>
                  {entry.field.replace(/_/g, ' ')}
                </div>
                <div style={{ flex: 1, display: 'flex', gap: 16, alignItems: 'center' }}>
                  <span style={{
                    fontSize: 12, fontFamily: 'var(--font-mono)',
                    color: entry.day_a_value === '—' ? 'var(--t4)' : 'var(--t2)',
                    minWidth: 60, textAlign: 'right',
                  }}>
                    {entry.day_a_value}
                  </span>
                  <span style={{ color: 'var(--t4)', fontSize: 10 }}>→</span>
                  <span style={{
                    fontSize: 12, fontFamily: 'var(--font-mono)',
                    color: entry.day_b_value.startsWith('+') ? 'var(--green)'
                         : entry.day_b_value.startsWith('-') ? 'var(--red)' : 'var(--sky)',
                  }}>
                    {entry.day_b_value}
                  </span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Empty state */}
      {!dayData && !cmpData && !loading && (
        <div className="panel fill" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
          <div style={{ textAlign: 'center', color: 'var(--t4)' }}>
            <div style={{ fontSize: 32, marginBottom: 12, opacity: 0.4 }}>⟲</div>
            <div style={{ fontSize: 12, marginBottom: 6 }}>Memory Reconstruction</div>
            <div style={{ fontSize: 11, color: 'var(--t4)', maxWidth: 300, lineHeight: 1.6 }}>
              Choose a date and reconstruct what your cognitive space looked like on that day.
              Compare two days to see what shifted.
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
