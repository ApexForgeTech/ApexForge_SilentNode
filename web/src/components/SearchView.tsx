import { useState, useCallback, useRef } from 'react'
import type { SearchResultItem } from '../types'
import { NODE_COLORS, NODE_ICONS } from '../types'
import { api } from '../api'

const KIND_ICON: Record<string, string> = {
  node: '◈',
  journal: '✦',
}

const KIND_COLOR: Record<string, string> = {
  node: 'var(--lavender-text)',
  journal: 'var(--amber)',
}

function timeAgo(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime()
  const m = Math.floor(diff / 60000)
  if (m < 1) return 'just now'
  if (m < 60) return `${m}m ago`
  const h = Math.floor(m / 60)
  if (h < 24) return `${h}h ago`
  const d = Math.floor(h / 24)
  return `${d}d ago`
}

export default function SearchView() {
  const [query, setQuery]     = useState('')
  const [items, setItems]     = useState<SearchResultItem[]>([])
  const [loading, setLoading] = useState(false)
  const [searched, setSearched] = useState(false)
  const debounce = useRef<ReturnType<typeof setTimeout> | null>(null)

  const runSearch = useCallback((q: string) => {
    if (!q.trim()) { setItems([]); setSearched(false); return }
    setLoading(true)
    api.search(q.trim(), 60)
      .then(r => { setItems(r.items); setSearched(true) })
      .catch(() => setItems([]))
      .finally(() => setLoading(false))
  }, [])

  function handleChange(e: React.ChangeEvent<HTMLInputElement>) {
    const v = e.target.value
    setQuery(v)
    if (debounce.current) clearTimeout(debounce.current)
    debounce.current = setTimeout(() => runSearch(v), 300)
  }

  function handleKey(e: React.KeyboardEvent<HTMLInputElement>) {
    if (e.key === 'Enter') {
      if (debounce.current) clearTimeout(debounce.current)
      runSearch(query)
    }
    if (e.key === 'Escape') { setQuery(''); setItems([]); setSearched(false) }
  }

  const nodes   = items.filter(i => i.kind === 'node')
  const journal = items.filter(i => i.kind === 'journal')

  return (
    <div className="panel" style={{ display: 'flex', flexDirection: 'column', gap: 14, padding: '16px 14px', height: '100%', overflow: 'hidden' }}>

      {/* ── Search bar ── */}
      <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
        <span style={{ color: 'var(--t3)', fontSize: 16 }}>⌕</span>
        <input
          autoFocus
          value={query}
          onChange={handleChange}
          onKeyDown={handleKey}
          placeholder="Search nodes, journal…"
          style={{
            flex: 1,
            background: 'var(--surface2)',
            border: '1px solid var(--border)',
            borderRadius: 6,
            padding: '7px 12px',
            color: 'var(--t1)',
            fontSize: 14,
            outline: 'none',
          }}
        />
        {loading && <span style={{ color: 'var(--t3)', fontSize: 12 }}>…</span>}
        {query && !loading && (
          <button className="btn-xs btn-ghost" onClick={() => { setQuery(''); setItems([]); setSearched(false) }}>✕</button>
        )}
      </div>

      {/* ── Results ── */}
      <div className="scroll" style={{ flex: 1 }}>
        {!searched && !loading && (
          <div style={{ color: 'var(--t3)', fontSize: 13, textAlign: 'center', marginTop: 40 }}>
            Type to search across your graph and journal
          </div>
        )}

        {searched && items.length === 0 && (
          <div style={{ color: 'var(--t3)', fontSize: 13, textAlign: 'center', marginTop: 40 }}>
            No results for <strong style={{ color: 'var(--t2)' }}>"{query}"</strong>
          </div>
        )}

        {nodes.length > 0 && (
          <section style={{ marginBottom: 18 }}>
            <div style={{ fontSize: 10, color: 'var(--t3)', letterSpacing: 1, textTransform: 'uppercase', marginBottom: 6 }}>
              Nodes · {nodes.length}
            </div>
            {nodes.map(item => (
              <ResultRow key={item.id} item={item} query={query} />
            ))}
          </section>
        )}

        {journal.length > 0 && (
          <section style={{ marginBottom: 18 }}>
            <div style={{ fontSize: 10, color: 'var(--t3)', letterSpacing: 1, textTransform: 'uppercase', marginBottom: 6 }}>
              Journal · {journal.length}
            </div>
            {journal.map(item => (
              <ResultRow key={item.id} item={item} query={query} />
            ))}
          </section>
        )}
      </div>

      {searched && items.length > 0 && (
        <div style={{ fontSize: 11, color: 'var(--t3)', textAlign: 'right' }}>
          {items.length} result{items.length !== 1 ? 's' : ''}
        </div>
      )}
    </div>
  )
}

function highlight(text: string, query: string): React.ReactNode {
  if (!query) return text
  const idx = text.toLowerCase().indexOf(query.toLowerCase())
  if (idx === -1) return text
  return (
    <>
      {text.slice(0, idx)}
      <mark style={{ background: 'var(--amber)', color: 'var(--bg)', borderRadius: 2, padding: '0 1px' }}>
        {text.slice(idx, idx + query.length)}
      </mark>
      {text.slice(idx + query.length)}
    </>
  )
}

function ResultRow({ item, query }: { item: SearchResultItem; query: string }) {
  const icon  = item.kind === 'node'
    ? (NODE_ICONS[item.node_type as keyof typeof NODE_ICONS] ?? '◈')
    : KIND_ICON.journal
  const color = item.kind === 'node'
    ? (NODE_COLORS[item.node_type as keyof typeof NODE_COLORS] ?? KIND_COLOR.node)
    : KIND_COLOR.journal

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: 3,
        padding: '8px 10px',
        marginBottom: 4,
        background: 'var(--surface2)',
        borderRadius: 7,
        borderLeft: `3px solid ${color}`,
        cursor: 'default',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
        <span style={{ color, fontSize: 13 }}>{icon}</span>
        <span style={{ color: 'var(--t1)', fontSize: 13, fontWeight: 500, flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
          {highlight(item.title, query)}
        </span>
        {item.node_type && (
          <span style={{ fontSize: 10, color: 'var(--t3)', background: 'var(--surface)', borderRadius: 4, padding: '1px 5px' }}>
            {item.node_type}
          </span>
        )}
        {item.timestamp && (
          <span style={{ fontSize: 10, color: 'var(--t3)', whiteSpace: 'nowrap' }}>
            {timeAgo(item.timestamp)}
          </span>
        )}
      </div>
      {item.preview !== item.title && (
        <div style={{ fontSize: 12, color: 'var(--t3)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', paddingLeft: 19 }}>
          {highlight(item.preview, query)}
        </div>
      )}
    </div>
  )
}
