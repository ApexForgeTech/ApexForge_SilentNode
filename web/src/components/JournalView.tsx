import { useEffect, useMemo, useState } from 'react'
import type { JournalEntry, SeasonReport, SNode } from '../types'
import { SEASON_COLORS } from '../types'
import { api } from '../api'
import { toast } from './Toast'

interface Props {
  entries: JournalEntry[]
  nodes: SNode[]
  season: SeasonReport | null
  onRefresh: () => void
}

export default function JournalView({ entries, nodes, season, onRefresh }: Props) {
  const [text,     setText]     = useState('')
  const [saving,   setSaving]   = useState(false)
  const [selected, setSelected] = useState<JournalEntry | null>(null)
  const [search,   setSearch]   = useState('')
  const [editing,  setEditing]  = useState(false)
  const [editText, setEditText] = useState('')
  const [mutating, setMutating] = useState(false)
  const [linkQuery, setLinkQuery] = useState('')
  const [focusSecondsByNode, setFocusSecondsByNode] = useState<Record<string, number>>({})

  const filtered = [...entries]
    .reverse()
    .filter(e => !search || e.content.toLowerCase().includes(search.toLowerCase()))
  const nodeMap = new Map(nodes.map(node => [node.id, node]))
  const selectedLinked = useMemo(
    () => new Set(selected?.linked_nodes ?? []),
    [selected?.id, selected?.linked_nodes.join('|')]
  )
  const linkMatches = nodes
    .filter(node => !selectedLinked.has(node.id))
    .filter(node => {
      const q = linkQuery.trim().toLowerCase()
      if (!q) return false
      return node.nickname.toLowerCase().includes(q) || node.content.toLowerCase().includes(q)
    })
    .slice(0, 8)

  useEffect(() => {
    api.trail(24 * 30)
      .then(events => {
        const totals: Record<string, number> = {}
        for (const event of events) {
          totals[event.node_id] = (totals[event.node_id] || 0) + event.duration_seconds
        }
        setFocusSecondsByNode(totals)
      })
      .catch(() => {})
  }, [])

  async function save() {
    if (!text.trim()) return
    setSaving(true)
    try {
      await api.addJournal(text.trim(), season?.season?.toLowerCase())
      setText('')
      toast('Journal entry saved')
      onRefresh()
    } catch (e) { toast(String(e), 'error') }
    setSaving(false)
  }

  function startEdit(entry: JournalEntry) {
    setEditing(true)
    setEditText(entry.content)
  }

  function closeDetail() {
    setSelected(null)
    setEditing(false)
    setEditText('')
    setLinkQuery('')
  }

  async function saveEdit() {
    if (!selected || !editText.trim()) return
    setMutating(true)
    try {
      const updated = await api.updateJournal(selected.id, editText.trim(), selected.season, selected.linked_nodes)
      setSelected(updated)
      setEditing(false)
      setEditText('')
      toast('Journal entry updated')
      onRefresh()
    } catch (e) { toast(String(e), 'error') }
    setMutating(false)
  }

  async function deleteEntry() {
    if (!selected) return
    const ok = window.confirm('Delete this journal entry?')
    if (!ok) return
    setMutating(true)
    try {
      await api.deleteJournal(selected.id)
      toast('Journal entry deleted')
      closeDetail()
      onRefresh()
    } catch (e) { toast(String(e), 'error') }
    setMutating(false)
  }

  async function quickLinkNode(nodeId: string) {
    if (!selected) return
    const nextLinks = Array.from(new Set([...selected.linked_nodes, nodeId]))
    setMutating(true)
    try {
      const updated = await api.updateJournal(selected.id, selected.content, selected.season, nextLinks)
      setSelected(updated)
      setLinkQuery('')
      toast('Journal linked to node')
      onRefresh()
    } catch (e) { toast(String(e), 'error') }
    setMutating(false)
  }

  async function unlinkNode(nodeId: string) {
    if (!selected) return
    const nextLinks = selected.linked_nodes.filter(id => id !== nodeId)
    setMutating(true)
    try {
      const updated = await api.updateJournal(selected.id, selected.content, selected.season, nextLinks)
      setSelected(updated)
      toast('Journal link removed')
      onRefresh()
    } catch (e) { toast(String(e), 'error') }
    setMutating(false)
  }

  function focusLabel(seconds: number) {
    if (!seconds) return 'no focus logged'
    const mins = Math.round(seconds / 60)
    if (mins < 60) return `${mins}m focus`
    const hours = Math.floor(mins / 60)
    const rest = mins % 60
    return rest ? `${hours}h ${rest}m focus` : `${hours}h focus`
  }

  return (
    <div style={{ display: 'flex', height: '100%', gap: 12 }}>

      {/* Left: new entry + list */}
      <div style={{ width: 280, display: 'flex', flexDirection: 'column', gap: 8, flexShrink: 0 }}>

        {/* New entry */}
        <div className="glass" style={{ padding: '12px 14px', display: 'flex', flexDirection: 'column', gap: 8 }}>
          <div className="section-head" style={{ padding: 0, border: 'none', fontSize: 9, marginBottom: 2 }}>
            <span style={{ color: 'var(--cyan)' }}>✎</span> New Entry
            {season && <span style={{ marginLeft: 'auto', fontSize: 9, color: SEASON_COLORS[season.season] ?? 'var(--text-muted)' }}>
              {season.season}
            </span>}
          </div>
          <textarea
            placeholder="What occupies your mind…"
            value={text}
            onChange={e => setText(e.target.value)}
            onKeyDown={e => { if (e.ctrlKey && e.key === 'Enter') save() }}
            style={{ minHeight: 90, fontSize: 12 }}
          />
          <button
            className="btn-primary"
            onClick={save}
            disabled={saving || !text.trim()}
          >
            {saving ? 'Saving…' : 'Save  Ctrl+Enter'}
          </button>
        </div>

        {/* Search */}
        <input
          type="text"
          placeholder="Search journal…"
          value={search}
          onChange={e => setSearch(e.target.value)}
          style={{ padding: '6px 10px' }}
        />

        {/* Entry list */}
        <div className="glass scroll-y" style={{ flex: 1, minHeight: 0 }}>
          <div className="section-head">
            Entries <span style={{ marginLeft: 4, color: 'var(--text-muted)' }}>({filtered.length})</span>
          </div>
          {filtered.length === 0 && (
            <div style={{ padding: '12px', color: 'var(--text-muted)', fontSize: 11 }}>
              {search ? 'No results' : 'No journal entries yet'}
            </div>
          )}
          {filtered.map(e => {
            const sc = SEASON_COLORS[e.season ?? ''] ?? 'var(--text-muted)'
            const isActive = selected?.id === e.id
            return (
              <div
                key={e.id}
                className={`list-item ${isActive ? 'active' : ''}`}
                onClick={() => {
                  setSelected(isActive ? null : e)
                  setEditing(false)
                  setEditText('')
                }}
              >
                <div style={{ width: 6, height: 6, borderRadius: '50%', background: sc, marginTop: 4, flexShrink: 0 }} />
                <div style={{ minWidth: 0 }}>
                  <div style={{ color: 'var(--text-muted)', fontSize: 9, marginBottom: 2 }}>
                    {e.timestamp.slice(0, 16).replace('T', '  ')}
                  </div>
                  <div style={{ color: 'var(--text-primary)', fontSize: 11, lineHeight: 1.4, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                    {e.content}
                  </div>
                </div>
              </div>
            )
          })}
        </div>
      </div>

      {/* Right: detail */}
      <div className="glass" style={{ flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
        {!selected ? (
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', color: 'var(--text-muted)', flexDirection: 'column', gap: 8 }}>
            <div style={{ fontSize: 32, color: 'var(--border)' }}>✎</div>
            <div style={{ fontSize: 12 }}>Select an entry to read</div>
          </div>
        ) : (
          <>
            <div className="section-head">
              <div style={{ color: SEASON_COLORS[selected.season ?? ''] ?? 'var(--text-muted)' }}>●</div>
              <span style={{ color: 'var(--text-secondary)' }}>{selected.timestamp.slice(0,16).replace('T','  ')}</span>
              {selected.season && (
                <span className="badge badge-cyan" style={{ marginLeft: 6 }}>{selected.season}</span>
              )}
              {!editing && (
                <>
                  <button
                    className="btn-xs"
                    style={{ marginLeft: 'auto' }}
                    onClick={() => startEdit(selected)}
                  >Edit</button>
                  <button
                    className="btn-xs btn-danger"
                    onClick={deleteEntry}
                    disabled={mutating}
                  >Delete</button>
                </>
              )}
              <button
                className="btn-xs"
                style={{ marginLeft: editing ? 'auto' : 0 }}
                onClick={closeDetail}
              >✕</button>
            </div>
            <div className="scroll-y" style={{ flex: 1, padding: '16px 20px' }}>
              {editing ? (
                <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
                  <textarea
                    value={editText}
                    onChange={e => setEditText(e.target.value)}
                    onKeyDown={e => { if (e.ctrlKey && e.key === 'Enter') saveEdit() }}
                    style={{ minHeight: 220, fontSize: 13, lineHeight: 1.7 }}
                  />
                  <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
                    <button
                      className="btn-xs"
                      onClick={() => {
                        setEditing(false)
                        setEditText('')
                      }}
                      disabled={mutating}
                    >Cancel</button>
                    <button
                      className="btn-primary"
                      onClick={saveEdit}
                      disabled={mutating || !editText.trim()}
                    >{mutating ? 'Saving…' : 'Save Changes'}</button>
                  </div>
                </div>
              ) : (
                <div style={{
                  color: 'var(--text-primary)', fontSize: 13, lineHeight: 1.9,
                  whiteSpace: 'pre-wrap', wordBreak: 'break-word',
                }}>
                  {selected.content}
                </div>
              )}
              {selected.linked_nodes.length > 0 && (
                <div style={{ marginTop: 20, paddingTop: 12, borderTop: '1px solid var(--border)' }}>
                  <div style={{ color: 'var(--text-muted)', fontSize: 10, marginBottom: 6, fontFamily: 'var(--font-head)', letterSpacing: '0.1em' }}>
                    LINKED NODES
                  </div>
                  <div style={{ display: 'flex', flexWrap: 'wrap', gap: 5 }}>
                    {selected.linked_nodes.map((id, index) => {
                      const node = nodeMap.get(id)
                      const label =
                        node?.nickname ||
                        node?.content.split('\n')[0] ||
                        selected.linked_node_previews?.[index] ||
                        `${id.slice(0, 8)}…`
                      return (
                        <button
                          key={id}
                          className="badge badge-cyan"
                          title={`${id} · click to unlink`}
                          disabled={mutating}
                          onClick={() => unlinkNode(id)}
                          style={{ fontSize: 9, cursor: 'pointer' }}
                        >
                          {label.slice(0, 34)} ×
                        </button>
                      )
                    })}
                  </div>
                </div>
              )}
              {selected && (
                <div style={{ marginTop: 14, paddingTop: 12, borderTop: '1px solid var(--border)' }}>
                  <div style={{ color: 'var(--text-muted)', fontSize: 10, marginBottom: 6, fontFamily: 'var(--font-head)', letterSpacing: '0.1em' }}>
                    QUICK LINK NODE
                  </div>
                  <input
                    type="text"
                    placeholder="Search existing nodes…"
                    value={linkQuery}
                    onChange={e => setLinkQuery(e.target.value)}
                    style={{ padding: '6px 10px', marginBottom: 8 }}
                  />
                  {linkMatches.length > 0 && (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: 5 }}>
                      {linkMatches.map(node => (
                        <button
                          key={node.id}
                          className="btn-xs"
                          disabled={mutating}
                          onClick={() => quickLinkNode(node.id)}
                          style={{ textAlign: 'left', display: 'flex', justifyContent: 'space-between', gap: 8 }}
                        >
                          <span>{(node.nickname || node.content.split('\n')[0]).slice(0, 42)}</span>
                          <span style={{ color: 'var(--text-muted)', fontFamily: 'var(--font-mono)' }}>
                            {focusLabel(focusSecondsByNode[node.id] || 0)}
                          </span>
                        </button>
                      ))}
                    </div>
                  )}
                </div>
              )}
            </div>
          </>
        )}
      </div>
    </div>
  )
}
