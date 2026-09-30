import axios from 'axios';
import type { TFunction } from 'i18next';
import { ErrorCodes, ErrorHelper } from '../../services/api';

/** No HTTP response at all: server down, DNS, offline or CORS. */
const isUnreachable = (err: unknown): boolean => axios.isAxiosError(err) && !err.response;

/** Maps a failed password sign-in to a message the user can act on. */
export function loginErrorMessage(err: unknown, t: TFunction): string {
  const info = ErrorHelper.formatErrorForDisplay(err, false);
  if (
    ErrorHelper.isErrorCode(err, ErrorCodes.USER_INVALID_CREDENTIALS) ||
    ErrorHelper.isErrorCode(err, ErrorCodes.USER_NOT_FOUND) ||
    info.status === 401
  ) {
    return t('auth.login.errors.invalidCredentials', 'Wrong username or password. Check both and try again.');
  }
  if (ErrorHelper.isErrorCode(err, ErrorCodes.USER_ACCOUNT_DISABLED)) {
    return t('auth.login.errors.accountDisabled', 'This account is disabled. Ask an administrator to re-enable it.');
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
  return info.message || t('auth.login.errors.generic', 'Sign-in failed. Try again.');
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
  if (path === '/login' || path.startsWith('/login?') || path.startsWith('/auth/callback')) return null;
  return path;
}
