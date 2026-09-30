import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import type { Document, OcrResponse } from '../../../services/api';
import { IconButton, SearchField, Skeleton, StatusMark, Switch, useToast } from '../../../ui';
import { ChevronLeft, ChevronRight, ContentCopy } from '../../../ui/icons';
import { formatNumber, isOcrActive, ocrState } from '../format';
import type { OcrLoadState } from '../hooks/useOcrText';
import { countMatches, splitMatches } from './highlight';
import styles from './OcrTextPanel.module.css';

export interface OcrTextPanelProps {
  document: Document;
  ocr: OcrResponse | null;
  ocrState: OcrLoadState;
  /** Latest recorded OCR failure, shown when OCR failed. */
  failure?: string | null;
  /** Prefills the find field (the `?q=` the user searched for) and scrolls to the first match. */
  initialQuery?: string;
}

/** Extracted text with find-in-text, a mono/readable toggle, copy-all and OCR stats. */
export function OcrTextPanel({ document, ocr, ocrState: loadState, failure, initialQuery = '' }: OcrTextPanelProps) {
  const { t, i18n } = useTranslation();
  const toast = useToast();
  const [query, setQuery] = useState(initialQuery);
  const [current, setCurrent] = useState(0);
  const [mono, setMono] = useState(false);
  const bodyRef = useRef<HTMLDivElement>(null);
  const text = ocr?.ocr_text ?? '';

  const segments = useMemo(() => splitMatches(text, query), [text, query]);
  const total = countMatches(segments);
  const active = total > 0 ? Math.min(current, total - 1) : 0;

  useEffect(() => setCurrent(0), [query]);

  useEffect(() => {
    if (total === 0) return;
    const el = bodyRef.current?.querySelector<HTMLElement>(`mark[data-match="${active}"]`);
    if (el && typeof el.scrollIntoView === 'function') el.scrollIntoView({ block: 'center' });
  }, [active, total, segments]);

  const step = (delta: number) => {
    if (total > 0) setCurrent((active + delta + total) % total);
  };

  const copyAll = async () => {
    try {
      await navigator.clipboard.writeText(text);
      toast.show({ title: t('document.text.copied', 'Text copied'), tone: 'success' });
    } catch {
      toast.show({ title: t('document.text.copyFailed', "Couldn't copy the text"), tone: 'danger' });
    }
  };

  const stats: string[] = [];
  const confidence = ocr?.ocr_confidence ?? document.ocr_confidence;
  if (confidence != null) {
    stats.push(t('document.text.confidence', { value: Math.round(confidence), defaultValue: '{{value}}% confidence' }));
  }
  const words = ocr?.ocr_word_count ?? document.ocr_word_count;
  if (words != null) {
    const n = formatNumber(words, i18n.language);
    stats.push(t('document.text.words', { count: words, n, defaultValue: '{{n}} words' }));
  }
  const time = ocr?.ocr_processing_time_ms ?? document.ocr_processing_time_ms;
  if (time) {
    stats.push(t('document.text.time', { ms: formatNumber(time, i18n.language), defaultValue: '{{ms}} ms' }));
  }


  const readout =
    query.trim() === ''
      ? ''
      : total === 0
        ? t('document.text.noMatches', 'No matches')
        : t('document.text.matchPosition', { current: active + 1, total, defaultValue: '{{current}} of {{total}}' });

  return (
    <section className={styles.panel} aria-labelledby="document-text-heading">
      <div className={styles.head}>
        <h2 id="document-text-heading" className={styles.heading}>
          {t('document.text.title', 'Text')}
        </h2>
        {stats.length > 0 ? <p className={styles.stats}>{stats.join(' · ')}</p> : null}
      </div>

      <div className={styles.toolbar}>
        <SearchField
          aria-label={t('document.text.find', 'Find in text')}
          placeholder={t('document.text.findPlaceholder', 'Find in text…')}
          value={query}
          onChange={setQuery}
          onSubmit={() => step(1)}
          className={styles.find}
        />
        <span className={styles.readout} role="status" aria-live="polite">
          {readout}
        </span>
        <IconButton
          size="sm"
          label={t('document.text.previousMatch', 'Previous match')}
          icon={<ChevronLeft fontSize="inherit" />}
          isDisabled={total === 0}
          onPress={() => step(-1)}
        />
        <IconButton
          size="sm"
          label={t('document.text.nextMatch', 'Next match')}
          icon={<ChevronRight fontSize="inherit" />}
          isDisabled={total === 0}
          onPress={() => step(1)}
        />
        <Switch isSelected={mono} onChange={setMono} label={t('document.text.monospace', 'Monospace')} />
        <IconButton
          size="sm"
          label={t('document.text.copyAll', 'Copy all text')}
          icon={<ContentCopy fontSize="inherit" />}
          isDisabled={!text}
          onPress={copyAll}
        />
      </div>

      <TextBody
        document={document}
        ocr={ocr}
        failure={failure}
        loadState={loadState}
        render={() => (
          <div
            ref={bodyRef}
            role="region"
            tabIndex={0}
            aria-label={t('document.text.body', 'Extracted text')}
            className={mono ? `${styles.body} ${styles.mono}` : styles.body}
          >
            {segments.map((s, i) =>
              s.match < 0 ? (
                <span key={i}>{s.text}</span>
              ) : (
                <mark
                  key={i}
                  data-match={s.match}
                  data-current={s.match === active ? 'true' : undefined}
                  className={styles.mark}
                >
                  {s.text}
                </mark>
              ),
            )}
          </div>
        )}
      />
    </section>
  );
}

function TextBody({
  document,
  ocr,
  failure,
  loadState,
  render,
}: {
  document: Document;
  ocr: OcrResponse | null;
  failure?: string | null;
  loadState: OcrLoadState;
  render: () => ReactNode;
}) {
  const { t } = useTranslation();
  const state = ocrState(document.ocr_status);

  if (loadState === 'loading') {
    return <Skeleton lines={8} label={t('document.text.loading', 'Loading text')} />;
  }
  if (ocr?.ocr_text) return <>{render()}</>;

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
    return (
      <div className={styles.notice}>
        <StatusMark state="failed" />
        <p>{failure || t('document.text.failed', 'OCR could not read this document.')}</p>
      </div>
    );
  }
  if (loadState === 'error') {
    return <p className={styles.notice}>{t('document.text.loadFailed', "Couldn't load the extracted text.")}</p>;
  }
  return <p className={styles.notice}>{t('document.text.empty', 'No text was found in this document.')}</p>;
}
