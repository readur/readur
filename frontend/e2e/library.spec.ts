import { test, expect } from './fixtures/auth';
import type { Page } from '@playwright/test';
import { TEST_FILES } from './utils/test-data';
import { TestHelpers, escapeRegExp } from './utils/test-helpers';

/**
 * Library (/documents): URL-driven sort and filters, row → detail slideout, keyboard walk.
 * Each test signs in as a fresh regular user, so the Library holds only what the test seeds.
 */

/** Seed text documents with unique content (identical files would be de-duplicated). */
async function seedTextDocuments(helpers: TestHelpers, names: string[]): Promise<string[]> {
  const ids: string[] = [];
  for (const name of names) {
    ids.push(await helpers.uploadBufferViaAPI(name, Buffer.from(`Seeded document ${name} ${Math.random()}`), 'text/plain'));
  }
  return ids;
}

const rowNames = async (helpers: TestHelpers) =>
  (await helpers.documentRows().getByRole('rowheader').allInnerTexts()).map((t) => t.split('\n')[0].trim());

async function openLibrary(page: Page, query = '') {
  await page.goto(`/documents${query}`);
  await expect(page.getByRole('heading', { level: 1, name: 'Library' })).toBeVisible();
}

test.describe('Library', () => {
  test('should sort by name across pages', async ({ dynamicUserPage: page }) => {
    test.setTimeout(120000);
    const helpers = new TestHelpers(page);
    const run = Math.random().toString(36).slice(2, 6);
    const names = Array.from({ length: 26 }, (_, i) => `sort-${String(i).padStart(2, '0')}-${run}.txt`);
    // Upload in a shuffled order so creation order differs from name order
    const shuffled = [...names].sort(() => Math.random() - 0.5);
    await seedTextDocuments(helpers, shuffled);

    await openLibrary(page, '?size=25');
    await expect(page.getByText('26 documents')).toBeVisible();
    await expect(helpers.documentRows()).toHaveCount(25);

    // Sort by name from the column header; the sort lives in the URL
    const nameHeader = helpers.documentsGrid().getByRole('columnheader', { name: 'Name' });
    await nameHeader.click();
    await expect(page).toHaveURL(/sort=filename/);
    if (/order=desc/.test(page.url())) {
      await nameHeader.click();
    }
    await expect(page).toHaveURL(/sort=filename&order=asc|order=asc.*sort=filename/);
    await expect(helpers.documentRows().first()).toContainText(names[0]);
    expect(await rowNames(helpers)).toEqual(names.slice(0, 25));

    // The server sorts: page 2 holds the alphabetically last document
    await page.getByRole('navigation', { name: 'Pagination' }).getByRole('button', { name: 'Next page' }).click();
    await expect(page).toHaveURL(/page=2/);
    await expect(helpers.documentRows()).toHaveCount(1);
    await expect(helpers.documentRows().first()).toContainText(names[25]);

    // Descending puts it first, and resets to page 1
    await nameHeader.click();
    await expect(page).toHaveURL(/order=desc/);
    await expect(page).not.toHaveURL(/page=2/);
    await expect(helpers.documentRows().first()).toContainText(names[25]);

    // Reloading keeps the sort
    await page.reload();
    await expect(helpers.documentRows().first()).toContainText(names[25]);
  });

  test('should update the URL from a filter chip', async ({ dynamicUserPage: page }) => {
    const helpers = new TestHelpers(page);
    const run = Math.random().toString(36).slice(2, 6);
    await seedTextDocuments(helpers, [`chip-a-${run}.txt`, `chip-b-${run}.txt`]);
    await helpers.uploadDocumentViaAPI(TEST_FILES.test1);

    await openLibrary(page);
    await expect(helpers.documentRows()).toHaveCount(3);

    const filters = page.getByRole('search', { name: 'Search and filter' });
    await filters.getByRole('button', { name: 'Type' }).click();
    await page.getByRole('dialog', { name: 'Type' }).getByText('Text', { exact: true }).click();
    await expect(page).toHaveURL(/[?&]type=text/);
    await page.keyboard.press('Escape');

    // Only the text documents remain, and the chip shows the value
    await expect(helpers.documentRows()).toHaveCount(2);
    await expect(helpers.documentRows().filter({ hasText: 'test1.png' })).toHaveCount(0);
    await expect(filters.getByRole('button', { name: /Type.*Text/ })).toBeVisible();

    // Status chip adds a second param
    await filters.getByRole('button', { name: 'Status' }).click();
    await page.getByRole('dialog', { name: 'Status' }).getByText('Failed', { exact: true }).click();
    await expect(page).toHaveURL(/[?&]status=failed/);
    await expect(page).toHaveURL(/[?&]type=text/);

    // Back restores the previous filter state
    await page.goBack();
    await expect(page).not.toHaveURL(/status=/);
    await expect(helpers.documentRows()).toHaveCount(2);
  });

  test('should open the slideout on row click, walk with ↓ and close with Esc', async ({ dynamicUserPage: page }) => {
    const helpers = new TestHelpers(page);
    const run = Math.random().toString(36).slice(2, 6);
    await seedTextDocuments(helpers, [`walk-a-${run}.txt`, `walk-b-${run}.txt`, `walk-c-${run}.txt`]);

    await openLibrary(page);
    await expect(helpers.documentRows()).toHaveCount(3);
    const [first, second] = await rowNames(helpers);

    // A row click opens the detail panel for that document
    const firstRow = helpers.documentRows().filter({ hasText: first });
    await firstRow.getByRole('rowheader').click();
    const panel = page.getByRole('dialog', { name: new RegExp(escapeRegExp(first)) });
    await expect(panel).toBeVisible();
    await expect(panel.getByRole('group', { name: 'Document facts' })).toBeVisible();
    await expect(panel.getByRole('button', { name: 'Open', exact: true })).toBeVisible();

    // ↓ moves to the next document on the page
    await page.keyboard.press('ArrowDown');
    await expect(page.getByRole('dialog', { name: new RegExp(escapeRegExp(second)) })).toBeVisible();
    await expect(page.getByRole('dialog', { name: new RegExp(escapeRegExp(first)) })).toHaveCount(0);

    // Esc closes it and returns focus to the row it was opened from
    await page.keyboard.press('Escape');
    await expect(page.getByRole('dialog')).toHaveCount(0);
    const focusedRowText = await page.evaluate(() => document.activeElement?.closest('[role="row"]')?.textContent ?? '');
    expect(focusedRowText).toContain(first);
  });

  test('should open a document from the slideout', async ({ dynamicUserPage: page }) => {
    const helpers = new TestHelpers(page);
    const id = await helpers.uploadDocumentViaAPI(TEST_FILES.test1);

    await openLibrary(page);
    await helpers.documentRows().filter({ hasText: 'test1.png' }).getByRole('rowheader').click();
    await page.getByRole('dialog').getByRole('button', { name: 'Open', exact: true }).click();

    await expect(page).toHaveURL(new RegExp(`/documents/${id}`));
    await expect(page.getByRole('heading', { level: 1, name: 'test1.png' })).toBeVisible();
  });
});
