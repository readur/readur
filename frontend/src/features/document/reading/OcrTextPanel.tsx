import { useEffect, useMemo, useRef, useState } from 'react';
import type { Selection } from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import type { Document, OcrResponse } from '../../../services/api';
import { Button, IconButton, Menu, MenuItem, MenuTrigger, useToast } from '../../../ui';
import { ContentCopy, Tune } from '../../../ui/icons';
import { formatNumber } from '../format';
import type { OcrLoadState } from '../hooks/useOcrText';
import { FindField } from './FindField';
import { countMatches, splitMatches, toParagraphs } from './highlight';
import { TextBody } from './TextBody';
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

export const MONO_STORAGE_KEY = 'readur.document.mono';

function readMono(): boolean {
  try {
    return window.localStorage.getItem(MONO_STORAGE_KEY) === '1';
  } catch {
    return false;
  }
}

function writeMono(on: boolean) {
  try {
    window.localStorage.setItem(MONO_STORAGE_KEY, on ? '1' : '0');
  } catch {
    // The choice still holds for this page.
  }
}

/** The extracted text, set for reading, with find, a monospace option and copy. */
export function OcrTextPanel({ document, ocr, ocrState: loadState, failure, initialQuery = '' }: OcrTextPanelProps) {
  const { t, i18n } = useTranslation();
  const toast = useToast();
  const [query, setQuery] = useState(initialQuery);
  const [current, setCurrent] = useState(0);
  const [mono, setMono] = useState(readMono);
  const bodyRef = useRef<HTMLDivElement>(null);
  const text = ocr?.ocr_text ?? '';

  const segments = useMemo(() => splitMatches(text, query), [text, query]);
  const paragraphs = useMemo(() => toParagraphs(segments), [segments]);
  const total = countMatches(segments);
  const active = total > 0 ? Math.min(current, total - 1) : 0;

  useEffect(() => setCurrent(0), [query]);

  useEffect(() => {
    if (total === 0) return;
    const el = bodyRef.current?.querySelector<HTMLElement>(`mark[data-match="${active}"]`);
    if (el && typeof el.scrollIntoView === 'function') el.scrollIntoView({ block: 'center' });
  }, [active, total, segments]);

  const step = (delta: 1 | -1) => {
    if (total > 0) setCurrent((active + delta + total) % total);
  };

  const copyAll = async () => {
    try {
      // Always the text exactly as extracted, not the tidied paragraphs on screen.
      await navigator.clipboard.writeText(text);
      toast.show({ title: t('document.text.copied', 'Text copied'), tone: 'success' });
    } catch {
      toast.show({ title: t('document.text.copyFailed', "Couldn't copy the text"), tone: 'danger' });
    }
  };

  const onOptions = (keys: Selection) => {
    const on = keys === 'all' || keys.has('mono');
    setMono(on);
    writeMono(on);
  };

  const stats: string[] = [];
  const words = ocr?.ocr_word_count ?? document.ocr_word_count;
  if (words != null) {
    const n = formatNumber(words, i18n.language);
    stats.push(t('document.text.words', { count: words, n, defaultValue: words === 1 ? '{{n}} word' : '{{n}} words' }));
  }
  const confidence = ocr?.ocr_confidence ?? document.ocr_confidence;
  if (confidence != null) {
    stats.push(t('document.text.confidence', { value: Math.round(confidence), defaultValue: '{{value}}% confidence' }));
  }
  const time = ocr?.ocr_processing_time_ms ?? document.ocr_processing_time_ms;
  if (time) {
    stats.push(t('document.text.time', { ms: formatNumber(time, i18n.language), defaultValue: '{{ms}} ms' }));
  }

  return (
    <section className={styles.panel} aria-labelledby="document-text-heading">
      <h2 id="document-text-heading" className="visually-hidden">
        {t('document.text.title', 'Text')}
      </h2>

      <div className={styles.toolbar}>
        <FindField value={query} onChange={setQuery} current={active} total={total} onStep={step} />
        <div className={styles.tools}>
          <MenuTrigger>
            <IconButton
              size="sm"
              label={t('document.text.displayOptions', 'Text display options')}
              icon={<Tune fontSize="inherit" />}
            />
            <Menu
              aria-label={t('document.text.displayOptions', 'Text display options')}
              selectionMode="multiple"
              selectedKeys={mono ? ['mono'] : []}
              onSelectionChange={onOptions}
            >
              <MenuItem id="mono" textValue={t('document.text.monospace', 'Monospace')}>
                {t('document.text.monospace', 'Monospace')}
              </MenuItem>
            </Menu>
          </MenuTrigger>
          <Button
            variant="secondary"
            size="sm"
            icon={<ContentCopy fontSize="inherit" />}
            aria-label={t('document.text.copyAll', 'Copy all text')}
            isDisabled={!text}
            onPress={copyAll}
          >
            {t('document.text.copy', 'Copy')}
          </Button>
        </div>
      </div>

      <TextBody document={document} ocr={ocr} failure={failure} loadState={loadState}>
        <div
          ref={bodyRef}
          role="region"
          tabIndex={0}
          aria-label={t('document.text.body', 'Extracted text')}
          className={styles.scroller}
          data-own-arrows=""
        >
          <article className={mono ? `${styles.prose} ${styles.mono}` : styles.prose}>
            {paragraphs.map((paragraph, p) => (
              <p key={p}>
                {paragraph.map((s, i) =>
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
              </p>
            ))}
          </article>
        </div>
      </TextBody>

      {stats.length > 0 ? <p className={styles.stats}>{stats.join(' · ')}</p> : null}
    </section>
  );
}
