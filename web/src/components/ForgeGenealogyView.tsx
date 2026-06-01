import { useEffect, useState } from 'react'
import type { ForgeArtifactData } from '../types'
import { api } from '../api'

export default function ForgeGenealogyView() {
  const [items, setItems] = useState<ForgeArtifactData[]>([])

  useEffect(() => {
    api.forgeGenealogy().then(setItems).catch(() => {})
  }, [])

  return (
    <div className="forge-lineage">
      {items.length === 0 && <div className="empty-state">No artifact genealogy yet.</div>}
      {items.map(a => (
        <div key={a.node_id} className="lineage-row">
          <div className="lineage-gen">{a.generation}</div>
          <div className="lineage-main">
            <strong>{a.label}</strong>
            <span>{a.artifact_type} · {a.parent_ids.length} parents · {a.child_ids.length} children</span>
            <div className="bar"><div className="bar-fill" style={{ width: `${Math.round(a.heat * 100)}%`, background: 'var(--amber)' }} /></div>
          </div>
        </div>
      ))}
    </div>
  )
}
