import api, { documentService, searchService } from '../../services/api';
import type { DocumentListParams, EnhancedSearchParams } from '../../services/api';
import { labelService } from '../../services/api/labels';
import type { DocumentResponse, EnhancedDocumentResponse, SearchSnippet } from '../../types/generated';
import { toLabelData, type LabelData } from '../labels';
import { dayBoundary } from './format';
import { UPLOADED, isSearch, type LibraryQuery } from './urlState';

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
  const uploadedOnly = query.sources.includes(UPLOADED);
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
    source_ids: uploadedOnly ? undefined : query.sources.length ? query.sources : undefined,
    source_types: uploadedOnly ? ['direct_upload'] : undefined,
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
    // Without an explicit sort, search results come back by relevance.
    ...(query.sortExplicit ? { sort_by: query.sort, sort_order: query.order } : {}),
  };
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
