import type { CSSProperties } from 'react';
import { sourceHue } from '../../lib/sourceColor';
import { WATCH_SOURCE_TYPE, type LibraryRow } from './data';
import styles from './SourceBadge.module.css';

/** The colour slot family for a row: uploads, the watch folder, or the connection by id. */
export function hueKind(row: Pick<LibraryRow, 'source_id' | 'source_type'>): string | undefined {
  if (row.source_id) return row.source_type ?? undefined;
  return row.source_type === WATCH_SOURCE_TYPE ? 'watch' : 'upload';
}

/** The CSS custom properties that colour a badge or dot for a row's source. */
export function sourceStyle(row: Pick<LibraryRow, 'source_id' | 'source_type'>): CSSProperties {
  const hue = sourceHue(row.source_id, hueKind(row));
  return { '--src': `var(${hue.varName})`, '--src-soft': `var(${hue.softVarName})` } as CSSProperties;
}

interface SourceBadgeProps {
  row: Pick<LibraryRow, 'source_id' | 'source_type'>;
  name: string;
  /** `dot` is the colour dot and the name as plain text; `chip` sits the name on a tint. */
  variant?: 'dot' | 'chip';
}

/** Where a document came from: its source colour plus the source's name (never colour alone). */
export function SourceBadge({ row, name, variant = 'dot' }: SourceBadgeProps) {
  return (
    <span className={variant === 'chip' ? styles.chip : styles.badge} style={sourceStyle(row)}>
      <span className={styles.dot} aria-hidden="true" />
      <span className={styles.name}>{name}</span>
    </span>
  );
}
