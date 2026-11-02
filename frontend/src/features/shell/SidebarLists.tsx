import type { CSSProperties, ReactNode } from 'react';
import { Link, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import type { LabelResponse, SourceResponse } from '../../services/api';
import { SourceTile } from '../../ui';
import { cx } from '../../ui/shared/FieldParts';
import { topCollections } from './useCollections';
import styles from './Sidebar.module.css';

interface ListProps {
  /** Accessible name of the nav landmark, also shown as its heading. */
  title: string;
  /** Right-aligned text beside the heading, e.g. the last sync time. */
  aside?: ReactNode;
  children: ReactNode;
}

function SidebarList({ title, aside, children }: ListProps) {
  return (
    <nav aria-label={title} className={styles.list}>
      <div className={styles.listHead}>
        <h2 className={styles.listTitle}>{title}</h2>
        {aside}
      </div>
      <ul className={styles.items}>{children}</ul>
    </nav>
  );
}

interface ItemProps {
  to: string;
  isCurrent: boolean;
  mark: ReactNode;
  name: string;
  trailing?: ReactNode;
}

function SidebarItem({ to, isCurrent, mark, name, trailing }: ItemProps) {
  return (
    <li>
      <Link to={to} className={cx(styles.item, isCurrent && styles.itemCurrent)} aria-current={isCurrent ? 'page' : undefined}>
        {mark}
        <span className={styles.itemName} title={name}>
          {name}
        </span>
        {trailing}
      </Link>
    </li>
  );
}

export const collectionPath = (id: string) => `/documents?label=${encodeURIComponent(id)}`;

/** The user's labels, most-used first, each linking to its documents. */
export function CollectionsList({ labels }: { labels: LabelResponse[] | null }) {
  const { t } = useTranslation();
  const { pathname, search } = useLocation();
  const currentLabel = pathname === '/documents' ? new URLSearchParams(search).get('label') : null;
  const shown = labels ? topCollections(labels) : [];

  return (
    <SidebarList title={t('shell.collections.title', 'Collections')}>
      {labels && shown.length === 0 ? <li className={styles.empty}>{t('shell.collections.empty', 'No collections yet')}</li> : null}
      {shown.map((label) => (
        <SidebarItem
          key={label.id}
          to={collectionPath(label.id)}
          isCurrent={currentLabel === label.id}
          name={label.name}
          mark={
            <span
              className={styles.labelDot}
              style={{ '--label-color': label.color } as CSSProperties}
              aria-hidden="true"
            />
          }
          trailing={<span className={styles.count}>{label.document_count}</span>}
        />
      ))}
      <li>
        <Link to="/settings/labels" className={styles.more}>
          {t('shell.collections.all', 'All collections')}
        </Link>
      </li>
    </SidebarList>
  );
}

type Health = 'syncing' | 'error' | 'check' | 'off';

/**
 * A word for anything other than healthy (healthy sources stay quiet), in the same order as the
 * connections list: off, syncing, error, then the last health check.
 */
export function sourceHealth(
  source: Pick<SourceResponse, 'enabled' | 'status'> & { validation_status?: string | null },
): Health | null {
  if (!source.enabled) return 'off';
  if (source.status === 'syncing') return 'syncing';
  if (source.status === 'error' || source.validation_status === 'critical') return 'error';
  if (source.validation_status === 'warning') return 'check';
  return null;
}

const HEALTH_FALLBACK: Record<Health, string> = { syncing: 'Syncing', error: 'Error', check: 'Check', off: 'Off' };

export const sourcePath = (id: string) => `/sources?section=connections&source=${encodeURIComponent(id)}`;

/** Uploads, the watch folder and every configured source, each in its own colour. */
export function SourcesList({ sources, aside }: { sources: SourceResponse[] | null; aside?: ReactNode }) {
  const { t } = useTranslation();
  const { pathname, search } = useLocation();
  const params = pathname === '/intake' || pathname === '/sources' ? new URLSearchParams(search) : null;
  const section = params?.get('section') ?? (pathname === '/sources' ? 'connections' : null);
  const currentSource = pathname === '/sources' && section === 'connections' ? params?.get('source') : null;
  const sorted = (sources ?? [])
    .filter((s) => typeof s?.id === 'string' && typeof s.name === 'string')
    .sort((a, b) => a.name.localeCompare(b.name));

  return (
    <SidebarList title={t('shell.sources.title', 'Sources')} aside={aside}>
      <SidebarItem
        to="/intake?section=upload"
        isCurrent={pathname === '/intake' && section === 'upload'}
        name={t('shell.sources.uploads', 'Uploads')}
        mark={<SourceTile sourceId={null} kind="upload" />}
      />
      <SidebarItem
        to="/sources?section=watch"
        isCurrent={pathname === '/sources' && section === 'watch'}
        name={t('shell.sources.watch', 'Watch folder')}
        mark={<SourceTile sourceId={null} kind="watch" />}
      />
      {sorted.map((source) => {
        const health = sourceHealth(source);
        return (
          <SidebarItem
            key={source.id}
            to={sourcePath(source.id)}
            isCurrent={currentSource === source.id}
            name={source.name}
            mark={<SourceTile sourceId={source.id} kind={source.source_type} />}
            trailing={
              health ? (
                <span className={cx(styles.health, styles[`health-${health}`])}>
                  {t(`shell.sources.health.${health}`, HEALTH_FALLBACK[health])}
                </span>
              ) : null
            }
          />
        );
      })}
    </SidebarList>
  );
}
