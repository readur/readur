import { useEffect, useId, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useLocation, useNavigate, useParams, useSearchParams } from 'react-router-dom';
import LabelSelector from '../labels/LabelSelector';
import type { LabelData } from '../labels/Label';
import { RetryHistoryModal } from '../../components/RetryHistoryModal';
import { documentService } from '../../services/api';
import { Button, EmptyState, useToast } from '../../ui';
import { acknowledge } from '../board/litStore';
import { DeleteDocumentDialog, ProcessedImageDialog } from './DocumentDialogs';
import { DocumentHeader } from './DocumentHeader';
import { DocumentToolbar } from './DocumentToolbar';
import { DocumentPageSkeleton } from './DocumentPageSkeleton';
import { DocumentDetails } from './details/DocumentDetails';
import { saveBlob } from './download';
import { fileKind } from './format';
import { useDocument } from './hooks/useDocument';
import { useDocumentLabels } from './hooks/useDocumentLabels';
import { useFillHeight } from './hooks/useFillHeight';
import { useOcrText } from './hooks/useOcrText';
import { useReadingView } from './hooks/useReadingView';
import { useRetryHistory } from './hooks/useRetryHistory';
import { useSourceName } from './hooks/useSourceName';
import { DocumentViewer } from './reading/DocumentViewer';
import { OcrTextPanel } from './reading/OcrTextPanel';
import { ReadingArea } from './reading/ReadingArea';
import { ViewSwitch } from './reading/ViewSwitch';
import { SharedLinksDialog } from './sharing/SharedLinksDialog';
import { SidePanel, type SidePanelTab } from './SidePanel';
import styles from './DocumentPage.module.css';

/** Where "Library" in the breadcrumb goes: back to the list the user came from, query intact. */
function useLibraryHref(query: string): string {
  const { state } = useLocation();
  const from = (state as { from?: unknown } | null)?.from;
  if (typeof from === 'string' && from.startsWith('/documents')) return from;
  return query ? `/documents?q=${encodeURIComponent(query)}` : '/documents';
}

/** Tells the sidebar and anything else listing labels that their counts moved. */
function announceLabelsChanged() {
  window.dispatchEvent(new CustomEvent('readur:labels-changed'));
}

/** /documents/:id: read one document, its text, or both side by side. */
export function DocumentPage() {
  const { t } = useTranslation();
  const { id } = useParams<{ id: string }>();
  const [params] = useSearchParams();
  const query = params.get('q') ?? '';
  const navigate = useNavigate();
  const toast = useToast();
  const libraryHref = useLibraryHref(query);

  const [awaitingRetry, setAwaitingRetry] = useState(false);
  const { document: doc, state, refresh, reload } = useDocument(id, awaitingRetry);
  const { ocr, state: ocrLoad } = useOcrText(doc);
  const labels = useDocumentLabels(id);
  const retry = useRetryHistory(id, `${doc?.ocr_status ?? ''}:${awaitingRetry}`);
  const sourceName = useSourceName(doc?.source_id);
  const reading = useReadingView({
    wantsText: query.trim() !== '',
    noPreview: doc ? fileKind(doc.mime_type) === 'other' && Boolean(doc.has_ocr_text) : false,
  });

  const [retrying, setRetrying] = useState(false);
  const [downloading, setDownloading] = useState(false);
  const [shareOpen, setShareOpen] = useState(false);
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [processedOpen, setProcessedOpen] = useState(false);
  const [historyOpen, setHistoryOpen] = useState(false);
  const [panelOpen, setPanelOpen] = useState(false);
  const [panelTab, setPanelTab] = useState<SidePanelTab>('comments');
  const [draftLabels, setDraftLabels] = useState<LabelData[] | null>(null);
  const [savingLabels, setSavingLabels] = useState(false);
  const [detailsOpen, setDetailsOpen] = useState(false);
  const detailsId = useId();
  const headRef = useRef<HTMLDivElement>(null);
  const areaRef = useRef<HTMLDivElement>(null);
  const fill = useFillHeight(areaRef, headRef, doc ? doc.id : null);

  useEffect(() => {
    if (id) acknowledge('document', id);
  }, [id]);

  // Stop the post-retry polling once the job has left the failed state (or after a minute).
  useEffect(() => {
    if (!awaitingRetry) return undefined;
    if (doc?.ocr_status && doc.ocr_status !== 'failed') {
      setAwaitingRetry(false);
      return undefined;
    }
    const timer = setTimeout(() => setAwaitingRetry(false), 60_000);
    return () => clearTimeout(timer);
  }, [awaitingRetry, doc?.ocr_status]);

  if (state === 'loading') return <DocumentPageSkeleton />;

  if (state === 'notFound' || state === 'error' || !doc) {
    const notFound = state !== 'error';
    return (
      <div className={styles.state}>
        <EmptyState
          headingAs="h1"
          title={notFound ? t('document.notFound', 'Document not found') : t('document.loadFailed', "Couldn't load this document")}
          description={
            notFound
              ? t('document.notFoundHint', 'It may have been deleted, or the link is wrong.')
              : t('document.loadFailedHint', 'Check your connection and try again.')
          }
          action={
            <div className={styles.stateActions}>
              {notFound ? null : <Button onPress={reload}>{t('document.retry', 'Try again')}</Button>}
              <Link className={styles.linkButton} to={libraryHref}>
                {t('document.backToLibrary', 'Back to Library')}
              </Link>
            </div>
          }
        />
      </div>
    );
  }

  const download = async () => {
    setDownloading(true);
    try {
      const res = await documentService.download(doc.id);
      saveBlob(res.data, doc.original_filename);
    } catch {
      toast.show({ title: t('document.toast.downloadFailed', "Couldn't download the file"), tone: 'danger' });
    } finally {
      setDownloading(false);
    }
  };

  const retryOcr = async () => {
    setRetrying(true);
    try {
      await documentService.bulkRetryOcr({ mode: 'specific', document_ids: [doc.id], priority_override: 15 });
      toast.show({ title: t('document.toast.retryQueued', 'OCR queued again'), tone: 'success' });
      setAwaitingRetry(true);
      await refresh();
    } catch {
      toast.show({ title: t('document.toast.retryFailed', "Couldn't queue OCR again"), tone: 'danger' });
    } finally {
      setRetrying(false);
    }
  };

  const deleteDocument = async () => {
    setDeleting(true);
    try {
      await documentService.delete(doc.id);
      toast.show({ title: t('document.toast.deleted', 'Document deleted'), tone: 'success' });
      navigate(libraryHref);
    } catch {
      toast.show({ title: t('document.toast.deleteFailed', "Couldn't delete the document"), tone: 'danger' });
      setDeleting(false);
      setDeleteOpen(false);
    }
  };

  const saveLabels = async () => {
    if (!draftLabels) return;
    setSavingLabels(true);
    try {
      await labels.save(draftLabels);
      setDraftLabels(null);
      announceLabelsChanged();
    } catch {
      toast.show({ title: t('document.toast.labelsFailed', "Couldn't save the labels"), tone: 'danger' });
    } finally {
      setSavingLabels(false);
    }
  };

  return (
    <div className={styles.page}>
      <div ref={headRef} className={styles.head}>
        <DocumentHeader
          document={doc}
          ocr={ocr}
          sourceName={sourceName}
          libraryHref={libraryHref}
          isRetrying={retrying}
          isDownloading={downloading}
          isCommentsOpen={panelOpen && panelTab === 'comments'}
          onDownload={download}
          onShare={() => setShareOpen(true)}
          onRetry={retryOcr}
          onDelete={() => setDeleteOpen(true)}
          onViewProcessed={() => setProcessedOpen(true)}
          onToggleComments={() => {
            setPanelTab('comments');
            setPanelOpen((open) => !(open && panelTab === 'comments'));
          }}
        />

        <DocumentToolbar
          labels={labels.labels}
          tags={doc.tags ?? []}
          isEditingLabels={draftLabels !== null}
          onEditLabels={() => setDraftLabels((d) => (d === null ? labels.labels : null))}
          detailsId={detailsId}
          isDetailsOpen={detailsOpen}
          onToggleDetails={() => setDetailsOpen((open) => !open)}
          viewSwitch={<ViewSwitch view={reading.view} canSplit={reading.canSplit} onChange={reading.setView} />}
        />

        {draftLabels !== null ? (
          <section className={styles.labelEditor} aria-label={t('document.labels.editor', 'Edit labels')}>
            <LabelSelector
              selectedLabels={draftLabels}
              availableLabels={labels.available}
              onLabelsChange={setDraftLabels}
              onCreateLabel={labels.create}
              placeholder={t('document.labels.placeholder', 'Search or create labels…')}
              size="small"
              disabled={labels.isLoading || savingLabels}
            />
            <div className={styles.labelEditorActions}>
              <Button variant="ghost" size="sm" isDisabled={savingLabels} onPress={() => setDraftLabels(null)}>
                {t('document.labels.cancel', 'Cancel')}
              </Button>
              <Button variant="primary" size="sm" isPending={savingLabels} onPress={saveLabels}>
                {t('document.labels.save', 'Save labels')}
              </Button>
            </div>
          </section>
        ) : null}

        {detailsOpen ? (
          <DocumentDetails
            id={detailsId}
            document={doc}
            ocr={ocr}
            retry={retry}
            onShowRetryHistory={() => setHistoryOpen(true)}
          />
        ) : null}
      </div>

      <ReadingArea
        ref={areaRef}
        view={reading.view}
        height={fill.height}
        pullUp={fill.pullUp}
        preview={<DocumentViewer documentId={doc.id} filename={doc.original_filename} mimeType={doc.mime_type} />}
        text={
          <OcrTextPanel
            document={doc}
            ocr={ocr}
            ocrState={ocrLoad}
            failure={doc.ocr_status === 'failed' ? retry.lastFailure : null}
            initialQuery={query}
          />
        }
      />

      <SidePanel documentId={doc.id} isOpen={panelOpen} onOpenChange={setPanelOpen} tab={panelTab} onTabChange={setPanelTab} />
      <SharedLinksDialog documentId={doc.id} filename={doc.original_filename} isOpen={shareOpen} onOpenChange={setShareOpen} />
      <DeleteDocumentDialog
        filename={doc.original_filename}
        isOpen={deleteOpen}
        isDeleting={deleting}
        onOpenChange={setDeleteOpen}
        onConfirm={deleteDocument}
      />
      <ProcessedImageDialog documentId={doc.id} isOpen={processedOpen} onOpenChange={setProcessedOpen} />
      <RetryHistoryModal
        open={historyOpen}
        onClose={() => setHistoryOpen(false)}
        documentId={doc.id}
        documentName={doc.original_filename}
      />
    </div>
  );
}

export default DocumentPage;
