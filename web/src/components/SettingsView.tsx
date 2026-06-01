import { useEffect, useState } from 'react'
import { api } from '../api'
import type { NotificationSettings } from '../types'

type SaveState = 'idle' | 'saving' | 'testing'

export default function SettingsView() {
  const [settings, setSettings] = useState<NotificationSettings | null>(null)
  const [telegramEnabled, setTelegramEnabled] = useState(false)
  const [telegramToken, setTelegramToken] = useState('')
  const [telegramChatId, setTelegramChatId] = useState('')
  const [defaultChannel, setDefaultChannel] = useState<'app' | 'telegram' | 'both'>('app')
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
      })
      setSettings(next)
      setTelegramToken('')
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
        </div>
      </section>
    </div>
  )
}

function channelValue(value: string): 'app' | 'telegram' | 'both' {
  if (value === 'telegram' || value === 'both') return value
  return 'app'
}
