import type { CSSProperties } from 'react';
import { sourceHue } from '../../lib/sourceColor';
import { WATCH_SOURCE_TYPE, type LibraryRow } from './data';
import { SourceBadge as UiSourceBadge } from '../../ui/SourceBadge';

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
  /** `dot` renders the icon tile and the name; `chip` sits both on an outlined pill. */
  variant?: 'dot' | 'chip';
}

/** Where a document came from: the ui SourceBadge for a library row (tile by default, chip in dense text). */
export function SourceBadge({ row, name, variant = 'dot' }: SourceBadgeProps) {
  const kind = hueKind(row);
  return (
    <UiSourceBadge
      sourceId={row.source_id}
      kind={kind}
      type={row.source_type ?? kind}
      name={name}
      variant={variant === 'chip' ? 'chip' : 'tile'}
    />
  );
}
