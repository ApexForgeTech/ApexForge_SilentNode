import { useEffect, useState } from 'react'
import type { SystemModeData, AtmosphereData } from '../types'
import { api } from '../api'

export default function SystemModesView({ onDeepFocus }: { onDeepFocus: () => void }) {
  const [modes, setModes] = useState<SystemModeData[]>([])
  const [atmospheres, setAtmospheres] = useState<AtmosphereData[]>([])
  const [savingMode, setSavingMode] = useState<string | null>(null)

  function refreshModes() {
    return api.modes().then(setModes).catch(() => {})
  }

  useEffect(() => {
    refreshModes()
    api.atmospheres().then(setAtmospheres).catch(() => {})
  }, [])

  async function chooseMode(mode: string | null) {
    setSavingMode(mode ?? 'auto')
    try {
      await api.setMode(mode)
      await refreshModes()
    } finally {
      setSavingMode(null)
    }
  }

  const source = modes.find(m => m.active)?.source ?? 'inferred'

  return (
    <div className="mode-grid">
      <section className="panel mode-panel">
        <div className="sec-head">
          <span>System Modes</span>
          <em>{source}</em>
        </div>
        <div className="mode-actions">
          <button className={source === 'inferred' ? 'active' : ''} onClick={() => chooseMode(null)} disabled={savingMode !== null}>
            Auto
          </button>
        </div>
        <div className="mode-list">
          {modes.map(mode => (
            <button
              key={mode.id}
              className={mode.active ? 'mode-card active' : 'mode-card'}
              onClick={() => chooseMode(mode.id)}
              disabled={savingMode !== null}
            >
              <div>
                <strong>{mode.label}</strong>
                <span>{mode.description}</span>
              </div>
              <em>{savingMode === mode.id ? '...' : `${Math.round(mode.intensity * 100)}%`}</em>
            </button>
          ))}
        </div>
        <button className="btn-primary" onClick={onDeepFocus}>Enter Deep Focus</button>
      </section>

      <section className="panel mode-panel">
        <div className="sec-head">Memory Atmospheres</div>
        <div className="atmosphere-list">
          {atmospheres.map(a => (
            <div key={a.id} className="atmosphere-row">
              <i style={{ background: a.color }} />
              <div>
                <strong>{a.label}</strong>
                <span>{a.audio_signature}</span>
                <small>{a.visual_signature}</small>
              </div>
              <em>{Math.round(a.intensity * 100)}%</em>
            </div>
          ))}
        </div>
      </section>
    </div>
  )
}
