import { createTheme } from '@mui/material/styles';
import {
  palette,
  shadows,
  gradients,
  radii,
  fontStack,
} from './design/tokens';

export { modernTokens } from './design/tokens';

/**
 * Glass-morphism helper — kept as a no-op for backwards compatibility with
 * existing call-sites (~43 across the app). The new design system uses
 * solid surfaces with hairline borders, so this returns an empty object.
 * Phase 7 cleanup will remove the remaining call-sites entirely.
 */
export const glassEffect = (_alphaValue: number = 0.1) => ({});

/**
 * Card style helper. Keep the export for back-compat but route through
 * the new design tokens (flatter, smaller hover effect).
 */
export const modernCard = {
  borderRadius: radii.xl,
  boxShadow: shadows.xs,
  border: `1px solid ${palette.neutral[200]}`,
  background: palette.neutral[0],
  transition: 'box-shadow var(--dur-base, 180ms) var(--ease-out, ease-out)',
  '&:hover': {
    boxShadow: shadows.sm,
  },
};

/**
 * Default MUI theme. Note: the runtime ThemeProvider in
 * contexts/ThemeContext.tsx creates a theme per mode (light/dark) and is the
 * authoritative source for live styling. This default export is kept for
 * any test setup or storybook that imports it directly.
 */
const theme = createTheme({
  palette: {
    primary: {
      main: palette.primary[500],
      light: palette.primary[300],
      dark: palette.primary[700],
      50: palette.primary[50],
      100: palette.primary[100],
      200: palette.primary[200],
      300: palette.primary[300],
      400: palette.primary[400],
      500: palette.primary[500],
      600: palette.primary[600],
      700: palette.primary[700],
      800: palette.primary[800],
      900: palette.primary[900],
    },
    secondary: {
      main: palette.secondary[500],
      light: palette.secondary[300],
      dark: palette.secondary[700],
    },
    background: {
      default: palette.neutral[50],
      paper: palette.neutral[0],
    },
    text: {
      primary: palette.neutral[900],
      secondary: palette.neutral[600],
    },
    divider: palette.neutral[200],
    success: {
      main: palette.success[500],
      light: palette.success[50],
      dark: palette.success[600],
    },
    warning: {
      main: palette.warning[500],
      light: palette.warning[50],
      dark: palette.warning[600],
    },
    error: {
      main: palette.error[500],
      light: palette.error[50],
      dark: palette.error[600],
    },
    info: {
      main: palette.info[500],
      light: palette.info[50],
      dark: palette.info[600],
    },
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
  shape: { borderRadius: 12 },
  components: {
    MuiButton: {
      styleOverrides: {
        root: {
          textTransform: 'none',
          borderRadius: radii.md,
          fontWeight: 500,
          boxShadow: 'none',
          '&:hover': { boxShadow: shadows.sm },
        },
        contained: {
          background: gradients.accent,
          '&:hover': { background: gradients.accentHover },
        },
      },
    },
    MuiCard: {
      styleOverrides: { root: modernCard },
    },
    MuiPaper: {
      styleOverrides: {
        root: {
          borderRadius: radii.lg,
          boxShadow: shadows.xs,
        },
      },
    },
    MuiChip: {
      styleOverrides: {
        root: { borderRadius: radii.sm, fontWeight: 500 },
      },
    },
    MuiAccordion: {
      styleOverrides: {
        root: {
          boxShadow: 'none',
          border: `1px solid ${palette.neutral[200]}`,
          borderRadius: radii.md,
          '&:before': { display: 'none' },
          '&.Mui-expanded': { margin: 0 },
        },
      },
    },
  },
});

export default theme;
