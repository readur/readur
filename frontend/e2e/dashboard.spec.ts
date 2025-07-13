import { test, expect } from './fixtures/auth';
import { TEST_CREDENTIALS } from './fixtures/auth';

test.describe('Dashboard', () => {
  test('should display welcome back message after login', async ({ authenticatedPage: page }) => {
    // Navigate to dashboard
    await page.goto('/dashboard');
    await page.waitForLoadState('networkidle');
    
    // Check for welcome message
    await expect(page.locator('h4:has-text("Welcome back,")')).toBeVisible();
    
    // Check for username in welcome message
    await expect(page.locator(`h4:has-text("Welcome back, ${TEST_CREDENTIALS.admin.username}!")`)).toBeVisible();
  });

  test('should display dashboard stats', async ({ authenticatedPage: page }) => {
    await page.goto('/dashboard');
    await page.waitForLoadState('networkidle');
    
    // Check for stats cards
    await expect(page.locator('text="Total Documents"')).toBeVisible();
    await expect(page.locator('text="Storage Used"')).toBeVisible();
    await expect(page.locator('text="OCR Processed"')).toBeVisible();
    await expect(page.locator('text="Searchable"')).toBeVisible();
  });

  test('should display quick actions', async ({ authenticatedPage: page }) => {
    await page.goto('/dashboard');
    await page.waitForLoadState('networkidle');
    
    // Check for quick action buttons
    await expect(page.locator('text="Upload Documents"')).toBeVisible();
    await expect(page.locator('text="Search Library"')).toBeVisible();
    await expect(page.locator('text="Browse Documents"')).toBeVisible();
  });

  test('should have working navigation', async ({ authenticatedPage: page }) => {
    await page.goto('/dashboard');
    await page.waitForLoadState('networkidle');
    
    // Test navigation to different pages
    await page.click('text="Upload Documents"');
    await page.waitForURL(/.*\/upload.*/, { timeout: 5000 });
    
    // Go back to dashboard
    await page.goto('/dashboard');
    await page.waitForLoadState('networkidle');
    
    // Verify we're back on dashboard
    await expect(page.locator('h4:has-text("Welcome back,")')).toBeVisible();
  });
});