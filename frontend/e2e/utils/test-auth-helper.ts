import type { Page } from '@playwright/test';

export interface TestCredentials {
  username: string;
  password: string;
  email: string;
}

export interface TestUserResponse {
  id: string;
  username: string;
  email: string;
  role: 'admin' | 'user';
}

export interface E2ETestUser {
  credentials: TestCredentials;
  userResponse: TestUserResponse;
  token?: string;
}

export const E2E_TIMEOUTS = {
  login: 15000,
  navigation: 15000,
  api: 8000,
  userCreation: 20000,
} as const;

/**
 * Credentials of the admin account the server seeds at startup
 * (ADMIN_USERNAME / ADMIN_PASSWORD). Test users are provisioned through this
 * account because self-registration is disabled by default and, when enabled,
 * produces accounts that must be approved before they can log in.
 *
 * E2E_ADMIN_USERNAME / E2E_ADMIN_PASSWORD take precedence so a local run can
 * point at a differently-configured server without touching server env.
 */
export function getSeededAdminCredentials(): { username: string; password: string } {
  const username = process.env.E2E_ADMIN_USERNAME || process.env.ADMIN_USERNAME || 'admin';
  const password = process.env.E2E_ADMIN_PASSWORD || process.env.ADMIN_PASSWORD;
  if (!password) {
    throw new Error(
      'E2E tests need the seeded admin password to provision users. ' +
        'Set E2E_ADMIN_PASSWORD (or ADMIN_PASSWORD) to the value the server was started with.'
    );
  }
  return { username, password };
}

// Cached per worker process; refreshed on 401 (e.g. after expiry or revocation).
let cachedAdminToken: string | undefined;

/**
 * E2E Test Auth Helper - Creates unique test users for each test run
 * Similar to the backend TestAuthHelper but for E2E browser tests
 */
export class E2ETestAuthHelper {
  constructor(private page: Page) {}

  /**
   * Log in as the seeded admin via the API and return a Bearer token.
   */
  async getSeededAdminToken(forceRefresh = false): Promise<string> {
    if (cachedAdminToken && !forceRefresh) {
      return cachedAdminToken;
    }

    const admin = getSeededAdminCredentials();
    const response = await this.page.request.post('/api/auth/login', {
      data: { username: admin.username, password: admin.password },
      timeout: E2E_TIMEOUTS.api,
    });

    if (!response.ok()) {
      const errorText = await response.text();
      throw new Error(
        `Seeded admin login failed for '${admin.username}'. Status: ${response.status()}, Body: ${errorText}`
      );
    }

    const body = await response.json();
    if (!body.token) {
      throw new Error('No token received from seeded admin login');
    }
    cachedAdminToken = body.token as string;
    return cachedAdminToken;
  }

  /**
   * Send an admin-authenticated request, retrying once with a fresh token on 401.
   */
  private async adminRequest(method: 'post' | 'put', url: string, data: unknown) {
    let token = await this.getSeededAdminToken();
    for (let attempt = 0; attempt < 2; attempt++) {
      const response = await this.page.request[method](url, {
        data,
        headers: { Authorization: `Bearer ${token}` },
        timeout: E2E_TIMEOUTS.userCreation,
      });
      if (response.status() !== 401 || attempt === 1) {
        return response;
      }
      token = await this.getSeededAdminToken(true);
    }
    throw new Error('unreachable');
  }

  /**
   * Create an active account with the given role through the admin users API.
   */
  private async provisionUser(prefix: string, password: string, role: 'user' | 'admin'): Promise<E2ETestUser> {
    const uniqueId = this.generateUniqueId();
    const credentials: TestCredentials = {
      username: `${prefix}_${uniqueId}`,
      email: `${prefix}_${uniqueId}@test.com`,
      password,
    };

    const response = await this.adminRequest('post', '/api/users', {
      username: credentials.username,
      email: credentials.email,
      password: credentials.password,
      role,
    });

    if (!response.ok()) {
      const errorText = await response.text();
      throw new Error(`Failed to create ${role} test user. Status: ${response.status()}, Body: ${errorText}`);
    }

    const userResponse: TestUserResponse = await response.json();
    return { credentials, userResponse };
  }

  /**
   * Create a unique regular test user via the admin users API
   */
  async createTestUser(): Promise<E2ETestUser> {
    try {
      return await this.provisionUser('e2e_user', 'testpass123', 'user');
    } catch (error) {
      console.error('❌ Failed to create E2E test user:', error);
      throw error;
    }
  }

  /**
   * Create a unique admin user via the admin users API
   */
  async createAdminUser(): Promise<E2ETestUser> {
    try {
      return await this.provisionUser('e2e_admin', 'adminpass123', 'admin');
    } catch (error) {
      console.error('❌ Failed to create E2E admin user:', error);
      throw error;
    }
  }

  /**
   * Activate an account (e.g. one created through self-registration, which
   * starts out inactive until approved by an admin).
   */
  async approveUser(userId: string): Promise<void> {
    const response = await this.adminRequest('put', `/api/users/${userId}`, { is_active: true });
    if (!response.ok()) {
      const errorText = await response.text();
      throw new Error(`Failed to approve user ${userId}. Status: ${response.status()}, Body: ${errorText}`);
    }
  }

  /**
   * Login a user via browser UI and return authentication status
   */
  async loginUser(credentials: TestCredentials): Promise<boolean> {
    
    try {
      // Go to home page and wait for it to load
      await this.page.goto('/');
      await this.page.waitForLoadState('networkidle');
      
      // Check if already logged in by looking for dashboard content
      const welcomeText = await this.page.locator('h4:has-text("Welcome back,")').isVisible().catch(() => false);
      
      if (welcomeText) {
        return true;
      }
      
      // Wait for login page to be ready - look for the distinctive login page content
      await this.page.waitForSelector('h3:has-text("Welcome to Readur")', { timeout: E2E_TIMEOUTS.login });
      await this.page.waitForSelector('h5:has-text("Sign in to your account")', { timeout: E2E_TIMEOUTS.login });
      
      // Material-UI creates input elements inside TextFields, but we need to wait for them to be ready
      // The inputs have the name attributes from react-hook-form register
      const usernameField = this.page.locator('input[name="username"]');
      const passwordField = this.page.locator('input[name="password"]');
      
      // Wait for both fields to be attached and visible
      await usernameField.waitFor({ state: 'attached', timeout: E2E_TIMEOUTS.login });
      await passwordField.waitFor({ state: 'attached', timeout: E2E_TIMEOUTS.login });
      
      // Browser-specific wait time
      const browserName = await this.page.context().browser()?.browserType().name() || '';
      const isWebKit = browserName === 'webkit';
      const isFirefox = browserName === 'firefox';
      
      if (isWebKit) {
        console.log('WebKit browser detected - adding extra wait time');
        await this.page.waitForTimeout(5000);
      } else if (isFirefox) {
        console.log('Firefox browser detected - adding extra wait time');
        await this.page.waitForTimeout(3000);
      }
      
      // Clear any existing content and fill the fields
      await usernameField.clear();
      await usernameField.fill(credentials.username);
      
      await passwordField.clear();
      await passwordField.fill(credentials.password);
      
      // Browser-specific wait for form validation
      if (isWebKit) {
        await this.page.waitForTimeout(3000);
      } else if (isFirefox) {
        await this.page.waitForTimeout(2000);
      }
      
      // Click submit button - look for the sign in button specifically
      const signInButton = this.page.locator('button[type="submit"]:has-text("Sign in")');
      await signInButton.waitFor({ state: 'visible', timeout: E2E_TIMEOUTS.login });
      
      if (isWebKit || isFirefox) {
        // WebKit and Firefox specific approach: don't wait for API response, just click and wait for navigation
        await signInButton.click();
        
        // Browser-specific wait before checking navigation
        await this.page.waitForTimeout(isWebKit ? 2000 : 1500);
        
        // Wait for navigation with longer timeout for WebKit/Firefox
        await this.page.waitForURL(/.*\/dashboard.*/, { timeout: 25000 });
        console.log(`Successfully navigated to: ${this.page.url()}`);
        
        // Wait for dashboard content to load with extra time
        await this.page.waitForFunction(() => {
          return document.querySelector('h4') !== null && 
                 (document.querySelector('h4')?.textContent?.includes('Welcome') ||
                  document.querySelector('[role="main"]') !== null);
        }, { timeout: 20000 });
        
      } else {
        // Standard approach for other browsers
        const loginPromise = this.page.waitForResponse(response => 
          response.url().includes('/auth/login') && response.status() === 200,
          { timeout: E2E_TIMEOUTS.login }
        );
        
        await signInButton.click();
        
        const response = await loginPromise;
        
        // Wait for navigation to dashboard with more flexible URL pattern
        await this.page.waitForURL(/.*\/dashboard.*/, { timeout: E2E_TIMEOUTS.navigation });
        console.log(`Successfully navigated to: ${this.page.url()}`);
        
        // Wait for dashboard content to load - be more flexible about the welcome message
        await this.page.waitForFunction(() => {
          return document.querySelector('h4') !== null && 
                 (document.querySelector('h4')?.textContent?.includes('Welcome') ||
                  document.querySelector('[role="main"]') !== null);
        }, { timeout: E2E_TIMEOUTS.navigation });
      }
      
      console.log(`Login as ${credentials.username} completed successfully`);
      return true;
    } catch (error) {
      console.error(`Login as ${credentials.username} failed:`, error);
      // Take a screenshot for debugging
      await this.page.screenshot({ 
        path: `test-results/login-failure-${credentials.username}-${Date.now()}.png`,
        fullPage: true 
      });
      return false;
    }
  }

  /**
   * Login a user via API and return authentication token
   */
  async loginUserAPI(credentials: TestCredentials): Promise<string> {
    console.log(`API login for E2E user: ${credentials.username}...`);

    try {
      const response = await this.page.request.post('/api/auth/login', {
        data: {
          username: credentials.username,
          password: credentials.password
        },
        timeout: E2E_TIMEOUTS.api
      });

      if (!response.ok()) {
        const errorText = await response.text();
        throw new Error(`API login failed. Status: ${response.status()}, Body: ${errorText}`);
      }

      const loginResponse = await response.json();
      const token = loginResponse.token;

      if (!token) {
        throw new Error('No token received from login response');
      }

      console.log(`✅ API login successful for ${credentials.username}`);
      return token;
    } catch (error) {
      console.error(`❌ API login failed for ${credentials.username}:`, error);
      throw error;
    }
  }

  /**
   * Logout user via browser UI
   */
  async logout(): Promise<boolean> {
    try {
      // Look for logout button/link and click it
      const logoutButton = this.page.locator('[data-testid="logout"], button:has-text("Logout"), a:has-text("Logout")').first();
      
      if (await logoutButton.isVisible({ timeout: 5000 })) {
        await logoutButton.click();
        
        // Wait for redirect to login page
        await this.page.waitForFunction(() => 
          window.location.pathname.includes('/login') || window.location.pathname === '/',
          { timeout: E2E_TIMEOUTS.navigation }
        );
        
        console.log('✅ Logout successful');
        return true;
      } else {
        console.log('⚠️ Logout button not found - may already be logged out');
        return true;
      }
    } catch (error) {
      console.error('❌ Logout failed:', error);
      return false;
    }
  }

  /**
   * Ensure user is logged out
   */
  async ensureLoggedOut(): Promise<void> {
    await this.page.goto('/');
    await this.page.waitForLoadState('networkidle');
    
    // If we see a login form, we're already logged out
    const usernameInput = await this.page.locator('input[name="username"]').isVisible().catch(() => false);
    if (usernameInput) {
      console.log('Already logged out - login form visible');
      return;
    }
    
    // Otherwise, try to logout
    await this.logout();
  }

  /**
   * Generate a unique ID for test users to avoid collisions
   */
  private generateUniqueId(): string {
    const timestamp = Date.now();
    const random = Math.random().toString(36).substring(2, 8);
    const processId = typeof process !== 'undefined' ? process.pid : Math.floor(Math.random() * 10000);
    return `${timestamp}_${processId}_${random}`;
  }

  /**
   * Clean up test user (optional - users are isolated per test run)
   */
  async cleanupUser(userId: string): Promise<boolean> {
    try {
      console.log(`Cleaning up E2E test user: ${userId}`);
      
      // This would require admin privileges or a special cleanup endpoint
      // For now, we rely on test isolation and database cleanup between test runs
      console.log(`⚠️ User cleanup not implemented - relying on test isolation`);
      
      return true;
    } catch (error) {
      console.error(`❌ Failed to cleanup user ${userId}:`, error);
      return false;
    }
  }
}

/**
 * Create an E2E test user and return credentials
 */
export async function createE2ETestUser(page: Page): Promise<E2ETestUser> {
  const authHelper = new E2ETestAuthHelper(page);
  return await authHelper.createTestUser();
}

/**
 * Create an E2E admin user and return credentials  
 */
export async function createE2EAdminUser(page: Page): Promise<E2ETestUser> {
  const authHelper = new E2ETestAuthHelper(page);
  return await authHelper.createAdminUser();
}

/**
 * Login an E2E user via browser UI
 */
export async function loginE2EUser(page: Page, credentials: TestCredentials): Promise<boolean> {
  const authHelper = new E2ETestAuthHelper(page);
  return await authHelper.loginUser(credentials);
}