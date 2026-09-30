import { useId, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button, Pass, PassCell, SlideOver, StatusMark, useToast } from '../../../ui';
import { documentService, ocrService, type FailedDocumentRow } from '../../../services/api';
import { RetryHistoryList } from '../../../components/RetryHistoryModal';
import { RetryRecommendations } from '../../../components/RetryRecommendations';
import LanguageSelector from '../../../components/LanguageSelector';
import { acknowledge } from '../../board/litStore';
import { categoryOf, ErrorCodes, hasCode, serverMessage } from '../shared/errors';
import { formatBytes, formatDateTime } from '../shared/format';
import { sharedStyles } from '../shared/parts';
import { attentionIdOfDocument } from '../shared/seenEvents';
import { FailedDocumentPreview } from './FailedDocumentPreview';
import { canRetry, confidenceText, failedName, failureSummary, reasonLabel, stageLabel } from './failureLabels';

export interface FailedDocumentPanelProps {
  document: FailedDocumentRow | null;
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  onChanged: () => void;
}

/** Why a document failed, what it looked like, its retry history, and a retry. */
export function FailedDocumentPanel({ document: doc, isOpen, onOpenChange, onChanged }: FailedDocumentPanelProps) {
  const { t, i18n } = useTranslation();
  const toast = useToast();
  const ids = { preview: useId(), history: useId(), recommend: useId(), error: useId() };
  const [retrying, setRetrying] = useState(false);
  const [languages, setLanguages] = useState<string[]>([]);
  const [primary, setPrimary] = useState<string | undefined>(undefined);

  if (!doc) return null;
  const lng = i18n.language;
  const name = failedName(doc);
  const confidence = confidenceText(doc.ocr_confidence);

  const retry = async () => {
    setRetrying(true);
    try {
      const res =
        languages.length > 0
          ? await ocrService.retryWithLanguage(doc.id, undefined, languages)
          : await documentService.retryOcr(doc.id);
      const data = res.data as { success?: boolean; message?: string; estimated_wait_minutes?: number } | undefined;
      if (data?.success === false) {
        toast.show({ title: t('intake.attention.retryFailed', 'Could not retry OCR'), description: data.message, tone: 'danger' });
        return;
      }
      acknowledge('attention', attentionIdOfDocument(doc.id));
      toast.show({
        title: t('intake.attention.retryStarted', 'OCR retry queued'),
        description:
          data?.estimated_wait_minutes !== undefined
            ? t('intake.attention.retryWait', '{{name}} · about {{minutes}} min', { name, minutes: data.estimated_wait_minutes })
            : name,
        tone: 'success',
      });
      onChanged();
    } catch (error) {
      let description = serverMessage(error);
      if (hasCode(error, ErrorCodes.DOCUMENT_NOT_FOUND)) description = t('intake.attention.notFound', 'The document no longer exists.');
      else if (hasCode(error, ErrorCodes.DOCUMENT_OCR_FAILED)) description = t('intake.attention.cannotRetry', 'This document cannot be retried.');
      else if (categoryOf(error) === 'network') description = t('intake.upload.errors.network', 'Network error. Check your connection.');
      toast.show({ title: t('intake.attention.retryFailed', 'Could not retry OCR'), description, tone: 'danger' });
    } finally {
      setRetrying(false);
    }
  };

  const download = async () => {
    try {
      await documentService.downloadFile(doc.id, name);
    } catch {
      toast.show({ title: t('intake.attention.downloadFailed', 'Could not download the file'), tone: 'danger' });
    }
  };

  const footer = (
    <div className={sharedStyles.toolbar}>
      <Button variant="primary" onPress={retry} isPending={retrying} isDisabled={!canRetry(doc)}>
        {t('intake.attention.retryOcr', 'Retry OCR')}
      </Button>
      <Button onPress={download}>{t('intake.attention.download', 'Download')}</Button>
      <Button
        variant="ghost"
        onPress={() => {
          acknowledge('attention', attentionIdOfDocument(doc.id));
          onOpenChange(false);
        }}
      >
        {t('intake.attention.markSeen', 'Mark as seen')}
      </Button>
    </div>
  );

  return (
    <SlideOver title={name} isOpen={isOpen} onOpenChange={onOpenChange} footer={footer}>
      <div className={sharedStyles.stack}>
        <StatusMark state="failed" />
        <Pass aria-label={t('intake.attention.failure', 'Failure')}>
          <PassCell label={t('intake.attention.col.reason', 'Reason')}>{failureSummary(t, doc)}</PassCell>
          <PassCell label={t('intake.attention.col.stage', 'Stage')}>{stageLabel(t, doc.failure_stage)}</PassCell>
          <PassCell label={t('intake.attention.col.retries', 'Retries')} mono>{String(doc.retry_count ?? 0)}</PassCell>
        </Pass>
        <Pass aria-label={t('intake.attention.file', 'File')}>
          <PassCell label={t('intake.attention.size', 'Size')} mono>{formatBytes(doc.file_size, 2)}</PassCell>
          <PassCell label={t('intake.attention.type', 'Type')} mono>{doc.mime_type || '—'}</PassCell>
          <PassCell label={t('intake.attention.source', 'Source')}>{doc.ingestion_source || '—'}</PassCell>
        </Pass>
        <Pass aria-label={t('intake.attention.dates', 'Dates')}>
          <PassCell label={t('intake.attention.created', 'Added')} mono>{formatDateTime(doc.created_at, lng)}</PassCell>
          <PassCell label={t('intake.attention.lastRetry', 'Last retry')} mono>
            {doc.last_retry_at ? formatDateTime(doc.last_retry_at, lng) : t('intake.attention.noRetries', 'none yet')}
          </PassCell>
          <PassCell label={t('intake.attention.updated', 'Updated')} mono>{formatDateTime(doc.updated_at, lng)}</PassCell>
        </Pass>
        {doc.failure_reason === 'low_ocr_confidence' ? (
          <Pass aria-label={t('intake.attention.ocrResult', 'OCR result')}>
            <PassCell label={t('intake.attention.confidence', 'Confidence')} mono>{confidence ?? '—'}</PassCell>
            <PassCell label={t('intake.attention.words', 'Words')} mono>
              {typeof doc.ocr_word_count === 'number' ? String(doc.ocr_word_count) : '—'}
            </PassCell>
          </Pass>
        ) : null}

        <section className={sharedStyles.stack} aria-labelledby={ids.error}>
          <h3 id={ids.error} className={sharedStyles.heading}>{t('intake.attention.errorMessage', 'Error message')}</h3>
          <pre className={sharedStyles.codeBlock}>
            {doc.error_message || t('intake.attention.noErrorMessage', 'No error message was recorded.')}
          </pre>
          <p className={sharedStyles.meta}>{t('intake.attention.reasonCode', 'Reason code: {{code}}', { code: reasonLabel(t, doc.failure_reason) })}</p>
        </section>

        {doc.tags.length > 0 ? (
          <p className={sharedStyles.meta}>{t('intake.attention.tags', 'Tags: {{tags}}', { tags: doc.tags.join(', ') })}</p>
        ) : null}

        <section className={sharedStyles.stack} aria-labelledby={ids.preview}>
          <h3 id={ids.preview} className={sharedStyles.heading}>{t('intake.attention.preview', 'File')}</h3>
          <FailedDocumentPreview failedDocumentId={doc.id} filename={name} mimeType={doc.mime_type ?? ''} />
        </section>

        <section className={sharedStyles.stack} aria-label={t('intake.attention.retryLanguages', 'Retry with languages')}>
          <h3 className={sharedStyles.heading}>{t('intake.attention.retryLanguages', 'Retry with languages')}</h3>
          <p className={sharedStyles.meta}>{t('intake.attention.retryLanguagesHint', 'Optional. Leave empty to use your default OCR language.')}</p>
          <LanguageSelector
            selectedLanguages={languages}
            primaryLanguage={primary}
            onLanguagesChange={(next, p) => {
              setLanguages(next);
              setPrimary(p ?? next[0]);
            }}
            disabled={retrying}
          />
        </section>

        <section className={sharedStyles.stack} aria-labelledby={ids.history}>
          <h3 id={ids.history} className={sharedStyles.heading}>{t('intake.retry.title', 'OCR retry history')}</h3>
          <RetryHistoryList documentId={doc.id} enabled={isOpen} />
        </section>

        <RetryRecommendations onRetrySuccess={onChanged} />
      </div>
    </SlideOver>
  );
}
