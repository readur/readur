import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { EmptyState, Skeleton, StatusMark } from '../../ui';
import { formatAge } from './format';
import { Region, RegionError } from './Region';
import { sourceState, type BoardSource } from './types';
import type { Resource } from './useResource';
import styles from './Board.module.css';

/** Compact list of connections with their state and last sync. */
export function ConnectionsPanel({ sources }: { sources: Resource<BoardSource[]> }) {
  const { t, i18n } = useTranslation();
  const { data, error, reload } = sources;
  const title = t('board.connections.title', 'Connections');

  return (
    <Region
      title={title}
      headerAction={
        <Link className={styles.link} to="/intake?section=connections">
          {t('board.connections.manage', 'Manage')}
        </Link>
      }
    >
      {error && !data ? (
        <RegionError message={t('board.connections.error', 'Connections could not be loaded.')} onRetry={reload} />
      ) : !data ? (
        <div className={styles.body}>
          <Skeleton lines={3} label={t('board.loading', 'Loading')} />
        </div>
      ) : data.length === 0 ? (
        <div className={styles.empty}>
          <EmptyState
            title={t('board.connections.empty', 'No connections')}
            action={
              <Link className={styles.linkButton} to="/intake?section=connections">
                {t('board.connections.add', 'Add connection')}
              </Link>
            }
          />
        </div>
      ) : (
        <ul className={styles.sources} aria-label={title}>
          {data.map((s) => (
            <li key={s.id} className={styles.source}>
              <span className={styles.sourceName}>{s.name}</span>
              <StatusMark state={sourceState(s)} size="sm" />
              <span className={styles.mono}>{s.last_sync_at ? formatAge(s.last_sync_at, i18n.language) : '—'}</span>
            </li>
          ))}
        </ul>
      )}
    </Region>
  );
}
