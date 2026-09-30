import { useEffect, useRef, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useAuth, type LoginResponse } from '../../contexts/AuthContext';
import { api } from '../../services/api';
import { callbackErrorMessage, exchangeErrorMessage } from './authErrors';
import { AuthLayout } from './AuthLayout';
import styles from './Auth.module.css';

const HOME_PATH = '/board';

/**
 * Reads the one-time sign-in code (or a failure code) from the URL fragment
 * (`/auth/callback#code=…` / `#error=…`). `error` is also read from the query so
 * failures reported before the server's redirect still show.
 */
function readCallbackParams(): { code: string | null; error: string | null } {
  const hash = window.location.hash.startsWith('#') ? window.location.hash.slice(1) : window.location.hash;
  const fragment = new URLSearchParams(hash);
  const query = new URLSearchParams(window.location.search);
  return { code: fragment.get('code'), error: fragment.get('error') ?? query.get('error') };
}

/** Removes the code from the address bar and the history entry straight away. */
function stripCallbackParams(): void {
  if (window.location.hash || window.location.search) {
    window.history.replaceState(window.history.state, '', window.location.pathname);
  }
}

/** Public /auth/callback page: exchanges the SSO sign-in code for a session, then opens the app. */
export default function CallbackRoute() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { completeLogin } = useAuth();
  const [error, setError] = useState('');
  // The code is single-use: redeem it once even if the effect runs twice (StrictMode).
  const started = useRef(false);

  useEffect(() => {
    if (started.current) return;
    started.current = true;

    const { code, error: callbackError } = readCallbackParams();
    stripCallbackParams();

    if (callbackError) {
      setError(callbackErrorMessage(callbackError, t));
      return;
    }
    if (!code) {
      setError(t('auth.oidcCallback.errors.missingCode', 'No authentication code received from server.'));
      return;
    }
    api
      .post<LoginResponse>('/auth/oidc/exchange', { code })
      .then((response) => {
        completeLogin(response.data);
        navigate(HOME_PATH, { replace: true });
      })
      .catch((err: unknown) => {
        console.error('OIDC callback error:', err);
        setError(exchangeErrorMessage(err, t));
      });
    // Runs once per mount: the code in the URL is consumed on the first run.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

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
