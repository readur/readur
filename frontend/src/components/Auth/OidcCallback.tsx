import React, { useEffect, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Box, CircularProgress, Typography, Alert, Container } from '@mui/material';
import { useAuth, LoginResponse } from '../../contexts/AuthContext';
import { api, ErrorHelper } from '../../services/api';

/**
 * Error codes the server may put in the callback URL. Only these are
 * recognised; any other value (including free text) is shown as a generic
 * failure and never rendered.
 */
export const OIDC_CALLBACK_ERROR_CODES = [
  'provider_error',
  'invalid_state',
  'auth_failed',
  'no_account',
  'account_disabled',
  'server_error',
] as const;

type OidcCallbackErrorCode = (typeof OIDC_CALLBACK_ERROR_CODES)[number];

const isKnownErrorCode = (value: string): value is OidcCallbackErrorCode =>
  (OIDC_CALLBACK_ERROR_CODES as readonly string[]).includes(value);

/**
 * Read the one-time handoff code (or an error code) from the URL fragment
 * (`/auth/callback#code=...` / `#error=...`). Query parameters are also
 * checked for `error` so provider-side failures still display.
 */
const readCallbackParams = (): { code: string | null; error: string | null } => {
  const hash = window.location.hash.startsWith('#')
    ? window.location.hash.slice(1)
    : window.location.hash;
  const fragment = new URLSearchParams(hash);
  const query = new URLSearchParams(window.location.search);
  return {
    code: fragment.get('code'),
    error: fragment.get('error') ?? query.get('error'),
  };
};

/** Remove the code from the address bar and browser history immediately. */
const stripCallbackParams = (): void => {
  if (window.location.hash || window.location.search) {
    window.history.replaceState(window.history.state, '', window.location.pathname);
  }
};

const OidcCallback: React.FC = () => {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const { completeLogin } = useAuth();
  const [error, setError] = useState<string>('');
  const [processing, setProcessing] = useState<boolean>(true);
  // The code is single-use; make sure it is only redeemed once even if the
  // effect runs twice (React StrictMode).
  const startedRef = useRef(false);

  useEffect(() => {
    if (startedRef.current) return;
    startedRef.current = true;

    const handleCallback = async () => {
      const { code, error: callbackError } = readCallbackParams();
      stripCallbackParams();

      try {
        if (callbackError) {
          const key = isKnownErrorCode(callbackError) ? callbackError : 'unknown';
          setError(t(`auth.oidcCallback.errors.${key}`));
          setProcessing(false);
          return;
        }

        if (!code) {
          setError(t('auth.oidcCallback.errors.missingCode'));
          setProcessing(false);
          return;
        }

        const response = await api.post<LoginResponse>('/auth/oidc/exchange', { code });
        completeLogin(response.data);
        navigate('/dashboard', { replace: true });
      } catch (err: any) {
        console.error('OIDC callback error:', err);

        const errorInfo = ErrorHelper.formatErrorForDisplay(err, true);
        const status = err?.response?.status;

        // Only fixed messages are shown; server-provided text is not rendered.
        if (status === 401) {
          setError(t('auth.oidcCallback.errors.invalidCode'));
        } else if (status === 429) {
          setError(t('auth.oidcCallback.errors.tooManyAttempts'));
        } else if (errorInfo.category === 'network') {
          setError(t('auth.errors.networkError'));
        } else if (errorInfo.category === 'server') {
          setError(t('auth.oidcCallback.errors.server_error'));
        } else {
          setError(t('auth.oidcCallback.errors.unknown'));
        }

        setProcessing(false);
      }
    };

    handleCallback();
  }, []);

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