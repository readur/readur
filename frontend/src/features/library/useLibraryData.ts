import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { LABELS_CHANGED_EVENT, type LabelData } from '../labels';
import {
  fetchLabels,
  fetchMimeTypes,
  fetchRows,
  fetchSources,
  isTooManyResults,
  listParams,
  searchParams,
  type LibraryRow,
  type LibrarySource,
  WATCH_SOURCE_TYPE,
} from './data';
import { ocrState } from './format';
import { mimeTypesFor } from './mime';
import { isSearch, type LibraryQuery } from './urlState';

export type RowsStatus = 'loading' | 'ready' | 'error' | 'tooMany';

/** How often the page re-reads its rows while one of them is still waiting for or in OCR. */
export const LIBRARY_OCR_POLL_MS = 10_000;

const inOcr = (row: Pick<LibraryRow, 'ocr_status'>) => {
  const state = ocrState(row.ocr_status);
  return state === 'pending' || state === 'processing';
};

/** Filter values: labels, connections and the MIME types present in the library. */
export function useFacets() {
  const [labels, setLabels] = useState<LabelData[]>([]);
  const [sources, setSources] = useState<LibrarySource[]>([]);
  const [mimeTypes, setMimeTypes] = useState<string[] | null>(null);
  const [mimeReady, setMimeReady] = useState(false);

  useEffect(() => {
    let live = true;
    fetchLabels()
      .then((l) => live && setLabels(l))
      .catch(() => undefined);
    fetchSources()
      .then((s) => live && setSources(s))
      .catch(() => undefined);
    fetchMimeTypes()
      .then((m) => live && setMimeTypes(m))
      .catch(() => undefined)
      .finally(() => live && setMimeReady(true));
    return () => {
      live = false;
    };
  }, []);

  // Labels made elsewhere (another page's "Save as collection", Settings) show up here too.
  useEffect(() => {
    let live = true;
    const refresh = () => {
      fetchLabels()
        .then((l) => live && setLabels(l))
        .catch(() => undefined);
    };
    window.addEventListener(LABELS_CHANGED_EVENT, refresh);
    return () => {
      live = false;
      window.removeEventListener(LABELS_CHANGED_EVENT, refresh);
    };
  }, []);

  const addLabel = useCallback(
    (label: LabelData) => setLabels((prev) => (prev.some((l) => l.id === label.id) ? prev : [...prev, label])),
    [],
  );

  return { labels, addLabel, sources, mimeTypes, mimeReady };
}

/**
 * The current page of rows for the query. A single-character query is not searched (the list
 * shows unsearched), and only the newest request may update the rows.
 */
export function useRows(query: LibraryQuery, knownMimeTypes: string[] | null, mimeReady: boolean, enabled = true) {
  const [rows, setRows] = useState<LibraryRow[]>([]);
  const [total, setTotal] = useState(0);
  const [status, setStatus] = useState<RowsStatus>('loading');
  const [reloadTick, setReloadTick] = useState(0);
  const latest = useRef(0);
  // Bumped by every load, poll and row patch: a poll's rows apply only if nothing happened since
  // it was sent, so a slow poll cannot bring back older rows or undo a change made in the drawer.
  const changes = useRef(0);

  const mimes = useMemo(() => mimeTypesFor(query.types, knownMimeTypes), [query.types, knownMimeTypes]);
  // Everything that changes the request, as one comparable key.
  const requestKey = useMemo(
    () => JSON.stringify(isSearch(query) ? ['s', searchParams(query, mimes)] : ['l', listParams(query, mimes)]),
    [query, mimes],
  );
  // A Type filter needs the MIME list first, so wait for the facets in that case.
  const waiting = !enabled || (query.types.length > 0 && !mimeReady);

  useEffect(() => {
    if (waiting) return;
    const id = ++latest.current;
    changes.current += 1;
    setStatus('loading');
    fetchRows(query, mimes)
      .then((page) => {
        if (id !== latest.current) return;
        setRows(page.rows);
        setTotal(page.total);
        setStatus('ready');
      })
      .catch((error) => {
        if (id !== latest.current) return;
        setRows([]);
        setTotal(0);
        setStatus(isTooManyResults(error) ? 'tooMany' : 'error');
      });
    // requestKey captures query and mimes.
  }, [requestKey, waiting, reloadTick]);

  // While a row is pending or in OCR, re-read the same page in the background (no loading state)
  // so its status and OCR n/m progress move without a manual refresh.
  const polling = status === 'ready' && !waiting && rows.some(inOcr);
  useEffect(() => {
    if (!polling) return undefined;
    const timer = window.setInterval(() => {
      if (typeof document !== 'undefined' && document.hidden) return;
      const id = ++changes.current;
      fetchRows(query, mimes)
        .then((page) => {
          if (id !== changes.current) return;
          setRows(page.rows);
          setTotal(page.total);
        })
        .catch(() => undefined);
    }, LIBRARY_OCR_POLL_MS);
    return () => window.clearInterval(timer);
    // requestKey captures query and mimes.
  }, [polling, requestKey]);

  const reload = useCallback(() => setReloadTick((n) => n + 1), []);
  const patchRow = useCallback((id: string, patch: Partial<LibraryRow>) => {
    changes.current += 1;
    setRows((prev) => prev.map((r) => (r.id === id ? { ...r, ...patch } : r)));
  }, []);

  return { rows, total, status, reload, patchRow, mimes };
}

/** A row's source as people know it: the connection's name, or "Upload". */
export function useSourceName(sources: readonly LibrarySource[]) {
  const { t } = useTranslation();
  const byId = useMemo(() => new Map(sources.map((s) => [s.id, s.name])), [sources]);
  return useCallback(
    (row: Pick<LibraryRow, 'source_id' | 'source_type'>) => {
      if (!row.source_id) {
        return row.source_type === WATCH_SOURCE_TYPE ? t('library.source.watch', 'Watch folder') : t('library.source.upload', 'Upload');
      }
      return byId.get(row.source_id) ?? row.source_type ?? t('library.source.unknown', 'Connection');
    },
    [byId, t],
  );
}
