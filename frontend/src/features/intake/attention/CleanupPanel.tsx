import { useId, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button, useToast } from '../../../ui';
import { useAuth } from '../../../contexts/AuthContext';
import { isAdmin } from '../../../auth/roles';
import { documentService, queueService } from '../../../services/api';
import { ConfirmDialog, Notice, sharedStyles } from '../shared/parts';
import type { CleanupResponse } from './cleanupTypes';
import styles from './Attention.module.css';

type Pending = 'preview' | 'deleteFailed' | 'retryAll' | 'requeue' | null;
type Confirm = 'deleteFailed' | 'retryAll' | 'requeue' | null;

const PREVIEW_IDS = 10;

/** Bulk actions for the whole library; each one asks for confirmation. */
export function CleanupPanel() {
  const { t } = useTranslation();
  const toast = useToast();
  const admin = isAdmin(useAuth().user);
  const ids = { failed: useId(), retry: useId(), requeue: useId() };
  const [pending, setPending] = useState<Pending>(null);
  const [confirm, setConfirm] = useState<Confirm>(null);
  const [failedPreview, setFailedPreview] = useState<CleanupResponse | null>(null);

  const previewFailed = async () => {
    setPending('preview');
    try {
      setFailedPreview((await documentService.deleteFailedOcr(true)).data as CleanupResponse);
    } catch {
      toast.show({ title: t('intake.cleanup.previewFailed', 'Could not preview failed documents'), tone: 'danger' });
    } finally {
      setPending(null);
    }
  };

  const deleteFailed = async () => {
    setPending('deleteFailed');
    try {
      const res = (await documentService.deleteFailedOcr(false)).data as CleanupResponse;
      toast.show({ title: res?.message || t('intake.cleanup.deleted', 'Failed documents deleted'), tone: 'success' });
      setFailedPreview(null);
      setConfirm(null);
    } catch {
      toast.show({ title: t('intake.cleanup.deleteFailed', 'Could not delete the failed documents'), tone: 'danger' });
    } finally {
      setPending(null);
    }
  };

  const retryAll = async () => {
    setPending('retryAll');
    try {
      const res = (await documentService.bulkRetryOcr({ mode: 'all', preview_only: false })).data;
      toast.show({
        title:
          res.queued_count > 0
            ? t('intake.cleanup.retryAllQueued', '{{count}} documents queued, about {{minutes}} min', {
                count: res.queued_count,
                minutes: Math.ceil(res.estimated_total_time_minutes),
              })
            : t('intake.cleanup.retryAllNone', 'No failed documents to retry'),
        tone: res.queued_count > 0 ? 'success' : 'info',
      });
      setConfirm(null);
    } catch {
      toast.show({ title: t('intake.cleanup.retryAllFailed', 'Could not retry the documents'), tone: 'danger' });
    } finally {
      setPending(null);
    }
  };

  const requeue = async () => {
    setPending('requeue');
    try {
      const res = await queueService.requeueFailed();
      const count = (res.data as { requeued_count?: number } | undefined)?.requeued_count ?? 0;
      toast.show({
        title: count > 0 ? t('intake.watch.requeued', '{{count}} jobs queued again', { count }) : t('intake.watch.nothingToRequeue', 'No failed jobs to retry'),
        tone: count > 0 ? 'success' : 'info',
      });
      setConfirm(null);
    } catch {
      toast.show({ title: t('intake.watch.requeueFailed', 'Could not retry the failed jobs'), tone: 'danger' });
    } finally {
      setPending(null);
    }
  };

  const failedIds = failedPreview?.document_ids ?? [];
  return (
    <div className={sharedStyles.section}>
      <section className={styles.action} aria-labelledby={ids.failed}>
        <h3 id={ids.failed} className={sharedStyles.heading}>{t('intake.cleanup.failedTitle', 'Delete documents whose OCR failed')}</h3>
        <p className={sharedStyles.lead}>
          {t('intake.cleanup.failedBody', 'Removes every document where OCR failed completely, including those with no confidence value. Preview first.')}
        </p>
        <div className={sharedStyles.toolbar}>
          <Button onPress={previewFailed} isPending={pending === 'preview'}>{t('intake.cleanup.preview', 'Preview')}</Button>
          <Button
            variant="danger"
            onPress={() => setConfirm('deleteFailed')}
            isDisabled={!failedPreview || failedPreview.matched_count === 0 || pending !== null}
          >
            {t('intake.cleanup.deleteFailedButton', 'Delete failed documents')}
          </Button>
        </div>
        {failedPreview ? (
          <Notice tone={failedPreview.matched_count > 0 ? 'info' : 'ok'} live="status" title={failedPreview.message}>
            {failedIds.length > 0 ? (
              <span className={sharedStyles.mono}>
                {failedIds.slice(0, PREVIEW_IDS).join(', ')}
                {failedIds.length > PREVIEW_IDS
                  ? ` ${t('intake.lowConfidence.more', 'and {{count}} more', { count: failedIds.length - PREVIEW_IDS })}`
                  : ''}
              </span>
            ) : null}
          </Notice>
        ) : null}
      </section>

      <section className={styles.action} aria-labelledby={ids.retry}>
        <h3 id={ids.retry} className={sharedStyles.heading}>{t('intake.cleanup.retryAllTitle', 'Retry OCR for all failed documents')}</h3>
        <p className={sharedStyles.lead}>
          {t('intake.cleanup.retryAllBody', 'Queues OCR again for every document whose OCR failed. Documents that were read successfully are not touched. With many failures this can take a long time.')}
        </p>
        <div>
          <Button onPress={() => setConfirm('retryAll')} isPending={pending === 'retryAll'}>
            {t('intake.cleanup.retryAllButton', 'Retry all failed documents')}
          </Button>
        </div>
      </section>

      {/* Requeueing the whole OCR queue is an admin-only endpoint. */}
      {admin ? (
        <section className={styles.action} aria-labelledby={ids.requeue}>
          <h3 id={ids.requeue} className={sharedStyles.heading}>{t('intake.cleanup.requeueTitle', 'Retry failed jobs only')}</h3>
          <p className={sharedStyles.lead}>{t('intake.watch.requeueBody', 'Every failed OCR job goes back into the queue.')}</p>
          <div>
            <Button onPress={() => setConfirm('requeue')} isPending={pending === 'requeue'}>
              {t('intake.cleanup.requeueButton', 'Retry failed jobs')}
            </Button>
          </div>
        </section>
      ) : null}

      <ConfirmDialog
        isOpen={confirm === 'deleteFailed'}
        onOpenChange={(open) => !open && setConfirm(null)}
        title={t('intake.cleanup.confirmDelete', 'Delete {{count}} documents whose OCR failed?', { count: failedPreview?.matched_count ?? 0 })}
        confirmLabel={t('intake.cleanup.deleteFailedButton', 'Delete failed documents')}
        isPending={pending === 'deleteFailed'}
        onConfirm={deleteFailed}
      >
        <p>{t('intake.attention.deleteBody', 'This cannot be undone. The documents and their files are deleted permanently.')}</p>
      </ConfirmDialog>
      <ConfirmDialog
        isOpen={confirm === 'retryAll'}
        onOpenChange={(open) => !open && setConfirm(null)}
        tone="primary"
        title={t('intake.cleanup.confirmRetryAll', 'Retry OCR for every failed document?')}
        confirmLabel={t('intake.cleanup.retryAllButton', 'Retry all failed documents')}
        isPending={pending === 'retryAll'}
        onConfirm={retryAll}
      >
        <p>{t('intake.cleanup.retryAllBody', 'Queues OCR again for every document whose OCR failed. Documents that were read successfully are not touched. With many failures this can take a long time.')}</p>
      </ConfirmDialog>
      <ConfirmDialog
        isOpen={confirm === 'requeue'}
        onOpenChange={(open) => !open && setConfirm(null)}
        tone="primary"
        title={t('intake.watch.requeueTitle', 'Retry failed jobs?')}
        confirmLabel={t('intake.watch.requeueConfirm', 'Retry jobs')}
        isPending={pending === 'requeue'}
        onConfirm={requeue}
      >
        <p>{t('intake.watch.requeueBody', 'Every failed OCR job goes back into the queue.')}</p>
      </ConfirmDialog>
    </div>
  );
}
