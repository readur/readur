/**
 * Remembers which failure events have already been flagged, so a failure is marked for the user
 * once and stays cleared after they acknowledge it, even though every reload lists it again.
 */
import { markLit, type LitKind } from '../../board/litStore';

const CAP = 2000;

function read(key: string): string[] {
  try {
    const raw = window.localStorage.getItem(key);
    const parsed = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? parsed.filter((v): v is string => typeof v === 'string') : [];
  } catch {
    return [];
  }
}

function write(key: string, values: string[]): void {
  try {
    window.localStorage.setItem(key, JSON.stringify(values.slice(-CAP)));
  } catch {
    /* storage full or blocked: events will be flagged again next time */
  }
}

export const SOURCE_EVENTS_KEY = 'readur.intake.sourceFailures.v1';
export const DOCUMENT_EVENTS_KEY = 'readur.intake.failedDocuments.v1';

/**
 * Flags each event (`eventKey`) not seen before by marking `(kind, id)` as failed.
 * Returns the number of newly flagged events.
 */
export function flagNewFailures(
  storageKey: string,
  kind: LitKind,
  events: ReadonlyArray<{ id: string; eventKey: string }>,
): number {
  if (events.length === 0) return 0;
  const seen = read(storageKey);
  const known = new Set(seen);
  let added = 0;
  for (const e of events) {
    if (known.has(e.eventKey)) continue;
    known.add(e.eventKey);
    seen.push(e.eventKey);
    markLit(kind, e.id, 'failed');
    added += 1;
  }
  if (added > 0) write(storageKey, seen);
  return added;
}
