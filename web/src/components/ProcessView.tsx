import { useState, useEffect } from 'react'
import type { ProcessData, SNode } from '../types'
import { api } from '../api'
import { toast } from './Toast'

const STATUS_COLORS: Record<string, string> = {
  running:  'var(--green)',
  sleeping: 'var(--sky)',
  idle:     'var(--t4)',
  stopped:  'var(--amber)',
  zombie:   'var(--red)',
  unknown:  'var(--t4)',
}

function memLabel(mb: number): string {
  if (mb < 1024) return `${mb.toFixed(0)} MB`
  return `${(mb / 1024).toFixed(1)} GB`
}

function uptimeLabel(s: number): string {
  if (s < 60) return `${s.toFixed(0)}s`
  if (s < 3600) return `${(s / 60).toFixed(0)}m`
  if (s < 86400) return `${(s / 3600).toFixed(1)}h`
  return `${(s / 86400).toFixed(0)}d`
}

interface Props { nodes: SNode[] }

export default function ProcessView({ nodes }: Props) {
  const [procs,   setProcs]   = useState<ProcessData[]>([])
  const [loading, setLoading] = useState(true)
  const [selected, setSelected] = useState<ProcessData | null>(null)
  const [linkNode, setLinkNode] = useState('')
  const [linking, setLinking] = useState(false)
  const [sortBy, setSortBy]   = useState<'cpu' | 'mem' | 'name'>('cpu')

  useEffect(() => {
    load()
    const id = setInterval(load, 15000)
    return () => clearInterval(id)
  }, [])

  async function load() {
    try {
      const p = await api.processes()
      setProcs(p)
    } catch (e) {}
    setLoading(false)
  }

  async function linkToNode() {
    if (!selected || !linkNode) return
    setLinking(true)
    try {
      await api.linkProcess(selected.pid, linkNode)
      toast(`Process ${selected.name} linked to node`)
      setLinkNode('')
      load()
    } catch (e) { toast(String(e), 'error') }
    setLinking(false)
  }

  const sorted = [...procs].sort((a, b) => {
    if (sortBy === 'cpu') return b.cpu_usage - a.cpu_usage
    if (sortBy === 'mem') return b.memory_mb - a.memory_mb
    return a.name.localeCompare(b.name)
  })

  const totalCpu = procs.reduce((s, p) => s + p.cpu_usage, 0)
  const totalMem = procs.reduce((s, p) => s + p.memory_mb, 0)
  const running  = procs.filter(p => p.status === 'running').length

  return (
    <div className="split">
      {/* Left: process list */}
      <div className="split-list panel">
        <div className="sec-head">
          <span style={{ color: 'var(--green)' }}>◑</span>
          Process Sovereignty
          <span style={{ marginLeft: 'auto', color: 'var(--t4)', fontSize: 10 }}>
            {running}/{procs.length}
          </span>
        </div>

        {/* Sort + summary */}
        <div style={{ padding: '6px 10px', borderBottom: '1px solid var(--line)', display: 'flex', gap: 4, alignItems: 'center' }}>
          {(['cpu', 'mem', 'name'] as const).map(s => (
            <button key={s}
              className={`btn-xs${sortBy === s ? ' btn-primary' : ''}`}
              onClick={() => setSortBy(s)}
            >
              {s.toUpperCase()}
            </button>
          ))}
          <button className="btn-xs" style={{ marginLeft: 'auto' }} onClick={load}>↻</button>
        </div>

        {/* Totals */}
        <div style={{
          padding: '6px 12px', borderBottom: '1px solid var(--line)',
          display: 'flex', gap: 16,
          background: 'rgba(255,255,255,0.02)',
        }}>
          {[
            [`${totalCpu.toFixed(0)}%`, 'CPU', 'var(--amber)'],
            [memLabel(totalMem), 'MEM', 'var(--sky)'],
            [`${running}`, 'RUNNING', 'var(--green)'],
          ].map(([v, l, c]) => (
            <div key={String(l)} style={{ textAlign: 'center' }}>
              <div style={{ fontSize: 12, fontWeight: 700, color: String(c), fontFamily: 'var(--font-mono)' }}>{v}</div>
              <div style={{ fontSize: 8, color: 'var(--t4)' }}>{l}</div>
            </div>
          ))}
        </div>

        <div className="scroll fill">
          {loading && <div style={{ padding: 16, color: 'var(--t4)', fontSize: 11 }}>Scanning processes…</div>}
          {sorted.map(p => {
            const col  = STATUS_COLORS[p.status] ?? 'var(--t4)'
            const isSel = selected?.pid === p.pid
            return (
              <div key={p.pid}
                className="list-row"
                style={isSel ? { background: 'rgba(167,139,250,0.07)', borderLeft: '2px solid rgba(167,139,250,0.5)' } : {}}
                onClick={() => setSelected(p)}
              >
                <div style={{ width: 7, height: 7, borderRadius: '50%', background: col, flexShrink: 0, marginTop: 4 }} />
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ fontSize: 11, color: 'var(--t1)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', marginBottom: 2 }}>
                    {p.name}
                    {p.linked_node_id && <span style={{ color: 'var(--lavender-text)', fontSize: 9, marginLeft: 5 }}>⟿ linked</span>}
                  </div>
                  <div style={{ display: 'flex', gap: 8 }}>
                    <span style={{ fontSize: 9, color: 'var(--amber)', fontFamily: 'var(--font-mono)' }}>
                      {p.cpu_usage.toFixed(1)}%
                    </span>
                    <span style={{ fontSize: 9, color: 'var(--sky)', fontFamily: 'var(--font-mono)' }}>
                      {memLabel(p.memory_mb)}
                    </span>
                    <span style={{ fontSize: 9, color: 'var(--t4)', marginLeft: 'auto' }}>
                      {uptimeLabel(p.uptime_seconds)}
                    </span>
                  </div>
                </div>
              </div>
            )
          })}
        </div>
      </div>

      {/* Right: detail */}
      <div className="split-detail" style={{ gap: 10 }}>
        {selected ? (
          <>
            {/* Process detail */}
            <div className="panel anim-in" style={{ flexShrink: 0 }}>
              <div className="sec-head">
                <span style={{ color: STATUS_COLORS[selected.status] ?? 'var(--t3)' }}>◑</span>
                {selected.name}
                <span className="badge badge-mt" style={{ marginLeft: 'auto', fontSize: 9 }}>
                  PID {selected.pid}
                </span>
              </div>
              <div style={{ padding: '12px 14px', display: 'flex', flexDirection: 'column', gap: 8 }}>
                <div style={{ color: 'var(--t3)', fontSize: 11, fontFamily: 'var(--font-mono)', wordBreak: 'break-all', lineHeight: 1.5 }}>
                  {selected.command}
                </div>
                <div style={{ display: 'flex', gap: 20, flexWrap: 'wrap' }}>
                  {[
                    ['Status',  selected.status,               STATUS_COLORS[selected.status] ?? 'var(--t3)'],
                    ['CPU',     `${selected.cpu_usage.toFixed(1)}%`, 'var(--amber)'],
                    ['Memory',  memLabel(selected.memory_mb),  'var(--sky)'],
                    ['Uptime',  uptimeLabel(selected.uptime_seconds), 'var(--t2)'],
                  ].map(([l, v, c]) => (
                    <div key={String(l)}>
                      <div style={{ fontSize: 9, color: 'var(--t4)', marginBottom: 2 }}>{l}</div>
                      <span style={{ color: String(c), fontSize: 12, fontFamily: 'var(--font-mono)' }}>{v}</span>
                    </div>
                  ))}
                </div>
                {selected.linked_node_id && (
                  <div style={{ display: 'flex', gap: 6, alignItems: 'center', paddingTop: 6, borderTop: '1px solid var(--line)' }}>
                    <span style={{ color: 'var(--lavender-text)', fontSize: 11 }}>⟿ Linked to node</span>
                    <span className="badge badge-lv" style={{ fontSize: 9 }}>
                      {selected.linked_node_id.slice(0, 8)}
                    </span>
                  </div>
                )}
              </div>
            </div>

            {/* Link to node */}
            <div className="panel" style={{ flexShrink: 0, padding: '12px 14px' }}>
              <div style={{ fontSize: 9, color: 'var(--t4)', textTransform: 'uppercase', letterSpacing: '0.08em', marginBottom: 8 }}>
                Link to Cognitive Node
              </div>
              <div style={{ display: 'flex', gap: 6 }}>
                <select value={linkNode} onChange={e => setLinkNode(e.target.value)} style={{ flex: 1 }}>
                  <option value="">Select a node…</option>
                  {nodes.map(n => (
                    <option key={n.id} value={n.id}>{n.content.slice(0, 45)}</option>
                  ))}
                </select>
                <button
                  className="btn-sm btn-primary"
                  onClick={linkToNode}
                  disabled={!linkNode || linking}
                >
                  {linking ? '…' : 'Link'}
                </button>
              </div>
              <div style={{ fontSize: 10, color: 'var(--t4)', marginTop: 6, lineHeight: 1.5 }}>
                Linking a process to a node allows SilentNode to track its cognitive significance and include it in the temporal record.
              </div>
            </div>

            {/* CPU bar */}
            <div className="panel" style={{ flexShrink: 0, padding: '12px 14px' }}>
              <div style={{ fontSize: 9, color: 'var(--t4)', marginBottom: 6 }}>RELATIVE CPU USAGE</div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <div className="bar fill">
                  <div className="bar-fill" style={{
                    width: `${Math.min(selected.cpu_usage, 100)}%`,
                    background: selected.cpu_usage > 50 ? 'var(--red)' : selected.cpu_usage > 20 ? 'var(--amber)' : 'var(--green)',
                  }} />
                </div>
                <span style={{
                  color: 'var(--amber)', fontSize: 12, fontFamily: 'var(--font-mono)', width: 44, textAlign: 'right',
                }}>
                  {selected.cpu_usage.toFixed(1)}%
                </span>
              </div>
            </div>
          </>
        ) : (
          <div className="panel fill" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <div style={{ textAlign: 'center', color: 'var(--t4)' }}>
              <div style={{ fontSize: 32, marginBottom: 12, opacity: 0.4 }}>◑</div>
              <div style={{ fontSize: 12, marginBottom: 6 }}>Process Sovereignty</div>
              <div style={{ fontSize: 11, maxWidth: 260, lineHeight: 1.6 }}>
                Select a process to inspect and link it to a cognitive node.
                No process runs outside SilentNode's awareness.
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
