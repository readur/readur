import React, { createContext, useContext, useState, useEffect, ReactNode } from 'react';
import { createTheme, Theme, ThemeProvider as MuiThemeProvider } from '@mui/material/styles';
import { PaletteMode } from '@mui/material';
import { modernTokens } from '../theme';

interface ThemeContextType {
  mode: PaletteMode;
  toggleTheme: () => void;
  modernTokens: typeof modernTokens;
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

// Glassmorphism effect that adapts to theme mode
const createGlassEffect = (mode: PaletteMode) => (alphaValue: number = 0.1) => ({
  background: mode === 'light' 
    ? `rgba(255, 255, 255, ${alphaValue})` 
    : `rgba(30, 30, 30, ${alphaValue})`,
  backdropFilter: 'blur(10px)',
  border: mode === 'light' 
    ? '1px solid rgba(255, 255, 255, 0.2)' 
    : '1px solid rgba(255, 255, 255, 0.1)',
  boxShadow: mode === 'light' 
    ? modernTokens.shadows.glass 
    : '0 8px 32px 0 rgba(0, 0, 0, 0.37)',
});

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
      divider: mode === 'light' ? 'rgba(0, 0, 0, 0.12)' : 'rgba(255, 255, 255, 0.12)',
    },
    typography: {
      fontFamily: [
        '-apple-system',
        'BlinkMacSystemFont',
        '"Segoe UI"',
        'Roboto',
        '"Helvetica Neue"',
        'Arial',
        'sans-serif',
      ].join(','),
      h4: {
        fontWeight: 600,
      },
      h5: {
        fontWeight: 600,
      },
      h6: {
        fontWeight: 600,
      },
    },
    components: {
      MuiButton: {
        styleOverrides: {
          root: {
            textTransform: 'none',
            borderRadius: 8,
          },
        },
      },
      MuiCard: {
        styleOverrides: {
          root: {
            borderRadius: 12,
            boxShadow: mode === 'light' 
              ? '0 2px 8px rgba(0,0,0,0.1)' 
              : '0 2px 8px rgba(0,0,0,0.3)',
            backgroundColor: mode === 'light' ? '#ffffff' : '#1e1e1e',
          },
        },
      },
      MuiPaper: {
        styleOverrides: {
          root: {
            borderRadius: 8,
            backgroundColor: mode === 'light' ? '#ffffff' : '#1e1e1e',
          },
        },
      },
      MuiAppBar: {
        styleOverrides: {
          root: {
            backgroundColor: mode === 'light' 
              ? 'rgba(255, 255, 255, 0.95)' 
              : 'rgba(30, 30, 30, 0.95)',
            backdropFilter: 'blur(20px)',
          },
        },
      },
      MuiDrawer: {
        styleOverrides: {
          paper: {
            backgroundColor: mode === 'light' ? '#ffffff' : '#1e1e1e',
            borderRight: mode === 'light' 
              ? '1px solid rgba(0, 0, 0, 0.12)' 
              : '1px solid rgba(255, 255, 255, 0.12)',
          },
        },
      },
      MuiTextField: {
        styleOverrides: {
          root: {
            '& .MuiOutlinedInput-root': {
              '& fieldset': {
                borderColor: mode === 'light' ? 'rgba(0, 0, 0, 0.23)' : 'rgba(255, 255, 255, 0.23)',
              },
              '&:hover fieldset': {
                borderColor: mode === 'light' ? 'rgba(0, 0, 0, 0.87)' : 'rgba(255, 255, 255, 0.87)',
              },
            },
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