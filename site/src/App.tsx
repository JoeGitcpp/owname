import { useEffect, useState } from 'react'
import { I18nContext, type Lang } from './i18n'
import { Landing }      from './views/Landing'
import { AuthFlow }     from './views/AuthFlow'
import { Commune }      from './views/Commune'
import { InstanceView } from './views/InstanceView'
import { Debrief }      from './views/Debrief'
import './views/auth-flow.css'

type Stage = 'landing' | 'auth' | 'commune' | 'instance' | 'debrief'

const LANG_KEY = 'aborn.lang'

function initialLang(): Lang {
  const saved = typeof localStorage !== 'undefined' ? localStorage.getItem(LANG_KEY) : null
  if (saved === 'zh' || saved === 'en') return saved
  const nav = typeof navigator !== 'undefined' ? navigator.language.toLowerCase() : 'zh'
  return nav.startsWith('zh') ? 'zh' : 'en'
}

export function App() {
  const [lang, setLangState]    = useState<Lang>(initialLang)
  const [stage, setStage]       = useState<Stage>('landing')
  const [booting, setBooting]   = useState(true)
  const [debriefSecs, setDebriefSecs]   = useState(0)
  const [instanceId, setInstanceId]     = useState<string | null>(null)
  const [instanceName, setInstanceName] = useState<string | null>(null)

  const setLang = (l: Lang) => {
    setLangState(l)
    try { localStorage.setItem(LANG_KEY, l) } catch {}
  }

  useEffect(() => { document.documentElement.lang = lang }, [lang])

  // On load, restore login: if a valid session cookie exists, resume the user's
  // persistent instance instead of showing landing. This is what makes a
  // returning logged-in user reconnect to THEIR AI (and survive a reload),
  // rather than starting a fresh anonymous session.
  useEffect(() => {
    let cancelled = false
    ;(async () => {
      try {
        const chk = await fetch('/auth/check').then(r => r.json())
        if (!cancelled && chk?.authed) {
          const me = await fetch('/instances/me').then(r => r.json())
          if (!cancelled && me?.instance) {
            setInstanceId(me.instance.id)
            setInstanceName(me.instance.name ?? me.instance.identityKey ?? 'instance')
            setStage('instance')
          }
        }
      } catch { /* offline / not authed → stay on landing */ }
      if (!cancelled) setBooting(false)
    })()
    return () => { cancelled = true }
  }, [])

  if (booting) {
    return (
      <I18nContext.Provider value={{ lang, setLang }}>
        <div style={{ position: 'fixed', inset: 0, display: 'grid', placeItems: 'center',
                      color: 'var(--ink-dim)', fontFamily: 'var(--font-mono)',
                      fontSize: 12, letterSpacing: '.12em' }}>···</div>
      </I18nContext.Provider>
    )
  }

  return (
    <I18nContext.Provider value={{ lang, setLang }}>
      {stage === 'landing' && (
        <Landing
          onEnter={() => setStage('commune')}
          onLogin={() => setStage('auth')}
        />
      )}

      {stage === 'auth' && (
        <AuthFlow
          onComplete={(id, name) => {
            setInstanceId(id)
            setInstanceName(name)
            setStage('instance')
          }}
          onBack={() => setStage('landing')}
        />
      )}

      {stage === 'commune' && (
        <Commune
          onLeave={(secs) => {
            setDebriefSecs(secs)
            setStage('debrief')
          }}
        />
      )}

      {stage === 'instance' && instanceId && instanceName && (
        <InstanceView
          instanceId={instanceId}
          instanceName={instanceName}
          onLeave={() => setStage('landing')}
        />
      )}

      {stage === 'debrief' && (
        <Debrief
          secs={debriefSecs}
          onReenter={() => setStage('commune')}
          onHome={() => setStage('landing')}
        />
      )}
    </I18nContext.Provider>
  )
}
