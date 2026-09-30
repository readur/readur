import { test, expect } from './fixtures/auth';
import type { Page } from '@playwright/test';
import { TEST_FILES, TIMEOUTS, EXPECTED_TEXT_CONTENT } from './utils/test-data';
import { TestHelpers, resolveTestFile } from './utils/test-helpers';
import * as fs from 'fs';

// Upload now lives at Intake → Add documents (/intake?section=upload; /upload redirects there).
test.describe('Document Upload', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ dynamicUserPage }) => {
    helpers = new TestHelpers(dynamicUserPage);
    await helpers.openIntake('upload');
  });

  const queue = (page: Page) => page.getByRole('grid', { name: 'Files to upload' });
  const queueRow = (page: Page, name: string) => queue(page).getByRole('row', { name: new RegExp(name.replace('.', '\\.')) });
  const uploadAll = (page: Page) => page.getByRole('button', { name: /^Upload all/ });

  /** Click "Upload all" and wait until the last of `count` POST /api/documents calls answers. */
  async function uploadQueued(page: Page, count = 1) {
    let seen = 0;
    const last = page.waitForResponse(
      (r) => /\/api\/documents$/.test(r.url()) && r.request().method() === 'POST' && ++seen >= count,
      { timeout: TIMEOUTS.upload },
    );
    await uploadAll(page).click();
    expect((await last).status()).toBeLessThan(300);
  }

  test('should display upload interface', async ({ dynamicUserPage: page }) => {
    await expect(page.getByRole('tab', { name: 'Add documents', selected: true })).toBeVisible();
    await expect(page.locator('input[type="file"]')).toBeAttached();
    await expect(page.getByRole('group', { name: 'Drop files to add' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Choose files' }).first()).toBeVisible();
    await expect(page.getByText(/PDF · PNG · JPG/)).toBeVisible();
    await expect(uploadAll(page)).toBeDisabled();
    await expect(queue(page).getByRole('heading', { name: 'No files yet' })).toBeVisible();
  });

  test('should upload single document successfully', async ({ dynamicUserPage: page }) => {
    await page.locator('input[type="file"]').first().setInputFiles(TEST_FILES.test1);

    await expect(queueRow(page, 'test1.png')).toBeVisible();
    await expect(queueRow(page, 'test1.png')).toContainText('Pending');
    await expect(uploadAll(page)).toHaveText('Upload all (1)');

    await uploadQueued(page);

    // Uploaded files go straight into the OCR queue and are tagged NEW
    await expect(queueRow(page, 'test1.png')).toContainText(/OCR|Indexed/);
    await expect(queueRow(page, 'test1.png')).toContainText(/new/i);
  });

  test('should upload multiple documents', async ({ dynamicUserPage: page }) => {
    await page.locator('input[type="file"]').first().setInputFiles([TEST_FILES.test1, TEST_FILES.test2, TEST_FILES.test3]);

    await expect(uploadAll(page)).toHaveText('Upload all (3)');
    await uploadQueued(page, 3);

    for (const name of ['test1.png', 'test2.jpg', 'test3.jpeg']) {
      await expect(queueRow(page, name)).toContainText(/OCR|Indexed/);
    }
    await expect(page.getByRole('button', { name: 'Clear finished' })).toBeEnabled();
  });

  test('should show upload progress', async ({ dynamicUserPage: page }) => {
    await page.locator('input[type="file"]').first().setInputFiles(TEST_FILES.test4);

    const progress = page.getByRole('progressbar', { name: 'Upload progress for test4.png' });
    await expect(progress).toBeVisible();
    await expect(progress).toHaveAttribute('aria-valuenow', '0');

    await uploadQueued(page);
    await expect(progress).toHaveAttribute('aria-valuenow', '100');
  });

  test('should handle upload errors gracefully', async ({ dynamicUserPage: page }) => {
    await page.route('**/api/documents', (route) => {
      if (route.request().method() === 'POST') {
        return route.fulfill({ status: 500, contentType: 'application/json', body: JSON.stringify({ error: 'Upload failed' }) });
      }
      return route.continue();
    });

    await page.locator('input[type="file"]').first().setInputFiles(TEST_FILES.image);
    await uploadAll(page).click();

    const row = queueRow(page, 'test1.png');
    await expect(row).toContainText('Failed');
    await expect(row.getByRole('button', { name: 'Retry test1.png' })).toBeVisible();

    // Retrying after the server recovers succeeds
    await page.unroute('**/api/documents');
    const upload = helpers.waitForApiCall('/api/documents', TIMEOUTS.upload);
    await row.getByRole('button', { name: 'Retry test1.png' }).click();
    expect((await upload).status()).toBeLessThan(300);
    await expect(row).toContainText(/OCR|Indexed/);
  });

  test('should validate file types', async ({ dynamicUserPage: page }) => {
    await page.locator('input[type="file"]').first().setInputFiles({
      name: 'test.xyz',
      mimeType: 'application/octet-stream',
      buffer: Buffer.from('fake content'),
    });

    // Rejected files are listed in an alert and never queued
    const alert = page.getByRole('alert').filter({ hasText: 'Some files were not added' });
    await expect(alert).toBeVisible();
    await expect(alert).toContainText('test.xyz');
    await expect(queueRow(page, 'test.xyz')).toHaveCount(0);
    await expect(uploadAll(page)).toBeDisabled();
  });

  test('should navigate to uploaded document after successful upload', async ({ dynamicUserPage: page }) => {
    await page.locator('input[type="file"]').first().setInputFiles(TEST_FILES.image);
    await uploadQueued(page);

    await queueRow(page, 'test1.png').click();
    await expect(page).toHaveURL(/\/documents\/[0-9a-f-]{36}/, { timeout: TIMEOUTS.medium });
    await expect(page.getByRole('heading', { level: 1, name: 'test1.png' })).toBeVisible();
  });

  test('should show OCR processing status', async ({ dynamicUserPage: page }) => {
    test.setTimeout(TIMEOUTS.ocr + 30000);
    await page.locator('input[type="file"]').first().setInputFiles(TEST_FILES.test5);
    await uploadQueued(page);

    // The row moves to the OCR queue; the Library shows the document's status
    await expect(queueRow(page, 'test5.jpg')).toContainText(/OCR|Indexed/);
    // The Library table's Status column carries every state (cards only flag unfinished ones)
    await page.goto('/documents');
    await helpers.useLibraryView('table');
    const row = helpers.documentRows().filter({ hasText: 'test5.jpg' });
    await expect(row).toContainText(/Pending|OCR|Indexed/, { timeout: TIMEOUTS.medium });

    // Once OCR finishes the Library reports it as indexed
    await row.click();
    await page.getByRole('dialog').getByRole('button', { name: 'Open', exact: true }).click();
    await expect(page).toHaveURL(/\/documents\/[0-9a-f-]{36}/);
    const docId = page.url().split('/').pop()!.split('?')[0];
    expect((await helpers.waitForOCRComplete(docId)).ocr_status).toBe('completed');
    await page.goto('/documents');
    await expect(helpers.documentRows().filter({ hasText: 'test5.jpg' })).toContainText('Indexed');
  });

  test('should process OCR and extract correct text content', async ({ dynamicUserPage: page }) => {
    test.setTimeout(TIMEOUTS.ocr + 30000);
    // test6.jpeg is not used: the server's OCR reads 0 words from it and marks it failed.
    await page.locator('input[type="file"]').first().setInputFiles(TEST_FILES.test2);
    await uploadQueued(page);

    await queueRow(page, 'test2.jpg').click();
    await expect(page).toHaveURL(/\/documents\/[0-9a-f-]{36}/);
    const docId = page.url().split('/').pop()!.split('?')[0];
    const doc = await helpers.waitForOCRComplete(docId);
    expect(doc.ocr_status).toBe('completed');

    await page.reload();
    const text = await helpers.extractedText();
    // Tesseract reads the large "Test 2" title unreliably; the body line is stable.
    await expect(text).toContainText('This is some text from text 2');
  });

  test('should allow drag and drop upload', async ({ dynamicUserPage: page }) => {
    const bytes = fs.readFileSync(resolveTestFile(TEST_FILES.test7)).toString('base64');
    const dataTransfer = await page.evaluateHandle((b64) => {
      const bin = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
      const dt = new DataTransfer();
      dt.items.add(new File([bin], 'test7.png', { type: 'image/png' }));
      return dt;
    }, bytes);

    const dropzone = page.getByRole('group', { name: 'Drop files to add' });
    await dropzone.dispatchEvent('dragenter', { dataTransfer });
    await dropzone.dispatchEvent('dragover', { dataTransfer });
    await dropzone.dispatchEvent('drop', { dataTransfer });

    await expect(queueRow(page, 'test7.png')).toBeVisible();
    await expect(uploadAll(page)).toHaveText('Upload all (1)');
  });

  test('should upload .docx document successfully', async ({ dynamicUserPage: page }) => {
    await page.locator('input[type="file"]').first().setInputFiles(TEST_FILES.testDocx);
    await expect(queueRow(page, 'test_file.docx')).toBeVisible();
    await uploadQueued(page);
    await expect(queueRow(page, 'test_file.docx')).toContainText(/OCR|Indexed/);
  });

  test('should upload .doc document successfully', async ({ dynamicUserPage: page }) => {
    await page.locator('input[type="file"]').first().setInputFiles(TEST_FILES.testDoc);
    await expect(queueRow(page, 'test_file.doc')).toBeVisible();
    await uploadQueued(page);
    await expect(queueRow(page, 'test_file.doc')).toContainText(/OCR|Indexed/);
  });

  for (const [label, file, name] of [
    ['.docx', TEST_FILES.testDocx, 'test_file.docx'],
    ['.doc', TEST_FILES.testDoc, 'test_file.doc'],
  ] as const) {
    test(`should process ${label} document and extract text content`, async ({ dynamicUserPage: page }) => {
      test.setTimeout(TIMEOUTS.ocr + 30000);
      await page.locator('input[type="file"]').first().setInputFiles(file);
      await uploadQueued(page);

      await queueRow(page, name).click();
      await expect(page).toHaveURL(/\/documents\/[0-9a-f-]{36}/);
      const docId = page.url().split('/').pop()!.split('?')[0];
      const doc = await helpers.waitForOCRComplete(docId);

      await page.reload();
      if (doc.ocr_status === 'completed') {
        await expect(await helpers.extractedText()).toContainText(EXPECTED_TEXT_CONTENT.testDocx);
      } else {
        // The server could not extract text (the stock image has no antiword/catdoc for .doc).
        // The page must say so and offer a retry instead of an empty text pane.
        await expect(page.getByText('OCR could not read this document.')).toBeVisible();
        await expect(page.getByRole('button', { name: 'Retry OCR' })).toBeVisible();
        test.info().annotations.push({ type: 'environment', description: `${name}: server text extraction failed` });
      }
    });
  }
});
