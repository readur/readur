import type { ReactNode } from 'react';
import { Download, Home, LibraryIcon, Search, Settings } from '../../ui/icons';

export type DestinationId = 'home' | 'search' | 'library' | 'intake' | 'settings';

export interface Destination {
  id: DestinationId;
  /** i18n key and English fallback for the label. */
  labelKey: string;
  fallback: string;
  path: string;
  icon: ReactNode;
  /** Path prefixes that make this destination the current one. */
  matches: string[];
  /** Shown in the phone tab bar (Settings lives in the drawer instead). */
  inTabBar: boolean;
}

export const HOME_PATH = '/home';

export const DESTINATIONS: readonly Destination[] = [
  {
    id: 'home',
    labelKey: 'shell.nav.home',
    fallback: 'Home',
    path: HOME_PATH,
    icon: <Home fontSize="inherit" />,
    matches: ['/home'],
    inTabBar: true,
  },
  {
    id: 'search',
    labelKey: 'shell.nav.search',
    fallback: 'Search',
    path: '/search',
    icon: <Search fontSize="inherit" />,
    matches: ['/search'],
    inTabBar: true,
  },
  {
    id: 'library',
    labelKey: 'shell.nav.library',
    fallback: 'Library',
    path: '/documents',
    icon: <LibraryIcon fontSize="inherit" />,
    matches: ['/documents'],
    inTabBar: true,
  },
  {
    id: 'intake',
    labelKey: 'shell.nav.intake',
    fallback: 'Intake',
    path: '/intake',
    icon: <Download fontSize="inherit" />,
    matches: ['/intake'],
    inTabBar: true,
  },
  {
    id: 'settings',
    labelKey: 'shell.nav.settings',
    fallback: 'Settings',
    path: '/settings',
    icon: <Settings fontSize="inherit" />,
    matches: ['/settings'],
    inTabBar: false,
  },
];

/** The destination the pathname belongs to, if any. */
export function activeDestination(pathname: string): DestinationId | undefined {
  return DESTINATIONS.find((d) =>
    d.matches.some((m) => pathname === m || pathname.startsWith(`${m}/`)),
  )?.id;
}

export interface SectionLink {
  id: string;
  labelKey: string;
  fallback: string;
  path: string;
}

export const INTAKE_SECTIONS: readonly SectionLink[] = [
  { id: 'upload', labelKey: 'shell.intake.upload', fallback: 'Upload', path: '/intake?section=upload' },
  { id: 'connections', labelKey: 'shell.intake.connections', fallback: 'Connections', path: '/intake?section=connections' },
  { id: 'watch', labelKey: 'shell.intake.watch', fallback: 'Watch folder', path: '/intake?section=watch' },
  { id: 'attention', labelKey: 'shell.intake.attention', fallback: 'Needs attention', path: '/intake?section=attention' },
  { id: 'ignored', labelKey: 'shell.intake.ignored', fallback: 'Ignored files', path: '/intake?section=ignored' },
];

export const SETTINGS_SECTIONS: readonly SectionLink[] = [
  { id: 'general', labelKey: 'shell.settings.general', fallback: 'General', path: '/settings' },
  { id: 'labels', labelKey: 'shell.settings.labels', fallback: 'Labels', path: '/settings/labels' },
  { id: 'debug', labelKey: 'shell.settings.debug', fallback: 'Debug', path: '/settings/debug' },
];
