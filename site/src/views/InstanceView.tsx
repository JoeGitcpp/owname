import { useI18n } from '../i18n'
import { Commune } from './Commune'

interface InstanceViewProps {
  instanceId:   string
  instanceName: string
  onLeave:      () => void
}

export function InstanceView({ instanceId: _instanceId, instanceName, onLeave }: InstanceViewProps) {
  const { t } = useI18n()

  // Real logout: clear the session cookie server-side, then leave. (Previously
  // this button only navigated home, leaving the user still logged in.)
  const handleLogout = async () => {
    try { await fetch('/auth/logout', { method: 'POST' }) } catch {}
    onLeave()
  }

  return (
    <div style={{ position: 'fixed', inset: 0 }}>
      {/* Gradient header strip — sits above the Commune canvas */}
      <header
        style={{
          position: 'absolute', top: 0, left: 0, right: 0, zIndex: 20,
          display: 'flex', alignItems: 'center', gap: 12,
          padding: '12px 18px',
          background: 'linear-gradient(to bottom, rgba(0,0,0,.55) 0%, transparent 100%)',
          pointerEvents: 'none',
        }}
      >
        <span
          style={{
            fontFamily: 'var(--font-mono)',
            fontSize: 13, letterSpacing: '.12em',
            color: 'var(--ink)',
          }}
        >
          {instanceName}
        </span>

        <span
          style={{
            fontSize: 11, letterSpacing: '.06em',
            color: 'var(--ink-dim)',
            padding: '3px 10px', borderRadius: 999,
            border: '1px solid rgba(255,255,255,.1)',
          }}
        >
          {t('instance.recognising')}
        </span>

        <button
          style={{
            marginLeft: 'auto',
            background: 'transparent',
            border: '1px solid rgba(255,255,255,.15)',
            color: 'var(--ink-dim)',
            fontSize: 11, letterSpacing: '.06em',
            padding: '4px 14px', borderRadius: 999,
            cursor: 'pointer', pointerEvents: 'all',
            transition: 'border-color .2s',
          }}
          onClick={handleLogout}
        >
          {t('instance.logout')}
        </button>
      </header>

      {/* Full commune experience unchanged — wrap onLeave to absorb secs param */}
      <Commune onLeave={() => onLeave()} />
    </div>
  )
}
