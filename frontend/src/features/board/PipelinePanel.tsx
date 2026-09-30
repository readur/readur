import { useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Button, Skeleton, useToast } from '../../ui';
import { useAuth } from '../../contexts/AuthContext';
import { isAdmin } from '../../auth/roles';
import { failureKind, humanizeFailureReason } from '../../lib/failureReason';
import { queueService } from '../../services/api';
import { POLL_MS, type FailedOcrPage, type QueueFigures } from './data';
import { formatAge, formatCount } from './format';
import { Region, RegionError } from './Region';
import { docName, type FailedOcrDocument } from './types';
import { useResource, type Resource } from './useResource';
import styles from './Home.module.css';

export interface PipelinePanelProps {
  failed: Resource<FailedOcrPage>;
  /** Queue figures; null for users who cannot see the queue. */
  stats: Resource<QueueFigures | null>;
}

/** How many recent failures the details list shows. */
const RECENT_FAILURES = 5;
/** How many distinct causes the failures line names. */
const CAUSES = 2;

const rawOf = (d: FailedOcrDocument) => d.error_message ?? '';

/** The most frequent causes among the recent failures, in plain words. */
function topCauses(docs: FailedOcrDocument[]): string[] {
  const counts = new Map<string, { n: number; summary: string }>();
  for (const d of docs) {
    const kind = failureKind(rawOf(d), d.failure_reason);
    const key = kind === 'other' ? `other:${humanizeFailureReason(rawOf(d), d.failure_reason).summary}` : kind;
    const entry = counts.get(key) ?? { n: 0, summary: humanizeFailureReason(rawOf(d), d.failure_reason).summary };
    entry.n += 1;
    counts.set(key, entry);
  }
  return [...counts.values()]
    .sort((a, b) => b.n - a.n)
    .slice(0, CAUSES)
    .map((c) => c.summary);
}

function ProcessingLine({ stats }: { stats: Resource<QueueFigures | null> }) {
  const { t, i18n } = useTranslation();
  const { user } = useAuth();
  const toast = useToast();
  const canManage = isAdmin(user);
  const ocr = useResource(() => queueService.getOcrStatus().then((r) => r.data), POLL_MS);
  const [busy, setBusy] = useState(false);
  const q = stats.data;
  const paused = ocr.data?.is_paused ?? false;

  const toggle = async (pause: boolean) => {
    setBusy(true);
    try {
      await (pause ? queueService.pauseOcr() : queueService.resumeOcr());
      ocr.reload();
    } catch {
      toast.show({
        title: pause ? t('board.processing.pauseFailed', 'Could not pause OCR') : t('board.processing.resumeFailed', 'Could not resume OCR'),
        tone: 'danger',
      });
    } finally {
      setBusy(false);
    }
  };

  if (stats.error && q === undefined) {
    return <RegionError message={t('board.processing.error', 'The OCR queue could not be loaded.')} onRetry={stats.reload} />;
  }
  if (q === undefined) return <Skeleton lines={1} label={t('board.loading', 'Loading')} />;
  if (q === null) return null;

  const idle = q.pending + q.processing === 0;
  const n = (v: number) => formatCount(v, i18n.language);
  return (
    <div className={styles.line} data-tone={paused ? 'warn' : idle ? 'ok' : 'accent'}>
      <span className={styles.lineMark} aria-hidden="true">
        {paused ? '◆' : idle ? '■' : '◐'}
      </span>
      <p className={styles.lineText}>
        {idle
          ? t('home.pipeline.idle', 'Nothing waiting')
          : t('home.pipeline.busy', 'Processing {{processing}} · pending {{pending}}', {
              processing: n(q.processing),
              pending: n(q.pending),
            })}
        {paused ? <span className={styles.lineNote}>{t('home.pipeline.paused', 'OCR is paused')}</span> : null}
      </p>
      {canManage && ocr.data ? (
        <Button size="sm" variant="secondary" isPending={busy} onPress={() => void toggle(!paused)}>
          {paused ? t('board.processing.resume', 'Resume OCR') : t('board.processing.pause', 'Pause OCR')}
        </Button>
      ) : null}
      {ocr.error && !ocr.data ? (
        <RegionError message={t('board.processing.statusError', 'The OCR state could not be loaded.')} onRetry={ocr.reload} />
      ) : null}
    </div>
  );
}

function FailureDetails({ docs }: { docs: FailedOcrDocument[] }) {
  const { t, i18n } = useTranslation();
  return (
    <details className={styles.details}>
      <summary>{t('home.pipeline.showRecent', 'Show the latest failures')}</summary>
      <ul className={styles.failures}>
        {docs.slice(0, RECENT_FAILURES).map((d) => {
          const human = humanizeFailureReason(rawOf(d), d.failure_reason);
          return (
            <li key={d.id} className={styles.failure}>
              <Link className={styles.failureName} to={`/documents/${d.id}`}>
                {docName(d)}
              </Link>
              <span className={styles.failureWhy}>{human.summary}</span>
              <span className={styles.failureAge}>{formatAge(d.last_retry_at || d.updated_at || d.created_at, i18n.language)}</span>
              {human.detail ? (
                <details className={styles.raw}>
                  <summary>{t('home.pipeline.rawError', 'Error text')}</summary>
                  <pre>{human.detail}</pre>
                </details>
              ) : null}
            </li>
          );
        })}
      </ul>
    </details>
  );
}

function FailuresLine({ failed }: { failed: Resource<FailedOcrPage> }) {
  const { t, i18n } = useTranslation();
  const docs = failed.data?.documents;
  const causes = useMemo(() => topCauses(docs ?? []), [docs]);

  if (failed.error && !failed.data) {
    return <RegionError message={t('home.pipeline.failedError', 'Failed documents could not be loaded.')} onRetry={failed.reload} />;
  }
  if (!failed.data) return null;
  const total = failed.data.total;
  if (total === 0) {
    return (
      <div className={styles.line} data-tone="ok">
        <span className={styles.lineMark} aria-hidden="true">
          ■
        </span>
        <p className={styles.lineText}>{t('home.pipeline.noFailures', 'No failed documents')}</p>
      </div>
    );
  }

  return (
    <div className={styles.failureBlock}>
      <div className={styles.line} data-tone="danger">
        <span className={styles.lineMark} aria-hidden="true">
          ▲
        </span>
        <p className={styles.lineText}>
          <strong className={styles.failCount}>
            {t('home.pipeline.failed', '{{formatted}} failed', { count: total, formatted: formatCount(total, i18n.language) })}
          </strong>
          {causes.length ? <span className={styles.lineNote}>{causes.join(' · ')}</span> : null}
        </p>
        <Link className={styles.secondaryLink} to="/intake?section=attention">
          {t('home.pipeline.review', 'Review')}
        </Link>
      </div>
      {failed.data.documents.length ? <FailureDetails docs={failed.data.documents} /> : null}
    </div>
  );
}

/** What is being processed and what failed, one line each. */
export function PipelinePanel({ failed, stats }: PipelinePanelProps) {
  const { t } = useTranslation();
  return (
    <Region title={t('home.pipeline.title', 'Processing')}>
      <div className={styles.lines}>
        <ProcessingLine stats={stats} />
        <FailuresLine failed={failed} />
      </div>
    </Region>
  );
}
