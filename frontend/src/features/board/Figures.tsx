import type { ReactNode } from 'react';
import { cx } from '../../ui/shared/FieldParts';
import styles from './Board.module.css';

/**
 * One row of labelled figures inside a pass-strip segment: small condensed labels above mono
 * values, cells sized to their values and separated by hairlines. Not a grid of tiles: the cells
 * run as one line and only wrap, as a line, when the row is too narrow.
 */
export function Figures({ 'aria-label': label, children }: { 'aria-label': string; children: ReactNode }) {
  return (
    <div role="group" aria-label={label} className={styles.figuresWrap}>
      <dl className={styles.figures}>{children}</dl>
    </div>
  );
}

export interface FigureProps {
  label: ReactNode;
  children: ReactNode;
  /** The strip's most important number, set larger. One per strip. */
  lead?: boolean;
  /** A mark or a control rather than a number. */
  plain?: boolean;
}

export function Figure({ label, children, lead, plain }: FigureProps) {
  return (
    <div className={cx(styles.figure, lead && styles.figureLead)}>
      <dt className={styles.figureLabel}>{label}</dt>
      <dd className={cx(styles.figureValue, plain && styles.figurePlain)}>{children}</dd>
    </div>
  );
}
