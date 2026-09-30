import React from 'react';
import { describe, test, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Routes, Route } from 'react-router-dom';
import OidcCallback, { OIDC_CALLBACK_ERROR_CODES } from '../OidcCallback';
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

  const expectedMessages: Record<string, RegExp> = {
    provider_error: /identity provider did not complete the sign-in/i,
    invalid_state: /sign-in request is invalid or has expired/i,
    auth_failed: /Authentication with the identity provider failed/i,
    no_account: /No account is available for this identity/i,
    account_disabled: /disabled or awaiting administrator approval/i,
    server_error: /server error occurred during sign-in/i,
  };

  test('covers every server error code', () => {
    expect([...OIDC_CALLBACK_ERROR_CODES].sort()).toEqual(Object.keys(expectedMessages).sort());
  });

  test.each(Object.entries(expectedMessages))(
    'maps the %s error code in the URL fragment to a fixed message',
    async (code, message) => {
      setUrl(`/auth/callback#error=${code}`);

      renderCallback();

      expect(await screen.findByText(message)).toBeInTheDocument();
      expect(api.post).not.toHaveBeenCalled();
      expect(window.location.hash).toBe('');
    }
  );

  test('never renders free text from the callback URL', async () => {
    setUrl('/auth/callback?error=Please%20call%20555-0100%20to%20verify');

    renderCallback();

    expect(await screen.findByText(/Sign-in failed. Please try logging in again./)).toBeInTheDocument();
    expect(screen.queryByText(/555-0100/)).not.toBeInTheDocument();
    expect(api.post).not.toHaveBeenCalled();
  });

  test('does not render the server message when the exchange fails unexpectedly', async () => {
    vi.mocked(api.post).mockRejectedValue({
      isAxiosError: true,
      response: { status: 400, data: { error: 'Some server-provided text' } },
    });
    setUrl('/auth/callback#code=abc');

    renderCallback();

    expect(await screen.findByText(/Sign-in failed. Please try logging in again./)).toBeInTheDocument();
    expect(screen.queryByText(/Some server-provided text/)).not.toBeInTheDocument();
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
