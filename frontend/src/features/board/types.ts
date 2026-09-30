import type { StatusState } from '../../ui';

/** The fields of a document the Board reads. */
export interface BoardDocument {
  id: string;
  filename?: string;
  original_filename?: string;
  file_size?: number;
  mime_type?: string;
  created_at?: string;
  has_ocr_text?: boolean;
  ocr_status?: string;
  ocr_progress_current?: number;
  ocr_progress_total?: number;
}

export interface BoardSource {
  id: string;
  name: string;
  enabled?: boolean;
  status?: string;
  last_sync_at?: string | null;
  last_error?: string | null;
  last_error_at?: string | null;
}

export interface FailedOcrDocument {
  id: string;
  filename?: string;
  original_filename?: string | null;
  failure_reason?: string;
  error_message?: string | null;
  created_at?: string;
  updated_at?: string;
  last_retry_at?: string | null;
}

/** One row of the "Needs attention" strip. */
export interface AttentionItem {
  key: string;
  kind: 'document' | 'source';
  id: string;
  name: string;
  reason: string;
  at?: string;
  state: StatusState;
}

export const docName = (d: { original_filename?: string | null; filename?: string }): string =>
  d.original_filename || d.filename || '';

/** Document row state as shown by the status mark. */
export function documentState(d: BoardDocument): StatusState {
  switch (d.ocr_status) {
    case 'completed':
      return 'completed';
    case 'failed':
      return 'failed';
    case 'processing':
      return 'processing';
    case 'pending':
    case 'queued':
      return 'pending';
    default:
      return d.has_ocr_text ? 'completed' : 'pending';
  }
}

export function sourceState(s: BoardSource): StatusState {
  if (s.enabled === false) return 'disabled';
  if (s.status === 'error') return 'error';
  if (s.status === 'syncing') return 'syncing';
  return 'healthy';
}
