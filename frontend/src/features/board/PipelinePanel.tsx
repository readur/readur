import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button, OutcomeBar, Skeleton, useToast, type OutcomeSegment } from '../../ui';
import { useAuth } from '../../contexts/AuthContext';
import { isAdmin } from '../../auth/roles';
import { queueService } from '../../services/api';
import { POLL_MS, type FailedOcrPage, type QueueFigures } from './data';
import { formatCount, formatMinutes } from './format';
import { Region, RegionError } from './Region';
import { useResource, type Resource } from './useResource';
import styles from './Home.module.css';

export interface PipelinePanelProps {
  failed: Resource<FailedOcrPage>;
  /** Queue figures; null for users who cannot see the queue. */
  stats: Resource<QueueFigures | null>;
}

/** Pause or resume OCR (admins only), with the OCR state it reflects. */
function useOcrToggle() {
  const { t } = useTranslation();
  const toast = useToast();
  const ocr = useResource(() => queueService.getOcrStatus().then((r) => r.data), POLL_MS);
  const [busy, setBusy] = useState(false);
  const paused = ocr.data?.is_paused ?? false;
  const toggle = async () => {
    const pause = !paused;
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
  return { ocr, paused, busy, toggle };
}

/**
 * Today's OCR pipeline as one segmented bar (done, processing, failed, queued) with "done / total"
 * beside the title, pause and resume for admins, and the oldest wait. Users who cannot see the
 * queue get only the failures; with nothing to show the card is left out.
 */
export function PipelinePanel({ failed, stats }: PipelinePanelProps) {
  const { t, i18n } = useTranslation();
  const { user } = useAuth();
  const { ocr, paused, busy, toggle } = useOcrToggle();
  const n = (v: number) => formatCount(v, i18n.language);
  const q = stats.data;
  const failedTotal = failed.data?.total ?? 0;

  if (q === null && !stats.error && failedTotal === 0) return null;

  const segments: OutcomeSegment[] = q
    ? [
        { id: 'done', label: t('home.pipeline.done', 'Done today'), value: q.completedToday, tone: 'ok' },
        { id: 'processing', label: t('home.pipeline.processing', 'Processing'), value: q.processing, tone: 'accent' },
        { id: 'failed', label: t('home.pipeline.failedSegment', 'Failed'), value: failedTotal, tone: 'danger' },
        { id: 'queued', label: t('home.pipeline.queued', 'Queued'), value: q.pending, tone: 'neutral' },
      ]
    : [{ id: 'failed', label: t('home.pipeline.failedSegment', 'Failed'), value: failedTotal, tone: 'danger' }];
  const total = segments.reduce((sum, s) => sum + s.value, 0);
  const idle = q ? q.pending + q.processing === 0 : true;

  const notes = [
    !idle && q && q.oldestPendingMinutes !== null && q.oldestPendingMinutes >= 1
      ? t('home.pipeline.oldest', 'oldest waiting {{wait}}', { wait: formatMinutes(q.oldestPendingMinutes) })
      : null,
    paused ? t('home.pipeline.paused', 'OCR is paused') : null,
  ].filter((x): x is string => Boolean(x));

  const title = t('home.pipeline.cardTitle', 'Processing pipeline');
  return (
    <Region
      title={title}
      className={styles.pipeline}
      headerAction={
        <>
          {q ? (
            <span className={styles.figure}>
              {n(q.completedToday)} / {n(total)}
            </span>
          ) : null}
          {isAdmin(user) && ocr.data ? (
            <Button size="sm" variant="secondary" isPending={busy} onPress={() => void toggle()}>
              {paused ? t('board.processing.resume', 'Resume OCR') : t('board.processing.pause', 'Pause OCR')}
            </Button>
          ) : null}
        </>
      }
    >
      {stats.error && q === undefined ? (
        <RegionError message={t('board.processing.error', 'The OCR queue could not be loaded.')} onRetry={stats.reload} />
      ) : q === undefined ? (
        <Skeleton lines={1} label={t('board.loading', 'Loading')} />
      ) : (
        <OutcomeBar label={title} segments={segments} />
      )}
      {notes.length ? (
        <p className={styles.note}>
          {notes.map((note) => (
            <span key={note} className={styles.notePart}>
              {note}
            </span>
          ))}
        </p>
      ) : null}
      {ocr.error && !ocr.data ? (
        <RegionError message={t('board.processing.statusError', 'The OCR state could not be loaded.')} onRetry={ocr.reload} />
      ) : null}
    </Region>
  );
}
