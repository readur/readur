import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import type { LabelResponse, SourceResponse } from '../../services/api';
import { AlertsButton } from './AlertsButton';
import { HOME_PATH, type DestinationId } from './destinations';
import { LanguageMenu } from './LanguageMenu';
import { PrimaryNav } from './PrimaryNav';
import { SearchTrigger } from './SearchTrigger';
import { CollectionsList, SourcesList } from './SidebarLists';
import { SyncedReadout } from './SyncedReadout';
import { ThemeToggle } from './ThemeToggle';
import { UserMenu } from './UserMenu';
import styles from './Sidebar.module.css';

export interface SidebarProps {
  onOpenPalette: () => void;
  labels: LabelResponse[] | null;
  sources: SourceResponse[] | null;
  lastSynced: Date | null;
  /** Figures shown beside destinations, e.g. the library's document count. */
  counts?: Partial<Record<DestinationId, number>>;
  /** In the phone drawer the account tools already sit in the top bar. */
  showTools?: boolean;
}

/** Wordmark, search, destinations, collections and sources, with the account tools at the foot. */
export function Sidebar({ onOpenPalette, labels, sources, lastSynced, counts, showTools = true }: SidebarProps) {
  const { t } = useTranslation();
  return (
    <div className={styles.sidebar}>
      <div className={styles.top}>
        <Link to={HOME_PATH} className={styles.wordmark} aria-label={t('shell.home', 'Readur home')}>
          <img src="/readur-64.png" alt="" width={28} height={28} className={styles.logo} />
          <span className={styles.wordmarkText} aria-hidden="true">
            {t('common.appName', 'Readur')}
          </span>
        </Link>
        <SearchTrigger onOpen={onOpenPalette} compact={false} />
      </div>
      <div className={styles.scroll}>
        <PrimaryNav variant="side" label={t('shell.nav.label', 'Main')} counts={counts} />
        <CollectionsList labels={labels} />
        <SourcesList sources={sources} aside={<SyncedReadout last={lastSynced} />} />
      </div>
      {showTools ? (
        <div className={styles.foot}>
          <UserMenu />
          <div className={styles.tools}>
            <AlertsButton />
            <LanguageMenu />
            <ThemeToggle />
          </div>
        </div>
      ) : null}
    </div>
  );
}
