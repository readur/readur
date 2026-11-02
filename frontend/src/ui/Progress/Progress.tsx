import { cx } from '../shared/FieldParts';
import styles from './Progress.module.css';

export interface ProgressBarProps {
  /** 0–100; clamped and rounded. */
  value: number;
  label: string;
  /** Show the mono percentage after the bar. */
  showValue?: boolean;
  tone?: 'accent' | 'ok' | 'warn' | 'danger';
  className?: string;
}

/** A single value: thin bar with an optional mono percentage. */
export function ProgressBar({ value, label, showValue = false, tone = 'accent', className }: ProgressBarProps) {
  const pct = Math.max(0, Math.min(100, Math.round(value)));
  return (
    <span className={cx(styles.progress, className)} data-tone={tone}>
      <span
        className={styles.track}
        role="progressbar"
        aria-label={label}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={pct}
      >
        <span className={styles.fill} style={{ transform: `scaleX(${pct / 100})` }} />
      </span>
      {showValue ? (
        <span className={styles.value} aria-hidden="true">
          {pct}%
        </span>
      ) : null}
    </span>
  );
}

export interface OutcomeSegment {
  id: string;
  label: string;
  value: number;
  tone: 'ok' | 'accent' | 'danger' | 'warn' | 'neutral';
}

export interface OutcomeBarProps {
  label: string;
  segments: OutcomeSegment[];
  className?: string;
}

/** One bar split by outcome (done / processing / failed / queued…) with a legend of counts. */
export function OutcomeBar({ label, segments, className }: OutcomeBarProps) {
  const total = segments.reduce((sum, s) => sum + s.value, 0) || 1;
  const summary = `${label}: ${segments.map((s) => `${s.label} ${s.value}`).join(', ')}`;
  return (
    <div className={cx(styles.outcome, className)}>
      <div className={styles.segments} role="img" aria-label={summary}>
        {segments
          .filter((s) => s.value > 0)
          .map((s) => (
            <span key={s.id} data-tone={s.tone} style={{ flexGrow: s.value / total }} />
          ))}
      </div>
      <ul className={styles.legend} aria-hidden="true">
        {segments.map((s) => (
          <li key={s.id} data-tone={s.tone}>
            <span className={styles.swatch} />
            {s.label} <b>{s.value}</b>
          </li>
        ))}
      </ul>
    </div>
  );
}
