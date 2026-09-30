const STEPS: [Intl.RelativeTimeFormatUnit, number][] = [
  ['second', 60],
  ['minute', 60],
  ['hour', 24],
  ['day', 30],
  ['month', 12],
];

/** "3 minutes ago" in the given language, via Intl.RelativeTimeFormat. */
export function relativeTime(iso: string, lng?: string, now: number = Date.now()): string {
  const then = new Date(iso).getTime();
  if (Number.isNaN(then)) return '';
  let value = Math.round((then - now) / 1000);
  const rtf = new Intl.RelativeTimeFormat(lng, { numeric: 'auto' });
  for (const [unit, size] of STEPS) {
    if (Math.abs(value) < size) return rtf.format(value, unit);
    value = Math.round(value / size);
  }
  return rtf.format(value, 'year');
}
