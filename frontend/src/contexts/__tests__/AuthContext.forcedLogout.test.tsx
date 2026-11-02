import { beforeEach, describe, expect, it, vi } from 'vitest';
import { act, render, screen, waitFor } from '@testing-library/react';
import type { AxiosAdapter, InternalAxiosRequestConfig } from 'axios';

// A real axios instance, so the session interceptor AuthContext installs is exercised.
vi.mock('../../services/api', async () => {
  const axios = (await import('axios')).default;
  const api = axios.create({ baseURL: '/api' });
  return { api, default: api, documentService: { list: vi.fn() } };
});

import { api } from '../../services/api';
import { AuthProvider, useAuth } from '../AuthContext';
import { AUTH_LOGOUT_EVENT } from '../../services/authEvents';
import { isLit, LIT_STORAGE_KEY, markLit } from '../../features/board/litStore';
import { readRecentSearches, saveRecentSearch } from '../../features/shell/recentSearches';

const ALICE = { id: '1', username: 'alice', email: 'a@example.com', role: 'user', is_active: true };

type Handler = (config: InternalAxiosRequestConfig) => Promise<{ status: number; data: unknown }>;

/** Routes every request through `handler`; a status >= 400 rejects like axios does. */
function serve(handler: Handler) {
  const adapter: AxiosAdapter = async (config) => {
    const { status, data } = await handler(config);
    const response = { status, data, headers: {}, config, statusText: '' };
    if (status >= 400) {
      return Promise.reject(Object.assign(new Error(`status ${status}`), { isAxiosError: true, config, response }));
    }
    return response;
  };
  api.defaults.adapter = adapter;
}

let auth: ReturnType<typeof useAuth>;
function Probe() {
  auth = useAuth();
  return <span data-testid="user">{auth.user?.username ?? 'none'}</span>;
}

function seedActivity() {
  localStorage.setItem('readur.board.dismissed.v1', '["a"]');
  localStorage.setItem('readur.intake.sourceFailures.v1', '["x"]');
  localStorage.setItem('themeMode', 'dark');
  saveRecentSearch('tax 2024');
  markLit('document', 'd1', 'new');
}

function expectActivityCleared() {
  expect(localStorage.getItem(LIT_STORAGE_KEY)).toBeNull();
  expect(readRecentSearches()).toEqual([]);
  expect(localStorage.getItem('readur.board.dismissed.v1')).toBeNull();
  expect(localStorage.getItem('readur.intake.sourceFailures.v1')).toBeNull();
  expect(isLit('document', 'd1')).toBe(false);
  // Preferences stay.
  expect(localStorage.getItem('themeMode')).toBe('dark');
}

async function signedIn() {
  render(
    <AuthProvider>
      <Probe />
    </AuthProvider>,
  );
  await waitFor(() => expect(screen.getByTestId('user')).toHaveTextContent('alice'));
}

describe('AuthProvider forced sign-out', () => {
  beforeEach(() => {
    localStorage.clear();
    vi.spyOn(console, 'error').mockImplementation(() => {});
    localStorage.setItem('token', 'current');
  });

  it('forgets per-browser activity when the current token is rejected', async () => {
    serve(async (config) => (config.url === '/auth/me' ? { status: 200, data: ALICE } : { status: 401, data: {} }));
    await signedIn();
    seedActivity();

    await act(async () => {
      await api.get('/documents').catch(() => undefined);
    });

    expect(screen.getByTestId('user')).toHaveTextContent('none');
    expect(localStorage.getItem('token')).toBeNull();
    expectActivityCleared();
  });

  it('forgets per-browser activity when the stored token is rejected on load', async () => {
    seedActivity();
    serve(async () => ({ status: 401, data: {} }));
    render(
      <AuthProvider>
        <Probe />
      </AuthProvider>,
    );
    await waitFor(() => expect(localStorage.getItem('token')).toBeNull());
    expectActivityCleared();
  });

  it('forgets per-browser activity on the logout event', async () => {
    serve(async () => ({ status: 200, data: ALICE }));
    await signedIn();
    seedActivity();
    act(() => {
      window.dispatchEvent(new Event(AUTH_LOGOUT_EVENT));
    });
    expect(screen.getByTestId('user')).toHaveTextContent('none');
    expectActivityCleared();
  });

  it('keeps the session and activity while a password change swaps the token', async () => {
    let releasePasswordChange: () => void = () => {};
    const passwordChanged = new Promise<void>((resolve) => {
      releasePasswordChange = resolve;
    });
    serve(async (config) => {
      if (config.url === '/auth/me') return { status: 200, data: ALICE };
      if (config.url === '/auth/password') {
        await passwordChanged;
        return { status: 200, data: { token: 'rotated', user: ALICE } };
      }
      // The server has already revoked the old token.
      return { status: 401, data: {} };
    });
    await signedIn();
    seedActivity();

    let change: Promise<void> = Promise.resolve();
    act(() => {
      change = auth.changePassword('old-password', 'new-password');
    });
    // A request with the old token is rejected before the new session arrives.
    await act(async () => {
      await api.get('/documents', { headers: { Authorization: 'Bearer current' } }).catch(() => undefined);
    });
    releasePasswordChange();
    await act(async () => {
      await change;
    });

    expect(screen.getByTestId('user')).toHaveTextContent('alice');
    expect(localStorage.getItem('token')).toBe('rotated');
    expect(isLit('document', 'd1')).toBe(true);
    expect(readRecentSearches()).toEqual(['tax 2024']);
    expect(localStorage.getItem('readur.board.dismissed.v1')).toBe('["a"]');
  });
});
