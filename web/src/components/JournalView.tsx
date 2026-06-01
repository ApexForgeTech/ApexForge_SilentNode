import { useState } from 'react'
import type { JournalEntry, SeasonReport } from '../types'
import { SEASON_COLORS } from '../types'
import { api } from '../api'
import { toast } from './Toast'

interface Props {
  entries: JournalEntry[]
  season: SeasonReport | null
  onRefresh: () => void
}

export default function JournalView({ entries, season, onRefresh }: Props) {
  const [text,     setText]     = useState('')
  const [saving,   setSaving]   = useState(false)
  const [selected, setSelected] = useState<JournalEntry | null>(null)
  const [search,   setSearch]   = useState('')

  const filtered = [...entries]
    .reverse()
    .filter(e => !search || e.content.toLowerCase().includes(search.toLowerCase()))

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
                onClick={() => setSelected(isActive ? null : e)}
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
              <button
                className="btn-xs"
                style={{ marginLeft: 'auto' }}
                onClick={() => setSelected(null)}
              >✕</button>
            </div>
            <div className="scroll-y" style={{ flex: 1, padding: '16px 20px' }}>
              <div style={{
                color: 'var(--text-primary)', fontSize: 13, lineHeight: 1.9,
                whiteSpace: 'pre-wrap', wordBreak: 'break-word',
              }}>
                {selected.content}
              </div>
              {selected.linked_nodes.length > 0 && (
                <div style={{ marginTop: 20, paddingTop: 12, borderTop: '1px solid var(--border)' }}>
                  <div style={{ color: 'var(--text-muted)', fontSize: 10, marginBottom: 6, fontFamily: 'var(--font-head)', letterSpacing: '0.1em' }}>
                    LINKED NODES
                  </div>
                  <div style={{ display: 'flex', flexWrap: 'wrap', gap: 5 }}>
                    {selected.linked_nodes.map(id => (
                      <span key={id} className="badge badge-cyan" style={{ fontSize: 9 }}>
                        {id.slice(0, 8)}…
                      </span>
                    ))}
                  </div>
                </div>
              )}
            </div>
          </>
        )}
      </div>
    </div>
  )
}
