import { useI18n } from '../i18n'
import './debrief.css'

const MIN_TRAINING_SECS = 5 * 60  // 5 minutes — mirrors training_pool.mjs threshold

function fmtDuration(secs: number, lang: 'zh' | 'en'): string {
  const m = Math.floor(secs / 60)
  const s = secs % 60
  if (lang === 'zh') {
    if (m > 0) return `${m} 分 ${s} 秒`
    return `${s} 秒`
  }
  if (m > 0) return `${m} min ${s} sec`
  return `${s} sec`
}

async function share(text: string) {
  if (navigator.share) {
    try {
      await navigator.share({ text, url: 'https://aborn.live' })
      return
    } catch { /* user cancelled or not supported */ }
  }
  try {
    await navigator.clipboard.writeText(`${text} https://aborn.live`)
  } catch { /* clipboard unavailable */ }
}

export function Debrief({
  secs,
  onReenter,
  onHome,
}: {
  secs: number
  onReenter: () => void
  onHome: () => void
}) {
  const { t, lang } = useI18n()
  const trained = secs >= MIN_TRAINING_SECS

  return (
    <div className="debrief">
      <div className="debrief__card fade-in">
        <p className="debrief__label">{t('debrief.nurtured')}</p>
        <p className="debrief__duration">{fmtDuration(secs, lang)}</p>
        <p className={`debrief__status ${trained ? 'debrief__status--trained' : 'debrief__status--short'}`}>
          {t(trained ? 'debrief.trained' : 'debrief.short')}
        </p>

        <div className="debrief__actions">
          <button className="debrief__btn debrief__btn--primary" onClick={onReenter}>
            {t('debrief.again')}
          </button>
          <button
            className="debrief__btn debrief__btn--share"
            onClick={() => share(t('debrief.share-text'))}
            aria-label={t('debrief.share')}
          >
            <ShareIcon />
            {t('debrief.share')}
          </button>
          <button className="debrief__btn" onClick={onHome}>
            {t('debrief.home')}
          </button>
        </div>
      </div>
    </div>
  )
}

function ShareIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.8}
         strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <circle cx="18" cy="5" r="3" />
      <circle cx="6" cy="12" r="3" />
      <circle cx="18" cy="19" r="3" />
      <line x1="8.59" y1="13.51" x2="15.42" y2="17.49" />
      <line x1="15.41" y1="6.51" x2="8.59" y2="10.49" />
    </svg>
  )
}
