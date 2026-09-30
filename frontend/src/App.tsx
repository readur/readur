import React from 'react';
import { CssBaseline } from '@mui/material';
import { useAuth } from './contexts/AuthContext';
// Legacy MUI theme bridge: the pages that still use MUI read their palette from it until they are replaced.
import { ThemeProvider } from './contexts/ThemeContext';
import { ToastProvider } from './ui';
import { AppRoutes } from './app/routes';
import { AppLoading } from './app/AppLoading';

function App(): React.ReactElement {
  const { loading } = useAuth();

  return (
    <ThemeProvider>
      <CssBaseline />
      <ToastProvider>{loading ? <AppLoading /> : <AppRoutes />}</ToastProvider>
    </ThemeProvider>
  );
}

export default App;
