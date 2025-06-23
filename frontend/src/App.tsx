import React from 'react';
import { Routes, Route, Navigate } from 'react-router-dom';
import { CssBaseline } from '@mui/material';
import { useAuth } from './contexts/AuthContext';
import { ThemeProvider } from './contexts/ThemeContext';
import { NotificationProvider } from './contexts/NotificationContext';
import Login from './components/Auth/Login';
import AppLayout from './components/Layout/AppLayout';
import Dashboard from './components/Dashboard/Dashboard';
import UploadPage from './pages/UploadPage';
import DocumentsPage from './pages/DocumentsPage';
import SearchPage from './pages/SearchPage';
import DocumentDetailsPage from './pages/DocumentDetailsPage';
import SettingsPage from './pages/SettingsPage';
import SourcesPage from './pages/SourcesPage';
import WatchFolderPage from './pages/WatchFolderPage';
import FailedOcrPage from './pages/FailedOcrPage';
import LabelsPage from './pages/LabelsPage';

function App(): React.ReactElement {
  const { user, loading } = useAuth();

  if (loading) {
    return (
      <ThemeProvider>
        <CssBaseline />
        <div style={{ 
          minHeight: '100vh', 
          display: 'flex', 
          alignItems: 'center', 
          justifyContent: 'center',
          background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
        }}>
          <div style={{
            width: '40px',
            height: '40px',
            border: '4px solid rgba(255, 255, 255, 0.3)',
            borderTop: '4px solid white',
            borderRadius: '50%',
            animation: 'spin 1s linear infinite',
          }} />
          <style>{`
            @keyframes spin {
              0% { transform: rotate(0deg); }
              100% { transform: rotate(360deg); }
            }
          `}</style>
        </div>
      </ThemeProvider>
    );
  }

  return (
    <ThemeProvider>
      <CssBaseline />
      <Routes>
        <Route path="/login" element={!user ? <Login /> : <Navigate to="/dashboard" />} />
        <Route
          path="/*"
          element={
            user ? (
              <NotificationProvider>
                <AppLayout>
                  <Routes>
                    <Route path="/" element={<Navigate to="/dashboard" />} />
                    <Route path="/dashboard" element={<Dashboard />} />
                    <Route path="/upload" element={<UploadPage />} />
                    <Route path="/documents" element={<DocumentsPage />} />
                    <Route path="/documents/:id" element={<DocumentDetailsPage />} />
                    <Route path="/search" element={<SearchPage />} />
                    <Route path="/labels" element={<LabelsPage />} />
                    <Route path="/sources" element={<SourcesPage />} />
                    <Route path="/watch" element={<WatchFolderPage />} />
                    <Route path="/settings" element={<SettingsPage />} />
                    <Route path="/failed-ocr" element={<FailedOcrPage />} />
                    <Route path="/profile" element={<div>Profile Page - Coming Soon</div>} />
                  </Routes>
                </AppLayout>
              </NotificationProvider>
            ) : (
              <Navigate to="/login" />
            )
          }
        />
      </Routes>
    </ThemeProvider>
  );
}

export default App;