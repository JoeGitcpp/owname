import { useState } from 'react'
import { useI18n, type DictKey } from '../i18n'
import { Overlay } from '../components/Overlay'
import './landing.css'

type OverlayId = 'about' | 'team' | 'legal'

const OVERLAYS: Record<OverlayId, { title: DictKey; body: DictKey }> = {
  about: { title: 'about.title', body: 'about.body' },
  team: { title: 'team.title', body: 'team.body' },
  legal: { title: 'legal.title', body: 'legal.body' },
}

export function Landing({ onEnter, onLogin }: { onEnter: () => void; onLogin: () => void }) {
  const { t, lang, setLang } = useI18n()
  const [overlay, setOverlay] = useState<OverlayId | null>(null)

  return (
    <div className="landing">
      <header className="landing__top">
        <button
          className="landing__lang"
          onClick={() => setLang(lang === 'zh' ? 'en' : 'zh')}
          aria-label="toggle language"
        >
          {t('nav.lang')}
        </button>
      </header>

      <main className="landing__center">
        <h1 className="landing__hero">aborn</h1>
        <p className="landing__tagline">{t('tagline')}</p>

        <div className="landing__actions">
          <button className="landing__enter" onClick={onEnter}>
            {t('landing.guest')}
          </button>
          <button className="landing__login" onClick={onLogin}>
            {t('landing.login')}
          </button>
        </div>

        <p className="landing__sub">{t('enter.sub')}</p>
      </main>

      <footer className="landing__foot">
        <button className="foot-link" onClick={() => setOverlay('about')}>{t('nav.about')}</button>
        <span className="foot-dot">·</span>
        <button className="foot-link" onClick={() => setOverlay('team')}>{t('nav.team')}</button>
        <span className="foot-dot">·</span>
        <button className="foot-link" onClick={() => setOverlay('legal')}>{t('nav.legal')}</button>
      </footer>

      {overlay && (
        <Overlay
          titleKey={OVERLAYS[overlay].title}
          bodyKey={OVERLAYS[overlay].body}
          onClose={() => setOverlay(null)}
        />
      )}
    </div>
  )
}
