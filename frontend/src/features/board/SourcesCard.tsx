import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { ButtonLink, EmptyState, Skeleton, SourceTile, StatusMark, type StatusState } from '../../ui';
import { ARRIVAL_DAYS, groupLanes, laneHealth, laneHref, type LaneHealth, type SourceArrivals } from './arrivals';
import { formatAge, formatCount } from './format';
import { Region, RegionError } from './Region';
import type { Resource } from './useResource';
import styles from './Home.module.css';

const MARK: Record<LaneHealth, StatusState> = {
  healthy: 'healthy',
  syncing: 'syncing',
  error: 'error',
  warning: 'warning',
  off: 'disabled',
  quiet: 'warning',
  idle: 'disabled',
};

function Health({ health }: { health: LaneHealth }) {
  const { t } = useTranslation();
  const label =
    health === 'quiet' ? t('home.lanes.quiet', 'Quiet') : health === 'idle' ? t('home.lanes.idle', 'Idle') : undefined;
  return <StatusMark state={MARK[health]} label={label} size="sm" />;
}

function useKindLabel() {
  const { t } = useTranslation();
  return (kind: string): string => {
    switch (kind) {
      case 'upload':
        return t('home.kind.upload', 'Web and API uploads');
      case 'watch':
        return t('home.kind.watch', 'Watch folder');
      case 'webdav':
        return t('home.kind.webdav', 'WebDAV');
      case 's3':
        return t('home.kind.s3', 'S3');
      case 'local_folder':
        return t('home.kind.local', 'Local folder');
      default:
        return kind;
    }
  };
}

function SourceRow({ lane, now }: { lane: SourceArrivals; now: number }) {
  const { t, i18n } = useTranslation();
  const kindLabel = useKindLabel()(lane.kind);
  const activity =
    lane.today > 0
      ? t('home.lanes.todayCount', '{{formatted}} today', { count: lane.today, formatted: formatCount(lane.today, i18n.language) })
      : lane.last_arrival_at
        ? t('home.lanes.lastArrival', 'Last arrival {{age}}', { age: formatAge(lane.last_arrival_at, i18n.language, now) })
        : t('home.lanes.never', 'Nothing yet');
  const meta = [kindLabel !== lane.name ? kindLabel : null, activity].filter(Boolean).join(' · ');

  return (
    <li className={styles.sourceRow}>
      <SourceTile sourceId={lane.source_id ?? lane.key} kind={lane.kind} type={lane.kind} />
      <span className={styles.sourceText}>
        <Link className={styles.sourceName} to={laneHref(lane)}>
          {lane.name}
        </Link>
        <span className={styles.sourceMeta}>{meta}</span>
      </span>
      <Health health={laneHealth(lane, now)} />
    </li>
  );
}

/**
 * Every way documents come in (each source, the watch folder, uploads) with its health. Problems
 * and the busiest come first; only five are shown (see groupLanes), the rest are one link away.
 */
export function SourcesCard({ arrivals, now }: { arrivals: Resource<SourceArrivals[]>; now: number }) {
  const { t, i18n } = useTranslation();
  const { data, error, reload } = arrivals;
  const title = t('home.sources.title', 'Sources');
  const groups = data ? groupLanes(data, now) : null;

  return (
    <Region
      title={title}
      className={styles.sources}
      headerAction={
        <Link className={styles.link} to="/sources">
          {t('home.sources.manage', 'Manage')}
        </Link>
      }
    >
      {error && !data ? (
        <RegionError message={t('home.lanes.error', 'Arrivals could not be loaded.')} onRetry={reload} />
      ) : !groups ? (
        <Skeleton lines={3} label={t('board.loading', 'Loading')} />
      ) : data?.length === 0 ? (
        <EmptyState
          title={t('home.lanes.empty', 'No sources yet')}
          action={
            <ButtonLink href="/sources?section=connections&new=1" variant="secondary" size="sm">
              {t('home.connectSource', 'Connect source')}
            </ButtonLink>
          }
        />
      ) : (
        <>
          {groups.shown.length ? (
            <ul className={styles.sourceList} aria-label={title}>
              {groups.shown.map((lane) => (
                <SourceRow key={lane.key} lane={lane} now={now} />
              ))}
            </ul>
          ) : (
            <p className={styles.note}>
              {t('home.lanes.allSilent', 'Nothing arrived from any source in the last {{days}} days.', { days: ARRIVAL_DAYS })}
            </p>
          )}
          {groups.hidden > 0 ? (
            <Link className={styles.link} to="/sources">
              {t('home.lanes.more', '{{formatted}} more sources', {
                count: groups.hidden,
                formatted: formatCount(groups.hidden, i18n.language),
              })}
            </Link>
          ) : null}
        </>
      )}
    </Region>
  );
}
