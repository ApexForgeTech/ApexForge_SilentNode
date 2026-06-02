import { useState, useRef } from 'react'
import type { SNode } from '../types'
import { NODE_COLORS } from '../types'
import { api } from '../api'
import { toast } from './Toast'

// The Forge — dedicated space for thought creation and connection
// Vision.md: "Creation is the highest act of cognition.
// The Forge is SilentNode's dedicated space for the act of making."

type ArtifactKind = 'thought' | 'connection' | 'journal' | 'cluster'

const KIND_ICONS: Record<ArtifactKind, string> = {
  thought:    '◆',
  connection: '⟿',
  journal:    '✎',
  cluster:    '⬡',
}
const KIND_LABELS: Record<ArtifactKind, string> = {
  thought:    'Materialize Thought',
  connection: 'Forge Connection',
  journal:    'Chronicle Entry',
  cluster:    'Name Cluster',
}
const KIND_DESCS: Record<ArtifactKind, string> = {
  thought:    'A new idea enters the universe — classified, gravity-weighted, placed.',
  connection: 'Two ideas are explicitly linked — bridge the silence between them.',
  journal:    'A moment is recorded — linked to the current cognitive context.',
  cluster:    'A group of related ideas is given a name — crystallization begins.',
}

interface Props {
  nodes: SNode[]
  onRefresh: () => void
}

export default function ForgeView({ nodes, onRefresh }: Props) {
  const [kind,     setKind]     = useState<ArtifactKind>('thought')
  const [text,     setText]     = useState('')
  const [nodeA,    setNodeA]    = useState('')
  const [nodeB,    setNodeB]    = useState('')
  const [weight,   setWeight]   = useState(1.0)
  const [forging,  setForging]  = useState(false)
  const [last,     setLast]     = useState<{ kind: ArtifactKind; content: string; ts: string }[]>([])

  const textRef = useRef<HTMLTextAreaElement>(null)

  async function forge() {
    if (forging) return
    setForging(true)
    try {
      if (kind === 'thought') {
        if (!text.trim()) { toast('Enter thought content', 'error'); return }
        await api.addThought(text.trim())
        toast('Thought materialized into the universe')
        addToHistory('thought', text.trim())
        setText('')
        onRefresh()
      }

      if (kind === 'connection') {
        if (!nodeA || !nodeB) { toast('Select both nodes', 'error'); return }
        if (nodeA === nodeB)  { toast('Cannot connect a node to itself', 'error'); return }
        await api.connect(nodeA, nodeB, weight)
        const a = nodes.find(n => n.id === nodeA)?.content.slice(0, 20) ?? nodeA.slice(0, 8)
        const b = nodes.find(n => n.id === nodeB)?.content.slice(0, 20) ?? nodeB.slice(0, 8)
        toast('Connection forged')
        addToHistory('connection', `${a} ⟿ ${b}`)
        setNodeA(''); setNodeB(''); setWeight(1.0)
        onRefresh()
      }

      if (kind === 'journal') {
        if (!text.trim()) { toast('Enter journal content', 'error'); return }
        await api.addJournal(text.trim())
        toast('Chronicle entry recorded')
        addToHistory('journal', text.trim())
        setText('')
        onRefresh()
      }

      if (kind === 'cluster') {
        if (!text.trim()) { toast('Enter cluster name', 'error'); return }
        // Materialize as an idea node that acts as cluster anchor
        await api.addThought(`[CLUSTER] ${text.trim()}`)
        toast('Cluster anchor materialized')
        addToHistory('cluster', text.trim())
        setText('')
        onRefresh()
      }
    } catch (e) {
      toast(String(e), 'error')
    }
    setForging(false)
    textRef.current?.focus()
  }

  function addToHistory(k: ArtifactKind, content: string) {
    setLast(prev => [
      { kind: k, content: content.slice(0, 60), ts: new Date().toLocaleTimeString() },
      ...prev.slice(0, 9),
    ])
  }

  const sortedNodes = [...nodes].sort((a, b) => b.gravity - a.gravity)

  return (
    <div style={{ display: 'flex', height: '100%', gap: 12 }}>

      {/* Left: Forge panel */}
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 10 }}>

        {/* Vision quote */}
        <div className="panel" style={{ padding: '12px 16px', flexShrink: 0 }}>
          <div style={{ fontSize: 11, color: 'var(--t3)', fontStyle: 'italic', lineHeight: 1.6 }}>
            "Creation is the highest act of cognition. When you create in The Forge,
            linked nodes gain energy, connected civilizations receive velocity, and
            the Lore System records the moment."
          </div>
        </div>

        {/* Kind selector */}
        <div className="panel" style={{ padding: '10px 14px', flexShrink: 0 }}>
          <div style={{ fontSize: 9, color: 'var(--t4)', letterSpacing: '0.08em', textTransform: 'uppercase', marginBottom: 10 }}>
            What will you forge?
          </div>
          <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
            {(['thought', 'connection', 'journal', 'cluster'] as ArtifactKind[]).map(k => (
              <button
                key={k}
                className={`btn-sm${kind === k ? ' btn-primary' : ''}`}
                onClick={() => setKind(k)}
                style={{ display: 'flex', alignItems: 'center', gap: 5 }}
              >
                <span>{KIND_ICONS[k]}</span>
                {KIND_LABELS[k]}
              </button>
            ))}
          </div>
          <div style={{ marginTop: 10, color: 'var(--t3)', fontSize: 11, lineHeight: 1.5 }}>
            {KIND_DESCS[kind]}
          </div>
        </div>

        {/* Forge form */}
        <div className="panel" style={{ flex: 1, display: 'flex', flexDirection: 'column' }}>
          <div className="sec-head">
            <span style={{ fontSize: 16 }}>{KIND_ICONS[kind]}</span>
            {KIND_LABELS[kind]}
          </div>
          <div style={{ padding: '14px', flex: 1, display: 'flex', flexDirection: 'column', gap: 12 }}>

            {/* Thought / Journal / Cluster form */}
            {(kind === 'thought' || kind === 'journal' || kind === 'cluster') && (
              <>
                <textarea
                  ref={textRef}
                  placeholder={
                    kind === 'thought'  ? 'Describe your thought in full…' :
                    kind === 'journal'  ? 'What are you experiencing right now…' :
                    'Name this cluster of ideas…'
                  }
                  value={text}
                  onChange={e => setText(e.target.value)}
                  onKeyDown={e => { if (e.ctrlKey && e.key === 'Enter') forge() }}
                  style={{ flex: 1, minHeight: 120, fontSize: 13, resize: 'none' }}
                  autoFocus
                />
                <div style={{ color: 'var(--t4)', fontSize: 10 }}>
                  {text.length} chars · Ctrl+Enter to forge
                </div>
              </>
            )}

            {/* Connection form */}
            {kind === 'connection' && (
              <>
                <div>
                  <div style={{ fontSize: 10, color: 'var(--t4)', marginBottom: 5 }}>SOURCE NODE</div>
                  <select value={nodeA} onChange={e => setNodeA(e.target.value)} style={{ width: '100%' }}>
                    <option value="">Select source…</option>
                    {sortedNodes.map(n => (
                      <option key={n.id} value={n.id}>
                        {NODE_COLORS[n.node_type] ? `[${n.node_type}] ` : ''}{n.content.slice(0, 50)}
                      </option>
                    ))}
                  </select>
                </div>
                <div style={{ textAlign: 'center', fontSize: 20, color: 'var(--lavender-text)' }}>⟿</div>
                <div>
                  <div style={{ fontSize: 10, color: 'var(--t4)', marginBottom: 5 }}>TARGET NODE</div>
                  <select value={nodeB} onChange={e => setNodeB(e.target.value)} style={{ width: '100%' }}>
                    <option value="">Select target…</option>
                    {sortedNodes.filter(n => n.id !== nodeA).map(n => (
                      <option key={n.id} value={n.id}>
                        {n.content.slice(0, 50)}
                      </option>
                    ))}
                  </select>
                </div>
                <div>
                  <div style={{ fontSize: 10, color: 'var(--t4)', marginBottom: 5 }}>
                    CONNECTION WEIGHT: {weight.toFixed(1)}
                  </div>
                  <input
                    type="range" min="0.1" max="2.0" step="0.1"
                    value={weight} onChange={e => setWeight(parseFloat(e.target.value))}
                    style={{ width: '100%', accentColor: 'var(--lavender)' }}
                  />
                  <div style={{ display: 'flex', justifyContent: 'space-between', color: 'var(--t4)', fontSize: 9 }}>
                    <span>Weak (0.1)</span>
                    <span>Normal (1.0)</span>
                    <span>Strong (2.0)</span>
                  </div>
                </div>
              </>
            )}

            {/* Forge button */}
            <button
              className="btn-primary"
              onClick={forge}
              disabled={forging || (
                (kind === 'thought' || kind === 'journal' || kind === 'cluster') ? !text.trim() :
                (!nodeA || !nodeB)
              )}
              style={{
                padding: '10px', fontSize: 13, fontWeight: 600,
                letterSpacing: '0.06em', marginTop: 'auto',
              }}
            >
              {forging ? 'Forging…' : `⚒ Forge ${KIND_LABELS[kind]}`}
            </button>
          </div>
        </div>
      </div>

      {/* Right: Forge history */}
      <div style={{ width: 280, flexShrink: 0, display: 'flex', flexDirection: 'column', gap: 10 }}>
        <div className="panel" style={{ flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
          <div className="sec-head">
            <span style={{ color: 'var(--amber)' }}>◈</span>
            Forge History
          </div>
          <div className="scroll" style={{ flex: 1 }}>
            {last.length === 0 ? (
              <div style={{ padding: '16px 14px', color: 'var(--t4)', fontSize: 11 }}>
                Nothing forged yet. Create something.
              </div>
            ) : last.map((h, i) => (
              <div key={i} style={{
                padding: '9px 12px',
                borderBottom: '1px solid rgba(255,255,255,0.04)',
                animation: i === 0 ? 'fade-up 0.2s ease both' : undefined,
              }}>
                <div style={{ display: 'flex', gap: 7, alignItems: 'center', marginBottom: 4 }}>
                  <span style={{ color: 'var(--amber)', fontSize: 12 }}>{KIND_ICONS[h.kind]}</span>
                  <span style={{ color: 'var(--t4)', fontSize: 9, fontFamily: 'var(--font-mono)' }}>{h.ts}</span>
                </div>
                <div style={{ color: 'var(--t2)', fontSize: 11, lineHeight: 1.4 }}>{h.content}</div>
              </div>
            ))}
          </div>
        </div>

        {/* Quick reference */}
        <div className="panel" style={{ padding: '12px 14px', flexShrink: 0 }}>
          <div style={{ fontSize: 9, color: 'var(--t4)', letterSpacing: '0.08em', textTransform: 'uppercase', marginBottom: 8 }}>
            Forge Effects
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 5 }}>
            {[
              ['Thought', 'Adds node, propagates contagion'],
              ['Connection', 'Creates edge, bridges silence'],
              ['Journal', 'Records moment, links to focus'],
              ['Cluster', 'Names attractor node for grouping'],
            ].map(([l, d]) => (
              <div key={l} style={{ fontSize: 11 }}>
                <span style={{ color: 'var(--t2)', fontWeight: 500 }}>{l}: </span>
                <span style={{ color: 'var(--t4)' }}>{d}</span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  )
}
