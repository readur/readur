import { useCallback, useEffect, useRef, useState } from 'react';
import { documentService, type Document } from '../../../services/api';
import { httpStatus, isOcrActive } from '../format';

/** How often the page re-reads the document while OCR is queued or running. */
export const POLL_INTERVAL_MS = 10_000;

export type DocumentLoadState = 'loading' | 'ready' | 'notFound' | 'error';

export interface UseDocumentResult {
  document: Document | null;
  state: DocumentLoadState;
  /** Re-read the document without showing the loading state. */
  refresh: () => Promise<void>;
  /** Re-read the document from scratch (used by the error state's retry). */
  reload: () => void;
  setDocument: (doc: Document) => void;
}

/**
 * Loads one document and keeps it fresh: polls every 10s while OCR is pending or processing
 * (or while `keepPolling` is set, e.g. right after a retry was queued) and stops once it settles.
 */
export function useDocument(id: string | undefined, keepPolling = false): UseDocumentResult {
  const [document, setDocument] = useState<Document | null>(null);
  const [state, setState] = useState<DocumentLoadState>('loading');
  const [attempt, setAttempt] = useState(0);
  const currentId = useRef(id);
  currentId.current = id;

  useEffect(() => {
    let cancelled = false;
    setDocument(null);
    if (!id) {
      setState('notFound');
      return undefined;
    }
    setState('loading');
    documentService
      .getById(id)
      .then((res) => {
        if (cancelled) return;
        setDocument(res.data);
        setState('ready');
      })
      .catch((err) => {
        if (cancelled) return;
        setState(httpStatus(err) === 404 ? 'notFound' : 'error');
      });
    return () => {
      cancelled = true;
    };
  }, [id, attempt]);

  const refresh = useCallback(async () => {
    const requested = currentId.current;
    if (!requested) return;
    try {
      const res = await documentService.getById(requested);
      if (currentId.current === requested) setDocument(res.data);
    } catch {
      // A failed background refresh keeps the last good copy on screen.
    }
  }, []);

  const reload = useCallback(() => setAttempt((n) => n + 1), []);

  const shouldPoll = state === 'ready' && (keepPolling || isOcrActive(document?.ocr_status));

  useEffect(() => {
    if (!shouldPoll) return undefined;
    const timer = setInterval(() => {
      void refresh();
    }, POLL_INTERVAL_MS);
    return () => clearInterval(timer);
  }, [shouldPoll, refresh]);

  return { document, state, refresh, reload, setDocument };
}
