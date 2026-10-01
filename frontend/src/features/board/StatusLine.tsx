import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import type { FailedOcrPage, QueueFigures } from './data';
import { formatCount } from './format';
import styles from './Home.module.css';

export interface StatusLineProps {
  /** Documents that arrived this week; undefined until known. */
  week: number | undefined;
  /** Queue figures; null when this user cannot see the queue, undefined until known. */
  queue: QueueFigures | null | undefined;
  failed: FailedOcrPage | undefined;
}

/**
 * The page's answer to "is it flowing?" in one row: what arrived this week, what is being
 * processed, and what failed. Parts that are not known yet (or not visible to this user) are left out.
 */
export function StatusLine({ week, queue, failed }: StatusLineProps) {
  const { t, i18n } = useTranslation();
  const n = (v: number) => formatCount(v, i18n.language);
  const busy = queue ? queue.processing + queue.pending : 0;

  return (
    <p className={styles.status} role="group" aria-label={t('home.status.label', 'Summary')}>
      {week !== undefined ? (
        <span className={styles.statusPart}>
          {week === 0
            ? t('home.week.none', 'Nothing arrived this week')
            : t('home.week.count', '{{formatted}} arrived this week', { count: week, formatted: n(week) })}
        </span>
      ) : null}
      {queue ? (
        <span className={styles.statusPart}>
          {busy === 0
            ? t('home.status.idle', 'nothing waiting')
            : queue.processing > 0
              ? t('home.status.busy', '{{processing}} processing · {{pending}} pending', {
                  processing: n(queue.processing),
                  pending: n(queue.pending),
                })
              : t('home.status.pending', '{{pending}} pending', { pending: n(queue.pending) })}
        </span>
      ) : null}
      {failed && failed.total > 0 ? (
        <span className={styles.statusPart}>
          <span className={styles.statusFailed}>
            <span aria-hidden="true">▲</span>{' '}
            <span>{t('home.pipeline.failed', '{{formatted}} failed', { count: failed.total, formatted: n(failed.total) })}</span>
          </span>
          <Link className={styles.statusLink} to="/intake?section=attention">
            {t('home.pipeline.review', 'Review')}
          </Link>
        </span>
      ) : null}
    </p>
  );
}
