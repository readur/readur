import { useEffect, useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button, Checkbox, IconButton } from '../../ui'
import { Close } from '../../ui/icons'
import { ocrService, type LanguageInfo } from '../../services/api'
import styles from './LanguageSelector.module.css'

interface LanguageSelectorProps {
  selectedLanguages: string[]
  primaryLanguage?: string
  onLanguagesChange: (languages: string[], primary?: string) => void
  maxLanguages?: number
  disabled?: boolean
  showPrimarySelector?: boolean
  className?: string
}

/**
 * Pick up to `maxLanguages` installed OCR languages and mark one as primary.
 * The picker unfolds inline (no portal), so it also works inside modal dialogs.
 */
function LanguageSelector({
  selectedLanguages,
  primaryLanguage,
  onLanguagesChange,
  maxLanguages = 4,
  disabled = false,
  showPrimarySelector = true,
  className = '',
}: LanguageSelectorProps) {
  const { t } = useTranslation()
  const panelId = useId()
  const [availableLanguages, setAvailableLanguages] = useState<LanguageInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const [isOpen, setIsOpen] = useState(false)

  useEffect(() => {
    let alive = true
    const fetchLanguages = async () => {
      try {
        setLoading(true)
        setError('')
        const response = await ocrService.getAvailableLanguages()
        if (alive) setAvailableLanguages(response.data.available_languages)
      } catch (err: unknown) {
        const message = (err as { response?: { data?: { message?: string } } })?.response?.data?.message
        if (alive) {
          setError(message || t('settings.languageSelector.loadFailed', 'Failed to load OCR languages'))
          setAvailableLanguages([{ code: 'eng', name: 'English', installed: true }])
        }
      } finally {
        if (alive) setLoading(false)
      }
    }
    void fetchLanguages()
    return () => {
      alive = false
    }
  }, [t])

  // The first selected language is primary when none is given.
  const effectivePrimary = primaryLanguage || selectedLanguages[0] || ''
  const remaining = Math.max(0, maxLanguages - selectedLanguages.length)
  const atMax = selectedLanguages.length >= maxLanguages

  const toggleLanguage = (code: string) => {
    if (disabled) return
    let next: string[]
    let nextPrimary = effectivePrimary
    if (selectedLanguages.includes(code)) {
      next = selectedLanguages.filter((c) => c !== code)
      if (code === effectivePrimary && next.length > 0) nextPrimary = next[0]
      else if (next.length === 0) nextPrimary = ''
    } else {
      if (atMax) return
      next = [...selectedLanguages, code]
      if (next.length === 1) nextPrimary = code
    }
    onLanguagesChange(next, nextPrimary)
  }

  const setPrimary = (code: string) => {
    if (disabled || !selectedLanguages.includes(code)) return
    onLanguagesChange(selectedLanguages, code)
  }

  const nameOf = (code: string) => availableLanguages.find((l) => l.code === code)?.name || code

  if (loading) {
    return (
      <div className={className} role="status">
        <span className={styles.meta}>{t('settings.languageSelector.loading', 'Loading languages...')}</span>
      </div>
    )
  }

  if (error) {
    return (
      <div className={className}>
        <p className={styles.error} role="alert">
          {error}
        </p>
      </div>
    )
  }

  const title = t('settings.languageSelector.title', 'OCR Languages')

  return (
    <div className={`${styles.root} ${className}`.trim()}>
      <p className={styles.heading}>
        {selectedLanguages.length > 0 ? `${title} (${selectedLanguages.length}/${maxLanguages})` : title}
      </p>

      {selectedLanguages.length > 0 ? (
        <ul className={styles.tags} aria-label={t('settings.languageSelector.selected', 'Selected languages')}>
          {selectedLanguages.map((code) => {
            const isPrimary = code === effectivePrimary
            return (
              <li key={code} className={styles.tag} data-primary={isPrimary || undefined}>
                <span>{nameOf(code)}</span>
                {isPrimary ? (
                  <span className={styles.primaryNote}>{t('settings.languageSelector.primaryNote', '(Primary)')}</span>
                ) : null}
                {!disabled ? (
                  <IconButton
                    size="sm"
                    label={t('settings.languageSelector.remove', 'Remove {{language}}', { language: nameOf(code) })}
                    icon={<Close fontSize="inherit" />}
                    onPress={() => toggleLanguage(code)}
                    className={styles.remove}
                  />
                ) : null}
              </li>
            )
          })}
        </ul>
      ) : (
        <p className={styles.empty}>
          {t('settings.languageSelector.none', 'No languages selected. Documents will use default OCR language.')}
        </p>
      )}

      {!disabled ? (
        <Button
          className={styles.trigger}
          aria-expanded={isOpen}
          aria-controls={panelId}
          onPress={() => setIsOpen((o) => !o)}
        >
          {selectedLanguages.length === 0
            ? t('settings.languageSelector.select', 'Select OCR languages...')
            : t('settings.languageSelector.addMore', 'Add more languages ({{count}} remaining)', { count: remaining })}
        </Button>
      ) : null}

      {isOpen && !disabled ? (
        <div id={panelId} className={styles.panel} role="group" aria-labelledby={`${panelId}-title`}>
          <p id={`${panelId}-title`} className={styles.panelTitle}>
            {t('settings.languageSelector.available', 'Available Languages')}
          </p>
          <ul className={styles.options}>
            {availableLanguages
              .filter((lang) => lang.installed)
              .map((lang) => {
                const isSelected = selectedLanguages.includes(lang.code)
                const isPrimary = lang.code === effectivePrimary
                return (
                  <li key={lang.code} className={styles.option} data-selected={isSelected || undefined}>
                    <Checkbox
                      label={lang.name}
                      isSelected={isSelected}
                      isDisabled={!isSelected && atMax}
                      onChange={() => toggleLanguage(lang.code)}
                    />
                    {isPrimary ? <span className={styles.primaryTag}>{t('settings.languageSelector.primaryTag', 'Primary')}</span> : null}
                    {isSelected && showPrimarySelector && selectedLanguages.length > 1 && !isPrimary ? (
                      <Button size="sm" variant="ghost" onPress={() => setPrimary(lang.code)}>
                        {`${t('settings.languageSelector.setPrimary', 'Set Primary')} `}
                        <span className="visually-hidden">{lang.name}</span>
                      </Button>
                    ) : null}
                  </li>
                )
              })}
          </ul>
          {atMax ? (
            <p className={styles.limit} role="status">
              {t('settings.languageSelector.max', 'Maximum {{count}} languages allowed for optimal performance.', {
                count: maxLanguages,
              })}
            </p>
          ) : null}
          <Button variant="ghost" onPress={() => setIsOpen(false)} className={styles.close}>
            {t('common.actions.close', 'Close')}
          </Button>
        </div>
      ) : null}

      {selectedLanguages.length > 1 ? (
        <p className={styles.meta}>
          {t(
            'settings.languageSelector.help',
            'The primary language is processed first for better accuracy. Multiple languages help with mixed-language documents.',
          )}
        </p>
      ) : null}
    </div>
  )
}

export default LanguageSelector
