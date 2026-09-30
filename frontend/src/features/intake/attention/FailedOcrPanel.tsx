import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  BoardTable,
  BulkActionBar,
  Button,
  EmptyState,
  Pagination,
  StatusMark,
  useToast,
  type BoardColumn,
  type Selection,
} from '../../../ui';
import { documentService, type BulkOcrRetryResponse, type FailedOcrDocumentRow } from '../../../services/api';
import { BulkRetryModal } from '../../../components/BulkRetryModal';
import { acknowledge, isShownLit, litReason, useLitCount } from '../../board/litStore';
import { serverMessage } from '../shared/errors';
import { HumanReason } from '../shared/HumanReason';
import { formatRelative } from '../shared/format';
import { ConfirmDialog, NameCell, Notice, sharedStyles } from '../shared/parts';
import { DOCUMENT_EVENTS_KEY, flagNewFailures } from '../shared/seenEvents';
import { useLoader } from '../shared/useLoader';
import { FailedDocumentPanel } from './FailedDocumentPanel';
import { attentionKeyOf, failedName, ocrFailureSummary } from './failureLabels';
import { ImportFailuresPanel } from './ImportFailuresPanel';
import { bulkDeleteResult, outcomeOf, toneOf } from './outcome';

export const FAILED_PAGE_SIZE = 25;

/**
 * Documents whose OCR failed (real documents, so every action works on them), then a
 * read-only list of other import failures.
 */
export function FailedOcrPanel() {
  const { t, i18n } = useTranslation();
  const toast = useToast();
  useLitCount('attention');
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(FAILED_PAGE_SIZE);
  const [selected, setSelected] = useState<Selection>(new Set());
  const [openId, setOpenId] = useState<string | null>(null);
  const [retryOpen, setRetryOpen] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [deleting, setDeleting] = useState(false);

  const failed = useLoader(
    async () => (await documentService.getFailedOcrDocuments(pageSize, (page - 1) * pageSize)).data,
    [page, pageSize],
  );
  const docs = failed.data?.documents ?? [];
  const total = failed.data?.pagination?.total ?? docs.length;
  const selectedDocs = selected === 'all' ? docs : docs.filter((d) => selected.has(d.id));
  const selectedIds = selectedDocs.map((d) => d.id);
  const open = docs.find((d) => d.id === openId) ?? null;

  useEffect(() => {
    flagNewFailures(
      DOCUMENT_EVENTS_KEY,
      'attention',
      docs.map((d) => ({
        id: attentionKeyOf(d),
        eventKey: attentionKeyOf(d),
        at: d.last_attempt_at || d.updated_at || d.created_at,
      })),
    );
  }, [docs]);

  const litOf = (d: FailedOcrDocumentRow) => isShownLit('attention', attentionKeyOf(d));
  const ackAll = (list: FailedOcrDocumentRow[]) => list.forEach((d) => acknowledge('attention', attentionKeyOf(d)));

  const columns: BoardColumn<FailedOcrDocumentRow>[] = [
    { id: 'name', label: t('intake.attention.col.name', 'Name'), render: (d) => <NameCell name={failedName(d)} tag={litOf(d) ? litReason('attention', attentionKeyOf(d)) : null} /> },
    { id: 'status', label: t('intake.attention.col.status', 'Status'), width: 110, render: () => <StatusMark state="failed" size="sm" /> },
    { id: 'reason', hideOnNarrow: true, label: t('intake.attention.col.reason', 'Reason'), width: 220, render: (d) => (d.ocr_error ? <HumanReason raw={d.ocr_error} code={d.ocr_failure_reason} summaryOnly /> : ocrFailureSummary(t, d)) },
    { id: 'retries', hideOnNarrow: true, label: t('intake.attention.col.retries', 'Retries'), align: 'end', width: 90, render: (d) => String(d.retry_count ?? 0) },
    { id: 'failed', hideOnNarrow: true, label: t('intake.attention.col.failed', 'Failed'), mono: true, width: 130, render: (d) => formatRelative(d.updated_at, i18n.language) },
  ];

  const onRetried = (result: BulkOcrRetryResponse) => {
    const outcome = outcomeOf(result.queued_count, result.matched_count);
    toast.show({
      title: t('intake.attention.retryQueued', '{{queued}} of {{matched}} documents queued', {
        queued: result.queued_count,
        matched: result.matched_count,
      }),
      description:
        outcome === 'all'
          ? undefined
          : t('intake.attention.retryPartial', 'Some documents were not queued. Check them and try again.'),
      tone: toneOf(outcome),
    });
    if (outcome === 'all') ackAll(selectedDocs);
    setSelected(new Set());
    void failed.reload();
  };

  const deleteSelected = async () => {
    setDeleting(true);
    try {
      const res = await documentService.bulkDelete(selectedIds);
      // Only documents the server confirms as deleted are cleared; the rest stay marked and selected.
      const { outcome, deleted, gone } = bulkDeleteResult(res.data, selectedIds);
      if (outcome === 'none') {
        toast.show({ title: t('intake.attention.deleteNone', 'No documents were deleted'), tone: 'danger' });
      } else {
        toast.show({
          title: t('intake.attention.deletedOf', '{{deleted}} of {{requested}} documents deleted', {
            deleted,
            requested: selectedIds.length,
          }),
          tone: toneOf(outcome),
        });
        ackAll(selectedDocs.filter((d) => gone.has(d.id)));
        setSelected(new Set(selectedIds.filter((id) => !gone.has(id))));
        setConfirmDelete(false);
      }
      await failed.reload();
    } catch (error) {
      toast.show({ title: t('intake.attention.deleteFailed', 'Could not delete the documents'), description: serverMessage(error), tone: 'danger' });
    } finally {
      setDeleting(false);
    }
  };

  return (
    <div className={sharedStyles.section}>
      <div className={sharedStyles.toolbar}>
        <p className={sharedStyles.lead}>
          {t('intake.attention.ocrLead', 'Documents whose text could not be read. Open one for details, or select several to retry or delete.')}
        </p>
        <div className={sharedStyles.toolbarEnd}>
          <Button onPress={() => void failed.reload()}>{t('intake.actions.refresh', 'Refresh')}</Button>
          <Button variant="primary" onPress={() => setRetryOpen(true)} isDisabled={total === 0}>
            {t('intake.attention.retryOptions', 'Retry…')}
          </Button>
        </div>
      </div>

      {failed.error && !failed.data ? (
        <Notice tone="danger" live="alert" title={t('intake.attention.loadFailed', 'Could not load failed documents.')}>
          <Button size="sm" onPress={() => void failed.reload()}>{t('intake.actions.retry', 'Retry')}</Button>
        </Notice>
      ) : (
        <BoardTable
          aria-label={t('intake.attention.failedLabel', 'Failed documents')}
          columns={columns}
          rows={docs}
          getRowId={(d) => d.id}
          selectionMode="multiple"
          selectedKeys={selected}
          onSelectionChange={setSelected}
          onRowAction={(id) => {
            const doc = docs.find((d) => d.id === id);
            if (doc) acknowledge('attention', attentionKeyOf(doc));
            setOpenId(id);
          }}
          isRowLit={litOf}
                    isLoading={failed.isLoading}
          emptyState={
            <EmptyState
              headingAs="h3"
              title={t('intake.attention.emptyTitle', 'No failed documents')}
              description={t('intake.attention.emptyBody', 'Every document was imported and read.')}
            />
          }
        />
      )}

      {total > pageSize ? (
        <Pagination page={page} pageSize={pageSize} total={total} onChange={(p, size) => { setPage(p); setPageSize(size); setSelected(new Set()); }} />
      ) : null}

      <ImportFailuresPanel />

      <BulkActionBar
        count={selectedIds.length}
        onClear={() => setSelected(new Set())}
        actions={[
          { id: 'retry', label: t('intake.attention.retry', 'Retry'), onPress: () => setRetryOpen(true) },
          { id: 'delete', label: t('intake.attention.delete', 'Delete'), tone: 'danger', onPress: () => setConfirmDelete(true) },
        ]}
      />

      <FailedDocumentPanel
        document={open}
        isOpen={Boolean(open)}
        onOpenChange={(isOpen) => !isOpen && setOpenId(null)}
        onChanged={() => void failed.reload()}
      />
      <BulkRetryModal open={retryOpen} onClose={() => setRetryOpen(false)} onSuccess={onRetried} selectedDocumentIds={selectedIds} />
      <ConfirmDialog
        isOpen={confirmDelete}
        onOpenChange={setConfirmDelete}
        title={t('intake.attention.deleteTitle', 'Delete {{count}} documents?', { count: selectedIds.length })}
        confirmLabel={t('intake.attention.deleteConfirm', 'Delete documents')}
        isPending={deleting}
        onConfirm={deleteSelected}
      >
        <p>{t('intake.attention.deleteBody', 'This cannot be undone. The documents and their files are deleted permanently.')}</p>
      </ConfirmDialog>
    </div>
  );
}
