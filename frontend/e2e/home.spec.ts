import { test, expect } from './fixtures/auth';
import { TEST_FILES } from './utils/test-data';
import { TestHelpers } from './utils/test-helpers';

// Home: is everything still coming in, what is processing, what just arrived.
test.describe('Home', () => {
  test('greets the signed-in user and shows every region', async ({ dynamicAdminPage: page, testAdmin }) => {
    await page.goto('/home');

    const heading = page.getByRole('heading', { level: 1 });
    await expect(heading).toHaveText(new RegExp(`^Good (morning|afternoon|evening), ${testAdmin.credentials.username}$`));
    await expect(page.getByText(/arrived this week$/)).toBeVisible();

    await expect(page.getByRole('region', { name: 'Coming in' })).toBeVisible();
    await expect(page.getByRole('region', { name: 'Processing' })).toBeVisible();
    await expect(page.getByRole('region', { name: 'Just arrived' })).toBeVisible();
  });

  test('shows a lane for uploads and the watch folder, each with a 14-day strip', async ({ dynamicAdminPage: page }) => {
    await page.goto('/home');

    const lanes = page.getByRole('list', { name: 'Coming in' });
    await expect(lanes.getByRole('link', { name: 'Uploads' })).toHaveAttribute('href', '/intake?section=upload');
    await expect(lanes.getByRole('link', { name: 'Watch folder' })).toHaveAttribute('href', '/intake?section=watch');
    await expect(lanes.getByRole('img', { name: /in the last 14 days/ }).first()).toBeVisible();
  });

  test('links to the next step from each region', async ({ dynamicAdminPage: page }) => {
    await page.goto('/home');

    const main = page.getByRole('main');
    await expect(main.getByRole('link', { name: 'Add documents' }).first()).toHaveAttribute('href', '/intake?section=upload');
    await expect(page.getByRole('region', { name: 'Just arrived' }).getByRole('link', { name: 'Open library' })).toHaveAttribute(
      'href',
      '/documents',
    );
    await expect(page.getByRole('region', { name: 'Coming in' }).getByRole('link', { name: 'Manage sources' })).toHaveAttribute(
      'href',
      '/intake?section=connections',
    );

    await main.getByRole('link', { name: 'Add documents' }).first().click();
    await expect(page).toHaveURL(/\/intake\?section=upload/);
  });

  test('shows a new document in Just arrived and opens it', async ({ dynamicUserPage: page }) => {
    const helpers = new TestHelpers(page);
    await helpers.uploadDocumentViaAPI(TEST_FILES.test2);

    await page.goto('/home');
    const recent = page.getByRole('region', { name: 'Just arrived' });
    const item = recent.getByRole('link', { name: /test2\.jpg/ });
    await expect(item).toBeVisible({ timeout: 20000 });

    await item.click();
    await expect(page).toHaveURL(/\/documents\/[0-9a-f-]{36}/);
  });
});
