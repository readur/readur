import { useEffect, useState } from 'react';

/** Below this width the top-bar nav collapses into the bottom tab bar. */
export const NARROW_QUERY = '(max-width: 719px)';

function query(q: string): MediaQueryList | null {
  try {
    if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return null;
    return window.matchMedia(q) ?? null;
  } catch {
    return null;
  }
}

/** Live `matchMedia` result; false wherever media queries are unavailable. */
export function useMediaQuery(q: string): boolean {
  const [matches, setMatches] = useState(() => Boolean(query(q)?.matches));

  useEffect(() => {
    const mql = query(q);
    setMatches(Boolean(mql?.matches));
    if (!mql || typeof mql.addEventListener !== 'function') return undefined;
    const onChange = (e: MediaQueryListEvent) => setMatches(e.matches);
    mql.addEventListener('change', onChange);
    return () => mql.removeEventListener('change', onChange);
  }, [q]);

  return matches;
}

export const useIsNarrow = (): boolean => useMediaQuery(NARROW_QUERY);
