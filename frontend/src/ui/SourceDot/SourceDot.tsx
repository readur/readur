import type { ReactNode } from 'react';
import { sourceHue, type SourceKind } from '../../lib/sourceColor';
import { cx } from '../shared/FieldParts';
import styles from './SourceDot.module.css';

export interface SourceDotProps {
  /** Source id; `null` with no kind means an upload. */
  sourceId: string | null | undefined;
  kind?: SourceKind | string | null;
  /**
   * `dot` is the bare mark (decorative unless `label` is set); `badge` is a tinted pill that
   * shows `children` (the source name) beside the dot.
   */
  variant?: 'dot' | 'badge';
  size?: 'sm' | 'md';
  /** Accessible name for a bare dot. Without it the dot is hidden from assistive tech. */
  label?: string;
  children?: ReactNode;
  className?: string;
}

/** A source's stable colour: as a dot beside its name, or as a tinted badge. */
export function SourceDot({ sourceId, kind, variant = 'dot', size = 'md', label, children, className }: SourceDotProps) {
  const { index } = sourceHue(sourceId, kind);
  if (variant === 'badge') {
    return (
      <span className={cx(styles.badge, styles[size], className)} data-slot={index}>
        <span className={styles.dot} aria-hidden="true" />
        <span className={styles.name}>{children}</span>
      </span>
    );
  }
  return (
    <span
      className={cx(styles.dot, styles[size], className)}
      data-slot={index}
      role={label ? 'img' : undefined}
      aria-label={label}
      aria-hidden={label ? undefined : true}
    />
  );
}
