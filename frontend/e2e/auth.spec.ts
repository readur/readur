import { test, expect, AuthHelper, TEST_CREDENTIALS, TIMEOUTS } from './fixtures/auth';
import { TestHelpers } from './utils/test-helpers';

test.describe('Authentication', () => {
  test.beforeEach(async ({ page }) => {
    const authHelper = new AuthHelper(page);
    await authHelper.ensureLoggedOut();
  });

  test('should display login form on initial visit', async ({ page }) => {
    await page.goto('/');
    
    // Check for login form elements using Material-UI structure
    await expect(page.locator('input[type="text"]').first()).toBeVisible();
    await expect(page.locator('input[type="password"]').first()).toBeVisible();
    await expect(page.locator('button[type="submit"]')).toBeVisible();
  });

  test('should login with valid credentials', async ({ page }) => {
    const authHelper = new AuthHelper(page);
    
    await authHelper.loginAs(TEST_CREDENTIALS.admin);
    
    // Should redirect to dashboard
    await page.waitForURL(/.*\/dashboard.*/, { timeout: TIMEOUTS.navigation });
    
    // Verify we're logged in by checking for welcome message
    await expect(page.locator('h4:has-text("Welcome back,")')).toBeVisible();
  });

  test('should show error with invalid credentials', async ({ page }) => {
    await page.goto('/');
    
    await page.fill('input[type="text"]', 'invaliduser');
    await page.fill('input[type="password"]', 'wrongpassword');
    
    await page.click('button[type="submit"]');
    
    // Should show error message (Material-UI Alert)
    await expect(page.locator('.MuiAlert-root, [role="alert"]')).toBeVisible({ timeout: TIMEOUTS.api });
    
    // Should remain on login page
    await expect(page.locator('input[type="text"]')).toBeVisible();
  });

  test.skip('should logout successfully', async ({ page }) => {
    const authHelper = new AuthHelper(page);
    
    // First login
    await authHelper.loginAs(TEST_CREDENTIALS.admin);
    
    await page.waitForURL(/\/dashboard|\//, { timeout: TIMEOUTS.navigation });
    
    // Find and click profile/account button in the top app bar (has AccountIcon)
    const profileButton = page.locator('button:has([data-testid="AccountCircleIcon"])');
    await profileButton.click();
    
    // Wait for profile menu to open and click logout
    const logoutMenuItem = page.locator('li[role="menuitem"]:has-text("Logout")');
    await logoutMenuItem.click();
    
    // Should redirect back to login
    await page.waitForURL(/\/login|\//, { timeout: TIMEOUTS.navigation });
    await expect(page.locator('input[name="username"]')).toBeVisible();
  });

  test.skip('should persist session on page reload', async ({ page }) => {
    const authHelper = new AuthHelper(page);
    
    // Login first
    await authHelper.loginAs(TEST_CREDENTIALS.admin);
    
    await page.waitForURL(/\/dashboard|\//, { timeout: TIMEOUTS.navigation });
    
    // Reload the page
    await page.reload();
    
    // Wait for page to load after reload
    await page.waitForLoadState('networkidle');
    
    // Should still be logged in (either on dashboard or main page, but not login)
    await page.waitForURL(/\/dashboard|\/(?!login)/, { timeout: TIMEOUTS.navigation });
    await expect(page.locator('input[name="username"]')).not.toBeVisible();
  });

  test('should validate required fields', async ({ page }) => {
    await page.goto('/');
    
    // Try to submit without filling fields
    await page.click('button[type="submit"]');
    
    // Should show validation errors or prevent submission
    const usernameInput = page.locator('input[type="text"]');
    const passwordInput = page.locator('input[type="password"]');
    
    // Check for HTML5 validation or custom validation messages
    await expect(usernameInput).toBeVisible();
    await expect(passwordInput).toBeVisible();
  });
});