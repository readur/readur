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
  /** Wire values from the API. */
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
 * produces accounts that must be approved before they can sign in.
 *
 * E2E_ADMIN_USERNAME / E2E_ADMIN_PASSWORD take precedence so a local run can
 * point at a differently-configured server without touching server env.
 *
 *   E2E_ADMIN_USERNAME=admin E2E_ADMIN_PASSWORD=... npx playwright test
 */
export function getSeededAdminCredentials(): { username: string; password: string } {
  const username = process.env.E2E_ADMIN_USERNAME || process.env.ADMIN_USERNAME || 'admin';
  const password = process.env.E2E_ADMIN_PASSWORD || process.env.ADMIN_PASSWORD;
  if (!password) {
    throw new Error(
      'E2E tests need the seeded admin password to provision users. ' +
        'Set E2E_ADMIN_PASSWORD (or ADMIN_PASSWORD) to the value the server was started with.',
    );
  }
  return { username, password };
}

/** Passwords for the users the suite provisions (override with E2E_USER_PASSWORD / E2E_ADMIN_TEST_PASSWORD). */
const USER_PASSWORD = process.env.E2E_USER_PASSWORD ?? 'testpass123';
const ADMIN_PASSWORD = process.env.E2E_ADMIN_TEST_PASSWORD ?? 'adminpass123';

// Cached per worker process; refreshed on 401 (e.g. after expiry or revocation).
let cachedAdminToken: string | undefined;

/**
 * Creates unique users per test through the admin users API and signs them in
 * through the sign-in page (`/login`: "Username", "Password", "Sign in" → `/home`).
 */
export class E2ETestAuthHelper {
  constructor(private page: Page) {}

  /** Sign in as the seeded admin through the API and return a bearer token. */
  async getSeededAdminToken(forceRefresh = false): Promise<string> {
    if (cachedAdminToken && !forceRefresh) return cachedAdminToken;
    cachedAdminToken = await this.loginUserAPI(getSeededAdminCredentials());
    return cachedAdminToken;
  }

  /** Send an admin-authenticated request, retrying once with a fresh token on 401. */
  private async adminRequest(method: 'post' | 'put', url: string, data: unknown) {
    let token = await this.getSeededAdminToken();
    for (let attempt = 0; attempt < 2; attempt++) {
      const response = await this.page.request[method](url, {
        data,
        headers: { Authorization: `Bearer ${token}` },
        timeout: E2E_TIMEOUTS.userCreation,
      });
      if (response.status() !== 401 || attempt === 1) return response;
      token = await this.getSeededAdminToken(true);
    }
    throw new Error('unreachable');
  }

  /** Create an active account with the given role through the admin users API. */
  private async provisionUser(prefix: string, password: string, role: 'user' | 'admin'): Promise<E2ETestUser> {
    const id = this.generateUniqueId();
    const credentials: TestCredentials = {
      username: `${prefix}_${id}`,
      email: `${prefix}_${id}@test.com`,
      password,
    };
    const response = await this.adminRequest('post', '/api/users', { ...credentials, role });
    if (!response.ok()) {
      throw new Error(`Failed to create ${role} test user. Status: ${response.status()}, Body: ${await response.text()}`);
    }
    return { credentials, userResponse: await response.json() };
  }

  async createTestUser(): Promise<E2ETestUser> {
    return this.provisionUser('e2e_user', USER_PASSWORD, 'user');
  }

  async createAdminUser(): Promise<E2ETestUser> {
    return this.provisionUser('e2e_admin', ADMIN_PASSWORD, 'admin');
  }

  /**
   * Activate an account (e.g. one created through self-registration, which
   * starts out inactive until an admin approves it).
   */
  async approveUser(userId: string): Promise<void> {
    const response = await this.adminRequest('put', `/api/users/${userId}`, { is_active: true });
    if (!response.ok()) {
      throw new Error(`Failed to approve user ${userId}. Status: ${response.status()}, Body: ${await response.text()}`);
    }
  }

  /** Sign in through the UI. Resolves true once Home has rendered. */
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

      await this.page.waitForURL(/\/home/, { timeout: E2E_TIMEOUTS.navigation });
      await expect(this.page.getByRole('heading', { level: 1 })).toBeVisible({
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
    const random = crypto.randomUUID().replace(/-/g, '').substring(0, 12);
    const pid = typeof process !== 'undefined' ? process.pid : 0;
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
