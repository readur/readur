import { test as base, expect } from '@playwright/test';
import type { Page } from '@playwright/test';
import { E2ETestAuthHelper, type E2ETestUser, type TestCredentials } from '../utils/test-auth-helper';

export const TIMEOUTS = {
  login: 15000,
  navigation: 15000,
  api: 8000
} as const;

export interface AuthFixture {
  dynamicAdminPage: Page;
  dynamicUserPage: Page;
  testUser: E2ETestUser;
  testAdmin: E2ETestUser;
}

export const test = base.extend<AuthFixture>({
  // Dynamic fixtures using API-created users
  testUser: async ({ page }, use) => {
    const authHelper = new E2ETestAuthHelper(page);
    const testUser = await authHelper.createTestUser();
    await use(testUser);
  },

  testAdmin: async ({ page }, use) => {
    const authHelper = new E2ETestAuthHelper(page);
    const testAdmin = await authHelper.createAdminUser();
    await use(testAdmin);
  },

  dynamicUserPage: async ({ page, testUser }, use) => {
    const authHelper = new E2ETestAuthHelper(page);
    const loginSuccess = await authHelper.loginUser(testUser.credentials);
    if (!loginSuccess) {
      throw new Error(`Failed to login dynamic test user: ${testUser.credentials.username}`);
    }
    await use(page);
  },

  dynamicAdminPage: async ({ page, testAdmin }, use) => {
    const authHelper = new E2ETestAuthHelper(page);
    const loginSuccess = await authHelper.loginUser(testAdmin.credentials);
    if (!loginSuccess) {
      throw new Error(`Failed to login dynamic test admin: ${testAdmin.credentials.username}`);
    }
    await use(page);
  },
});

export { expect };