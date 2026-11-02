import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useAuth } from '../../contexts/AuthContext';
import { PageHeader } from '../shell';
import { ButtonLink } from '../../ui';
import { Cloud, Upload } from '../../ui/icons';
import { fetchArrivals, weekTotal } from './arrivals';
import { fetchFailedOcr, fetchQueueFigures, POLL_MS } from './data';
import { JustArrived } from './JustArrived';
import { MarkAllSeen } from './MarkAllSeen';
import { NeedsAttention } from './NeedsAttention';
import { PipelinePanel } from './PipelinePanel';
import { SourcesCard } from './SourcesCard';
import { StatusLine } from './StatusLine';
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

/**
 * Home, laid out as the Studio mockup: the processing pipeline and what just arrived on the left,
 * what needs attention and the sources on the right. When the page is too narrow for two columns
 * the right column drops in under the pipeline (the DOM is already in that reading order).
 */
export default function Home() {
  const { t } = useTranslation();
  const now = useNow();
  const greeting = useGreeting(now);
  const arrivals = useResource(fetchArrivals, POLL_MS);
  const failed = useResource(fetchFailedOcr, POLL_MS);
  const stats = useResource(fetchQueueFigures, POLL_MS);

  const week = arrivals.data ? weekTotal(arrivals.data) : undefined;

  return (
    <>
      <PageHeader
        className={styles.header}
        title={greeting}
        meta={<StatusLine week={week} queue={stats.data} failed={failed.data} />}
        actions={
          <>
            <MarkAllSeen kinds={HOME_KINDS} />
            <ButtonLink href="/sources?section=connections&new=1" variant="secondary" icon={<Cloud fontSize="inherit" />}>
              {t('home.connectSource', 'Connect source')}
            </ButtonLink>
            <ButtonLink href="/intake?section=upload" variant="primary" icon={<Upload fontSize="inherit" />}>
              {t('home.upload', 'Upload')}
            </ButtonLink>
          </>
        }
      />
      <div className={styles.layout}>
        <PipelinePanel failed={failed} stats={stats} />
        <div className={styles.side}>
          <NeedsAttention failed={failed} />
          <SourcesCard arrivals={arrivals} now={now} />
        </div>
        <JustArrived lanes={arrivals.data} now={now} className={styles.recent} />
      </div>
    </>
  );
}
