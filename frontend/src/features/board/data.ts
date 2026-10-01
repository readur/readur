import { documentService, queueService } from '../../services/api';
import type { BoardDocument, FailedOcrDocument } from './types';

export const POLL_MS = 15_000;

/** How many of the newest documents Home shows. */
export const RECENT_COUNT = 12;

export interface RecentPage {
  documents: BoardDocument[];
  total: number;
}

const asArray = <T,>(data: unknown, key: string): T[] => {
  if (Array.isArray(data)) return data as T[];
  const inner = (data as Record<string, unknown> | null | undefined)?.[key];
  return Array.isArray(inner) ? (inner as T[]) : [];
};

/** The newest documents plus the library total. */
export async function fetchRecent(): Promise<RecentPage> {
  const res = await documentService.listWithPagination(RECENT_COUNT, 0);
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

export interface FailedOcrPage {
  /** The newest failed documents (at most {@link FAILED_SAMPLE}). */
  documents: FailedOcrDocument[];
  /** Every document whose OCR failed. */
  total: number;
}

/** How many of the newest failures are read to name their causes. */
export const FAILED_SAMPLE = 50;

/** Documents whose OCR failed (GET /documents/failed/ocr), newest first. */
export async function fetchFailedOcr(): Promise<FailedOcrPage> {
  const res = await documentService.getFailedOcrDocuments(FAILED_SAMPLE);
  const rows = asArray<FailedOcrRow>(res.data, 'documents');
  const total = (res.data as { pagination?: { total?: number } } | undefined)?.pagination?.total ?? rows.length;
  const documents = rows.map<FailedOcrDocument>((d) => ({
    id: d.id,
    filename: d.filename,
    original_filename: d.original_filename,
    failure_reason: d.failure_reason ?? d.ocr_failure_reason ?? d.failure_category,
    error_message: d.error_message ?? d.ocr_error ?? null,
    created_at: d.created_at,
    updated_at: d.updated_at,
    last_retry_at: d.last_retry_at ?? d.last_attempt_at ?? null,
  }));
  return { documents, total };
}

/** OCR queue figures. `failed` is deliberately absent: Home counts failed documents instead. */
export interface QueueFigures {
  pending: number;
  processing: number;
  completedToday: number;
  oldestPendingMinutes: number | null;
}

type RawQueueStats = Partial<Record<'pending' | 'pending_count' | 'processing' | 'processing_count' | 'completed_today' | 'oldest_pending_minutes', number | null>>;

/**
 * GET /queue/stats. The server answers `pending` / `processing`; older builds and the typed client
 * say `pending_count` / `processing_count`. Both are read so the figures are never silently zero.
 * The endpoint is admin-only: for other users this resolves to null (figures unavailable).
 */
export async function fetchQueueFigures(): Promise<QueueFigures | null> {
  try {
    const res = await queueService.getStats();
    const q = (res.data ?? {}) as RawQueueStats;
    return {
      pending: q.pending ?? q.pending_count ?? 0,
      processing: q.processing ?? q.processing_count ?? 0,
      completedToday: q.completed_today ?? 0,
      oldestPendingMinutes: q.oldest_pending_minutes ?? null,
    };
  } catch (error) {
    if ((error as { response?: { status?: number } })?.response?.status === 403) return null;
    throw error;
  }
}
