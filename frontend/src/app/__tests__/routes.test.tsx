import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import api from '../../services/api';
import { AuthContext } from '../../contexts/AuthContext';
import { ThemeModeProvider } from '../../theme/ThemeProvider';
import { createResponsiveMatchMediaMock } from '../../test/pwa-test-utils';
import { AppRoutes } from '../routes';
import { LEGACY_ROUTES, mergeSearch } from '../legacyRoutes';

vi.mock('../../services/api', () => ({
  default: { get: vi.fn() },
  documentService: { enhancedSearch: vi.fn() },
}));

vi.mock('../../features/board', async () => ({ default: (await import('./routeProbe')).probe('board') }));
vi.mock('../../features/library', async () => ({ default: (await import('./routeProbe')).probe('library') }));
vi.mock('../../features/library/SearchRoute', async () => ({ default: (await import('./routeProbe')).probe('search') }));
vi.mock('../../features/document', async () => ({ default: (await import('./routeProbe')).probe('document') }));
vi.mock('../../features/document/SharedRoute', async () => ({ default: (await import('./routeProbe')).probe('shared') }));
vi.mock('../../features/intake', async () => ({ default: (await import('./routeProbe')).probe('intake') }));
vi.mock('../../features/settings', async () => ({ default: (await import('./routeProbe')).probe('settings') }));
vi.mock('../../features/auth/LoginRoute', async () => ({ default: (await import('./routeProbe')).probe('login') }));
vi.mock('../../features/auth/CallbackRoute', async () => ({ default: (await import('./routeProbe')).probe('callback') }));

const user = { id: '1', username: 'ada', email: 'ada@example.com', role: 'admin' as const };

function renderAt(path: string, signedIn = true) {
  const auth = { user: signedIn ? user : null, loading: false, login: vi.fn(), register: vi.fn(), logout: vi.fn() };
  return render(
    <ThemeModeProvider>
      <MemoryRouter initialEntries={[path]}>
        <AuthContext.Provider value={auth}>
          <AppRoutes />
        </AuthContext.Provider>
      </MemoryRouter>
    </ThemeModeProvider>,
  );
}

const entry = (name: string) => screen.findByRole('heading', { name: `${name} entry` });
const location = () => screen.getByRole('status', { name: 'location' });

beforeEach(() => {
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    configurable: true,
    value: createResponsiveMatchMediaMock({}),
  });
  vi.mocked(api.get).mockReturnValue(new Promise(() => {}) as never);
});

describe('legacy redirects', () => {
  it('covers every old URL', () => {
    expect(LEGACY_ROUTES.map((r) => r.from).sort()).toEqual(
      ['/', '/dashboard', '/upload', '/sources', '/watch', '/documents/management', '/ignored-files', '/labels', '/debug', '/profile'].sort(),
    );
  });

  const expectedEntry: Record<string, string> = {
    '/board': 'board',
    '/intake': 'intake',
    '/settings': 'settings',
    '/settings/labels': 'settings',
    '/settings/debug': 'settings',
  };

  it.each(LEGACY_ROUTES.map((r) => [r.from, r.to]))('%s redirects to %s', async (from, to) => {
    renderAt(from);
    const target = mergeSearch(to, '');
    expect(await entry(expectedEntry[target.pathname])).toBeInTheDocument();
    expect(location().textContent).toBe(`${target.pathname}${target.search}`);
  });

  it.each(LEGACY_ROUTES.map((r) => [r.from, r.to]))('%s keeps other query parameters', async (from, to) => {
    renderAt(`${from}?page=2&q=tax#top`);
    const target = mergeSearch(to, '?page=2&q=tax');
    await entry(expectedEntry[target.pathname]);
    const params = new URLSearchParams(location().textContent?.split('?')[1]?.split('#')[0]);
    expect(params.get('page')).toBe('2');
    expect(params.get('q')).toBe('tax');
    expect(location().textContent).toBe(`${target.pathname}${target.search}#top`);
  });

  it('lets the target section win over an incoming one', () => {
    expect(mergeSearch('/intake?section=upload', '?section=old&x=1')).toEqual({
      pathname: '/intake',
      search: '?section=upload&x=1',
    });
  });

  it('sends /documents/management to Needs attention, not to a document', async () => {
    renderAt('/documents/management');
    expect(await entry('intake')).toBeInTheDocument();
    expect(location().textContent).toBe('/intake?section=attention');
    expect(screen.queryByRole('heading', { name: 'document entry' })).not.toBeInTheDocument();
  });
});

describe('destinations', () => {
  it.each([
    ['/board', 'board'],
    ['/documents', 'library'],
    ['/search?q=tax', 'search'],
    ['/intake?section=watch', 'intake'],
    ['/settings', 'settings'],
  ])('%s loads the %s entry inside the shell', async (path, name) => {
    renderAt(path);
    expect(await entry(name)).toBeInTheDocument();
    expect(screen.getByRole('banner')).toBeInTheDocument();
    expect(screen.getByRole('main')).toContainElement(screen.getByRole('heading', { name: `${name} entry` }));
  });

  it('passes the document id', async () => {
    renderAt('/documents/abc-123');
    expect(await entry('document')).toBeInTheDocument();
    expect(screen.getByRole('status', { name: 'params' })).toHaveTextContent('"id":"abc-123"');
  });

  it('passes the settings section', async () => {
    renderAt('/settings/labels');
    expect(await entry('settings')).toBeInTheDocument();
    expect(screen.getByRole('status', { name: 'params' })).toHaveTextContent('"section":"labels"');
  });

  it('sends unknown paths to the board', async () => {
    renderAt('/nope');
    expect(await entry('board')).toBeInTheDocument();
    expect(location().textContent).toBe('/board');
  });
});

describe('auth gating', () => {
  it('sends signed-out users to /login', async () => {
    renderAt('/documents', false);
    expect(await entry('login')).toBeInTheDocument();
    expect(location().textContent).toBe('/login');
    expect(screen.queryByRole('banner')).not.toBeInTheDocument();
  });

  it('sends signed-in users away from /login', async () => {
    renderAt('/login');
    expect(await entry('board')).toBeInTheDocument();
  });

  it.each([
    ['/shared/tok-1', 'shared'],
    ['/auth/callback?code=x', 'callback'],
  ])('%s is public and renders without the shell', async (path, name) => {
    renderAt(path, false);
    expect(await entry(name)).toBeInTheDocument();
    expect(screen.queryByRole('banner')).not.toBeInTheDocument();
  });
});
