import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button, Skeleton, StatusMark, useToast } from '../../ui';
import { useAuth } from '../../contexts/AuthContext';
import { isAdmin } from '../../auth/roles';
import { queueService } from '../../services/api';
import { POLL_MS, type FailedOcrPage, type QueueFigures } from './data';
import { Figure, Figures } from './Figures';
import { formatCount, formatMinutes } from './format';
import { Region, RegionError } from './Region';
import { useResource, type Resource } from './useResource';
import styles from './Board.module.css';

export interface ProcessingPanelProps {
  /**
   * The failed-OCR documents the Needs attention list shows. FAILED counts these, so the figure
   * and the list always agree (the OCR queue's own failure count tracks queue jobs, not documents).
   */
  failed: Resource<FailedOcrPage>;
  /** The queue figures (owned by the Board, which also reads them for the headline figure). */
  stats: Resource<QueueFigures | null>;
}

/** OCR queue at a glance, as one segment of the Board's pass strip; admins can pause and resume it. */
export function ProcessingPanel({ failed, stats }: ProcessingPanelProps) {
  const { t, i18n } = useTranslation();
  const { user } = useAuth();
  const toast = useToast();
  const canManage = isAdmin(user);
  const ocr = useResource(() => queueService.getOcrStatus().then((r) => r.data), POLL_MS);
  const [busy, setBusy] = useState(false);

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

  const n = (v?: number | null) => (v === undefined || v === null ? '—' : formatCount(v, i18n.language));
  // undefined: still loading; null: the queue figures are admin-only and this user cannot see them.
  const q = stats.data;
  const paused = ocr.data?.is_paused ?? false;
  const title = t('board.processing.title', 'Processing');

  return (
    <Region variant="segment" title={title}>
      {stats.error && q === undefined ? (
        <div className={styles.segmentBody}>
          <RegionError message={t('board.processing.error', 'The OCR queue could not be loaded.')} onRetry={stats.reload} />
        </div>
      ) : q === undefined ? (
        <div className={styles.segmentBody}>
          <Skeleton lines={1} label={t('board.loading', 'Loading')} />
        </div>
      ) : (
        <Figures aria-label={title}>
          <Figure lead label={t('board.processing.pending', 'Pending')}>{n(q?.pending)}</Figure>
          <Figure label={t('board.processing.processing', 'Processing')}>{n(q?.processing)}</Figure>
          <Figure label={t('board.processing.failed', 'Failed')}>{n(failed.data?.total)}</Figure>
          <Figure label={t('board.processing.doneToday', 'Done today')}>{n(q?.completedToday)}</Figure>
          <Figure label={t('board.processing.oldestWait', 'Oldest wait')}>{formatMinutes(q?.oldestPendingMinutes)}</Figure>
          {ocr.data ? (
            <Figure plain label={t('board.processing.ocr', 'OCR')}>
              <span className={styles.ocrState}>
                <StatusMark state={paused ? 'disabled' : 'healthy'} size="sm" />
                {canManage ? (
                  <Button size="sm" variant="secondary" isPending={busy} onPress={() => void toggle(!paused)}>
                    {paused ? t('board.processing.resume', 'Resume OCR') : t('board.processing.pause', 'Pause OCR')}
                  </Button>
                ) : null}
              </span>
            </Figure>
          ) : null}
        </Figures>
      )}
      {ocr.error && !ocr.data ? (
        <div className={styles.segmentBody}>
          <RegionError message={t('board.processing.statusError', 'The OCR state could not be loaded.')} onRetry={ocr.reload} />
        </div>
      ) : null}
    </Region>
  );
}
