import { Suspense, useCallback, useState, type ReactNode } from 'react';
import { Link, Outlet } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { CommandPalette, Skeleton, useCommandPaletteShortcut } from '../../ui';
import { cx } from '../../ui/shared/FieldParts';
import { useLitFeeders } from '../board/litFeeders';
import { AlertsButton } from './AlertsButton';
import { LanguageMenu } from './LanguageMenu';
import { PrimaryNav } from './PrimaryNav';
import { SearchTrigger } from './SearchTrigger';
import { SyncedReadout } from './SyncedReadout';
import { ThemeToggle } from './ThemeToggle';
import { UserMenu } from './UserMenu';
import { useIsNarrow, useMediaQuery } from './useMediaQuery';
import { usePaletteSources } from './usePaletteSources';
import styles from './AppShell.module.css';

const STANDALONE_QUERY = '(display-mode: standalone)';

/** Installed-app mode (display-mode: standalone, or iOS home-screen). */
function useIsStandalone(): boolean {
  const standalone = useMediaQuery(STANDALONE_QUERY);
  const ios = typeof navigator !== 'undefined' && (navigator as Navigator & { standalone?: boolean }).standalone === true;
  return standalone || ios;
}

export function PageFallback() {
  const { t } = useTranslation();
  return (
    <div className={styles.fallback}>
      <Skeleton height={28} width="40%" label={t('shell.loading', 'Loading')} />
      <Skeleton lines={6} />
    </div>
  );
}

export interface AppShellProps {
  /** Page content. Defaults to the matched child route. */
  children?: ReactNode;
}

/** Top bar with the four destinations, search, sync readout, alerts and account; bottom tab bar on narrow screens. */
export function AppShell({ children }: AppShellProps) {
  const { t } = useTranslation();
  const isNarrow = useIsNarrow();
  const isStandalone = useIsStandalone();
  const [paletteOpen, setPaletteOpen] = useState(false);
  const openPalette = useCallback(() => setPaletteOpen(true), []);
  const sources = usePaletteSources();
  useCommandPaletteShortcut(openPalette);
  useLitFeeders();

  // The installed app always gets the tab bar (CSS still hides it on wide screens).
  const showBottomBar = isNarrow || isStandalone;
  const navLabel = t('shell.nav.label', 'Main');

  return (
    <div className={cx(styles.shell, showBottomBar && styles.withBottomBar)}>
      <a href="#main" className={styles.skipLink}>
        {t('shell.skipToContent', 'Skip to content')}
      </a>
      <header className={styles.topBar}>
        <Link to="/board" className={styles.wordmark} aria-label={t('shell.home', 'Readur home')}>
          <img src="/readur-64.png" alt="" width={20} height={20} className={styles.logo} />
          <span className={styles.wordmarkText} aria-hidden="true">
            {t('common.appName', 'Readur')}
          </span>
        </Link>
        {isNarrow ? null : <PrimaryNav variant="top" label={navLabel} />}
        <div className={styles.search}>
          <SearchTrigger onOpen={openPalette} compact={isNarrow} />
        </div>
        <div className={styles.tools}>
          <SyncedReadout />
          <AlertsButton />
          <LanguageMenu />
          <ThemeToggle />
          <UserMenu />
        </div>
      </header>

      <main id="main" tabIndex={-1} className={styles.main}>
        <div className={styles.content}>
          <Suspense fallback={<PageFallback />}>{children ?? <Outlet />}</Suspense>
        </div>
      </main>

      {showBottomBar ? (
        <PrimaryNav variant="bottom" label={isNarrow ? navLabel : t('shell.nav.tabBar', 'Tab bar')} />
      ) : null}

      <CommandPalette
        isOpen={paletteOpen}
        onOpenChange={setPaletteOpen}
        sources={sources}
        placeholder={t('shell.search.placeholder', 'Search documents…')}
      />
    </div>
  );
}

export default AppShell;
