/**
 * A stable colour per document source. Uploads and the watch folder have fixed slots; every
 * configured source hashes its id into the remaining six, so the same source keeps the same hue
 * on every screen and across reloads.
 */

export type SourceKind = 'watch' | 'upload' | 'webdav' | 's3' | 'local' | 'local_folder';

export interface SourceHue {
  /** CSS custom property for the hue, e.g. `--src-4`. */
  varName: string;
  /** CSS custom property for its tint, e.g. `--src-4-soft`. */
  softVarName: string;
  /** Slot 1–8. */
  index: number;
}

export const UPLOAD_SLOT = 1;
export const WATCH_SLOT = 2;
const FIRST_SOURCE_SLOT = 3;
const SOURCE_SLOTS = 6;

/** FNV-1a, 32-bit: small, fast and stable across browsers. */
function hash(text: string): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < text.length; i += 1) {
    h ^= text.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  return h >>> 0;
}

function slot(index: number): SourceHue {
  return { index, varName: `--src-${index}`, softVarName: `--src-${index}-soft` };
}

/**
 * The hue for a source. `kind` 'upload' or 'watch' (or an id of exactly "upload" / "watch", as
 * the arrivals rows use) picks the fixed slot; a missing id with no kind counts as an upload.
 */
export function sourceHue(sourceId: string | null | undefined, kind?: SourceKind | string | null): SourceHue {
  if (kind === 'upload' || sourceId === 'upload') return slot(UPLOAD_SLOT);
  if (kind === 'watch' || sourceId === 'watch') return slot(WATCH_SLOT);
  if (!sourceId) return slot(UPLOAD_SLOT);
  return slot(FIRST_SOURCE_SLOT + (hash(sourceId) % SOURCE_SLOTS));
}

/** `var(--src-n)` for inline custom-property wiring, e.g. `style={{ '--dot': sourceColorVar(id) }}`. */
export function sourceColorVar(sourceId: string | null | undefined, kind?: SourceKind | string | null): string {
  return `var(${sourceHue(sourceId, kind).varName})`;
}
