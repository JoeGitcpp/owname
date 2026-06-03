import { useEffect } from 'react'
import { useI18n, type DictKey } from '../i18n'

// A simple reading overlay for About / Team / Legal. Body text supports
// paragraph breaks via blank lines in the dictionary.
export function Overlay({
  titleKey,
  bodyKey,
  onClose,
}: {
  titleKey: DictKey
  bodyKey: DictKey
  onClose: () => void
}) {
  const { t } = useI18n()

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [onClose])

  const paragraphs = t(bodyKey).split('\n\n')

  return (
    <div className="overlay fade-in" onClick={onClose}>
      <div className="overlay__panel rise" onClick={(e) => e.stopPropagation()}>
        <button className="overlay__close" onClick={onClose} aria-label={t('overlay.close')}>
          ×
        </button>
        <h2 className="overlay__title">{t(titleKey)}</h2>
        <div className="overlay__body">
          {paragraphs.map((p, i) => (
            <p key={i}>{p}</p>
          ))}
        </div>
      </div>
    </div>
  )
}
