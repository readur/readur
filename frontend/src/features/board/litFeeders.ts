/**
 * Feeds the change-tracking store with document events.
 *
 * Notifications carry no document ids (only a file name and, for batches, a count), so the
 * reliable source of "what changed" is a diff of the newest documents against what was seen
 * before. `syncDocuments` does that diff; the Board calls it on every poll and `useLitFeeders`
 * calls it when an upload / OCR notification arrives.
 */
import { useEffect, useRef, useSyncExternalStore } from 'react';
import { useNotifications } from '../../contexts/NotificationContext';
import { documentService } from '../../services/api';
import { litReason, markLit } from './litStore';
import type { BoardDocument } from './types';

/** id -> last seen ocr_status of the newest documents. */
const seen = new Map<string, string | undefined>();
let baselined = false;
/** Newest document creation time (server clock, ms) already accounted for; persisted across reloads. */
let lastSeen: number | undefined;
/** Library size at the last sync, to size a batch of arrivals beyond the ten fetched rows. */
let lastTotal: number | undefined;
/** Documents that arrived as part of a bulk import: they never light up individually. */
const quiet = new Set<string>();
const MAX_SEEN = 500;
const LAST_SEEN_KEY = 'readur.board.lastSeen.v1';
const LAST_TOTAL_KEY = 'readur.board.lastTotal.v1';
export const BULK_ARRIVALS_KEY = 'readur.board.bulkArrivals.v1';
/** More arrivals than this in one sync light a single summary instead of individual rows. */
export const BULK_THRESHOLD = 25;

function readNumber(key: string): number | undefined {
  try {
    const raw = window.localStorage.getItem(key);
    const n = raw === null ? NaN : Number(raw);
    return Number.isFinite(n) ? n : undefined;
  } catch {
    return undefined;
  }
}

function writeNumber(key: string, value: number): void {
  try {
    window.localStorage.setItem(key, String(value));
  } catch {
    /* storage unavailable: the value lasts for this session only */
  }
}

/* The bulk-arrivals summary: a count of documents that arrived in bulk and have not been looked at. */
const bulkListeners = new Set<() => void>();
let bulkCount: number | undefined;

function readBulk(): number {
  if (bulkCount === undefined) bulkCount = readNumber(BULK_ARRIVALS_KEY) ?? 0;
  return bulkCount;
}

function setBulk(value: number): void {
  if (readBulk() === value) return;
  bulkCount = value;
  if (value > 0) writeNumber(BULK_ARRIVALS_KEY, value);
  else {
    try {
      window.localStorage.removeItem(BULK_ARRIVALS_KEY);
    } catch {
      /* storage unavailable */
    }
  }
  bulkListeners.forEach((l) => l());
}

/** Clears the "N new documents" summary (it was seen, or the user marked everything seen). */
export function clearBulkArrivals(): void {
  setBulk(0);
}

/** Number of documents in the pending bulk-arrival summary; 0 when there is none. */
export function useBulkArrivals(): number {
  return useSyncExternalStore(
    (listener) => {
      bulkListeners.add(listener);
      return () => bulkListeners.delete(listener);
    },
    readBulk,
    () => 0,
  );
}

/** Forgets what was seen (tests, sign-out). */
export function resetDocumentBaseline(): void {
  seen.clear();
  quiet.clear();
  baselined = false;
  lastSeen = undefined;
  lastTotal = undefined;
  bulkCount = undefined;
  bulkListeners.forEach((l) => l());
}

/**
 * Compares `docs` with what was seen before and marks the differences:
 * - an unseen document created after the last-seen time is 'new' ('failed' if its OCR already failed);
 * - a known one whose OCR failed is 'failed';
 * - a known one whose OCR completed is 'changed', unless it is still unseen as 'new' (an arrival
 *   stays NEW until the user has seen it).
 * The last-seen time is stored, so documents that arrived while the app was closed are marked on
 * the first sync after a reload. With no stored time (first ever run) nothing is marked, unless
 * `newerThan` is given. When `total` (the library size) shows that more than BULK_THRESHOLD
 * documents arrived since the last sync, none of them is lit: a single "N new documents" summary
 * is raised instead (see useBulkArrivals).
 */
export function syncDocuments(docs: BoardDocument[], newerThan?: number, total?: number): void {
  if (!baselined && lastSeen === undefined) lastSeen = readNumber(LAST_SEEN_KEY);
  if (!baselined && lastTotal === undefined) lastTotal = readNumber(LAST_TOTAL_KEY);
  const threshold = lastSeen ?? newerThan;
  let newest = lastSeen;
  const arrivals: string[] = [];
  for (const doc of docs) {
    const created = doc.created_at ? Date.parse(doc.created_at) : NaN;
    if (!Number.isNaN(created) && (newest === undefined || created > newest)) newest = created;
    if (!seen.has(doc.id)) {
      // Entering the top of the list (for example after a deletion) is not an arrival.
      if (threshold !== undefined && !Number.isNaN(created) && created > threshold) arrivals.push(doc.id);
    } else if (seen.get(doc.id) !== doc.ocr_status && !quiet.has(doc.id)) {
      if (doc.ocr_status === 'failed') markLit('document', doc.id, 'failed');
      else if (doc.ocr_status === 'completed' && litReason('document', doc.id) !== 'new') {
        markLit('document', doc.id, 'changed');
      }
    }
    seen.set(doc.id, doc.ocr_status);
  }

  const arrived =
    threshold !== undefined && total !== undefined && lastTotal !== undefined
      ? Math.max(total - lastTotal, arrivals.length)
      : arrivals.length;
  if (arrived > BULK_THRESHOLD) {
    arrivals.forEach((id) => quiet.add(id));
    setBulk(readBulk() + arrived);
  } else {
    // An arrival whose OCR already failed carries the failure, so it reads CHANGED (with ▲ FAILED)
    // here exactly as it does in Needs attention.
    const failed = new Set(docs.filter((d) => d.ocr_status === 'failed').map((d) => d.id));
    arrivals.forEach((id) => markLit('document', id, failed.has(id) ? 'failed' : 'new'));
  }

  baselined = true;
  if (newest !== undefined && newest !== lastSeen) {
    lastSeen = newest;
    writeNumber(LAST_SEEN_KEY, newest);
  }
  if (total !== undefined && total !== lastTotal) {
    lastTotal = total;
    writeNumber(LAST_TOTAL_KEY, total);
  }
  while (seen.size > MAX_SEEN) seen.delete(seen.keys().next().value as string);
  while (quiet.size > MAX_SEEN) quiet.delete(quiet.keys().next().value as string);
}

/**
 * Subscribes to NotificationContext. A notification that names a document marks it directly;
 * any other notification refreshes the newest documents and diffs them. Mount once, in the shell.
 */
export function useLitFeeders(): void {
  const { notifications } = useNotifications();
  const handled = useRef<Set<string> | null>(null);
  const startedAt = useRef(Date.now() - 60_000);

  useEffect(() => {
    if (handled.current === null) {
      // Notifications that exist at mount time are history, not events.
      handled.current = new Set(notifications.map((n) => n.id));
      return;
    }
    const fresh = notifications.filter((n) => !handled.current!.has(n.id));
    if (fresh.length === 0) return;
    fresh.forEach((n) => handled.current!.add(n.id));

    let needsRefresh = false;
    for (const n of fresh) {
      const documentId = n.metadata?.documentId;
      if (documentId !== undefined && documentId !== null) {
        markLit('document', String(documentId), n.type === 'error' ? 'failed' : 'changed');
      } else {
        needsRefresh = true;
      }
    }
    if (!needsRefresh) return;
    // Started inside a promise so that a synchronous failure is handled like a rejected request.
    Promise.resolve()
      .then(() => documentService.listWithPagination(10, 0))
      .then((res) => {
        const data = res.data as { documents?: BoardDocument[]; pagination?: { total?: number } } | undefined;
        syncDocuments(data?.documents ?? [], startedAt.current, data?.pagination?.total);
      })
      .catch(() => {
        /* the next poll on the Board catches up */
      });
  }, [notifications]);
}
