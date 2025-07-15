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
  login: 15000,
  navigation: 15000,
  api: 8000
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
    
    // Go to home page and wait for it to load
    await this.page.goto('/');
    await this.page.waitForLoadState('networkidle');
    
    // Check if already logged in by looking for dashboard content
    const welcomeText = await this.page.locator('h4:has-text("Welcome back,")').isVisible().catch(() => false);
    
    if (welcomeText) {
      console.log('Already logged in - found welcome message');
      return;
    }
    
    // Wait for login page to be ready - look for the distinctive login page content
    await this.page.waitForSelector('h3:has-text("Welcome to Readur")', { timeout: TIMEOUTS.login });
    await this.page.waitForSelector('h5:has-text("Sign in to your account")', { timeout: TIMEOUTS.login });
    
    // Material-UI creates input elements inside TextFields, but we need to wait for them to be ready
    // The inputs have the name attributes from react-hook-form register
    const usernameField = this.page.locator('input[name="username"]');
    const passwordField = this.page.locator('input[name="password"]');
    
    // Wait for both fields to be attached and visible
    await usernameField.waitFor({ state: 'attached', timeout: TIMEOUTS.login });
    await passwordField.waitFor({ state: 'attached', timeout: TIMEOUTS.login });
    
    // WebKit can be slower - add extra wait time
    const browserName = await this.page.evaluate(() => navigator.userAgent);
    const isWebKit = browserName.includes('WebKit') && !browserName.includes('Chrome');
    if (isWebKit) {
      console.log('WebKit browser detected - adding extra wait time');
      await this.page.waitForTimeout(5000);
    }
    
    // Clear any existing content and fill the fields
    await usernameField.clear();
    await usernameField.fill(credentials.username);
    
    await passwordField.clear();
    await passwordField.fill(credentials.password);
    
    // WebKit needs extra time for form validation
    if (isWebKit) {
      await this.page.waitForTimeout(3000);
    }
    
    // Click submit button - look for the sign in button specifically
    const signInButton = this.page.locator('button[type="submit"]:has-text("Sign in")');
    await signInButton.waitFor({ state: 'visible', timeout: TIMEOUTS.login });
    
    if (isWebKit) {
      // WebKit-specific approach: don't wait for API response, just click and wait for navigation
      await signInButton.click();
      
      // WebKit needs more time before checking navigation
      await this.page.waitForTimeout(2000);
      
      // Wait for navigation with longer timeout for WebKit
      await this.page.waitForURL(/.*\/dashboard.*/, { timeout: 25000 });
      console.log(`Successfully navigated to: ${this.page.url()}`);
      
      // Wait for dashboard content to load with extra time for WebKit
      await this.page.waitForFunction(() => {
        return document.querySelector('h4') !== null && 
               (document.querySelector('h4')?.textContent?.includes('Welcome') ||
                document.querySelector('[role="main"]') !== null);
      }, { timeout: 20000 });
      
    } else {
      // Standard approach for other browsers
      const loginPromise = this.page.waitForResponse(response => 
        response.url().includes('/auth/login') && response.status() === 200,
        { timeout: TIMEOUTS.login }
      );
      
      await signInButton.click();
      
      try {
        const response = await loginPromise;
        
        // Wait for navigation to dashboard with more flexible URL pattern
        await this.page.waitForURL(/.*\/dashboard.*/, { timeout: TIMEOUTS.navigation });
        console.log(`Successfully navigated to: ${this.page.url()}`);
        
        // Wait for dashboard content to load - be more flexible about the welcome message
        await this.page.waitForFunction(() => {
          return document.querySelector('h4') !== null && 
                 (document.querySelector('h4')?.textContent?.includes('Welcome') ||
                  document.querySelector('[role="main"]') !== null);
        }, { timeout: TIMEOUTS.navigation });
        
      } catch (error) {
        // Take a screenshot for debugging
        await this.page.screenshot({ 
          path: `test-results/login-failure-${credentials.username}-${Date.now()}.png`,
          fullPage: true 
        });
        throw error;
      }
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