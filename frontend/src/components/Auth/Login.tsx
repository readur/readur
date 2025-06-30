import React, { useState } from 'react';
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
import { api } from '../../services/api';

interface LoginFormData {
  username: string;
  password: string;
}

const Login: React.FC = () => {
  const [showPassword, setShowPassword] = useState<boolean>(false);
  const [error, setError] = useState<string>('');
  const [loading, setLoading] = useState<boolean>(false);
  const [oidcLoading, setOidcLoading] = useState<boolean>(false);
  const { login } = useAuth();
  const navigate = useNavigate();
  const { mode } = useTheme();
  const theme = useMuiTheme();
  
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
      setError('Failed to log in. Please check your credentials.');
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
      setError('Failed to initiate OIDC login. Please try again.');
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
                Welcome to Readur
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
                Your intelligent document management platform
              </Typography>
            </Box>

            {/* Login Card */}
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
                    Sign in to your account
                  </Typography>

                  {error && (
                    <Alert severity="error" sx={{ mb: 3, borderRadius: 2 }}>
                      {error}
                    </Alert>
                  )}

                  <Box component="form" onSubmit={handleSubmit(onSubmit)}>
                    <TextField
                      fullWidth
                      label="Username"
                      margin="normal"
                      {...register('username', {
                        required: 'Username is required',
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
                      label="Password"
                      type={showPassword ? 'text' : 'password'}
                      margin="normal"
                      {...register('password', {
                        required: 'Password is required',
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
                      {loading ? 'Signing in...' : 'Sign in'}
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
                        or
                      </Typography>
                    </Box>

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
                      {oidcLoading ? 'Redirecting...' : 'Sign in with OIDC'}
                    </Button>

                    <Box sx={{ textAlign: 'center', mt: 2 }}>
                    </Box>
                  </Box>
                </CardContent>
              </Card>
            </Grow>

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
                © 2026 Readur. Powered by advanced OCR and AI technology.
              </Typography>
            </Box>
          </Box>
        </Fade>
      </Container>
    </Box>
  );
};

export default Login;