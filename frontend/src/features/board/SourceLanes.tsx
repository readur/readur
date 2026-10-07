import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { EmptyState, Skeleton, SourceDot, StatusMark, type StatusState } from '../../ui';
import { sourceHue } from '../../lib/sourceColor';
import { ArrivalBars } from './ArrivalBars';
import { ARRIVAL_DAYS, groupLanes, laneHealth, laneHref, type LaneHealth, type SourceArrivals } from './arrivals';
import { formatAge, formatCount } from './format';
import { Region, RegionError } from './Region';
import type { Resource } from './useResource';
import styles from './Home.module.css';

const MARK: Record<Exclude<LaneHealth, 'quiet' | 'idle'>, StatusState> = {
  healthy: 'healthy',
  syncing: 'syncing',
  error: 'error',
  warning: 'warning',
  off: 'disabled',
};

function Health({ health }: { health: LaneHealth }) {
  const { t } = useTranslation();
  if (health === 'quiet') {
    return (
      <span className={styles.quiet}>
        <span aria-hidden="true">◆</span>
        {t('home.lanes.quiet', 'Quiet')}
      </span>
    );
  }
  if (health === 'idle') {
    return (
      <span className={styles.idle}>
        <span aria-hidden="true">○</span>
        {t('home.lanes.idle', 'Idle')}
      </span>
    );
  }
  return <StatusMark state={MARK[health]} size="sm" />;
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

function Lane({ lane, now }: { lane: SourceArrivals; now: number }) {
  const { t, i18n } = useTranslation();
  const kindLabel = useKindLabel();
  const hue = sourceHue(lane.source_id ?? lane.key, lane.kind).index;
  const health = laneHealth(lane, now);
  const last = lane.last_arrival_at
    ? t('home.lanes.lastArrival', 'Last arrival {{age}}', { age: formatAge(lane.last_arrival_at, i18n.language, now) })
    : t('home.lanes.never', 'Nothing yet');

  return (
    <li className={styles.lane} data-health={health}>
      <div className={styles.laneName}>
        <SourceDot sourceId={lane.source_id ?? lane.key} kind={lane.kind} />
        <span className={styles.laneText}>
          <Link className={styles.laneLink} to={laneHref(lane)}>
            {lane.name}
          </Link>
          {kindLabel(lane.kind) !== lane.name ? <span className={styles.laneKind}>{kindLabel(lane.kind)}</span> : null}
        </span>
      </div>
      <ArrivalBars days={lane.days} hue={hue} />
      <p className={styles.laneToday}>
        <span className={styles.laneCount}>{formatCount(lane.today, i18n.language)}</span>{' '}
        <span className={styles.laneMeta}>{t('home.lanes.today', 'today')}</span>
      </p>
      <p className={styles.laneLast}>{last}</p>
      <div className={styles.laneHealth}>
        <Health health={health} />
      </div>
    </li>
  );
}

function LaneList({ lanes, now, label }: { lanes: SourceArrivals[]; now: number; label: string }) {
  return (
    <ul className={styles.lanes} aria-label={label}>
      {lanes.map((lane) => (
        <Lane key={lane.key} lane={lane} now={now} />
      ))}
    </ul>
  );
}

/**
 * One lane per way documents come in: each source, the watch folder and uploads. Problems and
 * the busiest lanes come first; only the five most active are shown (see groupLanes), the rest
 * are one link away in Intake.
 */
export function SourceLanes({ arrivals, now }: { arrivals: Resource<SourceArrivals[]>; now: number }) {
  const { t, i18n } = useTranslation();
  const { data, error, reload } = arrivals;
  const title = t('home.lanes.title', 'Coming in');
  const groups = data ? groupLanes(data, now) : null;
  const n = (v: number) => formatCount(v, i18n.language);

  return (
    <Region
      title={title}
      headerAction={
        <Link className={styles.link} to="/sources?section=connections">
          {t('home.lanes.manage', 'Manage sources')}
        </Link>
      }
    >
      {error && !data ? (
        <RegionError message={t('home.lanes.error', 'Arrivals could not be loaded.')} onRetry={reload} />
      ) : !groups ? (
        <div className={styles.panelBody}>
          <Skeleton lines={3} label={t('board.loading', 'Loading')} />
        </div>
      ) : data?.length === 0 ? (
        <EmptyState title={t('home.lanes.empty', 'No sources yet')} />
      ) : (
        <>
          {groups.shown.length ? (
            <LaneList lanes={groups.shown} now={now} label={title} />
          ) : (
            <p className={styles.panelBody}>{t('home.lanes.allSilent', 'Nothing arrived from any source in the last {{days}} days.', { days: ARRIVAL_DAYS })}</p>
          )}
          {groups.hidden > 0 ? (
            <Link className={styles.moreRow} to="/sources?section=connections">
              {t('home.lanes.more', '{{formatted}} more sources', { count: groups.hidden, formatted: n(groups.hidden) })}
              <span aria-hidden="true">→</span>
            </Link>
          ) : null}
        </>
      )}
    </Region>
  );
}
