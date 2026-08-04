import React, { useEffect, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { Box, CircularProgress, Typography, Alert } from '@mui/material';
import { useAuth } from '../../contexts/AuthContext';
import { api, ErrorHelper, ErrorCodes } from '../../services/api';
import { Panel } from '../../design/components';

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
        const errorParam = searchParams.get('error');

        if (errorParam) {
          setError(`Authentication failed: ${errorParam}`);
          setProcessing(false);
          return;
        }

        if (!token) {
          setError('No authentication token received from server');
          setProcessing(false);
          return;
        }

        localStorage.setItem('token', token);
        api.defaults.headers.common['Authorization'] = `Bearer ${token}`;
        window.location.href = '/dashboard';
      } catch (err: any) {
        console.error('OIDC callback error:', err);
        const errorInfo = ErrorHelper.formatErrorForDisplay(err, true);

        if (ErrorHelper.isErrorCode(err, ErrorCodes.USER_OIDC_AUTH_FAILED)) {
          setError('OIDC authentication failed. Please try logging in again or contact your administrator.');
        } else if (ErrorHelper.isErrorCode(err, ErrorCodes.USER_AUTH_PROVIDER_NOT_CONFIGURED)) {
          setError('OIDC is not configured on this server. Please use username/password login.');
        } else if (ErrorHelper.isErrorCode(err, ErrorCodes.USER_INVALID_CREDENTIALS)) {
          setError('Authentication failed. Your OIDC credentials may be invalid or expired.');
        } else if (ErrorHelper.isErrorCode(err, ErrorCodes.USER_ACCOUNT_DISABLED)) {
          setError('Your account has been disabled. Please contact an administrator for assistance.');
        } else if (
          ErrorHelper.isErrorCode(err, ErrorCodes.USER_SESSION_EXPIRED) ||
          ErrorHelper.isErrorCode(err, ErrorCodes.USER_TOKEN_EXPIRED)
        ) {
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

  const handleReturnToLogin = () => navigate('/login');

  return (
    <Box
      sx={{
        minHeight: '100vh',
        background: 'var(--bg-0)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        padding: 'var(--s-4)',
      }}
    >
      <Box sx={{ width: '100%', maxWidth: 420 }}>
        <Panel>
          {processing ? (
            <Box sx={{ textAlign: 'center', py: 'var(--s-4)' }}>
              <CircularProgress size={32} sx={{ mb: 'var(--s-4)', color: 'var(--accent-60)' }} />
              <Typography
                sx={{
                  fontFamily: 'var(--font-sans)',
                  fontWeight: 700,
                  fontSize: 'var(--fs-h2)',
                  color: 'var(--fg-0)',
                  letterSpacing: '-0.01em',
                  mb: 'var(--s-2)',
                }}
              >
                Completing authentication
              </Typography>
              <Typography
                sx={{
                  fontFamily: 'var(--font-sans)',
                  fontSize: 'var(--fs-body)',
                  color: 'var(--fg-2)',
                  lineHeight: 'var(--lh-body)',
                }}
              >
                Please wait while we process your sign-in.
              </Typography>
            </Box>
          ) : (
            <Alert
              severity="error"
              sx={{ textAlign: 'left' }}
              action={
                <Box
                  component="button"
                  onClick={handleReturnToLogin}
                  sx={{
                    background: 'none',
                    border: 'none',
                    color: 'var(--accent-60)',
                    cursor: 'pointer',
                    textDecoration: 'underline',
                    fontSize: '0.875rem',
                    fontFamily: 'var(--font-sans)',
                  }}
                >
                  Return to sign in
                </Box>
              }
            >
              <Typography sx={{ mb: 'var(--s-1)', fontWeight: 600 }}>
                Authentication error
              </Typography>
              {error}
            </Alert>
          )}
        </Panel>
      </Box>
    </Box>
  );
};

export default OidcCallback;
