import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  BoardTable,
  BulkActionBar,
  Button,
  EmptyState,
  Pagination,
  Select,
  SelectItem,
  StatusMark,
  useToast,
  type BoardColumn,
  type Selection,
} from '../../../ui';
import { documentService, ocrService, type BulkOcrRetryResponse, type FailedDocumentRow } from '../../../services/api';
import { BulkRetryModal } from '../../../components/BulkRetryModal';
import { acknowledge, isLit, useLitCount } from '../../board/litStore';
import { serverMessage } from '../shared/errors';
import { formatRelative } from '../shared/format';
import { ConfirmDialog, NameCell, Notice, sharedStyles } from '../shared/parts';
import { attentionIdOfDocument, DOCUMENT_EVENTS_KEY, flagNewFailures } from '../shared/seenEvents';
import { useLoader } from '../shared/useLoader';
import { FailedDocumentPanel } from './FailedDocumentPanel';
import { FAILURE_REASONS, FAILURE_STAGES, failedName, failureSummary, reasonLabel, stageLabel } from './failureLabels';

export const FAILED_PAGE_SIZE = 25;
const ALL = 'all';

/** Documents whose import or OCR failed. Each row opens its details; selected rows can be retried. */
export function FailedOcrPanel() {
  const { t, i18n } = useTranslation();
  const toast = useToast();
  useLitCount('attention');
  const [stage, setStage] = useState(ALL);
  const [reason, setReason] = useState(ALL);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(FAILED_PAGE_SIZE);
  const [selected, setSelected] = useState<Selection>(new Set());
  const [openId, setOpenId] = useState<string | null>(null);
  const [retryOpen, setRetryOpen] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [deleting, setDeleting] = useState(false);

  const failed = useLoader(
    async () =>
      (
        await ocrService.listFailedDocuments({
          limit: pageSize,
          offset: (page - 1) * pageSize,
          stage: stage === ALL ? undefined : stage,
          reason: reason === ALL ? undefined : reason,
        })
      ).data,
    [page, pageSize, stage, reason],
  );
  const docs = failed.data?.documents ?? [];
  const total = failed.data?.pagination?.total ?? docs.length;
  const selectedIds = selected === 'all' ? docs.map((d) => d.id) : docs.filter((d) => selected.has(d.id)).map((d) => d.id);
  const open = docs.find((d) => d.id === openId) ?? null;

  useEffect(() => {
    flagNewFailures(
      DOCUMENT_EVENTS_KEY,
      'attention',
      docs.map((d) => ({ id: attentionIdOfDocument(d.id), eventKey: `${d.id}@${d.updated_at}` })),
    );
  }, [docs]);

  const litOf = (d: FailedDocumentRow) => isLit('attention', attentionIdOfDocument(d.id));

  const columns: BoardColumn<FailedDocumentRow>[] = [
    { id: 'name', label: t('intake.attention.col.name', 'Name'), render: (d) => <NameCell name={failedName(d)} tag={litOf(d) ? 'changed' : null} /> },
    { id: 'status', label: t('intake.attention.col.status', 'Status'), width: 110, render: () => <StatusMark state="failed" size="sm" /> },
    { id: 'reason', label: t('intake.attention.col.reason', 'Reason'), width: 200, render: (d) => failureSummary(t, d) },
    { id: 'stage', label: t('intake.attention.col.stage', 'Stage'), width: 110, render: (d) => stageLabel(t, d.failure_stage) },
    { id: 'retries', label: t('intake.attention.col.retries', 'Retries'), align: 'end', width: 90, render: (d) => String(d.retry_count ?? 0) },
    { id: 'failed', label: t('intake.attention.col.failed', 'Failed'), mono: true, width: 130, render: (d) => formatRelative(d.updated_at, i18n.language) },
  ];

  const onRetried = (result: BulkOcrRetryResponse) => {
    toast.show({
      title: t('intake.attention.retryQueued', '{{queued}} of {{matched}} documents queued', {
        queued: result.queued_count,
        matched: result.matched_count,
      }),
      description: t('intake.attention.retryEta', 'About {{minutes}} min', { minutes: Math.round(result.estimated_total_time_minutes) }),
      tone: 'success',
    });
    selectedIds.forEach((id) => acknowledge('attention', attentionIdOfDocument(id)));
    setSelected(new Set());
    void failed.reload();
  };

  const deleteSelected = async () => {
    setDeleting(true);
    try {
      await documentService.bulkDelete(selectedIds);
      selectedIds.forEach((id) => acknowledge('attention', attentionIdOfDocument(id)));
      toast.show({ title: t('intake.attention.deleted', '{{count}} documents deleted', { count: selectedIds.length }), tone: 'success' });
      setSelected(new Set());
      setConfirmDelete(false);
      await failed.reload();
    } catch (error) {
      toast.show({ title: t('intake.attention.deleteFailed', 'Could not delete the documents'), description: serverMessage(error), tone: 'danger' });
    } finally {
      setDeleting(false);
    }
  };

  const resetPage = () => {
    setPage(1);
    setSelected(new Set());
  };

  return (
    <div className={sharedStyles.section}>
      <div className={sharedStyles.toolbar}>
        <Select label={t('intake.attention.stageFilter', 'Stage')} selectedKey={stage} onSelectionChange={(k) => { setStage(String(k)); resetPage(); }}>
          <SelectItem id={ALL}>{t('intake.attention.allStages', 'All stages')}</SelectItem>
          {FAILURE_STAGES.map((s) => (
            <SelectItem key={s} id={s}>{stageLabel(t, s)}</SelectItem>
          ))}
        </Select>
        <Select label={t('intake.attention.reasonFilter', 'Reason')} selectedKey={reason} onSelectionChange={(k) => { setReason(String(k)); resetPage(); }}>
          <SelectItem id={ALL}>{t('intake.attention.allReasons', 'All reasons')}</SelectItem>
          {FAILURE_REASONS.map((r) => (
            <SelectItem key={r} id={r}>{reasonLabel(t, r)}</SelectItem>
          ))}
        </Select>
        <Button
          variant="ghost"
          isDisabled={stage === ALL && reason === ALL}
          onPress={() => {
            setStage(ALL);
            setReason(ALL);
            resetPage();
          }}
        >
          {t('intake.attention.clearFilters', 'Clear filters')}
        </Button>
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
            acknowledge('attention', attentionIdOfDocument(id));
            setOpenId(id);
          }}
          isRowLit={litOf}
          renderRowDetail={(d) => (d.error_message ? d.error_message : null)}
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
