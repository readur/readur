import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { documentService } from '../../services/api';
import { labelService } from '../../services/api/labels';
import { Button, Dialog, Pass, PassCell, Skeleton, SlideOver, StatusMark, useToast } from '../../ui';
import { Delete, Download, OpenInNew, Refresh, Share } from '../../ui/icons';
import { DocumentThumbnail } from '../document/DocumentThumbnail';
import { LabelSelector, toLabelData, type LabelData, type LabelDraft } from '../labels';
import { StatusCell } from './cells';
import { displayName, type LibraryRow } from './data';
import { formatBytes, formatDateTime, ocrState } from './format';
import { HighlightedText, matchRanges } from './Highlight';
import { shortType } from './mime';
import { useOcrExcerpt } from './useOcrExcerpt';
import styles from './Library.module.css';

interface DetailPanelProps {
  row: LibraryRow | null;
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  onNavigate: (direction: 'previous' | 'next') => void;
  /** The search text, highlighted in the OCR excerpt. */
  query: string;
  sourceName: (row: LibraryRow) => string;
  availableLabels: LabelData[];
  onLabelCreated: (label: LabelData) => void;
  onRowChange: (id: string, patch: Partial<LibraryRow>) => void;
  onDeleted: (id: string) => void;
}

/** Right-hand panel for one document: facts, preview, OCR excerpt, labels and actions. */
export function DetailPanel(props: DetailPanelProps) {
  const { row, isOpen, onOpenChange, onNavigate } = props;
  const { t } = useTranslation();
  return (
    <SlideOver
      isOpen={isOpen && row !== null}
      onOpenChange={onOpenChange}
      onNavigate={onNavigate}
      title={
        row ? (
          <span className={styles.panelTitle}>
            <span className={styles.panelName}>{displayName(row)}</span>
            <StatusCell row={row} />
          </span>
        ) : (
          t('library.detail.title', 'Document')
        )
      }
      footer={row ? <DetailActions {...props} row={row} /> : null}
    >
      {row ? <DetailBody {...props} row={row} /> : null}
    </SlideOver>
  );
}

function DetailBody({
  row,
  query,
  sourceName,
  availableLabels,
  onLabelCreated,
  onRowChange,
}: DetailPanelProps & { row: LibraryRow }) {
  const { t, i18n } = useTranslation();
  const toast = useToast();
  const ocr = useOcrExcerpt(row.id);
  const excerpt = ocr.status === 'ready' ? ocr.excerpt : null;
  const confidence = row.ocr_confidence ?? excerpt?.confidence ?? null;
  const progress =
    row.ocr_progress_total && row.ocr_progress_total > 0 ? `${row.ocr_progress_current ?? 0}/${row.ocr_progress_total}` : null;

  const saveLabels = async (next: LabelData[]) => {
    const previous = row.labels;
    onRowChange(row.id, { labels: next });
    try {
      await labelService.setDocumentLabels(row.id, next.map((l) => l.id));
      toast.show({ title: t('library.detail.labelsSaved', 'Labels saved'), tone: 'success' });
    } catch {
      onRowChange(row.id, { labels: previous });
      toast.show({ title: t('library.detail.labelsFailed', 'Could not save labels'), tone: 'danger' });
    }
  };

  const createLabel = async (draft: LabelDraft) => {
    const res = await labelService.create(draft);
    const created = toLabelData(res.data);
    onLabelCreated(created);
    return created;
  };

  return (
    <div className={styles.detail}>
      <Pass aria-label={t('library.detail.facts', 'Document facts')}>
        <PassCell label={t('library.columns.type', 'Type')} mono>
          {shortType(row.mime_type)}
        </PassCell>
        <PassCell label={t('library.detail.pagesOcr', 'Pages/OCR')} mono>
          {progress ?? (excerpt?.pages != null ? String(excerpt.pages) : '—')}
        </PassCell>
        <PassCell label={t('library.columns.size', 'Size')} mono>
          {formatBytes(row.file_size, i18n.language)}
        </PassCell>
        <PassCell label={t('library.columns.source', 'Source')}>{sourceName(row)}</PassCell>
        <PassCell label={t('library.columns.added', 'Added')} mono>
          {formatDateTime(row.created_at, i18n.language)}
        </PassCell>
        <PassCell label={t('library.columns.updated', 'Updated')} mono>
          {formatDateTime(row.updated_at, i18n.language)}
        </PassCell>
        {excerpt?.language ? (
          <PassCell label={t('library.detail.language', 'Language')}>{excerpt.language}</PassCell>
        ) : null}
        {confidence != null ? (
          <PassCell label={t('library.detail.confidence', 'Confidence')} mono>
            {`${Math.round(confidence)}%`}
          </PassCell>
        ) : null}
      </Pass>

      <div className={styles.preview}>
        <DocumentThumbnail key={row.id} documentId={row.id} mimeType={row.mime_type} size="large" />
      </div>

      <section className={styles.detailSection} aria-labelledby={`ocr-${row.id}`}>
        <h3 id={`ocr-${row.id}`} className={styles.sectionHeading}>
          {t('library.detail.text', 'Text')}
        </h3>
        {ocr.status === 'loading' ? (
          <Skeleton lines={4} label={t('library.detail.loadingText', 'Loading text')} />
        ) : excerpt?.text ? (
          <p className={styles.excerpt}>
            <HighlightedText text={excerpt.text} ranges={matchRanges(excerpt.text, query)} />
            {excerpt.truncated ? '…' : null}
          </p>
        ) : (
          <p className={styles.muted}>
            {ocr.status === 'error'
              ? t('library.detail.textFailed', 'The text could not be loaded.')
              : t('library.detail.noText', 'No text has been read from this document yet.')}
          </p>
        )}
      </section>

      <section className={styles.detailSection}>
        <LabelSelector
          label={t('library.columns.labels', 'Labels')}
          size="small"
          selectedLabels={row.labels}
          availableLabels={availableLabels}
          onLabelsChange={(next) => void saveLabels(next)}
          onCreateLabel={createLabel}
        />
      </section>
    </div>
  );
}

function DetailActions({ row, onRowChange, onDeleted }: DetailPanelProps & { row: LibraryRow }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const toast = useToast();
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [busy, setBusy] = useState<'download' | 'retry' | 'delete' | null>(null);
  const failed = ocrState(row.ocr_status) === 'failed';

  const download = async () => {
    setBusy('download');
    try {
      await documentService.downloadFile(row.id, displayName(row));
    } catch {
      toast.show({ title: t('library.detail.downloadFailed', 'Download failed'), tone: 'danger' });
    } finally {
      setBusy(null);
    }
  };

  const retry = async () => {
    setBusy('retry');
    try {
      await documentService.retryOcr(row.id);
      onRowChange(row.id, { ocr_status: 'pending' });
      toast.show({ title: t('library.detail.retryQueued', 'Text recognition queued again'), tone: 'success' });
    } catch {
      toast.show({ title: t('library.detail.retryFailed', 'Could not retry text recognition'), tone: 'danger' });
    } finally {
      setBusy(null);
    }
  };

  const remove = async () => {
    setBusy('delete');
    try {
      await documentService.delete(row.id);
      setConfirmDelete(false);
      toast.show({ title: t('library.detail.deleted', 'Document deleted'), tone: 'success' });
      onDeleted(row.id);
    } catch {
      toast.show({ title: t('library.detail.deleteFailed', 'Could not delete the document'), tone: 'danger' });
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className={styles.actions}>
      <Button variant="primary" icon={<OpenInNew fontSize="small" />} onPress={() => navigate(`/documents/${row.id}`)}>
        {t('library.detail.open', 'Open')}
      </Button>
      <Button icon={<Download fontSize="small" />} isPending={busy === 'download'} onPress={() => void download()}>
        {t('library.detail.download', 'Download')}
      </Button>
      {failed ? (
        <Button icon={<Refresh fontSize="small" />} isPending={busy === 'retry'} onPress={() => void retry()}>
          {t('library.detail.retry', 'Retry OCR')}
        </Button>
      ) : null}
      <Button variant="ghost" icon={<Share fontSize="small" />} onPress={() => navigate(`/documents/${row.id}?share=1`)}>
        {t('library.detail.share', 'Share')}
      </Button>
      <Button variant="danger" icon={<Delete fontSize="small" />} onPress={() => setConfirmDelete(true)}>
        {t('library.detail.delete', 'Delete')}
      </Button>
      <Dialog
        role="alertdialog"
        size="sm"
        isOpen={confirmDelete}
        onOpenChange={setConfirmDelete}
        title={t('library.detail.deleteTitle', 'Delete this document?')}
        actions={
          <>
            <Button variant="ghost" onPress={() => setConfirmDelete(false)}>
              {t('library.cancel', 'Cancel')}
            </Button>
            <Button variant="danger" isPending={busy === 'delete'} onPress={() => void remove()}>
              {t('library.detail.delete', 'Delete')}
            </Button>
          </>
        }
      >
        {t('library.detail.deleteBody', {
          name: displayName(row),
          defaultValue: '“{{name}}” and its text will be removed. This cannot be undone.',
        })}
      </Dialog>
    </div>
  );
}
