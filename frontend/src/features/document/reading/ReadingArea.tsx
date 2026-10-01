import type { ReactNode, Ref } from 'react';
import { useTranslation } from 'react-i18next';
import type { ReadingView } from '../hooks/useReadingView';
import styles from './ReadingArea.module.css';

export interface ReadingAreaProps {
  view: ReadingView;
  preview: ReactNode;
  text: ReactNode;
  /** Measured height that reaches the bottom of the window. */
  height?: number;
  /** Negative bottom margin, to use the shell's bottom padding. */
  pullUp?: number;
  ref?: Ref<HTMLDivElement>;
}

/**
 * The file and its text in one box that fills the rest of the window. Both panes stay mounted
 * when hidden, so switching views keeps the PDF's scroll position and the find state.
 */
export function ReadingArea({ view, preview, text, height, pullUp, ref }: ReadingAreaProps) {
  const { t } = useTranslation();
  return (
    <div ref={ref} className={styles.area} data-view={view} style={height ? { height, marginBottom: pullUp ? -pullUp : undefined } : undefined}>
      <section className={styles.viewer} aria-label={t('document.view.document', 'Document')} hidden={view === 'text'}>
        {preview}
      </section>
      <div className={styles.text} hidden={view === 'document'}>
        {text}
      </div>
    </div>
  );
}
