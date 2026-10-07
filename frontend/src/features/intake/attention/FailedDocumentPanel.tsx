import { useId, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button, Pass, PassCell, SlideOver, StatusMark, useToast } from '../../../ui';
import { documentService, ocrService, type FailedOcrDocumentRow, type OcrRetryRecommendation } from '../../../services/api';
import { RetryHistoryList } from '../../../components/RetryHistoryModal';
import { RetryRecommendations } from '../../../components/RetryRecommendations';
import LanguageSelector from '../../../components/LanguageSelector';
import { acknowledge } from '../../board/litStore';
import { categoryOf, ErrorCodes, hasCode, serverMessage } from '../shared/errors';
import { formatBytes, formatDateTime } from '../shared/format';
import { HumanReason } from '../shared/HumanReason';
import { ConfirmDialog, sharedStyles } from '../shared/parts';
import { FailedDocumentPreview } from './FailedDocumentPreview';
import { attentionKeyOf, canRetry, failedName, ocrFailureSummary, reasonLabel } from './failureLabels';
import { outcomeOf } from './outcome';
import { useLastDefined } from '../../../lib/useLastDefined';

export interface FailedDocumentPanelProps {
  document: FailedOcrDocumentRow | null;
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  onChanged: () => void;
}

/** Why a document's OCR failed, what it looks like, its retry history, and a retry. */
export function FailedDocumentPanel({ document: current, isOpen, onOpenChange, onChanged }: FailedDocumentPanelProps) {
  const doc = useLastDefined(current);
  const { t, i18n } = useTranslation();
  const toast = useToast();
  const ids = { preview: useId(), history: useId(), error: useId() };
  const [retrying, setRetrying] = useState(false);
  const [languages, setLanguages] = useState<string[]>([]);
  const [primary, setPrimary] = useState<string | undefined>(undefined);
  /** A recommendation retries matching documents across the whole library: confirm it first. */
  const [recommended, setRecommended] = useState<OcrRetryRecommendation | null>(null);
  const [retryingGroup, setRetryingGroup] = useState(false);

  if (!doc) return null;
  const lng = i18n.language;
  const name = failedName(doc);

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
      acknowledge('attention', attentionKeyOf(doc));
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

  const retryGroup = async () => {
    if (!recommended) return;
    setRetryingGroup(true);
    try {
      const res = await documentService.bulkRetryOcr({ mode: 'filter', filter: recommended.filter, preview_only: false });
      const queued = res.data?.queued_count ?? 0;
      if (outcomeOf(queued, 1) === 'none') {
        // A 200 with nothing queued is not a success.
        toast.show({ title: t('intake.recommend.nothingQueued', 'Nothing was queued'), tone: 'danger' });
      } else {
        toast.show({
          title: t('intake.attention.retryQueued', '{{queued}} of {{matched}} documents queued', { queued, matched: res.data?.matched_count ?? queued }),
          tone: 'success',
        });
        onChanged();
      }
      setRecommended(null);
    } catch (error) {
      toast.show({ title: t('intake.recommend.retryFailed', 'Could not start the retry'), description: serverMessage(error), tone: 'danger' });
    } finally {
      setRetryingGroup(false);
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
          acknowledge('attention', attentionKeyOf(doc));
          onOpenChange(false);
        }}
      >
        {t('intake.attention.markSeen', 'Mark as seen')}
      </Button>
    </div>
  );

  return (
    <SlideOver title={name} isOpen={isOpen} onOpenChange={onOpenChange} footer={footer} resizable storageKey="failed-document">
      <div className={sharedStyles.stack}>
        <StatusMark state="failed" />
        <Pass aria-label={t('intake.attention.failure', 'Failure')}>
          <PassCell label={t('intake.attention.col.reason', 'Reason')}>{ocrFailureSummary(t, doc)}</PassCell>
          <PassCell label={t('intake.attention.col.retries', 'Retries')} mono>{String(doc.retry_count ?? 0)}</PassCell>
        </Pass>
        <Pass aria-label={t('intake.attention.file', 'File')}>
          <PassCell label={t('intake.attention.size', 'Size')} mono>{formatBytes(doc.file_size, 2)}</PassCell>
          <PassCell label={t('intake.attention.type', 'Type')} mono>{doc.mime_type || '—'}</PassCell>
        </Pass>
        <Pass aria-label={t('intake.attention.dates', 'Dates')}>
          <PassCell label={t('intake.attention.created', 'Added')} mono>{formatDateTime(doc.created_at, lng)}</PassCell>
          <PassCell label={t('intake.attention.lastRetry', 'Last retry')} mono>
            {doc.last_attempt_at ? formatDateTime(doc.last_attempt_at, lng) : t('intake.attention.noRetries', 'none yet')}
          </PassCell>
          <PassCell label={t('intake.attention.updated', 'Updated')} mono>{formatDateTime(doc.updated_at, lng)}</PassCell>
        </Pass>

        <section className={sharedStyles.stack} aria-labelledby={ids.error}>
          <h3 id={ids.error} className={sharedStyles.heading}>{t('intake.attention.errorMessage', 'Error message')}</h3>
          {doc.ocr_error ? (
            <HumanReason raw={doc.ocr_error} code={doc.ocr_failure_reason} />
          ) : (
            <p className={sharedStyles.meta}>{t('intake.attention.noErrorMessage', 'No error message was recorded.')}</p>
          )}
          {doc.ocr_failure_reason ? (
            <p className={sharedStyles.meta}>
              {t('intake.attention.reasonCode', 'Reason code: {{code}}', { code: reasonLabel(t, doc.ocr_failure_reason) })}
            </p>
          ) : null}
        </section>

        {doc.tags?.length > 0 ? (
          <p className={sharedStyles.meta}>{t('intake.attention.tags', 'Tags: {{tags}}', { tags: doc.tags.join(', ') })}</p>
        ) : null}

        <section className={sharedStyles.stack} aria-labelledby={ids.preview}>
          <h3 id={ids.preview} className={sharedStyles.heading}>{t('intake.attention.preview', 'File')}</h3>
          <FailedDocumentPreview id={doc.id} filename={name} mimeType={doc.mime_type ?? ''} load={documentService.view} />
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

        <RetryRecommendations onRetryClick={setRecommended} />
      </div>
      <ConfirmDialog
        isOpen={recommended !== null}
        onOpenChange={(open) => !open && !retryingGroup && setRecommended(null)}
        tone="primary"
        title={t('intake.recommend.confirmTitle', 'Retry {{count}} documents across your library?', { count: recommended?.document_count ?? 0 })}
        confirmLabel={t('intake.recommend.retry', 'Retry {{count}} documents', { count: recommended?.document_count ?? 0 })}
        isPending={retryingGroup}
        onConfirm={retryGroup}
      >
        <p>
          {t('intake.recommend.confirmBody', 'This retries every failed document that matches "{{title}}", not only {{name}}.', {
            title: recommended?.title ?? '',
            name,
          })}
        </p>
      </ConfirmDialog>
    </SlideOver>
  );
}
