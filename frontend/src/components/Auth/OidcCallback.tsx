import React, { useEffect, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { Box, CircularProgress, Typography, Alert, Container } from '@mui/material';
import { useAuth } from '../../contexts/AuthContext';
import { api } from '../../services/api';

const OidcCallback: React.FC = () => {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const { login } = useAuth();
  const [error, setError] = useState<string>('');
  const [processing, setProcessing] = useState<boolean>(true);

  useEffect(() => {
    const handleCallback = async () => {
      try {
        const code = searchParams.get('code');
        const error = searchParams.get('error');
        const state = searchParams.get('state');

        if (error) {
          setError(`Authentication failed: ${error}`);
          setProcessing(false);
          return;
        }

        if (!code) {
          setError('No authorization code received');
          setProcessing(false);
          return;
        }

        // Call the backend OIDC callback endpoint
        const response = await api.get(`/auth/oidc/callback?code=${code}&state=${state || ''}`);
        
        if (response.data && response.data.token) {
          // Store the token and user data
          localStorage.setItem('token', response.data.token);
          api.defaults.headers.common['Authorization'] = `Bearer ${response.data.token}`;
          
          // Redirect to dashboard - the auth context will pick up the token on next page load
          window.location.href = '/dashboard';
        } else {
          setError('Invalid response from authentication server');
          setProcessing(false);
        }
      } catch (err: any) {
        console.error('OIDC callback error:', err);
        setError(
          err.response?.data?.error || 
          err.message || 
          'Failed to complete authentication'
        );
        setProcessing(false);
      }
    };

    handleCallback();
  }, [searchParams, navigate, login]);

  const handleReturnToLogin = () => {
    navigate('/login');
  };

  return (
    <Box
      sx={{
        minHeight: '100vh',
        background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        p: 2,
      }}
    >
      <Container maxWidth="sm">
        <Box
          sx={{
            backgroundColor: 'rgba(255, 255, 255, 0.95)',
            borderRadius: 4,
            p: 4,
            textAlign: 'center',
            boxShadow: '0 25px 50px -12px rgba(0, 0, 0, 0.25)',
          }}
        >
          {processing ? (
            <>
              <CircularProgress size={60} sx={{ mb: 3, color: 'primary.main' }} />
              <Typography variant="h5" sx={{ mb: 2, fontWeight: 600 }}>
                Completing Authentication
              </Typography>
              <Typography variant="body1" color="text.secondary">
                Please wait while we process your authentication...
              </Typography>
            </>
          ) : (
            <>
              <Alert 
                severity="error" 
                sx={{ mb: 3, textAlign: 'left' }}
                action={
                  <Box
                    component="button"
                    onClick={handleReturnToLogin}
                    sx={{
                      background: 'none',
                      border: 'none',
                      color: 'primary.main',
                      cursor: 'pointer',
                      textDecoration: 'underline',
                      fontSize: '0.875rem',
                    }}
                  >
                    Return to Login
                  </Box>
                }
              >
                <Typography variant="h6" sx={{ mb: 1 }}>
                  Authentication Error
                </Typography>
                {error}
              </Alert>
            </>
          )}
        </Box>
      </Container>
    </Box>
  );
};

export default OidcCallback;