import { RefreshCw } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { cx } from '../shared/FieldParts';
import { Spinner } from '../Spinner';
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

type Tone = 'ok' | 'active' | 'warn' | 'danger' | 'neutral';

const VOCAB: Record<StatusState, { word: string; tone: Tone }> = {
  pending: { word: 'Pending', tone: 'neutral' },
  processing: { word: 'OCR', tone: 'active' },
  completed: { word: 'Indexed', tone: 'ok' },
  failed: { word: 'Failed', tone: 'danger' },
  healthy: { word: 'Healthy', tone: 'ok' },
  syncing: { word: 'Syncing', tone: 'active' },
  warning: { word: 'Check', tone: 'warn' },
  error: { word: 'Error', tone: 'danger' },
  disabled: { word: 'Off', tone: 'neutral' },
};

export const STATUS_STATES = Object.keys(VOCAB) as StatusState[];

/** Document or connection state: a haloed dot (a spinner while in progress) plus a word, never colour alone. */
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

  const pct = showProgress ? Math.round((progress.current / progress.total) * 100) : 0;
  const active = entry.tone === 'active';

  return (
    <span className={cx(styles.mark, styles[size], styles[entry.tone], className)} data-state={state} data-tone={entry.tone}>
      <span className={styles.lead} data-lead="" aria-hidden="true">
        {active ? (
          state === 'syncing' ? (
            <RefreshCw className={styles.spin} width={13} height={13} strokeWidth={2.25} />
          ) : (
            <Spinner size={13} />
          )
        ) : (
          <span className={styles.dot} />
        )}
      </span>
      <span className={styles.word}>{word}</span>
      {showProgress ? (
        <span className={styles.bar} data-bar="" aria-hidden="true">
          <span style={{ width: `${pct}%` }} />
        </span>
      ) : null}
    </span>
  );
}
