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
      const response = await page.request.get('/api/auth/config');
      expect(response.ok()).toBe(true);

      const config = await response.json();
      expect(config).toHaveProperty('enable_per_user_watch');
      expect(typeof config.enable_per_user_watch).toBe('boolean');
    });
  });

  test.describe('Settings → Users (Feature Disabled)', () => {
    test.beforeEach(async ({ dynamicAdminPage }) => {
      helpers = new TestHelpers(dynamicAdminPage);
      const config = await (await dynamicAdminPage.request.get('/api/auth/config')).json();
      test.skip(config.enable_per_user_watch === true, 'Server has per-user watch directories enabled');
    });

    test('should hide Watch Directory column when feature is disabled', async ({ dynamicAdminPage: page }) => {
      await page.goto('/settings/users');

      const table = page.getByRole('grid', { name: 'User Management' });
      await expect(table).toBeVisible({ timeout: TIMEOUTS.medium });
      await expect(table.getByRole('columnheader', { name: 'Username' })).toBeVisible();
      await expect(table.getByRole('columnheader', { name: /Watch Directory/i })).toHaveCount(0);
    });

    test('should hide watch directory action buttons when feature is disabled', async ({ dynamicAdminPage: page }) => {
      await page.goto('/settings/users');

      const table = page.getByRole('grid', { name: 'User Management' });
      await expect(table).toBeVisible({ timeout: TIMEOUTS.medium });
      await expect(table.getByRole('button', { name: /watch directory/i })).toHaveCount(0);

      // Sanity check: user rows with their normal actions are there
      await expect(table.getByRole('button', { name: /^Edit / }).first()).toBeVisible();
    });
  });

  test.describe('Intake → Watch folder (Feature Disabled)', () => {
    test.beforeEach(async ({ dynamicAdminPage }) => {
      helpers = new TestHelpers(dynamicAdminPage);
      const config = await (await dynamicAdminPage.request.get('/api/auth/config')).json();
      test.skip(config.enable_per_user_watch === true, 'Server has per-user watch directories enabled');
    });

    test('should hide Personal Watch Directory card when feature is disabled', async ({ dynamicAdminPage: page }) => {
      // The legacy /watch URL lands on Intake → Watch folder
      await page.goto('/watch');
      await expect(page).toHaveURL(/\/intake\?section=watch/);

      const folders = page.getByRole('grid', { name: 'Watched folders' });
      await expect(folders).toBeVisible({ timeout: TIMEOUTS.medium });
      await expect(folders.getByRole('gridcell', { name: 'Server', exact: true })).toBeVisible();
      await expect(folders.getByRole('gridcell', { name: /Personal/i })).toHaveCount(0);
      await expect(page.getByRole('button', { name: /personal/i })).toHaveCount(0);
    });

    test('should show global watch folder section when feature is disabled', async ({ dynamicAdminPage: page }) => {
      await helpers.openIntake('watch');

      await expect(page.getByRole('region', { name: 'Watched folders' })).toBeVisible({ timeout: TIMEOUTS.medium });
      await expect(page.getByRole('region', { name: 'Processing queue' })).toBeVisible();
      await expect(page.getByRole('region', { name: 'How the watch folder works' })).toBeVisible();
    });
  });
});
