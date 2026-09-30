import { useTranslation } from 'react-i18next';
import type { Document, OcrResponse } from '../../../services/api';
import type { StatusState } from '../../../ui';
import { Warning } from '../../../ui/icons';
import { formatDateTime, formatNumber } from '../format';
import styles from './Details.module.css';

interface ActivityEvent {
  id: string;
  at: string;
  /** OCR outcome, shown as a status mark. */
  state?: StatusState;
  title: string;
  description?: string;
}

/** What happened to the document so far: upload and the OCR outcome, oldest first. */
export function ActivityList({
  document,
  ocr,
  failure,
}: {
  document: Document;
  ocr: OcrResponse | null;
  failure?: string | null;
}) {
  const { t, i18n } = useTranslation();
  const events: ActivityEvent[] = [
    {
      id: 'upload',
      at: document.created_at,
      title: t('document.activity.uploaded', 'Added'),
      description: document.username
        ? t('document.activity.by', { name: document.username, defaultValue: 'by {{name}}' })
        : undefined,
    },
  ];

  // The API has no OCR completion time; the last update is the closest recorded moment.
  if (document.ocr_status === 'completed') {
    const words = ocr?.ocr_word_count ?? document.ocr_word_count;
    events.push({
      id: 'ocr',
      at: document.updated_at,
      state: 'completed',
      title: t('document.activity.ocrDone', 'Text extracted'),
      description:
        words != null
          ? t('document.text.words', { count: words, n: formatNumber(words, i18n.language), defaultValue: '{{n}} words' })
          : undefined,
    });
  } else if (document.ocr_status === 'failed') {
    events.push({
      id: 'ocr',
      at: document.updated_at,
      state: 'failed',
      title: t('document.activity.ocrFailed', 'OCR failed'),
      description: failure || undefined,
    });
  } else if (document.ocr_status === 'processing') {
    events.push({
      id: 'ocr',
      at: document.updated_at,
      state: 'processing',
      title: t('document.activity.ocrRunning', 'OCR running'),
    });
  }

  events.sort((a, b) => new Date(a.at).getTime() - new Date(b.at).getTime());

  return (
    <ol className={styles.activity}>
      {events.map((e) => (
        <li key={e.id} className={styles.event}>
          <div className={styles.eventText}>
            <span className={styles.eventTitle} data-state={e.state}>
              {e.state === 'failed' ? (
                <span aria-hidden="true" className={styles.eventGlyph}>
                  <Warning fontSize="inherit" />
                </span>
              ) : null}
              {e.title}
            </span>
            {e.description ? <span className={styles.eventDescription}>{e.description}</span> : null}
          </div>
          <time className={styles.eventTime} dateTime={e.at}>
            {formatDateTime(e.at, i18n.language)}
          </time>
        </li>
      ))}
    </ol>
  );
}
