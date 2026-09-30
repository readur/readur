import { useCallback, useEffect, useId, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '../../../ui';
import { ocrService, type LanguageInfo } from '../../../services/api';
import { cx } from '../../../ui/shared/FieldParts';
import field from '../../../ui/shared/field.module.css';
import styles from './OcrLanguageSelector.module.css';

export interface OcrLanguageSelectorProps {
  value?: string;
  onChange: (language: string) => void;
  label?: string;
  /** `small` renders a compact control. */
  size?: 'small' | 'medium';
  fullWidth?: boolean;
  disabled?: boolean;
  showCurrentIndicator?: boolean;
  required?: boolean;
  helperText?: string;
}

/**
 * Single OCR language picker; defaults to (and marks) the user's current language.
 * A native select keeps it usable inside any modal, including legacy dialogs that trap focus.
 */
export function OcrLanguageSelector({
  value = '',
  onChange,
  label,
  size = 'medium',
  fullWidth = true,
  disabled = false,
  showCurrentIndicator = true,
  required = false,
  helperText,
}: OcrLanguageSelectorProps) {
  const { t } = useTranslation();
  const id = useId();
  const [languages, setLanguages] = useState<LanguageInfo[]>([]);
  const [current, setCurrent] = useState('eng');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  // Read the latest props inside the fetch without re-running it on every change.
  const latest = useRef({ value, onChange });
  latest.current = { value, onChange };
  const fieldLabel = label ?? t('ocr.languageSelector.label', 'OCR Language');

  const fetchLanguages = useCallback(async () => {
    setLoading(true);
    setError('');
    try {
      const response = await ocrService.getAvailableLanguages();
      setLanguages(response.data.available_languages);
      setCurrent(response.data.current_user_language);
      if (!latest.current.value) latest.current.onChange(response.data.current_user_language);
    } catch (err: unknown) {
      const message = (err as { response?: { data?: { message?: string } } })?.response?.data?.message;
      setError(message || t('ocr.languageSelector.error', 'Failed to load OCR languages'));
      setLanguages([{ code: 'eng', name: 'English', installed: true }]);
      if (!latest.current.value) latest.current.onChange('eng');
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    void fetchLanguages();
  }, [fetchLanguages]);

  const rootClass = cx(field.field, styles.root, fullWidth && styles.full, size === 'small' && styles.small);
  const nameOf = (code: string) => languages.find((l) => l.code === code)?.name ?? code;

  if (loading) {
    return (
      <div className={rootClass} role="status">
        <span className={styles.meta}>{t('ocr.languageSelector.loading', 'Loading languages...')}</span>
      </div>
    );
  }

  const helpId = `${id}-help`;
  const selected = error ? 'eng' : value || current;

  return (
    <div className={rootClass}>
      {error ? (
        <div className={styles.error} role="alert">
          <span>{error}</span>
          <Button size="sm" variant="ghost" onPress={() => void fetchLanguages()}>
            {t('ocr.languageSelector.retry', 'Retry')}
          </Button>
        </div>
      ) : null}
      <label htmlFor={id} className={field.label}>
        {fieldLabel}
        {required ? <span aria-hidden="true"> *</span> : null}
      </label>
      <select
        id={id}
        className={cx(field.control, styles.select)}
        value={selected}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled || Boolean(error)}
        required={required}
        aria-describedby={helperText ? helpId : undefined}
      >
        {error ? (
          <option value="eng">{t('ocr.languageSelector.fallback', 'English (Fallback)')}</option>
        ) : (
          languages.map((lang) => (
            <option key={lang.code} value={lang.code}>
              {lang.name} ({lang.code})
              {showCurrentIndicator && lang.code === current ? ` · ${t('ocr.languageSelector.current', 'Current')}` : ''}
            </option>
          ))
        )}
      </select>
      {helperText ? (
        <span id={helpId} className={field.description}>
          {helperText}
        </span>
      ) : null}
      {!error && showCurrentIndicator && languages.length > 0 ? (
        <p className={styles.meta}>
          {t('ocr.languageSelector.languagesAvailable', {
            count: languages.length,
            plural: languages.length !== 1 ? 's' : '',
          })}
          {value && value !== current
            ? ` • ${t('ocr.languageSelector.selectingWillUpdate', { language: nameOf(value) })}`
            : null}
        </p>
      ) : null}
    </div>
  );
}

export default OcrLanguageSelector;
