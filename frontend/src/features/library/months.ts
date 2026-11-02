/** Calendar months as `YYYY-MM`, for grouping rows and drawing the match histogram. */

export interface MonthGroup<T> {
  /** `YYYY-MM`, or `''` for rows without a usable date. */
  key: string;
  items: T[];
}

/** The local-time month of an ISO timestamp, or `''` when it cannot be read. */
export function monthKey(iso: string | null | undefined): string {
  if (!iso) return '';
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return '';
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}`;
}

/** Consecutive runs of rows from the same month, in the order given. */
export function groupByMonth<T>(rows: readonly T[], dateOf: (row: T) => string): MonthGroup<T>[] {
  const groups: MonthGroup<T>[] = [];
  for (const row of rows) {
    const key = monthKey(dateOf(row));
    const last = groups.at(-1);
    if (last && last.key === key) last.items.push(row);
    else groups.push({ key, items: [row] });
  }
  return groups;
}

const MONTH = /^(\d{4})-(\d{2})$/;

function parts(key: string): [number, number] | null {
  const m = MONTH.exec(key);
  if (!m) return null;
  const month = Number(m[2]);
  return month >= 1 && month <= 12 ? [Number(m[1]), month] : null;
}

/** "March 2024" in the user's language. */
export function monthLabel(key: string, lng?: string, style: 'long' | 'short' = 'long'): string {
  const p = parts(key);
  if (!p) return key;
  return new Date(p[0], p[1] - 1, 1).toLocaleDateString(lng, { month: style, year: 'numeric' });
}

/** First and last day of a month as `YYYY-MM-DD`, the URL's date format. */
export function monthRange(key: string): { from: string; to: string } | null {
  const p = parts(key);
  if (!p) return null;
  const [y, m] = p;
  const last = new Date(y, m, 0).getDate();
  const mm = String(m).padStart(2, '0');
  return { from: `${y}-${mm}-01`, to: `${y}-${mm}-${String(last).padStart(2, '0')}` };
}

function nextMonth(key: string): string {
  const p = parts(key);
  if (!p) return key;
  const [y, m] = p;
  return m === 12 ? `${y + 1}-01` : `${y}-${String(m + 1).padStart(2, '0')}`;
}

export interface MonthBar {
  month: string;
  count: number;
}

/** Longest span drawn; older months beyond it are folded out of view. */
export const MAX_MONTHS = 240;

/**
 * Every month from the first to the last with matches, zero months included, so gaps in the
 * archive show as gaps. Input may be unsorted and may omit zero months.
 */
export function fillMonths(counts: readonly MonthBar[]): MonthBar[] {
  const valid = counts.filter((c) => parts(c.month) && c.count > 0);
  if (valid.length === 0) return [];
  const byMonth = new Map<string, number>();
  for (const c of valid) byMonth.set(c.month, (byMonth.get(c.month) ?? 0) + c.count);
  const keys = [...byMonth.keys()].sort();
  const bars: MonthBar[] = [];
  for (let key = keys[0]; key <= keys[keys.length - 1]; key = nextMonth(key)) {
    bars.push({ month: key, count: byMonth.get(key) ?? 0 });
  }
  return bars.length > MAX_MONTHS ? bars.slice(-MAX_MONTHS) : bars;
}

/** The months a `from`/`to` date range touches, as inclusive `YYYY-MM` bounds. */
export function rangeMonths(from: string | null, to: string | null): { start: string; end: string } | null {
  if (!from && !to) return null;
  return { start: from ? from.slice(0, 7) : '0000-00', end: to ? to.slice(0, 7) : '9999-99' };
}
