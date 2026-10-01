import { test, expect } from './fixtures/auth';
import type { Page } from '@playwright/test';
import { TEST_FILES } from './utils/test-data';
import { TestHelpers, escapeRegExp } from './utils/test-helpers';

/**
 * Library (/documents): thumbnail grid or table, URL-driven sort and filters, row → detail
 * slideout, keyboard walk, collection pages. Each test signs in as a fresh regular user, so the
 * Library holds only what the test seeds.
 */

/** Seed text documents with unique content (identical files would be de-duplicated). */
async function seedTextDocuments(helpers: TestHelpers, names: string[]): Promise<string[]> {
  const ids: string[] = [];
  for (const name of names) {
    ids.push(await helpers.uploadBufferViaAPI(name, Buffer.from(`Seeded document ${name} ${Math.random()}`), 'text/plain'));
  }
  return ids;
}

/**
 * The seeded names in row order, read from each row header's text. The header also holds the
 * type-code stub and a NEW/CHANGED tag, so each row is matched to the one seeded name it contains.
 */
const rowNames = async (helpers: TestHelpers, seeded: readonly string[]) => {
  const texts = await helpers.documentRows().getByRole('rowheader').allTextContents();
  return texts.map((text) => seeded.find((name) => text.includes(name)) ?? text.trim());
};

/** The Library in the given layout (the table unless asked otherwise). */
async function openLibrary(page: Page, query = '', view: 'table' | 'grid' = 'table') {
  await page.goto(`/documents${query}`);
  await expect(page.getByRole('heading', { level: 1, name: 'Library' })).toBeVisible();
  const layout = page.getByRole('radiogroup', { name: 'Layout' });
  const toggle = layout.getByRole('radio', { name: view === 'table' ? 'Table' : 'Grid' });
  if (!(await toggle.isChecked())) await toggle.click();
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
    expect(await rowNames(helpers, names)).toEqual(names.slice(0, 25));

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
    const seeded = [`walk-a-${run}.txt`, `walk-b-${run}.txt`, `walk-c-${run}.txt`];
    await seedTextDocuments(helpers, seeded);

    await openLibrary(page);
    await expect(helpers.documentRows()).toHaveCount(3);
    const [first, second] = await rowNames(helpers, seeded);

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
    // React Aria restores focus one animation frame after the dialog unmounts, so poll for it
    await expect
      .poll(() => page.evaluate(() => document.activeElement?.closest('[role="row"]')?.textContent ?? ''))
      .toContain(first);
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

  test('should show thumbnails by month in the grid and remember the layout', async ({ dynamicUserPage: page }) => {
    const helpers = new TestHelpers(page);
    const run = Math.random().toString(36).slice(2, 6);
    await seedTextDocuments(helpers, [`grid-a-${run}.txt`, `grid-b-${run}.txt`]);

    await openLibrary(page, '', 'grid');
    await expect(helpers.documentsGrid()).toHaveCount(0);
    // Newest first, under this month's heading
    await expect(page.getByRole('main').getByRole('heading', { level: 2 }).first()).toBeVisible();
    await expect(page.getByRole('checkbox', { name: `Select grid-a-${run}.txt` })).toBeAttached();

    // A card opens the same slideout as a row
    await page.getByRole('button', { name: new RegExp(`grid-b-${run}`) }).click();
    await expect(page.getByRole('dialog', { name: new RegExp(`grid-b-${run}`) })).toBeVisible();
    await page.keyboard.press('Escape');

    // The layout is remembered
    await page.getByRole('radiogroup', { name: 'Layout' }).getByRole('radio', { name: 'Table' }).click();
    await page.reload();
    await expect(helpers.documentsGrid()).toBeVisible();
  });

  test('should show a collection as its own page', async ({ dynamicUserPage: page }) => {
    const helpers = new TestHelpers(page);
    const run = Math.random().toString(36).slice(2, 6);
    const [id] = await seedTextDocuments(helpers, [`collected-${run}.txt`, `loose-${run}.txt`]);
    const name = `Receipts ${run}`;
    const label = await page.evaluate(async ({ name, id }) => {
      const token = localStorage.getItem('token');
      const headers = { 'Content-Type': 'application/json', ...(token ? { Authorization: `Bearer ${token}` } : {}) };
      const res = await fetch('/api/labels', { method: 'POST', headers, body: JSON.stringify({ name, color: '#28a745' }) });
      const created = await res.json();
      await fetch(`/api/labels/documents/${id}`, { method: 'PUT', headers, body: JSON.stringify({ label_ids: [created.id] }) });
      return created.id as string;
    }, { name, id });

    await page.goto(`/documents?label=${label}`);
    await expect(page.getByRole('heading', { level: 1, name })).toBeVisible();
    await expect(page.getByText(`collected-${run}.txt`)).toBeVisible();
    await expect(page.getByText(`loose-${run}.txt`)).toHaveCount(0);
  });
});
