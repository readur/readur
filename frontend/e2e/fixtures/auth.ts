import { test as base, expect } from '@playwright/test';
import type { Page } from '@playwright/test';
import { E2ETestAuthHelper, type E2ETestUser, type TestCredentials } from '../utils/test-auth-helper';

// Legacy credentials for backward compatibility (still available for seeded admin user)
export const TEST_CREDENTIALS = {
  admin: {
    username: 'admin',
    password: 'readur2024'
  },
  user: {
    username: 'user',
    password: 'userpass123'
  }
} as const;

export const TIMEOUTS = {
  login: 10000,
  navigation: 10000,
  api: 5000
} as const;

export interface AuthFixture {
  authenticatedPage: Page;
  adminPage: Page;
  userPage: Page;
  dynamicAdminPage: Page;
  dynamicUserPage: Page;
  testUser: E2ETestUser;
  testAdmin: E2ETestUser;
}

// Shared authentication helper functions
export class AuthHelper {
  constructor(private page: Page) {}

  async loginAs(credentials: typeof TEST_CREDENTIALS.admin | typeof TEST_CREDENTIALS.user) {
    console.log(`Attempting to login as ${credentials.username}...`);
    
    // Go to home page
    await this.page.goto('/');
    await this.page.waitForLoadState('networkidle');
    
    // Check if already logged in by looking for dashboard content
    const welcomeText = await this.page.locator('h4:has-text("Welcome back,")').isVisible().catch(() => false);
    
    if (welcomeText) {
      console.log('Already logged in - found welcome message');
      return;
    }
    
    // Look for login form - Material-UI TextFields with labels (based on react-hook-form register)
    const usernameField = this.page.locator('input[name="username"]').first();
    const passwordField = this.page.locator('input[name="password"]').first();
    
    // Wait for login form to be visible
    await usernameField.waitFor({ state: 'visible', timeout: TIMEOUTS.login });
    await passwordField.waitFor({ state: 'visible', timeout: TIMEOUTS.login });
    
    // Fill login form
    await usernameField.fill(credentials.username);
    await passwordField.fill(credentials.password);
    
    // Wait for login API response
    const loginPromise = this.page.waitForResponse(response => 
      response.url().includes('/auth/login') && response.status() === 200,
      { timeout: TIMEOUTS.login }
    );
    
    // Click submit button
    await this.page.click('button[type="submit"]');
    
    try {
      await loginPromise;
      console.log(`Login as ${credentials.username} successful`);
      
      // Wait for navigation to dashboard
      await this.page.waitForURL(/.*\/dashboard.*/, { timeout: TIMEOUTS.navigation });
      
      // Verify login by checking for welcome message
      await this.page.waitForSelector('h4:has-text("Welcome back,")', { timeout: TIMEOUTS.navigation });
      
      console.log('Navigation completed to:', this.page.url());
    } catch (error) {
      console.error(`Login as ${credentials.username} failed:`, error);
      throw error;
    }
  }

  async logout() {
    // Look for logout button/link and click it
    const logoutButton = this.page.locator('[data-testid="logout"], button:has-text("Logout"), a:has-text("Logout")').first();
    
    if (await logoutButton.isVisible()) {
      await logoutButton.click();
      
      // Wait for redirect to login page
      await this.page.waitForFunction(() => 
        window.location.pathname.includes('/login') || window.location.pathname === '/',
        { timeout: TIMEOUTS.navigation }
      );
    }
  }

  async ensureLoggedOut() {
    await this.page.goto('/');
    await this.page.waitForLoadState('networkidle');
    
    // If we see a login form, we're already logged out
    const usernameInput = await this.page.locator('input[name="username"]').isVisible().catch(() => false);
    if (usernameInput) {
      return;
    }
    
    // Otherwise, try to logout
    await this.logout();
  }
}

export const test = base.extend<AuthFixture>({
  // Legacy fixtures using seeded users (for backward compatibility)
  authenticatedPage: async ({ page }, use) => {
    const auth = new AuthHelper(page);
    await auth.loginAs(TEST_CREDENTIALS.admin);
    await use(page);
  },

  adminPage: async ({ page }, use) => {
    const auth = new AuthHelper(page);
    await auth.loginAs(TEST_CREDENTIALS.admin);
    await use(page);
  },

  userPage: async ({ page }, use) => {
    const auth = new AuthHelper(page);
    await auth.loginAs(TEST_CREDENTIALS.admin); // Use admin since 'user' doesn't exist
    await use(page);
  },

  // New dynamic fixtures using API-created users
  testUser: async ({ page }, use) => {
    const authHelper = new E2ETestAuthHelper(page);
    const testUser = await authHelper.createTestUser();
    console.log(`Created dynamic test user: ${testUser.credentials.username}`);
    await use(testUser);
  },

  testAdmin: async ({ page }, use) => {
    const authHelper = new E2ETestAuthHelper(page);
    const testAdmin = await authHelper.createAdminUser();
    console.log(`Created dynamic test admin: ${testAdmin.credentials.username}`);
    await use(testAdmin);
  },

  dynamicUserPage: async ({ page, testUser }, use) => {
    const authHelper = new E2ETestAuthHelper(page);
    const loginSuccess = await authHelper.loginUser(testUser.credentials);
    if (!loginSuccess) {
      throw new Error(`Failed to login dynamic test user: ${testUser.credentials.username}`);
    }
    console.log(`Logged in dynamic test user: ${testUser.credentials.username}`);
    await use(page);
  },

  dynamicAdminPage: async ({ page, testAdmin }, use) => {
    const authHelper = new E2ETestAuthHelper(page);
    const loginSuccess = await authHelper.loginUser(testAdmin.credentials);
    if (!loginSuccess) {
      throw new Error(`Failed to login dynamic test admin: ${testAdmin.credentials.username}`);
    }
    console.log(`Logged in dynamic test admin: ${testAdmin.credentials.username}`);
    await use(page);
  },
});

export { expect };