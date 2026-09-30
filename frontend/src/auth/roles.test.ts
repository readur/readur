import { describe, expect, it } from 'vitest';
import type { UserRole } from '../types/generated';
import { isAdmin, isAdminRole } from './roles';

describe('isAdmin', () => {
  it('is true for the API wire value "admin"', () => {
    const role: UserRole = 'admin';
    expect(isAdmin({ role })).toBe(true);
  });

  it('is false for "user"', () => {
    const role: UserRole = 'user';
    expect(isAdmin({ role })).toBe(false);
  });

  it('is case-insensitive so older payloads and mocks keep working', () => {
    expect(isAdmin({ role: 'Admin' })).toBe(true);
    expect(isAdmin({ role: 'ADMIN' })).toBe(true);
  });

  it('is false for a missing user or role', () => {
    expect(isAdmin(null)).toBe(false);
    expect(isAdmin(undefined)).toBe(false);
    expect(isAdmin({})).toBe(false);
    expect(isAdmin({ role: null })).toBe(false);
  });

  it('does not treat lookalike roles as admin', () => {
    expect(isAdmin({ role: 'administrator' })).toBe(false);
  });
});

describe('isAdminRole', () => {
  it('checks a bare role value', () => {
    expect(isAdminRole('admin')).toBe(true);
    expect(isAdminRole('user')).toBe(false);
    expect(isAdminRole(undefined)).toBe(false);
  });
});
