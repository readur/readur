import { useContext } from 'react';
import { AuthContext } from '../../../contexts/AuthContext';

/** The API sends `admin`; older client code compared against `Admin`. Accept either. */
export function isAdminRole(role: string | undefined | null): boolean {
  return (role ?? '').toLowerCase() === 'admin';
}

export function useIsAdmin(): boolean {
  const auth = useContext(AuthContext);
  return isAdminRole(auth?.user?.role);
}

export function useCurrentUserId(): string | undefined {
  return useContext(AuthContext)?.user?.id;
}
