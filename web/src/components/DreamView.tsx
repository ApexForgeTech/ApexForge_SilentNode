import { useState, useEffect } from 'react'
import type { DreamProposal, SNode } from '../types'
import { api } from '../api'
import { toast } from './Toast'

const KIND_COLORS: Record<string, string> = {
  SuggestEdge:  'var(--cyan)',
  ReviveGhost:  'var(--violet)',
  MergeNodes:   'var(--amber)',
  EntropyAlert: 'var(--crimson)',
}
const KIND_ICONS: Record<string, string> = {
  SuggestEdge:  '⟿',
  ReviveGhost:  '◌',
  MergeNodes:   '⊕',
  EntropyAlert: '⚠',
}

interface Props { nodes: SNode[] }

export default function DreamView({ nodes }: Props) {
  const [proposals, setProposals] = useState<DreamProposal[]>([])
  const [selected,  setSelected]  = useState<DreamProposal | null>(null)
  const [ignored, setIgnored] = useState<Set<string>>(() => {
    try {
      return new Set(JSON.parse(window.localStorage.getItem('silentnode.ignoredDreams') || '[]'))
    } catch {
      return new Set()
    }
  })
  const [synthesis, setSynthesis]  = useState('')
  const [synResult, setSynResult]  = useState<{ narrative: string; related_nodes: string[] } | null>(null)
  const [synLoading, setSynLoading] = useState(false)
  const [actionLoading, setActionLoading] = useState(false)
  const [loading, setLoading] = useState(true)

  const nodeMap = new Map(nodes.map(n => [n.id, n]))
  const ghostCount = nodes.filter(n => n.is_ghost).length
  const entropyLoad = nodes.length
    ? nodes.reduce((sum, n) => sum + n.entropy, 0) / nodes.length
    : 0

  useEffect(() => {
    loadProposals()
  }, [])

  async function loadProposals(selectFirst = true) {
    setLoading(true)
    api.dreamProposals()
      .then(p => {
        const filtered = p.filter(item => !ignored.has(proposalSignature(item)))
        setProposals(filtered)
        if (selectFirst) setSelected(filtered[0] ?? null)
      })
      .catch(() => {})
      .finally(() => setLoading(false))
  }

  async function runSynthesis() {
    if (!synthesis.trim()) return
    setSynLoading(true)
    try {
      const r = await api.synthesize(synthesis.trim())
      setSynResult(r)
    } catch (e) { toast(String(e), 'error') }
    setSynLoading(false)
  }

  async function applyProposal(proposal: DreamProposal) {
    setActionLoading(true)
    try {
      const result = await api.applyDreamProposal(proposal)
      toast(result.message || 'Dream action applied')
      await loadProposals()
    } catch (e) {
      toast(String(e), 'error')
    } finally {
      setActionLoading(false)
    }
  }

  function ignoreProposal(proposal: DreamProposal) {
    const nextIgnored = new Set(ignored)
    nextIgnored.add(proposalSignature(proposal))
    setIgnored(nextIgnored)
    window.localStorage.setItem('silentnode.ignoredDreams', JSON.stringify(Array.from(nextIgnored).slice(-200)))
    const next = proposals.filter(p => p.id !== proposal.id)
    setProposals(next)
    setSelected(next[0] ?? null)
    toast('Proposal ignored')
  }

  function proposalSignature(proposal: DreamProposal) {
    return [
      proposal.kind,
      proposal.from || '',
      proposal.to || '',
      proposal.node_id || '',
      proposal.a || '',
      proposal.b || '',
    ].join(':')
  }

  function involvedNodes(proposal: DreamProposal) {
    return [proposal.from, proposal.to, proposal.node_id, proposal.a, proposal.b]
      .filter(Boolean)
      .filter((id, index, arr) => arr.indexOf(id) === index) as string[]
  }

  return (
    <div className="dream-mode" style={{ display: 'flex', height: '100%', gap: 10 }}>

      {/* Dream proposals */}
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 10, overflow: 'hidden' }}>
        <div className="dream-atmosphere">
          <div>
            <span>Dream field</span>
            <strong>{proposals.length ? `${proposals.length} unstable proposal${proposals.length === 1 ? '' : 's'}` : 'Quiet field'}</strong>
          </div>
          <div className="dream-stats">
            <em>{ghostCount} ghosts</em>
            <em>{(entropyLoad * 100).toFixed(0)}% entropy</em>
          </div>
        </div>
        <div className="glass" style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden', flex: 1 }}>
          <div className="section-head">
            <span style={{ color: 'var(--violet)' }}>◈</span> Dream Proposals
            <span style={{ marginLeft: 4, color: 'var(--text-muted)' }}>({proposals.length})</span>
            <button className="btn-xs" style={{ marginLeft: 'auto' }} onClick={() => loadProposals(false)}>Refresh</button>
          </div>
          <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
            {/* List */}
            <div className="scroll-y" style={{ width: 260, borderRight: '1px solid var(--border)', flexShrink: 0 }}>
              {loading && <div style={{ padding: 12, color: 'var(--text-muted)', fontSize: 10 }}>Loading…</div>}
              {proposals.length === 0 && !loading && (
                <div style={{ padding: 16, color: 'var(--text-muted)', fontSize: 11 }}>
                  No proposals — graph may be sparse
                </div>
              )}
              {proposals.map(p => {
                const col = KIND_COLORS[p.kind] ?? 'var(--cyan)'
                const icon = KIND_ICONS[p.kind] ?? '◆'
                return (
                  <div
                    key={p.id}
                    className={`list-item ${selected?.id === p.id ? 'active' : ''}`}
                    onClick={() => setSelected(p)}
                  >
                    <span style={{ color: col, fontSize: 14 }}>{icon}</span>
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div style={{ fontSize: 9, fontFamily: 'var(--font-head)', color: col, marginBottom: 2, letterSpacing: '0.08em' }}>
                        {p.kind.replace(/([A-Z])/g, ' $1').trim()}
                      </div>
                      <div style={{ fontSize: 11, color: 'var(--text-secondary)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                        {p.description}
                      </div>
                    </div>
                    <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-end', gap: 2, flexShrink: 0 }}>
                      <span style={{ color: col, fontSize: 11, fontWeight: 700 }}>{(p.confidence * 100).toFixed(0)}%</span>
                      <span style={{ color: 'var(--text-muted)', fontSize: 9 }}>{p.risk}</span>
                      <div className="prog-track" style={{ width: 36 }}>
                        <div className="prog-fill" style={{ width: `${(p.confidence*100).toFixed(0)}%`, background: col }} />
                      </div>
                    </div>
                  </div>
                )
              })}
            </div>

            {/* Detail */}
            <div className="scroll-y" style={{ flex: 1, padding: '16px' }}>
              {!selected ? (
                <div style={{ color: 'var(--text-muted)', fontSize: 11 }}>Select a proposal</div>
              ) : (() => {
                const col = KIND_COLORS[selected.kind] ?? 'var(--cyan)'
                const icon = KIND_ICONS[selected.kind] ?? '◆'
                const involved = involvedNodes(selected)
                return (
                  <div className="anim-glow-in">
                    <div style={{ fontSize: 24, color: col, marginBottom: 8 }}>{icon}</div>
                    <div style={{ fontFamily: 'var(--font-head)', fontSize: 10, color: col, letterSpacing: '0.12em', marginBottom: 6 }}>
                      {selected.kind.replace(/([A-Z])/g, ' $1').trim()}
                    </div>
                    <div style={{ color: 'var(--text-primary)', fontSize: 13, lineHeight: 1.7, marginBottom: 16 }}>
                      {selected.description}
                    </div>
                    <div style={{ color: 'var(--text-secondary)', fontSize: 12, lineHeight: 1.6, marginBottom: 16 }}>
                      {selected.rationale}
                    </div>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 16 }}>
                      <span style={{ color: 'var(--text-muted)', fontSize: 11 }}>Confidence</span>
                      <div className="prog-track" style={{ flex: 1 }}>
                        <div className="prog-fill" style={{ width: `${(selected.confidence*100).toFixed(0)}%`, background: col }} />
                      </div>
                      <span style={{ color: col, fontWeight: 700, fontSize: 13 }}>{(selected.confidence*100).toFixed(0)}%</span>
                    </div>
                    <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8, marginBottom: 12 }}>
                      <button
                        className="btn-primary btn-sm"
                        disabled={actionLoading || !selected.action_label}
                        onClick={() => applyProposal(selected)}
                      >
                        {actionLoading ? 'Applying…' : selected.action_label || 'No action'}
                      </button>
                      <button className="btn-ghost btn-sm" disabled={actionLoading} onClick={() => ignoreProposal(selected)}>
                        Ignore
                      </button>
                    </div>
                    <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
                      <span className="badge" style={{ fontSize: 9 }}>Risk: {selected.risk}</span>
                      {selected.similarity !== undefined && <span className="badge badge-cyan" style={{ fontSize: 9 }}>Similarity {(selected.similarity * 100).toFixed(0)}%</span>}
                      {selected.entropy !== undefined && <span className="badge badge-amber" style={{ fontSize: 9 }}>Entropy {(selected.entropy * 100).toFixed(0)}%</span>}
                    </div>
                    {involved.length > 0 && (
                      <div style={{ marginTop: 14, paddingTop: 12, borderTop: '1px solid var(--border)' }}>
                        <div style={{ fontSize: 9, fontFamily: 'var(--font-head)', color: 'var(--text-muted)', letterSpacing: '0.1em', marginBottom: 6 }}>
                          INVOLVED NODES
                        </div>
                        <div style={{ display: 'flex', gap: 5, flexWrap: 'wrap' }}>
                          {involved.map(id => {
                            const node = nodeMap.get(id)
                            const label = node?.nickname || node?.content.split('\n')[0] || id.slice(0, 8)
                            return <span key={id} className="badge badge-cyan" title={id} style={{ fontSize: 9 }}>{label.slice(0, 34)}</span>
                          })}
                        </div>
                      </div>
                    )}
                  </div>
                )
              })()}
            </div>
          </div>
        </div>
      </div>

      {/* Synthesis panel */}
      <div style={{ width: 320, flexShrink: 0, display: 'flex', flexDirection: 'column', gap: 10 }}>
        <div className="glass" style={{ display: 'flex', flexDirection: 'column', flex: synResult ? 0.4 : 0, overflow: 'hidden' }}>
          <div className="section-head">
            <span style={{ color: 'var(--emerald)' }}>◈</span> Thought Synthesis
          </div>
          <div style={{ padding: '10px 14px', display: 'flex', flexDirection: 'column', gap: 8 }}>
            <textarea
              placeholder="Concept or question"
              value={synthesis}
              onChange={e => setSynthesis(e.target.value)}
              onKeyDown={e => { if (e.ctrlKey && e.key === 'Enter') runSynthesis() }}
              style={{ minHeight: 80 }}
            />
            <button
              className="btn-primary"
              onClick={runSynthesis}
              disabled={synLoading || !synthesis.trim()}
            >
              {synLoading ? 'Synthesizing…' : 'Synthesize'}
            </button>
          </div>
        </div>

        {synResult && (
          <div className="glass anim-glow-in" style={{ flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
            <div className="section-head">
              <span style={{ color: 'var(--emerald)' }}>◈</span> Result
            </div>
            <div className="scroll-y" style={{ flex: 1, padding: '12px 14px' }}>
              <div style={{ color: 'var(--text-primary)', fontSize: 12, lineHeight: 1.8, whiteSpace: 'pre-wrap', marginBottom: 12 }}>
                {synResult.narrative}
              </div>
              {synResult.related_nodes.length > 0 && (
                <>
                  <div style={{ fontSize: 9, fontFamily: 'var(--font-head)', color: 'var(--text-muted)', letterSpacing: '0.1em', marginBottom: 6 }}>RELATED NODES</div>
                  <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4 }}>
                    {synResult.related_nodes.map(id => {
                      const n = nodeMap.get(id)
                      return (
                        <span key={id} className="badge badge-cyan" style={{ fontSize: 9 }}>
                          {n?.content.slice(0,20) ?? id.slice(0,8)}
                        </span>
                      )
                    })}
                  </div>
                </>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
