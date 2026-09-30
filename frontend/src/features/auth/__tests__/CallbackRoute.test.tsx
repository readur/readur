import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import CallbackRoute from '../CallbackRoute';
import { api } from '../../../services/api';
import { renderAuth } from './authTestUtils';

vi.mock('../../../services/api', () => ({ api: { defaults: { headers: { common: {} as Record<string, string> } } } }));

const originalLocation = window.location;
const originalStorage = window.localStorage;
const hrefSetter = vi.fn();
const store = new Map<string, string>();
const headers = () => api.defaults.headers.common as Record<string, string>;

beforeEach(() => {
  hrefSetter.mockReset();
  store.clear();
  delete headers().Authorization;
  Object.defineProperty(window, 'localStorage', {
    configurable: true,
    value: {
      getItem: (k: string) => store.get(k) ?? null,
      setItem: (k: string, v: string) => void store.set(k, v),
      removeItem: (k: string) => void store.delete(k),
    },
  });
  Object.defineProperty(window, 'location', {
    configurable: true,
    value: {
      ...originalLocation,
      set href(v: string) {
        hrefSetter(v);
      },
      get href() {
        return 'http://localhost/auth/callback';
      },
    },
  });
});

afterEach(() => {
  Object.defineProperty(window, 'location', { configurable: true, value: originalLocation });
  Object.defineProperty(window, 'localStorage', { configurable: true, value: originalStorage });
});

const renderCallback = (search = '') =>
  renderAuth({ path: '/auth/callback', element: <CallbackRoute />, entry: `/auth/callback${search}` });

describe('CallbackRoute', () => {
  it('module exports a component', () => {
    expect(typeof CallbackRoute).toBe('function');
  });

  it('stores the token, sets the auth header and goes to /board', async () => {
    renderCallback('?token=abc123');
    expect(screen.getByRole('heading', { level: 1, name: 'Signing you in…' })).toBeInTheDocument();
    await waitFor(() => expect(hrefSetter).toHaveBeenCalledWith('/board'));
    expect(store.get('token')).toBe('abc123');
    expect(headers().Authorization).toBe('Bearer abc123');
  });

  it('shows the provider error with a way back to sign in', async () => {
    const user = userEvent.setup();
    renderCallback('?error=access_denied');
    expect(await screen.findByRole('alert')).toHaveTextContent('Access was denied');
    expect(hrefSetter).not.toHaveBeenCalled();
    await user.click(screen.getByRole('link', { name: 'Back to sign in' }));
    expect(await screen.findByRole('status', { name: 'location' })).toHaveTextContent('/login');
  });

  it('maps other standard provider codes to fixed messages', async () => {
    renderCallback('?error=server_error');
    expect(await screen.findByRole('alert')).toHaveTextContent('The SSO provider had a problem');
  });

  it('shows an unknown plain code only as a code, never as free text', async () => {
    renderCallback('?error=weird_thing');
    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('SSO sign-in failed (code: weird_thing)');
  });

  it('never renders attacker-chosen text from the error param', async () => {
    renderCallback('?error=' + encodeURIComponent('Your account is locked, call 555-0100'));
    const alert = await screen.findByRole('alert');
    expect(alert).not.toHaveTextContent('555-0100');
    expect(alert).toHaveTextContent('SSO sign-in failed. Try again or ask your administrator.');
  });

  it('shows an error when no token arrives', async () => {
    renderCallback();
    expect(await screen.findByRole('alert')).toHaveTextContent('did not send a sign-in token');
    expect(screen.getByRole('link', { name: 'Back to sign in' })).toHaveAttribute('href', '/login');
  });

  it('shows an error when the token cannot be stored', async () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
    Object.defineProperty(window, 'localStorage', {
      configurable: true,
      value: {
        setItem: () => {
          throw new Error('quota');
        },
      },
    });
    renderCallback('?token=abc');
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not finish signing you in');
    expect(hrefSetter).not.toHaveBeenCalled();
    spy.mockRestore();
  });
});
