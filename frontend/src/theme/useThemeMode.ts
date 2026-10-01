import { createContext, useContext } from 'react';

export type ThemeModeName = 'light' | 'dark';

export interface ThemeModeValue {
  mode: ThemeModeName;
  setMode: (mode: ThemeModeName) => void;
  toggle: () => void;
  prefersReducedMotion: boolean;
}

export const ThemeModeContext = createContext<ThemeModeValue | undefined>(undefined);

export const useThemeMode = (): ThemeModeValue => {
  const ctx = useContext(ThemeModeContext);
  if (!ctx) {
    throw new Error('useThemeMode must be used within a ThemeModeProvider');
  }
  return ctx;
};
