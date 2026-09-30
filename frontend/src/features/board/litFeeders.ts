/**
 * Feeds the change-tracking store with document events.
 *
 * Notifications carry no document ids (only a file name and, for batches, a count), so the
 * reliable source of "what changed" is a diff of the newest documents against what was seen
 * before. `syncDocuments` does that diff; the Board calls it on every poll and `useLitFeeders`
 * calls it when an upload / OCR notification arrives.
 */
import { useEffect, useRef } from 'react';
import { useNotifications } from '../../contexts/NotificationContext';
import { documentService } from '../../services/api';
import { markLit } from './litStore';
import type { BoardDocument } from './types';

/** id -> last seen ocr_status of the newest documents. */
const seen = new Map<string, string | undefined>();
let baselined = false;
/** Newest document creation time (server clock, ms) already accounted for; persisted across reloads. */
let lastSeen: number | undefined;
const MAX_SEEN = 500;
const LAST_SEEN_KEY = 'readur.board.lastSeen.v1';

function readLastSeen(): number | undefined {
  try {
    const raw = window.localStorage.getItem(LAST_SEEN_KEY);
    const n = raw === null ? NaN : Number(raw);
    return Number.isFinite(n) ? n : undefined;
  } catch {
    return undefined;
  }
}

function writeLastSeen(value: number): void {
  try {
    window.localStorage.setItem(LAST_SEEN_KEY, String(value));
  } catch {
    /* storage unavailable: the marker lasts for this session only */
  }
}

/** Forgets what was seen (tests, sign-out). */
export function resetDocumentBaseline(): void {
  seen.clear();
  baselined = false;
  lastSeen = undefined;
}

/**
 * Compares `docs` with what was seen before and marks the differences:
 * an unseen document created after the last-seen time is 'new'; a known one whose OCR status became
 * completed is 'changed', failed is 'failed'. The last-seen time is stored, so documents that
 * arrived while the app was closed are marked on the first sync after a reload. With no stored
 * time (first ever run) nothing is marked, unless `newerThan` is given.
 */
export function syncDocuments(docs: BoardDocument[], newerThan?: number): void {
  if (!baselined && lastSeen === undefined) lastSeen = readLastSeen();
  const threshold = lastSeen ?? newerThan;
  let newest = lastSeen;
  for (const doc of docs) {
    const created = doc.created_at ? Date.parse(doc.created_at) : NaN;
    if (!Number.isNaN(created) && (newest === undefined || created > newest)) newest = created;
    if (!seen.has(doc.id)) {
      // Entering the top of the list (for example after a deletion) is not an arrival.
      if (threshold !== undefined && !Number.isNaN(created) && created > threshold) markLit('document', doc.id, 'new');
    } else if (seen.get(doc.id) !== doc.ocr_status) {
      if (doc.ocr_status === 'completed') markLit('document', doc.id, 'changed');
      else if (doc.ocr_status === 'failed') markLit('document', doc.id, 'failed');
    }
    seen.set(doc.id, doc.ocr_status);
  }
  baselined = true;
  if (newest !== undefined && newest !== lastSeen) {
    lastSeen = newest;
    writeLastSeen(newest);
  }
  while (seen.size > MAX_SEEN) seen.delete(seen.keys().next().value as string);
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
      .then((res) => syncDocuments(res.data?.documents ?? [], startedAt.current))
      .catch(() => {
        /* the next poll on the Board catches up */
      });
  }, [notifications]);
}
