import { lazy, Suspense, type ReactNode } from 'react';
import { Navigate, Route, Routes } from 'react-router-dom';
import { useAuth } from '../contexts/AuthContext';
import { NotificationProvider } from '../contexts/NotificationContext';
import { AppShell, PageFallback } from '../features/shell';
import { LegacyRedirect } from './LegacyRedirect';
import { LEGACY_ROUTES } from './legacyRoutes';

// Each destination loads its feature's entry file. Feature work replaces those files, not this table.
const Board = lazy(() => import('../features/board'));
const Library = lazy(() => import('../features/library'));
const Search = lazy(() => import('../features/library/SearchRoute'));
const Document = lazy(() => import('../features/document'));
const Shared = lazy(() => import('../features/document/SharedRoute'));
const Intake = lazy(() => import('../features/intake'));
const Settings = lazy(() => import('../features/settings'));
const Login = lazy(() => import('../features/auth/LoginRoute'));
const Callback = lazy(() => import('../features/auth/CallbackRoute'));

export const HOME_PATH = '/board';

const Public = ({ children }: { children: ReactNode }) => (
  <Suspense fallback={<PageFallback />}>{children}</Suspense>
);

/** Signed-in routes render inside the shell; everything else goes to /login. */
function RequireUser() {
  const { user } = useAuth();
  if (!user) return <Navigate to="/login" replace />;
  return (
    <NotificationProvider>
      <AppShell />
    </NotificationProvider>
  );
}

export function AppRoutes() {
  const { user } = useAuth();
  return (
    <Routes>
      <Route
        path="/login"
        element={user ? <Navigate to={HOME_PATH} replace /> : <Public><Login /></Public>}
      />
      <Route path="/auth/callback" element={<Public><Callback /></Public>} />
      <Route path="/shared/:token" element={<Public><Shared /></Public>} />

      <Route element={<RequireUser />}>
        <Route path="/board" element={<Board />} />
        <Route path="/documents" element={<Library />} />
        <Route path="/documents/:id" element={<Document />} />
        <Route path="/search" element={<Search />} />
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
