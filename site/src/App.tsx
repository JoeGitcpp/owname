import { useEffect, useState } from 'react'
import { I18nContext, type Lang } from './i18n'
import { Landing } from './views/Landing'
import { Commune } from './views/Commune'

type Stage = 'landing' | 'commune'

const LANG_KEY = 'aborn.lang'

function initialLang(): Lang {
  const saved = typeof localStorage !== 'undefined' ? localStorage.getItem(LANG_KEY) : null
  if (saved === 'zh' || saved === 'en') return saved
  const nav = typeof navigator !== 'undefined' ? navigator.language.toLowerCase() : 'zh'
  return nav.startsWith('zh') ? 'zh' : 'en'
}

export function App() {
  const [lang, setLangState] = useState<Lang>(initialLang)
  const [stage, setStage] = useState<Stage>('landing')

  const setLang = (l: Lang) => {
    setLangState(l)
    try {
      localStorage.setItem(LANG_KEY, l)
    } catch {
      /* ignore storage failures (private mode) */
    }
  }

  useEffect(() => {
    document.documentElement.lang = lang
  }, [lang])

  return (
    <I18nContext.Provider value={{ lang, setLang }}>
      {stage === 'landing' ? (
        <Landing onEnter={() => setStage('commune')} />
      ) : (
        <Commune onLeave={() => setStage('landing')} />
      )}
    </I18nContext.Provider>
  )
}
