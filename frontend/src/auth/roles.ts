import type { UserRole } from '../types/generated';

/**
 * True for an admin. The API sends `admin`; the comparison is case-insensitive
 * so older mocks and cached payloads that say `Admin` keep working.
 */
export function isAdmin(user?: { role?: string | null } | null): boolean {
  return (user?.role ?? '').toLowerCase() === 'admin';
}

/** Same check for a bare role value (e.g. a role passed as a prop). */
export function isAdminRole(role?: UserRole | string | null): boolean {
  return isAdmin({ role });
}
