import { test, expect, TIMEOUTS } from './fixtures/auth';
import { E2ETestAuthHelper } from './utils/test-auth-helper';

test.describe('Authentication', () => {
  test('should display login form on initial visit', async ({ page }) => {
    await page.goto('/');

    // Signed-out visitors land on the sign-in page.
    await expect(page).toHaveURL(/\/login/);
    await expect(page.getByRole('heading', { level: 1, name: 'Sign in' })).toBeVisible();
    await expect(page.getByLabel('Username')).toBeVisible();
    await expect(page.getByLabel('Password', { exact: true })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Sign in', exact: true })).toBeVisible();
  });

  test('should login with valid credentials', async ({ page }) => {
    const authHelper = new E2ETestAuthHelper(page);
    const testUser = await authHelper.createTestUser();

    const loginSuccess = await authHelper.loginUser(testUser.credentials);
    expect(loginSuccess).toBe(true);

    // Sign-in lands on Home.
    await expect(page).toHaveURL(/\/home/, { timeout: TIMEOUTS.navigation });
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
    await expect(page.getByRole('navigation', { name: 'Main' })).toBeVisible();
  });

  test('should show error with invalid credentials', async ({ page }) => {
    await page.goto('/login');

    await page.getByLabel('Username').fill('invaliduser');
    await page.getByLabel('Password', { exact: true }).fill('wrongpassword');
    await page.getByRole('button', { name: 'Sign in', exact: true }).click();

    await expect(page.getByRole('alert')).toBeVisible({ timeout: TIMEOUTS.api });

    // Should remain on the sign-in page
    await expect(page).toHaveURL(/\/login/);
    await expect(page.getByLabel('Username')).toBeVisible();
  });

  test('should logout successfully', async ({ page }) => {
    const authHelper = new E2ETestAuthHelper(page);
    const testUser = await authHelper.createTestUser();
    expect(await authHelper.loginUser(testUser.credentials)).toBe(true);

    await authHelper.logout();

    await expect(page).toHaveURL(/\/login/);
    await expect(page.getByLabel('Username')).toBeVisible();

    // Protected pages send the visitor back to sign-in
    await page.goto('/documents');
    await expect(page).toHaveURL(/\/login/);
  });

  test('should persist session on page reload', async ({ page }) => {
    const authHelper = new E2ETestAuthHelper(page);
    const testUser = await authHelper.createTestUser();
    expect(await authHelper.loginUser(testUser.credentials)).toBe(true);

    await page.reload();

    await expect(page).toHaveURL(/\/home/, { timeout: TIMEOUTS.navigation });
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
    await expect(page.getByLabel('Username')).toHaveCount(0);
  });

  test('should validate required fields', async ({ page }) => {
    await page.goto('/login');

    let loginCalled = false;
    page.on('request', (r) => {
      if (r.url().includes('/api/auth/login')) loginCalled = true;
    });

    await page.getByRole('button', { name: 'Sign in', exact: true }).click();

    // Inline errors, no request
    await expect(page.getByText('Enter your username')).toBeVisible();
    await expect(page.getByText('Enter your password')).toBeVisible();
    await expect(page).toHaveURL(/\/login/);
    expect(loginCalled).toBe(false);
  });

  test('should return to the requested page after sign-in', async ({ page }) => {
    const authHelper = new E2ETestAuthHelper(page);
    const testUser = await authHelper.createTestUser();

    await page.goto('/settings/appearance');
    await expect(page).toHaveURL(/\/login/);

    await page.getByLabel('Username').fill(testUser.credentials.username);
    await page.getByLabel('Password', { exact: true }).fill(testUser.credentials.password);
    await page.getByRole('button', { name: 'Sign in', exact: true }).click();

    await expect(page).toHaveURL(/\/settings\/appearance/, { timeout: TIMEOUTS.navigation });
    // And it stays there (the signed-in /login redirect must not fire afterwards).
    await page.waitForTimeout(500);
    await expect(page).toHaveURL(/\/settings\/appearance/);
  });
});
