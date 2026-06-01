import { useRef, useEffect, useState, useCallback } from 'react'
import { api } from '../api'
import { toast } from './Toast'

interface Props { onClose: () => void; onAdded: (id: string) => void }

const TYPE_COLORS: Record<string, string> = {
  idea: '#a78bfa', memory: '#38bdf8', project: '#4ade80',
  person: '#fbbf24', artifact: '#818cf8', media: '#f472b6',
  process: '#86efac', world: '#f0eeff', other: '#94a3b8',
}

export default function AddNodeDialog({ onClose, onAdded }: Props) {
  const [text, setText] = useState('')
  const [nickname, setNickname] = useState('')
  const [kind, setKind] = useState('auto')
  const [customType, setCustomType] = useState('')
  const [customColor, setCustomColor] = useState('#94a3b8')
  const [busy, setBusy] = useState(false)

  // ML classifier state
  const [mlSuggestion, setMlSuggestion] = useState<{
    type: string; confidence: number; alternatives?: { type: string; confidence: number }[]; uncertain?: boolean
  } | null>(null)
  const [mlLoading, setMlLoading] = useState(false)
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  const ref = useRef<HTMLTextAreaElement>(null)
  useEffect(() => { ref.current?.focus() }, [])

  // ML classify while typing (debounced 600ms)
  const classifyText = useCallback((content: string, nick = nickname) => {
    if (debounceRef.current) clearTimeout(debounceRef.current)
    if (content.trim().length < 4) { setMlSuggestion(null); return }

    debounceRef.current = setTimeout(async () => {
      setMlLoading(true)
      try {
        const result = await api.mlClassify(content.trim(), nick.trim() || undefined)
        setMlSuggestion({
          type: result.type,
          confidence: result.confidence,
          alternatives: result.alternatives,
          uncertain: result.uncertain,
        })
      } catch { /* ML not trained yet */ }
      setMlLoading(false)
    }, 600)
  }, [nickname])

  function handleTextChange(val: string) {
    setText(val)
    if (kind === 'auto') classifyText(val)
  }

  function handleNicknameChange(val: string) {
    setNickname(val)
    if (kind === 'auto' && text.trim().length >= 4) classifyText(text, val)
  }

  async function submit() {
    if (!text.trim()) return
    setBusy(true)
    try {
      // If auto and ML suggested with >50% confidence, use suggestion
      const effectiveKind = (kind === 'auto' && mlSuggestion && mlSuggestion.confidence > 0.5)
        ? mlSuggestion.type
        : kind === 'auto' ? 'auto' : kind

      const cleanNickname = nickname.trim() || undefined
      const cleanCustomType = kind === 'other' ? (customType.trim() || 'Other') : undefined
      const cleanCustomColor = kind === 'other' ? customColor : undefined
      const node = effectiveKind === 'auto'
        ? await api.addThought(text.trim(), cleanNickname)
        : await api.createNode(text.trim(), effectiveKind, cleanNickname, cleanCustomType, cleanCustomColor)
      const node_id = 'node_id' in node ? node.node_id : node.id
      if (mlSuggestion) {
        api.mlFeedback({
          node_id,
          content: text.trim(),
          nickname: cleanNickname,
          predicted_type: mlSuggestion.type,
          selected_type: effectiveKind === 'auto' ? mlSuggestion.type : effectiveKind,
          confidence: mlSuggestion.confidence,
          source: kind === 'auto' ? 'auto_accept' : 'manual_override',
        }).catch(() => {})
      } else if (effectiveKind !== 'auto') {
        api.mlFeedback({
          node_id,
          content: text.trim(),
          nickname: cleanNickname,
          selected_type: effectiveKind,
          confidence: 1,
          source: 'manual_type',
        }).catch(() => {})
      }
      toast(`Node added: ${text.slice(0, 30)}`)
      onAdded(node_id)
    } catch (e) { toast(String(e), 'error') }
    setBusy(false)
  }

  const sugCol = mlSuggestion ? (TYPE_COLORS[mlSuggestion.type] ?? 'var(--cyan)') : ''

  return (
    <div className="modal-bg" onClick={e => { if (e.target === e.currentTarget) onClose() }}>
      <div className="modal">
        <div style={{ marginBottom: 14 }}>
          <div style={{ fontSize: 15, fontWeight: 600, color: 'var(--t1)', marginBottom: 4 }}>
            Add a thought
          </div>
          <div style={{ fontSize: 12, color: 'var(--t3)', lineHeight: 1.6 }}>
            Describe your thought. The system will classify it and place it in the universe.
          </div>
        </div>

        <textarea
          ref={ref}
          placeholder="What are you thinking about…"
          value={text}
          onChange={e => handleTextChange(e.target.value)}
          onKeyDown={e => {
            if (e.key === 'Escape') onClose()
            if (e.ctrlKey && e.key === 'Enter') submit()
          }}
          style={{ minHeight: 96, marginBottom: 8 }}
        />

        <input
          type="text"
          placeholder="Nickname (optional, defaults to first 3 words)"
          value={nickname}
          onChange={e => handleNicknameChange(e.target.value)}
          style={{ marginBottom: 8 }}
        />

        {/* ML suggestion chip */}
        {kind === 'auto' && text.trim().length >= 4 && (
          <div style={{ marginBottom: 10, minHeight: 24, display: 'flex', alignItems: 'center', gap: 6 }}>
            {mlLoading && (
              <span style={{ fontSize: 10, color: 'var(--t4)' }}>analyzing…</span>
            )}
            {!mlLoading && mlSuggestion && (
              <>
                <span style={{ fontSize: 10, color: 'var(--t4)' }}>ML suggests:</span>
                <span style={{
                  fontSize: 11, fontWeight: 700,
                  color: sugCol,
                  border: `1px solid ${sugCol}44`,
                  borderRadius: 4, padding: '2px 8px',
                  cursor: 'pointer',
                  background: `${sugCol}11`,
                }} onClick={() => setKind(mlSuggestion.type)}>
                  {mlSuggestion.type}
                </span>
                <span style={{ fontSize: 10, color: 'var(--t4)' }}>
                  {Math.round(mlSuggestion.confidence * 100)}% confident
                </span>
                {mlSuggestion.uncertain && (
                  <span style={{ fontSize: 9, color: 'var(--amber)', marginLeft: 4 }}>
                    review
                  </span>
                )}
                <span style={{ fontSize: 9, color: 'var(--t4)', marginLeft: 4 }}>
                  (click to apply)
                </span>
              </>
            )}
            {!mlLoading && !mlSuggestion && text.trim().length >= 4 && (
              <span style={{ fontSize: 10, color: 'var(--t4)' }}>
                ML model is not trained yet - <code>cargo run -- ml-train</code>
              </span>
            )}
          </div>
        )}

        <select
          value={kind}
          onChange={e => { setKind(e.target.value); setMlSuggestion(null) }}
          style={{ marginBottom: 12 }}
        >
          <option value="auto">Auto classify</option>
          <option value="idea">Idea</option>
          <option value="memory">Memory</option>
          <option value="project">Project</option>
          <option value="person">Person</option>
          <option value="artifact">Artifact</option>
          <option value="media">Media</option>
          <option value="process">Process</option>
          <option value="world">World</option>
          <option value="other">Other</option>
        </select>

        {kind === 'other' && (
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 44px', gap: 8, marginBottom: 12 }}>
            <input
              type="text"
              placeholder="Custom class name"
              value={customType}
              onChange={e => setCustomType(e.target.value)}
            />
            <input
              type="color"
              value={customColor}
              onChange={e => setCustomColor(e.target.value)}
              title="Custom color"
              style={{ height: 34, padding: 2 }}
            />
          </div>
        )}

        <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
          <button onClick={onClose} disabled={busy}>Cancel</button>
          <button className="btn-primary" onClick={submit} disabled={busy || !text.trim()}>
            {busy ? 'Materializing…' : 'Add'}
          </button>
        </div>
      </div>
    </div>
  )
}
