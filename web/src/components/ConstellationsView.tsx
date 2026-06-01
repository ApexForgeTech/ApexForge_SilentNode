import { useEffect, useState } from 'react'
import type { ConstellationData } from '../types'
import { api } from '../api'

export default function ConstellationsView() {
  const [items, setItems] = useState<ConstellationData[]>([])

  useEffect(() => {
    api.constellations().then(setItems).catch(() => {})
  }, [])

  return (
    <div className="constellation-grid">
      {items.length === 0 && <div className="empty-state">No life constellations yet.</div>}
      {items.map(c => (
        <div key={c.id} className="constellation-card">
          <div className="constellation-orbit">
            {c.member_previews.slice(0, 6).map((_, i) => <span key={i} style={{ transform: `rotate(${i * 60}deg) translateX(${34 + c.emotional_weight * 22}px)` }} />)}
            <strong>{Math.round(c.emotional_weight * 100)}</strong>
          </div>
          <div className="constellation-copy">
            <small>{c.kind}</small>
            <h3>{c.label}</h3>
            <p>{c.member_previews.join(' · ')}</p>
          </div>
        </div>
      ))}
    </div>
  )
}
