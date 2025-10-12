import React, { useEffect, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { Box, CircularProgress, Typography, Alert, Container } from '@mui/material';
import { useAuth } from '../../contexts/AuthContext';
import { api, ErrorHelper, ErrorCodes } from '../../services/api';

const OidcCallback: React.FC = () => {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const { login } = useAuth();
  const [error, setError] = useState<string>('');
  const [processing, setProcessing] = useState<boolean>(true);

  useEffect(() => {
    const handleCallback = async () => {
      try {
        const token = searchParams.get('token');
        const error = searchParams.get('error');

        if (error) {
          setError(`Authentication failed: ${error}`);
          setProcessing(false);
          return;
        }

        if (!token) {
          setError('No authentication token received from server');
          setProcessing(false);
          return;
        }

        // Store the token and set up API authorization
        localStorage.setItem('token', token);
        api.defaults.headers.common['Authorization'] = `Bearer ${token}`;

        // Redirect to dashboard - the page will reload and AuthContext will pick up the token
        window.location.href = '/dashboard';
      } catch (err: any) {
        console.error('OIDC callback error:', err);

        const errorInfo = ErrorHelper.formatErrorForDisplay(err, true);

        // Handle specific OIDC callback errors
        if (ErrorHelper.isErrorCode(err, ErrorCodes.USER_OIDC_AUTH_FAILED)) {
          setError('OIDC authentication failed. Please try logging in again or contact your administrator.');
        } else if (ErrorHelper.isErrorCode(err, ErrorCodes.USER_AUTH_PROVIDER_NOT_CONFIGURED)) {
          setError('OIDC is not configured on this server. Please use username/password login.');
        } else if (ErrorHelper.isErrorCode(err, ErrorCodes.USER_INVALID_CREDENTIALS)) {
          setError('Authentication failed. Your OIDC credentials may be invalid or expired.');
        } else if (ErrorHelper.isErrorCode(err, ErrorCodes.USER_ACCOUNT_DISABLED)) {
          setError('Your account has been disabled. Please contact an administrator for assistance.');
        } else if (ErrorHelper.isErrorCode(err, ErrorCodes.USER_SESSION_EXPIRED) ||
                   ErrorHelper.isErrorCode(err, ErrorCodes.USER_TOKEN_EXPIRED)) {
          setError('Authentication session expired. Please try logging in again.');
        } else if (errorInfo.category === 'network') {
          setError('Network error during authentication. Please check your connection and try again.');
        } else if (errorInfo.category === 'server') {
          setError('Server error during authentication. Please try again later or contact support.');
        } else {
          setError(errorInfo.message || 'Failed to complete authentication. Please try again.');
        }

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