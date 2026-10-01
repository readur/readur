import axios from 'axios';
import type { TFunction } from 'i18next';
import { ErrorCodes, ErrorHelper } from '../../services/api';

/** No HTTP response at all: server down, DNS, offline or CORS. */
const isUnreachable = (err: unknown): boolean => axios.isAxiosError(err) && !err.response;

const statusOf = (err: unknown): number | undefined =>
  (err as { response?: { status?: number } } | null)?.response?.status;

/** Seconds from a 429 response's `Retry-After` header, when it carries a positive number. */
export function retryAfterSeconds(err: unknown): number | null {
  const headers = (err as { response?: { headers?: Record<string, unknown> } } | null)?.response?.headers;
  const raw = headers?.['retry-after'] ?? headers?.['Retry-After'];
  const seconds = raw !== undefined ? parseInt(String(raw), 10) : NaN;
  return Number.isFinite(seconds) && seconds > 0 ? seconds : null;
}

/** Maps a failed password sign-in to a message the user can act on. */
export function loginErrorMessage(err: unknown, t: TFunction): string {
  const info = ErrorHelper.formatErrorForDisplay(err, false);
  if (statusOf(err) === 429) {
    const wait = retryAfterSeconds(err);
    return wait
      ? t('auth.errors.tooManyAttemptsRetry', {
          seconds: wait,
          defaultValue: 'Too many sign-in attempts. Please try again in {{seconds}} seconds.',
        })
      : t('auth.errors.tooManyAttempts', 'Too many sign-in attempts. Please try again later.');
  }
  if (ErrorHelper.isErrorCode(err, ErrorCodes.USER_ACCOUNT_DISABLED)) {
    return t('auth.login.errors.accountDisabled', 'This account is disabled. Ask an administrator to re-enable it.');
  }
  if (statusOf(err) === 403) {
    return t('auth.errors.accountDisabledOrPending', 'This account is disabled or awaiting administrator approval');
  }
  if (
    ErrorHelper.isErrorCode(err, ErrorCodes.USER_INVALID_CREDENTIALS) ||
    ErrorHelper.isErrorCode(err, ErrorCodes.USER_NOT_FOUND) ||
    info.status === 401
  ) {
    return t('auth.login.errors.invalidCredentials', 'Wrong username or password. Check both and try again.');
  }
  if (
    ErrorHelper.isErrorCode(err, ErrorCodes.USER_SESSION_EXPIRED) ||
    ErrorHelper.isErrorCode(err, ErrorCodes.USER_TOKEN_EXPIRED)
  ) {
    return t('auth.login.errors.sessionExpired', 'Your session expired. Sign in again.');
  }
  if (isUnreachable(err) || info.category === 'network') {
    return t('auth.login.errors.unreachable', 'Cannot reach the server. Check your connection and try again.');
  }
  if (info.category === 'server') {
    return t('auth.login.errors.server', 'The server had a problem. Try again in a moment.');
  }
  return t('auth.login.errors.generic', 'Sign-in failed. Try again.');
}

/** Maps a failed SSO start to a message the user can act on. */
export function ssoErrorMessage(err: unknown, t: TFunction): string {
  const info = ErrorHelper.formatErrorForDisplay(err, false);
  if (ErrorHelper.isErrorCode(err, ErrorCodes.USER_AUTH_PROVIDER_NOT_CONFIGURED)) {
    return t('auth.login.errors.ssoNotConfigured', 'SSO is not set up on this server. Sign in with your username and password.');
  }
  if (ErrorHelper.isErrorCode(err, ErrorCodes.USER_OIDC_AUTH_FAILED)) {
    return t('auth.login.errors.ssoFailed', 'SSO sign-in failed. Try again or ask your administrator.');
  }
  if (isUnreachable(err) || info.category === 'network') {
    return t('auth.login.errors.unreachable', 'Cannot reach the server. Check your connection and try again.');
  }
  return t('auth.login.errors.ssoStart', 'Could not start SSO sign-in. Try again.');
}

/** Only same-app absolute paths are honoured as a post-login destination. */
export function safeRedirect(from: unknown): string | null {
  let path: string | null = null;
  if (typeof from === 'string') path = from;
  else if (from && typeof from === 'object' && 'pathname' in from) {
    const l = from as { pathname?: string; search?: string; hash?: string };
    path = `${l.pathname ?? ''}${l.search ?? ''}${l.hash ?? ''}`;
  }
  if (!path || !path.startsWith('/') || path.startsWith('//')) return null;
  // Backslashes are treated as slashes by browsers (`/\evil.com`); control characters can smuggle URLs.
  for (const ch of path) {
    const code = ch.charCodeAt(0);
    if (ch === '\\' || code <= 0x1f || code === 0x7f) return null;
  }
  if (path === '/login' || path.startsWith('/login?') || path.startsWith('/auth/callback')) return null;
  return path;
}

/**
 * Failure codes the server puts in the callback URL fragment. Only these are recognised; any
 * other value (including free text) gets a generic message and is never rendered.
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

const CALLBACK_FALLBACKS: Record<OidcCallbackErrorCode | 'unknown', string> = {
  provider_error: 'The identity provider did not complete the sign-in. Please try again.',
  invalid_state: 'The sign-in request is invalid or has expired. Please try logging in again.',
  auth_failed: 'Authentication with the identity provider failed. Please try logging in again.',
  no_account: 'No account is available for this identity. Contact your administrator.',
  account_disabled: 'This account is disabled or awaiting administrator approval.',
  server_error: 'A server error occurred during sign-in. Please try again later.',
  unknown: 'Sign-in failed. Please try logging in again.',
};

const isKnownCallbackCode = (value: string): value is OidcCallbackErrorCode =>
  (OIDC_CALLBACK_ERROR_CODES as readonly string[]).includes(value);

/** Callback error text is never taken from the URL: known codes get a fixed message, anything else a generic one. */
export function callbackErrorMessage(code: string, t: TFunction): string {
  const key = isKnownCallbackCode(code) ? code : 'unknown';
  return t(`auth.oidcCallback.errors.${key}`, CALLBACK_FALLBACKS[key]);
}

/** Maps a failed exchange of the one-time sign-in code. Server-provided text is never shown. */
export function exchangeErrorMessage(err: unknown, t: TFunction): string {
  const status = statusOf(err);
  if (status === 401) {
    return t('auth.oidcCallback.errors.invalidCode', 'This sign-in link is invalid or has expired. Please try logging in again.');
  }
  if (status === 429) {
    return t('auth.oidcCallback.errors.tooManyAttempts', 'Too many attempts. Please wait a moment and try logging in again.');
  }
  const info = ErrorHelper.formatErrorForDisplay(err, false);
  if (isUnreachable(err) || info.category === 'network') {
    return t('auth.login.errors.unreachable', 'Cannot reach the server. Check your connection and try again.');
  }
  if (info.category === 'server') return callbackErrorMessage('server_error', t);
  return callbackErrorMessage('unknown', t);
}
