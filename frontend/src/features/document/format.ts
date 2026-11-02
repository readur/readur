import type { StatusState } from '../../ui';

/** Document OCR status as a StatusMark state. A missing status means the job is still queued. */
export function ocrState(status: string | null | undefined): StatusState {
  switch (status) {
    case 'processing':
      return 'processing';
    case 'completed':
      return 'completed';
    case 'failed':
      return 'failed';
    default:
      return 'pending';
  }
}

/** True while OCR has not reached a final state, so the page keeps polling. */
export function isOcrActive(status: string | null | undefined): boolean {
  const state = ocrState(status);
  return state === 'pending' || state === 'processing';
}

export function formatBytes(bytes: number | null | undefined): string {
  if (bytes == null || !Number.isFinite(bytes)) return '—';
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.min(units.length - 1, Math.floor(Math.log(bytes) / Math.log(1024)));
  const value = bytes / Math.pow(1024, i);
  return `${i === 0 ? value : value.toFixed(1)} ${units[i]}`;
}

export function formatDate(value: string | null | undefined, lng?: string): string {
  if (!value) return '—';
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return '—';
  return date.toLocaleDateString(lng, { year: 'numeric', month: 'short', day: '2-digit' });
}

export function formatDateTime(value: string | null | undefined, lng?: string): string {
  if (!value) return '—';
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return '—';
  return date.toLocaleString(lng, {
    year: 'numeric',
    month: 'short',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  });
}

const pad = (n: number) => String(n).padStart(2, '0');

/**
 * Short, fixed-width timestamp for mono cells: `2026-09-29 11:04` in the user's time zone.
 * Used by the document summary strip and the Library slideout (ADDED / UPDATED).
 */
export function formatStamp(value: string | null | undefined): string {
  if (!value) return '—';
  const d = new Date(value);
  if (Number.isNaN(d.getTime())) return '—';
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** Where a document came from, as a short readable word. Unknown types are tidied, not shouted. */
export function sourceLabel(
  sourceType: string | null | undefined,
  t: (key: string, fallback: string) => string,
): string {
  const type = (sourceType ?? '').trim().toLowerCase();
  switch (type) {
    case '':
    case 'upload':
    case 'web_upload':
    case 'direct_upload':
    case 'api_upload':
      return t('document.pass.uploaded', 'Upload');
    case 'webdav':
    case 'web_dav':
      return 'WebDAV';
    case 's3':
      return 'S3';
    case 'local':
    case 'local_folder':
      return t('document.pass.sourceLocalFolder', 'Local folder');
    case 'batch_ingest':
      return t('document.pass.sourceBatch', 'Batch import');
    default: {
      const words = type.replace(/[_-]+/g, ' ');
      return words.charAt(0).toUpperCase() + words.slice(1);
    }
  }
}

export function formatNumber(value: number, lng?: string): string {
  return value.toLocaleString(lng);
}

export type FileKind = 'pdf' | 'image' | 'text' | 'other';

export function fileKind(mimeType: string | null | undefined): FileKind {
  const mime = (mimeType ?? '').toLowerCase();
  if (mime.includes('pdf')) return 'pdf';
  if (mime.startsWith('image/') || mime.includes('image')) return 'image';
  if (mime.startsWith('text/')) return 'text';
  return 'other';
}

/** Pulls the server's `error` message out of an API failure, if there is one. */
export function apiErrorMessage(err: unknown): string | null {
  const data = (err as { response?: { data?: { error?: unknown; message?: unknown } } })?.response?.data;
  if (data && typeof data.error === 'string' && data.error) return data.error;
  if (data && typeof data.message === 'string' && data.message) return data.message;
  return null;
}

export function httpStatus(err: unknown): number | undefined {
  return (err as { response?: { status?: number } })?.response?.status;
}

/**
 * The OCR endpoint also returns `pages_processed` and `detected_language`, which the hand-written
 * `OcrResponse` type in the services barrel does not declare yet.
 */
export interface OcrExtras {
  pages_processed?: number | null;
  detected_language?: string | null;
}
