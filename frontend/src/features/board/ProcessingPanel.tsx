import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button, Pass, PassCell, Skeleton, StatusMark, useToast } from '../../ui';
import { useAuth } from '../../contexts/AuthContext';
import { queueService } from '../../services/api';
import { POLL_MS } from './data';
import { formatCount, formatMinutes } from './format';
import { Region, RegionError } from './Region';
import { useResource } from './useResource';
import styles from './Board.module.css';

/** OCR queue at a glance; admins can pause and resume it. */
export function ProcessingPanel() {
  const { t, i18n } = useTranslation();
  const { user } = useAuth();
  const toast = useToast();
  const isAdmin = user?.role === 'Admin';
  const stats = useResource(() => queueService.getStats().then((r) => r.data), POLL_MS);
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

  const n = (v?: number) => formatCount(v, i18n.language);
  const q = stats.data;
  const paused = ocr.data?.is_paused ?? false;

  return (
    <Region title={t('board.processing.title', 'Processing')}>
      {stats.error && !q ? (
        <RegionError message={t('board.processing.error', 'The OCR queue could not be loaded.')} onRetry={stats.reload} />
      ) : !q ? (
        <div className={styles.body}>
          <Skeleton lines={3} label={t('board.loading', 'Loading')} />
        </div>
      ) : (
        <Pass aria-label={t('board.processing.title', 'Processing')}>
          <PassCell label={t('board.processing.pending', 'Pending')} mono>{n(q.pending_count)}</PassCell>
          <PassCell label={t('board.processing.processing', 'Processing')} mono>{n(q.processing_count)}</PassCell>
          <PassCell label={t('board.processing.failed', 'Failed')} mono>{n(q.failed_count)}</PassCell>
          <PassCell label={t('board.processing.doneToday', 'Done today')} mono>{n(q.completed_today)}</PassCell>
          <PassCell label={t('board.processing.oldestWait', 'Oldest wait')} mono>{formatMinutes(q.oldest_pending_minutes)}</PassCell>
        </Pass>
      )}
      {ocr.error && !ocr.data ? (
        <RegionError message={t('board.processing.statusError', 'The OCR state could not be loaded.')} onRetry={ocr.reload} />
      ) : ocr.data ? (
        <div className={styles.processingHead}>
          <span className={styles.ocrState}>
            {t('board.processing.ocr', 'OCR')}
            <StatusMark state={paused ? 'disabled' : 'healthy'} size="sm" />
          </span>
          {isAdmin ? (
            <Button size="sm" variant="secondary" isPending={busy} onPress={() => void toggle(!paused)}>
              {paused ? t('board.processing.resume', 'Resume OCR') : t('board.processing.pause', 'Pause OCR')}
            </Button>
          ) : null}
        </div>
      ) : null}
    </Region>
  );
}
