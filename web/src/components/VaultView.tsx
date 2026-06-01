import { useState, useEffect } from 'react'
import { api } from '../api'
import type { ObsidianImportResult, ObsidianPreviewData } from '../types'

interface VaultEntry {
  name: string
  path: string
}

interface VaultState {
  vaults: VaultEntry[]
  current: string
}

export default function VaultView({ onSwitch }: { onSwitch?: (name: string) => void }) {
  const [state, setState] = useState<VaultState | null>(null)
  const [creating, setCreating] = useState(false)
  const [newName, setNewName] = useState('')
  const [newPath, setNewPath] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [obsidianPath, setObsidianPath] = useState('')
  const [includeCompleted, setIncludeCompleted] = useState(true)
  const [preview, setPreview] = useState<ObsidianPreviewData | null>(null)
  const [importResult, setImportResult] = useState<ObsidianImportResult | null>(null)

  async function load() {
    try {
      const data = await api.vaults()
      setState(data)
    } catch {
      setError('API offline')
    }
  }

  useEffect(() => { load() }, [])

  async function handleSwitch(name: string) {
    setLoading(true)
    setError(null)
    try {
      await api.switchVault(name)
      await load()
      onSwitch?.(name)
    } catch (e: any) {
      setError(e.message)
    } finally {
      setLoading(false)
    }
  }

  async function handleCreate() {
    const name = newName.trim()
    if (!name) return
    setLoading(true)
    setError(null)
    try {
      await api.createVault(name, newPath.trim() || undefined)
      setNewName('')
      setNewPath('')
      setCreating(false)
      await load()
    } catch (e: any) {
      setError(e.message)
    } finally {
      setLoading(false)
    }
  }

  async function handleDelete(name: string) {
    if (!confirm(`Delete vault "${name}"?`)) return
    setLoading(true)
    setError(null)
    try {
      await api.deleteVault(name)
      await load()
    } catch (e: any) {
      setError(e.message)
    } finally {
      setLoading(false)
    }
  }

  async function handlePreviewImport() {
    const path = obsidianPath.trim()
    if (!path) return
    setLoading(true)
    setError(null)
    setImportResult(null)
    try {
      setPreview(await api.obsidianPreview(path))
    } catch (e: any) {
      setError(e.message)
    } finally {
      setLoading(false)
    }
  }

  async function handleRunImport() {
    const path = obsidianPath.trim()
    if (!path || !preview || !state) return
    setLoading(true)
    setError(null)
    try {
      const result = await api.obsidianImport(path, includeCompleted)
      setImportResult(result)
      setPreview(await api.obsidianPreview(path))
      onSwitch?.(state.current)
    } catch (e: any) {
      setError(e.message)
    } finally {
      setLoading(false)
    }
  }

  if (!state) return <div className="panel-loading"><span /><strong>Loading vaults</strong></div>

  return (
    <div className="vault-view">
      <div className="vault-header">
        <h2>Vaults</h2>
        <p className="vault-sub">Like Obsidian — each vault is a separate workspace. No passwords.</p>
      </div>

      {error && <div className="vault-error">{error}</div>}

      <div className="vault-list">
        {state.vaults.map(v => (
          <div key={v.name} className={`vault-item ${v.name === state.current ? 'vault-active' : ''}`}>
            <div className="vault-info">
              <span className="vault-name">{v.name}</span>
              {v.name === state.current && <span className="vault-badge">active</span>}
              <span className="vault-path">{v.path}</span>
            </div>
            <div className="vault-actions">
              {v.name !== state.current && (
                <button
                  className="btn-vault-open"
                  onClick={() => handleSwitch(v.name)}
                  disabled={loading}
                >
                  Open
                </button>
              )}
              {v.name !== state.current && state.vaults.length > 1 && (
                <button
                  className="btn-vault-del"
                  onClick={() => handleDelete(v.name)}
                  disabled={loading}
                >
                  Delete
                </button>
              )}
            </div>
          </div>
        ))}
      </div>

      {creating ? (
        <div className="vault-create-form">
          <input
            className="vault-input"
            placeholder="Vault name (e.g. Work, Personal)"
            value={newName}
            onChange={e => setNewName(e.target.value)}
            onKeyDown={e => e.key === 'Enter' && handleCreate()}
            autoFocus
          />
          <input
            className="vault-input"
            placeholder="Path (leave empty = auto: data/<name>.sqlite)"
            value={newPath}
            onChange={e => setNewPath(e.target.value)}
          />
          <div className="vault-form-btns">
            <button className="btn-vault-confirm" onClick={handleCreate} disabled={loading || !newName.trim()}>
              Create
            </button>
            <button className="btn-vault-cancel" onClick={() => { setCreating(false); setNewName(''); setNewPath('') }}>
              Cancel
            </button>
          </div>
        </div>
      ) : (
        <button className="btn-vault-new" onClick={() => setCreating(true)}>
          + New Vault
        </button>
      )}

      <section className="obsidian-import">
        <div className="vault-section-head">
          <strong>Obsidian Import</strong>
          <span>Read-only preview, then confirm.</span>
        </div>
        <div className="obsidian-form">
          <input
            className="vault-input"
            placeholder="/path/to/ObsidianVault"
            value={obsidianPath}
            onChange={e => setObsidianPath(e.target.value)}
          />
          <label className="check-row">
            <input type="checkbox" checked={includeCompleted} onChange={e => setIncludeCompleted(e.target.checked)} />
            <span>Include completed tasks</span>
          </label>
          <div className="vault-form-btns">
            <button className="btn-vault-confirm" onClick={handlePreviewImport} disabled={loading || !obsidianPath.trim()}>
              Preview
            </button>
            <button className="btn-vault-open" onClick={handleRunImport} disabled={loading || !preview}>
              Import
            </button>
          </div>
        </div>

        {preview && (
          <div className="obsidian-preview">
            <div className="obsidian-stats">
              <span>{preview.files_scanned} files</span>
              <span>{preview.tasks.length} tasks</span>
              <span>{preview.tags.length} tags</span>
              <span>{preview.tasks.filter(t => t.duplicate).length} duplicates</span>
            </div>
            {preview.warnings.length > 0 && (
              <div className="vault-warning">
                {preview.warnings.slice(0, 3).map(w => <span key={w}>{w}</span>)}
              </div>
            )}
            <div className="obsidian-task-list">
              {preview.tasks.slice(0, 18).map(task => (
                <div key={`${task.source_file}:${task.line}`} className={task.duplicate ? 'obsidian-task duplicate' : 'obsidian-task'}>
                  <strong>{task.completed ? '[x]' : '[ ]'} {task.text}</strong>
                  <span>{task.source_file}:{task.line}</span>
                  <em>{[task.date, ...task.tags.map(t => `#${t}`), task.duplicate ? 'duplicate' : 'new'].filter(Boolean).join(' · ')}</em>
                </div>
              ))}
            </div>
          </div>
        )}

        {importResult && (
          <div className="import-result">
            created {importResult.tasks_created} tasks, {importResult.tag_nodes_created} tags, {importResult.day_nodes_created} days, {importResult.edges_created} links; skipped {importResult.tasks_skipped}
          </div>
        )}
      </section>
    </div>
  )
}
