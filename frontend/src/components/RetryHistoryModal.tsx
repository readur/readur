import React, { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { BoardTable, Button, Dialog, Skeleton, type BoardColumn } from '../ui';
import { documentService, type DocumentRetryHistoryItem } from '../services/api';
import styles from './RetryModals.module.css';

interface RetryHistoryModalProps {
  open: boolean;
  onClose: () => void;
  documentId: string;
  documentName?: string;
}

const RETRY_REASON_KEYS: Record<string, [string, string]> = {
  manual_retry: ['intake.retry.reason.manual', 'Manual retry'],
  bulk_retry_all: ['intake.retry.reason.bulkAll', 'Bulk retry (all)'],
  bulk_retry_specific: ['intake.retry.reason.bulkSelected', 'Bulk retry (selected)'],
  bulk_retry_filtered: ['intake.retry.reason.bulkFiltered', 'Bulk retry (filtered)'],
  scheduled_retry: ['intake.retry.reason.scheduled', 'Scheduled retry'],
  auto_retry: ['intake.retry.reason.auto', 'Automatic retry'],
};

export function priorityLevel(priority: number): 'veryHigh' | 'high' | 'medium' | 'low' | 'veryLow' {
  if (priority >= 15) return 'veryHigh';
  if (priority >= 12) return 'high';
  if (priority >= 8) return 'medium';
  if (priority >= 5) return 'low';
  return 'veryLow';
}

function useRetryHistory(documentId: string, enabled: boolean) {
  const [history, setHistory] = useState<DocumentRetryHistoryItem[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!documentId) return;
    setLoading(true);
    setError(null);
    try {
      const response = await documentService.getDocumentRetryHistory(documentId);
      setHistory(response.data?.retry_history || []);
      setTotal(response.data?.total_retries || 0);
    } catch (err: unknown) {
      const message = (err as { response?: { data?: { message?: string } } })?.response?.data?.message;
      setError(message || 'failed');
      setHistory([]);
      setTotal(0);
    } finally {
      setLoading(false);
    }
  }, [documentId]);

  useEffect(() => {
    if (enabled && documentId) void load();
  }, [enabled, documentId, load]);

  return { history, total, loading, error, load };
}

/** Retry attempts of one document, newest first. Used in the modal and in the failure details panel. */
export const RetryHistoryList: React.FC<{ documentId: string; enabled?: boolean }> = ({ documentId, enabled = true }) => {
  const { t, i18n } = useTranslation();
  const { history, total, loading, error, load } = useRetryHistory(documentId, enabled);
  const dateFormat = new Intl.DateTimeFormat(i18n.language, { dateStyle: 'medium', timeStyle: 'short' });

  const priorityWord = (priority: number) => {
    switch (priorityLevel(priority)) {
      case 'veryHigh':
        return t('intake.retry.priority.veryHigh', 'Very high');
      case 'high':
        return t('intake.retry.priority.high', 'High');
      case 'medium':
        return t('intake.retry.priority.medium', 'Medium');
      case 'low':
        return t('intake.retry.priority.low', 'Low');
      default:
        return t('intake.retry.priority.veryLow', 'Very low');
    }
  };

  const columns: BoardColumn<DocumentRetryHistoryItem>[] = [
    { id: 'when', label: t('intake.retry.col.when', 'When'), mono: true, width: 170, render: (i) => dateFormat.format(new Date(i.created_at)) },
    {
      id: 'reason',
      label: t('intake.retry.col.reason', 'Reason'),
      render: (i) => {
        const known = RETRY_REASON_KEYS[i.retry_reason];
        return known ? t(known[0], known[1]) : i.retry_reason.replace(/_/g, ' ');
      },
    },
    {
      id: 'previous',
      label: t('intake.retry.col.previous', 'Before'),
      render: (i) =>
        [i.previous_status, i.previous_failure_reason?.replace(/_/g, ' ')].filter(Boolean).join(' · ') || '—',
    },
    {
      id: 'priority',
      label: t('intake.retry.col.priority', 'Priority'),
      width: 130,
      render: (i) => `${priorityWord(i.priority)} (${i.priority})`,
    },
    {
      id: 'queue',
      label: t('intake.retry.col.queue', 'Queue'),
      width: 150,
      render: (i) =>
        i.queue_id
          ? t('intake.retry.queued', 'Queued · {{id}}', { id: i.queue_id.slice(0, 8) })
          : t('intake.retry.notQueued', 'Not queued'),
    },
  ];

  if (loading && history.length === 0) {
    return <Skeleton lines={3} label={t('intake.retry.loading', 'Loading retry history…')} />;
  }
  return (
    <div className={styles.stack}>
      {error ? (
        <p className={styles.error} role="alert">
          {error === 'failed' ? t('intake.retry.loadFailed', 'Could not load the retry history.') : error}
        </p>
      ) : null}
      {!error && history.length === 0 ? (
        <p className={styles.muted}>
          {t('intake.retry.empty', 'No retry attempts found for this document.')}
        </p>
      ) : null}
      {history.length > 0 ? (
        <>
          <p className={styles.muted}>
            {t('intake.retry.summary', '{{count}} retry attempts', { count: total })}
          </p>
          <BoardTable
            aria-label={t('intake.retry.title', 'OCR retry history')}
            columns={columns}
            rows={history}
            getRowId={(i) => i.id}
            density="compact"
            renderRowDetail={(i) => (i.previous_error ? i.previous_error : null)}
          />
          <p className={styles.muted}>
            {t(
              'intake.retry.legend',
              'Priority: very high 15–20, high 12–14, medium 8–11, low 5–7, very low 1–4.',
            )}
          </p>
        </>
      ) : null}
      <div>
        <Button size="sm" variant="ghost" onPress={() => void load()} isPending={loading}>
          {t('intake.actions.refresh', 'Refresh')}
        </Button>
      </div>
    </div>
  );
};

export const RetryHistoryModal: React.FC<RetryHistoryModalProps> = ({ open, onClose, documentId, documentName }) => {
  const { t } = useTranslation();
  return (
    <Dialog
      isOpen={open}
      onOpenChange={(isOpen) => !isOpen && onClose()}
      size="lg"
      title={t('intake.retry.title', 'OCR retry history')}
      actions={
        <Button variant="primary" onPress={onClose}>
          {t('intake.actions.close', 'Close')}
        </Button>
      }
    >
      <div className={styles.stack}>
        {documentName ? <p className={styles.subject}>{documentName}</p> : null}
        <RetryHistoryList documentId={documentId} enabled={open} />
      </div>
    </Dialog>
  );
};
