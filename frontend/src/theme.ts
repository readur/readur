import { createTheme, alpha } from '@mui/material/styles';

// Modern 2026 design tokens
export const modernTokens = {
  colors: {
    primary: {
      50: '#f0f4ff',
      100: '#e0eaff',
      200: '#c7d7fe',
      300: '#a5b8fc',
      400: '#8b93f8',
      500: '#6366f1',
      600: '#4f46e5',
      700: '#4338ca',
      800: '#3730a3',
      900: '#312e81',
    },
    secondary: {
      50: '#fdf4ff',
      100: '#fae8ff',
      200: '#f5d0fe',
      300: '#f0abfc',
      400: '#e879f9',
      500: '#d946ef',
      600: '#c026d3',
      700: '#a21caf',
      800: '#86198f',
      900: '#701a75',
    },
    neutral: {
      0: '#ffffff',
      50: '#fafafa',
      100: '#f5f5f5',
      200: '#e5e5e5',
      300: '#d4d4d4',
      400: '#a3a3a3',
      500: '#737373',
      600: '#525252',
      700: '#404040',
      800: '#262626',
      900: '#171717',
      950: '#0a0a0a',
    },
    success: {
      50: '#f0fdf4',
      100: '#dcfce7',
      200: '#bbf7d0',
      300: '#86efac',
      400: '#4ade80',
      500: '#22c55e',
      600: '#16a34a',
      700: '#15803d',
      800: '#166534',
      900: '#14532d',
    },
    warning: {
      50: '#fffbeb',
      100: '#fef3c7',
      200: '#fde68a',
      300: '#fcd34d',
      400: '#fbbf24',
      500: '#f59e0b',
      600: '#d97706',
      700: '#b45309',
      800: '#92400e',
      900: '#78350f',
    },
    error: {
      50: '#fef2f2',
      100: '#fee2e2',
      200: '#fecaca',
      300: '#fca5a5',
      400: '#f87171',
      500: '#ef4444',
      600: '#dc2626',
      700: '#b91c1c',
      800: '#991b1b',
      900: '#7f1d1d',
    },
    info: {
      50: '#eff6ff',
      100: '#dbeafe',
      200: '#bfdbfe',
      300: '#93c5fd',
      400: '#60a5fa',
      500: '#3b82f6',
      600: '#2563eb',
      700: '#1d4ed8',
      800: '#1e40af',
      900: '#1e3a8a',
    },
    blue: {
      50: '#eff6ff',
      100: '#dbeafe',
      200: '#bfdbfe',
      300: '#93c5fd',
      400: '#60a5fa',
      500: '#3b82f6',
      600: '#2563eb',
      700: '#1d4ed8',
      800: '#1e40af',
      900: '#1e3a8a',
    },
    orange: {
      50: '#fff7ed',
      100: '#ffedd5',
      200: '#fed7aa',
      300: '#fdba74',
      400: '#fb923c',
      500: '#f97316',
      600: '#ea580c',
      700: '#c2410c',
      800: '#9a3412',
      900: '#7c2d12',
    },
    green: {
      50: '#f0fdf4',
      100: '#dcfce7',
      200: '#bbf7d0',
      300: '#86efac',
      400: '#4ade80',
      500: '#22c55e',
      600: '#16a34a',
      700: '#15803d',
      800: '#166534',
      900: '#14532d',
    },
  },
  shadows: {
    xs: '0 1px 2px 0 rgb(0 0 0 / 0.05)',
    sm: '0 1px 3px 0 rgb(0 0 0 / 0.1), 0 1px 2px -1px rgb(0 0 0 / 0.1)',
    md: '0 4px 6px -1px rgb(0 0 0 / 0.1), 0 2px 4px -2px rgb(0 0 0 / 0.1)',
    lg: '0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1)',
    xl: '0 20px 25px -5px rgb(0 0 0 / 0.1), 0 8px 10px -6px rgb(0 0 0 / 0.1)',
    glass: '0 8px 32px 0 rgba(31, 38, 135, 0.37)',
  },
};

// Glassmorphism effect helper
export const glassEffect = (alphaValue: number = 0.1) => ({
  background: `rgba(255, 255, 255, ${alphaValue})`,
  backdropFilter: 'blur(10px)',
  border: '1px solid rgba(255, 255, 255, 0.2)',
  boxShadow: modernTokens.shadows.glass,
});

// Modern card style
export const modernCard = {
  borderRadius: 16,
  boxShadow: modernTokens.shadows.md,
  border: `1px solid ${modernTokens.colors.neutral[200]}`,
  background: modernTokens.colors.neutral[0],
  transition: 'all 0.2s ease-in-out',
  '&:hover': {
    boxShadow: modernTokens.shadows.lg,
    transform: 'translateY(-1px)',
  },
};

const theme = createTheme({
  palette: {
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
    },
    background: {
      default: modernTokens.colors.neutral[50],
      paper: modernTokens.colors.neutral[0],
    },
    text: {
      primary: modernTokens.colors.neutral[900],
      secondary: modernTokens.colors.neutral[600],
    },
    divider: modernTokens.colors.neutral[200],
    success: {
      main: modernTokens.colors.success[500],
      light: modernTokens.colors.success[50],
      dark: modernTokens.colors.success[600],
    },
    warning: {
      main: modernTokens.colors.warning[500],
      light: modernTokens.colors.warning[50],
      dark: modernTokens.colors.warning[600],
    },
    error: {
      main: modernTokens.colors.error[500],
      light: modernTokens.colors.error[50],
      dark: modernTokens.colors.error[600],
    },
    info: {
      main: modernTokens.colors.info[500],
      light: modernTokens.colors.info[50],
      dark: modernTokens.colors.info[600],
    },
  },
  typography: {
    fontFamily: [
      '"Inter"',
      '-apple-system',
      'BlinkMacSystemFont',
      '"Segoe UI"',
      'Roboto',
      '"Helvetica Neue"',
      'Arial',
      'sans-serif',
    ].join(','),
    h1: {
      fontSize: '2.25rem',
      fontWeight: 800,
      lineHeight: 1.2,
    },
    h2: {
      fontSize: '1.875rem',
      fontWeight: 700,
      lineHeight: 1.2,
    },
    h3: {
      fontSize: '1.5rem',
      fontWeight: 700,
      lineHeight: 1.2,
    },
    h4: {
      fontSize: '1.25rem',
      fontWeight: 600,
      lineHeight: 1.5,
    },
    h5: {
      fontSize: '1.125rem',
      fontWeight: 600,
      lineHeight: 1.5,
    },
    h6: {
      fontSize: '1rem',
      fontWeight: 600,
      lineHeight: 1.5,
    },
    body1: {
      fontSize: '1rem',
      fontWeight: 400,
      lineHeight: 1.75,
    },
    body2: {
      fontSize: '0.875rem',
      fontWeight: 400,
      lineHeight: 1.5,
    },
    caption: {
      fontSize: '0.75rem',
      fontWeight: 400,
      lineHeight: 1.5,
    },
  },
  shape: {
    borderRadius: 12,
  },
  components: {
    MuiButton: {
      styleOverrides: {
        root: {
          textTransform: 'none',
          borderRadius: 8,
          fontWeight: 500,
          boxShadow: 'none',
          '&:hover': {
            boxShadow: modernTokens.shadows.sm,
          },
        },
        contained: {
          background: `linear-gradient(135deg, ${modernTokens.colors.primary[500]} 0%, ${modernTokens.colors.primary[600]} 100%)`,
          '&:hover': {
            background: `linear-gradient(135deg, ${modernTokens.colors.primary[600]} 0%, ${modernTokens.colors.primary[700]} 100%)`,
          },
        },
      },
    },
    MuiCard: {
      styleOverrides: {
        root: modernCard,
      },
    },
    MuiPaper: {
      styleOverrides: {
        root: {
          borderRadius: 12,
          boxShadow: modernTokens.shadows.sm,
        },
      },
    },
    MuiChip: {
      styleOverrides: {
        root: {
          borderRadius: 8,
          fontWeight: 500,
        },
      },
    },
    MuiAccordion: {
      styleOverrides: {
        root: {
          boxShadow: 'none',
          border: `1px solid ${modernTokens.colors.neutral[200]}`,
          borderRadius: 8,
          '&:before': {
            display: 'none',
          },
          '&.Mui-expanded': {
            margin: 0,
          },
        },
      },
    },
  },
});

export default theme;