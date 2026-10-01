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

  test('puts what just arrived first, then the most active sources', async ({ dynamicAdminPage: page }) => {
    await page.goto('/home');

    await expect(page.getByRole('group', { name: 'Summary' })).toContainText(/arrived this week/);
    const recent = page.getByRole('region', { name: 'Just arrived' });
    const coming = page.getByRole('region', { name: 'Coming in' });
    await expect(coming).toBeVisible();
    const [recentTop, comingTop] = await Promise.all([recent.boundingBox(), coming.boundingBox()]);
    expect(recentTop!.y).toBeLessThan(comingTop!.y);

    // At most five lanes; the rest are one link away in Intake.
    const lanes = coming.getByRole('list', { name: 'Coming in' }).getByRole('listitem');
    expect(await lanes.count()).toBeLessThanOrEqual(5);
    const more = coming.getByRole('link', { name: /more sources/ });
    if (await more.count()) await expect(more).toHaveAttribute('href', '/intake?section=connections');
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
