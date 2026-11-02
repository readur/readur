import { formatRelativeTime } from '../../lib/relativeTime';
import type { StatusState } from '../../ui';

/** OCR status as a StatusMark state. A missing status means the job is still queued. */
export function ocrState(status: string | null | undefined): StatusState {
  switch (status) {
    case 'processing':
    case 'completed':
    case 'failed':
      return status;
    default:
      return 'pending';
  }
}

export function formatBytes(bytes: number | null | undefined, lng?: string): string {
  if (bytes == null || !Number.isFinite(bytes)) return '—';
  if (bytes < 1024) return `${bytes} B`;
  const units = ['KB', 'MB', 'GB', 'TB'];
  let value = bytes / 1024;
  let i = 0;
  while (value >= 1024 && i < units.length - 1) {
    value /= 1024;
    i += 1;
  }
  const digits = value < 10 ? 1 : 0;
  return `${value.toLocaleString(lng, { maximumFractionDigits: digits, minimumFractionDigits: digits })} ${units[i]}`;
}

export function formatCount(n: number, lng?: string): string {
  return n.toLocaleString(lng);
}

export function formatDateTime(value: string | null | undefined, lng?: string): string {
  if (!value) return '—';
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return '—';
  return date.toLocaleString(lng, { year: 'numeric', month: 'short', day: '2-digit', hour: '2-digit', minute: '2-digit' });
}

/**
 * "3 days ago" / "21 min. ago" / "now", then the date past a week: the app's one relative-time
 * format (lib/relativeTime), so the Library reads the same as Home and Intake.
 */
export function formatRelative(value: string | null | undefined, lng?: string, now = Date.now()): string {
  return formatRelativeTime(value, { locale: lng, now });
}

/** Local calendar date as YYYY-MM-DD (the URL's date format). */
export function isoDay(date: Date): string {
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, '0');
  const d = String(date.getDate()).padStart(2, '0');
  return `${y}-${m}-${d}`;
}

/** Start (00:00) or end (23:59:59.999) of a YYYY-MM-DD day in local time; null if malformed. */
export function dayBoundary(day: string, edge: 'start' | 'end'): Date | null {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(day)) return null;
  const date = new Date(`${day}T${edge === 'start' ? '00:00:00.000' : '23:59:59.999'}`);
  return Number.isNaN(date.getTime()) ? null : date;
}
