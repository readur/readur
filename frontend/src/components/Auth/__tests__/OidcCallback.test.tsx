import React from 'react';
import { describe, test, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Routes, Route } from 'react-router-dom';
import OidcCallback from '../OidcCallback';
import { api } from '../../../services/api';
import { AuthContext } from '../../../contexts/AuthContext';

const completeLogin = vi.fn();

const renderCallback = () =>
  render(
    <AuthContext.Provider value={{ completeLogin } as any}>
      <MemoryRouter initialEntries={['/auth/callback']}>
        <Routes>
          <Route path="/auth/callback" element={<OidcCallback />} />
          <Route path="/dashboard" element={<div>Dashboard page</div>} />
        </Routes>
      </MemoryRouter>
    </AuthContext.Provider>
  );

const setUrl = (url: string) => window.history.replaceState(null, '', url);

describe('OidcCallback', () => {
  beforeEach(() => {
    vi.spyOn(api, 'post').mockRejectedValue(new Error('unexpected request'));
    vi.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    setUrl('/');
  });

  test('redeems the one-time code from the URL fragment and signs in', async () => {
    const session = {
      token: 'jwt-token',
      user: { id: '1', username: 'alice', email: 'a@example.com', role: 'user', is_active: true },
    };
    vi.mocked(api.post).mockResolvedValue({ data: session });
    setUrl('/auth/callback#code=one-time-code');

    renderCallback();

    await waitFor(() => {
      expect(api.post).toHaveBeenCalledWith('/auth/oidc/exchange', { code: 'one-time-code' });
    });
    expect(await screen.findByText('Dashboard page')).toBeInTheDocument();
    expect(api.post).toHaveBeenCalledTimes(1);
    expect(completeLogin).toHaveBeenCalledWith(session);
  });

  test('strips the code from the address bar before exchanging it', async () => {
    let hashDuringExchange: string | undefined;
    vi.mocked(api.post).mockImplementation(async () => {
      hashDuringExchange = window.location.hash;
      return { data: { token: 't', user: {} } };
    });
    setUrl('/auth/callback#code=secret-code');

    renderCallback();

    await waitFor(() => expect(api.post).toHaveBeenCalled());
    expect(hashDuringExchange).toBe('');
    expect(window.location.hash).toBe('');
    expect(window.location.pathname).toBe('/auth/callback');
  });

  test('does not read a token from the query string', async () => {
    setUrl('/auth/callback?token=legacy-token');

    renderCallback();

    expect(await screen.findByText(/No authentication code received/i)).toBeInTheDocument();
    expect(api.post).not.toHaveBeenCalled();
    expect(completeLogin).not.toHaveBeenCalled();
  });

  test('shows an error passed back in the callback URL', async () => {
    setUrl('/auth/callback?error=access_denied');

    renderCallback();

    expect(await screen.findByText(/Authentication failed: access_denied/)).toBeInTheDocument();
    expect(api.post).not.toHaveBeenCalled();
  });

  test('shows an error when the code is invalid or expired', async () => {
    vi.mocked(api.post).mockRejectedValue({
      isAxiosError: true,
      response: { status: 401, data: { error: 'Invalid or expired code' } },
    });
    setUrl('/auth/callback#code=stale');

    renderCallback();

    expect(await screen.findByText(/invalid or has expired/i)).toBeInTheDocument();
    expect(completeLogin).not.toHaveBeenCalled();
    expect(screen.queryByText('Dashboard page')).not.toBeInTheDocument();
    expect(screen.getByText('Return to Login')).toBeInTheDocument();
  });
});
