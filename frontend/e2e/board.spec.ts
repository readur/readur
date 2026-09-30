import { test, expect } from './fixtures/auth';
import { TEST_FILES } from './utils/test-data';
import { TestHelpers } from './utils/test-helpers';

// The old Dashboard ("Welcome back", stat cards, quick actions) is now the Board.
test.describe('Board', () => {
  test('should show the board and the signed-in user', async ({ dynamicAdminPage: page, testAdmin }) => {
    await page.goto('/board');

    await expect(page.getByRole('heading', { level: 1, name: 'Board' })).toBeVisible();

    // The account menu names the signed-in user
    await page.getByRole('button', { name: 'Account menu' }).click();
    await expect(page.getByRole('menu', { name: 'Account menu' })).toContainText(testAdmin.credentials.username);
    await page.keyboard.press('Escape');
  });

  test('should display library totals and processing figures', async ({ dynamicAdminPage: page }) => {
    await page.goto('/board');

    const totals = page.getByRole('region', { name: 'Library' });
    await expect(totals).toBeVisible();
    for (const term of ['Documents', 'Storage', 'Indexed', 'Labels']) {
      await expect(totals.getByRole('term').filter({ hasText: term })).toBeVisible();
    }

    const processing = page.getByRole('region', { name: 'Processing' });
    for (const term of ['Pending', 'Processing', 'Failed', 'Done today']) {
      await expect(processing.getByRole('term').filter({ hasText: new RegExp(`^${term}$`) })).toBeVisible();
    }

    await expect(page.getByRole('region', { name: 'Recently added' })).toBeVisible();
    await expect(page.getByRole('region', { name: 'Connections' })).toBeVisible();
  });

  test('should display quick actions', async ({ dynamicAdminPage: page }) => {
    await page.goto('/board');

    const main = page.getByRole('main');
    await expect(main.getByRole('link', { name: 'Add documents' }).first()).toHaveAttribute('href', '/intake?section=upload');
    await expect(page.getByRole('region', { name: 'Recently added' }).getByRole('link', { name: 'Open library' })).toHaveAttribute(
      'href',
      '/documents',
    );
    await expect(page.getByRole('region', { name: 'Connections' }).getByRole('link', { name: 'Manage' })).toHaveAttribute(
      'href',
      '/intake?section=connections',
    );
  });

  test('should have working navigation', async ({ dynamicAdminPage: page }) => {
    await page.goto('/board');

    await page.getByRole('main').getByRole('link', { name: 'Add documents' }).first().click();
    await expect(page).toHaveURL(/\/intake\?section=upload/);
    await expect(page.getByRole('heading', { level: 1, name: 'Intake' })).toBeVisible();

    await page.getByRole('navigation', { name: 'Main' }).getByRole('link', { name: 'Board' }).click();
    await expect(page).toHaveURL(/\/board/);
    await expect(page.getByRole('heading', { level: 1, name: 'Board' })).toBeVisible();
  });

  test('should list a new document in arrivals', async ({ dynamicUserPage: page }) => {
    const helpers = new TestHelpers(page);
    await helpers.uploadDocumentViaAPI(TEST_FILES.test2);

    await page.goto('/board');
    const arrivals = page.getByRole('region', { name: 'Recently added' });
    await expect(arrivals.getByRole('row', { name: /test2\.jpg/ })).toBeVisible({ timeout: 20000 });

    await arrivals.getByRole('row', { name: /test2\.jpg/ }).click();
    await expect(page).toHaveURL(/\/documents\/[0-9a-f-]{36}/);
    await expect(page.getByRole('heading', { level: 1, name: 'test2.jpg' })).toBeVisible();
  });
});
