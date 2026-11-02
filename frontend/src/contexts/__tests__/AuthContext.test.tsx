import React from 'react';
import { describe, test, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, act } from '@testing-library/react';
import { AuthProvider, useAuth } from '../AuthContext';
import { api } from '../../services/api';

let auth: ReturnType<typeof useAuth>;

const Probe: React.FC = () => {
  auth = useAuth();
  return (
    <div>
      <span data-testid="loading">{String(auth.loading)}</span>
      <span data-testid="user">{auth.user?.username ?? 'none'}</span>
      <span data-testid="notice">{auth.sessionNotice ?? 'none'}</span>
    </div>
  );
};

const renderProvider = () =>
  render(
    <AuthProvider>
      <Probe />
    </AuthProvider>
  );

describe('AuthProvider', () => {
  beforeEach(() => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.spyOn(console, 'warn').mockImplementation(() => {});
  });

  test('keeps the stored token when loading the user fails for a non-auth reason', async () => {
    vi.mocked(localStorage.getItem).mockReturnValue('stored-token');
    vi.spyOn(api, 'get').mockRejectedValue({ isAxiosError: true, message: 'Network Error' });

    renderProvider();

    await waitFor(() => expect(screen.getByTestId('loading')).toHaveTextContent('false'));
    expect(localStorage.removeItem).not.toHaveBeenCalledWith('token');
    expect(screen.getByTestId('user')).toHaveTextContent('none');
  });

  test('keeps the stored token on a server error', async () => {
    vi.mocked(localStorage.getItem).mockReturnValue('stored-token');
    vi.spyOn(api, 'get').mockRejectedValue({ isAxiosError: true, response: { status: 503 } });

    renderProvider();

    await waitFor(() => expect(screen.getByTestId('loading')).toHaveTextContent('false'));
    expect(localStorage.removeItem).not.toHaveBeenCalledWith('token');
  });

  test('clears the session when the stored token is rejected', async () => {
    vi.mocked(localStorage.getItem).mockReturnValue('stored-token');
    vi.spyOn(api, 'get').mockRejectedValue({ isAxiosError: true, response: { status: 401 } });

    renderProvider();

    await waitFor(() => expect(localStorage.removeItem).toHaveBeenCalledWith('token'));
    expect(screen.getByTestId('loading')).toHaveTextContent('false');
  });

  test('signs out locally and reports when the server logout fails', async () => {
    vi.mocked(localStorage.getItem).mockReturnValue('stored-token');
    vi.spyOn(api, 'get').mockResolvedValue({
      data: { id: '1', username: 'alice', email: 'a@example.com', role: 'user', is_active: true },
    });
    const post = vi.spyOn(api, 'post').mockRejectedValue({ isAxiosError: true, message: 'Network Error' });

    renderProvider();
    await waitFor(() => expect(screen.getByTestId('user')).toHaveTextContent('alice'));

    await act(async () => {
      await auth.logout();
    });

    expect(post).toHaveBeenCalledWith('/auth/logout', undefined, {
      headers: { Authorization: 'Bearer stored-token' },
    });
    expect(localStorage.removeItem).toHaveBeenCalledWith('token');
    expect(screen.getByTestId('user')).toHaveTextContent('none');
    expect(screen.getByTestId('notice')).toHaveTextContent('logoutIncomplete');
    expect(console.warn).toHaveBeenCalled();

    act(() => auth.dismissSessionNotice());
    expect(screen.getByTestId('notice')).toHaveTextContent('none');
  });

  test('does not report anything when the server logout succeeds', async () => {
    vi.mocked(localStorage.getItem).mockReturnValue('stored-token');
    vi.spyOn(api, 'get').mockResolvedValue({
      data: { id: '1', username: 'alice', email: 'a@example.com', role: 'user', is_active: true },
    });
    vi.spyOn(api, 'post').mockResolvedValue({ status: 204 });

    renderProvider();
    await waitFor(() => expect(screen.getByTestId('user')).toHaveTextContent('alice'));

    await act(async () => {
      await auth.logout();
    });

    expect(screen.getByTestId('user')).toHaveTextContent('none');
    expect(screen.getByTestId('notice')).toHaveTextContent('none');
  });
});
