import React, { useState, useEffect } from 'react';
import {
  Box,
  TextField,
  Button,
  Typography,
  Alert,
  InputAdornment,
  IconButton,
  CircularProgress,
} from '@mui/material';
import {
  Visibility,
  VisibilityOff,
  Person as PersonIcon,
  Lock as LockIcon,
  Security as SecurityIcon,
} from '../../design/icons';
import { useForm, SubmitHandler } from 'react-hook-form';
import { useAuth } from '../../contexts/AuthContext';
import { useNavigate } from 'react-router-dom';
import { api, ErrorHelper, ErrorCodes } from '../../services/api';
import { useTranslation } from 'react-i18next';
import { Panel } from '../../design/components';

interface LoginFormData {
  username: string;
  password: string;
}

interface AuthConfig {
  allow_local_auth: boolean;
  oidc_enabled: boolean;
}

const Login: React.FC = () => {
  const [authConfig, setAuthConfig] = useState<AuthConfig | null>(null);
  const [configLoading, setConfigLoading] = useState<boolean>(true);
  const [showPassword, setShowPassword] = useState<boolean>(false);
  const [error, setError] = useState<string>('');
  const [loading, setLoading] = useState<boolean>(false);
  const [oidcLoading, setOidcLoading] = useState<boolean>(false);
  const { login } = useAuth();
  const navigate = useNavigate();
  const { t } = useTranslation();

  useEffect(() => {
    const fetchAuthConfig = async () => {
      try {
        const response = await fetch('/api/auth/config');
        if (response.ok) {
          setAuthConfig(await response.json());
        } else {
          setAuthConfig({ allow_local_auth: true, oidc_enabled: true });
        }
      } catch (err) {
        console.error('Failed to fetch auth config:', err);
        setAuthConfig({ allow_local_auth: true, oidc_enabled: true });
      } finally {
        setConfigLoading(false);
      }
    };
    fetchAuthConfig();
  }, []);

  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<LoginFormData>();

  const onSubmit: SubmitHandler<LoginFormData> = async (data) => {
    try {
      setError('');
      setLoading(true);
      await login(data.username, data.password);
      navigate('/dashboard');
    } catch (err) {
      console.error('Login failed:', err);
      const errorInfo = ErrorHelper.formatErrorForDisplay(err, true);
      if (ErrorHelper.isErrorCode(err, ErrorCodes.USER_INVALID_CREDENTIALS)) {
        setError(t('auth.errors.invalidCredentials'));
      } else if (ErrorHelper.isErrorCode(err, ErrorCodes.USER_ACCOUNT_DISABLED)) {
        setError(t('auth.errors.accountDisabled'));
      } else if (ErrorHelper.isErrorCode(err, ErrorCodes.USER_NOT_FOUND)) {
        setError(t('auth.errors.userNotFound'));
      } else if (
        ErrorHelper.isErrorCode(err, ErrorCodes.USER_SESSION_EXPIRED) ||
        ErrorHelper.isErrorCode(err, ErrorCodes.USER_TOKEN_EXPIRED)
      ) {
        setError(t('auth.errors.sessionExpired'));
      } else if (errorInfo.category === 'network') {
        setError(t('auth.errors.networkError'));
      } else if (errorInfo.category === 'server') {
        setError(t('auth.errors.serverError'));
      } else {
        setError(errorInfo.message || t('auth.errors.loginFailed'));
      }
    } finally {
      setLoading(false);
    }
  };

  const handleClickShowPassword = (): void => setShowPassword(!showPassword);

  const handleOidcLogin = async (): Promise<void> => {
    try {
      setError('');
      setOidcLoading(true);
      window.location.href = '/api/auth/oidc/login';
    } catch (err) {
      console.error('OIDC login failed:', err);
      const errorInfo = ErrorHelper.formatErrorForDisplay(err, true);
      if (ErrorHelper.isErrorCode(err, ErrorCodes.USER_OIDC_AUTH_FAILED)) {
        setError(t('auth.errors.oidcAuthFailed'));
      } else if (ErrorHelper.isErrorCode(err, ErrorCodes.USER_AUTH_PROVIDER_NOT_CONFIGURED)) {
        setError(t('auth.errors.oidcNotConfigured'));
      } else if (errorInfo.category === 'network') {
        setError(t('auth.errors.networkError'));
      } else {
        setError(errorInfo.message || t('auth.errors.oidcInitFailed'));
      }
      setOidcLoading(false);
    }
  };

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
        {/* Brand + headline */}
        <Box sx={{ textAlign: 'left', mb: 'var(--s-6)' }}>
          <Box
            sx={{
              display: 'flex',
              alignItems: 'center',
              gap: 'var(--s-3)',
              mb: 'var(--s-5)',
            }}
          >
            <Box
              sx={{
                width: 40,
                height: 40,
                borderRadius: 'var(--r-2)',
                background: 'var(--accent-grad)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                boxShadow: 'var(--shadow-sm)',
                overflow: 'hidden',
              }}
            >
              <Box
                component="img"
                src="/readur-32.png"
                srcSet="/readur-32.png 1x, /readur-64.png 2x"
                alt={t('common.appName')}
                sx={{ width: 26, height: 26, objectFit: 'contain' }}
              />
            </Box>
            <Box>
              <Box
                sx={{
                  fontFamily: 'var(--font-sans)',
                  fontWeight: 800,
                  fontSize: 18,
                  lineHeight: 1,
                  color: 'var(--fg-0)',
                  letterSpacing: '-0.025em',
                }}
              >
                {t('common.appName')}
              </Box>
              <Box
                sx={{
                  fontFamily: 'var(--font-mono)',
                  fontWeight: 600,
                  fontSize: 10,
                  color: 'var(--accent-60)',
                  letterSpacing: '0.1em',
                  marginTop: '4px',
                  textTransform: 'uppercase',
                }}
              >
                v2.9 · sign in
              </Box>
            </Box>
          </Box>
          <Typography
            sx={{
              fontFamily: 'var(--font-sans)',
              fontWeight: 800,
              fontSize: 'var(--fs-display-md)',
              lineHeight: 'var(--lh-tight)',
              letterSpacing: 'var(--tracking-display)',
              color: 'var(--fg-0)',
              margin: 0,
            }}
          >
            {t('auth.signInToAccount')}
          </Typography>
        </Box>

        {configLoading && (
          <Box sx={{ display: 'flex', justifyContent: 'center', mt: 'var(--s-6)' }}>
            <CircularProgress size={28} sx={{ color: 'var(--accent-60)' }} />
          </Box>
        )}

        {!configLoading && authConfig && (
          <Panel>
            {error && (
              <Alert severity="error" sx={{ mb: 'var(--s-4)', borderRadius: 'var(--r-2)' }}>
                {error}
              </Alert>
            )}

            <Box component="form" onSubmit={handleSubmit(onSubmit)}>
              {authConfig.allow_local_auth && (
                <>
                  <TextField
                    fullWidth
                    label={t('auth.username')}
                    margin="normal"
                    {...register('username', { required: t('auth.usernameRequired') })}
                    error={!!errors.username}
                    helperText={errors.username?.message}
                    InputProps={{
                      startAdornment: (
                        <InputAdornment position="start">
                          <PersonIcon sx={{ color: 'var(--fg-3)', fontSize: 18 }} />
                        </InputAdornment>
                      ),
                    }}
                    sx={{ mb: 'var(--s-3)' }}
                  />

                  <TextField
                    fullWidth
                    label={t('auth.password')}
                    type={showPassword ? 'text' : 'password'}
                    margin="normal"
                    {...register('password', { required: t('auth.passwordRequired') })}
                    error={!!errors.password}
                    helperText={errors.password?.message}
                    InputProps={{
                      startAdornment: (
                        <InputAdornment position="start">
                          <LockIcon sx={{ color: 'var(--fg-3)', fontSize: 18 }} />
                        </InputAdornment>
                      ),
                      endAdornment: (
                        <InputAdornment position="end">
                          <IconButton
                            onClick={handleClickShowPassword}
                            edge="end"
                            sx={{ color: 'var(--fg-3)' }}
                            aria-label="toggle password visibility"
                          >
                            {showPassword ? <VisibilityOff fontSize="small" /> : <Visibility fontSize="small" />}
                          </IconButton>
                        </InputAdornment>
                      ),
                    }}
                    sx={{ mb: 'var(--s-5)' }}
                  />

                  <Button
                    type="submit"
                    fullWidth
                    variant="contained"
                    size="large"
                    disabled={loading || oidcLoading}
                    sx={{
                      py: 1.25,
                      fontWeight: 600,
                      fontSize: '0.95rem',
                    }}
                  >
                    {loading ? t('auth.signingIn') : t('auth.signIn')}
                  </Button>

                  {authConfig.oidc_enabled && (
                    <Box
                      sx={{
                        display: 'flex',
                        alignItems: 'center',
                        my: 'var(--s-5)',
                        gap: 'var(--s-3)',
                        '&::before, &::after': {
                          content: '""',
                          flex: 1,
                          height: '1px',
                          backgroundColor: 'var(--line-1)',
                        },
                      }}
                    >
                      <Box
                        component="span"
                        className="rd-label"
                        sx={{ flexShrink: 0 }}
                      >
                        {t('common.or')}
                      </Box>
                    </Box>
                  )}
                </>
              )}

              {authConfig.oidc_enabled && (
                <Button
                  fullWidth
                  variant="outlined"
                  size="large"
                  disabled={loading || oidcLoading}
                  onClick={handleOidcLogin}
                  startIcon={<SecurityIcon />}
                  sx={{
                    py: 1.25,
                    fontWeight: 600,
                    fontSize: '0.95rem',
                    borderColor: 'var(--line-2)',
                    color: 'var(--fg-0)',
                    '&:hover': {
                      background: 'var(--bg-2)',
                      borderColor: 'var(--accent-50)',
                    },
                  }}
                >
                  {oidcLoading ? t('auth.redirecting') : t('auth.signInWithOIDC')}
                </Button>
              )}
            </Box>
          </Panel>
        )}

        <Box sx={{ textAlign: 'center', mt: 'var(--s-6)' }}>
          <Typography
            sx={{
              fontFamily: 'var(--font-mono)',
              fontSize: 'var(--fs-micro)',
              color: 'var(--fg-3)',
              letterSpacing: 'var(--tracking-caps)',
              textTransform: 'uppercase',
            }}
          >
            {t('common.copyright')}
          </Typography>
        </Box>
      </Box>
    </Box>
  );
};

export default Login;
