import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import type { Document, OcrResponse } from '../../../services/api';
import { Button, IconButton, StatusMark, useToast } from '../../../ui';
import { ContentCopy, History } from '../../../ui/icons';
import { humanizeFailureReason } from '../../../lib/failureReason';
import { formatBytes, formatDateTime, formatNumber, ocrState } from '../format';
import type { RetryInfo } from '../hooks/useRetryHistory';
import { ActivityList } from './ActivityList';
import { MetadataDisplay } from './MetadataDisplay';
import styles from './Details.module.css';

export interface DocumentDetailsProps {
  document: Document;
  ocr: OcrResponse | null;
  retry: RetryInfo;
  onShowRetryHistory: () => void;
  /** Element id, so the Details toggle can point at it with aria-controls. */
  id: string;
}

function Field({ label, children, mono }: { label: string; children: ReactNode; mono?: boolean }) {
  return (
    <div className={styles.row}>
      <dt className={styles.term}>{label}</dt>
      <dd className={mono ? `${styles.value} ${styles.mono}` : styles.value}>{children}</dd>
    </div>
  );
}

function Group({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className={styles.block}>
      <h2 className={styles.blockTitle}>{title}</h2>
      <dl className={styles.grid}>{children}</dl>
    </div>
  );
}

/** File metadata, processing details and activity for one document. */
export function DocumentDetails({
  document: doc,
  ocr,
  retry,
  onShowRetryHistory,
  id,
}: DocumentDetailsProps) {
  const { t, i18n } = useTranslation();
  const toast = useToast();
  const lng = i18n.language;

  const copyHash = async () => {
    if (!doc.file_hash) return;
    try {
      await navigator.clipboard.writeText(doc.file_hash);
      toast.show({ title: t('document.details.hashCopied', 'Hash copied'), tone: 'success' });
    } catch {
      toast.show({ title: t('document.details.copyFailed', "Couldn't copy"), tone: 'danger' });
    }
  };

  const confidence = ocr?.ocr_confidence ?? doc.ocr_confidence;
  const words = ocr?.ocr_word_count ?? doc.ocr_word_count;
  const time = ocr?.ocr_processing_time_ms ?? doc.ocr_processing_time_ms;
  const failure = doc.ocr_status === 'failed' ? retry.lastFailure : null;
  const retries = retry.total;

  return (
    <section id={id} className={styles.details} aria-label={t('document.details.title', 'Details')}>
      <Group title={t('document.details.file', 'File')}>
        <Field label={t('document.details.filename', 'Filename')}>{doc.original_filename}</Field>
        <Field label={t('document.details.mimeType', 'MIME type')}>{doc.mime_type}</Field>
        <Field label={t('document.details.size', 'Size')}>{formatBytes(doc.file_size)}</Field>
        <Field label={t('document.details.added', 'Added')}>{formatDateTime(doc.created_at, lng)}</Field>
        {doc.updated_at !== doc.created_at ? (
          <Field label={t('document.details.modified', 'Last modified')}>{formatDateTime(doc.updated_at, lng)}</Field>
        ) : null}
        <Field label={t('document.details.addedBy', 'Added by')}>
          {doc.username || t('document.details.unknown', 'Unknown')}
        </Field>
        {doc.file_owner ? <Field label={t('document.details.owner', 'Owner')}>{doc.file_owner}</Field> : null}
        {doc.file_group ? <Field label={t('document.details.group', 'Group')}>{doc.file_group}</Field> : null}
        {doc.file_permissions != null ? (
          <Field label={t('document.details.permissions', 'Permissions')} mono>
            {`${doc.file_permissions.toString(8)} (${doc.file_permissions})`}
          </Field>
        ) : null}
        <Field label={t('document.details.hash', 'SHA-256')} mono>
          {doc.file_hash ? (
            <span className={styles.hash}>
              <span className={styles.hashText}>{doc.file_hash}</span>
              <IconButton
                size="sm"
                label={t('document.details.copyHash', 'Copy hash')}
                icon={<ContentCopy fontSize="inherit" />}
                onPress={copyHash}
              />
            </span>
          ) : (
            t('document.details.notAvailable', 'Not available')
          )}
        </Field>
      </Group>

      {doc.source_type || doc.source_path || doc.source_id || doc.original_created_at || doc.original_modified_at ? (
        <Group title={t('document.details.source', 'Source')}>
          {doc.source_type ? <Field label={t('document.details.sourceType', 'Type')}>{doc.source_type.replace(/_/g, ' ')}</Field> : null}
          {doc.source_path ? <Field label={t('document.details.sourcePath', 'Path')}>{doc.source_path}</Field> : null}
          {doc.source_id ? <Field label={t('document.details.sourceId', 'Source ID')} mono>{doc.source_id}</Field> : null}
          {doc.original_created_at ? (
            <Field label={t('document.details.originalCreated', 'Originally created')}>
              {formatDateTime(doc.original_created_at, lng)}
            </Field>
          ) : null}
          {doc.original_modified_at ? (
            <Field label={t('document.details.originalModified', 'Originally modified')}>
              {formatDateTime(doc.original_modified_at, lng)}
            </Field>
          ) : null}
        </Group>
      ) : null}

      <Group title={t('document.details.processing', 'Processing')}>
        <Field label={t('document.details.status', 'Status')}>
          <StatusMark state={ocrState(doc.ocr_status)} size="sm" />
        </Field>
        {confidence != null ? (
          <Field label={t('document.details.confidence', 'Confidence')}>{`${Math.round(confidence)}%`}</Field>
        ) : null}
        {words != null ? <Field label={t('document.details.words', 'Words')}>{formatNumber(words, lng)}</Field> : null}
        {time ? <Field label={t('document.details.time', 'Processing time')}>{`${formatNumber(time, lng)} ms`}</Field> : null}
        {retries > 0 ? <Field label={t('document.details.retries', 'Retries')}>{retries}</Field> : null}
        {failure ? (
          <Field label={t('document.details.failure', 'Failure reason')}>{humanizeFailureReason(failure).summary}</Field>
        ) : null}
      </Group>

      <div className={styles.block}>
        <h2 className={styles.blockTitle}>{t('document.details.activity', 'Activity')}</h2>
        <ActivityList document={doc} ocr={ocr} failure={failure} />
        {retries > 0 ? (
          <Button size="sm" icon={<History fontSize="inherit" />} onPress={onShowRetryHistory}>
            {t('document.details.retryHistory', { count: retries, defaultValue: 'Retry history ({{count}})' })}
          </Button>
        ) : null}
      </div>

      <MetadataDisplay metadata={doc.source_metadata} title={t('document.details.sourceMetadata', 'Source metadata')} />
    </section>
  );
}
