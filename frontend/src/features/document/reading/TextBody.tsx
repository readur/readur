import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import type { Document, OcrResponse } from '../../../services/api';
import { Skeleton, StatusMark } from '../../../ui';
import { humanizeFailureReason } from '../../../lib/failureReason';
import { isOcrActive, ocrState } from '../format';
import type { OcrLoadState } from '../hooks/useOcrText';
import styles from './OcrTextPanel.module.css';

export interface TextBodyProps {
  document: Document;
  ocr: OcrResponse | null;
  failure?: string | null;
  loadState: OcrLoadState;
  /** The text itself, shown once there is some. */
  children: ReactNode;
}

/** The text, or why there is none yet: loading, OCR still running, failed, or empty. */
export function TextBody({ document, ocr, failure, loadState, children }: TextBodyProps) {
  const { t } = useTranslation();
  const state = ocrState(document.ocr_status);

  if (loadState === 'loading') {
    return (
      <div className={styles.notice}>
        <Skeleton lines={8} label={t('document.text.loading', 'Loading text')} />
      </div>
    );
  }
  if (ocr?.ocr_text) return <>{children}</>;

  if (!document.has_ocr_text && isOcrActive(document.ocr_status)) {
    return (
      <div className={styles.notice}>
        <StatusMark
          state={state}
          progress={
            document.ocr_progress_total
              ? { current: document.ocr_progress_current ?? 0, total: document.ocr_progress_total }
              : undefined
          }
        />
        <p>{t('document.text.pending', 'The text appears here when OCR finishes.')}</p>
      </div>
    );
  }
  if (state === 'failed') {
    const human = failure ? humanizeFailureReason(failure) : null;
    return (
      <div className={styles.notice}>
        <StatusMark state="failed" />
        <p>{human?.summary || t('document.text.failed', 'OCR could not read this document.')}</p>
        {human?.detail ? (
          <details className={styles.raw}>
            <summary>{t('document.text.failureDetail', 'Show the full message')}</summary>
            <pre>{human.detail}</pre>
          </details>
        ) : null}
      </div>
    );
  }
  if (loadState === 'error') {
    return (
      <div className={styles.notice}>
        <p>{t('document.text.loadFailed', "Couldn't load the extracted text.")}</p>
      </div>
    );
  }
  return (
    <div className={styles.notice}>
      <p>{t('document.text.empty', 'No text was found in this document.')}</p>
    </div>
  );
}
