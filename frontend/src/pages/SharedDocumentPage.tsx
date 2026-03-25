import React, { useState, useEffect } from 'react';
import { useParams } from 'react-router-dom';
import {
  Box,
  Typography,
  TextField,
  Button,
  Paper,
  CircularProgress,
  Alert,
  CssBaseline,
  InputAdornment,
  IconButton,
  Divider,
} from '@mui/material';
import { ThemeProvider, createTheme } from '@mui/material/styles';
import {
  Download as DownloadIcon,
  Visibility as ViewIcon,
  Lock as LockIcon,
  VisibilityOff as VisibilityOffIcon,
  InsertDriveFile as FileIcon,
} from '@mui/icons-material';
import { sharedLinksPublicService, type SharedDocumentMetadata } from '../services/api';
import axios from 'axios';

const theme = createTheme({
  palette: {
    mode: 'light',
    primary: { main: '#6366f1' },
  },
  typography: {
    fontFamily: '"Inter", "Roboto", "Helvetica", "Arial", sans-serif',
  },
  shape: { borderRadius: 12 },
});

function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  const k = 1024;
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${units[i]}`;
}

type PageState = 'loading' | 'password' | 'ready' | 'error';

const SharedDocumentPage: React.FC = () => {
  const { token } = useParams<{ token: string }>();

  const [state, setState] = useState<PageState>('loading');
  const [metadata, setMetadata] = useState<SharedDocumentMetadata | null>(null);
  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [verifiedPassword, setVerifiedPassword] = useState<string | undefined>(undefined);
  const [verifying, setVerifying] = useState(false);
  const [errorMessage, setErrorMessage] = useState('');
  const [passwordError, setPasswordError] = useState('');
  const [downloading, setDownloading] = useState(false);

  const handleFileAction = async (action: 'download' | 'view') => {
    if (!token) return;
    setDownloading(true);
    try {
      const response = action === 'download'
        ? await sharedLinksPublicService.downloadDocument(token, verifiedPassword)
        : await sharedLinksPublicService.viewDocument(token, verifiedPassword);

      const blob = new Blob([response.data]);
      const contentDisposition = response.headers['content-disposition'] || '';
      const filenameMatch = contentDisposition.match(/filename="?([^";\n]+)"?/);
      const filename = filenameMatch?.[1] || metadata?.original_filename || 'download';

      if (action === 'view') {
        // Open in new tab
        const contentType = response.headers['content-type'] || 'application/octet-stream';
        const viewBlob = new Blob([response.data], { type: contentType });
        const url = URL.createObjectURL(viewBlob);
        window.open(url, '_blank');
        setTimeout(() => URL.revokeObjectURL(url), 60000);
      } else {
        // Trigger download
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = filename;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
      }
    } catch (err) {
      if (axios.isAxiosError(err) && err.response?.status === 410) {
        setErrorMessage('This shared link has expired or reached its view limit.');
        setState('error');
      } else {
        setErrorMessage('Failed to access the document. Please try again.');
      }
    } finally {
      setDownloading(false);
    }
  };

  useEffect(() => {
    if (!token) {
      setErrorMessage('Invalid shared link.');
      setState('error');
      return;
    }

    const fetchMetadata = async () => {
      try {
        const response = await sharedLinksPublicService.getMetadata(token);
        setMetadata(response.data);

        if (response.data.requires_password) {
          setState('password');
        } else {
          setState('ready');
        }
      } catch (err) {
        if (axios.isAxiosError(err)) {
          const status = err.response?.status;
          if (status === 404) {
            setErrorMessage('This shared link does not exist or has been removed.');
          } else if (status === 410) {
            setErrorMessage('This shared link has expired or been revoked.');
          } else if (err.response?.data?.error) {
            setErrorMessage(err.response.data.error);
          } else {
            setErrorMessage('Unable to load the shared document. Please try again later.');
          }
        } else {
          setErrorMessage('An unexpected error occurred.');
        }
        setState('error');
      }
    };

    fetchMetadata();
  }, [token]);

  const handlePasswordSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!token || !password.trim()) return;

    setVerifying(true);
    setPasswordError('');

    try {
      const response = await sharedLinksPublicService.verifyPassword(token, password.trim());
      if (response.data.valid) {
        setVerifiedPassword(password.trim());
        setState('ready');
      } else {
        setPasswordError('Incorrect password. Please try again.');
      }
    } catch (err) {
      if (axios.isAxiosError(err) && err.response?.data?.error) {
        setPasswordError(err.response.data.error);
      } else {
        setPasswordError('Verification failed. Please try again.');
      }
    } finally {
      setVerifying(false);
    }
  };

  const renderContent = () => {
    if (state === 'loading') {
      return (
        <Box sx={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 2 }}>
          <CircularProgress size={32} />
          <Typography variant="body2" color="text.secondary">
            Loading shared document...
          </Typography>
        </Box>
      );
    }

    if (state === 'error') {
      return (
        <Alert severity="error" variant="outlined" sx={{ maxWidth: 480, width: '100%' }}>
          {errorMessage}
        </Alert>
      );
    }

    if (state === 'password') {
      return (
        <Paper
          component="form"
          onSubmit={handlePasswordSubmit}
          elevation={0}
          sx={{
            p: 4,
            maxWidth: 400,
            width: '100%',
            border: '1px solid',
            borderColor: 'divider',
          }}
        >
          <Box sx={{ display: 'flex', flexDirection: 'column', alignItems: 'center', mb: 3 }}>
            <LockIcon sx={{ fontSize: 40, color: 'text.secondary', mb: 1.5 }} />
            <Typography variant="h6" gutterBottom>
              Password Required
            </Typography>
            <Typography variant="body2" color="text.secondary" textAlign="center">
              This document is password-protected. Enter the password to continue.
            </Typography>
          </Box>

          {passwordError && (
            <Alert severity="error" sx={{ mb: 2 }}>
              {passwordError}
            </Alert>
          )}

          <TextField
            label="Password"
            type={showPassword ? 'text' : 'password'}
            value={password}
            onChange={(e) => {
              setPassword(e.target.value);
              if (passwordError) setPasswordError('');
            }}
            fullWidth
            autoFocus
            disabled={verifying}
            error={!!passwordError}
            sx={{ mb: 2 }}
            slotProps={{
              input: {
                endAdornment: (
                  <InputAdornment position="end">
                    <IconButton
                      onClick={() => setShowPassword(!showPassword)}
                      edge="end"
                      aria-label={showPassword ? 'Hide password' : 'Show password'}
                    >
                      {showPassword ? <VisibilityOffIcon /> : <ViewIcon />}
                    </IconButton>
                  </InputAdornment>
                ),
              },
            }}
          />

          <Button
            type="submit"
            variant="contained"
            fullWidth
            disabled={verifying || !password.trim()}
            startIcon={verifying ? <CircularProgress size={16} /> : undefined}
          >
            {verifying ? 'Verifying...' : 'Continue'}
          </Button>
        </Paper>
      );
    }

    // state === 'ready'
    if (!metadata || !token) return null;

    return (
      <Paper
        elevation={0}
        sx={{
          p: 4,
          maxWidth: 480,
          width: '100%',
          border: '1px solid',
          borderColor: 'divider',
        }}
      >
        <Box sx={{ display: 'flex', flexDirection: 'column', alignItems: 'center', mb: 3 }}>
          <FileIcon sx={{ fontSize: 48, color: 'primary.main', mb: 1.5, opacity: 0.8 }} />
          <Typography variant="h6" sx={{ wordBreak: 'break-word', textAlign: 'center' }}>
            {metadata.original_filename}
          </Typography>
        </Box>

        <Divider sx={{ mb: 2 }} />

        <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1, mb: 3 }}>
          <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
            <Typography variant="body2" color="text.secondary">Type</Typography>
            <Typography variant="body2">{metadata.mime_type}</Typography>
          </Box>
          <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
            <Typography variant="body2" color="text.secondary">Size</Typography>
            <Typography variant="body2">{formatFileSize(metadata.file_size)}</Typography>
          </Box>
          <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
            <Typography variant="body2" color="text.secondary">Shared</Typography>
            <Typography variant="body2">
              {new Date(metadata.created_at).toLocaleDateString(undefined, {
                year: 'numeric',
                month: 'short',
                day: 'numeric',
              })}
            </Typography>
          </Box>
        </Box>

        <Box sx={{ display: 'flex', gap: 1.5 }}>
          <Button
            variant="outlined"
            fullWidth
            startIcon={<ViewIcon />}
            onClick={() => handleFileAction('view')}
            disabled={downloading}
          >
            View
          </Button>
          <Button
            variant="contained"
            fullWidth
            startIcon={downloading ? <CircularProgress size={20} color="inherit" /> : <DownloadIcon />}
            onClick={() => handleFileAction('download')}
            disabled={downloading}
          >
            Download
          </Button>
        </Box>
      </Paper>
    );
  };

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <Box
        sx={{
          minHeight: '100vh',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          px: 2,
          py: 4,
          bgcolor: 'grey.50',
        }}
      >
        <Typography
          variant="h5"
          sx={{
            mb: 4,
            fontWeight: 700,
            color: 'primary.main',
            letterSpacing: '-0.02em',
          }}
        >
          Readur
        </Typography>

        {renderContent()}

        <Typography variant="caption" color="text.disabled" sx={{ mt: 4 }}>
          Shared securely via Readur
        </Typography>
      </Box>
    </ThemeProvider>
  );
};

export default SharedDocumentPage;
