import { useState, useEffect } from 'react'
import type { HealthReport, PageRankEntry, BridgeEdge, SNode } from '../types'
import { NODE_COLORS } from '../types'
import { api } from '../api'

function Bar({ val, color = 'var(--cyan)' }: { val: number; color?: string }) {
  return (
    <div className="prog-track" style={{ flex: 1 }}>
      <div className="prog-fill" style={{ width: `${(val*100).toFixed(0)}%`, background: color }} />
    </div>
  )
}

interface Props { nodes: SNode[] }

export default function AnalyticsView({ nodes }: Props) {
  const [health,   setHealth]   = useState<HealthReport | null>(null)
  const [pagerank, setPagerank] = useState<PageRankEntry[]>([])
  const [bridges,  setBridges]  = useState<BridgeEdge[]>([])
  const [loading,  setLoading]  = useState(true)

  const nodeMap = new Map(nodes.map(n => [n.id, n]))

  useEffect(() => {
    setLoading(true)
    Promise.allSettled([
      api.health(),
      api.pagerank(20),
      api.bridges(),
    ]).then(([h, p, b]) => {
      if (h.status === 'fulfilled') setHealth(h.value)
      if (p.status === 'fulfilled') setPagerank(p.value)
      if (b.status === 'fulfilled') setBridges(b.value)
      setLoading(false)
    })
  }, [])

  const healthColor = health
    ? health.score > 0.75 ? 'var(--emerald)' : health.score > 0.5 ? 'var(--amber)' : 'var(--crimson)'
    : 'var(--text-muted)'

  function download(content: string, filename: string, mime: string) {
    const a = document.createElement('a')
    a.href = URL.createObjectURL(new Blob([content], { type: mime }))
    a.download = filename; a.click()
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', gap: 10 }}>

      {/* Health strip */}
      <div className="glass" style={{ padding: '12px 16px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 16, flexWrap: 'wrap' }}>
          <div>
            <div style={{ fontFamily: 'var(--font-head)', fontSize: 9, color: 'var(--text-muted)', letterSpacing: '0.1em', marginBottom: 4 }}>
              GRAPH HEALTH
            </div>
            {health ? (
              <div style={{ fontFamily: 'var(--font-head)', fontSize: 22, color: healthColor, fontWeight: 900 }}>
                {(health.score * 100).toFixed(0)}%
                <span style={{ fontSize: 10, marginLeft: 8, color: healthColor, fontWeight: 400 }}>{health.label}</span>
              </div>
            ) : (
              <div style={{ color: 'var(--text-muted)', fontSize: 11 }}>{loading ? 'Loading…' : '—'}</div>
            )}
          </div>
          {health && (
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, auto)', gap: '4px 16px', flex: 1 }}>
              {[
                { l: 'Density',    v: health.density,       fmt: (x: number) => x.toFixed(4), col: 'var(--cyan)' },
                { l: 'Activity',   v: health.activity_rate, fmt: (x: number) => `${(x*100).toFixed(0)}%`, col: 'var(--emerald)' },
                { l: 'Decay',      v: health.decay_ratio,   fmt: (x: number) => `${(x*100).toFixed(0)}%`, col: health.decay_ratio > 0.3 ? 'var(--crimson)' : 'var(--text-secondary)' },
                { l: 'Avg Entropy', v: health.avg_entropy,  fmt: (x: number) => x.toFixed(3), col: health.avg_entropy > 0.5 ? 'var(--amber)' : 'var(--text-secondary)' },
                { l: 'Components', v: health.component_count, fmt: (x: number) => String(x), col: 'var(--text-secondary)' },
                { l: 'Bridges',    v: health.bridge_count,  fmt: (x: number) => String(x), col: health.bridge_count > 3 ? 'var(--amber)' : 'var(--text-secondary)' },
              ].map(({ l, v, fmt, col }) => (
                <div key={l}>
                  <div style={{ color: 'var(--text-muted)', fontSize: 9 }}>{l}</div>
                  <div style={{ color: col, fontWeight: 700, fontSize: 12 }}>{fmt(v)}</div>
                </div>
              ))}
            </div>
          )}
          <div style={{ display: 'flex', gap: 6, flexShrink: 0 }}>
            <button className="btn-sm" onClick={async () => { const d = await api.exportCsv(); download(d, 'nodes.csv', 'text/csv') }}>↓ CSV</button>
            <button className="btn-sm" onClick={async () => { const d = await api.exportDot(); download(d, 'graph.dot', 'text/plain') }}>↓ DOT</button>
            <button className="btn-sm" onClick={async () => { const d = await api.exportMarkdown(); download(d, 'graph.md', 'text/markdown') }}>↓ MD</button>
          </div>
        </div>
        {health && (
          <div style={{ marginTop: 10, display: 'flex', alignItems: 'center', gap: 8 }}>
            <Bar val={health.score} color={healthColor} />
            <span style={{ color: 'var(--text-muted)', fontSize: 10, whiteSpace: 'nowrap' }}>{health.summary}</span>
          </div>
        )}
      </div>

      {/* PageRank + Bridges */}
      <div style={{ display: 'flex', gap: 10, flex: 1, overflow: 'hidden' }}>

        {/* PageRank */}
        <div className="glass" style={{ flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
          <div className="section-head">
            <span style={{ color: 'var(--cyan)' }}>◆</span> PageRank Influence
          </div>
          <div className="scroll-y" style={{ flex: 1 }}>
            {loading && <div style={{ padding: 12, color: 'var(--text-muted)', fontSize: 10 }}>Loading…</div>}
            {pagerank.map((e, i) => {
              const n = nodeMap.get(e.node_id)
              const col = NODE_COLORS[n?.node_type ?? ''] ?? 'var(--cyan)'
              const maxScore = pagerank[0]?.score ?? 1
              return (
                <div key={e.node_id} style={{
                  padding: '7px 12px', borderBottom: '1px solid rgba(40,70,120,0.2)',
                  display: 'flex', alignItems: 'center', gap: 8,
                }}>
                  <span style={{ color: 'var(--text-muted)', fontSize: 10, width: 20 }}>{i+1}</span>
                  <div style={{ width: 7, height: 7, borderRadius: '50%', background: col, flexShrink: 0 }} />
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div style={{ color: 'var(--text-primary)', fontSize: 11, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                      {e.content_preview}
                    </div>
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 6, flexShrink: 0 }}>
                    <div className="prog-track" style={{ width: 60 }}>
                      <div className="prog-fill" style={{ width: `${((e.score/maxScore)*100).toFixed(0)}%`, background: col }} />
                    </div>
                    <span style={{ color: 'var(--text-muted)', fontSize: 10, width: 40, textAlign: 'right' }}>
                      {e.score.toFixed(4)}
                    </span>
                  </div>
                </div>
              )
            })}
          </div>
        </div>

        {/* Bridges */}
        <div className="glass" style={{ width: 300, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
          <div className="section-head">
            <span style={{ color: 'var(--amber)' }}>⚡</span> Bridge Edges
            <span style={{ marginLeft: 4, color: 'var(--text-muted)' }}>({bridges.length})</span>
          </div>
          <div className="scroll-y" style={{ flex: 1 }}>
            {bridges.length === 0 && !loading && (
              <div style={{ padding: 16, color: 'var(--emerald)', fontSize: 11 }}>
                ✓ No bridges — graph is robust
              </div>
            )}
            {bridges.map((b, i) => (
              <div key={i} style={{ padding: '8px 12px', borderBottom: '1px solid rgba(40,70,120,0.2)' }}>
                <div style={{ color: 'var(--amber)', fontSize: 9, fontFamily: 'var(--font-head)', letterSpacing: '0.08em', marginBottom: 4 }}>
                  CRITICAL BRIDGE
                </div>
                <div style={{ color: 'var(--text-primary)', fontSize: 11, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                  {b.source_preview}
                </div>
                <div style={{ color: 'var(--border-bright)', fontSize: 10, padding: '1px 8px' }}>→</div>
                <div style={{ color: 'var(--text-primary)', fontSize: 11, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                  {b.target_preview}
                </div>
                <div style={{ color: 'var(--text-muted)', fontSize: 9, marginTop: 3 }}>weight {b.weight.toFixed(2)}</div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  )
}
