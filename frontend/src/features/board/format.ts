/** Locale-aware formatting helpers for Home. */

export function formatCount(n?: number | null, locale?: string): string {
  return new Intl.NumberFormat(locale).format(n ?? 0);
}

const STEPS: Array<[Intl.RelativeTimeFormatUnit, number]> = [
  ['year', 365 * 24 * 3600],
  ['month', 30 * 24 * 3600],
  ['day', 24 * 3600],
  ['hour', 3600],
  ['minute', 60],
];

/** "3 min. ago", "2 days ago" (short, localized). Returns an em dash for missing or invalid dates. */
export function formatAge(iso?: string | null, locale?: string, now: number = Date.now()): string {
  const at = iso ? Date.parse(iso) : NaN;
  if (Number.isNaN(at)) return '—';
  const seconds = Math.round((now - at) / 1000);
  const rtf = new Intl.RelativeTimeFormat(locale, { numeric: 'always', style: 'narrow' });
  for (const [unit, size] of STEPS) {
    if (Math.abs(seconds) >= size) return rtf.format(-Math.trunc(seconds / size), unit);
  }
  return rtf.format(-Math.max(0, seconds), 'second');
}

/** Minutes as a short duration, e.g. "45m" or "2h 10m". */
export function formatMinutes(minutes: number): string {
  const total = Math.max(0, Math.round(minutes));
  if (total < 60) return `${total}m`;
  const h = Math.floor(total / 60);
  const m = total % 60;
  return m ? `${h}h ${m}m` : `${h}h`;
}
