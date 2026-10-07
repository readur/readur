import { useEffect, useId, useRef, useState, type Key } from 'react';
import { useTranslation } from 'react-i18next';
import { useSearchParams } from 'react-router-dom';
import { RetryHistoryModal } from '../../../components/RetryHistoryModal';
import { documentService } from '../../../services/api';
import { Button, EmptyState, Skeleton, SlideOver, Tab, TabList, TabPanel, Tabs, TruncatedText, useToast } from '../../../ui';
import { acknowledge } from '../../board/litStore';
import type { LabelData } from '../../labels';
import { CommentsPanel, useComments } from '../comments/CommentsPanel';
import { DocumentDetails } from '../details/DocumentDetails';
import { DeleteDocumentDialog, ProcessedImageDialog } from '../DocumentDialogs';
import { saveBlob } from '../download';
import { fileKind } from '../format';
import { useDocument } from '../hooks/useDocument';
import { useDocumentLabels } from '../hooks/useDocumentLabels';
import { useOcrText } from '../hooks/useOcrText';
import { useRetryHistory } from '../hooks/useRetryHistory';
import { useSourceName } from '../hooks/useSourceName';
import { DocumentViewer } from '../reading/DocumentViewer';
import { OcrTextPanel } from '../reading/OcrTextPanel';
import { SharedLinksManager } from '../sharing/SharedLinksManager';
import { notifyDocumentsChanged, useDocumentList, type DocumentListProvider } from './DocumentDrawerContext';
import { DrawerActions, DrawerHead } from './DrawerHead';
import { DrawerLabels } from './DrawerLabels';
import styles from './DocumentDrawer.module.css';

export type DocumentTab = 'text' | 'details' | 'comments' | 'links';

/** Stepping through records waits this long before loading a preview, so ↑/↓ stays light. */
export const PREVIEW_DELAY_MS = 200;

export interface DocumentDrawerProps {
  /** The document shown (kept while the drawer animates closed). */
  id: string;
  isOpen: boolean;
  onClose: () => void;
  /** Moves to another document without adding history (↑/↓). */
  onStep: (id: string) => void;
}

/**
 * Everything about one document in a drawer over the current page: status, facts and actions,
 * labels, the file itself, then its text, details, comments and share links in tabs.
 */
export function DocumentDrawer({ id, isOpen, onClose, onStep }: DocumentDrawerProps) {
  const { t } = useTranslation();
  const list = useDocumentList();
  const [tab, setTab] = useState<DocumentTab>('text');
  const [title, setTitle] = useState<string | null>(null);
  const previous = useRef<string | null>(null);
  const stepped = previous.current !== null && previous.current !== id;
  useEffect(() => {
    previous.current = isOpen ? id : null;
  }, [id, isOpen]);
  // A fresh opening starts on the text.
  useEffect(() => {
    if (isOpen) return;
    setTab('text');
  }, [isOpen]);

  const index = list ? list.ids.indexOf(id) : -1;
  const onNavigate =
    list && index >= 0
      ? (direction: 'previous' | 'next') => {
          const next = list.ids[index + (direction === 'next' ? 1 : -1)];
          if (next) onStep(next);
        }
      : undefined;

  const name = title ?? list?.peek?.(id)?.name ?? t('document.drawer.title', 'Document');

  return (
    <SlideOver
      title={<TruncatedText>{name}</TruncatedText>}
      isOpen={isOpen}
      onOpenChange={(open) => !open && onClose()}
      onNavigate={onNavigate}
      resizable
      storageKey="document"
      defaultShare={0.6}
      layout="fill"
    >
      <DocumentBody
        key={id}
        id={id}
        list={list}
        tab={tab}
        onTabChange={setTab}
        delayPreview={stepped}
        onTitle={setTitle}
        onClose={onClose}
      />
    </SlideOver>
  );
}

export default DocumentDrawer;

interface DocumentBodyProps {
  id: string;
  list: DocumentListProvider | null;
  tab: DocumentTab;
  onTabChange: (tab: DocumentTab) => void;
  delayPreview: boolean;
  onTitle: (title: string | null) => void;
  onClose: () => void;
}

function DocumentBody({ id, list, tab, onTabChange, delayPreview, onTitle, onClose }: DocumentBodyProps) {
  const { t } = useTranslation();
  const toast = useToast();
  const [params] = useSearchParams();
  const query = params.get('q') ?? '';

  const [awaitingRetry, setAwaitingRetry] = useState(false);
  const { document: doc, state, refresh, reload } = useDocument(id, awaitingRetry);
  const { ocr, state: ocrLoad } = useOcrText(doc);
  const labels = useDocumentLabels(id);
  const retry = useRetryHistory(id, `${doc?.ocr_status ?? ''}:${awaitingRetry}`);
  const sourceName = useSourceName(doc?.source_id);
  const comments = useComments(id, { poll: tab === 'comments' });

  const [retrying, setRetrying] = useState(false);
  const [downloading, setDownloading] = useState(false);
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [processedOpen, setProcessedOpen] = useState(false);
  const [historyOpen, setHistoryOpen] = useState(false);
  const [previewReady, setPreviewReady] = useState(!delayPreview);
  const detailsId = useId();

  useEffect(() => {
    acknowledge('document', id);
  }, [id]);

  useEffect(() => {
    onTitle(doc ? doc.original_filename : null);
  }, [doc, onTitle]);

  useEffect(() => {
    if (previewReady) return undefined;
    const timer = setTimeout(() => setPreviewReady(true), PREVIEW_DELAY_MS);
    return () => clearTimeout(timer);
  }, [previewReady]);

  // Tell the page's list when OCR moves on, so its row's status follows.
  const ocrStatus = doc?.ocr_status;
  const onChanged = list?.onChanged;
  useEffect(() => {
    if (ocrStatus !== undefined) onChanged?.(id, { ocr_status: ocrStatus });
  }, [id, ocrStatus, onChanged]);

  // Stop the post-retry polling once the job has left the failed state (or after a minute).
  useEffect(() => {
    if (!awaitingRetry) return undefined;
    if (ocrStatus && ocrStatus !== 'failed') {
      setAwaitingRetry(false);
      return undefined;
    }
    const timer = setTimeout(() => setAwaitingRetry(false), 60_000);
    return () => clearTimeout(timer);
  }, [awaitingRetry, ocrStatus]);

  if (state === 'loading') {
    return (
      <div className={styles.loading} role="status" aria-label={t('document.loading', 'Loading document')}>
        <Skeleton width="60%" height={16} />
        <Skeleton width="40%" height={28} />
        <Skeleton height={240} />
        <Skeleton lines={6} />
      </div>
    );
  }

  if (state === 'notFound' || state === 'error' || !doc) {
    const notFound = state !== 'error';
    return (
      <div className={styles.state}>
        <EmptyState
          headingAs="h3"
          title={notFound ? t('document.notFound', 'Document not found') : t('document.loadFailed', "Couldn't load this document")}
          description={
            notFound
              ? t('document.notFoundHint', 'It may have been deleted, or the link is wrong.')
              : t('document.loadFailedHint', 'Check your connection and try again.')
          }
          action={
            <div className={styles.stateActions}>
              {notFound ? null : <Button onPress={reload}>{t('document.retry', 'Try again')}</Button>}
              <Button variant={notFound ? 'primary' : 'secondary'} onPress={onClose}>
                {t('document.drawer.close', 'Close')}
              </Button>
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
      await documentService.retryOcr(doc.id);
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
      list?.onDeleted?.(doc.id);
      notifyDocumentsChanged({ deleted: [doc.id] });
      setDeleteOpen(false);
      onClose();
    } catch {
      toast.show({ title: t('document.toast.deleteFailed', "Couldn't delete the document"), tone: 'danger' });
      setDeleting(false);
      setDeleteOpen(false);
    }
  };

  const saveLabels = async (next: LabelData[]) => {
    const before = labels.labels;
    list?.onChanged?.(doc.id, { labels: next });
    if (await labels.save(next)) return;
    list?.onChanged?.(doc.id, { labels: before });
    toast.show({ title: t('document.toast.labelsFailed', "Couldn't save the labels"), tone: 'danger' });
  };

  const noPreview = fileKind(doc.mime_type) === 'other';

  return (
    <>
      <DrawerHead document={doc} ocr={ocr} sourceName={sourceName} />
      <DrawerLabels labels={labels} tags={doc.tags ?? []} onSave={(next) => void saveLabels(next)} />

      <section
        className={styles.preview}
        data-compact={noPreview || undefined}
        aria-label={t('document.drawer.preview', 'Preview')}
      >
        {previewReady ? (
          <DocumentViewer
            documentId={doc.id}
            filename={doc.original_filename}
            mimeType={doc.mime_type}
            openImageInNewTab
          />
        ) : (
          <Skeleton height="100%" label={t('document.viewer.loading', 'Loading preview')} />
        )}
      </section>

      <Tabs className={styles.tabs} selectedKey={tab} onSelectionChange={(key: Key) => onTabChange(key as DocumentTab)}>
        <TabList aria-label={t('document.drawer.tabs', 'About this document')} className={styles.tabList}>
          <Tab id="text">{t('document.tabs.text', 'Text')}</Tab>
          <Tab id="details">{t('document.tabs.details', 'Details')}</Tab>
          <Tab id="comments">
            {t('document.tabs.comments', 'Comments')}
            {comments.count > 0 ? (
              <>
                {' '}
                <span className={styles.count}>{comments.count}</span>
              </>
            ) : null}
          </Tab>
          <Tab id="links">{t('document.tabs.links', 'Share links')}</Tab>
        </TabList>
        <TabPanel id="text" className={styles.textPanel}>
          <OcrTextPanel
            document={doc}
            ocr={ocr}
            ocrState={ocrLoad}
            failure={doc.ocr_status === 'failed' ? retry.lastFailure : null}
            initialQuery={query}
          />
        </TabPanel>
        <TabPanel id="details" className={styles.panel}>
          <DocumentDetails
            id={detailsId}
            document={doc}
            ocr={ocr}
            retry={retry}
            onShowRetryHistory={() => setHistoryOpen(true)}
          />
        </TabPanel>
        <TabPanel id="comments" className={styles.panel}>
          <CommentsPanel documentId={doc.id} comments={comments} />
        </TabPanel>
        <TabPanel id="links" className={styles.panel}>
          <SharedLinksManager documentId={doc.id} />
        </TabPanel>
      </Tabs>

      <DrawerActions
        document={doc}
        isRetrying={retrying}
        isDownloading={downloading}
        onDownload={download}
        onShare={() => onTabChange('links')}
        onRetry={retryOcr}
        onDelete={() => setDeleteOpen(true)}
        onViewProcessed={() => setProcessedOpen(true)}
      />

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
    </>
  );
}
