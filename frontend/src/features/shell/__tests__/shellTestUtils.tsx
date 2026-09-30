import { useEffect, type ReactNode } from 'react';
import { render } from '@testing-library/react';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import { vi } from 'vitest';
import { AuthContext } from '../../../contexts/AuthContext';
import { NotificationProvider, useNotifications } from '../../../contexts/NotificationContext';
import { ThemeModeProvider } from '../../../theme/ThemeProvider';
import { createResponsiveMatchMediaMock } from '../../../test/pwa-test-utils';
import type { NotificationType } from '../../../types/notification';
import { AppShell } from '../AppShell';

export class MemoryStorage implements Storage {
  private data = new Map<string, string>();
  get length() {
    return this.data.size;
  }
  clear() {
    this.data.clear();
  }
  getItem(key: string) {
    return this.data.has(key) ? (this.data.get(key) as string) : null;
  }
  key(i: number) {
    return Array.from(this.data.keys())[i] ?? null;
  }
  removeItem(key: string) {
    this.data.delete(key);
  }
  setItem(key: string, value: string) {
    this.data.set(key, String(value));
  }
}

export function installStorage(storage: Storage = new MemoryStorage()): Storage {
  Object.defineProperty(window, 'localStorage', { value: storage, configurable: true, writable: true });
  return storage;
}

/** Media queries that should match, e.g. `{ '(max-width: 719px)': true }`. */
export function setMedia(queries: Record<string, boolean> = {}) {
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    configurable: true,
    value: createResponsiveMatchMediaMock(queries),
  });
}

export const NARROW = { '(max-width: 719px)': true };
export const STANDALONE = { '(display-mode: standalone)': true };

/** Renders the current location so tests can assert navigation by role. */
export function LocationProbe() {
  const { pathname, search } = useLocation();
  return (
    <output aria-label="location">
      {pathname}
      {search}
    </output>
  );
}

export interface SeedNotification {
  type: NotificationType;
  title: string;
  message: string;
}

function Seeder({ items }: { items: SeedNotification[] }) {
  const { addNotification } = useNotifications();
  useEffect(() => {
    items.forEach((n) => addNotification(n));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  return null;
}

export const testUser = { id: '1', username: 'ada', email: 'ada@example.com', role: 'admin' as const };

export interface RenderShellOptions {
  path?: string;
  logout?: () => void;
  notifications?: SeedNotification[];
  page?: ReactNode;
}

export function renderShell({ path = '/board', logout = vi.fn(), notifications = [], page }: RenderShellOptions = {}) {
  const auth = { user: testUser, loading: false, login: vi.fn(), register: vi.fn(), logout };
  const tree = (
    <ThemeModeProvider>
      <MemoryRouter initialEntries={[path]}>
        <AuthContext.Provider value={auth}>
          <NotificationProvider>
            <Seeder items={notifications} />
            <Routes>
              <Route path="/login" element={<LocationProbe />} />
              <Route element={<AppShell />}>
                <Route
                  path="*"
                  element={
                    <>
                      {page}
                      <LocationProbe />
                    </>
                  }
                />
              </Route>
            </Routes>
          </NotificationProvider>
        </AuthContext.Provider>
      </MemoryRouter>
    </ThemeModeProvider>
  );
  const utils = render(tree);
  return { ...utils, auth, rerenderShell: () => utils.rerender(tree) };
}
