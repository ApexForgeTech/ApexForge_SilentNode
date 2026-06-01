import { useEffect, useMemo, useState } from 'react'
import { api } from '../api'
import type { VisionCoverageData } from '../types'

function pct(v: number) {
  return `${Math.round(v * 100)}%`
}

function tone(status: string) {
  if (status === 'live') return 'var(--green)'
  if (status === 'partial') return 'var(--amber)'
  return 'var(--t3)'
}

export default function VisionCoverageView() {
  const [coverage, setCoverage] = useState<VisionCoverageData | null>(null)
  const [filter, setFilter] = useState<'all' | 'live' | 'partial' | 'stub'>('all')
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    api.visionCoverage()
      .then(data => { setCoverage(data); setError(null) })
      .catch(e => setError(e.message))
  }, [])

  const items = useMemo(() => {
    if (!coverage) return []
    return coverage.items.filter(item => filter === 'all' || item.status === filter)
  }, [coverage, filter])

  if (error) {
    return <div className="empty-state">Vision coverage unavailable: {error}</div>
  }

  if (!coverage) {
    return <div className="empty-state">Reading vision coverage...</div>
  }

  return (
    <div className="vision-coverage">
      <section className="sn-panel">
        <div className="sn-panel-head">
          <span>Vision Coverage</span>
          <b>{pct(coverage.completion_ratio)}</b>
        </div>
        <p className="muted-copy">{coverage.summary}</p>
        <p className="muted-copy">{coverage.generated_from}</p>
        <div className="vision-filters">
          {(['all', 'live', 'partial', 'stub'] as const).map(item => (
            <button key={item} className={filter === item ? 'active' : ''} onClick={() => setFilter(item)}>
              {item}
            </button>
          ))}
        </div>
      </section>

      <div className="vision-grid">
        {items.map(item => (
          <article className="vision-card" key={item.concept}>
            <div className="vision-card-head">
              <div>
                <strong>{item.concept}</strong>
                <span>{item.area}</span>
              </div>
              <em style={{ color: tone(item.status) }}>{item.status} · {pct(item.confidence)}</em>
            </div>
            <div className="vision-evidence">
              <div>
                <small>Backend</small>
                {item.backend_evidence.map(ev => <span key={ev}>{ev}</span>)}
              </div>
              <div>
                <small>Web</small>
                {item.web_evidence.map(ev => <span key={ev}>{ev}</span>)}
              </div>
            </div>
            <p>{item.gap}</p>
          </article>
        ))}
      </div>
    </div>
  )
}
