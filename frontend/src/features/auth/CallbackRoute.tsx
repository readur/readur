import { useEffect, useState } from 'react';
import { Link, useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { api } from '../../services/api';
import { AuthLayout } from './AuthLayout';
import styles from './Auth.module.css';

const HOME_PATH = '/board';

/** Public /auth/callback page: stores the SSO token, then reloads into the app. */
export default function CallbackRoute() {
  const { t } = useTranslation();
  const [searchParams] = useSearchParams();
  const [error, setError] = useState('');

  useEffect(() => {
    const token = searchParams.get('token');
    const errorParam = searchParams.get('error');
    if (errorParam) {
      setError(t('auth.callback.failed', { defaultValue: 'SSO sign-in failed: {{reason}}', reason: errorParam }));
      return;
    }
    if (!token) {
      setError(t('auth.callback.noToken', 'The server did not send a sign-in token. Go back and try again.'));
      return;
    }
    try {
      localStorage.setItem('token', token);
      api.defaults.headers.common['Authorization'] = `Bearer ${token}`;
      // Full load so the auth context restores the session from the stored token.
      window.location.href = HOME_PATH;
    } catch (err) {
      console.error('OIDC callback error:', err);
      setError(t('auth.callback.generic', 'Could not finish signing you in. Try again.'));
    }
  }, [searchParams, t]);

  return (
    <AuthLayout>
      {error ? (
        <div className={styles.panel}>
          <h1 className={styles.title}>{t('auth.callback.errorTitle', 'Sign-in failed')}</h1>
          <p role="alert" className={styles.alert}>
            <span>{error}</span>
          </p>
          <Link to="/login" className={styles.backLink}>
            {t('auth.callback.back', 'Back to sign in')}
          </Link>
        </div>
      ) : (
        <div className={styles.status} role="status">
          <h1 className={styles.title}>{t('auth.callback.title', 'Signing you in…')}</h1>
          <div className={styles.statusRow}>
            <span className={styles.mark} aria-hidden="true">◐</span>
            <span>{t('auth.callback.wait', 'One moment while we finish signing you in.')}</span>
          </div>
        </div>
      )}
    </AuthLayout>
  );
}
