import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { formatRelativeTime } from '../../lib/relativeTime';
import type { TFunction } from 'i18next';
import styles from './Sidebar.module.css';

/** "synced 21 min. ago" through the shared relative-time formatter, in the UI language. */
export function formatSynced(t: TFunction, at: Date, now: number, locale?: string): string {
  const when = formatRelativeTime(at, { now, locale, absoluteAfterDays: null });
  return t('shell.synced.at', { when, defaultValue: 'synced {{when}}' });
}

/** "synced 2 min. ago" for the most recent sync across sources. Renders nothing until one has synced. */
export function SyncedReadout({ last }: { last: Date | null }) {
  const { t, i18n } = useTranslation();
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    setNow(Date.now());
    const timer = window.setInterval(() => setNow(Date.now()), 30_000);
    return () => window.clearInterval(timer);
  }, [last]);

  if (!last) return null;
  return (
    <time className={styles.synced} dateTime={last.toISOString()} title={last.toLocaleString()}>
      {formatSynced(t, last, now, i18n.language)}
    </time>
  );
}
