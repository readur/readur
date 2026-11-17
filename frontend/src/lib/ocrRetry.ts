import type { RetryOcrResponse } from '../types/generated';

/**
 * What a `POST /documents/{id}/ocr/retry` answer means for the user:
 * - `queued`: a new OCR job was queued;
 * - `alreadyQueued`: the request succeeded, but the document already had a pending or running
 *   job (for example the server's own automatic retry), so nothing new was queued;
 * - `refused`: the server answered `success: false`.
 *
 * A body without `queued` (older servers) counts as queued when it does not say it failed.
 */
export type OcrRetryOutcome = 'queued' | 'alreadyQueued' | 'refused';

export function ocrRetryOutcome(
  data: Partial<RetryOcrResponse> | undefined | null,
): OcrRetryOutcome {
  if (data?.success === false) return 'refused';
  if (data?.queued === false) return 'alreadyQueued';
  return 'queued';
}
