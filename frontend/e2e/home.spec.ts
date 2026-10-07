import { test, expect } from './fixtures/auth';
import { TEST_FILES } from './utils/test-data';
import { TestHelpers } from './utils/test-helpers';

// Home: the processing pipeline, what needs attention, the sources, and what just arrived.
test.describe('Home', () => {
  test('greets the signed-in user and shows every region', async ({ dynamicAdminPage: page, testAdmin }) => {
    await page.goto('/home');

    const heading = page.getByRole('heading', { level: 1 });
    await expect(heading).toHaveText(new RegExp(`^Good (morning|afternoon|evening), ${testAdmin.credentials.username}$`));
    await expect(page.getByText(/arrived this week$/)).toBeVisible();

    // An admin sees the OCR queue, so the pipeline card is always there.
    await expect(page.getByRole('region', { name: 'Processing pipeline' })).toBeVisible();
    await expect(page.getByRole('region', { name: 'Sources' })).toBeVisible();
    await expect(page.getByRole('region', { name: 'Just arrived' })).toBeVisible();
  });

  test('lays out the pipeline and Just arrived on the left, sources on the right', async ({ dynamicAdminPage: page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto('/home');

    const pipeline = page.getByRole('region', { name: 'Processing pipeline' });
    const recent = page.getByRole('region', { name: 'Just arrived' });
    const sources = page.getByRole('region', { name: 'Sources' });
    await expect(sources).toBeVisible();
    const [p, r, s] = await Promise.all([pipeline.boundingBox(), recent.boundingBox(), sources.boundingBox()]);
    expect(p!.y).toBeLessThan(r!.y);
    expect(s!.x).toBeGreaterThan(p!.x + p!.width - 1);

    // At most five sources; the rest are one link away.
    const rows = sources.getByRole('list', { name: 'Sources' }).getByRole('listitem');
    expect(await rows.count()).toBeLessThanOrEqual(5);
    const more = sources.getByRole('link', { name: /more sources/ });
    if (await more.count()) await expect(more).toHaveAttribute('href', '/sources');
  });

  test('drops the right column under the pipeline when the window is narrow', async ({ dynamicAdminPage: page }) => {
    await page.setViewportSize({ width: 1024, height: 800 });
    await page.goto('/home');
    const pipeline = page.getByRole('region', { name: 'Processing pipeline' });
    const sources = page.getByRole('region', { name: 'Sources' });
    const recent = page.getByRole('region', { name: 'Just arrived' });
    await expect(sources).toBeVisible();
    const [p, s, r] = await Promise.all([pipeline.boundingBox(), sources.boundingBox(), recent.boundingBox()]);
    expect(s!.y).toBeGreaterThan(p!.y + p!.height - 1);
    expect(r!.y).toBeGreaterThan(s!.y);
  });

  test('links to the next step from each region', async ({ dynamicAdminPage: page }) => {
    await page.goto('/home');

    const main = page.getByRole('main');
    await expect(main.getByRole('link', { name: 'Connect source' })).toHaveAttribute('href', '/sources?section=connections&new=1');
    await expect(main.getByRole('link', { name: 'Upload' }).first()).toHaveAttribute('href', '/intake?section=upload');
    await expect(page.getByRole('region', { name: 'Just arrived' }).getByRole('link', { name: 'View all' })).toHaveAttribute(
      'href',
      '/documents',
    );
    await expect(page.getByRole('region', { name: 'Sources' }).getByRole('link', { name: 'Manage' })).toHaveAttribute('href', '/sources');

    await main.getByRole('link', { name: 'Connect source' }).click();
    await expect(page.getByRole('dialog', { name: 'Add connection' })).toBeVisible();
    await expect(page).toHaveURL(/\/sources\?section=connections$/);
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
