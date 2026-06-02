import { useEffect, useState } from 'react'
import { api } from '../api'
import type { NotificationEventSettings, NotificationSettings } from '../types'

type SaveState = 'idle' | 'saving' | 'testing'

export default function SettingsView() {
  const [settings, setSettings] = useState<NotificationSettings | null>(null)
  const [telegramEnabled, setTelegramEnabled] = useState(false)
  const [telegramToken, setTelegramToken] = useState('')
  const [telegramChatId, setTelegramChatId] = useState('')
  const [defaultChannel, setDefaultChannel] = useState<'app' | 'telegram' | 'both'>('app')
  const [events, setEvents] = useState<NotificationEventSettings>(defaultEvents)
  const [state, setState] = useState<SaveState>('idle')
  const [notice, setNotice] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  async function load() {
    setError(null)
    try {
      const next = await api.notificationSettings()
      setSettings(next)
      setTelegramEnabled(next.telegram_enabled)
      setTelegramChatId(next.telegram_chat_id ?? '')
      setDefaultChannel(channelValue(next.default_channel))
      setEvents({ ...defaultEvents, ...next.events })
    } catch (e: any) {
      setError(e.message || 'Settings unavailable')
    }
  }

  useEffect(() => {
    load()
  }, [])

  async function save() {
    setState('saving')
    setError(null)
    setNotice(null)
    try {
      const next = await api.saveNotificationSettings({
        telegram_enabled: telegramEnabled,
        telegram_bot_token: telegramToken.trim() || undefined,
        telegram_chat_id: telegramChatId.trim(),
        default_channel: defaultChannel,
        events,
      })
      setSettings(next)
      setTelegramToken('')
      setEvents({ ...defaultEvents, ...next.events })
      setNotice('Settings saved')
    } catch (e: any) {
      setError(e.message || 'Save failed')
    } finally {
      setState('idle')
    }
  }

  async function testTelegram() {
    setState('testing')
    setError(null)
    setNotice(null)
    try {
      await api.testTelegramNotification('SilentNode test notification')
      setNotice('Telegram notification sent')
    } catch (e: any) {
      setError(e.message || 'Telegram test failed')
    } finally {
      setState('idle')
    }
  }

  if (!settings) {
    return (
      <div className="panel-loading">
        <span />
        <strong>Loading settings</strong>
      </div>
    )
  }

  const busy = state !== 'idle'
  const canTest = telegramEnabled && (settings.telegram_token_set || telegramToken.trim()) && telegramChatId.trim()

  return (
    <div className="mode-grid">
      <section className="panel mode-panel">
        <div className="sec-head">
          <span>Settings</span>
          <em>local</em>
        </div>

        {(error || notice) && (
          <div className={error ? 'vault-error' : 'vault-badge'}>
            {error ?? notice}
          </div>
        )}

        <div className="mode-list">
          <label className="check-row">
            <input
              type="checkbox"
              checked={telegramEnabled}
              onChange={e => setTelegramEnabled(e.target.checked)}
            />
            <span>Telegram notifications</span>
          </label>

          <label className="settings-field">
            <span>Bot token</span>
            <input
              className="vault-input"
              type="password"
              autoComplete="off"
              placeholder={settings.telegram_token_preview ?? 'Paste bot token'}
              value={telegramToken}
              onChange={e => setTelegramToken(e.target.value)}
            />
          </label>

          <label className="settings-field">
            <span>Chat ID</span>
            <input
              className="vault-input"
              placeholder="123456789"
              value={telegramChatId}
              onChange={e => setTelegramChatId(e.target.value)}
            />
          </label>

          <label className="settings-field">
            <span>Default channel</span>
            <select
              className="vault-input"
              value={defaultChannel}
              onChange={e => setDefaultChannel(channelValue(e.target.value))}
            >
              <option value="app">App</option>
              <option value="telegram">Telegram</option>
              <option value="both">Both</option>
            </select>
          </label>

          <div className="settings-field">
            <span>Telegram event triggers</span>
            <div style={{ display: 'grid', gap: 10 }}>
              {eventGroups.map(group => (
                <div key={group.title} className="glass" style={{ padding: 10 }}>
                  <div style={{ color: 'var(--text-secondary)', fontSize: 11, fontWeight: 700, marginBottom: 8 }}>
                    {group.title}
                  </div>
                  <div style={{ display: 'grid', gap: 6 }}>
                    {group.items.map(item => (
                      <label key={item.key} className="check-row" style={{ margin: 0 }}>
                        <input
                          type="checkbox"
                          checked={Boolean(events[item.key])}
                          onChange={e => setEvents(prev => ({ ...prev, [item.key]: e.target.checked }))}
                        />
                        <span>
                          {item.label}
                          <small style={{ display: 'block', color: 'var(--text-muted)', fontSize: 10, marginTop: 2 }}>
                            {item.help}
                          </small>
                        </span>
                      </label>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>

        <div className="vault-form-btns">
          <button className="btn-vault-confirm" onClick={save} disabled={busy}>
            {state === 'saving' ? 'Saving...' : 'Save'}
          </button>
          <button className="btn-vault-open" onClick={testTelegram} disabled={busy || !canTest}>
            {state === 'testing' ? 'Sending...' : 'Send test'}
          </button>
        </div>
      </section>

      <section className="panel mode-panel">
        <div className="sec-head">
          <span>Notification Status</span>
          <em>{telegramEnabled ? 'enabled' : 'disabled'}</em>
        </div>
        <div className="atmosphere-list">
          <div className="atmosphere-row">
            <i style={{ background: settings.telegram_token_set ? 'var(--green)' : 'var(--text-muted)' }} />
            <div>
              <strong>Telegram token</strong>
              <span>{settings.telegram_token_preview ?? 'Not configured'}</span>
            </div>
          </div>
          <div className="atmosphere-row">
            <i style={{ background: telegramChatId.trim() ? 'var(--sky)' : 'var(--text-muted)' }} />
            <div>
              <strong>Telegram chat</strong>
              <span>{telegramChatId.trim() || 'Not configured'}</span>
            </div>
          </div>
          <div className="atmosphere-row">
            <i style={{ background: 'var(--amber)' }} />
            <div>
              <strong>Default channel</strong>
              <span>{defaultChannel}</span>
            </div>
          </div>
          <div className="atmosphere-row">
            <i style={{ background: activeEventCount(events) ? 'var(--green)' : 'var(--text-muted)' }} />
            <div>
              <strong>Telegram triggers</strong>
              <span>{activeEventCount(events)} enabled</span>
            </div>
          </div>
        </div>
      </section>
    </div>
  )
}

function channelValue(value: string): 'app' | 'telegram' | 'both' {
  if (value === 'telegram' || value === 'both') return value
  return 'app'
}

const defaultEvents: NotificationEventSettings = {
  node_created: false,
  node_updated: false,
  node_deleted: false,
  focus_started: false,
  focus_stopped: false,
  focus_logged: false,
  mode_changed: false,
  task_created: false,
  task_completed: false,
  calendar_changed: false,
  dream_action: false,
  schedule_reminder: true,
}

const eventGroups: {
  title: string
  items: { key: keyof NotificationEventSettings; label: string; help: string }[]
}[] = [
  {
    title: 'Nodes',
    items: [
      { key: 'node_created', label: 'Node created', help: 'New thought or node is added.' },
      { key: 'node_updated', label: 'Node updated', help: 'Content, type, color, schedule, or nickname changes.' },
      { key: 'node_deleted', label: 'Node deleted', help: 'A node is removed from the vault.' },
    ],
  },
  {
    title: 'Focus',
    items: [
      { key: 'focus_started', label: 'Focus started', help: 'Active focus mode begins.' },
      { key: 'focus_stopped', label: 'Focus stopped', help: 'Active focus ends or times out.' },
      { key: 'focus_logged', label: 'Focus logged', help: 'A manual focus session is recorded.' },
      { key: 'mode_changed', label: 'System mode changed', help: 'Builder, researcher, ghost, memory, or auto mode changes.' },
    ],
  },
  {
    title: 'Planning',
    items: [
      { key: 'task_created', label: 'Task created', help: 'A daily task is added.' },
      { key: 'task_completed', label: 'Task completed', help: 'A task is completed or reopened.' },
      { key: 'calendar_changed', label: 'Calendar changed', help: 'Calendar event is created or deleted.' },
      { key: 'schedule_reminder', label: 'Schedule reminders', help: 'Node schedule reminders are sent by the background scheduler.' },
    ],
  },
  {
    title: 'Intelligence',
    items: [
      { key: 'dream_action', label: 'Dream action applied', help: 'A Dream proposal is applied.' },
    ],
  },
]

function activeEventCount(events: NotificationEventSettings) {
  return Object.values(events).filter(Boolean).length
}
