import type { ReactNode } from 'react';
import { Dashboard, Description, Download, Settings } from '../../ui/icons';

export type DestinationId = 'board' | 'library' | 'intake' | 'settings';

export interface Destination {
  id: DestinationId;
  /** i18n key and English fallback for the label. */
  labelKey: string;
  fallback: string;
  path: string;
  icon: ReactNode;
  /** Path prefixes that make this destination the current one. */
  matches: string[];
}

export const DESTINATIONS: readonly Destination[] = [
  {
    id: 'board',
    labelKey: 'shell.nav.board',
    fallback: 'Board',
    path: '/board',
    icon: <Dashboard fontSize="inherit" />,
    matches: ['/board'],
  },
  {
    id: 'library',
    labelKey: 'shell.nav.library',
    fallback: 'Library',
    path: '/documents',
    icon: <Description fontSize="inherit" />,
    matches: ['/documents', '/search'],
  },
  {
    id: 'intake',
    labelKey: 'shell.nav.intake',
    fallback: 'Intake',
    path: '/intake',
    icon: <Download fontSize="inherit" />,
    matches: ['/intake'],
  },
  {
    id: 'settings',
    labelKey: 'shell.nav.settings',
    fallback: 'Settings',
    path: '/settings',
    icon: <Settings fontSize="inherit" />,
    matches: ['/settings'],
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
