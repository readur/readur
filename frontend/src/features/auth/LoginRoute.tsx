import { useState, type FormEvent } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useAuth } from '../../contexts/AuthContext';
import { useFeatureFlags } from '../../contexts/FeatureFlagsContext';
import { Button, IconButton, Skeleton, TextField } from '../../ui';
import { Visibility, VisibilityOff, Error as ErrorIcon } from '../../ui/icons';
import { AuthLayout } from './AuthLayout';
import { loginErrorMessage, safeRedirect, ssoErrorMessage } from './authErrors';
import pkg from '../../../package.json';
import styles from './Auth.module.css';

const HOME_PATH = '/board';
const OIDC_LOGIN_URL = '/api/auth/oidc/login';

/** Public /login page. Field visibility follows the server's auth config. */
export default function LoginRoute() {
  const { t } = useTranslation();
  const { login } = useAuth();
  const { flags, loading } = useFeatureFlags();
  const navigate = useNavigate();
  const location = useLocation();

  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [error, setError] = useState('');
  const [missing, setMissing] = useState({ username: false, password: false });
  const [pending, setPending] = useState(false);
  const [ssoPending, setSsoPending] = useState(false);

  const busy = pending || ssoPending;
  const destination = safeRedirect((location.state as { from?: unknown } | null)?.from) ?? HOME_PATH;

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    if (busy) return;
    const next = { username: username.trim() === '', password: password === '' };
    setMissing(next);
    if (next.username || next.password) return;
    setError('');
    setPending(true);
    try {
      await login(username, password);
      navigate(destination, { replace: true });
    } catch (err) {
      setError(loginErrorMessage(err, t));
      setPending(false);
    }
  };

  const onSso = () => {
    if (busy) return;
    setError('');
    setSsoPending(true);
    try {
      window.location.href = OIDC_LOGIN_URL;
    } catch (err) {
      setError(ssoErrorMessage(err, t));
      setSsoPending(false);
    }
  };

  const noMethods = !flags.allowLocalAuth && !flags.oidcEnabled;

  return (
    <AuthLayout footer={`v${pkg.version}`}>
      <h1 className={styles.title}>{t('auth.login.title', 'Sign in')}</h1>
      {loading ? (
        <Skeleton lines={3} label={t('auth.login.loading', 'Loading sign-in options')} />
      ) : (
        <div className={styles.panel}>
          {error ? (
            <p role="alert" className={styles.alert}>
              <ErrorIcon fontSize="small" aria-hidden="true" />
              <span>{error}</span>
            </p>
          ) : null}

          {flags.allowLocalAuth ? (
            <form className={styles.form} onSubmit={onSubmit} noValidate>
              <TextField
                label={t('auth.login.username', 'Username')}
                name="username"
                autoComplete="username"
                autoFocus
                value={username}
                onChange={(v) => {
                  setUsername(v);
                  if (missing.username) setMissing((m) => ({ ...m, username: false }));
                }}
                isInvalid={missing.username}
                errorMessage={t('auth.login.usernameRequired', 'Enter your username')}
              />
              <div className={styles.passwordRow}>
                <TextField
                  label={t('auth.login.password', 'Password')}
                  name="password"
                  type={showPassword ? 'text' : 'password'}
                  autoComplete="current-password"
                  value={password}
                  onChange={(v) => {
                    setPassword(v);
                    if (missing.password) setMissing((m) => ({ ...m, password: false }));
                  }}
                  isInvalid={missing.password}
                  errorMessage={t('auth.login.passwordRequired', 'Enter your password')}
                />
                <IconButton
                  label={
                    showPassword
                      ? t('auth.login.hidePassword', 'Hide password')
                      : t('auth.login.showPassword', 'Show password')
                  }
                  icon={showPassword ? <VisibilityOff fontSize="small" /> : <Visibility fontSize="small" />}
                  onPress={() => setShowPassword((s) => !s)}
                  aria-pressed={showPassword}
                />
              </div>
              <Button type="submit" variant="primary" isPending={pending} isDisabled={ssoPending}>
                {pending ? t('auth.login.signingIn', 'Signing in…') : t('auth.login.submit', 'Sign in')}
              </Button>
            </form>
          ) : null}

          {flags.allowLocalAuth && flags.oidcEnabled ? (
            <div className={styles.divider}>{t('auth.login.or', 'or')}</div>
          ) : null}

          {flags.oidcEnabled ? (
            <Button variant="secondary" onPress={onSso} isPending={ssoPending} isDisabled={pending}>
              {ssoPending ? t('auth.login.redirecting', 'Redirecting…') : t('auth.login.sso', 'Sign in with SSO')}
            </Button>
          ) : null}

          {noMethods ? (
            <p className={styles.notice}>
              {t(
                'auth.login.noMethods',
                'No sign-in method is enabled on this server. Ask an administrator to enable password or SSO sign-in.',
              )}
            </p>
          ) : null}
        </div>
      )}
    </AuthLayout>
  );
}
