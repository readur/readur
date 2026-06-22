import React, { createContext, useContext, useState, useEffect, ReactNode } from 'react';
import { createTheme, Theme, ThemeProvider as MuiThemeProvider } from '@mui/material/styles';
import { PaletteMode } from '@mui/material';
import { modernTokens } from '../theme';
import { fontStack, gradients, radii, shadows as designShadows } from '../design/tokens';

interface ThemeContextType {
  mode: PaletteMode;
  toggleTheme: () => void;
  modernTokens: typeof modernTokens;
  /**
   * Legacy glass-effect helper. Returns an empty style object in the new
   * design system (we use solid surfaces with hairline borders instead).
   * Kept here so existing call-sites compile until they're swept in Phase 7.
   */
  glassEffect: (alphaValue?: number) => object;
}

const ThemeContext = createContext<ThemeContextType | undefined>(undefined);

export const useTheme = (): ThemeContextType => {
  const context = useContext(ThemeContext);
  if (!context) {
    throw new Error('useTheme must be used within a ThemeProvider');
  }
  return context;
};

interface ThemeProviderProps {
  children: ReactNode;
}

// No-op stand-in for the old glass effect — see Phase 7 cleanup.
const createGlassEffect = (_mode: PaletteMode) => (_alphaValue: number = 0.1) => ({});

const createAppTheme = (mode: PaletteMode): Theme => {
  return createTheme({
    palette: {
      mode,
      primary: {
        main: modernTokens.colors.primary[500],
        light: modernTokens.colors.primary[300],
        dark: modernTokens.colors.primary[700],
        50: modernTokens.colors.primary[50],
        100: modernTokens.colors.primary[100],
        200: modernTokens.colors.primary[200],
        300: modernTokens.colors.primary[300],
        400: modernTokens.colors.primary[400],
        500: modernTokens.colors.primary[500],
        600: modernTokens.colors.primary[600],
        700: modernTokens.colors.primary[700],
        800: modernTokens.colors.primary[800],
        900: modernTokens.colors.primary[900],
      },
      secondary: {
        main: modernTokens.colors.secondary[500],
        light: modernTokens.colors.secondary[300],
        dark: modernTokens.colors.secondary[700],
        50: modernTokens.colors.secondary[50],
        100: modernTokens.colors.secondary[100],
        200: modernTokens.colors.secondary[200],
        300: modernTokens.colors.secondary[300],
        400: modernTokens.colors.secondary[400],
        500: modernTokens.colors.secondary[500],
        600: modernTokens.colors.secondary[600],
        700: modernTokens.colors.secondary[700],
        800: modernTokens.colors.secondary[800],
        900: modernTokens.colors.secondary[900],
      },
      background: {
        default: mode === 'light' ? '#fafafa' : '#121212',
        paper: mode === 'light' ? '#ffffff' : '#1e1e1e',
      },
      text: {
        primary: mode === 'light' ? '#333333' : '#f8fafc',
        secondary: mode === 'light' ? '#666666' : '#cbd5e1',
      },
      success: {
        main: modernTokens.colors.success[500],
        light: modernTokens.colors.success[50],
        dark: modernTokens.colors.success[600],
        50: modernTokens.colors.success[50],
        100: mode === 'light' ? '#dcfce7' : '#14532d',
        200: mode === 'light' ? '#bbf7d0' : '#166534',
        300: mode === 'light' ? '#86efac' : '#15803d',
        400: mode === 'light' ? '#4ade80' : '#16a34a',
        500: modernTokens.colors.success[500],
        600: modernTokens.colors.success[600],
        700: mode === 'light' ? '#15803d' : '#4ade80',
        800: mode === 'light' ? '#166534' : '#86efac',
        900: mode === 'light' ? '#14532d' : '#dcfce7',
      },
      warning: {
        main: modernTokens.colors.warning[500],
        light: modernTokens.colors.warning[50],
        dark: modernTokens.colors.warning[600],
        50: modernTokens.colors.warning[50],
        100: mode === 'light' ? '#fef3c7' : '#78350f',
        200: mode === 'light' ? '#fde68a' : '#92400e',
        300: mode === 'light' ? '#fcd34d' : '#b45309',
        400: mode === 'light' ? '#fbbf24' : '#d97706',
        500: modernTokens.colors.warning[500],
        600: modernTokens.colors.warning[600],
        700: mode === 'light' ? '#b45309' : '#fbbf24',
        800: mode === 'light' ? '#92400e' : '#fcd34d',
        900: mode === 'light' ? '#78350f' : '#fef3c7',
      },
      error: {
        main: modernTokens.colors.error[500],
        light: modernTokens.colors.error[50],
        dark: modernTokens.colors.error[600],
        50: modernTokens.colors.error[50],
        100: mode === 'light' ? '#fee2e2' : '#7f1d1d',
        200: mode === 'light' ? '#fecaca' : '#991b1b',
        300: mode === 'light' ? '#fca5a5' : '#b91c1c',
        400: mode === 'light' ? '#f87171' : '#dc2626',
        500: modernTokens.colors.error[500],
        600: modernTokens.colors.error[600],
        700: mode === 'light' ? '#b91c1c' : '#f87171',
        800: mode === 'light' ? '#991b1b' : '#fca5a5',
        900: mode === 'light' ? '#7f1d1d' : '#fee2e2',
      },
      info: {
        main: modernTokens.colors.info[500],
        light: modernTokens.colors.info[50],
        dark: modernTokens.colors.info[600],
        50: modernTokens.colors.info[50],
        100: mode === 'light' ? '#dbeafe' : '#1e3a8a',
        200: mode === 'light' ? '#bfdbfe' : '#1e40af',
        300: mode === 'light' ? '#93c5fd' : '#1d4ed8',
        400: mode === 'light' ? '#60a5fa' : '#2563eb',
        500: modernTokens.colors.info[500],
        600: modernTokens.colors.info[600],
        700: mode === 'light' ? '#1d4ed8' : '#60a5fa',
        800: mode === 'light' ? '#1e40af' : '#93c5fd',
        900: mode === 'light' ? '#1e3a8a' : '#dbeafe',
      },
      divider: mode === 'light' ? modernTokens.colors.neutral[200] : modernTokens.colors.neutral[800],
    },
    typography: {
      fontFamily: fontStack.sans,
      h1: { fontSize: '2.25rem', fontWeight: 800, lineHeight: 1.2, letterSpacing: '-0.025em' },
      h2: { fontSize: '1.875rem', fontWeight: 700, lineHeight: 1.2, letterSpacing: '-0.02em' },
      h3: { fontSize: '1.5rem', fontWeight: 700, lineHeight: 1.2, letterSpacing: '-0.015em' },
      h4: { fontSize: '1.25rem', fontWeight: 600, lineHeight: 1.4 },
      h5: { fontSize: '1.125rem', fontWeight: 600, lineHeight: 1.5 },
      h6: { fontSize: '1rem', fontWeight: 600, lineHeight: 1.5 },
      body1: { fontSize: '0.9375rem', fontWeight: 400, lineHeight: 1.55 },
      body2: { fontSize: '0.875rem', fontWeight: 400, lineHeight: 1.5 },
      caption: { fontSize: '0.75rem', fontWeight: 400, lineHeight: 1.5 },
    },
    shape: { borderRadius: radii.lg },
    components: {
      MuiButton: {
        styleOverrides: {
          root: {
            textTransform: 'none',
            borderRadius: radii.md,
            fontWeight: 500,
            boxShadow: 'none',
            '&:hover': { boxShadow: designShadows.sm },
          },
          contained: {
            background: gradients.accent,
            color: '#FFFFFF',
            '&:hover': { background: gradients.accentHover },
          },
        },
      },
      MuiCard: {
        styleOverrides: {
          root: {
            borderRadius: radii.xl,
            border: mode === 'light'
              ? `1px solid ${modernTokens.colors.neutral[200]}`
              : `1px solid ${modernTokens.colors.neutral[800]}`,
            boxShadow: designShadows.xs,
            backgroundColor: mode === 'light' ? '#FFFFFF' : modernTokens.colors.neutral[900],
            backgroundImage: 'none',
          },
        },
      },
      MuiPaper: {
        styleOverrides: {
          root: {
            borderRadius: radii.lg,
            backgroundColor: mode === 'light' ? '#FFFFFF' : modernTokens.colors.neutral[900],
            backgroundImage: 'none',
            boxShadow: designShadows.xs,
          },
        },
      },
      MuiAppBar: {
        styleOverrides: {
          root: {
            backgroundColor: mode === 'light' ? '#FFFFFF' : modernTokens.colors.neutral[900],
            color: mode === 'light' ? modernTokens.colors.neutral[900] : modernTokens.colors.neutral[50],
            backgroundImage: 'none',
            borderBottom: mode === 'light'
              ? `1px solid ${modernTokens.colors.neutral[200]}`
              : `1px solid ${modernTokens.colors.neutral[800]}`,
            boxShadow: 'none',
          },
        },
      },
      MuiDrawer: {
        styleOverrides: {
          paper: {
            backgroundColor: mode === 'light' ? '#FFFFFF' : modernTokens.colors.neutral[900],
            backgroundImage: 'none',
            borderRight: mode === 'light'
              ? `1px solid ${modernTokens.colors.neutral[200]}`
              : `1px solid ${modernTokens.colors.neutral[800]}`,
          },
        },
      },
      MuiTextField: {
        styleOverrides: {
          root: {
            '& .MuiOutlinedInput-root': {
              borderRadius: radii.md,
              '& fieldset': {
                borderColor: mode === 'light'
                  ? modernTokens.colors.neutral[300]
                  : modernTokens.colors.neutral[700],
              },
              '&:hover fieldset': {
                borderColor: mode === 'light'
                  ? modernTokens.colors.neutral[500]
                  : modernTokens.colors.neutral[500],
              },
              '&.Mui-focused fieldset': {
                borderColor: modernTokens.colors.primary[500],
                borderWidth: 1,
              },
            },
          },
        },
      },
      MuiChip: {
        styleOverrides: {
          root: { borderRadius: radii.sm, fontWeight: 500 },
        },
      },
      MuiTab: {
        styleOverrides: {
          root: {
            textTransform: 'none',
            fontWeight: 500,
            fontSize: '0.8125rem',
            minHeight: 44,
            '&.Mui-selected': { fontWeight: 600 },
          },
        },
      },
      MuiTableHead: {
        styleOverrides: {
          root: {
            '& .MuiTableCell-head': {
              fontFamily: fontStack.sans,
              fontWeight: 600,
              fontSize: 10,
              letterSpacing: '0.06em',
              textTransform: 'uppercase',
              color: mode === 'light' ? modernTokens.colors.neutral[500] : modernTokens.colors.neutral[400],
              backgroundColor: mode === 'light' ? modernTokens.colors.neutral[100] : modernTokens.colors.neutral[800],
              borderBottom: mode === 'light'
                ? `1px solid ${modernTokens.colors.neutral[200]}`
                : `1px solid ${modernTokens.colors.neutral[700]}`,
            },
          },
        },
      },
      MuiTableRow: {
        styleOverrides: {
          root: {
            '&:hover': {
              backgroundColor: mode === 'light'
                ? modernTokens.colors.primary[50]
                : 'rgba(99, 102, 241, 0.08)',
            },
          },
        },
      },
      MuiTableCell: {
        styleOverrides: {
          root: {
            borderBottom: mode === 'light'
              ? `1px solid ${modernTokens.colors.neutral[200]}`
              : `1px solid ${modernTokens.colors.neutral[800]}`,
            fontSize: 13,
          },
        },
      },
      MuiAccordion: {
        styleOverrides: {
          root: {
            boxShadow: 'none',
            border: mode === 'light'
              ? `1px solid ${modernTokens.colors.neutral[200]}`
              : `1px solid ${modernTokens.colors.neutral[800]}`,
            borderRadius: radii.md,
            backgroundImage: 'none',
            '&:before': { display: 'none' },
            '&.Mui-expanded': { margin: 0 },
          },
        },
      },
      MuiMenu: {
        styleOverrides: {
          paper: {
            borderRadius: radii.md,
            border: mode === 'light'
              ? `1px solid ${modernTokens.colors.neutral[200]}`
              : `1px solid ${modernTokens.colors.neutral[800]}`,
            boxShadow: designShadows.lg,
          },
        },
      },
      MuiDialog: {
        styleOverrides: {
          paper: {
            borderRadius: radii.xl,
            boxShadow: designShadows.xl,
          },
        },
      },
    },
  });
};

export const ThemeProvider: React.FC<ThemeProviderProps> = ({ children }) => {
  const [mode, setMode] = useState<PaletteMode>(() => {
    const savedMode = localStorage.getItem('themeMode');
    if (savedMode === 'light' || savedMode === 'dark') {
      return savedMode;
    }
    // Default to system preference or light mode
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  });

  const toggleTheme = () => {
    const newMode = mode === 'light' ? 'dark' : 'light';
    setMode(newMode);
    localStorage.setItem('themeMode', newMode);
  };

  const theme = createAppTheme(mode);
  const glassEffect = createGlassEffect(mode);

  // Sync `<html class="dark">` so CSS custom properties in
  // design/global.css flip alongside the MUI palette.
  useEffect(() => {
    document.documentElement.classList.toggle('dark', mode === 'dark');
  }, [mode]);

  // Listen for system theme changes
  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const handleChange = (e: MediaQueryListEvent) => {
      // Only update if user hasn't manually set a preference
      if (!localStorage.getItem('themeMode')) {
        setMode(e.matches ? 'dark' : 'light');
      }
    };

    mediaQuery.addEventListener('change', handleChange);
    return () => mediaQuery.removeEventListener('change', handleChange);
  }, []);

  return (
    <ThemeContext.Provider value={{ mode, toggleTheme, modernTokens, glassEffect }}>
      <MuiThemeProvider theme={theme}>
        {children}
      </MuiThemeProvider>
    </ThemeContext.Provider>
  );
};