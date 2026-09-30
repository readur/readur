import React, { useCallback, useEffect, useMemo, useState, ReactNode } from 'react';
import { ThemeModeContext, ThemeModeName, ThemeModeValue } from './useThemeMode';

const STORAGE_KEY = 'themeMode';
const DARK_QUERY = '(prefers-color-scheme: dark)';
const MOTION_QUERY = '(prefers-reduced-motion: reduce)';

const getMql = (query: string): MediaQueryList | null => {
  try {
    return typeof window !== 'undefined' && typeof window.matchMedia === 'function'
      ? window.matchMedia(query)
      : null;
  } catch {
    return null;
  }
};

const readSaved = (): ThemeModeName | null => {
  try {
    const v = window.localStorage.getItem(STORAGE_KEY);
    return v === 'light' || v === 'dark' ? v : null;
  } catch {
    return null;
  }
};

const initialMode = (): ThemeModeName =>
  readSaved() ?? (getMql(DARK_QUERY)?.matches ? 'dark' : 'light');

const subscribe = (mql: MediaQueryList | null, fn: (e: MediaQueryListEvent) => void) => {
  if (!mql || typeof mql.addEventListener !== 'function') return () => {};
  mql.addEventListener('change', fn);
  return () => mql.removeEventListener('change', fn);
};

export const ThemeModeProvider: React.FC<{ children: ReactNode }> = ({ children }) => {
  const [mode, setModeState] = useState<ThemeModeName>(initialMode);
  const [prefersReducedMotion, setReduced] = useState<boolean>(
    () => !!getMql(MOTION_QUERY)?.matches,
  );

  const setMode = useCallback((next: ThemeModeName) => {
    setModeState(next);
    try {
      window.localStorage.setItem(STORAGE_KEY, next);
    } catch {
      /* storage unavailable: the mode still applies for this session */
    }
  }, []);

  const toggle = useCallback(() => {
    setMode(mode === 'light' ? 'dark' : 'light');
  }, [mode, setMode]);

  useEffect(() => {
    const root = document.documentElement;
    root.dataset.theme = mode;
  }, [mode]);

  useEffect(
    () =>
      subscribe(getMql(DARK_QUERY), (e) => {
        if (!readSaved()) setModeState(e.matches ? 'dark' : 'light');
      }),
    [],
  );

  useEffect(() => subscribe(getMql(MOTION_QUERY), (e) => setReduced(e.matches)), []);

  const value = useMemo<ThemeModeValue>(
    () => ({ mode, setMode, toggle, prefersReducedMotion }),
    [mode, setMode, toggle, prefersReducedMotion],
  );

  return <ThemeModeContext.Provider value={value}>{children}</ThemeModeContext.Provider>;
};

export { ThemeModeProvider as ThemeProvider };
