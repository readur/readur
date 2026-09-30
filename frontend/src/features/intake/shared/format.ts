/** Short, locale-aware formatting helpers for Intake. Results are meant for mono cells. */

const UNITS = ['B', 'KB', 'MB', 'GB', 'TB'];

/** "0 B", "1 KB", "1.5 MB". One decimal at most, trailing zeros dropped. */
export function formatBytes(bytes?: number | null, digits = 1): string {
  if (!bytes || bytes <= 0 || Number.isNaN(bytes)) return '0 B';
  const i = Math.min(UNITS.length - 1, Math.floor(Math.log(bytes) / Math.log(1024)));
  return `${parseFloat((bytes / Math.pow(1024, i)).toFixed(digits))} ${UNITS[i]}`;
}

export function formatCount(n?: number | null, locale?: string): string {
  return new Intl.NumberFormat(locale).format(n ?? 0);
}

/** Seconds as "45s", "1m 30s" or "1h 1m". */
export function formatDuration(seconds: number): string {
  const s = Math.max(0, Math.round(seconds));
  if (s < 60) return `${s}s`;
  if (s < 3600) return `${Math.floor(s / 60)}m ${s % 60}s`;
  return `${Math.floor(s / 3600)}h ${Math.floor((s % 3600) / 60)}m`;
}

/** Minutes as "45m" or "2h 10m"; an em dash when unknown. */
export function formatMinutes(minutes?: number | null): string {
  if (minutes === undefined || minutes === null || Number.isNaN(minutes) || minutes <= 0) return '—';
  const total = Math.round(minutes);
  if (total < 60) return `${total}m`;
  return `${Math.floor(total / 60)}h ${total % 60}m`;
}

const STEPS: Array<[Intl.RelativeTimeFormatUnit, number]> = [
  ['year', 365 * 24 * 3600],
  ['month', 30 * 24 * 3600],
  ['day', 24 * 3600],
  ['hour', 3600],
  ['minute', 60],
];

/** Relative time, past or future: "3 min. ago", "in 12 min." (narrow). Em dash when missing. */
export function formatRelative(at?: string | number | null, locale?: string, now: number = Date.now()): string {
  const time = typeof at === 'number' ? at : at ? Date.parse(at) : NaN;
  if (Number.isNaN(time)) return '—';
  const seconds = Math.round((time - now) / 1000);
  const rtf = new Intl.RelativeTimeFormat(locale, { numeric: 'always', style: 'narrow' });
  for (const [unit, size] of STEPS) {
    if (Math.abs(seconds) >= size) return rtf.format(Math.trunc(seconds / size), unit);
  }
  return rtf.format(seconds, 'second');
}

/** "12 Mar 2026, 14:05" in the user's locale. Em dash when missing. */
export function formatDateTime(iso?: string | null, locale?: string): string {
  const time = iso ? Date.parse(iso) : NaN;
  if (Number.isNaN(time)) return '—';
  return new Intl.DateTimeFormat(locale, { dateStyle: 'medium', timeStyle: 'short' }).format(time);
}

export function formatDate(iso?: string | null, locale?: string): string {
  const time = iso ? Date.parse(iso) : NaN;
  if (Number.isNaN(time)) return '—';
  return new Intl.DateTimeFormat(locale, { dateStyle: 'medium' }).format(time);
}

/** "low_ocr_confidence" → "low ocr confidence". */
export function humanize(value?: string | null): string {
  return (value ?? '').replace(/_/g, ' ').trim();
}

/** A 16-character hash prefix for display. */
export function shortHash(hash: string, length = 16): string {
  return hash.length > length ? `${hash.slice(0, length)}…` : hash;
}
