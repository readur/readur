import type { ReactNode } from 'react';
import { render } from '@testing-library/react';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import { AuthContext } from '../../../contexts/AuthContext';
import type { useAuth } from '../../../contexts/AuthContext';
import { FeatureFlagsContext } from '../../../contexts/FeatureFlagsContext';

export interface Flags {
  allowLocalAuth: boolean;
  allowRegistration: boolean;
  oidcEnabled: boolean;
}

type AuthValue = ReturnType<typeof useAuth>;

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
  /** Any other auth context fields (completeLogin, register, sessionNotice, …). */
  auth?: Partial<AuthValue>;
  entry?: string | { pathname: string; state?: unknown };
  element: ReactNode;
  path: string;
}

/** Renders a public auth page with mocked auth + feature flags and a location probe. */
export function renderAuth({ flags, loading = false, login = async () => {}, auth, entry, element, path }: RenderAuthOptions) {
  const allFlags = { allowLocalAuth: true, allowRegistration: false, oidcEnabled: false, enablePerUserWatch: false, ...flags };
  const value: AuthValue = {
    user: null,
    loading: false,
    login,
    register: async () => ({ pendingApproval: true, user: null as never }),
    logout: async () => {},
    changePassword: async () => {},
    completeLogin: () => {},
    sessionNotice: null,
    dismissSessionNotice: () => {},
    ...auth,
  };
  return render(
    <AuthContext.Provider value={value}>
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
