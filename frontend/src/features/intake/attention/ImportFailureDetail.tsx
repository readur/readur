import { useId } from 'react';
import { useTranslation } from 'react-i18next';
import { Pass, PassCell, SlideOver, StatusMark } from '../../../ui';
import { ocrService, type FailedDocumentRow } from '../../../services/api';
import { formatBytes, formatDateTime } from '../shared/format';
import { sharedStyles } from '../shared/parts';
import { FailedDocumentPreview } from './FailedDocumentPreview';
import { confidenceText, failedName, failureSummary, reasonLabel, stageLabel } from './failureLabels';

export interface ImportFailureDetailProps {
  record: FailedDocumentRow | null;
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
}

/** Read-only details of a failed import record. It is not a document, so there are no actions. */
export function ImportFailureDetail({ record, isOpen, onOpenChange }: ImportFailureDetailProps) {
  const { t, i18n } = useTranslation();
  const ids = { error: useId(), preview: useId() };
  if (!record) return null;
  const lng = i18n.language;
  const name = failedName(record);

  return (
    <SlideOver title={name} isOpen={isOpen} onOpenChange={onOpenChange}>
      <div className={sharedStyles.stack}>
        <StatusMark state="failed" />
        <p className={sharedStyles.meta}>
          {t('intake.attention.recordOnly', 'This is a record of a failed import; the file was not added to the library.')}
        </p>
        <Pass aria-label={t('intake.attention.failure', 'Failure')}>
          <PassCell label={t('intake.attention.col.reason', 'Reason')}>{failureSummary(t, record)}</PassCell>
          <PassCell label={t('intake.attention.col.stage', 'Stage')}>{stageLabel(t, record.failure_stage)}</PassCell>
          <PassCell label={t('intake.attention.col.retries', 'Retries')} mono>{String(record.retry_count ?? 0)}</PassCell>
        </Pass>
        <Pass aria-label={t('intake.attention.file', 'File')}>
          <PassCell label={t('intake.attention.size', 'Size')} mono>{formatBytes(record.file_size, 2)}</PassCell>
          <PassCell label={t('intake.attention.type', 'Type')} mono>{record.mime_type || '—'}</PassCell>
          <PassCell label={t('intake.attention.source', 'Source')}>{record.ingestion_source || '—'}</PassCell>
        </Pass>
        <Pass aria-label={t('intake.attention.dates', 'Dates')}>
          <PassCell label={t('intake.attention.created', 'Added')} mono>{formatDateTime(record.created_at, lng)}</PassCell>
          <PassCell label={t('intake.attention.lastRetry', 'Last retry')} mono>
            {record.last_retry_at ? formatDateTime(record.last_retry_at, lng) : t('intake.attention.noRetries', 'none yet')}
          </PassCell>
          <PassCell label={t('intake.attention.updated', 'Updated')} mono>{formatDateTime(record.updated_at, lng)}</PassCell>
        </Pass>
        {record.failure_reason === 'low_ocr_confidence' ? (
          <Pass aria-label={t('intake.attention.ocrResult', 'OCR result')}>
            <PassCell label={t('intake.attention.confidence', 'Confidence')} mono>{confidenceText(record.ocr_confidence) ?? '—'}</PassCell>
            <PassCell label={t('intake.attention.words', 'Words')} mono>
              {typeof record.ocr_word_count === 'number' ? String(record.ocr_word_count) : '—'}
            </PassCell>
          </Pass>
        ) : null}
        <section className={sharedStyles.stack} aria-labelledby={ids.error}>
          <h3 id={ids.error} className={sharedStyles.heading}>{t('intake.attention.errorMessage', 'Error message')}</h3>
          <pre className={sharedStyles.codeBlock}>
            {record.error_message || t('intake.attention.noErrorMessage', 'No error message was recorded.')}
          </pre>
          <p className={sharedStyles.meta}>{t('intake.attention.reasonCode', 'Reason code: {{code}}', { code: reasonLabel(t, record.failure_reason) })}</p>
        </section>
        {record.tags.length > 0 ? (
          <p className={sharedStyles.meta}>{t('intake.attention.tags', 'Tags: {{tags}}', { tags: record.tags.join(', ') })}</p>
        ) : null}
        <section className={sharedStyles.stack} aria-labelledby={ids.preview}>
          <h3 id={ids.preview} className={sharedStyles.heading}>{t('intake.attention.preview', 'File')}</h3>
          <FailedDocumentPreview
            id={record.id}
            filename={name}
            mimeType={record.mime_type ?? ''}
            load={ocrService.viewFailedDocument}
          />
        </section>
      </div>
    </SlideOver>
  );
}
