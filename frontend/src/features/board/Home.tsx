import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useAuth } from '../../contexts/AuthContext';
import { PageHeader } from '../shell';
import { fetchArrivals, weekTotal } from './arrivals';
import { fetchFailedOcr, fetchQueueFigures, POLL_MS } from './data';
import { formatCount } from './format';
import { JustArrived } from './JustArrived';
import { MarkAllSeen } from './MarkAllSeen';
import { PipelinePanel } from './PipelinePanel';
import { SourceLanes } from './SourceLanes';
import { useResource } from './useResource';
import styles from './Home.module.css';

const HOME_KINDS = ['document'] as const;
const MINUTE_MS = 60_000;

/** The current time, refreshed every minute so "2m ago" and quiet lanes stay true. */
function useNow(): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), MINUTE_MS);
    return () => window.clearInterval(timer);
  }, []);
  return now;
}

function useGreeting(now: number): string {
  const { t } = useTranslation();
  const { user } = useAuth();
  const hour = new Date(now).getHours();
  const name = user?.username ?? '';
  if (hour < 5 || hour >= 18) return name ? t('home.greeting.evening', 'Good evening, {{name}}', { name }) : t('home.greeting.eveningPlain', 'Good evening');
  if (hour < 12) return name ? t('home.greeting.morning', 'Good morning, {{name}}', { name }) : t('home.greeting.morningPlain', 'Good morning');
  return name ? t('home.greeting.afternoon', 'Good afternoon, {{name}}', { name }) : t('home.greeting.afternoonPlain', 'Good afternoon');
}

/** Home: is everything still coming in, what is being processed, what just arrived. */
export default function Home() {
  const { t, i18n } = useTranslation();
  const now = useNow();
  const greeting = useGreeting(now);
  const arrivals = useResource(fetchArrivals, POLL_MS);
  const failed = useResource(fetchFailedOcr, POLL_MS);
  const stats = useResource(fetchQueueFigures, POLL_MS);

  const week = arrivals.data ? weekTotal(arrivals.data) : null;
  const meta =
    week === null
      ? undefined
      : week === 0
        ? t('home.week.none', 'Nothing arrived this week')
        : t('home.week.count', '{{formatted}} arrived this week', { count: week, formatted: formatCount(week, i18n.language) });

  return (
    <>
      <PageHeader
        title={greeting}
        meta={meta}
        actions={
          <>
            <MarkAllSeen kinds={HOME_KINDS} />
            <Link className={styles.primaryLink} to="/intake?section=upload">
              {t('board.addDocuments', 'Add documents')}
            </Link>
          </>
        }
      />
      <div className={styles.page}>
        <SourceLanes arrivals={arrivals} now={now} />
        <PipelinePanel failed={failed} stats={stats} />
        <JustArrived lanes={arrivals.data} now={now} />
      </div>
    </>
  );
}
