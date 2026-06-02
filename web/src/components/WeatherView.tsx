import { useState, useEffect } from 'react'
import type { WeatherData } from '../types'
import { api } from '../api'

const STATE_ICONS: Record<string, string> = {
  Energetic:  '⚡',
  Calm:       '◇',
  Fading:     '◌',
  Reflective: '◉',
  Turbulent:  '⚠',
}

const STATE_DESCS: Record<string, string[]> = {
  Energetic: [
    'High creative output — ideas are expanding outward',
    'Particle motion is fast and directed',
    'New connections are forming rapidly',
    'Recommended: capture new ideas, branch out',
  ],
  Calm: [
    'Deep focus state — clarity is high',
    'Particle motion is slow and deliberate',
    'Existing nodes are consolidating',
    'Recommended: deep work, architecture, refinement',
  ],
  Fading: [
    'Cognitive depletion — attention is diffusing',
    'Many nodes approaching ghost state',
    'Ghost visibility is increasing',
    'Recommended: rest, gentle review, journaling',
  ],
  Reflective: [
    'Contemplative state — past echoes are surfacing',
    'Ghost nodes are pulsing with residual energy',
    'Temporal archaeology is most productive now',
    'Recommended: review old work, archaeology, lore review',
  ],
  Turbulent: [
    'Cognitive overload — attention is scattered',
    'Too many competing attractors',
    'Particle motion is chaotic',
    'Recommended: focus on one node, eliminate distractions',
  ],
}

function AnimatedOrb({ weather }: { weather: WeatherData }) {
  const r = Math.round(weather.color_r * 255)
  const g = Math.round(weather.color_g * 255)
  const b = Math.round(weather.color_b * 255)
  const col = `rgb(${r},${g},${b})`

  return (
    <div style={{
      width: 120, height: 120, borderRadius: '50%',
      background: `radial-gradient(circle at 35% 35%, rgba(${r},${g},${b},0.9), rgba(${r},${g},${b},0.3))`,
      boxShadow: `0 0 40px rgba(${r},${g},${b},${(weather.intensity * 0.5).toFixed(2)}), 0 0 80px rgba(${r},${g},${b},${(weather.intensity * 0.2).toFixed(2)})`,
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      fontSize: 36,
      animation: weather.state === 'Turbulent' ? 'turbulence 0.5s ease infinite alternate'
               : weather.state === 'Energetic' ? 'pulse-soft 1.2s ease infinite'
               : weather.state === 'Fading'    ? 'fade-pulse 3s ease infinite'
               : weather.state === 'Calm'      ? 'breathe 4s ease infinite'
               : 'drift 6s ease infinite',
      flexShrink: 0,
    }}>
      {STATE_ICONS[weather.state] ?? '◈'}
      <style>{`
        @keyframes pulse-soft { 0%,100%{transform:scale(1)} 50%{transform:scale(1.08)} }
        @keyframes fade-pulse  { 0%,100%{opacity:0.6} 50%{opacity:1} }
        @keyframes breathe     { 0%,100%{transform:scale(0.95)} 50%{transform:scale(1.05)} }
        @keyframes drift       { 0%,100%{transform:scale(1)translateY(0)} 50%{transform:scale(1.02)translateY(-4px)} }
        @keyframes turbulence  { 0%{transform:rotate(-2deg)scale(1)} 100%{transform:rotate(2deg)scale(1.05)} }
      `}</style>
    </div>
  )
}

export default function WeatherView() {
  const [weather, setWeather] = useState<WeatherData | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    api.weather().then(w => { setWeather(w); setLoading(false) })
    const id = setInterval(() => {
      api.weather().then(w => setWeather(w)).catch(() => {})
    }, 15000)
    return () => clearInterval(id)
  }, [])

  if (loading || !weather) return (
    <div style={{ flex:1, display:'flex', alignItems:'center', justifyContent:'center', color:'var(--t4)' }}>
      Deriving ambient state…
    </div>
  )

  const r = Math.round(weather.color_r * 255)
  const g = Math.round(weather.color_g * 255)
  const b = Math.round(weather.color_b * 255)
  const descs = STATE_DESCS[weather.state] ?? []
  const metrics = [
    ['Entropy', weather.avg_entropy, 'Average node drift'],
    ['Ghosts', weather.ghost_ratio, 'Dormant node pressure'],
    ['Focus', Math.min(weather.recent_focus_hours / 4, 1), `${weather.recent_focus_hours.toFixed(1)}h last 24h`],
    ['Weighted', Math.min(weather.weighted_focus_hours / 3, 1), `${weather.weighted_focus_hours.toFixed(1)}h depth-weighted`],
    ['Exploration', weather.exploration, 'Unique nodes touched'],
    ['Deep work', weather.deep_ratio, 'Deep work share'],
  ] as const

  return (
    <div className="col" style={{ height:'100%', gap:12, padding:4 }}>

      {/* Main weather display */}
      <div className="panel" style={{ padding:'24px', display:'flex', gap:28, alignItems:'center', flexShrink:0 }}>
        <AnimatedOrb weather={weather} />

        <div style={{ flex:1 }}>
          <div style={{
            fontSize:11, fontWeight:600, letterSpacing:'0.08em', textTransform:'uppercase',
            color:`rgb(${r},${g},${b})`, marginBottom:6, opacity:0.8,
          }}>
            Cognitive Weather
          </div>
          <div style={{
            fontSize:28, fontWeight:700,
            color:`rgb(${r},${g},${b})`, marginBottom:8, lineHeight:1,
          }}>
            {weather.state}
          </div>
          <div style={{ color:'var(--t2)', fontSize:13, lineHeight:1.6, marginBottom:12 }}>
            {weather.description}
          </div>

          {/* Intensity bar */}
          <div style={{ display:'flex', alignItems:'center', gap:10 }}>
            <span style={{ color:'var(--t4)', fontSize:10, width:60 }}>Intensity</span>
            <div className="bar" style={{ flex:1 }}>
              <div className="bar-fill" style={{
                width:`${(weather.intensity * 100).toFixed(0)}%`,
                background:`rgb(${r},${g},${b})`,
              }} />
            </div>
            <span style={{ color:`rgb(${r},${g},${b})`, fontSize:11, fontFamily:'var(--font-mono)', width:36, textAlign:'right' }}>
              {(weather.intensity * 100).toFixed(0)}%
            </span>
          </div>
        </div>
      </div>

      {/* State description */}
      <div className="panel" style={{ flexShrink:0 }}>
        <div className="sec-head">
          <span style={{ color:`rgb(${r},${g},${b})` }}>◈</span>
          Ambient Conditions
        </div>
        <div style={{ padding:'10px 14px', display:'flex', flexDirection:'column', gap:6 }}>
          {descs.map((d, i) => (
            <div key={i} style={{ display:'flex', gap:8, alignItems:'flex-start' }}>
              <span style={{ color:`rgb(${r},${g},${b})`, fontSize:10, marginTop:2, opacity:0.7 }}>·</span>
              <span style={{ color:'var(--t2)', fontSize:12, lineHeight:1.5 }}>{d}</span>
            </div>
          ))}
        </div>
      </div>

      {/* Live signal breakdown */}
      <div className="panel fill scroll">
        <div className="sec-head">
          <span style={{ color:'var(--t3)' }}>◐</span>
          Live Signals
        </div>
        <div style={{ padding:'10px 14px', display:'flex', flexDirection:'column', gap:10 }}>
          {metrics.map(([label, value, detail]) => {
            const pct = Math.max(0, Math.min(value, 1))
            return (
              <div key={label}>
                <div style={{ display:'flex', alignItems:'baseline', justifyContent:'space-between', marginBottom:5 }}>
                  <span style={{ color:'var(--t2)', fontSize:12, fontWeight:600 }}>{label}</span>
                  <span style={{ color:'var(--t4)', fontSize:10 }}>{detail}</span>
                </div>
                <div className="bar">
                  <div className="bar-fill" style={{
                    width:`${(pct * 100).toFixed(0)}%`,
                    background:`rgb(${r},${g},${b})`,
                    opacity:0.85,
                  }} />
                </div>
              </div>
            )
          })}
        </div>
      </div>
    </div>
  )
}
