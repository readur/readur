import i18n from 'i18next';

/**
 * The one relative-time formatter: "21 min. ago", "in 3 hr.", "now", in the current UI language
 * (Intl.RelativeTimeFormat, style 'short'). Past a week it gives the calendar date instead
 * ("30 Sep 2026"), because "5 weeks ago" is harder to place than the date itself.
 */

export type TimeInput = string | number | Date | null | undefined;

export interface RelativeTimeOptions {
  /** Reference time in ms; defaults to Date.now(). */
  now?: number;
  /** BCP 47 language; defaults to the current i18n language. */
  locale?: string;
  /** Beyond this many days (past or future) show the date instead; `null` never does. Default 7. */
  absoluteAfterDays?: number | null;
  /** What to return for a missing or unparseable time. Default an em dash. */
  fallback?: string;
}

const MINUTE = 60;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

const STEPS: Array<[Intl.RelativeTimeFormatUnit, number]> = [
  ['year', 365 * DAY],
  ['month', 30 * DAY],
  ['week', 7 * DAY],
  ['day', DAY],
  ['hour', HOUR],
  ['minute', MINUTE],
];

const toMillis = (at: TimeInput): number => {
  if (at === null || at === undefined || at === '') return NaN;
  if (at instanceof Date) return at.getTime();
  return typeof at === 'number' ? at : Date.parse(at);
};

const currentLocale = (): string | undefined => i18n.language || i18n.resolvedLanguage || undefined;

/** "30 Sep 2026" in the given (or current) language. */
export function formatAbsoluteDate(at: TimeInput, locale: string | undefined = currentLocale(), fallback = '—'): string {
  const time = toMillis(at);
  if (Number.isNaN(time)) return fallback;
  return new Intl.DateTimeFormat(locale, { day: 'numeric', month: 'short', year: 'numeric' }).format(time);
}

/** Relative time for past or future moments; see the module comment. */
export function formatRelativeTime(at: TimeInput, options: RelativeTimeOptions = {}): string {
  const { now = Date.now(), locale = currentLocale(), absoluteAfterDays = 7, fallback = '—' } = options;
  const time = toMillis(at);
  if (Number.isNaN(time)) return fallback;
  const seconds = Math.round((time - now) / 1000);
  const abs = Math.abs(seconds);

  if (absoluteAfterDays !== null && abs > absoluteAfterDays * DAY) return formatAbsoluteDate(time, locale, fallback);
  if (abs < MINUTE) return new Intl.RelativeTimeFormat(locale, { numeric: 'auto', style: 'short' }).format(0, 'second');

  const rtf = new Intl.RelativeTimeFormat(locale, { numeric: 'always', style: 'short' });
  for (const [unit, size] of STEPS) {
    if (abs >= size) return rtf.format(Math.trunc(seconds / size), unit);
  }
  return rtf.format(seconds, 'second');
}
