import { useState, useEffect } from 'react'
import type { MembraneStatus, MembraneRule } from '../types'
import { api } from '../api'
import { toast } from './Toast'

const DIR_COLORS: Record<string, string> = {
  inbound:  'var(--sky)',
  outbound: 'var(--amber)',
  both:     'var(--lavender-text)',
}

export default function MembraneView() {
  const [status, setStatus] = useState<MembraneStatus | null>(null)
  const [loading, setLoading] = useState(true)
  const [adding, setAdding]   = useState(false)
  const [deleting, setDeleting] = useState<string | null>(null)

  // Form
  const [pattern,   setPattern]   = useState('')
  const [direction, setDirection] = useState('both')
  const [allow,     setAllow]     = useState(true)
  const [desc,      setDesc]      = useState('')
  const [submitting, setSubmitting] = useState(false)

  useEffect(() => { load() }, [])

  async function load() {
    setLoading(true)
    try {
      setStatus(await api.membrane())
    } catch (e) { /* offline */ }
    setLoading(false)
  }

  async function addRule() {
    if (!pattern.trim()) { toast('Pattern required', 'error'); return }
    setSubmitting(true)
    try {
      await api.addMembraneRule({
        pattern: pattern.trim(),
        direction,
        allow,
        description: desc.trim() || undefined,
      })
      toast('Rule added')
      setAdding(false); setPattern(''); setDesc('')
      load()
    } catch (e) { toast(String(e), 'error') }
    setSubmitting(false)
  }

  async function removeRule(id: string) {
    setDeleting(id)
    try {
      await api.deleteMembraneRule(id)
      toast('Rule removed')
      load()
    } catch (e) { toast(String(e), 'error') }
    setDeleting(null)
  }

  const integrity = status?.integrity_score ?? 0
  const intCol = integrity > 0.7 ? 'var(--green)' : integrity > 0.4 ? 'var(--amber)' : 'var(--red)'

  return (
    <div className="split">
      {/* Left: rule list */}
      <div className="split-list panel">
        <div className="sec-head">
          <span style={{ color: 'var(--lavender-text)' }}>⬡</span>
          Digital Membrane
          <button
            className="btn-xs btn-primary"
            style={{ marginLeft: 'auto' }}
            onClick={() => setAdding(a => !a)}
          >
            {adding ? '✕' : '+ Rule'}
          </button>
        </div>

        {/* Add rule form */}
        {adding && (
          <div style={{ padding: '10px 12px', borderBottom: '1px solid var(--line)', display: 'flex', flexDirection: 'column', gap: 6 }}>
            <input
              type="text"
              placeholder="Pattern (e.g. *.github.com, /home/*)"
              value={pattern}
              onChange={e => setPattern(e.target.value)}
            />
            <div style={{ display: 'flex', gap: 6 }}>
              <select value={direction} onChange={e => setDirection(e.target.value)} style={{ flex: 1 }}>
                <option value="both">Both</option>
                <option value="inbound">Inbound</option>
                <option value="outbound">Outbound</option>
              </select>
              <select value={allow ? 'allow' : 'block'} onChange={e => setAllow(e.target.value === 'allow')} style={{ flex: 1 }}>
                <option value="allow">Allow</option>
                <option value="block">Block</option>
              </select>
            </div>
            <input
              type="text"
              placeholder="Description (optional)…"
              value={desc}
              onChange={e => setDesc(e.target.value)}
            />
            <button className="btn-primary btn-sm" onClick={addRule} disabled={submitting}>
              {submitting ? 'Adding…' : 'Add Rule'}
            </button>
          </div>
        )}

        <div className="scroll fill">
          {loading && <div style={{ padding: 16, color: 'var(--t4)', fontSize: 11 }}>Loading membrane…</div>}
          {!loading && status?.rules.length === 0 && (
            <div style={{ padding: 16, color: 'var(--t4)', fontSize: 11, lineHeight: 1.6 }}>
              No rules — the membrane is fully permeable.
              Add rules to control what enters and exits the cognitive universe.
            </div>
          )}
          {status?.rules.map(rule => (
            <RuleRow key={rule.id} rule={rule} onDelete={removeRule} deleting={deleting === rule.id} />
          ))}
        </div>
      </div>

      {/* Right: status */}
      <div className="split-detail" style={{ gap: 10 }}>
        {/* Integrity gauge */}
        <div className="panel anim-in" style={{ flexShrink: 0 }}>
          <div className="sec-head">
            <span style={{ color: intCol }}>◈</span>
            Membrane Integrity
          </div>
          <div style={{ padding: '16px 20px', display: 'flex', gap: 20, alignItems: 'center' }}>
            {/* Radial-style display */}
            <div style={{
              width: 80, height: 80, borderRadius: '50%',
              border: `3px solid ${intCol}`,
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              flexShrink: 0,
              boxShadow: `0 0 20px ${intCol}33`,
              position: 'relative',
            }}>
              <span style={{ fontSize: 18, fontWeight: 700, color: intCol, fontFamily: 'var(--font-mono)' }}>
                {(integrity * 100).toFixed(0)}
              </span>
              <span style={{ position: 'absolute', bottom: 10, fontSize: 9, color: intCol }}>%</span>
            </div>
            <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 8 }}>
              {status && [
                ['Rules',   status.rule_count,    'var(--lavender-text)'],
                ['Blocked', status.blocked_count, status.blocked_count > 0 ? 'var(--red)' : 'var(--t4)'],
              ].map(([l, v, c]) => (
                <div key={String(l)} className="m-row" style={{ padding: '3px 0' }}>
                  <span className="m-label">{l}</span>
                  <span className="m-val" style={{ color: String(c) }}>{v}</span>
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* What is the membrane */}
        <div className="panel fill scroll">
          <div className="sec-head">
            <span style={{ color: 'var(--t3)' }}>◌</span>
            About the Membrane
          </div>
          <div style={{ padding: '12px 14px', display: 'flex', flexDirection: 'column', gap: 10, fontSize: 11, color: 'var(--t3)', lineHeight: 1.7 }}>
            <p>
              The Digital Membrane is the architectural boundary between SilentNode's internal universe and the external digital world.
            </p>
            <p>
              Every external crossing — inbound data, outbound requests — passes through here.
              Rules determine what is <span style={{ color: 'var(--green)' }}>allowed</span> and what is <span style={{ color: 'var(--red)' }}>blocked</span>.
            </p>
            <div style={{ borderTop: '1px solid var(--line)', paddingTop: 10 }}>
              <div style={{ fontSize: 9, color: 'var(--t4)', letterSpacing: '0.08em', textTransform: 'uppercase', marginBottom: 8 }}>
                Direction Legend
              </div>
              {[
                ['inbound',  'Data entering the universe', 'var(--sky)'],
                ['outbound', 'Data leaving the universe',  'var(--amber)'],
                ['both',     'Both directions',            'var(--lavender-text)'],
              ].map(([k, v, c]) => (
                <div key={k} style={{ display: 'flex', gap: 8, alignItems: 'center', marginBottom: 4 }}>
                  <span className="badge" style={{
                    color: String(c), borderColor: String(c) + '44',
                    background: String(c) + '11', minWidth: 70, justifyContent: 'center',
                    fontSize: 9,
                  }}>{k}</span>
                  <span style={{ color: 'var(--t3)', fontSize: 10 }}>{v}</span>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

function RuleRow({
  rule, onDelete, deleting,
}: {
  rule: MembraneRule
  onDelete: (id: string) => void
  deleting: boolean
}) {
  const dirCol = DIR_COLORS[rule.direction] ?? 'var(--t3)'
  return (
    <div style={{
      padding: '8px 12px',
      borderBottom: '1px solid rgba(255,255,255,0.04)',
      display: 'flex', flexDirection: 'column', gap: 4,
    }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
        <span style={{
          width: 7, height: 7, borderRadius: '50%', flexShrink: 0,
          background: rule.allow ? 'var(--green)' : 'var(--red)',
        }} />
        <span style={{
          fontSize: 11, fontFamily: 'var(--font-mono)', color: 'var(--t1)',
          flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
        }}>
          {rule.pattern}
        </span>
        <span className="badge" style={{
          fontSize: 8, color: dirCol, borderColor: dirCol + '44', background: dirCol + '11',
        }}>
          {rule.direction}
        </span>
        <span className="badge" style={{
          fontSize: 8,
          color: rule.allow ? 'var(--green)' : 'var(--red)',
          borderColor: (rule.allow ? 'var(--green)' : 'var(--red)') + '44',
          background: (rule.allow ? 'var(--green)' : 'var(--red)') + '11',
        }}>
          {rule.allow ? 'allow' : 'block'}
        </span>
        <button
          className="btn-ghost btn-xs"
          style={{ opacity: 0.4, padding: '2px 5px' }}
          onClick={() => onDelete(rule.id)}
          disabled={deleting}
        >
          {deleting ? '…' : '✕'}
        </button>
      </div>
      {rule.description && (
        <div style={{ fontSize: 10, color: 'var(--t4)', lineHeight: 1.4 }}>{rule.description}</div>
      )}
    </div>
  )
}
