import api, { documentService, searchService } from '../../services/api';
import type { DocumentListParams, EnhancedSearchParams } from '../../services/api';
import { labelService } from '../../services/api/labels';
import type { DocumentResponse, EnhancedDocumentResponse, MonthCount, SearchSnippet } from '../../types/generated';
import { toLabelData, type LabelData } from '../labels';
import { dayBoundary } from './format';
import { UPLOADED, WATCHED, isSearch, type LibraryQuery } from './urlState';

/** One board row: the fields shared by the list and search responses. */
export interface LibraryRow {
  id: string;
  filename: string;
  original_filename: string;
  file_size: number;
  mime_type: string;
  labels: LabelData[];
  created_at: string;
  updated_at: string;
  source_id: string | null;
  source_type: string | null;
  has_ocr_text: boolean;
  ocr_status: string | null;
  ocr_confidence: number | null;
  ocr_progress_current?: number;
  ocr_progress_total?: number;
  snippets?: SearchSnippet[];
}

/** `source_type` of documents picked up from the watch folder (they have no `source_id`). */
export const WATCH_SOURCE_TYPE = 'watch_folder';
/** `source_type` of documents uploaded through the app. */
export const UPLOAD_SOURCE_TYPE = 'direct_upload';

export interface LibrarySource {
  id: string;
  name: string;
  source_type: string;
}

export interface RowsPage {
  rows: LibraryRow[];
  total: number;
}

export function toRow(doc: DocumentResponse | EnhancedDocumentResponse): LibraryRow {
  return {
    id: doc.id,
    filename: doc.filename,
    original_filename: doc.original_filename,
    file_size: doc.file_size,
    mime_type: doc.mime_type,
    labels: (doc.labels ?? []).map(toLabelData),
    created_at: doc.created_at,
    updated_at: doc.updated_at,
    source_id: doc.source_id ?? null,
    source_type: doc.source_type ?? null,
    has_ocr_text: doc.has_ocr_text,
    ocr_status: doc.ocr_status,
    ocr_confidence: doc.ocr_confidence,
    ocr_progress_current: doc.ocr_progress_current,
    ocr_progress_total: doc.ocr_progress_total,
    snippets: 'snippets' in doc ? doc.snippets : undefined,
  };
}

export const displayName = (row: Pick<LibraryRow, 'original_filename' | 'filename'>) =>
  row.original_filename || row.filename;

/** Filters shared by the list and search requests. */
function filterParams(query: LibraryQuery, mimeTypes: string[]) {
  // Uploads and the watch folder are source types; the API cannot mix them with connection ids.
  const kinds = [
    ...(query.sources.includes(UPLOADED) ? [UPLOAD_SOURCE_TYPE] : []),
    ...(query.sources.includes(WATCHED) ? [WATCH_SOURCE_TYPE] : []),
  ];
  const connections = query.sources.filter((s) => s !== UPLOADED && s !== WATCHED);
  const from = query.from ? dayBoundary(query.from, 'start') : null;
  // An end before the start is rejected by the API; the Added panel flags it, so ignore it here.
  const inverted = Boolean(query.from && query.to && query.from > query.to);
  const to = query.to && !inverted ? dayBoundary(query.to, 'end') : null;
  return {
    limit: query.size,
    offset: (query.page - 1) * query.size,
    ocr_status: query.status ?? undefined,
    mime_types: mimeTypes.length ? mimeTypes : undefined,
    label_ids: query.labels.length ? query.labels : undefined,
    source_ids: kinds.length ? undefined : connections.length ? connections : undefined,
    source_types: kinds.length ? kinds : undefined,
    created_from: from ?? undefined,
    created_to: to ?? undefined,
  };
}

export function listParams(query: LibraryQuery, mimeTypes: string[]): DocumentListParams {
  return { ...filterParams(query, mimeTypes), sort_by: query.sort, sort_order: query.order };
}

export function searchParams(query: LibraryQuery, mimeTypes: string[]): EnhancedSearchParams {
  return {
    ...filterParams(query, mimeTypes),
    query: query.q.trim(),
    include_snippets: true,
    search_mode: query.mode ?? undefined,
    // Newest first unless the user asked for the best matches first.
    ...(query.relevance ? {} : { sort_by: query.sort, sort_order: query.order }),
  };
}

/**
 * Params for the month histogram: the same search and filters, but every date, so the chart keeps
 * showing the whole spread of matches while a month is picked.
 */
export function timelineParams(query: LibraryQuery, mimeTypes: string[]): EnhancedSearchParams {
  const filters = filterParams(query, mimeTypes);
  return {
    query: query.q.trim(),
    search_mode: query.mode ?? undefined,
    ocr_status: filters.ocr_status,
    mime_types: filters.mime_types,
    label_ids: filters.label_ids,
    source_ids: filters.source_ids,
    source_types: filters.source_types,
  };
}

export async function fetchTimeline(query: LibraryQuery, mimeTypes: string[]): Promise<MonthCount[]> {
  const res = await searchService.getTimeline(timelineParams(query, mimeTypes));
  return Array.isArray(res.data) ? res.data : [];
}

/** The API returns at most this many results per request. */
const ID_PAGE = 1000;

/** The ids of every match of the search (not just the current page), for acting on all of them. */
export async function fetchAllMatchIds(query: LibraryQuery, mimeTypes: string[], total: number): Promise<string[]> {
  const base = searchParams(query, mimeTypes);
  const ids: string[] = [];
  for (let offset = 0; offset < total; offset += ID_PAGE) {
    const res = await searchService.enhancedSearch({ ...base, include_snippets: false, limit: ID_PAGE, offset });
    const docs = res.data.documents ?? [];
    ids.push(...docs.map((d) => d.id));
    if (docs.length < ID_PAGE) break;
  }
  return Array.from(new Set(ids));
}

export async function fetchRows(query: LibraryQuery, mimeTypes: string[]): Promise<RowsPage> {
  if (isSearch(query)) {
    const res = await searchService.enhancedSearch(searchParams(query, mimeTypes));
    return { rows: (res.data.documents ?? []).map(toRow), total: res.data.total ?? 0 };
  }
  const res = await documentService.listFiltered(listParams(query, mimeTypes));
  return { rows: (res.data.documents ?? []).map(toRow), total: res.data.pagination?.total ?? 0 };
}

/** The backend refuses searches with more than 10,000 matches. */
export function isTooManyResults(error: unknown): boolean {
  const e = error as { response?: { status?: number; data?: { error_code?: string } } } | null;
  return e?.response?.status === 413 || e?.response?.data?.error_code === 'SEARCH_TOO_MANY_RESULTS';
}

export async function fetchLabels(): Promise<LabelData[]> {
  const res = await labelService.list(false);
  return Array.isArray(res.data) ? res.data.map(toLabelData) : [];
}

export async function fetchSources(): Promise<LibrarySource[]> {
  const res = await api.get('/sources');
  const data = res.data as unknown;
  const items = Array.isArray(data) ? data : ((data as { sources?: unknown[] } | null)?.sources ?? []);
  return (items as LibrarySource[]).filter((s) => s && typeof s.id === 'string');
}

/** Every MIME type present in the library. */
export async function fetchMimeTypes(): Promise<string[]> {
  const res = await documentService.getFacets();
  return (res.data?.mime_types ?? []).map((f) => f.value);
}
