import api, { documentService } from '../../services/api';
import type { BoardDocument, BoardSource, FailedOcrDocument } from './types';

export const POLL_MS = 15_000;

export interface ArrivalsPage {
  documents: BoardDocument[];
  total: number;
}

export interface LibraryTotals {
  documents: number;
  storageBytes: number;
  withOcr: number;
}

const asArray = <T,>(data: unknown, key: string): T[] => {
  if (Array.isArray(data)) return data as T[];
  const inner = (data as Record<string, unknown> | null | undefined)?.[key];
  return Array.isArray(inner) ? (inner as T[]) : [];
};

/** The 10 newest documents plus the library total. */
export async function fetchArrivals(): Promise<ArrivalsPage> {
  const res = await documentService.listWithPagination(10, 0);
  const documents = asArray<BoardDocument>(res.data, 'documents');
  const total = (res.data as { pagination?: { total?: number } })?.pagination?.total ?? documents.length;
  return { documents, total };
}

type FailedOcrRow = FailedOcrDocument & {
  ocr_failure_reason?: string | null;
  ocr_error?: string | null;
  failure_category?: string;
  last_attempt_at?: string | null;
};

/**
 * Documents whose OCR failed (GET /documents/failed/ocr). Their ids are real document ids, so
 * Retry works on them. The rows are mapped onto the strip's shape (reason, message, last try).
 */
export async function fetchFailedOcr(): Promise<FailedOcrDocument[]> {
  const res = await documentService.getFailedOcrDocuments(10);
  return asArray<FailedOcrRow>(res.data, 'documents').map((d) => ({
    id: d.id,
    filename: d.filename,
    original_filename: d.original_filename,
    failure_reason: d.failure_reason ?? d.ocr_failure_reason ?? d.failure_category,
    error_message: d.error_message ?? d.ocr_error ?? null,
    created_at: d.created_at,
    updated_at: d.updated_at,
    last_retry_at: d.last_retry_at ?? d.last_attempt_at ?? null,
  }));
}

export async function fetchSources(): Promise<BoardSource[]> {
  const res = await api.get('/sources');
  return asArray<BoardSource>(res.data, 'sources');
}

export async function fetchTotals(): Promise<LibraryTotals> {
  const res = await api.get('/metrics');
  const d = res.data?.documents;
  if (!d) throw new Error('metrics unavailable');
  return {
    documents: d.total_documents ?? 0,
    storageBytes: d.total_storage_bytes ?? 0,
    withOcr: d.documents_with_ocr ?? 0,
  };
}

/** Number of labels, or null when the labels call fails (the cell then shows a dash). */
export async function fetchLabelCount(): Promise<number | null> {
  try {
    const res = await api.get('/labels?include_counts=false');
    return asArray(res.data, 'labels').length;
  } catch {
    return null;
  }
}
