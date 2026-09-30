import { useId, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { BoardTable, Button, Pagination, Select, SelectItem, StatusMark, type BoardColumn } from '../../../ui';
import { ocrService, type FailedDocumentRow } from '../../../services/api';
import { formatRelative } from '../shared/format';
import { Notice, sharedStyles } from '../shared/parts';
import { useLoader } from '../shared/useLoader';
import { FAILURE_REASONS, FAILURE_STAGES, failedName, failureSummary, reasonLabel, stageLabel } from './failureLabels';
import { ImportFailureDetail } from './ImportFailureDetail';

export const IMPORT_FAILURES_PAGE_SIZE = 25;
const ALL = 'all';
/** OCR failures are listed above with their actions; this list covers the other stages. */
const STAGES = FAILURE_STAGES.filter((s) => s !== 'ocr');

/**
 * Failed import records (GET /documents/failed) for stages other than OCR. These are records,
 * not documents, so the list is read-only: details can be opened, nothing can be retried or
 * deleted from here.
 */
export function ImportFailuresPanel() {
  const { t, i18n } = useTranslation();
  const headingId = useId();
  const [stage, setStage] = useState(ALL);
  const [reason, setReason] = useState(ALL);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(IMPORT_FAILURES_PAGE_SIZE);
  const [openId, setOpenId] = useState<string | null>(null);

  const list = useLoader(
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
  const rows = (list.data?.documents ?? []).filter((d) => d.failure_stage !== 'ocr');
  const total = list.data?.pagination?.total ?? rows.length;
  const open = rows.find((d) => d.id === openId) ?? null;

  const columns: BoardColumn<FailedDocumentRow>[] = [
    { id: 'name', label: t('intake.attention.col.name', 'Name'), render: (d) => failedName(d) },
    { id: 'status', label: t('intake.attention.col.status', 'Status'), width: 110, render: () => <StatusMark state="failed" size="sm" /> },
    { id: 'reason', label: t('intake.attention.col.reason', 'Reason'), width: 200, render: (d) => failureSummary(t, d) },
    { id: 'stage', label: t('intake.attention.col.stage', 'Stage'), width: 120, render: (d) => stageLabel(t, d.failure_stage) },
    { id: 'failed', label: t('intake.attention.col.failed', 'Failed'), mono: true, width: 130, render: (d) => formatRelative(d.updated_at, i18n.language) },
  ];

  const reset = () => setPage(1);

  return (
    <section className={sharedStyles.stack} aria-labelledby={headingId}>
      <h3 id={headingId} className={sharedStyles.heading}>{t('intake.attention.importTitle', 'Other import failures')}</h3>
      <p className={sharedStyles.lead}>
        {t('intake.attention.importLead', 'Files that failed before or outside OCR. They are records only: open one to see why.')}
      </p>
      <div className={sharedStyles.toolbar}>
        <Select label={t('intake.attention.stageFilter', 'Stage')} selectedKey={stage} onSelectionChange={(k) => { setStage(String(k)); reset(); }}>
          <SelectItem id={ALL}>{t('intake.attention.allStages', 'All stages')}</SelectItem>
          {STAGES.map((s) => (
            <SelectItem key={s} id={s}>{stageLabel(t, s)}</SelectItem>
          ))}
        </Select>
        <Select label={t('intake.attention.reasonFilter', 'Reason')} selectedKey={reason} onSelectionChange={(k) => { setReason(String(k)); reset(); }}>
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
            reset();
          }}
        >
          {t('intake.attention.clearFilters', 'Clear filters')}
        </Button>
      </div>
      {list.error && !list.data ? (
        <Notice tone="danger" live="alert" title={t('intake.attention.importLoadFailed', 'Could not load import failures.')}>
          <Button size="sm" onPress={() => void list.reload()}>{t('intake.actions.retry', 'Retry')}</Button>
        </Notice>
      ) : (
        <BoardTable
          aria-labelledby={headingId}
          columns={columns}
          rows={rows}
          getRowId={(d) => d.id}
          onRowAction={setOpenId}
          renderRowDetail={(d) => d.error_message || null}
          isLoading={list.isLoading}
          density="compact"
          emptyState={<p className={sharedStyles.meta}>{t('intake.attention.importEmpty', 'No other import failures.')}</p>}
        />
      )}
      {total > pageSize ? (
        <Pagination page={page} pageSize={pageSize} total={total} onChange={(p, size) => { setPage(p); setPageSize(size); }} />
      ) : null}
      <ImportFailureDetail record={open} isOpen={Boolean(open)} onOpenChange={(o) => !o && setOpenId(null)} />
    </section>
  );
}
