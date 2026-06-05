import { useState, useRef } from 'react'
import { useI18n }          from '../i18n'

type Step = 'contact' | 'code' | 'name'
type ContactType = 'phone' | 'email'

interface AuthFlowProps {
  onComplete: (instanceId: string, instanceName: string) => void
  onBack: () => void
}

function detectType(value: string): ContactType {
  return value.includes('@') ? 'email' : 'phone'
}

async function jsonPost(url: string, body: unknown): Promise<{ ok: boolean; error?: string; instance?: { id: string; name?: string; identityKey?: string } }> {
  try {
    const r = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    })
    return r.json()
  } catch {
    return { ok: false, error: 'network_error' }
  }
}

export function AuthFlow({ onComplete, onBack }: AuthFlowProps) {
  const { t } = useI18n()
  const [step, setStep]       = useState<Step>('contact')
  const [contact, setContact] = useState('')
  const [otp, setOtp]         = useState('')
  const [name, setName]       = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError]     = useState<string | null>(null)
  const [countdown, setCountdown] = useState(0)
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null)

  const contactType = detectType(contact)

  const startCountdown = () => {
    setCountdown(60)
    timerRef.current = setInterval(() => {
      setCountdown(c => {
        if (c <= 1) { clearInterval(timerRef.current!); return 0 }
        return c - 1
      })
    }, 1000)
  }

  const handleSendCode = async () => {
    setError(null)
    setLoading(true)
    const url  = contactType === 'phone' ? '/auth/request-phone' : '/auth/request'
    const body = contactType === 'phone' ? { phone: contact } : { email: contact }
    const res  = await jsonPost(url, body)
    setLoading(false)
    if (!res.ok) {
      setError(res.error === 'rate_limit' ? t('auth.error.rate-limit') : t('auth.error.invalid'))
      return
    }
    startCountdown()
    setStep('code')
  }

  const handleVerify = async () => {
    setError(null)
    setLoading(true)
    const url  = contactType === 'phone' ? '/auth/verify-phone' : '/auth/verify'
    const body = contactType === 'phone' ? { phone: contact, otp } : { email: contact, otp }
    const res  = await jsonPost(url, body)
    setLoading(false)
    if (!res.ok) { setError(t('auth.error.wrong-code')); return }

    // Check if instance already exists
    try {
      const me = await fetch('/instances/me').then(r => r.json())
      if (me?.instance) {
        onComplete(me.instance.id, me.instance.name ?? me.instance.identityKey)
        return
      }
    } catch {}
    setStep('name')
  }

  const handleCreate = async () => {
    if (!name.trim()) return
    setError(null)
    setLoading(true)
    const res = await jsonPost('/instances', {})
    setLoading(false)
    if (!res.ok && res.error !== 'instance_exists') {
      setError(res.error ?? 'error')
      return
    }
    onComplete(res.instance?.id ?? 'ok', name.trim())
  }

  return (
    <div className="auth-flow">
      {step === 'contact' && (
        <div className="auth-flow__step">
          <h2 className="auth-flow__title">{t('auth.title')}</h2>
          <p className="auth-flow__hint">{t('landing.login.hint')}</p>
          <input
            className="auth-flow__input"
            type="text"
            value={contact}
            onChange={e => setContact(e.target.value)}
            placeholder={t('auth.placeholder.phone')}
            onKeyDown={e => e.key === 'Enter' && contact.trim() && handleSendCode()}
            autoFocus
          />
          {error && <p className="auth-flow__error">{error}</p>}
          <div className="auth-flow__actions">
            <button className="auth-flow__btn auth-flow__btn--ghost" onClick={onBack}>
              {t('auth.back')}
            </button>
            <button
              className="auth-flow__btn auth-flow__btn--primary"
              onClick={handleSendCode}
              disabled={!contact.trim() || loading}
            >
              {loading ? '…' : t('auth.send-code')}
            </button>
          </div>
        </div>
      )}

      {step === 'code' && (
        <div className="auth-flow__step">
          <h2 className="auth-flow__title">{t('auth.enter-code')}</h2>
          <p className="auth-flow__hint">{t('auth.code-sent')} · {contact}</p>
          <input
            className="auth-flow__input auth-flow__input--code"
            type="text"
            inputMode="numeric"
            maxLength={6}
            value={otp}
            onChange={e => setOtp(e.target.value.replace(/\D/g, '').slice(0, 6))}
            onKeyDown={e => e.key === 'Enter' && otp.length === 6 && handleVerify()}
            autoFocus
          />
          {error && <p className="auth-flow__error">{error}</p>}
          <div className="auth-flow__actions">
            <button
              className="auth-flow__btn auth-flow__btn--ghost"
              onClick={() => { setStep('contact'); setOtp(''); setError(null) }}
            >
              {t('auth.back')}
            </button>
            <button
              className="auth-flow__btn auth-flow__btn--ghost"
              onClick={handleSendCode}
              disabled={countdown > 0 || loading}
            >
              {countdown > 0 ? `${t('auth.resend')} (${countdown}s)` : t('auth.resend')}
            </button>
            <button
              className="auth-flow__btn auth-flow__btn--primary"
              onClick={handleVerify}
              disabled={otp.length !== 6 || loading}
            >
              {loading ? '…' : t('auth.verify')}
            </button>
          </div>
        </div>
      )}

      {step === 'name' && (
        <div className="auth-flow__step">
          <h2 className="auth-flow__title">{t('auth.name-your-instance')}</h2>
          <input
            className="auth-flow__input"
            type="text"
            value={name}
            onChange={e => setName(e.target.value)}
            placeholder={t('auth.name-placeholder')}
            onKeyDown={e => e.key === 'Enter' && name.trim() && handleCreate()}
            autoFocus
            maxLength={30}
          />
          {error && <p className="auth-flow__error">{error}</p>}
          <div className="auth-flow__actions">
            <button
              className="auth-flow__btn auth-flow__btn--primary"
              onClick={handleCreate}
              disabled={!name.trim() || loading}
            >
              {loading ? '…' : t('auth.create')}
            </button>
          </div>
        </div>
      )}
    </div>
  )
}
