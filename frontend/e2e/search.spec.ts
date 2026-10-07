import { test, expect } from './fixtures/auth';
import type { Locator, Page } from '@playwright/test';
import { SEARCH_QUERIES, TEST_FILES, TIMEOUTS } from './utils/test-data';
import { TestHelpers } from './utils/test-helpers';

/**
 * Search (/search?q=…): one large field, a count, a month histogram, filters and results grouped by
 * month with their passages. /documents?q=… moves here. Each test signs in as a fresh user and
 * seeds OCR'd images with known text.
 */
test.describe('Search Functionality', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ dynamicUserPage }) => {
    helpers = new TestHelpers(dynamicUserPage);
    // test1.png → "Test 1 / This is some text from text 1", test3.jpeg → "... text 3"
    const ids = [await helpers.uploadDocumentViaAPI(TEST_FILES.test1), await helpers.uploadDocumentViaAPI(TEST_FILES.test3)];
    for (const id of ids) await helpers.waitForOCRComplete(id);
    await dynamicUserPage.goto('/search');
    await expect(dynamicUserPage.getByRole('heading', { level: 1, name: 'Search' })).toBeVisible();
  });

  const searchBox = (page: Page) => page.getByRole('searchbox', { name: 'Search documents' });
  /** One result per document: its name is the level-3 heading. */
  const results = (page: Page): Locator => page.getByRole('main').getByRole('heading', { level: 3 });
  const result = (page: Page, name: string): Locator => page.getByRole('main').getByRole('listitem').filter({ has: page.getByRole('heading', { level: 3, name }) });

  async function search(page: Page, q: string) {
    const box = searchBox(page);
    await box.fill(q);
    const response = helpers.waitForApiCall('/api/search/enhanced', TIMEOUTS.medium);
    await box.press('Enter');
    await response;
    await expect(page).toHaveURL(new RegExp(`[?&]q=${encodeURIComponent(q).replace(/%20/g, '(\\+|%20)')}`));
  }

  test('should display search interface', async ({ dynamicUserPage: page }) => {
    await expect(searchBox(page)).toBeVisible();
    await expect(searchBox(page)).toBeFocused();
    await expect(page.getByRole('button', { name: 'Search help' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Find anything in your documents' })).toBeVisible();
    for (const name of ['Type', 'Collection', 'Source', 'Added', 'Status']) {
      await expect(page.getByRole('search', { name: 'Search and filter' }).getByRole('button', { name })).toBeVisible();
    }
  });

  test('should perform basic search', async ({ dynamicUserPage: page }) => {
    await search(page, SEARCH_QUERIES.content); // "some text from text"

    await expect(page.getByText(`2 documents match “${SEARCH_QUERIES.content}”`)).toBeVisible();
    await expect(results(page)).toHaveCount(2);
    await expect(result(page, 'test1.png')).toBeVisible();
    await expect(result(page, 'test3.jpeg')).toBeVisible();
    // Newest first, under a month heading, with a histogram of every match by month
    await expect(page.getByRole('main').getByRole('heading', { level: 2 }).first()).toBeVisible();
    await expect(page.getByRole('group', { name: 'Matches by month' })).toBeVisible();
  });

  test('should carry a search typed in the Library over to Search', async ({ dynamicUserPage: page }) => {
    await page.goto('/documents');
    await searchBox(page).fill('text 3');
    await searchBox(page).press('Enter');
    await expect(page).toHaveURL(/\/search\?q=text(\+|%20)3/);
    await expect(results(page)).toHaveCount(1);
  });

  test('should show search suggestions', async ({ dynamicUserPage: page }) => {
    // Type-ahead suggestions live in the ⌘K palette
    await page.getByRole('button', { name: 'Search documents' }).first().click();
    const palette = page.getByRole('dialog', { name: 'Command palette' });
    await palette.getByRole('searchbox', { name: 'Search' }).pressSequentially('some text', { delay: 50 });
    const documents = palette.getByRole('group', { name: 'Documents' });
    await expect(documents.getByRole('menuitem', { name: /test1\.png/ })).toBeVisible({ timeout: TIMEOUTS.short });
  });

  test('should filter search results', async ({ dynamicUserPage: page }) => {
    await search(page, SEARCH_QUERIES.content);
    await expect(results(page)).toHaveCount(2);

    await page.getByRole('search', { name: 'Search and filter' }).getByRole('button', { name: 'Type' }).click();
    await page.getByRole('dialog', { name: 'Type' }).getByText('PDF', { exact: true }).click();
    await expect(page).toHaveURL(/type=pdf/);
    await expect(page).toHaveURL(/q=/);
    await page.keyboard.press('Escape');

    // No PDFs match: the empty state offers to clear the filters
    await expect(page.getByRole('heading', { name: 'No matches' })).toBeVisible();
    await page.getByRole('button', { name: 'Clear filters' }).click();
    await expect(page).not.toHaveURL(/type=/);
    await expect(results(page)).toHaveCount(2);
  });

  test('should narrow to a month from the histogram', async ({ dynamicUserPage: page }) => {
    await search(page, SEARCH_QUERIES.content);
    const month = page.getByRole('group', { name: 'Matches by month' }).getByRole('button').last();
    await month.click();
    await expect(page).toHaveURL(/from=\d{4}-\d{2}-01&to=\d{4}-\d{2}-\d{2}/);
    await expect(month).toHaveAttribute('aria-pressed', 'true');
    await expect(results(page)).toHaveCount(2);
    await page.getByRole('button', { name: 'Show all dates' }).click();
    await expect(page).not.toHaveURL(/from=/);
  });

  test('should perform advanced search', async ({ dynamicUserPage: page }) => {
    await page.getByRole('button', { name: 'Search help' }).click();
    const help = page.getByRole('dialog', { name: 'Search help' });
    await expect(help).toBeVisible();
    await help.getByText('Exact phrase', { exact: true }).click();
    await expect(page).toHaveURL(/mode=phrase/);
    await page.keyboard.press('Escape');

    await search(page, 'text from text 3');
    await expect(results(page)).toHaveCount(1);
    await expect(results(page).first()).toContainText('test3.jpeg');
  });

  test('should handle empty search results', async ({ dynamicUserPage: page }) => {
    await search(page, SEARCH_QUERIES.noResults);
    await expect(page.getByRole('heading', { name: 'No matches' })).toBeVisible({ timeout: TIMEOUTS.medium });
    await expect(page.getByText(/Check the spelling/)).toBeVisible();
  });

  test('should open a result in the drawer over the results, finding the search words', async ({ dynamicUserPage: page }) => {
    await search(page, 'text 1');
    await results(page).filter({ hasText: 'test1.png' }).getByRole('link').first().click();
    await expect(page).toHaveURL(/\/search\?q=text(\+|%20)1.*&document=[0-9a-f-]{36}/, { timeout: TIMEOUTS.medium });
    const drawer = page.getByRole('dialog', { name: 'test1.png' });
    await expect(drawer).toBeVisible();
    await expect(drawer.getByRole('region', { name: 'Extracted text' }).locator('mark').first()).toBeVisible();
    // Closing leaves you on the same results.
    await page.keyboard.press('Escape');
    await expect(page).toHaveURL(/\/search\?q=text(\+|%20)1/);
    await expect(page).not.toHaveURL(/document=/);
    await expect(results(page).filter({ hasText: 'test1.png' })).toBeVisible();
  });

  test('should save the matches as a collection that appears in the sidebar', async ({ dynamicUserPage: page }) => {
    await search(page, SEARCH_QUERIES.content);
    await page.getByText('Select all on this page').click();
    await page.getByRole('toolbar', { name: 'Bulk actions' }).getByRole('button', { name: 'Save as collection' }).click();
    const dialog = page.getByRole('dialog', { name: 'Save 2 documents as a collection' });
    const name = `Scans ${Math.random().toString(36).slice(2, 6)}`;
    await dialog.getByRole('textbox', { name: /Name/ }).fill(name);
    await dialog.getByRole('button', { name: 'Save' }).click();
    await expect(page.getByText(`2 documents saved to “${name}”`)).toBeVisible();
    const link = page.getByRole('link', { name: new RegExp(name) }).first();
    await expect(link).toBeVisible();
    await link.click();
    await expect(page.getByRole('heading', { level: 1, name })).toBeVisible();
  });

  test('should preserve search state on page reload', async ({ dynamicUserPage: page }) => {
    await search(page, SEARCH_QUERIES.content);
    await expect(results(page)).toHaveCount(2);

    await page.reload();
    await expect(searchBox(page)).toHaveValue(SEARCH_QUERIES.content);
    await expect(results(page)).toHaveCount(2);
  });

  test('should sort search results by best match', async ({ dynamicUserPage: page }) => {
    await search(page, SEARCH_QUERIES.content);
    await page.getByRole('radio', { name: 'Best match' }).click();
    await expect(page).toHaveURL(/sort=relevance/);
    await expect(page.getByRole('main').getByRole('heading', { level: 2 })).toHaveCount(0);
    await expect(results(page)).toHaveCount(2);
  });

  // Needs more matches than the smallest page (25); pagination is covered by library.spec.ts.
  test.skip('should paginate search results', async () => {});

  test('should highlight search terms in results', async ({ dynamicUserPage: page }) => {
    await search(page, 'some text');
    await expect(page.getByRole('main').locator('mark').first()).toBeVisible({ timeout: TIMEOUTS.medium });
  });

  test('should clear search results', async ({ dynamicUserPage: page }) => {
    await search(page, 'text 3');
    await expect(results(page)).toHaveCount(1);

    await page.getByRole('button', { name: 'Clear' }).first().click();
    await expect(page).not.toHaveURL(/q=/);
    await expect(searchBox(page)).toHaveValue('');
    await expect(page.getByRole('heading', { name: 'Find anything in your documents' })).toBeVisible();
  });
});
