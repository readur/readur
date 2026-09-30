import { lazy, Suspense, type ComponentType, type ReactNode } from 'react';
import { Navigate, Route, Routes, useLocation } from 'react-router-dom';
import { useAuth } from '../contexts/AuthContext';
import { NotificationProvider } from '../contexts/NotificationContext';
import { AppShell, HOME_PATH, PageFallback } from '../features/shell';
import { safeRedirect } from '../features/auth/authErrors';
import { LegacyRedirect } from './LegacyRedirect';
import { LEGACY_ROUTES, searchAliasTarget } from './legacyRoutes';

// Each destination loads its feature's entry file. Feature work replaces those files, not this table.
const Home = lazy(() => import('../features/board'));
const Library = lazy(() => import('../features/library'));
// The Search page is the library feature's `SearchPage` export; until that exists /search falls
// back to the older route, which hands the query to the Library.
const Search = lazy(async (): Promise<{ default: ComponentType }> => {
  const mod = (await import('../features/library')) as unknown as { SearchPage?: ComponentType };
  let page: ComponentType | undefined;
  try {
    page = mod.SearchPage;
  } catch {
    page = undefined; // a test double without the export
  }
  if (page) return { default: page };
  return import('../features/library/SearchRoute');
});
const Document = lazy(() => import('../features/document'));
const Shared = lazy(() => import('../features/document/SharedRoute'));
const Intake = lazy(() => import('../features/intake'));
const Settings = lazy(() => import('../features/settings'));
const Login = lazy(() => import('../features/auth/LoginRoute'));
const Callback = lazy(() => import('../features/auth/CallbackRoute'));
const Register = lazy(() => import('../features/auth/RegisterRoute'));

export { HOME_PATH };

const Public = ({ children }: { children: ReactNode }) => (
  <Suspense fallback={<PageFallback />}>{children}</Suspense>
);

/** Signed-in routes render inside the shell; everything else goes to /login. */
function RequireUser() {
  const { user } = useAuth();
  const location = useLocation();
  if (!user) return <Navigate to="/login" replace state={{ from: location }} />;
  return (
    <NotificationProvider>
      <AppShell />
    </NotificationProvider>
  );
}

/** /search?query=… is an older spelling of /search?q=…; everything else renders the page. */
function SearchEntry() {
  const { search, hash } = useLocation();
  const target = searchAliasTarget(search);
  if (target !== null) return <Navigate replace to={{ pathname: '/search', search: target, hash }} />;
  return <Search />;
}

/**
 * A signed-in visitor on /login goes where they were headed (RequireUser's `state.from`),
 * else home. Sign-in flips `user` before LoginRoute's own navigate runs, so this redirect
 * must agree with it or it wins and drops the requested route.
 */
function SignedInRedirect() {
  const location = useLocation();
  const to = safeRedirect((location.state as { from?: unknown } | null)?.from) ?? HOME_PATH;
  return <Navigate to={to} replace />;
}

export function AppRoutes() {
  const { user } = useAuth();
  return (
    <Routes>
      <Route
        path="/login"
        element={user ? <SignedInRedirect /> : <Public><Login /></Public>}
      />
      <Route
        path="/register"
        element={user ? <Navigate to={HOME_PATH} replace /> : <Public><Register /></Public>}
      />
      <Route path="/auth/callback" element={<Public><Callback /></Public>} />
      <Route path="/shared/:token" element={<Public><Shared /></Public>} />

      <Route element={<RequireUser />}>
        <Route path={HOME_PATH} element={<Home />} />
        <Route path="/documents" element={<Library />} />
        <Route path="/documents/:id" element={<Document />} />
        <Route path="/search" element={<SearchEntry />} />
        <Route path="/intake" element={<Intake />} />
        <Route path="/settings/:section?" element={<Settings />} />
        {LEGACY_ROUTES.map(({ from, to }) => (
          <Route key={from} path={from} element={<LegacyRedirect to={to} />} />
        ))}
        <Route path="*" element={<Navigate to={HOME_PATH} replace />} />
      </Route>
    </Routes>
  );
}

export default AppRoutes;
