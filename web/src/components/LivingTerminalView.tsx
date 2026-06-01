import { useEffect, useState } from 'react'
import type { TerminalContextData } from '../types'
import { api } from '../api'

export default function LivingTerminalView() {
  const [ctx, setCtx] = useState<TerminalContextData | null>(null)

  useEffect(() => {
    load()
    const id = setInterval(load, 8000)
    return () => clearInterval(id)
  }, [])

  function load() {
    api.terminalContext().then(setCtx).catch(() => {})
  }

  return (
    <div className="terminal-panel">
      <div className="terminal-head">
        <span>Living Terminal</span>
        <button className="btn-xs" onClick={load}>Refresh</button>
      </div>
      <div className="terminal-body">
        {(ctx?.lines ?? ['waiting for process context...']).map((line, index) => (
          <pre key={index}><b>{index === 0 ? '>' : '$'}</b> {line}</pre>
        ))}
      </div>
      <div className="terminal-context">
        <span>{ctx?.active_processes ?? 0} processes</span>
        <span>{ctx?.linked_processes ?? 0} linked</span>
        <span>{ctx?.suggested_node_preview || 'no context node'}</span>
      </div>
    </div>
  )
}
