import { useState, useEffect } from 'react'
import type { CalendarEvent, DailyTask, FocusWindow, SNode } from '../types'
import { api } from '../api'
import { toast } from './Toast'

const CAT_COLORS: Record<string, string> = {
  deadline:  'var(--red)',
  milestone: 'var(--amber)',
  meeting:   'var(--sky)',
  review:    'var(--lavender-text)',
  personal:  'var(--green)',
  task:      'var(--teal)',
  recurring: 'var(--t3)',
}
const CAT_ICONS: Record<string, string> = {
  deadline:  '⚠',
  milestone: '◆',
  meeting:   '◎',
  review:    '◉',
  personal:  '◌',
  task:      '✓',
  recurring: '⟳',
}

function todayKey(): string {
  const d = new Date()
  const offset = d.getTimezoneOffset()
  return new Date(d.getTime() - offset * 60_000).toISOString().slice(0, 10)
}

function hoursLabel(h: number): string {
  if (h < 0) return 'past'
  if (h < 1) return `${(h * 60).toFixed(0)}m`
  if (h < 24) return `${h.toFixed(0)}h`
  return `${(h / 24).toFixed(0)}d`
}

interface Props { nodes: SNode[] }

export default function CalendarView({ nodes }: Props) {
  const [events,  setEvents]  = useState<CalendarEvent[]>([])
  const [tasks,   setTasks]   = useState<DailyTask[]>([])
  const [windows, setWindows] = useState<FocusWindow[]>([])
  const [selected, setSelected] = useState<CalendarEvent | null>(null)
  const [loading,  setLoading]  = useState(true)
  const [adding,   setAdding]   = useState(false)
  const [taskDate, setTaskDate] = useState(todayKey)
  const [taskTitle, setTaskTitle] = useState('')
  const [taskTags, setTaskTags] = useState('')
  const [taskNotes, setTaskNotes] = useState('')
  const [taskDue, setTaskDue] = useState('')
  const [taskBusy, setTaskBusy] = useState(false)

  // Form state
  const [title,    setTitle]    = useState('')
  const [desc,     setDesc]     = useState('')
  const [cat,      setCat]      = useState('meeting')
  const [startAt,  setStartAt]  = useState(() => {
    const d = new Date(); d.setMinutes(0, 0, 0); d.setHours(d.getHours() + 1)
    return d.toISOString().slice(0, 16)
  })
  const [endAt,    setEndAt]    = useState(() => {
    const d = new Date(); d.setMinutes(0, 0, 0); d.setHours(d.getHours() + 2)
    return d.toISOString().slice(0, 16)
  })
  const [linkedNode, setLinkedNode] = useState('')
  const [submitting, setSubmitting] = useState(false)

  useEffect(() => {
    load()
  }, [])

  async function load() {
    setLoading(true)
    const [evRes, winRes, taskRes] = await Promise.allSettled([
      api.calendarEvents(),
      api.calendarFocusWindows(),
      api.tasks(taskDate),
    ])
    if (evRes.status === 'fulfilled') setEvents(evRes.value)
    if (winRes.status === 'fulfilled') setWindows(winRes.value)
    if (taskRes.status === 'fulfilled') setTasks(taskRes.value)
    setLoading(false)
  }

  useEffect(() => {
    api.tasks(taskDate)
      .then(setTasks)
      .catch(e => toast(String(e), 'error'))
  }, [taskDate])

  async function addEvent() {
    if (!title.trim()) { toast('Title required', 'error'); return }
    setSubmitting(true)
    try {
      await api.addCalendarEvent({
        title: title.trim(),
        description: desc.trim() || undefined,
        category: cat,
        start_at: new Date(startAt).toISOString(),
        end_at: new Date(endAt).toISOString(),
        linked_node_id: linkedNode || undefined,
      })
      toast('Event added')
      setAdding(false); setTitle(''); setDesc('')
      load()
    } catch (e) { toast(String(e), 'error') }
    setSubmitting(false)
  }

  async function removeEvent(id: string) {
    try {
      await api.deleteCalendarEvent(id)
      setEvents(ev => ev.filter(e => e.id !== id))
      if (selected?.id === id) setSelected(null)
      toast('Event removed')
    } catch (e) { toast(String(e), 'error') }
  }

  async function addTask() {
    if (!taskTitle.trim()) { toast('Task title required', 'error'); return }
    setTaskBusy(true)
    try {
      const tags = taskTags.split(',').map(t => t.trim().replace(/^#/, '')).filter(Boolean)
      const due_at = taskDue ? new Date(taskDue).toISOString() : undefined
      await api.addTask({
        title: taskTitle.trim(),
        date: taskDate,
        tags,
        notes: taskNotes.trim() || undefined,
        due_at,
      })
      setTaskTitle(''); setTaskTags(''); setTaskNotes(''); setTaskDue('')
      toast('Task added')
      load()
    } catch (e) { toast(String(e), 'error') }
    setTaskBusy(false)
  }

  async function toggleTask(task: DailyTask) {
    try {
      await api.completeTask(task.node_id, task.status !== 'done')
      setTasks(items => items.map(item => item.node_id === task.node_id
        ? { ...item, status: task.status === 'done' ? 'todo' : 'done' }
        : item))
      load()
    } catch (e) { toast(String(e), 'error') }
  }

  const approaching = events.filter(e => e.is_approaching)
  const upcoming    = events.filter(e => e.hours_until >= 0 && !e.is_approaching).slice(0, 20)
  const past        = events.filter(e => e.hours_until < 0)

  return (
    <div className="split">
      {/* Left: daily tasks + event list */}
      <div className="split-list panel">
        <div className="sec-head">
          <span style={{ color: 'var(--teal)' }}>✓</span>
          Daily Tasks
          <input
            type="date"
            value={taskDate}
            onChange={e => setTaskDate(e.target.value)}
            style={{ marginLeft: 'auto', width: 128, fontSize: 10 }}
          />
        </div>

        <div style={{ padding: '10px 12px', borderBottom: '1px solid var(--line)', display: 'flex', flexDirection: 'column', gap: 7 }}>
          <input type="text" placeholder="Write a task for this day…" value={taskTitle} onChange={e => setTaskTitle(e.target.value)} />
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 6 }}>
            <input type="text" placeholder="tags, comma separated" value={taskTags} onChange={e => setTaskTags(e.target.value)} />
            <input type="datetime-local" value={taskDue} onChange={e => setTaskDue(e.target.value)} style={{ fontSize: 10 }} />
          </div>
          <textarea placeholder="notes…" value={taskNotes} onChange={e => setTaskNotes(e.target.value)} rows={2} />
          <button className="btn-primary btn-sm" onClick={addTask} disabled={taskBusy}>
            {taskBusy ? 'Adding…' : 'Add Daily Task'}
          </button>
        </div>

        <div className="scroll" style={{ maxHeight: 260, borderBottom: '1px solid var(--line)' }}>
          {tasks.length === 0 && !loading && (
            <div style={{ padding: 14, color: 'var(--t4)', fontSize: 11, lineHeight: 1.5 }}>
              No tasks for this day. Native SilentNode tasks do not require Obsidian; imported Obsidian tasks appear here when their date matches.
            </div>
          )}
          {tasks.map(task => (
            <TaskRow key={task.node_id} task={task} onToggle={toggleTask} />
          ))}
        </div>

        <div className="sec-head">
          <span style={{ color: 'var(--sky)' }}>◎</span>
          Calendar Events
          <button
            className="btn-xs btn-primary"
            style={{ marginLeft: 'auto' }}
            onClick={() => setAdding(a => !a)}
          >
            {adding ? '✕' : '+ Event'}
          </button>
        </div>

        {/* Add form */}
        {adding && (
          <div style={{ padding: '10px 12px', borderBottom: '1px solid var(--line)', display: 'flex', flexDirection: 'column', gap: 6 }}>
            <input type="text" placeholder="Event title…" value={title} onChange={e => setTitle(e.target.value)} />
            <select value={cat} onChange={e => setCat(e.target.value)}>
              {['meeting','deadline','task','review','personal','recurring','milestone'].map(c => (
                <option key={c} value={c}>{c}</option>
              ))}
            </select>
            <div style={{ display: 'flex', gap: 6 }}>
              <div style={{ flex: 1 }}>
                <div style={{ fontSize: 9, color: 'var(--t4)', marginBottom: 2 }}>START</div>
                <input type="datetime-local" value={startAt} onChange={e => setStartAt(e.target.value)} style={{ fontSize: 10 }} />
              </div>
              <div style={{ flex: 1 }}>
                <div style={{ fontSize: 9, color: 'var(--t4)', marginBottom: 2 }}>END</div>
                <input type="datetime-local" value={endAt} onChange={e => setEndAt(e.target.value)} style={{ fontSize: 10 }} />
              </div>
            </div>
            <select value={linkedNode} onChange={e => setLinkedNode(e.target.value)}>
              <option value="">No linked node</option>
              {nodes.map(n => <option key={n.id} value={n.id}>{n.content.slice(0, 40)}</option>)}
            </select>
            <button className="btn-primary btn-sm" onClick={addEvent} disabled={submitting}>
              {submitting ? 'Adding…' : 'Add Event'}
            </button>
          </div>
        )}

        <div className="scroll fill">
          {loading && <div style={{ padding: 16, color: 'var(--t4)', fontSize: 11 }}>Loading calendar…</div>}

          {/* Approaching */}
          {approaching.length > 0 && (
            <>
              <div style={{ padding: '6px 12px', background: 'rgba(248,113,113,0.05)', borderBottom: '1px solid rgba(248,113,113,0.1)' }}>
                <span style={{ fontSize: 9, color: 'var(--red)', fontWeight: 600, letterSpacing: '0.08em' }}>
                  ⚠ APPROACHING ({approaching.length})
                </span>
              </div>
              {approaching.map(ev => <EventRow key={ev.id} ev={ev} selected={selected} onSelect={setSelected} onDelete={removeEvent} />)}
            </>
          )}

          {/* Upcoming */}
          {upcoming.length > 0 && (
            <>
              <div style={{ padding: '6px 12px', borderBottom: '1px solid var(--line)' }}>
                <span style={{ fontSize: 9, color: 'var(--t4)', fontWeight: 600, letterSpacing: '0.08em' }}>
                  UPCOMING
                </span>
              </div>
              {upcoming.map(ev => <EventRow key={ev.id} ev={ev} selected={selected} onSelect={setSelected} onDelete={removeEvent} />)}
            </>
          )}

          {/* Past */}
          {past.length > 0 && (
            <>
              <div style={{ padding: '6px 12px', borderBottom: '1px solid var(--line)' }}>
                <span style={{ fontSize: 9, color: 'var(--t4)', fontWeight: 600, letterSpacing: '0.08em' }}>
                  PAST ({past.length})
                </span>
              </div>
              {past.slice(0, 5).map(ev => <EventRow key={ev.id} ev={ev} selected={selected} onSelect={setSelected} onDelete={removeEvent} />)}
            </>
          )}

          {!loading && events.length === 0 && (
            <div style={{ padding: 20, color: 'var(--t4)', fontSize: 11, lineHeight: 1.6 }}>
              No events yet. Add your deadlines, meetings, and milestones to let SilentNode derive temporal intelligence around them.
            </div>
          )}
        </div>
      </div>

      {/* Right: detail + focus windows */}
      <div className="split-detail" style={{ gap: 10 }}>
        {/* Event detail */}
        {selected && (
          <div className="panel anim-in" style={{ flexShrink: 0 }}>
            <div className="sec-head">
              <span style={{ color: CAT_COLORS[selected.category] ?? 'var(--sky)' }}>
                {CAT_ICONS[selected.category] ?? '◎'}
              </span>
              {selected.title}
            </div>
            <div style={{ padding: '12px 14px', display: 'flex', flexDirection: 'column', gap: 8 }}>
              {selected.description && (
                <div style={{ color: 'var(--t2)', fontSize: 12, lineHeight: 1.6 }}>{selected.description}</div>
              )}
              <div style={{ display: 'flex', gap: 16, flexWrap: 'wrap' }}>
                <div>
                  <div style={{ fontSize: 9, color: 'var(--t4)', marginBottom: 2 }}>STARTS</div>
                  <div style={{ fontSize: 11, color: 'var(--t2)', fontFamily: 'var(--font-mono)' }}>
                    {new Date(selected.start_at).toLocaleString()}
                  </div>
                </div>
                <div>
                  <div style={{ fontSize: 9, color: 'var(--t4)', marginBottom: 2 }}>TIME UNTIL</div>
                  <div style={{
                    fontSize: 13, fontWeight: 700, fontFamily: 'var(--font-mono)',
                    color: selected.is_approaching ? 'var(--red)' : selected.hours_until < 0 ? 'var(--t4)' : 'var(--amber)',
                  }}>
                    {hoursLabel(selected.hours_until)}
                  </div>
                </div>
                <div>
                  <div style={{ fontSize: 9, color: 'var(--t4)', marginBottom: 2 }}>GRAVITY</div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                    <div className="bar" style={{ width: 60 }}>
                      <div className="bar-fill" style={{
                        width: `${Math.min(selected.computed_gravity * 33, 100)}%`,
                        background: CAT_COLORS[selected.category] ?? 'var(--sky)',
                      }} />
                    </div>
                    <span style={{ fontSize: 10, color: 'var(--t3)', fontFamily: 'var(--font-mono)' }}>
                      {selected.computed_gravity.toFixed(2)}
                    </span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Focus windows */}
        <div className="panel fill scroll">
          <div className="sec-head">
            <span style={{ color: 'var(--green)' }}>◐</span>
            Suggested Focus Windows
            <span style={{ marginLeft: 'auto', color: 'var(--t4)', fontSize: 10 }}>{windows.length}</span>
          </div>
          {windows.length === 0 && !loading && (
            <div style={{ padding: '12px 14px', color: 'var(--t4)', fontSize: 11 }}>
              No focus windows — add events to get temporal intelligence
            </div>
          )}
          {windows.map((w, i) => (
            <div key={i} style={{ padding: '9px 14px', borderBottom: '1px solid rgba(255,255,255,0.04)' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
                <span style={{ color: 'var(--green)', fontSize: 13, fontFamily: 'var(--font-mono)', fontWeight: 700 }}>
                  {String(w.start_hour).padStart(2, '0')}:00 – {String(w.end_hour).padStart(2, '0')}:00
                </span>
                <div className="bar" style={{ flex: 1 }}>
                  <div className="bar-fill" style={{ width: `${(w.score * 100).toFixed(0)}%`, background: 'var(--green)' }} />
                </div>
                <span style={{ fontSize: 10, color: 'var(--green)', fontFamily: 'var(--font-mono)' }}>
                  {(w.score * 100).toFixed(0)}%
                </span>
              </div>
              <div style={{ fontSize: 11, color: 'var(--t3)', lineHeight: 1.4 }}>{w.reason}</div>
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}

function EventRow({
  ev, selected, onSelect, onDelete,
}: {
  ev: CalendarEvent
  selected: CalendarEvent | null
  onSelect: (e: CalendarEvent) => void
  onDelete: (id: string) => void
}) {
  const col  = CAT_COLORS[ev.category] ?? 'var(--sky)'
  const icon = CAT_ICONS[ev.category] ?? '◎'
  const isSel = selected?.id === ev.id
  return (
    <div
      className="list-row"
      style={isSel ? { background: 'rgba(167,139,250,0.07)', borderLeft: '2px solid rgba(167,139,250,0.5)' } : {}}
      onClick={() => onSelect(ev)}
    >
      <span style={{ color: col, fontSize: 12, marginTop: 1 }}>{icon}</span>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ fontSize: 11, color: ev.is_approaching ? 'var(--red)' : 'var(--t1)', fontWeight: ev.is_approaching ? 600 : 400, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
          {ev.title}
        </div>
        <div style={{ fontSize: 9, color: 'var(--t4)', marginTop: 1 }}>
          {new Date(ev.start_at).toLocaleDateString()} · {hoursLabel(ev.hours_until)}
        </div>
      </div>
      <button
        className="btn-ghost btn-xs"
        style={{ flexShrink: 0, opacity: 0.4, padding: '2px 5px' }}
        onClick={e => { e.stopPropagation(); onDelete(ev.id) }}
      >
        ✕
      </button>
    </div>
  )
}

function TaskRow({
  task, onToggle,
}: {
  task: DailyTask
  onToggle: (task: DailyTask) => void
}) {
  const done = task.status === 'done'
  return (
    <div className="list-row" style={{ alignItems: 'flex-start' }}>
      <button
        className="btn-ghost btn-xs"
        style={{
          width: 22,
          height: 22,
          padding: 0,
          color: done ? 'var(--green)' : 'var(--t4)',
          borderColor: done ? 'rgba(74,222,128,0.35)' : 'var(--line)',
        }}
        onClick={() => onToggle(task)}
        title={done ? 'Mark todo' : 'Mark done'}
      >
        {done ? '✓' : '○'}
      </button>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{
          fontSize: 11,
          color: done ? 'var(--t4)' : 'var(--t1)',
          textDecoration: done ? 'line-through' : 'none',
          lineHeight: 1.4,
        }}>
          {task.title}
        </div>
        <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap', marginTop: 4 }}>
          <span style={{ fontSize: 9, color: task.source === 'obsidian' ? 'var(--lavender-text)' : 'var(--teal)', fontFamily: 'var(--font-mono)' }}>
            {task.source}
          </span>
          {task.tags.map(tag => (
            <span key={tag} style={{ fontSize: 9, color: 'var(--t4)', fontFamily: 'var(--font-mono)' }}>#{tag}</span>
          ))}
        </div>
      </div>
    </div>
  )
}
