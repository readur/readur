import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { ButtonLink, Notice } from '../../ui';
import { failureKind, humanizeFailureReason } from '../../lib/failureReason';
import type { FailedOcrPage } from './data';
import { formatCount } from './format';
import { Region, RegionError } from './Region';
import type { FailedOcrDocument } from './types';
import type { Resource } from './useResource';
import styles from './Home.module.css';

/** How many distinct causes the notice names. */
const CAUSES = 2;

const rawOf = (d: FailedOcrDocument) => d.error_message ?? '';

/** The most frequent causes among the recent failures, in plain words. */
export function topCauses(docs: FailedOcrDocument[]): string[] {
  const counts = new Map<string, { n: number; summary: string }>();
  for (const d of docs) {
    const kind = failureKind(rawOf(d), d.failure_reason);
    const summary = humanizeFailureReason(rawOf(d), d.failure_reason).summary;
    const key = kind === 'other' ? `other:${summary}` : kind;
    const entry = counts.get(key) ?? { n: 0, summary };
    entry.n += 1;
    counts.set(key, entry);
  }
  return [...counts.values()]
    .sort((a, b) => b.n - a.n)
    .slice(0, CAUSES)
    .map((c) => c.summary);
}

/** What needs a person: failed OCR, as a notice with its main causes and a Review action. */
export function NeedsAttention({ failed }: { failed: Resource<FailedOcrPage> }) {
  const { t, i18n } = useTranslation();
  const docs = failed.data?.documents;
  const causes = useMemo(() => topCauses(docs ?? []), [docs]);
  const title = t('home.attention.title', 'Needs attention');

  if (failed.error && !failed.data) {
    return (
      <Region surface="plain" title={title} className={styles.attention}>
        <RegionError message={t('home.pipeline.failedError', 'Failed documents could not be loaded.')} onRetry={failed.reload} />
      </Region>
    );
  }
  const total = failed.data?.total ?? 0;
  if (total === 0) return null;

  return (
    <Region surface="plain" title={title} count={1} className={styles.attention}>
      <Notice
        tone="danger"
        live="off"
        title={t('home.attention.failed', '{{formatted}} documents failed OCR', {
          count: total,
          formatted: formatCount(total, i18n.language),
        })}
        action={
          <ButtonLink href="/intake?section=attention" variant="ghost" size="sm">
            {t('home.pipeline.review', 'Review')}
          </ButtonLink>
        }
      >
        {causes.length ? causes.join(' · ') : null}
      </Notice>
    </Region>
  );
}
