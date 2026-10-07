import { test, expect } from './fixtures/auth';
import type { Page } from '@playwright/test';
import { TEST_FILES, TIMEOUTS } from './utils/test-data';
import { TestHelpers } from './utils/test-helpers';

/**
 * Library list + the document drawer. Every test signs in as a fresh user and seeds its own
 * document through the API, so nothing depends on data left by other tests.
 */
test.describe('Document Management', () => {
  let helpers: TestHelpers;
  let docId: string;

  test.beforeEach(async ({ dynamicUserPage }) => {
    helpers = new TestHelpers(dynamicUserPage);
    docId = await helpers.uploadDocumentViaAPI(TEST_FILES.test1);
    await helpers.waitForOCRComplete(docId);
  });

  /** Opens the seeded document's drawer from the Library and returns it. */
  const openDocument = async (page: Page) => {
    await page.goto('/documents');
    const drawer = await helpers.openDocumentCard('test1.png');
    await expect(page).toHaveURL(new RegExp(`/documents\\?document=${docId}`), { timeout: TIMEOUTS.medium });
    await expect(drawer.getByRole('group', { name: 'Document summary' })).toBeVisible();
    return drawer;
  };

  test('should display document list', async ({ dynamicUserPage: page }) => {
    await page.goto('/documents');

    await expect(page.getByRole('heading', { level: 1, name: 'Library' })).toBeVisible();
    await expect(page.getByRole('searchbox', { name: 'Search documents' })).toBeVisible();

    // The default layout lists it as a card, under a month heading carrying the month's count
    await expect(helpers.documentCard('test1.png')).toBeVisible();
    await expect(page.getByRole('main').getByRole('heading', { level: 2 }).getByText('1 document', { exact: true })).toBeVisible();

    // The table has no month headings, so the only count left is the Library total
    await helpers.useLibraryView('table');
    await expect(page.getByText('1 document', { exact: true })).toBeVisible();
    const row = helpers.documentRows().filter({ hasText: 'test1.png' });
    await expect(row).toBeVisible();
    await expect(row.getByRole('gridcell', { name: 'PNG', exact: true })).toBeVisible();
    await expect(row.getByRole('gridcell', { name: 'Indexed', exact: true })).toBeVisible();
  });

  test('should open the document in a drawer that lives in the URL', async ({ dynamicUserPage: page }) => {
    const drawer = await openDocument(page);
    // A reload or a shared link reopens it.
    await page.reload();
    await expect(page.getByRole('dialog', { name: 'test1.png' })).toBeVisible({ timeout: TIMEOUTS.medium });
    // Closing keeps you on the Library; Back reopens the drawer.
    await page.getByRole('dialog').getByRole('button', { name: 'Close', exact: true }).first().click();
    await expect(page).toHaveURL(/\/documents$/);
    await expect(drawer).toBeHidden();
    // An old /documents/<id> link lands on the same drawer.
    await page.goto(`/documents/${docId}`);
    await expect(page).toHaveURL(new RegExp(`/documents\\?document=${docId}`));
    await expect(page.getByRole('dialog', { name: 'test1.png' })).toBeVisible({ timeout: TIMEOUTS.medium });
  });

  test('should display document metadata', async ({ dynamicUserPage: page }) => {
    const drawer = await openDocument(page);

    // One facts line under the name: type, size, source, when it was added and the OCR score.
    const summary = drawer.getByRole('group', { name: 'Document summary' });
    await expect(summary).toContainText('Indexed');
    await expect(summary).toContainText('PNG');
    await expect(summary).toContainText(/KB/);
    await expect(summary).toContainText('Upload');
    await expect(summary).toContainText(/Added /);
    await expect(summary).toContainText(/OCR \d+%/);
    await expect(summary).not.toContainText('—');

    // The rest sits in the Details tab.
    await drawer.getByRole('tab', { name: 'Details' }).click();
    await expect(drawer.getByRole('region', { name: 'Details' }).getByText('SHA-256')).toBeVisible();
  });

  test('should keep the file above the Text, Details, Comments and Share links tabs', async ({ dynamicUserPage: page }) => {
    const drawer = await openDocument(page);
    await expect(drawer.getByRole('region', { name: 'Preview' })).toBeVisible();
    const tabs = drawer.getByRole('tablist', { name: 'About this document' });
    await expect(tabs.getByRole('tab', { name: 'Text' })).toHaveAttribute('aria-selected', 'true');
    await expect(drawer.getByRole('region', { name: 'Extracted text' })).toBeVisible();
    await tabs.getByRole('tab', { name: 'Share links' }).click();
    await expect(drawer.getByRole('button', { name: 'Create link' })).toBeVisible();
    await tabs.getByRole('tab', { name: /Comments/ }).click();
    await expect(drawer.getByRole('textbox', { name: 'New comment' })).toBeVisible();
    // The preview stays put whichever tab is chosen.
    await expect(drawer.getByRole('region', { name: 'Preview' })).toBeVisible();
  });

  test('should allow document download', async ({ dynamicUserPage: page }) => {
    const drawer = await openDocument(page);

    const downloadPromise = page.waitForEvent('download');
    await drawer.getByRole('button', { name: 'Download' }).click();
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toBe('test1.png');
  });

  test('should allow document deletion', async ({ dynamicUserPage: page }) => {
    const drawer = await openDocument(page);

    await drawer.getByRole('button', { name: 'More actions' }).click();
    await page.getByRole('menuitem', { name: 'Delete document' }).click();

    const confirm = page.getByRole('alertdialog', { name: 'Delete this document?' });
    await expect(confirm).toBeVisible();
    const deleted = helpers.waitForApiCall(`/api/documents/${docId}`);
    await confirm.getByRole('button', { name: 'Delete', exact: true }).click();
    expect((await deleted).ok()).toBe(true);

    await expect(page).toHaveURL(/\/documents$/, { timeout: TIMEOUTS.medium });
    await expect(drawer).toBeHidden();
    await expect(page.getByText('0 documents', { exact: true })).toBeVisible({ timeout: TIMEOUTS.medium });
    await expect(helpers.documentCard('test1.png')).toHaveCount(0);
  });

  test('should filter documents by type', async ({ dynamicUserPage: page }) => {
    await helpers.uploadBufferViaAPI('notes.txt', Buffer.from(`plain text ${Date.now()}`), 'text/plain');
    await page.goto('/documents');
    await expect(helpers.documentCards()).toHaveCount(2);

    await page.getByRole('search', { name: 'Search and filter' }).getByRole('button', { name: 'Type' }).click();
    await page.getByRole('dialog', { name: 'Type' }).getByText('Images', { exact: true }).click();
    await expect(page).toHaveURL(/type=image/);
    await page.keyboard.press('Escape');

    await expect(helpers.documentCards()).toHaveCount(1);
    await expect(helpers.documentCard('test1.png')).toBeVisible();
  });

  test('should sort documents', async ({ dynamicUserPage: page }) => {
    await helpers.uploadBufferViaAPI('big.txt', Buffer.from('x'.repeat(20000) + Date.now()), 'text/plain');
    await page.goto('/documents');
    // Sorting by a column is a table feature
    await helpers.useLibraryView('table');
    await expect(helpers.documentRows()).toHaveCount(2);

    const size = helpers.documentsGrid().getByRole('columnheader', { name: 'Size' });
    await size.click();
    await expect(page).toHaveURL(/sort=file_size/);
    const firstAfterOne = await helpers.documentRows().first().getByRole('rowheader').innerText();
    await size.click();
    await expect(page).toHaveURL(/sort=file_size/);
    await expect(helpers.documentRows().first().getByRole('rowheader')).not.toHaveText(firstAfterOne);
    await expect(size).toHaveAttribute('aria-sort', /ascending|descending/);
  });

  test('should display OCR status', async ({ dynamicUserPage: page }) => {
    const drawer = await openDocument(page);
    await expect(drawer.getByRole('group', { name: 'Document summary' })).toContainText('Indexed');
  });

  test('should search within document content', async ({ dynamicUserPage: page }) => {
    await openDocument(page);
    await helpers.extractedText();

    const drawer = page.getByRole('dialog');
    await drawer.getByRole('searchbox', { name: 'Find in text' }).fill('text');
    await expect(drawer.getByRole('region', { name: 'Extracted text' }).locator('mark').first()).toBeVisible({
      timeout: TIMEOUTS.short,
    });
    await expect(drawer.getByRole('button', { name: 'Next match' })).toBeEnabled();
  });

  // Pagination across pages is covered by library.spec.ts ("should sort by name across pages"),
  // which seeds 26 documents; seeding that many here would double the suite's upload load.
  test.skip('should paginate document list', async () => {});

  test('should show document thumbnails', async ({ dynamicUserPage: page }) => {
    await page.goto('/documents');
    await expect(helpers.documentCard('test1.png').locator('img').first()).toBeVisible({ timeout: TIMEOUTS.medium });

    // The drawer shows the file itself above its tabs.
    const drawer = await helpers.openDocumentCard('test1.png');
    await expect(drawer.getByRole('region', { name: 'Preview' }).getByRole('img', { name: 'test1.png' })).toBeVisible({
      timeout: TIMEOUTS.medium,
    });
  });
});
