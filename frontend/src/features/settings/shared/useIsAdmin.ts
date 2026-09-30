import { useContext } from 'react';
import { AuthContext } from '../../../contexts/AuthContext';
import { isAdmin } from '../../../auth/roles';

export function useIsAdmin(): boolean {
  const auth = useContext(AuthContext);
  return isAdmin(auth?.user);
}

export function useCurrentUserId(): string | undefined {
  return useContext(AuthContext)?.user?.id;
}
