import { useCallback, useEffect, useMemo, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { BoardTable, Button, StatusMark, useToast, type BoardColumn } from '../../ui';
import { documentService, sourcesService } from '../../services/api';
import { loadDismissed, saveDismissed } from './dismissed';
import { formatAge, humanizeReason } from './format';
import { DOCUMENT_EVENTS_KEY, flagNewFailures } from '../intake/shared/seenEvents';
import type { FailedOcrPage } from './data';
import { acknowledge, isShownLit, useAcknowledgeOnLeave, useLitCount, useShownLit } from './litStore';
import { ChangedTag, Region, RegionError } from './Region';
import { docName, type AttentionItem, type BoardSource, type FailedOcrDocument } from './types';
import type { Resource } from './useResource';
import styles from './Board.module.css';

export interface AttentionStripProps {
  failed: Resource<FailedOcrPage>;
  sources: Resource<BoardSource[]>;
}

/** Dismissals are per failure occurrence: a newer failure of the same item gets a new key. */
const occurrenceKey = (base: string, occurrence?: string | null) => (occurrence ? `${base}@${occurrence}` : base);

export function buildAttentionItems(failed: FailedOcrDocument[], sources: BoardSource[]): AttentionItem[] {
  const docs = failed.map<AttentionItem>((d) => ({
    key: occurrenceKey(`document:${d.id}`, d.last_retry_at || d.updated_at || d.created_at),
    kind: 'document',
    id: d.id,
    name: docName(d),
    reason: d.error_message || humanizeReason(d.failure_reason),
    at: d.created_at,
    state: 'failed',
  }));
  const bad = sources
    .filter((s) => s.enabled !== false && s.status === 'error')
    .map<AttentionItem>((s) => ({
      key: occurrenceKey(`source:${s.id}`, s.last_error_at),
      kind: 'source',
      id: s.id,
      name: s.name,
      reason: s.last_error || '',
      at: s.last_error_at ?? undefined,
      state: 'error',
    }));
  return [...bad, ...docs];
}

function AttentionName({ item }: { item: AttentionItem }) {
  const { lit, reason } = useShownLit('attention', item.key);
  return (
    <span className={styles.name}>
      {lit ? <ChangedTag reason={reason} /> : null}
      <span className={styles.nameText}>{item.name}</span>
    </span>
  );
}

/** Failed OCR documents and connections in error. Renders nothing when there is nothing to do. */
export function AttentionStrip({ failed, sources }: AttentionStripProps) {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const toast = useToast();
  const [dismissed, setDismissed] = useState<string[]>(loadDismissed);

  const items = useMemo(
    () => buildAttentionItems(failed.data?.documents ?? [], sources.data ?? []).filter((i) => !dismissed.includes(i.key)),
    [failed.data, sources.data, dismissed],
  );

  useLitCount('attention'); // re-render when a row is marked or acknowledged
  // Each failure occurrence is flagged once, in the same event log Intake › Needs attention uses,
  // so it is lit on both surfaces until seen on either, and never re-lit after that.
  useEffect(() => {
    flagNewFailures(
      DOCUMENT_EVENTS_KEY,
      'attention',
      items.map((i) => ({ id: i.key, eventKey: i.key })),
    );
  }, [items]);
  // Rows seen lit during this visit are acknowledged when the user leaves the Board.
  useAcknowledgeOnLeave(
    'attention',
    items.map((i) => i.key),
  );

  const dismiss = useCallback((key: string) => {
    acknowledge('attention', key);
    setDismissed((prev) => saveDismissed(prev.includes(key) ? prev : [...prev, key]));
  }, []);

  const retry = useCallback(
    async (item: AttentionItem) => {
      try {
        if (item.kind === 'document') {
          const res = await documentService.retryOcr(item.id);
          const body = res?.data as { success?: boolean; message?: string } | undefined;
          // The API answers 200 with success:false when OCR is already running for the document.
          if (body?.success === false) {
            toast.show({
              title: t('board.attention.retryFailed', 'Could not retry'),
              description: body.message || item.name,
              tone: 'danger',
            });
            return;
          }
          failed.reload();
        } else {
          await sourcesService.triggerSync(item.id);
          sources.reload();
        }
        // No dismissal: a repeat failure is a new occurrence and shows up again by itself.
        toast.show({ title: t('board.attention.retryQueued', 'Retry started'), description: item.name, tone: 'success' });
      } catch {
        toast.show({ title: t('board.attention.retryFailed', 'Could not retry'), description: item.name, tone: 'danger' });
      }
    },
    [failed, sources, toast, t],
  );

  const open = useCallback(
    (id: string) => {
      const item = items.find((i) => i.key === id);
      navigate(item?.kind === 'source' ? '/intake?section=connections' : '/intake?section=attention');
    },
    [items, navigate],
  );

  const columns = useMemo<BoardColumn<AttentionItem>[]>(
    () => [
      {
        id: 'name',
        label: t('board.col.name', 'Name'),
        isRowHeader: true,
        render: (i) => <AttentionName item={i} />,
      },
      {
        id: 'status',
        hideOnNarrow: true,
        label: t('board.col.status', 'Status'),
        width: 110,
        render: (i) => <StatusMark state={i.state} size="sm" />,
      },
      {
        id: 'reason',
        hideOnNarrow: true,
        label: t('board.col.reason', 'Reason'),
        render: (i) => <span className={styles.reason}>{i.reason || '—'}</span>,
      },
      { id: 'age', hideOnNarrow: true, label: t('board.col.age', 'Age'), width: 80, align: 'end', mono: true, render: (i) => formatAge(i.at, i18n.language) },
      {
        id: 'actions',
        label: <span className={styles.srOnly}>{t('board.col.actions', 'Actions')}</span>,
        textValue: t('board.col.actions', 'Actions'),
        width: 170,
        align: 'end',
        render: (i) => (
          <span className={styles.rowActions}>
            <Button size="sm" variant="secondary" aria-label={t('board.attention.retryNamed', 'Retry {{name}}', { name: i.name })} onPress={() => void retry(i)}>
              {t('board.retry', 'Retry')}
            </Button>
            <Button size="sm" variant="ghost" aria-label={t('board.attention.dismissNamed', 'Dismiss {{name}}', { name: i.name })} onPress={() => dismiss(i.key)}>
              {t('board.attention.dismiss', 'Dismiss')}
            </Button>
          </span>
        ),
      },
    ],
    [t, i18n.language, retry, dismiss],
  );

  const failedError = Boolean(failed.error) || Boolean(sources.error);
  if (items.length === 0 && !failedError) return null;

  return (
    <Region
      className={styles.attention}
      title={t('board.attention.title', 'Needs attention')}
      headerAction={
        <Link className={styles.link} to="/intake?section=attention">
          {t('board.viewAll', 'View all')}
        </Link>
      }
    >
      {failedError ? (
        <RegionError
          message={t('board.attention.error', 'Some items that need attention could not be loaded.')}
          onRetry={() => {
            if (failed.error) failed.reload();
            if (sources.error) sources.reload();
          }}
        />
      ) : null}
      {items.length > 0 ? (
        <BoardTable
          aria-label={t('board.attention.title', 'Needs attention')}
          density="compact"
          columns={columns}
          rows={items}
          getRowId={(i) => i.key}
          isRowLit={(i) => isShownLit('attention', i.key)}
          onRowAction={open}
        />
      ) : null}
    </Region>
  );
}
