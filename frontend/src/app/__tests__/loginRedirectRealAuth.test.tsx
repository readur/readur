import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, type InitialEntry } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { api } from '../../services/api';
import { AuthProvider } from '../../contexts/AuthContext';
import { FeatureFlagsContext } from '../../contexts/FeatureFlagsContext';
import { ThemeModeProvider } from '../../theme/ThemeProvider';
import { createResponsiveMatchMediaMock } from '../../test/pwa-test-utils';
import { AppRoutes } from '../routes';

// The real AuthProvider talks to the named `api` client. Its login resolves on a later task,
// like a real network response, so React commits `user` on its own schedule.
vi.mock('../../services/api', async () => {
  const errors = await import('../../services/errors');
  const client = { get: vi.fn(), post: vi.fn(), defaults: { headers: { common: {} } } };
  return {
    default: client,
    api: client,
    ErrorHelper: errors.ErrorHelper,
    ErrorCodes: errors.ErrorCodes,
    documentService: { enhancedSearch: vi.fn() },
  };
});

vi.mock('../../features/board', async () => ({ default: (await import('./routeProbe')).probe('board') }));
vi.mock('../../features/library', async () => ({ default: (await import('./routeProbe')).probe('library') }));
vi.mock('../../features/library/SearchRoute', async () => ({ default: (await import('./routeProbe')).probe('search') }));
vi.mock('../../features/document', async () => ({ default: (await import('./routeProbe')).probe('document') }));
vi.mock('../../features/document/SharedRoute', async () => ({ default: (await import('./routeProbe')).probe('shared') }));
vi.mock('../../features/intake', async () => ({ default: (await import('./routeProbe')).probe('intake') }));
vi.mock('../../features/settings', async () => ({ default: (await import('./routeProbe')).probe('settings') }));
vi.mock('../../features/auth/CallbackRoute', async () => ({ default: (await import('./routeProbe')).probe('callback') }));

const USER = { id: '1', username: 'ada', email: 'a@b.c', role: 'user' };
const later = <T,>(value: T) => new Promise<T>((resolve) => setTimeout(() => resolve(value), 0));

beforeEach(() => {
  localStorage.clear();
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    configurable: true,
    value: createResponsiveMatchMediaMock({}),
  });
  vi.mocked(api.get).mockReturnValue(new Promise(() => {}) as never);
  vi.mocked(api.post).mockImplementation(() => later({ data: { token: 't', user: USER } }) as never);
});

const renderAt = (entry: InitialEntry) =>
  render(
    <ThemeModeProvider>
      <FeatureFlagsContext.Provider
        value={{ flags: { allowLocalAuth: true, allowRegistration: false, oidcEnabled: false, enablePerUserWatch: false }, loading: false, error: null }}
      >
        <MemoryRouter initialEntries={[entry]}>
          <AuthProvider>
            <AppRoutes />
          </AuthProvider>
        </MemoryRouter>
      </FeatureFlagsContext.Provider>
    </ThemeModeProvider>,
  );

const signIn = async () => {
  const user = userEvent.setup();
  await user.type(await screen.findByRole('textbox', { name: 'Username' }), 'ada');
  await user.type(screen.getByLabelText('Password'), 'secret');
  await user.click(screen.getByRole('button', { name: 'Sign in' }));
};

const signedIn = () => {
  localStorage.setItem('token', 't');
  vi.mocked(api.get).mockResolvedValue({ data: USER } as never);
};

describe('sign-in with the real AuthProvider', () => {
  it('returns to the requested route, not /board', async () => {
    renderAt('/documents?q=x#top');
    expect(await screen.findByRole('heading', { level: 1, name: 'Sign in' })).toBeInTheDocument();
    await signIn();
    await waitFor(() => expect(screen.getByRole('heading', { name: 'library entry' })).toBeInTheDocument());
    expect(screen.getByRole('status', { name: 'location' })).toHaveTextContent('/documents?q=x#top');
    // Nothing redirects afterwards.
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.getByRole('status', { name: 'location' })).toHaveTextContent('/documents?q=x#top');
  });

  it('goes to /board when /login is opened directly', async () => {
    renderAt('/login');
    await signIn();
    await waitFor(() => expect(screen.getByRole('heading', { name: 'board entry' })).toBeInTheDocument());
  });

  it('sends a signed-in visitor on /login to the requested route', async () => {
    signedIn();
    renderAt({ pathname: '/login', state: { from: { pathname: '/intake', search: '?section=watch' } } });
    expect(await screen.findByRole('heading', { name: 'intake entry' })).toBeInTheDocument();
    expect(screen.getByRole('status', { name: 'location' })).toHaveTextContent('/intake?section=watch');
  });

  it('ignores an off-site requested route for a signed-in visitor', async () => {
    signedIn();
    renderAt({ pathname: '/login', state: { from: '//evil.example' } });
    expect(await screen.findByRole('heading', { name: 'board entry' })).toBeInTheDocument();
  });
});
