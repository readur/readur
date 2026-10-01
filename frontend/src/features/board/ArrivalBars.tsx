import type { CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import type { DayCount } from './arrivals';
import { formatCount } from './format';
import styles from './Home.module.css';

export interface ArrivalBarsProps {
  days: DayCount[];
  /** The lane's colour slot (1–8). */
  hue: number;
}

const dayLabel = (date: string, locale: string) =>
  new Intl.DateTimeFormat(locale, { day: 'numeric', month: 'short', timeZone: 'UTC' }).format(new Date(`${date}T00:00:00Z`));

/**
 * One bar per day, oldest on the left and today on the right, scaled to the lane's busiest day.
 * Read as one image; its label says the total and the busiest day.
 */
export function ArrivalBars({ days, hue }: ArrivalBarsProps) {
  const { t, i18n } = useTranslation();
  const locale = i18n.language;
  const total = days.reduce((sum, d) => sum + d.count, 0);
  const busiest = days.reduce<DayCount | null>((best, d) => (d.count > (best?.count ?? 0) ? d : best), null);
  const max = busiest?.count ?? 0;
  const label = busiest
    ? t('home.lanes.barsLabel', '{{total}} in the last {{days}} days, most on {{day}} ({{most}})', {
        total: formatCount(total, locale),
        days: days.length,
        day: dayLabel(busiest.date, locale),
        most: formatCount(busiest.count, locale),
      })
    : t('home.lanes.barsEmpty', 'Nothing in the last {{days}} days', { days: days.length });

  return (
    <div className={styles.bars} role="img" aria-label={label} data-hue={hue}>
      {days.map((d, i) => {
        const style = { '--h': max ? d.count / max : 0, '--i': i } as CSSProperties;
        return (
          <span
            key={d.date}
            className={styles.bar}
            data-zero={d.count === 0 || undefined}
            data-today={i === days.length - 1 || undefined}
            style={style}
            title={`${dayLabel(d.date, locale)}: ${formatCount(d.count, locale)}`}
          />
        );
      })}
    </div>
  );
}
