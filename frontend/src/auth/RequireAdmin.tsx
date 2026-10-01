import type { ReactNode } from 'react';
import { useContext } from 'react';
import { Navigate } from 'react-router-dom';
import { AuthContext } from '../contexts/AuthContext';
import { isAdmin } from './roles';

export interface RequireAdminProps {
  children: ReactNode;
  /**
   * What non-admins see instead. Defaults to a redirect home, which suits route-level use; pass
   * `null` (or a message) for a section inside a page.
   */
  fallback?: ReactNode;
}

/**
 * Renders its children only for administrators. The server enforces the same rules; this only
 * keeps admin-only UI, and the requests it would make, away from other users.
 */
export function RequireAdmin({ children, fallback }: RequireAdminProps) {
  const user = useContext(AuthContext)?.user;
  if (!isAdmin(user)) return <>{fallback === undefined ? <Navigate to="/board" replace /> : fallback}</>;
  return <>{children}</>;
}

export default RequireAdmin;
