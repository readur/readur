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
  role: 'Admin' | 'User';
}

export interface E2ETestUser {
  credentials: TestCredentials;
  userResponse: TestUserResponse;
  token?: string;
}

export const E2E_TIMEOUTS = {
  login: 10000,
  navigation: 10000,
  api: 5000,
  userCreation: 15000,
} as const;

/**
 * E2E Test Auth Helper - Creates unique test users for each test run
 * Similar to the backend TestAuthHelper but for E2E browser tests
 */
export class E2ETestAuthHelper {
  constructor(private page: Page) {}

  /**
   * Create a unique test user via API call
   */
  async createTestUser(): Promise<E2ETestUser> {
    const uniqueId = this.generateUniqueId();
    const credentials: TestCredentials = {
      username: `e2e_user_${uniqueId}`,
      email: `e2e_user_${uniqueId}@test.com`,
      password: 'testpass123'
    };

    console.log(`Creating E2E test user: ${credentials.username}`);

    try {
      // Make API call to create user
      const response = await this.page.request.post('/api/auth/register', {
        data: {
          username: credentials.username,
          email: credentials.email,
          password: credentials.password
        },
        timeout: E2E_TIMEOUTS.userCreation
      });

      if (!response.ok()) {
        const errorText = await response.text();
        throw new Error(`Failed to create test user. Status: ${response.status()}, Body: ${errorText}`);
      }

      const userResponse: TestUserResponse = await response.json();
      console.log(`✅ Created E2E test user: ${userResponse.username} (${userResponse.id})`);

      return {
        credentials,
        userResponse,
      };
    } catch (error) {
      console.error('❌ Failed to create E2E test user:', error);
      throw error;
    }
  }

  /**
   * Create a unique admin user via API call
   */
  async createAdminUser(): Promise<E2ETestUser> {
    const uniqueId = this.generateUniqueId();
    const credentials: TestCredentials = {
      username: `e2e_admin_${uniqueId}`,
      email: `e2e_admin_${uniqueId}@test.com`,
      password: 'adminpass123'
    };

    console.log(`Creating E2E admin user: ${credentials.username}`);

    try {
      // Make API call to create admin user
      const response = await this.page.request.post('/api/auth/register', {
        data: {
          username: credentials.username,
          email: credentials.email,
          password: credentials.password,
          role: 'admin'
        },
        timeout: E2E_TIMEOUTS.userCreation
      });

      if (!response.ok()) {
        const errorText = await response.text();
        throw new Error(`Failed to create admin user. Status: ${response.status()}, Body: ${errorText}`);
      }

      const userResponse: TestUserResponse = await response.json();
      console.log(`✅ Created E2E admin user: ${userResponse.username} (${userResponse.id})`);

      return {
        credentials,
        userResponse,
      };
    } catch (error) {
      console.error('❌ Failed to create E2E admin user:', error);
      throw error;
    }
  }

  /**
   * Login a user via browser UI and return authentication status
   */
  async loginUser(credentials: TestCredentials): Promise<boolean> {
    console.log(`Attempting to login E2E user: ${credentials.username}...`);
    
    try {
      // Go to home page
      await this.page.goto('/');
      await this.page.waitForLoadState('networkidle');
      
      // Check if already logged in by looking for dashboard content
      const welcomeText = await this.page.locator('h4:has-text("Welcome back,")').isVisible().catch(() => false);
      
      if (welcomeText) {
        console.log('Already logged in - found welcome message');
        return true;
      }
      
      // Look for login form - Material-UI TextFields with labels (based on react-hook-form register)
      const usernameField = this.page.locator('input[name="username"]').first();
      const passwordField = this.page.locator('input[name="password"]').first();
      
      // Wait for login form to be visible
      await usernameField.waitFor({ state: 'visible', timeout: E2E_TIMEOUTS.login });
      await passwordField.waitFor({ state: 'visible', timeout: E2E_TIMEOUTS.login });
      
      // Fill login form
      await usernameField.fill(credentials.username);
      await passwordField.fill(credentials.password);
      
      // Wait for login API response
      const loginPromise = this.page.waitForResponse(response => 
        response.url().includes('/auth/login') && response.status() === 200,
        { timeout: E2E_TIMEOUTS.login }
      );
      
      // Click submit button
      await this.page.click('button[type="submit"]');
      
      await loginPromise;
      console.log(`Login as ${credentials.username} successful`);
      
      // Wait for navigation to dashboard
      await this.page.waitForURL(/.*\/dashboard.*/, { timeout: E2E_TIMEOUTS.navigation });
      
      // Verify login by checking for welcome message
      await this.page.waitForSelector('h4:has-text("Welcome back,")', { timeout: E2E_TIMEOUTS.navigation });
      
      console.log('Navigation completed to:', this.page.url());
      return true;
    } catch (error) {
      console.error(`Login as ${credentials.username} failed:`, error);
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