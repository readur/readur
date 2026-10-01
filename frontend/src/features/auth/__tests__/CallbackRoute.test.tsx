import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import CallbackRoute from '../CallbackRoute';
import { OIDC_CALLBACK_ERROR_CODES } from '../authErrors';
import { api } from '../../../services/api';
import { renderAuth } from './authTestUtils';

vi.mock('../../../services/api', async () => {
  const errors = await import('../../../services/errors');
  return {
    api: { post: vi.fn(), defaults: { headers: { common: {} as Record<string, string> } } },
    ErrorHelper: errors.ErrorHelper,
    ErrorCodes: errors.ErrorCodes,
  };
});

const completeLogin = vi.fn();
const setUrl = (url: string) => window.history.replaceState(null, '', url);

const renderCallback = () =>
  renderAuth({ path: '/auth/callback', element: <CallbackRoute />, auth: { completeLogin } });

beforeEach(() => {
  completeLogin.mockReset();
  vi.mocked(api.post).mockReset();
  vi.mocked(api.post).mockRejectedValue(new Error('unexpected request'));
  vi.spyOn(console, 'error').mockImplementation(() => {});
});

afterEach(() => {
  setUrl('/');
});

describe('CallbackRoute', () => {
  it('redeems the one-time code from the URL fragment, signs in and goes to /home', async () => {
    const session = {
      token: 'jwt-token',
      user: { id: '1', username: 'alice', email: 'a@example.com', role: 'user' as const, is_active: true },
    };
    vi.mocked(api.post).mockResolvedValue({ data: session });
    setUrl('/auth/callback#code=one-time-code');

    renderCallback();
    expect(screen.getByRole('heading', { level: 1, name: 'Signing you in…' })).toBeInTheDocument();

    expect(await screen.findByRole('status', { name: 'location' })).toHaveTextContent('/home');
    expect(api.post).toHaveBeenCalledWith('/auth/oidc/exchange', { code: 'one-time-code' });
    expect(api.post).toHaveBeenCalledTimes(1);
    expect(completeLogin).toHaveBeenCalledWith(session);
  });

  it('strips the code from the address bar before exchanging it', async () => {
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

  it('does not read a token from the query string', async () => {
    setUrl('/auth/callback?token=legacy-token');

    renderCallback();

    expect(await screen.findByRole('alert')).toHaveTextContent('No authentication code received');
    expect(api.post).not.toHaveBeenCalled();
    expect(completeLogin).not.toHaveBeenCalled();
    expect(window.location.search).toBe('');
  });

  const expectedMessages: Record<string, RegExp> = {
    provider_error: /identity provider did not complete the sign-in/i,
    invalid_state: /sign-in request is invalid or has expired/i,
    auth_failed: /Authentication with the identity provider failed/i,
    no_account: /No account is available for this identity/i,
    account_disabled: /disabled or awaiting administrator approval/i,
    server_error: /server error occurred during sign-in/i,
  };

  it('covers every server error code', () => {
    expect([...OIDC_CALLBACK_ERROR_CODES].sort()).toEqual(Object.keys(expectedMessages).sort());
  });

  it.each(Object.entries(expectedMessages))('maps the %s error code in the fragment to a fixed message', async (code, message) => {
    setUrl(`/auth/callback#error=${code}`);

    renderCallback();

    expect(await screen.findByRole('alert')).toHaveTextContent(message);
    expect(api.post).not.toHaveBeenCalled();
    expect(window.location.hash).toBe('');
  });

  it('shows the error with a way back to sign in', async () => {
    const user = userEvent.setup();
    setUrl('/auth/callback#error=auth_failed');
    renderCallback();
    await screen.findByRole('alert');
    await user.click(screen.getByRole('link', { name: 'Back to sign in' }));
    expect(await screen.findByRole('status', { name: 'location' })).toHaveTextContent('/login');
  });

  it('never renders free text from the callback URL', async () => {
    setUrl('/auth/callback?error=' + encodeURIComponent('Your account is locked, call 555-0100'));

    renderCallback();

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('Sign-in failed. Please try logging in again.');
    expect(alert).not.toHaveTextContent('555-0100');
    expect(api.post).not.toHaveBeenCalled();
  });

  it('shows an error when the code is invalid or expired', async () => {
    vi.mocked(api.post).mockRejectedValue({ isAxiosError: true, response: { status: 401, data: { error: 'Invalid' } } });
    setUrl('/auth/callback#code=stale');

    renderCallback();

    expect(await screen.findByRole('alert')).toHaveTextContent(/invalid or has expired/i);
    expect(completeLogin).not.toHaveBeenCalled();
    expect(screen.getByRole('link', { name: 'Back to sign in' })).toHaveAttribute('href', '/login');
  });

  it('asks the user to wait after too many attempts', async () => {
    vi.mocked(api.post).mockRejectedValue({ isAxiosError: true, response: { status: 429, data: {} } });
    setUrl('/auth/callback#code=abc');

    renderCallback();

    expect(await screen.findByRole('alert')).toHaveTextContent('Too many attempts');
  });

  it('does not render the server message when the exchange fails unexpectedly', async () => {
    vi.mocked(api.post).mockRejectedValue({
      isAxiosError: true,
      response: { status: 400, data: { error: 'Some server-provided text' } },
    });
    setUrl('/auth/callback#code=abc');

    renderCallback();

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('Sign-in failed. Please try logging in again.');
    expect(alert).not.toHaveTextContent('Some server-provided text');
  });

  it('reports a server error during the exchange with a fixed message', async () => {
    vi.mocked(api.post).mockRejectedValue({
      isAxiosError: true,
      response: { status: 500, data: { error: 'stack trace' } },
    });
    setUrl('/auth/callback#code=abc');

    renderCallback();

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('server error occurred during sign-in');
    expect(alert).not.toHaveTextContent('stack trace');
  });
});
