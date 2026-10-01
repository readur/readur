import { useContext, useEffect, useState } from 'react';
import { ThemeModeContext } from '../../theme/useThemeMode';

const QUERY = '(prefers-reduced-motion: reduce)';

function getQuery(): MediaQueryList | null {
  try {
    return typeof window !== 'undefined' && typeof window.matchMedia === 'function'
      ? window.matchMedia(QUERY)
      : null;
  } catch {
    return null;
  }
}

/**
 * Whether the user asked for reduced motion. Reads `useThemeMode().prefersReducedMotion`
 * when a theme provider is mounted and falls back to the media query otherwise, so
 * primitives stay usable (and testable) outside the provider.
 */
export function usePrefersReducedMotion(): boolean {
  const theme = useContext(ThemeModeContext);
  const [fallback, setFallback] = useState<boolean>(() => Boolean(getQuery()?.matches));
  const hasTheme = theme !== undefined;

  useEffect(() => {
    if (hasTheme) return undefined;
    const mql = getQuery();
    if (!mql || typeof mql.addEventListener !== 'function') return undefined;
    const onChange = (e: MediaQueryListEvent) => setFallback(e.matches);
    mql.addEventListener('change', onChange);
    return () => mql.removeEventListener('change', onChange);
  }, [hasTheme]);

  return hasTheme ? theme.prefersReducedMotion : fallback;
}
