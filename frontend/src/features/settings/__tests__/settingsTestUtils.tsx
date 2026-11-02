import type { ReactNode } from 'react';
import { render } from '@testing-library/react';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import { vi } from 'vitest';
import { AuthContext } from '../../../contexts/AuthContext';
import { FeatureFlagsContext } from '../../../contexts/FeatureFlagsContext';
import { ThemeModeProvider } from '../../../theme/ThemeProvider';
import { ToastProvider } from '../../../ui';
import { createResponsiveMatchMediaMock } from '../../../test/pwa-test-utils';

export { apiMock } from './apiMock';

export const adminUser = { id: 'u-admin', username: 'ada', email: 'ada@example.com', role: 'admin' as const };
export const plainUser = { id: 'u-user', username: 'bob', email: 'bob@example.com', role: 'user' as const };

export function LocationProbe() {
  const { pathname, search, hash } = useLocation();
  return (
    <output aria-label="location">
      {pathname}
      {search}
      {hash}
    </output>
  );
}

export interface RenderOptions {
  path?: string;
  user?: { id: string; username: string; email: string; role: string } | null;
  perUserWatch?: boolean;
  /** Route pattern; defaults to the settings route. */
  route?: string;
  /**
   * Media queries that match. Defaults to reduced motion so folds open and close synchronously.
   * Pass `null` to keep a `matchMedia` the test installed itself.
   */
  media?: Record<string, boolean> | null;
  /** Extra auth context fields, e.g. `changePassword`. */
  auth?: Record<string, unknown>;
}

export const REDUCED_MOTION = { '(prefers-reduced-motion: reduce)': true };

export function setMedia(queries: Record<string, boolean>) {
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    configurable: true,
    value: createResponsiveMatchMediaMock(queries),
  });
}

/** Renders `ui` at `path` inside the providers a settings screen needs. */
export function renderSettings(
  ui: ReactNode,
  { path = '/settings', user = adminUser, perUserWatch = false, route = '/settings/:section?', media = REDUCED_MOTION, auth: authExtra }: RenderOptions = {},
) {
  if (media) setMedia(media);
  const auth = {
    user: user as never,
    loading: false,
    login: vi.fn(),
    register: vi.fn(),
    logout: vi.fn(),
    changePassword: vi.fn(),
    completeLogin: vi.fn(),
    sessionNotice: null,
    dismissSessionNotice: vi.fn(),
    ...authExtra,
  };
  const flags = {
    flags: { allowLocalAuth: true, allowRegistration: false, oidcEnabled: false, enablePerUserWatch: perUserWatch },
    loading: false,
    error: null,
  };
  return render(
    <ThemeModeProvider>
      <ToastProvider>
        <MemoryRouter initialEntries={[path]}>
          <AuthContext.Provider value={auth}>
            <FeatureFlagsContext.Provider value={flags}>
              <Routes>
                <Route
                  path={route}
                  element={
                    <>
                      {ui}
                      <LocationProbe />
                    </>
                  }
                />
                <Route path="*" element={<LocationProbe />} />
              </Routes>
            </FeatureFlagsContext.Provider>
          </AuthContext.Provider>
        </MemoryRouter>
      </ToastProvider>
    </ThemeModeProvider>,
  );
}

/** Replaces `window.localStorage` with a fresh in-memory store and returns it. */
export function installStorage(): Storage {
  const data = new Map<string, string>();
  const storage: Storage = {
    get length() {
      return data.size;
    },
    clear: () => data.clear(),
    getItem: (k) => (data.has(k) ? (data.get(k) as string) : null),
    key: (i) => Array.from(data.keys())[i] ?? null,
    removeItem: (k) => void data.delete(k),
    setItem: (k, v) => void data.set(k, String(v)),
  };
  Object.defineProperty(window, 'localStorage', { value: storage, configurable: true, writable: true });
  return storage;
}

/** A 2xx axios-like response. */
export const ok = <T,>(data: T, status = 200) => ({ status, data });

/** An axios-like error with a status and optional `{ error, code }` body. */
export function httpError(status: number, body: Record<string, unknown> = {}, message = `Request failed with status code ${status}`) {
  return Object.assign(new Error(message), { isAxiosError: true, response: { status, data: body } });
}
