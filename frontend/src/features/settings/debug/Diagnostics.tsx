import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '../../../ui';
import { documentService } from '../../../services/api';
import { AuthenticatedImage } from '../../document/AuthenticatedImage';
import { DocumentViewer } from '../../document/reading/DocumentViewer';
import { Facts, YesNo } from '../shared/Facts';
import { Notice } from '../shared/Notice';
import shared from '../shared/shared.module.css';
import { HistoryTable } from './HistoryTable';
import { mb, when, type DebugInfo } from './types';
import styles from './Debug.module.css';

/** Everything below the pipeline: failure record, attempt log, file summary, images and user settings. */
export function Diagnostics({ info }: { info: DebugInfo }) {
  const { t } = useTranslation();
  const yes = t('debug.steps.fileInformation.yes');
  const no = t('debug.steps.fileInformation.no');
  const f = info.failed_document_info;
  const fa = info.file_analysis;
  const hasProcessedImage = (info.pipeline_steps || []).some((s) => s.step === 3 && s.details?.has_processed_image);
  const [showSettings, setShowSettings] = useState(false);
  const us = info.user_settings;
  const FD = 'debug.failedDocument';
  const FS = 'debug.fileAnalysisSummary';
  const U = 'debug.userSettings';

  return (
    <>
      {f ? (
        <section aria-labelledby="debug-failed" className={styles.block}>
          <h3 id="debug-failed" className={styles.blockTitle}>{t(`${FD}.title`)}</h3>
          <div className={shared.grid2}>
            <Facts
              title={t(`${FD}.failureDetails`)}
              items={[
                { label: t(`${FD}.failureReason`), value: f.failure_reason },
                { label: t(`${FD}.failureStage`), value: f.failure_stage },
                { label: t(`${FD}.retryCount`), value: f.retry_count || 0, mono: true },
                { label: t(`${FD}.created`), value: when(f.created_at), mono: true },
                ...(f.last_retry_at ? [{ label: t(`${FD}.lastRetry`), value: when(f.last_retry_at), mono: true }] : []),
              ]}
            />
            {f.failed_ocr_text ? (
              <Facts
                title={t(`${FD}.failedOcrResults`)}
                items={[
                  { label: t(`${FD}.ocrTextLength`), value: `${f.failed_ocr_text.length} ${t('debug.steps.fileAnalysis.chars')}`, mono: true },
                  { label: t(`${FD}.ocrConfidence`), value: `${f.failed_ocr_confidence?.toFixed(1)}%`, mono: true },
                  { label: t(`${FD}.wordCount`), value: f.failed_ocr_word_count || 0, mono: true },
                  { label: t(`${FD}.processingTime`), value: `${f.failed_ocr_processing_time_ms || 0}ms`, mono: true },
                ]}
              />
            ) : (
              <div className={shared.factsBox}>
                <p className={shared.factsTitle}>{t(`${FD}.failedOcrResults`)}</p>
                <p className={shared.meta}>{t(`${FD}.noOcrResults`)}</p>
              </div>
            )}
          </div>
          {f.error_message ? (
            <Notice tone="danger">
              <strong>{t(`${FD}.errorMessage`)}</strong> {f.error_message}
            </Notice>
          ) : null}
          {f.content_preview ? (
            <div className={shared.factsBox}>
              <p className={shared.factsTitle}>{t(`${FD}.contentPreview`)}</p>
              <pre className={styles.preview}>{f.content_preview}</pre>
            </div>
          ) : null}
        </section>
      ) : null}

      {info.detailed_processing_logs && info.detailed_processing_logs.length > 0 ? (
        <section className={styles.block}>
          <p className={shared.meta}>{t('debug.processingLogs.description')}</p>
          <HistoryTable title={t('debug.processingLogs.title')} rows={info.detailed_processing_logs} detailed />
        </section>
      ) : null}

      {fa ? (
        <section aria-labelledby="debug-file-summary" className={styles.block}>
          <h3 id="debug-file-summary" className={styles.blockTitle}>{t(`${FS}.title`)}</h3>
          <div className={shared.grid2}>
            <Facts
              title={t(`${FS}.fileProperties`)}
              items={[
                { label: t(`${FS}.fileType`), value: fa.file_type, mono: true },
                { label: t(`${FS}.size`), value: mb(fa.file_size_bytes), mono: true },
                { label: t(`${FS}.readable`), value: <YesNo value={fa.is_readable} yes={yes} no={no} /> },
              ]}
            />
            {fa.pdf_info ? (
              <Facts
                title={t(`${FS}.pdfProperties`)}
                items={[
                  { label: t(`${FS}.validPdf`), value: <YesNo value={fa.pdf_info.is_valid_pdf} yes={yes} no={no} /> },
                  { label: t(`${FS}.hasTextContent`), value: <YesNo value={fa.pdf_info.has_text_content} yes={yes} no={no} /> },
                  { label: t(`${FS}.textLength`), value: `${fa.pdf_info.estimated_text_length} ${t('debug.steps.fileAnalysis.chars')}`, mono: true },
                  { label: t(`${FS}.pageCount`), value: fa.pdf_info.page_count || t('debug.steps.fileMetadata.unknown'), mono: true },
                  { label: t(`${FS}.encrypted`), value: <YesNo value={fa.pdf_info.is_encrypted} yes={yes} no={no} /> },
                ]}
              />
            ) : null}
          </div>
          {fa.pdf_info?.text_extraction_error ? (
            <Notice tone="danger">
              <strong>{t(`${FS}.pdfTextExtractionIssue`)}</strong> {fa.pdf_info.text_extraction_error}
            </Notice>
          ) : null}
        </section>
      ) : null}

      {hasProcessedImage ? (
        <section aria-labelledby="debug-images" className={styles.block}>
          <h3 id="debug-images" className={styles.blockTitle}>{t('debug.processedImages.title')}</h3>
          <div className={shared.grid2}>
            <figure className={styles.figure}>
              <figcaption className={shared.factsTitle}>{t('debug.processedImages.originalDocument')}</figcaption>
              <div className={styles.frame}>
                <DocumentViewer
                  documentId={info.document_id}
                  filename={info.filename ?? t('debug.processedImages.originalDocument')}
                  mimeType={fa?.mime_type || 'application/octet-stream'}
                />
              </div>
            </figure>
            <figure className={styles.figure}>
              <figcaption className={shared.factsTitle}>{t('debug.processedImages.processedImage')}</figcaption>
              <AuthenticatedImage
                load={() => documentService.getProcessedImage(info.document_id)}
                resourceKey={info.document_id}
                alt={t('debug.processedImages.processedImage')}
                unavailableText={t('debug.processedImages.notAvailable')}
                className={styles.previewImage}
                messageClassName={shared.meta}
              />
            </figure>
          </div>
        </section>
      ) : null}

      <section className={styles.block}>
        <Button size="sm" variant="ghost" aria-expanded={showSettings} onPress={() => setShowSettings((v) => !v)}>
          {t(`${U}.title`)}
        </Button>
        {showSettings ? (
          <div className={shared.grid2}>
            <Facts
              title={t(`${U}.ocrSettings`)}
              items={[
                { label: t(`${U}.backgroundOcr`), value: us?.enable_background_ocr ? t(`${U}.enabled`) : t(`${U}.disabled`) },
                { label: t(`${U}.minConfidence`), value: `${us?.ocr_min_confidence || 'N/A'}%`, mono: true },
                { label: t(`${U}.maxFileSize`), value: `${us?.max_file_size_mb || 'N/A'} MB`, mono: true },
              ]}
            />
            <Facts
              title={t(`${U}.qualityThresholds`)}
              items={[
                { label: t(`${U}.brightness`), value: us?.ocr_quality_threshold_brightness || 'N/A', mono: true },
                { label: t(`${U}.contrast`), value: us?.ocr_quality_threshold_contrast || 'N/A', mono: true },
                { label: t(`${U}.noise`), value: us?.ocr_quality_threshold_noise || 'N/A', mono: true },
                { label: t(`${U}.sharpness`), value: us?.ocr_quality_threshold_sharpness || 'N/A', mono: true },
              ]}
            />
          </div>
        ) : null}
      </section>
    </>
  );
}
