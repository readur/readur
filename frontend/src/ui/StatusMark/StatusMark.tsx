import { useTranslation } from 'react-i18next';
import { cx } from '../shared/FieldParts';
import styles from './StatusMark.module.css';

export type StatusState =
  | 'pending'
  | 'processing'
  | 'completed'
  | 'failed'
  | 'healthy'
  | 'syncing'
  | 'warning'
  | 'error'
  | 'disabled';

export interface StatusMarkProps {
  state: StatusState;
  /** Known progress for `processing`; renders as `OCR 3/12`. */
  progress?: { current: number; total: number };
  size?: 'sm' | 'md';
  className?: string;
}

type Tone = 'ok' | 'danger' | 'neutral';

const VOCAB: Record<StatusState, { glyph: string; word: string; tone: Tone }> = {
  pending: { glyph: '○', word: 'PENDING', tone: 'neutral' },
  processing: { glyph: '◐', word: 'OCR', tone: 'neutral' },
  completed: { glyph: '■', word: 'INDEXED', tone: 'ok' },
  failed: { glyph: '▲', word: 'FAILED', tone: 'danger' },
  healthy: { glyph: '■', word: 'HEALTHY', tone: 'ok' },
  syncing: { glyph: '◐', word: 'SYNCING', tone: 'neutral' },
  warning: { glyph: '◆', word: 'CHECK', tone: 'neutral' },
  error: { glyph: '▲', word: 'ERROR', tone: 'danger' },
  disabled: { glyph: '—', word: 'OFF', tone: 'neutral' },
};

export const STATUS_STATES = Object.keys(VOCAB) as StatusState[];

/** Document or connection state: a shape plus a word, never colour alone. */
export function StatusMark({ state, progress, size = 'md', className }: StatusMarkProps) {
  const { t } = useTranslation();
  const entry = VOCAB[state] ?? VOCAB.pending;
  const showProgress = state === 'processing' && progress && progress.total > 0;
  const word = showProgress
    ? t('status.processingProgress', {
        defaultValue: 'OCR {{current}}/{{total}}',
        current: progress.current,
        total: progress.total,
      })
    : t(`status.${state}`, { defaultValue: entry.word });

  return (
    <span className={cx(styles.mark, styles[size], styles[entry.tone], className)} data-state={state}>
      <span className={styles.glyph} aria-hidden="true">
        {entry.glyph}
      </span>
      <span className={styles.word}>{word}</span>
    </span>
  );
}
