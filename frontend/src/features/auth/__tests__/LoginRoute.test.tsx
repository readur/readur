import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { AxiosError } from 'axios';
import LoginRoute from '../LoginRoute';
import { renderAuth, type RenderAuthOptions } from './authTestUtils';

vi.mock('../../../services/api', async () => {
  const errors = await import('../../../services/errors');
  return { ErrorHelper: errors.ErrorHelper, ErrorCodes: errors.ErrorCodes, api: { defaults: { headers: { common: {} } } } };
});

const originalLocation = window.location;
const hrefSetter = vi.fn();

beforeEach(() => {
  hrefSetter.mockReset();
  Object.defineProperty(window, 'location', {
    configurable: true,
    value: {
      ...originalLocation,
      set href(v: string) {
        hrefSetter(v);
      },
      get href() {
        return 'http://localhost/login';
      },
    },
  });
});

afterEach(() => {
  Object.defineProperty(window, 'location', { configurable: true, value: originalLocation });
});

const renderLogin = (opts: Partial<RenderAuthOptions> = {}) =>
  renderAuth({ path: '/login', element: <LoginRoute />, ...opts });

const fill = async (user: ReturnType<typeof userEvent.setup>, name = 'ada', pw = 'secret') => {
  await user.type(screen.getByRole('textbox', { name: 'Username' }), name);
  await user.type(screen.getByLabelText('Password'), pw);
};

const httpError = (status: number) =>
  new AxiosError('failed', 'ERR_BAD_REQUEST', undefined, undefined, { status, data: {}, statusText: '', headers: {}, config: {} as never });

const where = () => screen.getByRole('status', { name: 'location' });

describe('LoginRoute layout', () => {
  it('renders the wordmark, one h1 and the version line', () => {
    renderLogin();
    expect(screen.getByRole('heading', { level: 1, name: 'Sign in' })).toBeInTheDocument();
    expect(screen.getByText('Readur')).toBeInTheDocument();
    expect(screen.getByText(/^v\d+\.\d+\.\d+/)).toBeInTheDocument();
  });

  it('has a main landmark and no sign-up link', () => {
    renderLogin();
    expect(screen.getByRole('main')).toBeInTheDocument();
    expect(screen.queryByText(/sign up/i)).not.toBeInTheDocument();
  });

  it('focuses the username field on load', () => {
    renderLogin();
    expect(screen.getByRole('textbox', { name: 'Username' })).toHaveFocus();
  });

  it('shows a loading state while flags load', () => {
    renderLogin({ loading: true });
    expect(screen.getByRole('status', { name: 'Loading sign-in options' })).toBeInTheDocument();
    expect(screen.queryByLabelText('Password')).not.toBeInTheDocument();
  });
});

describe('LoginRoute auth flags', () => {
  it('shows only the password form when local auth is on and SSO is off', () => {
    renderLogin({ flags: { allowLocalAuth: true, oidcEnabled: false } });
    expect(screen.getByLabelText('Password')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Sign in' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Sign in with SSO' })).not.toBeInTheDocument();
  });

  it('shows only the SSO button when local auth is off', () => {
    renderLogin({ flags: { allowLocalAuth: false, oidcEnabled: true } });
    expect(screen.queryByLabelText('Password')).not.toBeInTheDocument();
    expect(screen.queryByRole('textbox', { name: 'Username' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Sign in with SSO' })).toBeInTheDocument();
  });

  it('shows both when both are enabled', () => {
    renderLogin({ flags: { allowLocalAuth: true, oidcEnabled: true } });
    expect(screen.getByLabelText('Password')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Sign in with SSO' })).toBeInTheDocument();
  });

  it('explains when no method is enabled', () => {
    renderLogin({ flags: { allowLocalAuth: false, oidcEnabled: false } });
    expect(screen.getByText(/No sign-in method is enabled/)).toBeInTheDocument();
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });

  it('gives the fields autocomplete attributes', () => {
    renderLogin();
    expect(screen.getByRole('textbox', { name: 'Username' })).toHaveAttribute('autocomplete', 'username');
    expect(screen.getByLabelText('Password')).toHaveAttribute('autocomplete', 'current-password');
  });
});

describe('LoginRoute password sign-in', () => {
  it('calls login with the credentials and goes to /board', async () => {
    const login = vi.fn().mockResolvedValue(undefined);
    const user = userEvent.setup();
    renderLogin({ login });
    await fill(user);
    await user.click(screen.getByRole('button', { name: 'Sign in' }));
    expect(login).toHaveBeenCalledWith('ada', 'secret');
    await waitFor(() => expect(where()).toHaveTextContent('/board'));
  });

  it('returns to the originally requested route after sign-in', async () => {
    const user = userEvent.setup();
    renderLogin({
      login: vi.fn().mockResolvedValue(undefined),
      entry: {
        pathname: '/login',
        state: { from: { pathname: '/documents/42', search: '?tab=text', hash: '' } },
      },
    });
    await fill(user);
    await user.click(screen.getByRole('button', { name: 'Sign in' }));
    await waitFor(() => expect(where()).toHaveTextContent('/documents/42?tab=text'));
  });

  it('ignores an off-site "from" and goes to /board', async () => {
    const user = userEvent.setup();
    renderLogin({
      login: vi.fn().mockResolvedValue(undefined),
      entry: { pathname: '/login', state: { from: '//evil.example' } },
    });
    await fill(user);
    await user.click(screen.getByRole('button', { name: 'Sign in' }));
    await waitFor(() => expect(where()).toHaveTextContent('/board'));
  });

  it.each(['/\\evil.example', '/ok\nhttp://evil.example'])('ignores an unsafe "from" %j', async (from) => {
    const user = userEvent.setup();
    renderLogin({
      login: vi.fn().mockResolvedValue(undefined),
      entry: { pathname: '/login', state: { from } },
    });
    await fill(user);
    await user.click(screen.getByRole('button', { name: 'Sign in' }));
    await waitFor(() => expect(where()).toHaveTextContent('/board'));
  });

  it('submits with Enter from the password field', async () => {
    const login = vi.fn().mockResolvedValue(undefined);
    const user = userEvent.setup();
    renderLogin({ login });
    await user.type(screen.getByRole('textbox', { name: 'Username' }), 'ada');
    await user.type(screen.getByLabelText('Password'), 'secret{Enter}');
    expect(login).toHaveBeenCalledWith('ada', 'secret');
  });

  it('shows a pending state while signing in', async () => {
    let resolve!: () => void;
    const login = vi.fn(() => new Promise<void>((r) => (resolve = r)));
    const user = userEvent.setup();
    renderLogin({ login });
    await fill(user);
    await user.click(screen.getByRole('button', { name: 'Sign in' }));
    const button = await screen.findByRole('button', { name: 'Signing in…' });
    expect(button).toHaveAttribute('aria-busy', 'true');
    resolve();
    await waitFor(() => expect(where()).toBeInTheDocument());
  });

  it('asks for missing fields without calling login', async () => {
    const login = vi.fn();
    const user = userEvent.setup();
    renderLogin({ login });
    await user.click(screen.getByRole('button', { name: 'Sign in' }));
    expect(login).not.toHaveBeenCalled();
    expect(await screen.findByText('Enter your username')).toBeInTheDocument();
    expect(screen.getByText('Enter your password')).toBeInTheDocument();
  });

  it('toggles password visibility', async () => {
    const user = userEvent.setup();
    renderLogin();
    const input = screen.getByLabelText('Password');
    expect(input).toHaveAttribute('type', 'password');
    await user.click(screen.getByRole('button', { name: 'Show password' }));
    expect(input).toHaveAttribute('type', 'text');
    await user.click(screen.getByRole('button', { name: 'Hide password' }));
    expect(input).toHaveAttribute('type', 'password');
  });
});

describe('LoginRoute errors', () => {
  const failWith = (err: unknown) => vi.fn().mockRejectedValue(err);

  it('explains wrong credentials in an alert', async () => {
    const user = userEvent.setup();
    renderLogin({ login: failWith(httpError(401)) });
    await fill(user);
    await user.click(screen.getByRole('button', { name: 'Sign in' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Wrong username or password');
    expect(screen.getByRole('button', { name: 'Sign in' })).not.toHaveAttribute('aria-busy');
  });

  it('shows the generic message, not server text, for an unmapped status', async () => {
    const user = userEvent.setup();
    renderLogin({
      login: failWith(
        new AxiosError('failed', 'ERR_BAD_REQUEST', undefined, undefined, {
          status: 429,
          data: { message: 'SERVER-SECRET-DETAIL' },
          statusText: '',
          headers: {},
          config: {} as never,
        }),
      ),
    });
    await fill(user);
    await user.click(screen.getByRole('button', { name: 'Sign in' }));
    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('Sign-in failed. Try again.');
    expect(alert).not.toHaveTextContent('SERVER-SECRET-DETAIL');
  });

  it('explains an unreachable server in an alert', async () => {
    const user = userEvent.setup();
    renderLogin({ login: failWith(new AxiosError('Network Error', 'ERR_NETWORK')) });
    await fill(user);
    await user.click(screen.getByRole('button', { name: 'Sign in' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Cannot reach the server');
  });

  it('clears the alert on the next attempt', async () => {
    const login = vi
      .fn()
      .mockRejectedValueOnce(httpError(401))
      .mockResolvedValueOnce(undefined);
    const user = userEvent.setup();
    renderLogin({ login });
    await fill(user);
    await user.click(screen.getByRole('button', { name: 'Sign in' }));
    expect(await screen.findByRole('alert')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Sign in' }));
    await waitFor(() => expect(screen.queryByRole('alert')).not.toBeInTheDocument());
  });
});

describe('LoginRoute SSO', () => {
  it('starts the OIDC flow', async () => {
    const user = userEvent.setup();
    renderLogin({ flags: { oidcEnabled: true } });
    await user.click(screen.getByRole('button', { name: 'Sign in with SSO' }));
    expect(hrefSetter).toHaveBeenCalledWith('/api/auth/oidc/login');
    expect(await screen.findByRole('button', { name: 'Redirecting…' })).toBeInTheDocument();
  });

  it('shows an alert when the SSO redirect cannot start', async () => {
    Object.defineProperty(window, 'location', {
      configurable: true,
      value: {
        set href(_v: string) {
          throw new Error('blocked');
        },
        get href() {
          return '';
        },
      },
    });
    const user = userEvent.setup();
    renderLogin({ flags: { oidcEnabled: true } });
    await user.click(screen.getByRole('button', { name: 'Sign in with SSO' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not start SSO sign-in');
    expect(screen.getByRole('button', { name: 'Sign in with SSO' })).toBeEnabled();
  });
});
