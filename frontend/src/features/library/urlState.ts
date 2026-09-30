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

export const DEFAULT_SORT: SortField = 'created_at';
export const DEFAULT_ORDER: SortOrder = 'desc';
export const DEFAULT_SIZE = 50;
/** Search needs at least this many characters. */
export const MIN_QUERY = 2;

/** Everything the Library shows is derived from this, and it all lives in the URL. */
export interface LibraryQuery {
  q: string;
  sort: SortField;
  order: SortOrder;
  /** True when the URL names a sort; search otherwise orders by relevance. */
  sortExplicit: boolean;
  types: TypeGroup[];
  labels: string[];
  status: OcrStatus | null;
  /** Connection ids, or `[UPLOADED]`. */
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
  const size = Number(params.get('size'));
  const page = Math.floor(Number(params.get('page')));
  return {
    q: params.get('q') ?? '',
    sort: sort ?? DEFAULT_SORT,
    order: oneOf(params.get('order'), ['asc', 'desc'] as const) ?? DEFAULT_ORDER,
    sortExplicit: sort !== null,
    types: list(params.get('type')).filter((t): t is TypeGroup => (TYPE_GROUPS as readonly string[]).includes(t)),
    labels: list(params.get('labels')),
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
  if (query.sortExplicit) {
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

export type QueryPatch = Partial<Omit<LibraryQuery, 'sortExplicit'>>;

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
  const key = params.toString();
  const query = useMemo(() => parseQuery(new URLSearchParams(key)), [key]);

  const update = useCallback(
    (patch: QueryPatch) => {
      setParams(
        (prev) => {
          const current = parseQuery(prev);
          const next: LibraryQuery = { ...current, ...patch };
          if ('sort' in patch || 'order' in patch) next.sortExplicit = true;
          if (!('page' in patch)) next.page = 1;
          return toParams(next);
        },
        { replace: isTypingOnly(patch, query) },
      );
    },
    [setParams, query],
  );

  /** Back to the default order (relevance while searching). */
  const clearSort = useCallback(() => {
    setParams((prev) => {
      const next = parseQuery(prev);
      next.sortExplicit = false;
      next.sort = DEFAULT_SORT;
      next.order = DEFAULT_ORDER;
      next.page = 1;
      return toParams(next);
    });
  }, [setParams]);

  const clearFilters = useCallback(() => {
    update({ types: [], labels: [], status: null, sources: [], from: null, to: null });
  }, [update]);

  return { query, update, clearSort, clearFilters };
}

function isTypingOnly(patch: QueryPatch, current: LibraryQuery): boolean {
  const keys = Object.keys(patch);
  if (keys.length !== 1 || keys[0] !== 'q') return false;
  // Starting or ending a search is its own history step.
  return Boolean(current.q) && Boolean(patch.q);
}
