import React, { useState, useEffect } from 'react'
import { Link as RouterLink, Navigate, useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import {
  Alert,
  Box,
  Button,
  Card,
  CardContent,
  CircularProgress,
  Container,
  Link,
  TextField,
  Typography,
} from '@mui/material'
import { useAuth } from '../contexts/AuthContext'

interface AuthConfig {
  allow_local_auth: boolean
  allow_registration?: boolean
  oidc_enabled: boolean
}

const MIN_PASSWORD_LENGTH = 8

function Register() {
  const navigate = useNavigate()
  const { t } = useTranslation()
  const [username, setUsername] = useState('')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)
  const [configLoading, setConfigLoading] = useState(true)
  const [registrationAllowed, setRegistrationAllowed] = useState(false)
  const [pendingApproval, setPendingApproval] = useState(false)
  const { register } = useAuth()

  // Registration is only available when the server explicitly enables it.
  useEffect(() => {
    const fetchAuthConfig = async () => {
      try {
        const response = await fetch('/api/auth/config')
        if (response.ok) {
          const config: AuthConfig = await response.json()
          setRegistrationAllowed(config.allow_local_auth && config.allow_registration === true)
        }
      } catch (err) {
        console.error('Failed to fetch auth config:', err)
      } finally {
        setConfigLoading(false)
      }
    }

    fetchAuthConfig()
  }, [])

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setError('')

    if (password.length < MIN_PASSWORD_LENGTH) {
      setError(t('register.errors.passwordTooShort', {
        min: MIN_PASSWORD_LENGTH,
        defaultValue: 'Password must be at least {{min}} characters long',
      }))
      return
    }

    setLoading(true)
    try {
      const result = await register(username, email, password)
      if (result.pendingApproval) {
        setPendingApproval(true)
      } else {
        navigate('/dashboard')
      }
    } catch (err: any) {
      const status = err?.response?.status
      const serverMessage = err?.response?.data?.error ?? err?.response?.data?.message
      if (status === 403) {
        setError(t('register.errors.disabled', 'Self-registration is disabled on this server.'))
      } else if (status === 429) {
        setError(t('register.errors.rateLimited', 'Too many registration attempts. Please try again later.'))
      } else {
        setError(typeof serverMessage === 'string' && serverMessage ? serverMessage : t('register.errors.failed'))
      }
    } finally {
      setLoading(false)
    }
  }

  if (configLoading) {
    return (
      <Box sx={{ minHeight: '100vh', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <CircularProgress aria-label={t('common.loading', 'Loading...')} />
      </Box>
    )
  }

  if (!registrationAllowed) {
    return <Navigate to="/login" replace />
  }

  return (
    <Box
      sx={{
        minHeight: '100vh',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        bgcolor: 'background.default',
        p: 2,
      }}
    >
      <Container maxWidth="xs">
        <Card elevation={2} sx={{ borderRadius: 3 }}>
          <CardContent sx={{ p: 4 }}>
            <Typography variant="h5" component="h1" sx={{ mb: 3, fontWeight: 600, textAlign: 'center' }}>
              {t('register.title')}
            </Typography>

            {pendingApproval ? (
              <>
                <Alert severity="success" sx={{ mb: 3 }}>
                  {t(
                    'register.pendingApproval',
                    'Account created. An administrator must approve it before you can sign in.'
                  )}
                </Alert>
                <Button component={RouterLink} to="/login" variant="contained" fullWidth>
                  {t('register.actions.backToSignIn', 'Back to sign in')}
                </Button>
              </>
            ) : (
              <Box component="form" onSubmit={handleSubmit} noValidate>
                {error && (
                  <Alert severity="error" sx={{ mb: 2 }}>
                    {error}
                  </Alert>
                )}
                <TextField
                  id="username"
                  name="username"
                  label={t('register.fields.username')}
                  autoComplete="username"
                  required
                  fullWidth
                  margin="normal"
                  value={username}
                  onChange={(e) => setUsername(e.target.value)}
                />
                <TextField
                  id="email"
                  name="email"
                  type="email"
                  label={t('register.fields.email')}
                  autoComplete="email"
                  required
                  fullWidth
                  margin="normal"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                />
                <TextField
                  id="password"
                  name="password"
                  type="password"
                  label={t('register.fields.password')}
                  autoComplete="new-password"
                  required
                  fullWidth
                  margin="normal"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  helperText={t('register.passwordHint', {
                    min: MIN_PASSWORD_LENGTH,
                    defaultValue: 'At least {{min}} characters',
                  })}
                />
                <Button
                  type="submit"
                  variant="contained"
                  fullWidth
                  size="large"
                  disabled={loading || !username || !email || !password}
                  sx={{ mt: 2, mb: 2 }}
                >
                  {loading ? t('register.actions.creating') : t('register.actions.signup')}
                </Button>
                <Box sx={{ textAlign: 'center' }}>
                  <Link component={RouterLink} to="/login" variant="body2">
                    {t('register.links.signin')}
                  </Link>
                </Box>
              </Box>
            )}
          </CardContent>
        </Card>
      </Container>
    </Box>
  )
}

export default Register
