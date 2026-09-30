import { useState, type ReactNode } from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import api from '../../services/api';
import { AuthContext } from '../../contexts/AuthContext';
import { FeatureFlagsContext } from '../../contexts/FeatureFlagsContext';
import { ThemeModeProvider } from '../../theme/ThemeProvider';
import { createResponsiveMatchMediaMock } from '../../test/pwa-test-utils';
import { AppRoutes } from '../routes';

vi.mock('../../services/api', async () => {
  const errors = await import('../../services/errors');
  return {
    default: { get: vi.fn() },
    api: { defaults: { headers: { common: {} } } },
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

/** Auth provider whose login really flips the user, like the app's. */
function StatefulAuth({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<null | { id: string; username: string; email: string; role: 'user' }>(null);
  const login = async (username: string) => setUser({ id: '1', username, email: 'a@b.c', role: 'user' });
  return (
    <AuthContext.Provider value={{ user, loading: false, login, register: vi.fn(), logout: () => setUser(null) }}>
      {children}
    </AuthContext.Provider>
  );
}

beforeEach(() => {
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    configurable: true,
    value: createResponsiveMatchMediaMock({}),
  });
  vi.mocked(api.get).mockReturnValue(new Promise(() => {}) as never);
});

describe('return to the requested route after sign-in', () => {
  const renderAt = (path: string) =>
    render(
      <ThemeModeProvider>
        <FeatureFlagsContext.Provider
          value={{ flags: { allowLocalAuth: true, oidcEnabled: false, enablePerUserWatch: false }, loading: false, error: null }}
        >
          <MemoryRouter initialEntries={[path]}>
            <StatefulAuth>
              <AppRoutes />
            </StatefulAuth>
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

  it('lands on /documents?q=x after an unauthenticated visit and sign-in', async () => {
    renderAt('/documents?q=x');
    expect(await screen.findByRole('heading', { level: 1, name: 'Sign in' })).toBeInTheDocument();
    await signIn();
    await waitFor(() => expect(screen.getByRole('heading', { name: 'library entry' })).toBeInTheDocument());
    expect(screen.getByRole('status', { name: 'location' })).toHaveTextContent('/documents?q=x');
  });

  it('goes to /board when /login is visited directly', async () => {
    renderAt('/login');
    await signIn();
    await waitFor(() => expect(screen.getByRole('heading', { name: 'board entry' })).toBeInTheDocument());
  });
});
