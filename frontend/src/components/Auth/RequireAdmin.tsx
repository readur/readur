import React from 'react';
import { Navigate } from 'react-router-dom';
import { useAuth, isAdmin } from '../../contexts/AuthContext';

interface RequireAdminProps {
  children: React.ReactNode;
  /**
   * What to render for non-admin users. Defaults to redirecting to the
   * dashboard, which suits route-level use; pass `null` to render nothing
   * (e.g. for a tab or section inside a page).
   */
  fallback?: React.ReactNode;
}

/**
 * Renders its children only for administrators. The server enforces the same
 * rules; this only keeps admin-only UI out of view for other users.
 */
const RequireAdmin: React.FC<RequireAdminProps> = ({ children, fallback }) => {
  const { user } = useAuth();

  if (!isAdmin(user)) {
    return <>{fallback === undefined ? <Navigate to="/dashboard" replace /> : fallback}</>;
  }

  return <>{children}</>;
};

export default RequireAdmin;
