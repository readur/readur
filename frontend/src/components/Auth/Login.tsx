import React, { useState, useEffect } from 'react';
import {
  Box,
  Card,
  CardContent,
  TextField,
  Button,
  Typography,
  Container,
  Alert,
  InputAdornment,
  IconButton,
  Fade,
  Grow,
  CircularProgress,
} from '@mui/material';
import {
  Visibility,
  VisibilityOff,
  Email as EmailIcon,
  Lock as LockIcon,
  CloudUpload as LogoIcon,
  Security as SecurityIcon,
} from '@mui/icons-material';
import { useForm, SubmitHandler } from 'react-hook-form';
import { useAuth } from '../../contexts/AuthContext';
import { useNavigate } from 'react-router-dom';
import { useTheme } from '../../contexts/ThemeContext';
import { useTheme as useMuiTheme } from '@mui/material/styles';
import { api, ErrorHelper, ErrorCodes } from '../../services/api';
import { useTranslation } from 'react-i18next';

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
  const { mode } = useTheme();
  const theme = useMuiTheme();
  const { t } = useTranslation();

  // Fetch authentication configuration from backend
  useEffect(() => {
    const fetchAuthConfig = async () => {
      try {
        const response = await fetch('/api/auth/config');
        if (response.ok) {
          const config = await response.json();
          setAuthConfig(config);
        } else {
          // Default to allowing both if config fetch fails
          setAuthConfig({ allow_local_auth: true, oidc_enabled: true });
        }
      } catch (err) {
        console.error('Failed to fetch auth config:', err);
        // Default to allowing both if config fetch fails
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
      
      // Handle specific login errors
      if (ErrorHelper.isErrorCode(err, ErrorCodes.USER_INVALID_CREDENTIALS)) {
        setError(t('auth.errors.invalidCredentials'));
      } else if (ErrorHelper.isErrorCode(err, ErrorCodes.USER_ACCOUNT_DISABLED)) {
        setError(t('auth.errors.accountDisabled'));
      } else if (ErrorHelper.isErrorCode(err, ErrorCodes.USER_NOT_FOUND)) {
        setError(t('auth.errors.userNotFound'));
      } else if (ErrorHelper.isErrorCode(err, ErrorCodes.USER_SESSION_EXPIRED) ||
                 ErrorHelper.isErrorCode(err, ErrorCodes.USER_TOKEN_EXPIRED)) {
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

  const handleClickShowPassword = (): void => {
    setShowPassword(!showPassword);
  };

  const handleOidcLogin = async (): Promise<void> => {
    try {
      setError('');
      setOidcLoading(true);
      // Redirect to OIDC login endpoint
      window.location.href = '/api/auth/oidc/login';
    } catch (err) {
      console.error('OIDC login failed:', err);
      
      const errorInfo = ErrorHelper.formatErrorForDisplay(err, true);
      
      // Handle specific OIDC errors
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
        background: mode === 'light' 
          ? 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)'
          : 'linear-gradient(135deg, #1e293b 0%, #334155 50%, #475569 100%)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        p: 2,
      }}
    >
      <Container maxWidth="sm">
        <Fade in={true} timeout={800}>
          <Box>
            {/* Logo and Header */}
            <Box sx={{ textAlign: 'center', mb: 4 }}>
              <Grow in={true} timeout={1000}>
                <Box
                  sx={{
                    width: 80,
                    height: 80,
                    borderRadius: 3,
                    background: 'linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%)',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    color: 'white',
                    fontSize: '2rem',
                    fontWeight: 'bold',
                    mx: 'auto',
                    mb: 3,
                    boxShadow: '0 20px 25px -5px rgb(0 0 0 / 0.1), 0 10px 10px -5px rgb(0 0 0 / 0.04)',
                  }}
                >
                  <LogoIcon fontSize="large" />
                </Box>
              </Grow>
              <Typography
                variant="h3"
                sx={{
                  color: 'white',
                  fontWeight: 700,
                  mb: 1,
                  textShadow: mode === 'light' 
                    ? '0 4px 6px rgba(0, 0, 0, 0.1)'
                    : '0 4px 12px rgba(0, 0, 0, 0.5)',
                }}
              >
                {t('common.welcome', { appName: t('common.appName') })}
              </Typography>
              <Typography
                variant="h6"
                sx={{
                  color: mode === 'light'
                    ? 'rgba(255, 255, 255, 0.8)'
                    : 'rgba(255, 255, 255, 0.9)',
                  fontWeight: 400,
                }}
              >
                {t('auth.intelligentDocumentPlatform')}
              </Typography>
            </Box>

            {/* Loading state while fetching config */}
            {configLoading && (
              <Box sx={{ display: 'flex', justifyContent: 'center', mt: 4 }}>
                <CircularProgress sx={{ color: 'white' }} />
              </Box>
            )}

            {/* Login Card */}
            {!configLoading && authConfig && (
              <Grow in={true} timeout={1200}>
                <Card
                  elevation={0}
                  sx={{
                    borderRadius: 4,
                    backdropFilter: 'blur(20px)',
                    backgroundColor: mode === 'light'
                      ? 'rgba(255, 255, 255, 0.95)'
                      : 'rgba(30, 30, 30, 0.95)',
                    border: mode === 'light'
                      ? '1px solid rgba(255, 255, 255, 0.2)'
                    : '1px solid rgba(255, 255, 255, 0.1)',
                  boxShadow: mode === 'light'
                    ? '0 25px 50px -12px rgba(0, 0, 0, 0.25)'
                    : '0 25px 50px -12px rgba(0, 0, 0, 0.6)',
                }}
              >
                <CardContent sx={{ p: 4 }}>
                  <Typography
                    variant="h5"
                    sx={{
                      textAlign: 'center',
                      mb: 3,
                      fontWeight: 600,
                      color: 'text.primary',
                    }}
                  >
                    {t('auth.signInToAccount')}
                  </Typography>

                  {error && (
                    <Alert severity="error" sx={{ mb: 3, borderRadius: 2 }}>
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
                          {...register('username', {
                            required: t('auth.usernameRequired'),
                          })}
                          error={!!errors.username}
                          helperText={errors.username?.message}
                          InputProps={{
                            startAdornment: (
                              <InputAdornment position="start">
                                <EmailIcon sx={{ color: 'text.secondary' }} />
                              </InputAdornment>
                            ),
                          }}
                          sx={{ mb: 2 }}
                        />

                        <TextField
                          fullWidth
                          label={t('auth.password')}
                          type={showPassword ? 'text' : 'password'}
                          margin="normal"
                          {...register('password', {
                            required: t('auth.passwordRequired'),
                          })}
                          error={!!errors.password}
                          helperText={errors.password?.message}
                          InputProps={{
                            startAdornment: (
                              <InputAdornment position="start">
                                <LockIcon sx={{ color: 'text.secondary' }} />
                              </InputAdornment>
                            ),
                            endAdornment: (
                              <InputAdornment position="end">
                                <IconButton
                                  onClick={handleClickShowPassword}
                                  edge="end"
                                  sx={{ color: 'text.secondary' }}
                                >
                                  {showPassword ? <VisibilityOff /> : <Visibility />}
                                </IconButton>
                              </InputAdornment>
                            ),
                          }}
                          sx={{ mb: 3 }}
                        />

                        <Button
                          type="submit"
                          fullWidth
                          variant="contained"
                          size="large"
                          disabled={loading || oidcLoading}
                          sx={{
                            py: 1.5,
                            mb: 2,
                            background: 'linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%)',
                            borderRadius: 2,
                            fontSize: '1rem',
                            fontWeight: 600,
                            textTransform: 'none',
                            boxShadow: '0 4px 6px -1px rgb(0 0 0 / 0.1)',
                            '&:hover': {
                              background: 'linear-gradient(135deg, #4f46e5 0%, #7c3aed 100%)',
                              boxShadow: '0 10px 15px -3px rgb(0 0 0 / 0.1)',
                            },
                            '&:disabled': {
                              background: 'rgba(0, 0, 0, 0.12)',
                            },
                          }}
                        >
                          {loading ? t('auth.signingIn') : t('auth.signIn')}
                        </Button>

                        <Box
                          sx={{
                            display: 'flex',
                            alignItems: 'center',
                            my: 2,
                            '&::before': {
                              content: '""',
                              flex: 1,
                              height: '1px',
                              backgroundColor: 'divider',
                            },
                            '&::after': {
                              content: '""',
                              flex: 1,
                              height: '1px',
                              backgroundColor: 'divider',
                            },
                          }}
                        >
                          <Typography
                            variant="body2"
                            sx={{
                              px: 2,
                              color: 'text.secondary',
                            }}
                          >
                            {t('common.or')}
                          </Typography>
                        </Box>
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
                          py: 1.5,
                          mb: 2,
                          borderRadius: 2,
                          fontSize: '1rem',
                          fontWeight: 600,
                          textTransform: 'none',
                          borderColor: 'primary.main',
                          color: 'primary.main',
                          '&:hover': {
                            backgroundColor: 'primary.main',
                            color: 'white',
                            borderColor: 'primary.main',
                          },
                          '&:disabled': {
                            borderColor: 'rgba(0, 0, 0, 0.12)',
                            color: 'rgba(0, 0, 0, 0.26)',
                          },
                        }}
                      >
                        {oidcLoading ? t('auth.redirecting') : t('auth.signInWithOIDC')}
                      </Button>
                    )}

                    <Box sx={{ textAlign: 'center', mt: 2 }}>
                    </Box>
                  </Box>
                </CardContent>
              </Card>
            </Grow>
            )}

            {/* Footer */}
            <Box sx={{ textAlign: 'center', mt: 4 }}>
              <Typography
                variant="body2"
                sx={{
                  color: mode === 'light'
                    ? 'rgba(255, 255, 255, 0.7)'
                    : 'rgba(255, 255, 255, 0.8)',
                }}
              >
                {t('common.copyright')}
              </Typography>
            </Box>
          </Box>
        </Fade>
      </Container>
    </Box>
  );
};

export default Login;