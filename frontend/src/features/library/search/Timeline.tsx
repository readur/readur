import { useMemo, useRef, useState, type KeyboardEvent, type MouseEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { formatCount } from '../format';
import { monthLabel, rangeMonths, type MonthBar } from '../months';
import styles from './Timeline.module.css';

interface TimelineProps {
  bars: MonthBar[];
  /** The Added filter, `YYYY-MM-DD`; the months it covers are drawn as picked. */
  from: string | null;
  to: string | null;
  /** Pick one month, or stretch the pick to a span (Shift); `null` clears it. */
  onPick: (range: { start: string; end: string } | null) => void;
  isLoading?: boolean;
}

/** Up to this many bars get a month name under them; longer spans only mark the years. */
const LABEL_EVERY_MONTH = 18;

/**
 * Matches per month as a bar chart. Each month with matches is a button: press it to show only
 * that month, Shift+press to stretch the pick, press a lone picked month again to clear. Arrow
 * keys move between months (one tab stop for the whole chart).
 */
export function Timeline({ bars, from, to, onPick, isLoading = false }: TimelineProps) {
  const { t, i18n } = useTranslation();
  const lng = i18n.language;
  const max = useMemo(() => bars.reduce((m, b) => Math.max(m, b.count), 0), [bars]);
  const picked = rangeMonths(from, to);
  const isPicked = (month: string) => Boolean(picked && month >= picked.start && month <= picked.end);
  const active = bars.filter((b) => b.count > 0);

  const [focusMonth, setFocusMonth] = useState<string | null>(null);
  const [hoverMonth, setHoverMonth] = useState<string | null>(null);
  const anchor = useRef<string | null>(null);
  const refs = useRef(new Map<string, HTMLButtonElement>());

  // The one bar in the tab order: the focused one, else the first picked one, else the newest.
  const tabMonth =
    (focusMonth && active.some((b) => b.month === focusMonth) ? focusMonth : null) ??
    active.find((b) => isPicked(b.month))?.month ??
    active.at(-1)?.month ??
    null;

  if (bars.length === 0) return null;

  const press = (month: string, extend: boolean) => {
    if (extend && anchor.current) {
      const [start, end] = [anchor.current, month].sort();
      onPick({ start, end });
      return;
    }
    if (picked && picked.start === month && picked.end === month) {
      anchor.current = null;
      onPick(null);
      return;
    }
    anchor.current = month;
    onPick({ start: month, end: month });
  };

  const move = (e: KeyboardEvent<HTMLButtonElement>, month: string) => {
    const index = active.findIndex((b) => b.month === month);
    const target =
      e.key === 'ArrowRight' ? active[index + 1]
      : e.key === 'ArrowLeft' ? active[index - 1]
      : e.key === 'Home' ? active[0]
      : e.key === 'End' ? active.at(-1)
      : undefined;
    if (!target) return;
    e.preventDefault();
    setFocusMonth(target.month);
    refs.current.get(target.month)?.focus();
  };

  const shown = hoverMonth ?? focusMonth;
  const shownBar = shown ? bars.find((b) => b.month === shown) : undefined;
  const readout = shownBar
    ? t('library.timeline.month', {
        month: monthLabel(shownBar.month, lng),
        count: shownBar.count,
        formatted: formatCount(shownBar.count, lng),
        defaultValue: '{{month}}: {{formatted}} matches',
        defaultValue_one: '{{month}}: 1 match',
      })
    : t('library.timeline.span', {
        first: monthLabel(bars[0].month, lng, 'short'),
        last: monthLabel(bars[bars.length - 1].month, lng, 'short'),
        defaultValue: '{{first}} – {{last}}',
      });
  const labelAll = bars.length <= LABEL_EVERY_MONTH;

  return (
    <div className={styles.timeline} data-loading={isLoading || undefined}>
      <div
        className={styles.bars}
        role="group"
        aria-label={t('library.timeline.label', 'Matches by month')}
        data-dense={bars.length > 60 || undefined}
        onMouseLeave={() => setHoverMonth(null)}
      >
        {bars.map((b) => {
          const height = max > 0 ? Math.max(4, Math.round((b.count / max) * 100)) : 0;
          if (b.count === 0) {
            return (
              <span key={b.month} className={styles.slot} aria-hidden="true">
                <span className={styles.empty} />
              </span>
            );
          }
          return (
            <button
              key={b.month}
              ref={(el) => {
                if (el) refs.current.set(b.month, el);
                else refs.current.delete(b.month);
              }}
              type="button"
              className={styles.slot}
              tabIndex={b.month === tabMonth ? 0 : -1}
              aria-pressed={isPicked(b.month)}
              aria-label={t('library.timeline.month', {
                month: monthLabel(b.month, lng),
                count: b.count,
                formatted: formatCount(b.count, lng),
                defaultValue: '{{month}}: {{formatted}} matches',
                defaultValue_one: '{{month}}: 1 match',
              })}
              data-picked={isPicked(b.month) || undefined}
              data-muted={(picked && !isPicked(b.month)) || undefined}
              onClick={(e: MouseEvent) => press(b.month, e.shiftKey)}
              onKeyDown={(e) => move(e, b.month)}
              onFocus={() => setFocusMonth(b.month)}
              onBlur={() => setFocusMonth((m) => (m === b.month ? null : m))}
              onMouseEnter={() => setHoverMonth(b.month)}
            >
              <span className={styles.bar} style={{ height: `${height}%` }} />
            </button>
          );
        })}
      </div>
      <div className={styles.axis} aria-hidden="true">
        {bars.map((b, i) => {
          const january = b.month.endsWith('-01');
          const text = labelAll
            ? january || i === 0
              ? monthLabel(b.month, lng, 'short')
              : new Date(Number(b.month.slice(0, 4)), Number(b.month.slice(5)) - 1, 1).toLocaleDateString(lng, { month: 'short' })
            : january || i === 0
              ? b.month.slice(0, 4)
              : '';
          return (
            <span key={b.month} className={styles.tick}>
              {text ? <span className={styles.tickText}>{text}</span> : null}
            </span>
          );
        })}
      </div>
      {/* Visual only: each bar already names its month and count. */}
      <p className={styles.readout} aria-hidden="true">
        {readout}
      </p>
    </div>
  );
}
