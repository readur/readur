/** Old URLs and where they live now. Other query parameters on the old URL are carried over. */
export interface LegacyRoute {
  from: string;
  to: string;
}

export const LEGACY_ROUTES: readonly LegacyRoute[] = [
  { from: '/', to: '/board' },
  { from: '/dashboard', to: '/board' },
  { from: '/upload', to: '/intake?section=upload' },
  { from: '/sources', to: '/intake?section=connections' },
  { from: '/watch', to: '/intake?section=watch' },
  { from: '/documents/management', to: '/intake?section=attention' },
  { from: '/ignored-files', to: '/intake?section=ignored' },
  { from: '/labels', to: '/settings/labels' },
  { from: '/debug', to: '/settings/debug' },
  { from: '/profile', to: '/settings/account' },
];

/**
 * Joins a redirect target with the query string of the URL being redirected. Parameters named
 * in the target win; every other incoming parameter is kept.
 */
export function mergeSearch(to: string, incomingSearch: string): { pathname: string; search: string } {
  const [pathname, targetQuery = ''] = to.split('?');
  const merged = new URLSearchParams(incomingSearch);
  new URLSearchParams(targetQuery).forEach((value, key) => merged.set(key, value));
  const search = merged.toString();
  return { pathname, search: search ? `?${search}` : '' };
}
