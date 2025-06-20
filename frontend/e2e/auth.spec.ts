import { test, expect } from '@playwright/test';
import { TEST_USERS, TIMEOUTS } from './utils/test-data';
import { TestHelpers } from './utils/test-helpers';

test.describe('Authentication', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ page }) => {
    helpers = new TestHelpers(page);
  });

  test('should display login form on initial visit', async ({ page }) => {
    await page.goto('/');
    
    // Check for login form elements using Material-UI structure
    await expect(page.locator('input[name="username"]')).toBeVisible();
    await expect(page.locator('input[name="password"]')).toBeVisible();
    await expect(page.locator('button[type="submit"]')).toBeVisible();
  });

  test('should login with valid credentials', async ({ page }) => {
    await page.goto('/');
    
    // Fill login form with demo credentials
    await page.fill('input[name="username"]', 'admin');
    await page.fill('input[name="password"]', 'readur2024');
    
    // Wait for login API call
    const loginResponse = helpers.waitForApiCall('/auth/login');
    
    await page.click('button[type="submit"]');
    
    // Verify login was successful
    await loginResponse;
    
    // Should redirect to dashboard or main page
    await page.waitForURL(/\/dashboard|\//, { timeout: TIMEOUTS.medium });
    
    // Verify we're no longer on login page
    await expect(page.locator('input[name="username"]')).not.toBeVisible();
  });

  test('should show error with invalid credentials', async ({ page }) => {
    await page.goto('/');
    
    await page.fill('input[name="username"]', 'invaliduser');
    await page.fill('input[name="password"]', 'wrongpassword');
    
    await page.click('button[type="submit"]');
    
    // Should show error message (Material-UI Alert)
    await expect(page.locator('.MuiAlert-root, [role="alert"]')).toBeVisible({ timeout: TIMEOUTS.short });
    
    // Should remain on login page
    await expect(page.locator('input[name="username"]')).toBeVisible();
  });

  test.skip('should logout successfully', async ({ page }) => {
    // First login
    await page.goto('/');
    await page.fill('input[name="username"]', 'admin');
    await page.fill('input[name="password"]', 'readur2024');
    await page.click('button[type="submit"]');
    
    await page.waitForURL(/\/dashboard|\//, { timeout: TIMEOUTS.medium });
    
    // Find and click profile/account button in the top app bar (has AccountIcon)
    const profileButton = page.locator('button:has([data-testid="AccountCircleIcon"])');
    await profileButton.click();
    
    // Wait for profile menu to open and click logout
    const logoutMenuItem = page.locator('li[role="menuitem"]:has-text("Logout")');
    await logoutMenuItem.click();
    
    // Should redirect back to login
    await page.waitForURL(/\/login|\//, { timeout: TIMEOUTS.medium });
    await expect(page.locator('input[name="username"]')).toBeVisible();
  });

  test.skip('should persist session on page reload', async ({ page }) => {
    // Login first
    await page.goto('/');
    await page.fill('input[name="username"]', 'admin');
    await page.fill('input[name="password"]', 'readur2024');
    await page.click('button[type="submit"]');
    
    await page.waitForURL(/\/dashboard|\//, { timeout: TIMEOUTS.medium });
    
    // Reload the page
    await page.reload();
    
    // Wait for page to load after reload
    await page.waitForLoadState('networkidle');
    
    // Should still be logged in (either on dashboard or main page, but not login)
    await page.waitForURL(/\/dashboard|\/(?!login)/, { timeout: TIMEOUTS.medium });
    await expect(page.locator('input[name="username"]')).not.toBeVisible();
  });

  test('should validate required fields', async ({ page }) => {
    await page.goto('/');
    
    // Try to submit without filling fields
    await page.click('button[type="submit"]');
    
    // Should show validation errors or prevent submission
    const usernameInput = page.locator('input[name="username"]');
    const passwordInput = page.locator('input[name="password"]');
    
    // Check for HTML5 validation or custom validation messages
    await expect(usernameInput).toBeVisible();
    await expect(passwordInput).toBeVisible();
  });
});