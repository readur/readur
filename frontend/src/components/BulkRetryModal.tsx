import React, { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  Label,
  Radio,
  RadioGroup,
  Slider,
  SliderOutput,
  SliderThumb,
  SliderTrack,
  ToggleButton,
} from 'react-aria-components';
import { Button, Checkbox, Dialog, Pass, PassCell, TextField } from '../ui';
import {
  documentService,
  ocrService,
  ErrorHelper,
  ErrorCodes,
  type BulkOcrRetryRequest,
  type BulkOcrRetryResponse,
  type OcrRetryFilter,
} from '../services/api';
import LanguageSelector from './LanguageSelector';
import styles from './RetryModals.module.css';

interface BulkRetryModalProps {
  open: boolean;
  onClose: () => void;
  onSuccess: (result: BulkOcrRetryResponse) => void;
  selectedDocumentIds?: string[];
}

type Mode = 'all' | 'specific' | 'filter';

const COMMON_MIME_TYPES = [
  { value: 'application/pdf', label: 'PDF' },
  { value: 'image/png', label: 'PNG' },
  { value: 'image/jpeg', label: 'JPEG' },
  { value: 'image/tiff', label: 'TIFF' },
  { value: 'text/plain', label: 'Text' },
];

const COMMON_FAILURE_REASONS: Array<{ value: string; key: string; label: string }> = [
  { value: 'pdf_font_encoding', key: 'intake.bulkRetry.reason.font', label: 'Font encoding issues' },
  { value: 'ocr_timeout', key: 'intake.bulkRetry.reason.timeout', label: 'Processing timeout' },
  { value: 'pdf_corruption', key: 'intake.bulkRetry.reason.corruption', label: 'File corruption' },
  { value: 'low_ocr_confidence', key: 'intake.bulkRetry.reason.lowConfidence', label: 'Low confidence' },
  { value: 'no_extractable_text', key: 'intake.bulkRetry.reason.noText', label: 'No text found' },
  { value: 'ocr_memory_limit', key: 'intake.bulkRetry.reason.memory', label: 'Memory limit' },
];

const FILE_SIZE_PRESETS = [
  { label: '< 1 MB', value: 1024 * 1024 },
  { label: '< 5 MB', value: 5 * 1024 * 1024 },
  { label: '< 10 MB', value: 10 * 1024 * 1024 },
  { label: '< 50 MB', value: 50 * 1024 * 1024 },
];

const formatFileSize = (bytes: number) => {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
};

/** Retry OCR for all failed documents, the selected ones, or those matching criteria; preview first. */
export const BulkRetryModal: React.FC<BulkRetryModalProps> = ({ open, onClose, onSuccess, selectedDocumentIds = [] }) => {
  const { t } = useTranslation();
  const [mode, setMode] = useState<Mode>('all');
  const [filter, setFilter] = useState<OcrRetryFilter>({});
  const [usePriority, setUsePriority] = useState(false);
  const [priority, setPriority] = useState(10);
  const [languages, setLanguages] = useState<string[]>([]);
  const [primaryLanguage, setPrimaryLanguage] = useState<string | undefined>(undefined);
  const [loading, setLoading] = useState(false);
  const [preview, setPreview] = useState<BulkOcrRetryResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Start from a clean form each time the modal opens.
  useEffect(() => {
    if (!open) return;
    setMode(selectedDocumentIds.length > 0 ? 'specific' : 'all');
    setFilter({});
    setUsePriority(false);
    setPriority(10);
    setLanguages([]);
    setPrimaryLanguage(undefined);
    setPreview(null);
    setError(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  const changeFilter = <K extends keyof OcrRetryFilter>(key: K, value: OcrRetryFilter[K]) => {
    setFilter((prev) => ({ ...prev, [key]: value }));
    setPreview(null);
  };
  const toggleIn = (key: 'mime_types' | 'failure_reasons', value: string) => {
    const current = filter[key] ?? [];
    changeFilter(key, current.includes(value) ? current.filter((v) => v !== value) : [...current, value]);
  };

  const buildRequest = (previewOnly: boolean): BulkOcrRetryRequest => {
    const request: BulkOcrRetryRequest = { mode, preview_only: previewOnly };
    if (mode === 'specific') request.document_ids = selectedDocumentIds;
    else if (mode === 'filter') request.filter = filter;
    if (usePriority) request.priority_override = priority;
    return request;
  };

  const describeError = (err: unknown, action: 'preview' | 'execute') => {
    const info = ErrorHelper.formatErrorForDisplay(err, true);
    if (ErrorHelper.isErrorCode(err, ErrorCodes.USER_SESSION_EXPIRED) || ErrorHelper.isErrorCode(err, ErrorCodes.USER_TOKEN_EXPIRED)) {
      return t('intake.bulkRetry.errors.session', 'Your session has expired. Refresh the page and sign in again.');
    }
    if (ErrorHelper.isErrorCode(err, ErrorCodes.USER_PERMISSION_DENIED)) {
      return t('intake.bulkRetry.errors.permission', 'You do not have permission to retry documents.');
    }
    if (ErrorHelper.isErrorCode(err, ErrorCodes.DOCUMENT_NOT_FOUND)) {
      return t('intake.bulkRetry.errors.none', 'No documents match these criteria.');
    }
    if (action === 'execute' && ErrorHelper.isErrorCode(err, ErrorCodes.DOCUMENT_OCR_FAILED)) {
      return t('intake.bulkRetry.errors.ocr', 'Some documents cannot be retried because of processing issues.');
    }
    if (info.category === 'server') return t('intake.bulkRetry.errors.server', 'Server error. Try again later.');
    if (info.category === 'network') return t('intake.bulkRetry.errors.network', 'Network error. Check your connection and try again.');
    return (
      info.message ||
      (action === 'preview'
        ? t('intake.bulkRetry.errors.preview', 'Could not preview the retry')
        : t('intake.bulkRetry.errors.execute', 'Could not start the retry'))
    );
  };

  const handlePreview = async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await documentService.bulkRetryOcr(buildRequest(true));
      setPreview(response.data);
    } catch (err) {
      setError(describeError(err, 'preview'));
      setPreview(null);
    } finally {
      setLoading(false);
    }
  };

  /** Selected documents with chosen languages are retried one by one with those languages. */
  const retryWithLanguages = async (): Promise<BulkOcrRetryResponse> => {
    const results = await Promise.allSettled(
      selectedDocumentIds.map((id) => ocrService.retryWithLanguage(id, undefined, languages)),
    );
    const queued = results.filter((r) => r.status === 'fulfilled').length;
    return {
      success: queued > 0,
      message: '',
      queued_count: queued,
      matched_count: selectedDocumentIds.length,
      documents: [],
      estimated_total_time_minutes: preview?.estimated_total_time_minutes ?? 0,
    };
  };

  const handleExecute = async () => {
    setLoading(true);
    setError(null);
    try {
      const result =
        mode === 'specific' && languages.length > 0
          ? await retryWithLanguages()
          : (await documentService.bulkRetryOcr(buildRequest(false))).data;
      onSuccess(result);
      onClose();
    } catch (err) {
      setError(describeError(err, 'execute'));
    } finally {
      setLoading(false);
    }
  };

  const formatDuration = (minutes: number) => {
    if (minutes < 1) return t('intake.bulkRetry.seconds', '{{n}} seconds', { n: Math.round(minutes * 60) });
    if (minutes < 60) return t('intake.bulkRetry.minutes', '{{n}} minutes', { n: Math.round(minutes) });
    return t('intake.bulkRetry.hours', '{{n}} hours', { n: Math.round(minutes / 60) });
  };

  const matched = preview?.matched_count ?? 0;
  return (
    <Dialog
      isOpen={open}
      onOpenChange={(isOpen) => !isOpen && onClose()}
      size="lg"
      isDismissable={!loading}
      title={t('intake.bulkRetry.title', 'Bulk OCR retry')}
      actions={
        <>
          <Button variant="ghost" onPress={onClose} isDisabled={loading}>
            {t('intake.actions.cancel', 'Cancel')}
          </Button>
          <Button onPress={() => void handlePreview()} isDisabled={loading}>
            {t('intake.bulkRetry.preview', 'Preview')}
          </Button>
          <Button variant="primary" onPress={() => void handleExecute()} isPending={loading} isDisabled={!preview || matched === 0}>
            {t('intake.bulkRetry.execute', 'Retry {{count}} documents', { count: matched })}
          </Button>
        </>
      }
    >
      <div className={styles.stack}>
        {error ? (
          <p className={styles.error} role="alert">
            {error}
          </p>
        ) : null}

        <RadioGroup
          className={styles.radioGroup}
          value={mode}
          onChange={(value) => {
            setMode(value as Mode);
            setPreview(null);
            setError(null);
          }}
        >
          <Label className={styles.heading}>{t('intake.bulkRetry.mode', 'Retry mode')}</Label>
          <Radio value="all" className={styles.radio}>
            {t('intake.bulkRetry.modeAll', 'Retry all failed OCR documents')}
          </Radio>
          <Radio value="specific" className={styles.radio} isDisabled={selectedDocumentIds.length === 0}>
            {t('intake.bulkRetry.modeSelected', 'Retry selected documents ({{count}} selected)', { count: selectedDocumentIds.length })}
          </Radio>
          <Radio value="filter" className={styles.radio}>
            {t('intake.bulkRetry.modeFilter', 'Retry documents matching criteria')}
          </Radio>
        </RadioGroup>

        {mode === 'filter' ? (
          <fieldset className={styles.fieldset}>
            <legend className={styles.heading}>{t('intake.bulkRetry.criteria', 'Criteria')}</legend>
            <div className={styles.toggleRow} role="group" aria-label={t('intake.bulkRetry.fileTypes', 'File types')}>
              <span className={styles.label}>{t('intake.bulkRetry.fileTypes', 'File types')}</span>
              {COMMON_MIME_TYPES.map(({ value, label }) => (
                <ToggleButton
                  key={value}
                  className={styles.toggle}
                  isSelected={filter.mime_types?.includes(value) ?? false}
                  onChange={() => toggleIn('mime_types', value)}
                >
                  {label}
                </ToggleButton>
              ))}
            </div>
            <div className={styles.toggleRow} role="group" aria-label={t('intake.bulkRetry.reasons', 'Failure reasons')}>
              <span className={styles.label}>{t('intake.bulkRetry.reasons', 'Failure reasons')}</span>
              {COMMON_FAILURE_REASONS.map(({ value, key, label }) => (
                <ToggleButton
                  key={value}
                  className={styles.toggle}
                  isSelected={filter.failure_reasons?.includes(value) ?? false}
                  onChange={() => toggleIn('failure_reasons', value)}
                >
                  {t(key, label)}
                </ToggleButton>
              ))}
            </div>
            <div className={styles.toggleRow} role="group" aria-label={t('intake.bulkRetry.maxSize', 'Maximum file size')}>
              <span className={styles.label}>{t('intake.bulkRetry.maxSize', 'Maximum file size')}</span>
              {FILE_SIZE_PRESETS.map(({ label, value }) => (
                <ToggleButton
                  key={value}
                  className={styles.toggle}
                  isSelected={filter.max_file_size === value}
                  onChange={() => changeFilter('max_file_size', filter.max_file_size === value ? undefined : value)}
                >
                  {label}
                </ToggleButton>
              ))}
            </div>
            {filter.max_file_size ? (
              <p className={styles.muted}>
                {t('intake.bulkRetry.maxSizeValue', 'Up to {{size}}', { size: formatFileSize(filter.max_file_size) })}
              </p>
            ) : null}
            <TextField
              label={t('intake.bulkRetry.limit', 'Maximum documents to retry')}
              description={t('intake.bulkRetry.limitHint', 'Leave empty for no limit (1–1000).')}
              inputMode="numeric"
              value={filter.limit ? String(filter.limit) : ''}
              onChange={(v) => {
                const n = parseInt(v, 10);
                changeFilter('limit', Number.isNaN(n) ? undefined : Math.max(1, Math.min(1000, n)));
              }}
            />
          </fieldset>
        ) : null}

        <fieldset className={styles.fieldset}>
          <legend className={styles.heading}>{t('intake.bulkRetry.languages', 'OCR languages')}</legend>
          <p className={styles.muted}>
            {mode === 'specific'
              ? t('intake.bulkRetry.languagesHint', 'Optional. Read the selected documents again with these languages.')
              : t('intake.bulkRetry.languagesSelectedOnly', 'Choosing languages applies to selected documents only.')}
          </p>
          <LanguageSelector
            selectedLanguages={languages}
            primaryLanguage={primaryLanguage}
            onLanguagesChange={(next, primary) => {
              setLanguages(next);
              setPrimaryLanguage(primary ?? next[0]);
            }}
            disabled={mode !== 'specific' || loading}
          />
        </fieldset>

        <fieldset className={styles.fieldset}>
          <legend className={styles.heading}>{t('intake.bulkRetry.advanced', 'Advanced')}</legend>
          <Checkbox
            label={t('intake.bulkRetry.priorityOverride', 'Override processing priority')}
            isSelected={usePriority}
            onChange={setUsePriority}
          />
          {usePriority ? (
            <Slider className={styles.slider} minValue={1} maxValue={20} value={priority} onChange={(v) => setPriority(v as number)}>
              <div className={styles.headRow}>
                <Label className={styles.label}>{t('intake.bulkRetry.priority', 'Priority (higher is more urgent)')}</Label>
                <SliderOutput className={styles.mono} />
              </div>
              <SliderTrack className={styles.track}>
                {({ state }) => (
                  <>
                    <div className={styles.trackFill} style={{ width: `${state.getThumbPercent(0) * 100}%` }} />
                    <SliderThumb className={styles.thumb} />
                  </>
                )}
              </SliderTrack>
            </Slider>
          ) : null}
        </fieldset>

        {preview ? (
          <section className={styles.stack} aria-label={t('intake.bulkRetry.previewResults', 'Preview results')}>
            <Pass>
              <PassCell label={t('intake.bulkRetry.matched', 'Documents matched')} mono>
                {String(preview.matched_count)}
              </PassCell>
              <PassCell label={t('intake.bulkRetry.time', 'Estimated time')} mono>
                {formatDuration(preview.estimated_total_time_minutes)}
              </PassCell>
            </Pass>
            {preview.documents?.length ? (
              <ul className={styles.sample}>
                {preview.documents.slice(0, 10).map((doc) => (
                  <li key={doc.id}>
                    <span>{doc.filename}</span>
                    <span className={styles.mono}> · {formatFileSize(doc.file_size)}</span>
                    {doc.ocr_failure_reason ? <span className={styles.muted}> · {doc.ocr_failure_reason}</span> : null}
                  </li>
                ))}
                {preview.documents.length > 10 ? (
                  <li className={styles.muted}>
                    {t('intake.bulkRetry.more', 'and {{count}} more', { count: preview.documents.length - 10 })}
                  </li>
                ) : null}
              </ul>
            ) : null}
          </section>
        ) : null}
      </div>
    </Dialog>
  );
};
