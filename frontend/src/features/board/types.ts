import type { LabelData } from '../labels';

/** The fields of a document Home reads. */
export interface BoardDocument {
  id: string;
  filename?: string;
  original_filename?: string;
  file_size?: number;
  mime_type?: string;
  created_at?: string;
  has_ocr_text?: boolean;
  ocr_status?: string | null;
  ocr_progress_current?: number;
  ocr_progress_total?: number;
  source_id?: string | null;
  source_type?: string | null;
  labels?: LabelData[];
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

export const docName = (d: { original_filename?: string | null; filename?: string }): string =>
  d.original_filename || d.filename || '';
