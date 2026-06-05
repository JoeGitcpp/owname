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
  const [debriefSecs, setDebriefSecs]   = useState(0)
  const [instanceId, setInstanceId]     = useState<string | null>(null)
  const [instanceName, setInstanceName] = useState<string | null>(null)

  const setLang = (l: Lang) => {
    setLangState(l)
    try { localStorage.setItem(LANG_KEY, l) } catch {}
  }

  useEffect(() => { document.documentElement.lang = lang }, [lang])

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
