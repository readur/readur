import { test, expect } from './fixtures/auth';
import type { Page } from '@playwright/test';
import { SEARCH_QUERIES, TEST_FILES, TIMEOUTS } from './utils/test-data';
import { TestHelpers } from './utils/test-helpers';

/**
 * Search is part of the Library: /documents?q=… (the old /search URL redirects there).
 * Each test signs in as a fresh user and seeds OCR'd images with known text.
 */
test.describe('Search Functionality', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ dynamicUserPage }) => {
    helpers = new TestHelpers(dynamicUserPage);
    // test1.png → "Test 1 / This is some text from text 1", test3.jpeg → "... text 3"
    const ids = [await helpers.uploadDocumentViaAPI(TEST_FILES.test1), await helpers.uploadDocumentViaAPI(TEST_FILES.test3)];
    for (const id of ids) await helpers.waitForOCRComplete(id);
    await dynamicUserPage.goto('/search');
    await expect(dynamicUserPage).toHaveURL(/\/documents/);
  });

  const searchBox = (page: Page) => page.getByRole('searchbox', { name: 'Search documents' });

  async function search(page: Page, q: string) {
    const box = searchBox(page);
    await box.fill(q);
    const response = helpers.waitForApiCall('/api/search', TIMEOUTS.medium);
    await box.press('Enter');
    await response;
    await expect(page).toHaveURL(new RegExp(`[?&]q=${encodeURIComponent(q).replace(/%20/g, '(\\+|%20)')}`));
  }

  test('should display search interface', async ({ dynamicUserPage: page }) => {
    await expect(page.getByRole('heading', { level: 1, name: 'Library' })).toBeVisible();
    await expect(searchBox(page)).toBeVisible();
    await expect(searchBox(page)).toHaveAttribute('placeholder', 'Search names and text…');
    await expect(page.getByRole('button', { name: 'Search help' })).toBeVisible();
  });

  test('should perform basic search', async ({ dynamicUserPage: page }) => {
    await search(page, SEARCH_QUERIES.content); // "some text from text"

    await expect(helpers.documentRows()).toHaveCount(2);
    await expect(helpers.documentRows().filter({ hasText: 'test1.png' })).toBeVisible();
    await expect(helpers.documentRows().filter({ hasText: 'test3.jpeg' })).toBeVisible();
  });

  test('should show search suggestions', async ({ dynamicUserPage: page }) => {
    // Type-ahead suggestions moved from the search bar to the ⌘K palette
    await page.getByRole('button', { name: 'Search documents' }).click();
    const palette = page.getByRole('dialog', { name: 'Command palette' });
    await palette.getByRole('searchbox', { name: 'Search' }).pressSequentially('some text', { delay: 50 });
    const documents = palette.getByRole('group', { name: 'Documents' });
    await expect(documents.getByRole('menuitem', { name: /test1\.png/ })).toBeVisible({ timeout: TIMEOUTS.short });
    await expect(documents.getByRole('menuitem', { name: /Show all results/ })).toBeVisible();
  });

  test('should filter search results', async ({ dynamicUserPage: page }) => {
    await search(page, SEARCH_QUERIES.content);
    await expect(helpers.documentRows()).toHaveCount(2);

    await page.getByRole('search', { name: 'Search and filter' }).getByRole('button', { name: 'Type' }).click();
    await page.getByRole('dialog', { name: 'Type' }).getByText('PDF', { exact: true }).click();
    await expect(page).toHaveURL(/type=pdf/);
    await expect(page).toHaveURL(/q=/);
    await page.keyboard.press('Escape');

    // No PDFs match: the empty state offers to clear the filters
    await expect(page.getByText('No matches')).toBeVisible();
    await page.getByRole('button', { name: 'Clear filters' }).click();
    await expect(page).not.toHaveURL(/type=/);
  });

  test('should perform advanced search', async ({ dynamicUserPage: page }) => {
    // The old title/content/date form is replaced by match modes (Search help) and filter chips
    await page.getByRole('button', { name: 'Search help' }).click();
    const help = page.getByRole('dialog', { name: 'Search help' });
    await expect(help).toBeVisible();
    await help.getByText('Exact phrase', { exact: true }).click();
    await expect(page).toHaveURL(/mode=phrase/);
    await page.keyboard.press('Escape');

    await search(page, 'text from text 3');
    await expect(helpers.documentRows()).toHaveCount(1);
    await expect(helpers.documentRows().first()).toContainText('test3.jpeg');
  });

  test('should handle empty search results', async ({ dynamicUserPage: page }) => {
    await search(page, SEARCH_QUERIES.noResults);
    await expect(page.getByText('No matches')).toBeVisible({ timeout: TIMEOUTS.medium });
    await expect(page.getByText('Nothing fits the current search and filters.')).toBeVisible();
  });

  test('should navigate to document from search results', async ({ dynamicUserPage: page }) => {
    await search(page, 'text 1');
    const row = helpers.documentRows().filter({ hasText: 'test1.png' });
    await row.getByRole('rowheader').click();
    await page.getByRole('dialog').getByRole('button', { name: 'Open', exact: true }).click();
    await expect(page).toHaveURL(/\/documents\/[0-9a-f-]{36}/, { timeout: TIMEOUTS.medium });
    await expect(page.getByRole('heading', { level: 1, name: 'test1.png' })).toBeVisible();
  });

  test('should preserve search state on page reload', async ({ dynamicUserPage: page }) => {
    await search(page, SEARCH_QUERIES.content);
    await expect(helpers.documentRows()).toHaveCount(2);

    await page.reload();
    await expect(searchBox(page)).toHaveValue(SEARCH_QUERIES.content);
    await expect(helpers.documentRows()).toHaveCount(2);
  });

  test('should sort search results', async ({ dynamicUserPage: page }) => {
    await search(page, SEARCH_QUERIES.content);
    await expect(helpers.documentRows()).toHaveCount(2);

    const name = helpers.documentsGrid().getByRole('columnheader', { name: 'Name' });
    await name.click();
    await expect(page).toHaveURL(/sort=filename/);
    await expect(page).toHaveURL(/q=/);
    const order = /order=asc/.test(page.url()) ? ['test1.png', 'test3.jpeg'] : ['test3.jpeg', 'test1.png'];
    await expect(helpers.documentRows().nth(0)).toContainText(order[0]);
    await expect(helpers.documentRows().nth(1)).toContainText(order[1]);
  });

  // Needs more matches than the smallest page (25); pagination is covered by library.spec.ts.
  test.skip('should paginate search results', async () => {});

  test('should highlight search terms in results', async ({ dynamicUserPage: page }) => {
    await search(page, 'some text');
    await expect(helpers.documentsGrid().locator('mark').first()).toBeVisible({ timeout: TIMEOUTS.medium });
  });

  test('should clear search results', async ({ dynamicUserPage: page }) => {
    await search(page, 'text 3');
    await expect(helpers.documentRows()).toHaveCount(1);

    await page.getByRole('search', { name: 'Search and filter' }).getByRole('button', { name: 'Clear' }).click();
    await expect(page).not.toHaveURL(/q=/);
    await expect(searchBox(page)).toHaveValue('');
    await expect(helpers.documentRows()).toHaveCount(2);
  });
});
