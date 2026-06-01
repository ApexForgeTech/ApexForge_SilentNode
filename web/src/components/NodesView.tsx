import { useEffect, useState } from 'react'
import type { SNode } from '../types'
import { NODE_COLORS, NODE_ICONS } from '../types'
import { api } from '../api'
import { toast } from './Toast'
import NodeDetail from './NodeDetail'

const ALL_TYPES = ['', 'idea', 'memory', 'project', 'person', 'artifact', 'media', 'process', 'world', 'other', 'ghost', 'fossil']

function eColor(e: number) {
  return e > 0.65 ? 'var(--red)' : e > 0.35 ? 'var(--amber)' : 'var(--green)'
}

interface Props {
  nodes: SNode[]
  onRefresh: () => void
  onAddThought: () => void
}

export default function NodesView({ nodes, onRefresh, onAddThought }: Props) {
  const [search,     setSearch]     = useState('')
  const [typeF,      setTypeF]      = useState('')
  const [sort,       setSort]       = useState<'gravity'|'entropy'|'accessed'|'created'>('gravity')
  const [detail,     setDetail]     = useState<SNode | null>(null)
  const [selected,   setSelected]   = useState<Set<string>>(new Set())
  const [connecting, setConnecting] = useState<string | null>(null)

  useEffect(() => {
    if (!detail) return
    const fresh = nodes.find(n => n.id === detail.id)
    if (fresh && fresh !== detail) setDetail(fresh)
  }, [nodes, detail])

  const filtered = nodes
    .filter(n => {
      if (search) {
        const q = search.toLowerCase()
        if (!n.content.toLowerCase().includes(q) && !n.nickname.toLowerCase().includes(q)) return false
      }
      if (typeF  && n.node_type !== typeF) return false
      return true
    })
    .sort((a, b) => {
      if (sort === 'gravity')  return b.gravity - a.gravity
      if (sort === 'entropy')  return b.entropy - a.entropy
      if (sort === 'accessed') return b.accessed_at.localeCompare(a.accessed_at)
      return b.created_at.localeCompare(a.created_at)
    })

  // ── Selection ──────────────────────────────────────────────
  function toggleSelect(id: string, e: React.MouseEvent) {
    e.stopPropagation()
    setSelected(prev => {
      const next = new Set(prev)
      next.has(id) ? next.delete(id) : next.add(id)
      return next
    })
  }
  function selectAll() {
    setSelected(new Set(filtered.map(n => n.id)))
  }
  function clearSelect() { setSelected(new Set()) }

  // ── Bulk delete ────────────────────────────────────────────
  async function bulkDelete() {
    if (!selected.size) return
    if (!confirm(`Delete ${selected.size} nodes?`)) return
    try {
      const r = await api.deleteNodes([...selected])
      toast(`${r.deleted} node(s) deleted`)
      setSelected(new Set())
      onRefresh()
    } catch (e) { toast(String(e), 'error') }
  }

  // ── Quick node actions ─────────────────────────────────────
  async function quickAction(node: SNode, action: string) {
    try {
      if (action === 'void')       { const r = await api.voidToggle(node.id); toast(r.voided ? 'Sent to void' : 'Extracted from void') }
      if (action === 'fossilize')  { await api.fossilize(node.id);  toast('Fossilized') }
      if (action === 'excavate')   { await api.excavate(node.id);   toast('Excavated') }
      if (action === 'revive')     { await api.revive(node.id);     toast('Revived') }
      if (action === 'connect') {
        if (!connecting) { setConnecting(node.id); toast('Now click another node to connect'); return }
        if (connecting === node.id) { setConnecting(null); return }
        await api.connect(connecting, node.id)
        toast('Nodes connected')
        setConnecting(null)
      }
      onRefresh()
    } catch (e) { toast(String(e), 'error') }
  }

  const COL_STYLE = {
    header: { fontSize: 9, fontWeight: 600, letterSpacing: '0.07em', textTransform: 'uppercase' as const, color: 'var(--t4)' }
  }

  return (
    <div style={{ display: 'flex', height: '100%', gap: 10 }}>

      {/* ── Node list ──────────────────────────────────────── */}
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 8, overflow: 'hidden' }}>

        {/* Toolbar */}
        <div className="panel" style={{ padding: '8px 10px', display: 'flex', gap: 7, alignItems: 'center', flexWrap: 'wrap' }}>
          <input
            type="text" placeholder="Search…"
            value={search} onChange={e => setSearch(e.target.value)}
            style={{ flex: 1, minWidth: 100, fontSize: 12 }}
          />
          <select value={typeF} onChange={e => setTypeF(e.target.value)} style={{ padding: '5px 8px', fontSize: 12 }}>
            <option value="">All types</option>
            {ALL_TYPES.slice(1).map(t => <option key={t} value={t}>{t}</option>)}
          </select>
          <select value={sort} onChange={e => setSort(e.target.value as typeof sort)} style={{ padding: '5px 8px', fontSize: 12 }}>
            <option value="gravity">↓ Gravity</option>
            <option value="entropy">↓ Entropy</option>
            <option value="accessed">↓ Last access</option>
            <option value="created">↓ Created</option>
          </select>
          <button className="btn-sm btn-primary" onClick={onAddThought}>+ Add</button>
        </div>

        {/* Bulk action bar */}
        {selected.size > 0 && (
          <div className="panel" style={{
            padding: '6px 10px', display: 'flex', gap: 8, alignItems: 'center',
            background: 'rgba(248,113,113,0.06)', borderColor: 'rgba(248,113,113,0.2)',
          }}>
            <span style={{ color: 'var(--t2)', fontSize: 12 }}>
              {selected.size} selected
            </span>
            <button className="btn-sm btn-danger" onClick={bulkDelete}>Delete selected</button>
            <button className="btn-sm" onClick={clearSelect}>Clear</button>
            <button className="btn-sm" onClick={selectAll} style={{ marginLeft: 'auto' }}>Select all ({filtered.length})</button>
          </div>
        )}

        {/* Connect mode banner */}
        {connecting && (
          <div className="panel" style={{
            padding: '7px 12px', display: 'flex', gap: 10, alignItems: 'center',
            background: 'rgba(167,139,250,0.07)', borderColor: 'rgba(167,139,250,0.3)',
          }}>
            <span style={{ color: 'var(--lavender-text)', fontSize: 12 }}>
              Link mode — click target node to connect
            </span>
            <button className="btn-xs" onClick={() => setConnecting(null)} style={{ marginLeft: 'auto' }}>Cancel</button>
          </div>
        )}

        {/* Table */}
        <div className="panel" style={{ flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
          {/* Header */}
          <div style={{
            display: 'grid',
            gridTemplateColumns: '28px 28px 150px 1fr 70px 60px 54px 54px 90px',
            padding: '6px 10px', gap: 6,
            borderBottom: '1px solid var(--line)',
            background: 'var(--surface)',
            position: 'sticky', top: 0,
          }}>
            {['', '⬡', 'Name', 'Content', 'Type', 'Gravity', 'Entropy', 'Access', 'Actions'].map(h => (
              <div key={h} style={COL_STYLE.header}>{h}</div>
            ))}
          </div>

          <div className="scroll" style={{ flex: 1 }}>
            {filtered.length === 0 && (
              <div style={{ padding: 20, color: 'var(--t4)', fontSize: 12, textAlign: 'center' }}>
                {search || typeF ? 'No nodes match filters' : 'No nodes — press N to add one'}
              </div>
            )}

            {filtered.map(n => {
              const col   = n.node_type === 'other' && (n.custom_color || n.aura_color)
                ? (n.custom_color || n.aura_color)
                : (NODE_COLORS[n.node_type] ?? 'var(--lavender-text)')
              const icon  = NODE_ICONS[n.node_type]  ?? '◆'
              const typeLabel = n.node_type === 'other' ? (n.custom_type || 'other') : n.node_type
              const isSel = selected.has(n.id)
              const isCon = connecting === n.id
              const isTgt = Boolean(connecting) && !isCon
              const eCol  = eColor(n.entropy)

              return (
                <div
                  key={n.id}
                  onClick={() => {
                    if (connecting && !isCon) { quickAction(n, 'connect'); return }
                    setDetail(prev => prev?.id === n.id ? null : n)
                  }}
                  style={{
                    display: 'grid',
                    gridTemplateColumns: '28px 28px 150px 1fr 70px 60px 54px 54px 90px',
                    padding: '5px 10px', gap: 6,
                    alignItems: 'center',
                    borderBottom: '1px solid rgba(255,255,255,0.04)',
                    cursor: 'pointer',
                    background: isCon
                      ? 'rgba(167,139,250,0.12)'
                      : isTgt
                      ? 'rgba(167,139,250,0.04)'
                      : isSel
                      ? 'rgba(248,113,113,0.06)'
                      : detail?.id === n.id
                      ? 'rgba(167,139,250,0.06)'
                      : 'transparent',
                    borderLeft: detail?.id === n.id ? '2px solid rgba(167,139,250,0.4)' : '2px solid transparent',
                    transition: 'background 0.1s',
                  }}
                >
                  {/* Checkbox */}
                  <div onClick={e => toggleSelect(n.id, e)} style={{ display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                    <div style={{
                      width: 14, height: 14, borderRadius: 3,
                      border: `1px solid ${isSel ? 'rgba(248,113,113,0.6)' : 'var(--line-mid)'}`,
                      background: isSel ? 'rgba(248,113,113,0.2)' : 'transparent',
                      display: 'flex', alignItems: 'center', justifyContent: 'center',
                      fontSize: 9, color: 'var(--red)',
                    }}>
                      {isSel && '✓'}
                    </div>
                  </div>

                  {/* Icon */}
                  <span style={{ color: col, fontSize: 14, textAlign: 'center' }}>{icon}</span>

                  {/* Nickname */}
                  <div style={{
                    minWidth: 0,
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                    fontSize: 12,
                    color: 'var(--lavender-text)',
                    fontWeight: 600,
                  }}>
                    {n.nickname}
                  </div>

                  {/* Content */}
                  <div style={{ minWidth: 0 }}>
                    <div style={{
                      overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                      fontSize: 12, color: 'var(--t1)',
                    }}>
                      {n.content}
                    </div>
                    <div style={{ display: 'flex', gap: 4, marginTop: 2, flexWrap: 'wrap' }}>
                      {n.is_ghost  && <span className="badge badge-mt" style={{ fontSize: 8 }}>ghost</span>}
                      {n.is_fossil && <span className="badge badge-am" style={{ fontSize: 8 }}>fossil</span>}
                      {n.is_void   && <span className="badge badge-lv" style={{ fontSize: 8 }}>void</span>}
                    </div>
                  </div>

                  {/* Type */}
                  <span style={{ color: col, fontSize: 10 }}>{typeLabel}</span>

                  {/* Gravity */}
                  <span style={{ color: 'var(--lavender-text)', fontSize: 11, fontFamily: 'var(--font-mono)' }}>
                    {n.gravity.toFixed(2)}
                  </span>

                  {/* Entropy */}
                  <span style={{ color: eCol, fontSize: 11, fontFamily: 'var(--font-mono)' }}>
                    {(n.entropy * 100).toFixed(0)}%
                  </span>

                  {/* Access */}
                  <span style={{ color: 'var(--t4)', fontSize: 10 }}>{n.access_count}</span>

                  {/* Quick actions */}
                  <div
                    style={{ display: 'flex', gap: 3 }}
                    onClick={e => e.stopPropagation()}
                  >
                    <button
                      className="btn-xs btn-ghost"
                      title={n.is_void ? 'Un-void' : 'Send to void'}
                      style={{ padding: '1px 5px', fontSize: 9, color: n.is_void ? 'var(--lavender-text)' : 'var(--t4)' }}
                      onClick={() => quickAction(n, 'void')}
                    >
                      {n.is_void ? '◈↑' : '◈↓'}
                    </button>
                    {n.is_ghost && (
                      <button className="btn-xs btn-ghost" title="Revive" style={{ padding: '1px 5px', fontSize: 9 }}
                        onClick={() => quickAction(n, 'revive')}>↑</button>
                    )}
                    {!n.is_fossil && !n.is_ghost && (
                      <button className="btn-xs btn-ghost" title="Fossilize" style={{ padding: '1px 5px', fontSize: 9 }}
                        onClick={() => quickAction(n, 'fossilize')}>◫</button>
                    )}
                    {n.is_fossil && (
                      <button className="btn-xs btn-ghost" title="Excavate" style={{ padding: '1px 5px', fontSize: 9, color: 'var(--amber)' }}
                        onClick={() => quickAction(n, 'excavate')}>⛏</button>
                    )}
                    <button
                      className="btn-xs btn-ghost"
                      title={connecting === n.id ? 'Cancel link' : 'Start link'}
                      style={{ padding: '1px 5px', fontSize: 9, color: connecting === n.id ? 'var(--lavender-text)' : 'var(--t4)' }}
                      onClick={() => quickAction(n, 'connect')}
                    >
                      ⟿
                    </button>
                  </div>
                </div>
              )
            })}
          </div>
        </div>

        <div style={{ color: 'var(--t4)', fontSize: 10 }}>
          {filtered.length}/{nodes.length} nodes
          {selected.size > 0 && <span style={{ color: 'var(--red)', marginLeft: 8 }}>{selected.size} selected</span>}
          {connecting && <span style={{ color: 'var(--lavender-text)', marginLeft: 8 }}>Link mode active</span>}
        </div>
      </div>

      {/* ── Detail panel ───────────────────────────────────── */}
      {detail && (
        <div style={{ width: 290, flexShrink: 0, overflow: 'hidden' }}>
          <div className="panel" style={{ height: '100%', background: 'var(--surface)' }}>
            <NodeDetail node={detail} nodes={nodes} onClose={() => setDetail(null)} onRefresh={onRefresh} />
          </div>
        </div>
      )}
    </div>
  )
}
