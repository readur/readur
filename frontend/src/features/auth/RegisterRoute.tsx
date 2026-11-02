import { useState, type FormEvent } from 'react';
import { Link, Navigate, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import type { TFunction } from 'i18next';
import { useAuth } from '../../contexts/AuthContext';
import { useFeatureFlags } from '../../contexts/FeatureFlagsContext';
import { Button, Skeleton, TextField } from '../../ui';
import { Error as ErrorIcon } from '../../ui/icons';
import { AuthLayout } from './AuthLayout';
import { MIN_PASSWORD_LENGTH, validateNewPassword } from './passwordPolicy';
import styles from './Auth.module.css';

const HOME_PATH = '/home';

/** Maps a failed registration to a fixed message. */
export function registerErrorMessage(err: unknown, t: TFunction): string {
  const response = (err as { response?: { status?: number; data?: { error?: unknown; message?: unknown } } } | null)
    ?.response;
  if (response?.status === 403) return t('register.errors.disabled', 'Self-registration is disabled on this server.');
  if (response?.status === 429) {
    return t('register.errors.rateLimited', 'Too many registration attempts. Please try again later.');
  }
  const serverMessage = response?.data?.error ?? response?.data?.message;
  return typeof serverMessage === 'string' && serverMessage ? serverMessage : t('register.errors.failed', 'Failed to register');
}

/**
 * Public /register page, reachable only when the server allows self-registration. New accounts
 * may need an administrator's approval before they can sign in.
 */
export default function RegisterRoute() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { register } = useAuth();
  const { flags, loading } = useFeatureFlags();
  const [username, setUsername] = useState('');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');
  const [pending, setPending] = useState(false);
  const [pendingApproval, setPendingApproval] = useState(false);

  if (!loading && !(flags.allowLocalAuth && flags.allowRegistration)) return <Navigate to="/login" replace />;

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    if (pending) return;
    setError('');
    const policyError = validateNewPassword(password, t);
    if (policyError) {
      setError(policyError);
      return;
    }
    setPending(true);
    try {
      const result = await register(username, email, password);
      if (result.pendingApproval) {
        setPendingApproval(true);
        setPending(false);
      } else {
        navigate(HOME_PATH, { replace: true });
      }
    } catch (err) {
      setError(registerErrorMessage(err, t));
      setPending(false);
    }
  };

  return (
    <AuthLayout>
      <h1 className={styles.title}>{t('register.title', 'Create your Readur account')}</h1>
      {loading ? (
        <Skeleton lines={3} label={t('common.status.loading', 'Loading...')} />
      ) : pendingApproval ? (
        <div className={styles.panel}>
          <p role="status" className={styles.info}>
            <span>
              {t('register.pendingApproval', 'Account created. An administrator must approve it before you can sign in.')}
            </span>
          </p>
          <Link to="/login" className={styles.backLink}>
            {t('register.actions.backToSignIn', 'Back to sign in')}
          </Link>
        </div>
      ) : (
        <div className={styles.panel}>
          {error ? (
            <p role="alert" className={styles.alert}>
              <ErrorIcon fontSize="small" aria-hidden="true" />
              <span>{error}</span>
            </p>
          ) : null}
          <form className={styles.form} onSubmit={onSubmit} noValidate>
            <TextField
              label={t('register.fields.username', 'Username')}
              name="username"
              autoComplete="username"
              autoFocus
              isRequired
              value={username}
              onChange={setUsername}
            />
            <TextField
              label={t('register.fields.email', 'Email')}
              name="email"
              type="email"
              autoComplete="email"
              isRequired
              value={email}
              onChange={setEmail}
            />
            <TextField
              label={t('register.fields.password', 'Password')}
              name="password"
              type="password"
              autoComplete="new-password"
              isRequired
              value={password}
              onChange={setPassword}
              description={t('register.passwordHint', { min: MIN_PASSWORD_LENGTH, defaultValue: 'At least {{min}} characters' })}
            />
            <Button
              type="submit"
              variant="primary"
              isPending={pending}
              isDisabled={!username || !email || !password}
            >
              {pending ? t('register.actions.creating', 'Creating account...') : t('register.actions.signup', 'Sign up')}
            </Button>
          </form>
          <p className={styles.linkRow}>
            <Link to="/login" className={styles.backLink}>
              {t('register.links.signin', 'Already have an account? Sign in')}
            </Link>
          </p>
        </div>
      )}
    </AuthLayout>
  );
}
