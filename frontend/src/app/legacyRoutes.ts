/** Old URLs and where they live now. Other query parameters on the old URL are carried over. */
export interface LegacyRoute {
  from: string;
  to: string;
}

export const LEGACY_ROUTES: readonly LegacyRoute[] = [
  { from: '/', to: '/home' },
  { from: '/board', to: '/home' },
  { from: '/dashboard', to: '/home' },
  { from: '/upload', to: '/intake?section=upload' },
  { from: '/watch', to: '/sources?section=watch' },
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

/**
 * For a /search query string that still uses the older `query` parameter, the same string with
 * it renamed to `q` (an existing `q` wins). Null when there is nothing to rename.
 */
export function searchAliasTarget(incomingSearch: string): string | null {
  const params = new URLSearchParams(incomingSearch);
  if (!params.has('query')) return null;
  const query = params.get('query') ?? '';
  params.delete('query');
  const next = new URLSearchParams();
  const q = params.get('q') ?? query;
  if (q) next.set('q', q);
  params.delete('q');
  params.forEach((value, key) => next.append(key, value));
  const search = next.toString();
  return search ? `?${search}` : '';
}
