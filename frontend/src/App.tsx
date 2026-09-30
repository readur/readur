import React from 'react';
import { useAuth } from './contexts/AuthContext';
import { ToastProvider } from './ui';
import { AppRoutes } from './app/routes';
import { AppLoading } from './app/AppLoading';

function App(): React.ReactElement {
  const { loading } = useAuth();

  return <ToastProvider>{loading ? <AppLoading /> : <AppRoutes />}</ToastProvider>;
}

export default App;
