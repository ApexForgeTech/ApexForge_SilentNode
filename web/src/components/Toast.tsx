import { useEffect, useState } from 'react'

export interface ToastMsg {
  id: number
  text: string
  type: 'success' | 'error' | 'info'
}

let nextId = 1
const listeners: Array<(msg: ToastMsg) => void> = []

export function toast(text: string, type: ToastMsg['type'] = 'success') {
  const msg: ToastMsg = { id: nextId++, text, type }
  listeners.forEach(fn => fn(msg))
}

export function ToastContainer() {
  const [msgs, setMsgs] = useState<ToastMsg[]>([])

  useEffect(() => {
    const fn = (msg: ToastMsg) => {
      setMsgs(prev => [...prev, msg])
      setTimeout(() => setMsgs(prev => prev.filter(m => m.id !== msg.id)), 3200)
    }
    listeners.push(fn)
    return () => { const i = listeners.indexOf(fn); if (i > -1) listeners.splice(i, 1) }
  }, [])

  if (!msgs.length) return null
  return (
    <div className="toast-wrap">
      {msgs.map(m => (
        <div key={m.id} className={`toast toast-${m.type}`}>
          {m.type === 'success' && '✓ '}
          {m.type === 'error'   && '✕ '}
          {m.type === 'info'    && '◈ '}
          {m.text}
        </div>
      ))}
    </div>
  )
}
