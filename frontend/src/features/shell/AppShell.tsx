import { Suspense, useCallback, useEffect, useState, type ReactNode } from 'react';
import { Link, Outlet, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { CommandPalette, IconButton, Skeleton, useCommandPaletteShortcut } from '../../ui';
import { Menu } from '../../ui/icons';
import { cx } from '../../ui/shared/FieldParts';
import { useLitFeeders } from '../board/litFeeders';
import { DocumentDrawerProvider } from '../document/drawer/DocumentDrawerContext';
import { DocumentDrawerHost } from '../document/drawer/DocumentDrawerHost';
import { AlertsButton } from './AlertsButton';
import { HOME_PATH } from './destinations';
import { MobileDrawer } from './MobileDrawer';
import { PrimaryNav } from './PrimaryNav';
import { SearchTrigger } from './SearchTrigger';
import { Sidebar } from './Sidebar';
import { useCollections } from './useCollections';
import { useLatestSync, useSourcesList } from './useLastSynced';
import { useDocumentTotal } from './useDocumentTotal';
import { SyncToasts } from './SyncToasts';
import { useMediaQuery } from './useMediaQuery';
import { usePaletteSources } from './usePaletteSources';
import styles from './AppShell.module.css';

const STANDALONE_QUERY = '(display-mode: standalone)';

/** Below this width the sidebar becomes a drawer and the tab bar appears. */
export const DRAWER_QUERY = '(max-width: 899px)';

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

/**
 * Left sidebar with destinations, collections and sources; on narrow screens a slim top bar opens
 * the same sidebar as a drawer and a tab bar holds the main destinations.
 */
export function AppShell({ children }: AppShellProps) {
  const { t } = useTranslation();
  const { key: locationKey } = useLocation();
  const isDrawerLayout = useMediaQuery(DRAWER_QUERY);
  const isStandalone = useIsStandalone();
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const openPalette = useCallback(() => {
    setDrawerOpen(false);
    setPaletteOpen(true);
  }, []);
  const paletteSources = usePaletteSources();
  const labels = useCollections();
  const sources = useSourcesList();
  const documentTotal = useDocumentTotal();
  const lastSynced = useLatestSync(sources);
  useCommandPaletteShortcut(openPalette);
  useLitFeeders();

  // Following a link in the drawer closes it.
  useEffect(() => setDrawerOpen(false), [locationKey]);
  useEffect(() => {
    if (!isDrawerLayout) setDrawerOpen(false);
  }, [isDrawerLayout]);

  // The installed app always gets the tab bar (CSS still hides it on wide screens).
  const showBottomBar = isDrawerLayout || isStandalone;
  const sidebar = (
    <Sidebar
      onOpenPalette={openPalette}
      labels={labels}
      sources={sources}
      lastSynced={lastSynced}
      counts={documentTotal !== null ? { library: documentTotal } : undefined}
    />
  );

  return (
    <DocumentDrawerProvider>
      <div className={cx(styles.shell, isDrawerLayout && styles.drawerLayout, showBottomBar && styles.withBottomBar)}>
        <a href="#main" className={styles.skipLink}>
          {t('shell.skipToContent', 'Skip to content')}
        </a>
        <SyncToasts sources={sources} />
        {isDrawerLayout ? (
          <header className={styles.mobileBar}>
            <IconButton
              label={t('shell.menu.open', 'Open menu')}
              icon={<Menu fontSize="inherit" />}
              onPress={() => setDrawerOpen(true)}
              aria-expanded={drawerOpen}
            />
            <Link to={HOME_PATH} className={styles.wordmark} aria-label={t('shell.home', 'Readur home')}>
              <img src="/readur-64.png" alt="" width={24} height={24} className={styles.logo} />
              <span className={styles.wordmarkText} aria-hidden="true">
                {t('common.appName', 'Readur')}
              </span>
            </Link>
            <div className={styles.mobileTools}>
              <SearchTrigger onOpen={openPalette} compact />
              <AlertsButton />
            </div>
          </header>
        ) : (
          <header className={styles.sidebarFrame}>{sidebar}</header>
        )}

        <main id="main" tabIndex={-1} className={styles.main}>
          <div className={styles.content}>
            <Suspense fallback={<PageFallback />}>{children ?? <Outlet />}</Suspense>
          </div>
        </main>

        {/* The document named by ?document=, over whatever page is showing. */}
        <DocumentDrawerHost />

        {isDrawerLayout ? (
          <MobileDrawer isOpen={drawerOpen} onOpenChange={setDrawerOpen}>
            {sidebar}
          </MobileDrawer>
        ) : null}

        {showBottomBar ? (
          <PrimaryNav
            variant="bottom"
            label={isDrawerLayout ? t('shell.nav.label', 'Main') : t('shell.nav.tabBar', 'Tab bar')}
          />
        ) : null}

        <CommandPalette
          isOpen={paletteOpen}
          onOpenChange={setPaletteOpen}
          sources={paletteSources}
          placeholder={t('shell.search.placeholder', 'Search documents…')}
        />
      </div>
    </DocumentDrawerProvider>
  );
}

export default AppShell;
