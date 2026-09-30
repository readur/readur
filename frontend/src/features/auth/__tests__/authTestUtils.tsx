import type { ReactNode } from 'react';
import { render } from '@testing-library/react';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import { AuthContext } from '../../../contexts/AuthContext';
import { FeatureFlagsContext } from '../../../contexts/FeatureFlagsContext';

export interface Flags {
  allowLocalAuth: boolean;
  oidcEnabled: boolean;
}

function LocationProbe() {
  const { pathname, search } = useLocation();
  return (
    <output aria-label="location">
      {pathname}
      {search}
    </output>
  );
}

export interface RenderAuthOptions {
  flags?: Partial<Flags>;
  loading?: boolean;
  login?: (u: string, p: string) => Promise<void>;
  entry?: string | { pathname: string; state?: unknown };
  element: ReactNode;
  path: string;
}

/** Renders a public auth page with mocked auth + feature flags and a location probe. */
export function renderAuth({ flags, loading = false, login = async () => {}, entry, element, path }: RenderAuthOptions) {
  const allFlags = { allowLocalAuth: true, oidcEnabled: false, enablePerUserWatch: false, ...flags };
  return render(
    <AuthContext.Provider
      value={{ user: null, loading: false, login, register: async () => {}, logout: () => {} }}
    >
      <FeatureFlagsContext.Provider value={{ flags: allFlags, loading, error: null }}>
        <MemoryRouter initialEntries={[entry ?? path]}>
          <Routes>
            <Route path={path} element={element} />
            <Route path="*" element={<LocationProbe />} />
          </Routes>
        </MemoryRouter>
      </FeatureFlagsContext.Provider>
    </AuthContext.Provider>,
  );
}
