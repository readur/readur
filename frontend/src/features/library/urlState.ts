import { useCallback, useMemo } from 'react';
import { useSearchParams } from 'react-router-dom';
import type { SearchMode } from '../../types/generated';
import { TYPE_GROUPS, type TypeGroup } from './mime';

export const SORT_FIELDS = ['created_at', 'updated_at', 'filename', 'file_size', 'ocr_status', 'mime_type'] as const;
export type SortField = (typeof SORT_FIELDS)[number];
export type SortOrder = 'asc' | 'desc';

export const STATUSES = ['pending', 'processing', 'completed', 'failed'] as const;
export type OcrStatus = (typeof STATUSES)[number];

export const PAGE_SIZES = [25, 50, 100] as const;
export const SEARCH_MODES: readonly SearchMode[] = ['simple', 'phrase', 'boolean', 'fuzzy'];

/** The `source` value for documents added by upload rather than a connection. */
export const UPLOADED = 'uploaded';
/** The `source` value for documents picked up from the watch folder. */
export const WATCHED = 'watch';

export const DEFAULT_SORT: SortField = 'created_at';
export const DEFAULT_ORDER: SortOrder = 'desc';
export const DEFAULT_SIZE = 50;
/** `sort=relevance`: search results by best match instead of newest first. */
export const RELEVANCE = 'relevance';
/** Search needs at least this many characters. */
export const MIN_QUERY = 2;

/** Everything the Library shows is derived from this, and it all lives in the URL. */
export interface LibraryQuery {
  q: string;
  sort: SortField;
  order: SortOrder;
  /** True when the URL names a sort field. */
  sortExplicit: boolean;
  /** Search results in relevance order (`sort=relevance`) rather than by date. */
  relevance: boolean;
  types: TypeGroup[];
  labels: string[];
  status: OcrStatus | null;
  /** Connection ids, or any of `UPLOADED` / `WATCHED` (never both kinds together). */
  sources: string[];
  /** YYYY-MM-DD, inclusive. */
  from: string | null;
  to: string | null;
  page: number;
  size: number;
  /** Search syntax; unset means simple. */
  mode: SearchMode | null;
}

const list = (value: string | null) =>
  (value ?? '')
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean);

const oneOf = <T extends string>(value: string | null, allowed: readonly T[]): T | null =>
  value !== null && (allowed as readonly string[]).includes(value) ? (value as T) : null;

const day = (value: string | null) => (value && /^\d{4}-\d{2}-\d{2}$/.test(value) ? value : null);

export function parseQuery(params: URLSearchParams): LibraryQuery {
  const sort = oneOf(params.get('sort'), SORT_FIELDS);
  const relevance = params.get('sort') === RELEVANCE;
  // `label=<id>` is the collection link from the sidebar; `labels=` the filter's own list.
  const labels = [...list(params.get('labels')), ...list(params.get('label'))];
  const size = Number(params.get('size'));
  const page = Math.floor(Number(params.get('page')));
  return {
    q: params.get('q') ?? '',
    sort: sort ?? DEFAULT_SORT,
    order: oneOf(params.get('order'), ['asc', 'desc'] as const) ?? DEFAULT_ORDER,
    sortExplicit: sort !== null,
    relevance: relevance && sort === null,
    types: list(params.get('type')).filter((t): t is TypeGroup => (TYPE_GROUPS as readonly string[]).includes(t)),
    labels: Array.from(new Set(labels)),
    status: oneOf(params.get('status'), STATUSES),
    sources: list(params.get('source')),
    from: day(params.get('from')),
    to: day(params.get('to')),
    page: Number.isFinite(page) && page > 1 ? page : 1,
    size: (PAGE_SIZES as readonly number[]).includes(size) ? size : DEFAULT_SIZE,
    mode: oneOf(params.get('mode'), SEARCH_MODES),
  };
}

/** The URL for a query; defaults are left out so URLs stay short and shareable. */
export function toParams(query: LibraryQuery): URLSearchParams {
  const p = new URLSearchParams();
  if (query.q) p.set('q', query.q);
  if (query.relevance) {
    p.set('sort', RELEVANCE);
  } else if (query.sortExplicit) {
    p.set('sort', query.sort);
    p.set('order', query.order);
  }
  if (query.types.length) p.set('type', query.types.join(','));
  if (query.labels.length) p.set('labels', query.labels.join(','));
  if (query.status) p.set('status', query.status);
  if (query.sources.length) p.set('source', query.sources.join(','));
  if (query.from) p.set('from', query.from);
  if (query.to) p.set('to', query.to);
  if (query.mode && query.mode !== 'simple') p.set('mode', query.mode);
  if (query.page > 1) p.set('page', String(query.page));
  if (query.size !== DEFAULT_SIZE) p.set('size', String(query.size));
  return p;
}

/** Every URL key the Library owns; anything else (an open drawer, say) belongs to someone else. */
export const LIBRARY_KEYS = ['q', 'sort', 'order', 'type', 'labels', 'label', 'status', 'source', 'from', 'to', 'page', 'size', 'mode'] as const;

/** `query`'s params followed by every param of `prev` the Library does not own, as they were. */
export function withQuery(prev: URLSearchParams, query: LibraryQuery): URLSearchParams {
  const next = toParams(query);
  const owned: readonly string[] = LIBRARY_KEYS;
  for (const [key, value] of prev) if (!owned.includes(key)) next.append(key, value);
  return next;
}

export type QueryPatch = Partial<Omit<LibraryQuery, 'sortExplicit'>>;

/** The patch that removes every filter (not the search text). */
export const NO_FILTERS: QueryPatch = { types: [], labels: [], status: null, sources: [], from: null, to: null };

export function hasFilters(query: LibraryQuery): boolean {
  return Boolean(
    query.types.length || query.labels.length || query.status || query.sources.length || query.from || query.to,
  );
}

/** Search mode: a query long enough to send. */
export const isSearch = (query: LibraryQuery) => query.q.trim().length >= MIN_QUERY;

/**
 * URL-backed Library state. Any change other than the page itself returns to page 1. Typing in
 * the search field replaces the history entry, so Back leaves the search instead of undoing it
 * one keystroke at a time.
 */
export function useLibraryQuery() {
  const [params, setParams] = useSearchParams();
  // Keyed on the Library's own params only, so an open drawer does not make a new query.
  const key = toParams(parseQuery(params)).toString();
  const query = useMemo(() => parseQuery(new URLSearchParams(key)), [key]);

  const update = useCallback(
    (patch: QueryPatch) => {
      setParams(
        (prev) => {
          const current = parseQuery(prev);
          const next: LibraryQuery = { ...current, ...patch };
          if ('sort' in patch || 'order' in patch) {
            next.sortExplicit = true;
            if (!('relevance' in patch)) next.relevance = false;
          }
          if (patch.relevance) next.sortExplicit = false;
          if (!('page' in patch)) next.page = 1;
          return withQuery(prev, next);
        },
        { replace: isTypingOnly(patch, query) },
      );
    },
    [setParams, query],
  );

  const clearFilters = useCallback(() => update(NO_FILTERS), [update]);

  return { query, update, clearFilters };
}

function isTypingOnly(patch: QueryPatch, current: LibraryQuery): boolean {
  const keys = Object.keys(patch);
  if (keys.length !== 1 || keys[0] !== 'q') return false;
  // Starting or ending a search is its own history step.
  return Boolean(current.q) && Boolean(patch.q);
}
