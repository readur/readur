import { test as base, expect } from '@playwright/test';
import type { Page } from '@playwright/test';

// Centralized test credentials to eliminate duplication
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
}

// Shared authentication helper functions
export class AuthHelper {
  constructor(private page: Page) {}

  async loginAs(credentials: typeof TEST_CREDENTIALS.admin | typeof TEST_CREDENTIALS.user) {
    console.log(`Attempting to login as ${credentials.username}...`);
    
    // Go to home page
    await this.page.goto('/');
    await this.page.waitForLoadState('networkidle');
    
    // Check if already logged in
    const usernameInput = await this.page.locator('input[name="username"]').isVisible().catch(() => false);
    
    if (!usernameInput) {
      console.log('Already logged in or no login form found');
      return;
    }
    
    // Fill login form
    await this.page.fill('input[name="username"]', credentials.username);
    await this.page.fill('input[name="password"]', credentials.password);
    
    // Wait for login API response
    const loginPromise = this.page.waitForResponse(response => 
      response.url().includes('/auth/login') && response.status() === 200,
      { timeout: TIMEOUTS.login }
    );
    
    await this.page.click('button[type="submit"]');
    
    try {
      await loginPromise;
      console.log(`Login as ${credentials.username} successful`);
      
      // Wait for navigation away from login page
      await this.page.waitForFunction(() => 
        !window.location.pathname.includes('/login'),
        { timeout: TIMEOUTS.navigation }
      );
      
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
    await auth.loginAs(TEST_CREDENTIALS.user);
    await use(page);
  },
});

export { expect };