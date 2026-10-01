import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Skeleton } from '../../../ui';
import { previewSandbox } from '../../../services/contentSafety';
import { statusOf } from '../shared/errors';
import { Notice } from '../shared/parts';
import styles from './Attention.module.css';

export interface FailedDocumentPreviewProps {
  id: string;
  filename: string;
  mimeType: string;
  /** Fetches the file as a blob: the document view for documents, the record view for failed imports. */
  load: (id: string) => Promise<{ data: Blob }>;
}

/** The stored file of a failed document: images and PDFs inline, text in a frame, others named. */
export function FailedDocumentPreview({ id, filename, mimeType, load }: FailedDocumentPreviewProps) {
  const { t } = useTranslation();
  const [url, setUrl] = useState<string | null>(null);
  const [error, setError] = useState<'missing' | 'failed' | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let alive = true;
    let objectUrl: string | null = null;
    setLoading(true);
    setError(null);
    setUrl(null);
    load(id)
      .then((res) => {
        if (!alive) return;
        // Text is always shown as plain text so markup in the file is never interpreted.
        const blobType = mimeType?.startsWith('text/') ? 'text/plain' : mimeType;
        objectUrl = window.URL.createObjectURL(new Blob([res.data], { type: blobType }));
        setUrl(objectUrl);
      })
      .catch((err) => {
        if (alive) setError(statusOf(err) === 404 ? 'missing' : 'failed');
      })
      .finally(() => alive && setLoading(false));
    return () => {
      alive = false;
      if (objectUrl) window.URL.revokeObjectURL(objectUrl);
    };
  }, [id, mimeType, load]);

  if (loading) return <Skeleton height={200} label={t('intake.preview.loading', 'Loading preview')} />;
  if (error) {
    return (
      <Notice
        tone="danger"
        title={
          error === 'missing'
            ? t('intake.preview.missing', 'The file was not found or has been deleted.')
            : t('intake.preview.failed', 'The file could not be loaded for preview.')
        }
      >
        {t('intake.preview.moved', 'The original file may have been deleted or moved from storage.')}
      </Notice>
    );
  }
  if (!url) return null;
  if (mimeType?.startsWith('image/')) return <img className={styles.previewImage} src={url} alt={filename} />;
  if (mimeType === 'application/pdf' || mimeType?.startsWith('text/')) {
    return <iframe className={styles.previewFrame} src={url} title={filename} sandbox={previewSandbox(mimeType)} />;
  }
  return (
    <Notice title={t('intake.preview.unsupported', 'No preview for this file type ({{type}})', { type: mimeType || t('intake.preview.unknownType', 'unknown') })}>
      {t('intake.preview.download', 'Download the file to open it on your computer.')}
    </Notice>
  );
}
