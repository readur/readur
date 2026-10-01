import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import styles from './Details.module.css';

export interface MetadataDisplayProps {
  /** Arbitrary key/value metadata (EXIF, PDF info, source attributes…). */
  metadata: unknown;
  title?: string;
  /** Collapsed behind a disclosure, with a field count. Default false (always open). */
  compact?: boolean;
}

/**
 * Keys the server fills with values that can't be trusted. `page_count` counts "/Type /Page" in
 * the raw PDF, which also matches "/Type /Pages", so it overstates every PDF by one or more.
 */
const UNRELIABLE = new Set(['page_count']);

const DATE_HINTS = ['date', 'time', 'created', 'modified'];

const ACRONYMS = new Set(['pdf', 'id', 'url', 'exif', 'gps', 'dpi', 'iso', 'ocr', 'mime']);

/** `pdf_creation_date` → "PDF creation date": sentence case, known acronyms kept upper. */
export function formatKeyName(key: string): string {
  const words = key
    .replace(/([a-z])([A-Z])/g, '$1 $2')
    .replace(/[_-]+/g, ' ')
    .trim()
    .split(/\s+/)
    .map((w) => (ACRONYMS.has(w.toLowerCase()) ? w.toUpperCase() : w.toLowerCase()));
  const sentence = words.join(' ');
  return sentence.charAt(0).toUpperCase() + sentence.slice(1);
}

function MetadataValue({ name, value }: { name: string; value: unknown }): ReactNode {
  const { t, i18n } = useTranslation();
  if (name === 'permissions' && typeof value === 'number') {
    return <span className={styles.mono}>{`${value.toString(8)} (${t('document.details.octal', 'octal')})`}</span>;
  }
  if (Array.isArray(value)) {
    return (
      <span className={styles.tags}>
        {value.map((item, i) => (
          <span key={i} className={styles.tag}>
            {String(item)}
          </span>
        ))}
      </span>
    );
  }
  if (value !== null && typeof value === 'object') {
    return <pre className={styles.json}>{JSON.stringify(value, null, 2)}</pre>;
  }
  if (typeof value === 'boolean') {
    return <>{value ? t('document.details.yes', 'Yes') : t('document.details.no', 'No')}</>;
  }
  if (typeof value === 'string' && DATE_HINTS.some((h) => name.toLowerCase().includes(h))) {
    const date = new Date(value);
    if (!Number.isNaN(date.getTime())) return <>{date.toLocaleString(i18n.language)}</>;
  }
  if (typeof value === 'number') return <>{value}</>;
  return <>{String(value)}</>;
}

/** Labelled grid of free-form metadata fields. Renders nothing when there are none. */
export function MetadataDisplay({ metadata, title, compact = false }: MetadataDisplayProps) {
  const { t } = useTranslation();
  if (!metadata || typeof metadata !== 'object') return null;
  const entries = Object.entries(metadata as Record<string, unknown>).filter(([key]) => !UNRELIABLE.has(key));
  if (entries.length === 0) return null;
  const heading = title ?? t('document.details.sourceMetadata', 'Source metadata');

  const grid = (
    <dl className={styles.grid}>
      {entries.map(([key, value]) => (
        <div key={key} className={styles.row}>
          <dt className={styles.term}>{formatKeyName(key)}</dt>
          <dd className={styles.value}>
            <MetadataValue name={key} value={value} />
          </dd>
        </div>
      ))}
    </dl>
  );

  if (compact) {
    return (
      <details className={styles.section}>
        <summary className={styles.summary}>
          <span>{heading}</span>
          <span className={styles.count}>
            {t('document.details.fieldCount', { count: entries.length, defaultValue: '{{count}} fields' })}
          </span>
        </summary>
        <div className={styles.sectionBody}>{grid}</div>
      </details>
    );
  }
  return (
    <div className={styles.block}>
      <h2 className={styles.blockTitle}>{heading}</h2>
      {grid}
    </div>
  );
}

export default MetadataDisplay;
