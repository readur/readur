import { test, expect } from './fixtures/auth';
import type { Page } from '@playwright/test';
import { TEST_FILES, TIMEOUTS } from './utils/test-data';
import { TestHelpers } from './utils/test-helpers';

/**
 * Library list + document page. Every test signs in as a fresh user and seeds its own
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

  const openDocument = async (page: Page) => {
    await page.goto('/documents');
    await helpers.documentRows().filter({ hasText: 'test1.png' }).getByRole('rowheader').click();
    await page.getByRole('dialog').getByRole('button', { name: 'Open', exact: true }).click();
    await expect(page).toHaveURL(new RegExp(`/documents/${docId}`), { timeout: TIMEOUTS.medium });
    await expect(page.getByRole('heading', { level: 1, name: 'test1.png' })).toBeVisible();
  };

  test('should display document list', async ({ dynamicUserPage: page }) => {
    await page.goto('/documents');

    await expect(page.getByRole('heading', { level: 1, name: 'Library' })).toBeVisible();
    await expect(page.getByText('1 document', { exact: true })).toBeVisible();
    await expect(page.getByRole('searchbox', { name: 'Search documents' })).toBeVisible();

    const row = helpers.documentRows().filter({ hasText: 'test1.png' });
    await expect(row).toBeVisible();
    await expect(row.getByRole('gridcell', { name: 'PNG', exact: true })).toBeVisible();
    await expect(row.getByRole('gridcell', { name: 'INDEXED', exact: true })).toBeVisible();
  });

  test('should navigate to document details', async ({ dynamicUserPage: page }) => {
    await openDocument(page);
    await expect(page.getByRole('navigation', { name: 'Breadcrumb' }).getByRole('link', { name: 'Library' })).toBeVisible();
  });

  test('should display document metadata', async ({ dynamicUserPage: page }) => {
    await openDocument(page);

    // One facts line under the name: type, size, source, when it was added and the OCR score.
    const summary = page.getByRole('group', { name: 'Document summary' });
    await expect(summary).toContainText('INDEXED');
    await expect(summary).toContainText('PNG');
    await expect(summary).toContainText(/KB/);
    await expect(summary).toContainText('Upload');
    await expect(summary).toContainText(/Added /);
    await expect(summary).toContainText(/OCR \d+%/);
    await expect(summary).not.toContainText('—');

    // The rest sits behind the Details disclosure.
    await page.getByRole('button', { name: 'Details' }).click();
    await expect(page.getByRole('region', { name: 'Details' }).getByText('SHA-256')).toBeVisible();
  });

  test('should switch between document, side by side and text views', async ({ dynamicUserPage: page }) => {
    await openDocument(page);
    const views = page.getByRole('radiogroup', { name: 'View' });
    await views.getByRole('radio', { name: 'Text' }).click();
    await expect(page.getByRole('region', { name: 'Extracted text' })).toBeVisible();
    await expect(page.getByRole('region', { name: 'Document', exact: true })).toBeHidden();

    // The choice is remembered on the next document visit.
    await page.reload();
    await expect(views.getByRole('radio', { name: 'Text' })).toBeChecked();
    await views.getByRole('radio', { name: 'Document' }).click();
    await expect(page.getByRole('region', { name: 'Document', exact: true })).toBeVisible();
    await expect(page.getByRole('region', { name: 'Extracted text' })).toBeHidden();

    // The page itself does not scroll: the reading area fills the window.
    const overflow = await page.evaluate(() => document.documentElement.scrollHeight - window.innerHeight);
    expect(overflow).toBeLessThanOrEqual(1);
  });

  test('should allow document download', async ({ dynamicUserPage: page }) => {
    await openDocument(page);

    const downloadPromise = page.waitForEvent('download');
    await page.getByRole('main').getByRole('button', { name: 'Download' }).click();
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toBe('test1.png');
  });

  test('should allow document deletion', async ({ dynamicUserPage: page }) => {
    await openDocument(page);

    await page.getByRole('button', { name: 'More actions' }).click();
    await page.getByRole('menuitem', { name: 'Delete document' }).click();

    const confirm = page.getByRole('alertdialog', { name: 'Delete this document?' });
    await expect(confirm).toBeVisible();
    const deleted = helpers.waitForApiCall(`/api/documents/${docId}`);
    await confirm.getByRole('button', { name: 'Delete', exact: true }).click();
    expect((await deleted).ok()).toBe(true);

    await expect(page).toHaveURL(/\/documents(\?|$)/, { timeout: TIMEOUTS.medium });
    await expect(helpers.documentRows().filter({ hasText: 'test1.png' })).toHaveCount(0);
  });

  test('should filter documents by type', async ({ dynamicUserPage: page }) => {
    await helpers.uploadBufferViaAPI('notes.txt', Buffer.from(`plain text ${Date.now()}`), 'text/plain');
    await page.goto('/documents');
    await expect(helpers.documentRows()).toHaveCount(2);

    await page.getByRole('search', { name: 'Search and filter' }).getByRole('button', { name: 'Type' }).click();
    await page.getByRole('dialog', { name: 'Type' }).getByText('Images', { exact: true }).click();
    await expect(page).toHaveURL(/type=image/);
    await page.keyboard.press('Escape');

    await expect(helpers.documentRows()).toHaveCount(1);
    await expect(helpers.documentRows().first()).toContainText('test1.png');
  });

  test('should sort documents', async ({ dynamicUserPage: page }) => {
    await helpers.uploadBufferViaAPI('big.txt', Buffer.from('x'.repeat(20000) + Date.now()), 'text/plain');
    await page.goto('/documents');
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
    await openDocument(page);
    await expect(page.getByRole('group', { name: 'Document summary' })).toContainText('INDEXED');
  });

  test('should search within document content', async ({ dynamicUserPage: page }) => {
    await openDocument(page);
    await helpers.extractedText();

    await page.getByRole('searchbox', { name: 'Find in text' }).fill('text');
    await expect(page.getByRole('region', { name: 'Extracted text' }).locator('mark').first()).toBeVisible({
      timeout: TIMEOUTS.short,
    });
    await expect(page.getByRole('button', { name: 'Next match' })).toBeEnabled();
  });

  // Pagination across pages is covered by library.spec.ts ("should sort by name across pages"),
  // which seeds 26 documents; seeding that many here would double the suite's upload load.
  test.skip('should paginate document list', async () => {});

  test('should show document thumbnails', async ({ dynamicUserPage: page }) => {
    await page.goto('/documents');
    await helpers.documentRows().filter({ hasText: 'test1.png' }).getByRole('rowheader').click();
    const panel = page.getByRole('dialog');
    await expect(panel.locator('img').first()).toBeVisible({ timeout: TIMEOUTS.medium });

    await panel.getByRole('button', { name: 'Open', exact: true }).click();
    await expect(page.getByRole('img', { name: 'test1.png' })).toBeVisible({ timeout: TIMEOUTS.medium });
  });
});
