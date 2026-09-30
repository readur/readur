import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { TFunction } from 'i18next';
import { useLastSynced } from './useLastSynced';
import styles from './AppShell.module.css';

const MINUTE = 60_000;

export function formatSynced(t: TFunction, at: Date, now: number): string {
  const minutes = Math.max(0, Math.floor((now - at.getTime()) / MINUTE));
  if (minutes < 1) return t('shell.synced.justNow', 'synced just now');
  if (minutes < 60) return t('shell.synced.minutes', { count: minutes, defaultValue: 'synced {{count}}m ago' });
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return t('shell.synced.hours', { count: hours, defaultValue: 'synced {{count}}h ago' });
  return t('shell.synced.days', { count: Math.floor(hours / 24), defaultValue: 'synced {{count}}d ago' });
}

/** Mono "synced 2m ago" readout. Renders nothing until some source has synced. */
export function SyncedReadout() {
  const { t } = useTranslation();
  const last = useLastSynced();
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    setNow(Date.now());
    const timer = window.setInterval(() => setNow(Date.now()), 30_000);
    return () => window.clearInterval(timer);
  }, [last]);

  if (!last) return null;
  return (
    <time className={styles.synced} dateTime={last.toISOString()} title={last.toLocaleString()}>
      {formatSynced(t, last, now)}
    </time>
  );
}
