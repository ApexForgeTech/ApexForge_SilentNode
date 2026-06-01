import { useState, useEffect } from 'react'
import type { SoulData } from '../types'
import { api } from '../api'

const GLOW_PATTERNS: Record<string, string> = {
  pulse:   'Regular heartbeat — steady & rhythmic',
  radiate: 'Continuous outward radiation — expansive',
  breathe: 'Slow breathing — contemplative',
  flicker: 'Chaotic sparks — unstable energy',
  steady:  'Constant calm — mature & grounded',
}
const PARTICLE_STYLES: Record<string, string> = {
  aggressive:  'Fast, sharp, high-velocity — builder mode',
  crystalline: 'Slow, geometric, precise — structured thinking',
  fluid:       'Smooth, flowing, curved — creative flow',
  organic:     'Random, biological, warm — natural growth',
}

function SoulCard({ soul }: { soul: SoulData }) {
  const pc = soul.primary_color
  const sc = soul.secondary_color
  const primaryCss   = `rgb(${Math.round(pc[0]*255)},${Math.round(pc[1]*255)},${Math.round(pc[2]*255)})`
  const secondaryCss = `rgb(${Math.round(sc[0]*255)},${Math.round(sc[1]*255)},${Math.round(sc[2]*255)})`

  return (
    <div style={{
      background: 'var(--surface)',
      border: '1px solid var(--line)',
      borderRadius: 8,
      overflow: 'hidden',
      display: 'flex', flexDirection: 'column',
    }}>
      {/* Color bar */}
      <div style={{
        height: 4,
        background: `linear-gradient(90deg, ${primaryCss}, ${secondaryCss})`,
      }} />

      {/* Header */}
      <div style={{ padding: '12px 14px', borderBottom: '1px solid var(--line)' }}>
        <div style={{ display: 'flex', gap: 10, alignItems: 'flex-start' }}>
          {/* Soul orb */}
          <div style={{
            width: 36, height: 36, borderRadius: '50%', flexShrink: 0,
            background: `radial-gradient(circle at 35% 35%, ${primaryCss}, ${secondaryCss})`,
            boxShadow: `0 0 16px ${primaryCss}44`,
          }} />
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ color: 'var(--t1)', fontSize: 13, fontWeight: 600, lineHeight: 1.3, marginBottom: 3 }}>
              {soul.content_preview}
            </div>
            <div style={{ display: 'flex', gap: 5, flexWrap: 'wrap' }}>
              <span className="badge badge-mt" style={{ fontSize: 9 }}>{soul.particle_style}</span>
              <span className="badge badge-mt" style={{ fontSize: 9 }}>{soul.glow_pattern}</span>
            </div>
          </div>
        </div>
      </div>

      {/* Metrics */}
      <div style={{ padding: '10px 14px', display: 'flex', flexDirection: 'column', gap: 6 }}>
        {[
          { label: 'Activity',  val: soul.activity_level, col: primaryCss },
          { label: 'Maturity',  val: soul.maturity,       col: secondaryCss },
          { label: 'Social',    val: soul.social_density,  col: 'var(--sky)' },
        ].map(({ label, val, col }) => (
          <div key={label} style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <span style={{ color: 'var(--t4)', fontSize: 10, width: 52, flexShrink: 0 }}>{label}</span>
            <div className="bar fill">
              <div className="bar-fill" style={{ width: `${(val * 100).toFixed(0)}%`, background: col }} />
            </div>
            <span style={{ color: col, fontSize: 10, fontFamily: 'var(--font-mono)', width: 30, textAlign: 'right', flexShrink: 0 }}>
              {(val * 100).toFixed(0)}%
            </span>
          </div>
        ))}
      </div>

      {/* Description */}
      <div style={{ padding: '0 14px 12px', display: 'flex', flexDirection: 'column', gap: 4 }}>
        <div style={{ color: 'var(--t3)', fontSize: 10, lineHeight: 1.5 }}>
          {PARTICLE_STYLES[soul.particle_style]}
        </div>
        <div style={{ color: 'var(--t4)', fontSize: 10, lineHeight: 1.5 }}>
          {GLOW_PATTERNS[soul.glow_pattern]}
        </div>
      </div>
    </div>
  )
}

export default function SoulsView() {
  const [souls,   setSouls]   = useState<SoulData[]>([])
  const [loading, setLoading] = useState(true)
  const [filter,  setFilter]  = useState<string>('')

  useEffect(() => {
    api.souls()
      .then(s => { setSouls(s); setLoading(false) })
      .catch(() => setLoading(false))
  }, [])

  const filtered = souls.filter(s =>
    !filter || s.content_preview.toLowerCase().includes(filter.toLowerCase())
      || s.particle_style === filter || s.glow_pattern === filter
  )

  if (loading) return (
    <div style={{ flex:1, display:'flex', alignItems:'center', justifyContent:'center', color:'var(--t4)' }}>
      Deriving project souls…
    </div>
  )

  return (
    <div className="col" style={{ height: '100%', gap: 10 }}>

      {/* Header */}
      <div style={{ display: 'flex', gap: 10, alignItems: 'center', flexShrink: 0 }}>
        <input type="text" placeholder="Search projects…" value={filter}
          onChange={e => setFilter(e.target.value)} style={{ flex: 1, fontSize: 12 }} />
        <span style={{ color: 'var(--t4)', fontSize: 11 }}>{filtered.length} souls</span>
        <button className="btn-sm" onClick={() => {
          setLoading(true)
          api.souls().then(s => { setSouls(s); setLoading(false) })
        }}>↺</button>
      </div>

      {/* Intro */}
      {souls.length === 0 && (
        <div className="panel" style={{ padding: '20px 16px' }}>
          <div style={{ color: 'var(--t3)', fontSize: 12, lineHeight: 1.7 }}>
            <div style={{ marginBottom: 8 }}>
              No project souls yet. Project Souls emerge from Project and World type nodes.
            </div>
            <div style={{ color: 'var(--t4)', fontSize: 11 }}>
              Each project develops its own visual identity — a unique particle style, glow pattern, and color palette
              derived from the project's content and activity history. Add Project nodes to generate souls.
            </div>
          </div>
        </div>
      )}

      {/* Grid */}
      <div className="scroll fill" style={{
        display: 'grid',
        gridTemplateColumns: 'repeat(auto-fill, minmax(260px, 1fr))',
        gap: 10,
        alignContent: 'start',
      }}>
        {filtered.map(s => <SoulCard key={s.project_id} soul={s} />)}
      </div>

      {/* Legend */}
      {souls.length > 0 && (
        <div className="panel" style={{ padding: '10px 14px', flexShrink: 0 }}>
          <div style={{ display: 'flex', gap: 20, flexWrap: 'wrap' }}>
            <div>
              <div style={{ color: 'var(--t4)', fontSize: 9, letterSpacing: '0.08em', textTransform: 'uppercase', marginBottom: 5 }}>
                Particle Styles
              </div>
              <div style={{ display: 'flex', gap: 5, flexWrap: 'wrap' }}>
                {Object.keys(PARTICLE_STYLES).map(s => (
                  <button key={s} className={`btn-xs${filter === s ? ' btn-primary' : ''}`}
                    onClick={() => setFilter(filter === s ? '' : s)}>{s}</button>
                ))}
              </div>
            </div>
            <div>
              <div style={{ color: 'var(--t4)', fontSize: 9, letterSpacing: '0.08em', textTransform: 'uppercase', marginBottom: 5 }}>
                Glow Patterns
              </div>
              <div style={{ display: 'flex', gap: 5, flexWrap: 'wrap' }}>
                {Object.keys(GLOW_PATTERNS).map(g => (
                  <button key={g} className={`btn-xs${filter === g ? ' btn-primary' : ''}`}
                    onClick={() => setFilter(filter === g ? '' : g)}>{g}</button>
                ))}
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
