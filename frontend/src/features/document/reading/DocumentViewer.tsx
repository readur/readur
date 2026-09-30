import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { documentService } from '../../../services/api';
import { Skeleton } from '../../../ui';
import { typeCodeOf } from '../../../lib/fileType';
import { fileKind } from '../format';
import styles from './DocumentViewer.module.css';

export interface DocumentViewerProps {
  documentId: string;
  filename: string;
  mimeType: string;
  /**
   * Loads the file as a blob. Defaults to the signed-in view endpoint; the public shared page
   * passes its own loader.
   */
  load?: () => Promise<{ data: BlobPart }>;
}

type ViewState = { status: 'loading' } | { status: 'error' } | { status: 'ready'; url: string; text?: string };

/** Inline preview of the original file: PDF in a frame, images, and plain text. */
export function DocumentViewer({ documentId, filename, mimeType, load }: DocumentViewerProps) {
  const { t } = useTranslation();
  const [view, setView] = useState<ViewState>({ status: 'loading' });
  const kind = fileKind(mimeType);

  useEffect(() => {
    let cancelled = false;
    let url: string | null = null;
    setView({ status: 'loading' });
    const fetchFile = load ?? (() => documentService.view(documentId));
    fetchFile()
      .then(async (res) => {
        const blob = new Blob([res.data], { type: mimeType });
        const text = kind === 'text' ? await blobText(blob) : undefined;
        if (cancelled) return;
        url = URL.createObjectURL(blob);
        setView({ status: 'ready', url, text });
      })
      .catch(() => {
        if (!cancelled) setView({ status: 'error' });
      });
    return () => {
      cancelled = true;
      if (url) URL.revokeObjectURL(url);
    };
    // `load` is expected to be stable per document; re-running on identity would refetch.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [documentId, mimeType, kind]);

  if (view.status === 'loading') {
    return (
      <div className={styles.frame}>
        <Skeleton height={480} label={t('document.viewer.loading', 'Loading preview')} />
      </div>
    );
  }
  if (view.status === 'error') {
    return (
      <div className={styles.message} role="alert">
        {t('document.viewer.error', "Couldn't load the preview.")}
      </div>
    );
  }
  if (kind === 'image') {
    return (
      <div className={styles.frame}>
        <img className={styles.image} src={view.url} alt={filename} />
      </div>
    );
  }
  if (kind === 'pdf') {
    return (
      <div className={styles.frame}>
        <iframe className={styles.pdf} src={view.url} title={filename} />
      </div>
    );
  }
  if (kind === 'text') {
    return (
      <div className={styles.frame}>
        <pre className={styles.text} tabIndex={0} aria-label={filename}>
          {view.text}
        </pre>
      </div>
    );
  }
  return (
    <div className={styles.message}>
      <p className={styles.messageTitle}>{t('document.viewer.unsupported', 'No preview for this file type')}</p>
      <p>
        {t('document.viewer.unsupportedHint', {
          type: typeCodeOf(mimeType, filename),
          defaultValue: '{{type}} files can’t be shown in the browser. Download the file to open it.',
        })}
      </p>
    </div>
  );
}

function blobText(blob: Blob): Promise<string> {
  if (typeof blob.text === 'function') return blob.text();
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result ?? ''));
    reader.onerror = () => reject(reader.error);
    reader.readAsText(blob);
  });
}

export default DocumentViewer;
