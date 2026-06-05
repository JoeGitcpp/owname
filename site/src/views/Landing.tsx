import { useState } from 'react'
import { useI18n, type DictKey } from '../i18n'
import { Field } from '../components/Field'
import { Overlay } from '../components/Overlay'
import { idleSenses } from '../lib/idle'
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
  const idle = idleSenses()

  return (
    <div className="landing">
      <Field read={() => idle} className="landing__field" readout={false} />
      <div className="landing__veil" />

      <header className="landing__top">
        <span className="landing__mark">aborn</span>
        <button
          className="landing__lang"
          onClick={() => setLang(lang === 'zh' ? 'en' : 'zh')}
          aria-label="toggle language"
        >
          {t('nav.lang')}
        </button>
      </header>

      <main className="landing__center">
        <button className="commune-btn" onClick={onEnter}>
          <span className="commune-btn__ring" />
          <span className="commune-btn__label">{t('landing.guest')}</span>
        </button>
        <p className="landing__tagline">{t('tagline')}</p>
        <p className="landing__sub">{t('enter.sub')}</p>

        {/* Login option — upgrade to a persistent instance */}
        <div className="landing__login-section">
          <p className="landing__login-hint">{t('landing.login.hint')}</p>
          <button className="landing__login-btn" onClick={onLogin}>
            {t('landing.login')}
          </button>
        </div>
      </main>

      <footer className="landing__foot">
        <button className="foot-link" onClick={() => setOverlay('about')}>
          {t('nav.about')}
        </button>
        <span className="foot-dot">·</span>
        <button className="foot-link" onClick={() => setOverlay('team')}>
          {t('nav.team')}
        </button>
        <span className="foot-dot">·</span>
        <button className="foot-link" onClick={() => setOverlay('legal')}>
          {t('nav.legal')}
        </button>
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
