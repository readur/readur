import { test, expect } from './fixtures/auth';
import { TIMEOUTS } from './utils/test-data';
import { TestHelpers } from './utils/test-helpers';

/**
 * Tests for the per-user watch directory feature flag.
 * These tests verify UI behavior when the feature is disabled (default server config).
 */
test.describe('Per-User Watch Directory Feature Flag', () => {
  let helpers: TestHelpers;

  test.describe('Auth Config Endpoint', () => {
    test('should return enable_per_user_watch field in auth config', async ({ dynamicAdminPage: page }) => {
      // Make a direct API call to check the auth config endpoint
      const response = await page.request.get('/api/auth/config');

      expect(response.ok()).toBe(true);

      const config = await response.json();

      // Verify the enable_per_user_watch field exists (should be false by default in test environment)
      expect(config).toHaveProperty('enable_per_user_watch');
      expect(typeof config.enable_per_user_watch).toBe('boolean');
    });
  });

  test.describe('Settings Page - User Management Tab (Feature Disabled)', () => {
    test.beforeEach(async ({ dynamicAdminPage }) => {
      helpers = new TestHelpers(dynamicAdminPage);
    });

    test('should hide Watch Directory column when feature is disabled', async ({ dynamicAdminPage: page }) => {
      // Navigate to Settings page
      await helpers.navigateToPage('/settings');

      // Click on User Management tab using text-based selection
      const userManagementTab = page.getByRole('tab', { name: /User Management/i });
      await userManagementTab.click();

      // Wait for the user table to be visible
      await expect(page.locator('table')).toBeVisible({ timeout: TIMEOUTS.medium });

      // Verify the "Watch Directory" table header is NOT visible
      const watchDirectoryHeader = page.locator('th:has-text("Watch Directory")');
      await expect(watchDirectoryHeader).not.toBeVisible({ timeout: TIMEOUTS.short });
    });

    test('should hide watch directory action buttons when feature is disabled', async ({ dynamicAdminPage: page }) => {
      // Navigate to Settings page
      await helpers.navigateToPage('/settings');

      // Click on User Management tab using text-based selection
      const userManagementTab = page.getByRole('tab', { name: /User Management/i });
      await userManagementTab.click();

      // Wait for the user table to be visible
      await expect(page.locator('table')).toBeVisible({ timeout: TIMEOUTS.medium });

      // Verify watch directory action buttons are NOT visible
      // These include CreateNewFolderIcon for creating watch directories
      const createFolderButton = page.locator('[data-testid="CreateNewFolderIcon"]');
      await expect(createFolderButton).not.toBeVisible({ timeout: TIMEOUTS.short });

      // Standard user management buttons (Edit, Delete) should still be visible
      const editButton = page.locator('button:has([data-testid="EditIcon"])').first();
      await expect(editButton).toBeVisible({ timeout: TIMEOUTS.short });
    });
  });

  test.describe('Watch Folder Page (Feature Disabled)', () => {
    test.beforeEach(async ({ dynamicAdminPage }) => {
      helpers = new TestHelpers(dynamicAdminPage);
    });

    test('should hide Personal Watch Directory card when feature is disabled', async ({ dynamicAdminPage: page }) => {
      // Navigate to Watch Folder page
      await helpers.navigateToPage('/watch');

      // Wait for the page to load by checking for the global watch folder section
      const globalWatchSection = page.locator('h6:has-text("Global Watch Folder")');
      await expect(globalWatchSection).toBeVisible({ timeout: TIMEOUTS.medium });

      // Verify the "Personal Watch Directory" card is NOT visible
      const personalWatchCard = page.locator('h6:has-text("Personal Watch Directory")');
      await expect(personalWatchCard).not.toBeVisible({ timeout: TIMEOUTS.short });
    });

    test('should show global watch folder section when feature is disabled', async ({ dynamicAdminPage: page }) => {
      // Navigate to Watch Folder page
      await helpers.navigateToPage('/watch');

      // Verify the global watch folder configuration section IS visible
      // This confirms the page loaded correctly even when per-user watch is disabled
      const globalWatchSection = page.locator('h6:has-text("Global Watch Folder")');
      await expect(globalWatchSection).toBeVisible({ timeout: TIMEOUTS.medium });

      // Also verify key elements of the global section are present
      const watchedDirectoryLabel = page.locator('text=Watched Directory');
      await expect(watchedDirectoryLabel).toBeVisible({ timeout: TIMEOUTS.short });
    });
  });
});
