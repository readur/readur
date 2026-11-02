/** Body of the cleanup endpoints (DELETE /documents/cleanup/*), in preview or delete mode. */
export interface CleanupDocument {
  id: string;
  filename: string;
  original_filename?: string | null;
  file_size: number;
  ocr_confidence?: number | null;
  ocr_status?: string | null;
  created_at: string;
}

export interface CleanupResponse {
  success?: boolean;
  message: string;
  matched_count: number;
  deleted_count?: number;
  document_ids?: string[];
  documents?: CleanupDocument[];
}

/** How many rows of a preview are listed; the rest are counted. */
export const PREVIEW_ROWS = 20;
