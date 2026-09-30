import { expect, type Page } from '@playwright/test';

export interface TestCredentials {
  username: string;
  password: string;
  email: string;
}

export interface TestUserResponse {
  id: string;
  username: string;
  email: string;
  /** Wire values from the API: 'admin' | 'user'. */
  role: string;
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
 * Optional pre-provisioned admin (for example the one printed in the server log on
 * first boot). When both are set, `createAdminUser()` signs in as that account
 * instead of registering a fresh admin.
 *
 *   E2E_ADMIN_USERNAME=admin E2E_ADMIN_PASSWORD=... npx playwright test
 */
export function envAdminCredentials(): TestCredentials | null {
  const username = process.env.E2E_ADMIN_USERNAME;
  const password = process.env.E2E_ADMIN_PASSWORD;
  if (!username || !password) return null;
  return { username, password, email: process.env.E2E_ADMIN_EMAIL ?? `${username}@localhost` };
}

/** Password used for users the suite registers itself (override with E2E_USER_PASSWORD). */
const USER_PASSWORD = process.env.E2E_USER_PASSWORD ?? 'testpass123';
const ADMIN_PASSWORD = process.env.E2E_ADMIN_TEST_PASSWORD ?? 'adminpass123';

/**
 * Creates unique users per test through the API and signs them in through the
 * sign-in page (`/login`: "Username", "Password", "Sign in" → `/board`).
 */
export class E2ETestAuthHelper {
  constructor(private page: Page) {}

  async createTestUser(): Promise<E2ETestUser> {
    const id = this.generateUniqueId();
    return this.register({
      username: `e2e_user_${id}`,
      email: `e2e_user_${id}@test.com`,
      password: USER_PASSWORD,
    });
  }

  async createAdminUser(): Promise<E2ETestUser> {
    const envAdmin = envAdminCredentials();
    if (envAdmin) {
      const token = await this.loginUserAPI(envAdmin);
      const me = await this.page.request.get('/api/auth/me', { headers: { Authorization: `Bearer ${token}` } });
      if (!me.ok()) throw new Error(`Could not read the configured admin: ${me.status()}`);
      return { credentials: envAdmin, userResponse: await me.json(), token };
    }
    const id = this.generateUniqueId();
    return this.register(
      { username: `e2e_admin_${id}`, email: `e2e_admin_${id}@test.com`, password: ADMIN_PASSWORD },
      'admin',
    );
  }

  private async register(credentials: TestCredentials, role?: 'admin'): Promise<E2ETestUser> {
    const response = await this.page.request.post('/api/auth/register', {
      data: { ...credentials, ...(role ? { role } : {}) },
      timeout: E2E_TIMEOUTS.userCreation,
    });
    if (!response.ok()) {
      throw new Error(`Failed to create test user. Status: ${response.status()}, Body: ${await response.text()}`);
    }
    return { credentials, userResponse: await response.json() };
  }

  /** Sign in through the UI. Resolves true once the Board has rendered. */
  async loginUser(credentials: TestCredentials): Promise<boolean> {
    try {
      await this.page.goto('/login');
      const username = this.page.getByLabel('Username');
      await expect(username).toBeVisible({ timeout: E2E_TIMEOUTS.login });
      await username.fill(credentials.username);
      await this.page.getByLabel('Password', { exact: true }).fill(credentials.password);

      const loginResponse = this.page.waitForResponse(
        (r) => r.url().includes('/api/auth/login') && r.request().method() === 'POST',
        { timeout: E2E_TIMEOUTS.login },
      );
      await this.page.getByRole('button', { name: 'Sign in', exact: true }).click();
      const response = await loginResponse;
      if (!response.ok()) throw new Error(`Login returned ${response.status()}`);

      await this.page.waitForURL(/\/board/, { timeout: E2E_TIMEOUTS.navigation });
      await expect(this.page.getByRole('heading', { level: 1, name: 'Board' })).toBeVisible({
        timeout: E2E_TIMEOUTS.navigation,
      });
      return true;
    } catch (error) {
      console.error(`Login as ${credentials.username} failed:`, error);
      await this.page
        .screenshot({ path: `test-results/login-failure-${credentials.username}-${Date.now()}.png`, fullPage: true })
        .catch(() => undefined);
      return false;
    }
  }

  async loginUserAPI(credentials: Pick<TestCredentials, 'username' | 'password'>): Promise<string> {
    const response = await this.page.request.post('/api/auth/login', {
      data: { username: credentials.username, password: credentials.password },
      timeout: E2E_TIMEOUTS.api,
    });
    if (!response.ok()) {
      throw new Error(`API login failed. Status: ${response.status()}, Body: ${await response.text()}`);
    }
    const { token } = await response.json();
    if (!token) throw new Error('No token received from login response');
    return token;
  }

  /** Log out through the account menu. */
  async logout(): Promise<void> {
    await this.page.getByRole('button', { name: 'Account menu' }).click();
    await this.page.getByRole('menuitem', { name: 'Log out' }).click();
    await this.page.waitForURL(/\/login/, { timeout: E2E_TIMEOUTS.navigation });
  }

  async ensureLoggedOut(): Promise<void> {
    await this.page.goto('/login');
    const username = this.page.getByLabel('Username');
    const accountMenu = this.page.getByRole('button', { name: 'Account menu' });
    await expect(username.or(accountMenu)).toBeVisible({ timeout: E2E_TIMEOUTS.navigation });
    if (await username.isVisible()) return;
    await this.logout();
  }

  private generateUniqueId(): string {
    const random = crypto.randomUUID().replace(/-/g, '').substring(0, 8);
    const pid = typeof process !== 'undefined' ? process.pid : crypto.getRandomValues(new Uint16Array(1))[0];
    return `${Date.now()}_${pid}_${random}`;
  }
}

export async function createE2ETestUser(page: Page): Promise<E2ETestUser> {
  return new E2ETestAuthHelper(page).createTestUser();
}

export async function createE2EAdminUser(page: Page): Promise<E2ETestUser> {
  return new E2ETestAuthHelper(page).createAdminUser();
}

export async function loginE2EUser(page: Page, credentials: TestCredentials): Promise<boolean> {
  return new E2ETestAuthHelper(page).loginUser(credentials);
}
