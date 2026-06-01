import { useState, useEffect } from 'react'
import type { SilenceData } from '../types'
import { api } from '../api'
import { toast } from './Toast'

function Bar({ val, color = 'var(--lavender-text)' }: { val: number; color?: string }) {
  return (
    <div className="bar fill">
      <div className="bar-fill" style={{ width: `${(val * 100).toFixed(0)}%`, background: color }} />
    </div>
  )
}

export default function SilenceView() {
  const [data,    setData]    = useState<SilenceData | null>(null)
  const [loading, setLoading] = useState(true)
  const [tab,     setTab]     = useState<'bridges'|'implied'>('bridges')

  useEffect(() => {
    api.silence()
      .then(d => { setData(d); setLoading(false) })
      .catch(() => setLoading(false))
  }, [])

  async function connectNodes(nodeA: string, nodeB: string) {
    try {
      await api.connect(nodeA, nodeB)
      toast('Bridge created — nodes connected')
    } catch (e) { toast(String(e), 'error') }
  }

  async function addImplied(content: string) {
    try {
      await api.addThought(content)
      toast(`Node materialized: ${content.slice(0, 30)}`)
    } catch (e) { toast(String(e), 'error') }
  }

  if (loading) return (
    <div style={{ flex:1, display:'flex', alignItems:'center', justifyContent:'center', color:'var(--t4)' }}>
      Analyzing the silence between your thoughts…
    </div>
  )

  return (
    <div className="col" style={{ height:'100%', gap:10 }}>

      {/* Intro quote */}
      <div style={{ padding:'10px 14px', borderBottom:'1px solid var(--line)', flexShrink:0 }}>
        <div style={{ fontSize:11, color:'var(--t3)', fontStyle:'italic', lineHeight:1.6 }}>
          "Empty space is not empty. In SilentNode, the space between nodes is data —
          what is absent is as meaningful as what is present."
        </div>
      </div>

      {/* Tabs */}
      <div style={{ display:'flex', gap:10, alignItems:'center', flexShrink:0 }}>
        <div className="tabs">
          <button className={`tab${tab === 'bridges' ? ' active' : ''}`} onClick={() => setTab('bridges')}>
            ⟿ Missing Bridges ({data?.missing_bridges.length ?? 0})
          </button>
          <button className={`tab${tab === 'implied' ? ' active' : ''}`} onClick={() => setTab('implied')}>
            ◈ Implied Concepts ({data?.implied_concepts.length ?? 0})
          </button>
        </div>
        <button className="btn-sm" onClick={() => {
          setLoading(true)
          api.silence().then(d => { setData(d); setLoading(false) })
        }}>↺</button>
      </div>

      {/* Missing Bridges */}
      {tab === 'bridges' && (
        <div className="panel fill scroll">
          <div className="sec-head">
            <span style={{ color:'var(--sky)' }}>⟿</span>
            Missing Connections
            <span style={{ marginLeft:4, color:'var(--t4)', fontSize:10 }}>
              similar nodes with no edge between them
            </span>
          </div>

          {(data?.missing_bridges.length ?? 0) === 0 && (
            <div style={{ padding:'20px 14px', color:'var(--t4)', fontSize:12 }}>
              No significant missing bridges detected. Your graph is well-connected.
            </div>
          )}

          {data?.missing_bridges.map((b, i) => (
            <div key={i} style={{ padding:'12px 14px', borderBottom:'1px solid rgba(255,255,255,0.04)' }}>
              <div style={{ display:'flex', alignItems:'center', gap:8, marginBottom:8 }}>
                <div className="bar" style={{ width:80 }}>
                  <div className="bar-fill" style={{ width:`${(b.similarity * 100).toFixed(0)}%`, background:'var(--sky)' }} />
                </div>
                <span style={{ color:'var(--sky)', fontSize:10, fontFamily:'var(--font-mono)' }}>
                  {(b.similarity * 100).toFixed(0)}% similar
                </span>
              </div>

              <div style={{ display:'flex', alignItems:'center', gap:8, marginBottom:8 }}>
                <div style={{
                  flex:1, padding:'6px 10px',
                  background:'rgba(255,255,255,0.03)', borderRadius:4,
                  border:'1px solid var(--line)',
                  fontSize:12, color:'var(--t1)',
                  overflow:'hidden', textOverflow:'ellipsis', whiteSpace:'nowrap',
                }}>
                  {b.preview_a}
                </div>
                <span style={{ color:'var(--t4)', fontSize:16 }}>⟿</span>
                <div style={{
                  flex:1, padding:'6px 10px',
                  background:'rgba(255,255,255,0.03)', borderRadius:4,
                  border:'1px solid var(--line)',
                  fontSize:12, color:'var(--t1)',
                  overflow:'hidden', textOverflow:'ellipsis', whiteSpace:'nowrap',
                }}>
                  {b.preview_b}
                </div>
              </div>

              <div style={{ display:'flex', justifyContent:'space-between', alignItems:'center' }}>
                <span style={{ color:'var(--t3)', fontSize:11 }}>{b.reason}</span>
                <button
                  className="btn-sm btn-primary"
                  onClick={() => connectNodes(b.node_a, b.node_b)}
                >
                  Bridge gap
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Implied Concepts */}
      {tab === 'implied' && (
        <div className="panel fill scroll">
          <div className="sec-head">
            <span style={{ color:'var(--lavender-text)' }}>◈</span>
            Absent Concepts
            <span style={{ marginLeft:4, color:'var(--t4)', fontSize:10 }}>
              ideas implied by your graph structure but not yet created
            </span>
          </div>

          {(data?.implied_concepts.length ?? 0) === 0 && (
            <div style={{ padding:'20px 14px', color:'var(--t4)', fontSize:12 }}>
              No implied concepts detected. Add more connected nodes to reveal hidden patterns.
            </div>
          )}

          {data?.implied_concepts.map((c, i) => (
            <div key={i} style={{ padding:'14px', borderBottom:'1px solid rgba(255,255,255,0.04)' }}>
              <div style={{ display:'flex', justifyContent:'space-between', alignItems:'flex-start', marginBottom:8 }}>
                <div style={{
                  flex:1, fontSize:14, fontWeight:600, color:'var(--t1)', lineHeight:1.4,
                  paddingRight:12,
                }}>
                  "{c.suggested_content}"
                </div>
                <div style={{ display:'flex', gap:6, flexShrink:0 }}>
                  <div style={{ display:'flex', alignItems:'center', gap:6 }}>
                    <Bar val={c.confidence} color="var(--lavender-text)" />
                    <span style={{ color:'var(--lavender-text)', fontSize:11, fontFamily:'var(--font-mono)', width:36 }}>
                      {(c.confidence * 100).toFixed(0)}%
                    </span>
                  </div>
                </div>
              </div>

              <div style={{ marginBottom:8 }}>
                <div style={{ color:'var(--t4)', fontSize:10, marginBottom:4 }}>Implied by:</div>
                <div style={{ display:'flex', gap:4, flexWrap:'wrap' }}>
                  {c.implied_by_previews.map((p, j) => (
                    <span key={j} className="badge badge-mt" style={{ fontSize:9 }}>{p}</span>
                  ))}
                </div>
              </div>

              <button
                className="btn-sm btn-primary"
                onClick={() => addImplied(c.suggested_content)}
              >
                + Materialize this thought
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
