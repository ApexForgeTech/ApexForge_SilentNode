import { useState } from 'react'
import type { SNode, ArchaeologyData, ResurrectedNode } from '../types'
import { NODE_COLORS, NODE_ICONS } from '../types'
import { api } from '../api'
import { toast } from './Toast'

const CHANGE_COLORS: Record<string, string> = {
  Created:     'var(--green)',
  Modified:    'var(--amber)',
  Accessed:    'var(--sky)',
  GhostEntered: 'var(--t4)',
  GhostExited: 'var(--lavender-text)',
  Voided:      'var(--violet)',
  Fossilized:  'var(--amber)',
}

function timeLabel(ts: string): string {
  const d = new Date(ts)
  return d.toLocaleString()
}

interface Props { nodes: SNode[] }

export default function ArchaeologyView({ nodes }: Props) {
  const [selectedNode, setSelectedNode] = useState('')
  const [data, setData]                 = useState<ArchaeologyData | null>(null)
  const [resurrected, setResurrected]   = useState<ResurrectedNode | null>(null)
  const [loading, setLoading]           = useState(false)
  const [resLoading, setResLoading]     = useState(false)
  const [selectedIdx, setSelectedIdx]   = useState<number | null>(null)

  async function openArchaeology(nodeId: string) {
    if (!nodeId) return
    setLoading(true); setData(null); setResurrected(null); setSelectedIdx(null)
    try {
      const d = await api.archaeology(nodeId)
      setData(d)
    } catch (e) {
      toast(String(e), 'error')
    }
    setLoading(false)
  }

  async function resurrectAt(index: number) {
    if (!selectedNode) return
    setResLoading(true)
    try {
      const r = await api.archaeologyResurrect(selectedNode, index)
      setResurrected(r)
      setSelectedIdx(index)
    } catch (e) {
      toast(String(e), 'error')
    }
    setResLoading(false)
  }

  const sortedNodes = [...nodes].sort((a, b) => b.access_count - a.access_count)

  return (
    <div className="split">
      {/* Left: node picker + timeline */}
      <div className="split-list panel">
        <div className="sec-head">
          <span style={{ color: 'var(--amber)' }}>⟲</span>
          Thought Archaeology
        </div>

        {/* Node selector */}
        <div style={{ padding: '8px 10px', borderBottom: '1px solid var(--line)' }}>
          <select
            value={selectedNode}
            onChange={e => { setSelectedNode(e.target.value); openArchaeology(e.target.value) }}
            style={{ width: '100%' }}
          >
            <option value="">Select node to excavate…</option>
            {sortedNodes.map(n => (
              <option key={n.id} value={n.id}>
                {n.content.slice(0, 45)}
              </option>
            ))}
          </select>
        </div>

        {/* Timeline */}
        <div className="scroll fill">
          {loading && (
            <div style={{ padding: 16, color: 'var(--t4)', fontSize: 11 }}>Excavating temporal record…</div>
          )}
          {!loading && !data && !selectedNode && (
            <div style={{ padding: 16, color: 'var(--t4)', fontSize: 11, lineHeight: 1.6 }}>
              Select a node to descend into its temporal record. Every modification, access, and state change is preserved.
            </div>
          )}
          {data && (
            <div style={{ padding: '8px 0' }}>
              <div style={{
                padding: '6px 12px', marginBottom: 4,
                display: 'flex', gap: 10, alignItems: 'center',
                background: 'rgba(255,255,255,0.02)',
              }}>
                <span style={{ color: 'var(--t3)', fontSize: 10 }}>
                  {data.snapshot_count} snapshots recorded
                </span>
              </div>
              {data.timeline.length === 0 && (
                <div style={{ padding: 16, color: 'var(--t4)', fontSize: 11, lineHeight: 1.6 }}>
                  No temporal snapshots recorded for this node yet.
                </div>
              )}
              {data.timeline.map(entry => {
                const col  = CHANGE_COLORS[entry.change_type] ?? 'var(--t3)'
                const isSel = selectedIdx === entry.index
                return (
                  <div key={entry.index}
                    style={{
                      padding: '7px 12px 7px 24px',
                      borderBottom: '1px solid rgba(255,255,255,0.03)',
                      cursor: 'pointer',
                      position: 'relative',
                      background: isSel ? 'rgba(167,139,250,0.06)' : 'transparent',
                      borderLeft: isSel ? '2px solid rgba(167,139,250,0.5)' : '2px solid transparent',
                      transition: 'background 0.15s',
                    }}
                    onClick={() => resurrectAt(entry.index)}
                  >
                    <div style={{
                      position: 'absolute', left: 8, top: '50%', transform: 'translateY(-50%)',
                      width: 8, height: 8, borderRadius: '50%',
                      background: col, border: `1px solid ${col}`,
                      boxShadow: isSel ? `0 0 6px ${col}` : 'none',
                    }} />
                    <div style={{ fontSize: 10, color: col, fontWeight: 600, letterSpacing: '0.04em', marginBottom: 2 }}>
                      {entry.change_type}
                    </div>
                    <div style={{ fontSize: 9, color: 'var(--t4)', fontFamily: 'var(--font-mono)' }}>
                      {timeLabel(entry.timestamp)}
                    </div>
                    <div style={{ fontSize: 9, color: 'var(--t4)' }}>snapshot #{entry.index}</div>
                  </div>
                )
              })}
            </div>
          )}
        </div>
      </div>

      {/* Right: detail / resurrected state */}
      <div className="split-detail" style={{ gap: 10 }}>
        {/* Current state */}
        {data && (
          <div className="panel anim-in" style={{ flexShrink: 0 }}>
            <div className="sec-head">
              <span style={{ color: 'var(--sky)' }}>◉</span>
              Current State
              <span style={{ marginLeft: 'auto', color: 'var(--t4)', fontSize: 10 }}>
                {data.snapshot_count} snapshots
              </span>
            </div>
            <div style={{ padding: '12px 14px', display: 'flex', flexDirection: 'column', gap: 6 }}>
              <div style={{ color: 'var(--t1)', fontSize: 13, lineHeight: 1.6 }}>
                {data.current_content}
              </div>
              <div style={{ display: 'flex', gap: 16, marginTop: 4 }}>
                <div>
                  <div style={{ fontSize: 9, color: 'var(--t4)', marginBottom: 2 }}>ENTROPY</div>
                  <span style={{
                    color: data.current_entropy > 0.7 ? 'var(--red)' : data.current_entropy > 0.4 ? 'var(--amber)' : 'var(--green)',
                    fontFamily: 'var(--font-mono)', fontSize: 12,
                  }}>
                    {data.current_entropy.toFixed(3)}
                  </span>
                </div>
                <div>
                  <div style={{ fontSize: 9, color: 'var(--t4)', marginBottom: 2 }}>LAST CHANGE</div>
                  <span style={{ color: 'var(--t2)', fontSize: 11 }}>
                    {timeLabel(data.current_timestamp)}
                  </span>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Resurrected snapshot */}
        {resurrected && (
          <div className="panel anim-glow-in" style={{ flexShrink: 0 }}>
            <div className="sec-head">
              <span style={{ color: 'var(--violet)' }}>◌</span>
              Resurrected — Snapshot #{resurrected.snapshot_index}
              {resLoading && <span style={{ marginLeft: 6, color: 'var(--t4)', fontSize: 10 }}>loading…</span>}
            </div>
            <div style={{ padding: '12px 14px', display: 'flex', flexDirection: 'column', gap: 8 }}>
              <div style={{ color: 'var(--lavender-text)', fontSize: 13, lineHeight: 1.6 }}>
                {resurrected.content}
              </div>
              <div style={{ display: 'flex', gap: 16, flexWrap: 'wrap', marginTop: 4 }}>
                {[
                  ['Timestamp', timeLabel(resurrected.timestamp), 'var(--t3)'],
                  ['Entropy',   resurrected.entropy.toFixed(3),  'var(--amber)'],
                  ['Gravity',   resurrected.gravity.toFixed(2),  'var(--sky)'],
                ].map(([l, v, c]) => (
                  <div key={String(l)}>
                    <div style={{ fontSize: 9, color: 'var(--t4)', marginBottom: 2 }}>{l}</div>
                    <span style={{ color: String(c), fontSize: 11 }}>{v}</span>
                  </div>
                ))}
                {resurrected.is_ghost && (
                  <span className="badge badge-mt">ghost</span>
                )}
                {resurrected.is_fossil && (
                  <span className="badge badge-am">fossil</span>
                )}
              </div>
            </div>
          </div>
        )}

        {/* Instruction */}
        {data && !resurrected && data.timeline.length > 0 && (
          <div style={{ padding: '16px 14px', color: 'var(--t4)', fontSize: 11, lineHeight: 1.7 }}>
            Click any snapshot on the left to resurrect the node at that point in time. You can see exactly what it looked like — its content, entropy level, and state flags.
          </div>
        )}

        {/* Empty history */}
        {data && data.timeline.length === 0 && (
          <div className="panel anim-in" style={{ padding: 20 }}>
            <div style={{ color: 'var(--amber)', fontSize: 13, marginBottom: 8 }}>No temporal record found</div>
            <div style={{ color: 'var(--t4)', fontSize: 11, lineHeight: 1.6 }}>
              This node has no recorded snapshots yet. Capture a temporal snapshot, then return here to inspect its history.
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
