/**
 * The one vocabulary for changed rows, shared by every surface (Board, Library, Intake):
 * - NEW: the item arrived since the user's last visit;
 * - CHANGED: something happened to it since they last saw it (OCR finished, OCR failed, a
 *   connection errored). A failure additionally shows the ▲ FAILED status mark in its row.
 * The same stored reason therefore produces the same tag word wherever the row appears.
 */
import type { LitReason } from './litStore';

export type LitTag = 'new' | 'changed';

export function litTagOf(reason: LitReason | string | null | undefined): LitTag | null {
  if (!reason) return null;
  return reason === 'new' ? 'new' : 'changed';
}
