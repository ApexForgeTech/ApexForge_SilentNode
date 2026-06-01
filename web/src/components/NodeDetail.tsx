import { useState, useEffect } from 'react'
import type { SNode } from '../types'
import { api } from '../api'
import { toast } from './Toast'

const NODE_COLORS: Record<string, string> = {
  idea: '#a78bfa', memory: '#38bdf8', project: '#4ade80',
  person: '#fbbf24', artifact: '#818cf8', media: '#f472b6',
  process: '#86efac', world: '#f0eeff', ghost: '#55506d', fossil: '#a16207',
  other: '#94a3b8',
}
const NODE_ICONS: Record<string, string> = {
  idea:'◆', memory:'◉', project:'▣', person:'◎', artifact:'◧',
  media:'◐', process:'◑', world:'◯', ghost:'◌', fossil:'◫', other:'◇',
}

const ALL_TYPES = ['idea','memory','project','person','artifact','media','process','world','other']

function eColor(e: number) {
  return e > 0.65 ? 'var(--red)' : e > 0.35 ? 'var(--amber)' : 'var(--green)'
}

interface Props { node: SNode; nodes?: SNode[]; onClose: () => void; onRefresh: () => void }

const RISK_COLORS: Record<string, string> = {
  critical: 'var(--red)',
  high:     'var(--amber)',
  medium:   'var(--sky)',
  low:      'var(--green)',
}

export default function NodeDetail({ node, nodes = [], onClose, onRefresh }: Props) {
  const [busy, setBusy] = useState(false)
  const [editing, setEditing] = useState(false)
  const [expanded, setExpanded] = useState(false)
  const [editContent, setEditContent] = useState(node.content)
  const [editNickname, setEditNickname] = useState(node.nickname)
  const [editType, setEditType] = useState(node.node_type)
  const [editCustomType, setEditCustomType] = useState(node.custom_type || '')
  const [editColor, setEditColor] = useState(node.aura_color || '#94a3b8')
  const [scheduleMode, setScheduleMode] = useState<string>(node.schedule?.mode || 'none')
  const [scheduleStatus, setScheduleStatus] = useState<string>(node.schedule?.status || 'active')
  const [scheduleStart, setScheduleStart] = useState(node.schedule?.start_at || '')
  const [scheduleEnd, setScheduleEnd] = useState(node.schedule?.end_at || '')
  const [scheduleTime, setScheduleTime] = useState(node.schedule?.time_of_day || '')
  const [scheduleInterval, setScheduleInterval] = useState(String(node.schedule?.interval_minutes || 60))
  const [scheduleReminder, setScheduleReminder] = useState(Boolean(node.schedule?.reminder_enabled))
  const [scheduleReminderMinutes, setScheduleReminderMinutes] = useState(String(node.schedule?.reminder_minutes_before || 10))
  const [scheduleDays, setScheduleDays] = useState<number[]>(node.schedule?.days_of_week || [])
  const [linkQuery, setLinkQuery] = useState('')
  const [ghostRisk, setGhostRisk] = useState<{
    risk_level: string; days_to_ghost: number; risk_score: number
  } | null>(null)
  const [nextFocus, setNextFocus] = useState<{ content: string; probability: number }[]>([])

  function resetEditState() {
    setEditContent(node.content)
    setEditNickname(node.nickname)
    setEditType(node.node_type)
    setEditCustomType(node.custom_type || '')
    setEditColor(node.custom_color || node.aura_color || '#94a3b8')
    setScheduleMode(node.schedule?.mode || 'none')
    setScheduleStatus(node.schedule?.status || 'active')
    setScheduleStart(node.schedule?.start_at || '')
    setScheduleEnd(node.schedule?.end_at || '')
    setScheduleTime(node.schedule?.time_of_day || '')
    setScheduleInterval(String(node.schedule?.interval_minutes || 60))
    setScheduleReminder(Boolean(node.schedule?.reminder_enabled))
    setScheduleReminderMinutes(String(node.schedule?.reminder_minutes_before || 10))
    setScheduleDays(node.schedule?.days_of_week || [])
    setLinkQuery('')
  }

  useEffect(() => {
    resetEditState()
    // ML ghost risk for this node
    api.mlGhostRisk().then(risks => {
      const mine = risks.find((r: any) => r.node_id === node.id)
      if (mine) setGhostRisk(mine)
    }).catch(() => {})

    // ML next focus from this node
    api.mlNextFocus(node.id).then(nf => setNextFocus(nf.slice(0, 3))).catch(() => {})
  }, [node.id])
  const col  = node.node_type === 'other' && node.aura_color?.startsWith('#')
    ? node.aura_color
    : (NODE_COLORS[node.node_type] ?? 'var(--lavender-text)')
  const icon = NODE_ICONS[node.node_type]  ?? '◆'
  const displayType = node.node_type === 'other'
    ? (node.custom_type || 'Other')
    : node.node_type

  async function saveEdit() {
    setBusy(true)
    try {
      const schedule = scheduleMode === 'none'
        ? { mode: 'none' }
        : {
          mode: scheduleMode,
          status: scheduleStatus,
          start_at: scheduleStart || undefined,
          end_at: scheduleEnd || undefined,
          time_of_day: scheduleTime || undefined,
          interval_minutes: Number(scheduleInterval) || undefined,
          days_of_week: scheduleDays,
          reminder_enabled: scheduleReminder,
          reminder_minutes_before: Number(scheduleReminderMinutes) || 10,
        }
      await api.updateNode(node.id, {
        content: editContent,
        node_type: editType,
        nickname: editNickname,
        aura_color: editType === 'other' ? editColor : undefined,
        custom_type: editType === 'other' ? (editCustomType.trim() || 'Other') : undefined,
        custom_color: editType === 'other' ? editColor : undefined,
        schedule,
      })
      toast('Node updated')
      setEditing(false)
      setExpanded(false)
      onRefresh()
    } catch (e) { toast(String(e), 'error') }
    setBusy(false)
  }

  async function quickLink(targetId: string) {
    setBusy(true)
    try {
      await api.connect(node.id, targetId)
      toast('Nodes linked')
      setLinkQuery('')
      onRefresh()
    } catch (e) { toast(String(e), 'error') }
    setBusy(false)
  }

  async function act(msg: string, fn: () => Promise<unknown>) {
    setBusy(true)
    try { await fn(); toast(msg); onRefresh() }
    catch (e) { toast(String(e), 'error') }
    setBusy(false)
  }

  const flags = [
    node.is_ghost  && 'GHOST',
    node.is_fossil && 'FOSSIL',
    node.is_void   && 'VOID',
  ].filter(Boolean) as string[]
  const linkMatches = nodes
    .filter(n => n.id !== node.id)
    .filter(n => {
      const q = linkQuery.trim().toLowerCase()
      if (!q) return false
      return n.nickname.toLowerCase().includes(q) || n.content.toLowerCase().includes(q)
    })
    .slice(0, 5)
  const dayLabels = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat']

  function toggleDay(day: number) {
    setScheduleDays(prev => prev.includes(day) ? prev.filter(d => d !== day) : [...prev, day].sort())
  }

  function renderEditorSurface(isExpanded: boolean) {
    return (
      <div style={{
        display: 'grid',
        gridTemplateColumns: isExpanded ? 'minmax(0, 1fr) 260px' : '1fr',
        gap: isExpanded ? 14 : 8,
        minHeight: isExpanded ? '70vh' : undefined,
      }}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8, minHeight: 0 }}>
          <textarea
            value={editContent}
            onChange={e => setEditContent(e.target.value)}
            style={{ minHeight: isExpanded ? 360 : 80, fontSize: 13, lineHeight: 1.6, resize: 'vertical' }}
            placeholder="Content..."
          />
          {isExpanded && (
            <div className="panel" style={{ padding: 10, minHeight: 120, whiteSpace: 'pre-wrap', color: 'var(--t2)', fontSize: 13, lineHeight: 1.6 }}>
              {editContent || 'Preview will appear here.'}
            </div>
          )}
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: 8, minWidth: 0 }}>
          <input
            type="text"
            value={editNickname}
            onChange={e => setEditNickname(e.target.value)}
            placeholder="Nickname"
            style={{ fontSize: 12 }}
          />
          <select value={editType} onChange={e => setEditType(e.target.value)} style={{ fontSize: 12 }}>
            {ALL_TYPES.map(t => (
              <option key={t} value={t}>{t.charAt(0).toUpperCase() + t.slice(1)}</option>
            ))}
          </select>
          {editType === 'other' && (
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 44px', gap: 8, alignItems: 'center' }}>
              <input
                type="text"
                value={editCustomType}
                onChange={e => setEditCustomType(e.target.value)}
                placeholder="Custom class name"
                style={{ fontSize: 12 }}
              />
              <input
                type="color"
                value={editColor}
                onChange={e => setEditColor(e.target.value)}
                title="Custom color"
                style={{ height: 28, padding: 2, cursor: 'pointer' }}
              />
            </div>
          )}

          <div className="panel" style={{ padding: 10, display: 'flex', flexDirection: 'column', gap: 7 }}>
            <div style={{ fontSize: 10, color: 'var(--t4)', fontWeight: 700, letterSpacing: '0.06em', textTransform: 'uppercase' }}>Schedule</div>
            <select value={scheduleMode} onChange={e => setScheduleMode(e.target.value)} style={{ fontSize: 12 }}>
              <option value="none">No schedule</option>
              <option value="once">Once</option>
              <option value="daily">Daily</option>
              <option value="weekly">Weekly</option>
              <option value="interval">Interval</option>
              <option value="custom_days">Custom days</option>
            </select>
            {scheduleMode !== 'none' && (
              <>
                <select value={scheduleStatus} onChange={e => setScheduleStatus(e.target.value)} style={{ fontSize: 12 }}>
                  <option value="active">Active</option>
                  <option value="paused">Paused</option>
                  <option value="completed">Completed</option>
                </select>
                <input type="datetime-local" value={scheduleStart} onChange={e => setScheduleStart(e.target.value)} />
                <input type="datetime-local" value={scheduleEnd} onChange={e => setScheduleEnd(e.target.value)} />
                {(scheduleMode === 'daily' || scheduleMode === 'weekly' || scheduleMode === 'custom_days') && (
                  <input type="time" value={scheduleTime} onChange={e => setScheduleTime(e.target.value)} />
                )}
                {scheduleMode === 'interval' && (
                  <input type="number" min={1} value={scheduleInterval} onChange={e => setScheduleInterval(e.target.value)} placeholder="Interval minutes" />
                )}
                {(scheduleMode === 'weekly' || scheduleMode === 'custom_days') && (
                  <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap' }}>
                    {dayLabels.map((label, idx) => (
                      <button
                        key={label}
                        type="button"
                        className={`btn-xs ${scheduleDays.includes(idx) ? 'btn-primary' : ''}`}
                        onClick={() => toggleDay(idx)}
                      >
                        {label}
                      </button>
                    ))}
                  </div>
                )}
                <label style={{ display: 'flex', gap: 6, alignItems: 'center', color: 'var(--t3)', fontSize: 11 }}>
                  <input type="checkbox" checked={scheduleReminder} onChange={e => setScheduleReminder(e.target.checked)} />
                  Reminder
                </label>
                {scheduleReminder && (
                  <input type="number" min={0} value={scheduleReminderMinutes} onChange={e => setScheduleReminderMinutes(e.target.value)} placeholder="Minutes before" />
                )}
              </>
            )}
          </div>

          <div className="panel" style={{ padding: 10, display: 'flex', flexDirection: 'column', gap: 7 }}>
            <div style={{ fontSize: 10, color: 'var(--t4)', fontWeight: 700, letterSpacing: '0.06em', textTransform: 'uppercase' }}>Quick Link</div>
            <input value={linkQuery} onChange={e => setLinkQuery(e.target.value)} placeholder="Search nodes to link..." />
            {linkMatches.map(match => (
              <button key={match.id} className="btn-xs" disabled={busy} onClick={() => quickLink(match.id)} style={{ textAlign: 'left' }}>
                {match.nickname}
              </button>
            ))}
          </div>

          <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
            <button className="btn-sm btn-primary" disabled={busy} onClick={saveEdit}>Save</button>
            <button className="btn-sm" disabled={busy} onClick={() => { setEditing(false); setExpanded(false); resetEditState() }}>Cancel</button>
            {!isExpanded && <button className="btn-sm" disabled={busy} onClick={() => setExpanded(true)}>Open editor</button>}
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="col anim-right" style={{ height: '100%' }}>

      {/* Header */}
      <div style={{
        padding: '12px 14px',
        borderBottom: '1px solid var(--line)',
        display: 'flex', gap: 10, alignItems: 'flex-start',
      }}>
        <span style={{ fontSize: 20, color: col, lineHeight: 1, marginTop: 1, flexShrink: 0 }}>{icon}</span>
        <div className="fill">
          <div style={{
            fontSize: 10, fontWeight: 600, letterSpacing: '0.06em',
            textTransform: 'uppercase', color: col, marginBottom: 3, opacity: 0.85,
          }}>
            {displayType}
          </div>
          <div style={{ color: 'var(--lavender-text)', fontSize: 13, fontWeight: 700, marginBottom: 5, wordBreak: 'break-word' }}>
            {node.nickname}
          </div>
          <div style={{ color: 'var(--t1)', fontSize: 13, lineHeight: 1.5, wordBreak: 'break-word' }}>
            {node.content}
          </div>
          {flags.length > 0 && (
            <div style={{ display: 'flex', gap: 4, marginTop: 6, flexWrap: 'wrap' }}>
              {flags.map(f => (
                <span key={f} className={`badge ${f === 'GHOST' ? 'badge-mt' : f === 'FOSSIL' ? 'badge-am' : 'badge-lv'}`}>
                  {f}
                </span>
              ))}
            </div>
          )}
          <div style={{ display: 'flex', gap: 4, marginTop: 6, flexWrap: 'wrap' }}>
            {node.entropy_state && <span className="badge badge-mt">{node.entropy_state}</span>}
            {node.velocity_state && <span className="badge badge-sky">{node.velocity_state}</span>}
          </div>
        </div>
        <div style={{ display: 'flex', gap: 4, flexShrink: 0 }}>
          <button className="btn-ghost btn-xs" onClick={() => {
            setEditing(e => !e)
            setEditContent(node.content)
            setEditNickname(node.nickname)
            setEditType(node.node_type)
            setEditCustomType(node.custom_type || '')
            setEditColor(node.custom_color || node.aura_color || '#94a3b8')
          }} title="Edit node">✎</button>
          <button className="btn-ghost btn-xs" onClick={onClose}>✕</button>
        </div>
      </div>

      {editing && (
        <div style={{ padding: '12px 14px', borderBottom: '1px solid var(--line)', display: 'flex', flexDirection: 'column', gap: 8 }}>
          {renderEditorSurface(false)}
        </div>
      )}

      <div className="scroll fill" style={{ padding: '12px 14px', display: 'flex', flexDirection: 'column', gap: 16 }}>

        {/* Gauges */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: 7 }}>
          {[
            { label: 'Entropy',  val: node.entropy,                   col: eColor(node.entropy) },
            { label: 'Gravity',  val: Math.min(node.gravity / 10, 1), col: 'var(--lavender-text)' },
            { label: 'Velocity', val: Math.min(node.velocity / 10, 1), col: 'var(--sky)' },
          ].map(({ label, val, col: c }) => (
            <div key={label} style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <span style={{ color: 'var(--t4)', fontSize: 10, width: 54, flexShrink: 0 }}>{label}</span>
              <div className="bar fill">
                <div className="bar-fill" style={{ width: `${(val*100).toFixed(0)}%`, background: c }} />
              </div>
              <span style={{ color: c, fontSize: 10, width: 28, textAlign: 'right', fontFamily: 'var(--font-mono)', flexShrink: 0 }}>
                {(val * 100).toFixed(0)}%
              </span>
            </div>
          ))}
        </div>

        {/* Stats */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
          {[
            ['Access count', node.access_count],
            ['Gravity',      node.gravity.toFixed(3)],
            ['Created',      node.created_at.slice(0, 10)],
            ['Last access',  node.accessed_at.slice(0, 10)],
          ].map(([k, v]) => (
            <div key={String(k)} className="m-row" style={{ padding: '4px 0' }}>
              <span className="m-label">{k}</span>
              <span className="m-val" style={{ fontFamily: 'var(--font-mono)', fontSize: 11 }}>{v}</span>
            </div>
          ))}
          <div className="m-row" style={{ padding: '4px 0' }}>
            <span className="m-label">ID</span>
            <span style={{ color: 'var(--t4)', fontSize: 10, fontFamily: 'var(--font-mono)' }}>
              {node.id.slice(0, 14)}…
            </span>
          </div>
        </div>

        {/* Focus */}
        <div>
          <div style={{ fontSize: 10, color: 'var(--t4)', fontWeight: 600, letterSpacing: '0.06em', marginBottom: 7, textTransform: 'uppercase' }}>
            Record Focus
          </div>
          <div style={{ display: 'flex', gap: 5, flexWrap: 'wrap' }}>
            <button className="btn-sm" disabled={busy} onClick={() => act('Glance recorded', () => api.recordFocus(node.id, 10, 'Glance'))}>
              Glance
            </button>
            <button className="btn-sm" disabled={busy} onClick={() => act('Read recorded', () => api.recordFocus(node.id, 30, 'Read'))}>
              Read
            </button>
            <button className="btn-sm btn-primary" disabled={busy} onClick={() => act('DeepWork recorded', () => api.recordFocus(node.id, 120, 'DeepWork'))}>
              DeepWork
            </button>
          </div>
        </div>

        {/* ML Ghost Risk */}
        {ghostRisk && (
          <div>
            <div style={{ fontSize: 10, color: 'var(--t4)', fontWeight: 600, letterSpacing: '0.06em', marginBottom: 7, textTransform: 'uppercase' }}>
              ML Ghost Risk
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
              <span style={{
                fontSize: 11, fontWeight: 700,
                color: RISK_COLORS[ghostRisk.risk_level] ?? 'var(--t3)',
                border: `1px solid ${RISK_COLORS[ghostRisk.risk_level] ?? 'var(--t3)'}44`,
                borderRadius: 4, padding: '2px 8px',
              }}>
                {ghostRisk.risk_level.toUpperCase()}
              </span>
              <span style={{ fontSize: 11, color: 'var(--t3)' }}>
                {ghostRisk.days_to_ghost < 1
                  ? 'about to become a ghost'
                  : `≈${ghostRisk.days_to_ghost} days left`}
              </span>
            </div>
            <div className="bar fill" style={{ marginBottom: 4 }}>
              <div className="bar-fill" style={{
                width: `${(ghostRisk.risk_score * 100).toFixed(0)}%`,
                background: RISK_COLORS[ghostRisk.risk_level] ?? 'var(--t3)',
              }} />
            </div>
          </div>
        )}

        {/* ML Next Focus */}
        {nextFocus.length > 0 && (
          <div>
            <div style={{ fontSize: 10, color: 'var(--t4)', fontWeight: 600, letterSpacing: '0.06em', marginBottom: 7, textTransform: 'uppercase' }}>
              ML: Next Focus
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
              {nextFocus.map((nf, i) => (
                <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                  <span style={{ color: 'var(--sky)', fontSize: 10, width: 32 }}>
                    {Math.round(nf.probability * 100)}%
                  </span>
                  <span style={{ color: 'var(--t2)', fontSize: 11, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                    {nf.content}
                  </span>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Schedule */}
        {node.schedule && (
          <div>
            <div style={{ fontSize: 10, color: 'var(--t4)', fontWeight: 600, letterSpacing: '0.06em', marginBottom: 7, textTransform: 'uppercase' }}>
              Schedule
            </div>
            <div className="panel" style={{ padding: 9, display: 'flex', flexDirection: 'column', gap: 4 }}>
              <div style={{ color: 'var(--t2)', fontSize: 12 }}>
                {node.schedule.mode.replace('_', ' ')} · {node.schedule.status}
              </div>
              {node.schedule.time_of_day && <div style={{ color: 'var(--t4)', fontSize: 11 }}>Time {node.schedule.time_of_day}</div>}
              {node.schedule.interval_minutes && <div style={{ color: 'var(--t4)', fontSize: 11 }}>Every {node.schedule.interval_minutes} minutes</div>}
              {node.schedule.days_of_week?.length > 0 && (
                <div style={{ color: 'var(--t4)', fontSize: 11 }}>
                  {node.schedule.days_of_week.map(day => dayLabels[day]).join(', ')}
                </div>
              )}
              {node.schedule.reminder_enabled && (
                <div style={{ color: 'var(--amber)', fontSize: 11 }}>
                  Reminder {node.schedule.reminder_minutes_before} min before
                </div>
              )}
            </div>
          </div>
        )}

        {/* Actions */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: 5 }}>
          <button
            className="btn-sm btn-danger"
            disabled={busy}
            onClick={async () => {
              if (!confirm(`Delete "${node.content.slice(0,40)}"?`)) return
              setBusy(true)
              try { await api.deleteNode(node.id); toast('Deleted'); onRefresh(); onClose() }
              catch (e) { toast(String(e), 'error') }
              setBusy(false)
            }}
          >
            Delete node
          </button>
        </div>

        {/* Aura */}
        {node.aura_color && node.aura_color.startsWith('#') && (
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <div style={{
              width: 12, height: 12, borderRadius: '50%',
              background: node.aura_color, flexShrink: 0,
            }} />
            <span style={{ color: 'var(--t4)', fontSize: 10, fontFamily: 'var(--font-mono)' }}>
              Aura {node.aura_color}
            </span>
          </div>
        )}
      </div>
      {expanded && (
        <div className="modal-bg" onClick={e => { if (e.target === e.currentTarget) setExpanded(false) }}>
          <div className="modal" style={{ width: 'min(1080px, 94vw)', maxHeight: '92vh', overflow: 'auto' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 12 }}>
              <div style={{ color: col, fontSize: 12, fontWeight: 700, textTransform: 'uppercase' }}>{displayType}</div>
              <div style={{ color: 'var(--t1)', fontSize: 16, fontWeight: 700 }}>{editNickname || node.nickname}</div>
              <button className="btn-xs" style={{ marginLeft: 'auto' }} onClick={() => setExpanded(false)}>Close</button>
            </div>
            {renderEditorSurface(true)}
          </div>
        </div>
      )}
    </div>
  )
}
