import type { ReactNode } from 'react';
import { cx } from '../shared/FieldParts';
import { TruncatedText } from './TruncatedText';
import styles from './Pass.module.css';

export interface PassProps {
  children: ReactNode;
  /** `header` renders large values for page headers. */
  variant?: 'default' | 'header';
  'aria-label'?: string;
  className?: string;
}

/**
 * Segmented grid of labelled cells separated by 1px rules. Rendered as a description list
 * (in a named group when `aria-label` is given);
 * wraps to two columns when its container is narrower than 480px, where a last odd cell takes the
 * whole row.
 */
export function Pass({ children, variant = 'default', className, 'aria-label': ariaLabel }: PassProps) {
  return (
    <div
      className={cx(styles.container, className)}
      role={ariaLabel ? 'group' : undefined}
      aria-label={ariaLabel}
    >
      <dl className={cx(styles.pass, variant === 'header' && styles.header)}>
        {children}
      </dl>
    </div>
  );
}

export interface PassCellProps {
  label: ReactNode;
  children: ReactNode;
  /** Render the value in the data face (counts, sizes, dates, IDs). */
  mono?: boolean;
  /** Number of grid tracks this cell spans. */
  span?: number;
  /**
   * A long value (a timestamp, a path): takes a full row when the container is narrower than
   * 400px instead of being cut. Callers keep the cells before it paired.
   */
  wide?: boolean;
  className?: string;
}

export function PassCell({ label, children, mono, span, wide, className }: PassCellProps) {
  const style = span && span > 1 ? { gridColumn: `span ${span}` } : undefined;
  const isText = typeof children === 'string' || typeof children === 'number';
  return (
    <div className={cx(styles.cell, wide && styles.wide, className)} style={style}>
      <dt className={styles.label}>{label}</dt>
      <dd className={cx(styles.value, mono && styles.mono)}>
        {isText ? <TruncatedText>{children}</TruncatedText> : children}
      </dd>
    </div>
  );
}
