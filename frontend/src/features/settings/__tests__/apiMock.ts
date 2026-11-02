import { vi } from 'vitest';

/**
 * Stand-in for the axios instance exported by `services/api`. This module must not import
 * anything that itself imports `services/api`, or the mock factory would deadlock.
 */
export const apiMock = {
  get: vi.fn(),
  post: vi.fn(),
  put: vi.fn(),
  delete: vi.fn(),
};

/** Use as `vi.mock('…/services/api', async (orig) => (await import('./apiMock')).mockApiModule(orig))`. */
export async function mockApiModule(importOriginal: () => Promise<unknown>) {
  const actual = (await importOriginal()) as Record<string, unknown>;
  return { ...actual, default: apiMock, api: apiMock };
}
