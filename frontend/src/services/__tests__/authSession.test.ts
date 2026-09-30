import { describe, test, expect, vi } from 'vitest';
import axios from 'axios';
import {
  AUTH_LOGOUT_EVENT,
  installSessionInterceptor,
  isCurrentSessionToken,
  shouldResetSessionOn401,
  withSessionRotation,
} from '../authEvents';
import { isSafeInlineMime, previewSandbox } from '../contentSafety';

describe('shouldResetSessionOn401', () => {
  test('resets the session for ordinary API requests', () => {
    expect(shouldResetSessionOn401('/documents')).toBe(true);
    expect(shouldResetSessionOn401('/auth/me')).toBe(true);
  });

  test('leaves credential-checking auth endpoints to their callers', () => {
    for (const url of ['/auth/login', '/auth/register', '/auth/password', '/auth/oidc/exchange', '/auth/logout']) {
      expect(shouldResetSessionOn401(url)).toBe(false);
    }
  });
});

describe('session interceptor', () => {
  const createApi = () => {
    const api = axios.create({ baseURL: '/api' });
    installSessionInterceptor(api);
    api.defaults.adapter = (config: any) =>
      Promise.reject({
        config,
        isAxiosError: true,
        response: { status: 401, data: { error: 'Unauthorized' }, headers: {}, config },
      });
    return api;
  };

  test('clears the session when an authenticated request is rejected', async () => {
    vi.mocked(localStorage.getItem).mockReturnValue('stale');
    const api = createApi();
    const listener = vi.fn();
    window.addEventListener(AUTH_LOGOUT_EVENT, listener);

    await expect(
      api.get('/documents', { headers: { Authorization: 'Bearer stale' } })
    ).rejects.toBeDefined();

    expect(localStorage.removeItem).toHaveBeenCalledWith('token');
    expect(listener).toHaveBeenCalledTimes(1);
    window.removeEventListener(AUTH_LOGOUT_EVENT, listener);
  });

  test('keeps a newer session when a request with an older token is rejected', async () => {
    vi.mocked(localStorage.getItem).mockReturnValue('fresh');
    const api = createApi();
    const listener = vi.fn();
    window.addEventListener(AUTH_LOGOUT_EVENT, listener);

    await expect(
      api.get('/documents', { headers: { Authorization: 'Bearer stale' } })
    ).rejects.toBeDefined();

    expect(localStorage.removeItem).not.toHaveBeenCalled();
    expect(listener).not.toHaveBeenCalled();
    window.removeEventListener(AUTH_LOGOUT_EVENT, listener);
  });

  test('ignores rejected requests sent without a token', async () => {
    vi.mocked(localStorage.getItem).mockReturnValue('current');
    const api = createApi();
    const listener = vi.fn();
    window.addEventListener(AUTH_LOGOUT_EVENT, listener);

    await expect(api.get('/documents')).rejects.toBeDefined();

    expect(localStorage.removeItem).not.toHaveBeenCalled();
    expect(listener).not.toHaveBeenCalled();
    window.removeEventListener(AUTH_LOGOUT_EVENT, listener);
  });

  test('isCurrentSessionToken compares against the stored token', () => {
    vi.mocked(localStorage.getItem).mockReturnValue('abc');
    expect(isCurrentSessionToken('Bearer abc')).toBe(true);
    expect(isCurrentSessionToken('Bearer other')).toBe(false);
    expect(isCurrentSessionToken(undefined)).toBe(false);
    vi.mocked(localStorage.getItem).mockReturnValue(null);
    expect(isCurrentSessionToken('Bearer abc')).toBe(false);
  });

  test('keeps the session while its token is being replaced', async () => {
    vi.mocked(localStorage.getItem).mockReturnValue('old');
    const api = createApi();
    const listener = vi.fn();
    window.addEventListener(AUTH_LOGOUT_EVENT, listener);

    // A request with the old token is rejected after the server revoked it
    // but before the password-change response stored the new token.
    await withSessionRotation(async () => {
      await expect(
        api.get('/documents', { headers: { Authorization: 'Bearer old' } })
      ).rejects.toBeDefined();
    });
    expect(localStorage.removeItem).not.toHaveBeenCalled();
    expect(listener).not.toHaveBeenCalled();

    // Once the rotation is over (e.g. it failed), a rejected token ends the session again.
    await expect(
      api.get('/documents', { headers: { Authorization: 'Bearer old' } })
    ).rejects.toBeDefined();
    expect(localStorage.removeItem).toHaveBeenCalledWith('token');
    expect(listener).toHaveBeenCalledTimes(1);
    window.removeEventListener(AUTH_LOGOUT_EVENT, listener);
  });

  test('lifts the rotation guard when the rotation fails', async () => {
    vi.mocked(localStorage.getItem).mockReturnValue('old');
    const api = createApi();
    const listener = vi.fn();
    window.addEventListener(AUTH_LOGOUT_EVENT, listener);

    await expect(withSessionRotation(async () => { throw new Error('failed') })).rejects.toThrow('failed');
    await expect(
      api.get('/documents', { headers: { Authorization: 'Bearer old' } })
    ).rejects.toBeDefined();
    expect(listener).toHaveBeenCalledTimes(1);
    window.removeEventListener(AUTH_LOGOUT_EVENT, listener);
  });

  test('does not clear the session on a failed login', async () => {
    const api = createApi();
    const listener = vi.fn();
    window.addEventListener(AUTH_LOGOUT_EVENT, listener);

    await expect(
      api.post('/auth/login', { username: 'a', password: 'b' }, { headers: { Authorization: 'Bearer x' } })
    ).rejects.toBeDefined();

    expect(listener).not.toHaveBeenCalled();
    window.removeEventListener(AUTH_LOGOUT_EVENT, listener);
  });
});

describe('content safety helpers', () => {
  test('only sandboxes non-PDF previews', () => {
    expect(previewSandbox('application/pdf')).toBeUndefined();
    expect(previewSandbox('text/html')).toBe('');
    expect(previewSandbox('text/plain')).toBe('');
    expect(previewSandbox(undefined)).toBe('');
  });

  test('only allows passive types to open directly', () => {
    expect(isSafeInlineMime('application/pdf')).toBe(true);
    expect(isSafeInlineMime('image/png')).toBe(true);
    expect(isSafeInlineMime('text/plain; charset=utf-8')).toBe(true);
    expect(isSafeInlineMime('text/html')).toBe(false);
    expect(isSafeInlineMime('image/svg+xml')).toBe(false);
    expect(isSafeInlineMime('')).toBe(false);
  });
});
