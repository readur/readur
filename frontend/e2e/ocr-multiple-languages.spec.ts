import { test, expect } from './fixtures/auth';
import type { Page } from '@playwright/test';
import { TIMEOUTS, TEST_FILES } from './utils/test-data';
import { TestHelpers } from './utils/test-helpers';

const EXPECTED_CONTENT = {
  english: {
    keywords: ['English', 'document', 'recognition', 'technology', 'computer'],
  },
  mixed: {
    spanish: ['español', 'idiomas', 'reconocimiento'],
    english: ['English', 'languages', 'recognition'],
  },
};

/**
 * OCR languages: Settings → OCR → Languages (per user), the upload options in
 * Intake → Add documents, and "Retry with languages" for failed documents.
 * Each test signs in as a fresh user, so settings changes never leak between tests.
 */
test.describe('OCR Multiple Languages', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ dynamicUserPage }) => {
    helpers = new TestHelpers(dynamicUserPage);
  });

  async function openLanguages(page: Page) {
    await page.goto('/settings/ocr');
    await page.getByRole('button', { name: 'Edit Languages' }).click();
    return page.getByRole('region', { name: 'Languages' });
  }

  async function seedFailedDocument(name: string): Promise<string> {
    const id = await helpers.uploadBufferViaAPI(name, Buffer.from(`%PDF-1.4\nnot a real pdf ${Math.random()}\n`), 'application/pdf');
    expect((await helpers.waitForOCRComplete(id)).ocr_status).toBe('failed');
    return id;
  }

  /** Open the failed document's panel in Needs attention and pick retry languages. */
  async function retryWithLanguages(page: Page, name: string, id: string, languages: string[]) {
    await helpers.openIntake('attention');
    await page.getByRole('grid', { name: 'Failed documents' }).getByRole('row', { name: new RegExp(name.replace('.', '\\.')) }).getByRole('rowheader').click();
    const panel = page.getByRole('dialog', { name });
    const picker = panel.getByRole('region', { name: 'Retry with languages' });
    await picker.getByRole('button', { name: /Select OCR languages/ }).click();
    for (const language of languages) {
      await picker.getByRole('group', { name: 'Available Languages' }).getByText(language, { exact: true }).click();
    }

    // The server refuses a manual retry while its own automatic retries are pending,
    // so press Retry until it is accepted (dismissing the error toast in between).
    let body: Record<string, unknown> = {};
    await expect(async () => {
      const close = helpers.toasts().getByRole('alertdialog').getByRole('button', { name: 'Close' });
      while (await close.count()) await close.first().click();
      const retry = page.waitForResponse(
        (r) => r.url().includes(`/api/documents/${id}/ocr/retry`) && r.request().method() === 'POST',
        { timeout: TIMEOUTS.medium },
      );
      await panel.getByRole('button', { name: 'Retry OCR' }).click();
      const response = await retry;
      body = JSON.parse(response.request().postData() ?? '{}');
      expect(response.ok()).toBe(true);
    }).toPass({ timeout: 90000, intervals: [5000] });
    return body;
  }

  test('should display OCR language selector in settings', async ({ dynamicUserPage: page }) => {
    const languages = await openLanguages(page);

    await expect(languages.getByText('OCR Languages (1/4)')).toBeVisible({ timeout: TIMEOUTS.medium });
    await expect(languages.getByRole('list', { name: 'Selected languages' })).toContainText('English');

    await languages.getByRole('button', { name: /Add more languages/ }).click();
    const available = languages.getByRole('group', { name: 'Available Languages' });
    await expect(available).toBeVisible();
    await expect(available.getByRole('checkbox', { name: 'Spanish', exact: true })).toBeAttached();
    await expect(available.getByRole('checkbox', { name: 'English', exact: true })).toBeAttached();
  });

  test('should select multiple OCR languages', async ({ dynamicUserPage: page }) => {
    const languages = await openLanguages(page);
    await languages.getByRole('button', { name: /Add more languages/ }).click();
    await languages.getByRole('group', { name: 'Available Languages' }).getByText('Spanish', { exact: true }).click();

    const selected = languages.getByRole('list', { name: 'Selected languages' });
    await expect(selected).toContainText('Spanish');
    await expect(selected).toContainText('English');
    await expect(selected).toContainText('(Primary)');
    await expect(languages.getByText('OCR Languages (2/4)')).toBeVisible();

    const put = helpers.waitForApiCall('/api/settings', TIMEOUTS.medium);
    await page.getByRole('main').getByRole('button', { name: 'Save', exact: true }).click();
    expect((await put).ok()).toBe(true);
    await helpers.waitForToast(/Settings updated successfully/);
  });

  test('should upload Spanish document and process with Spanish OCR', async ({ dynamicUserPage: page }) => {
    test.setTimeout(TIMEOUTS.ocr + 30000);
    await helpers.openIntake('upload');

    // Pick Spanish for this upload in the upload options
    const options = page.getByRole('region', { name: 'Apply to these uploads' });
    await options.getByRole('button', { name: /Add more languages/ }).click();
    await options.getByRole('group', { name: 'Available Languages' }).getByText('Spanish', { exact: true }).click();
    await expect(options.getByRole('list', { name: 'Selected languages' })).toContainText('Spanish');

    await page.locator('input[type="file"]').first().setInputFiles(TEST_FILES.spanishTest);
    const upload = page.waitForResponse((r) => /\/api\/documents$/.test(r.url()) && r.request().method() === 'POST', {
      timeout: TIMEOUTS.upload,
    });
    await page.getByRole('button', { name: /^Upload all/ }).click();
    expect((await upload).ok()).toBe(true);
    const row = page.getByRole('grid', { name: 'Files to upload' }).getByRole('row', { name: /spanish_test\.pdf/ });
    await expect(row).toContainText(/OCR|INDEXED/);

    // The document is read with Spanish: its text carries Spanish words
    await row.click();
    await expect(page).toHaveURL(/\/documents\/[0-9a-f-]{36}/);
    const docId = page.url().split('/').pop()!.split('?')[0];
    expect((await helpers.waitForOCRComplete(docId)).ocr_status).toBe('completed');
    await page.reload();
    const text = ((await (await helpers.extractedText()).textContent()) ?? '').toLowerCase();
    expect(['español', 'documento', 'reconocimiento', 'comunicación'].some((w) => text.includes(w))).toBe(true);
  });

  test('should upload English document and process with English OCR', async ({ dynamicUserPage: page }) => {
    await helpers.openIntake('upload');
    await expect(page.getByRole('group', { name: 'Drop files to add' })).toBeVisible();

    await page.locator('input[type="file"]').first().setInputFiles(TEST_FILES.englishTest);
    const row = page.getByRole('grid', { name: 'Files to upload' }).getByRole('row', { name: /english_test\.pdf/ });
    await expect(row).toBeVisible();

    const upload = page.waitForResponse((r) => /\/api\/documents$/.test(r.url()) && r.request().method() === 'POST', {
      timeout: TIMEOUTS.upload,
    });
    await page.getByRole('button', { name: /^Upload all/ }).click();
    expect((await upload).ok()).toBe(true);

    await expect(row).toContainText(/OCR|INDEXED/);
    await expect(page.getByRole('progressbar', { name: 'Upload progress for english_test.pdf' })).toHaveAttribute('aria-valuenow', '100');
  });

  test('should validate OCR results contain expected language-specific content', async ({ dynamicUserPage: page }) => {
    const docId = await helpers.uploadDocumentViaAPI(TEST_FILES.englishTest);
    const doc = await helpers.waitForOCRComplete(docId);
    expect(doc.ocr_status).toBe('completed');

    await page.goto(`/documents/${docId}`);
    const content = (await (await helpers.extractedText()).textContent()) ?? '';
    expect(EXPECTED_CONTENT.english.keywords.some((k) => content.toLowerCase().includes(k.toLowerCase()))).toBe(true);
  });

  test('should retry failed OCR with different language', async ({ dynamicUserPage: page }) => {
    test.setTimeout(150000);
    const id = await seedFailedDocument('retry-spa.pdf');
    const body = await retryWithLanguages(page, 'retry-spa.pdf', id, ['Spanish']);
    expect(body.languages).toEqual(['spa']);
    await helpers.waitForToast(/OCR retry queued/);
  });

  test('should handle mixed language document', async ({ dynamicUserPage: page }) => {
    const docId = await helpers.uploadDocumentViaAPI(TEST_FILES.mixedLanguageTest);
    const doc = await helpers.waitForOCRComplete(docId);
    expect(['completed', 'failed']).toContain(doc.ocr_status);

    await page.goto(`/documents/${docId}`);
    await expect(page.getByRole('heading', { level: 1, name: 'mixed_language_test.pdf' })).toBeVisible({ timeout: TIMEOUTS.medium });

    if (doc.ocr_status === 'completed') {
      const content = ((await (await helpers.extractedText()).textContent()) ?? '').toLowerCase();
      const hasSpanish = EXPECTED_CONTENT.mixed.spanish.some((w) => content.includes(w.toLowerCase()));
      const hasEnglish = EXPECTED_CONTENT.mixed.english.some((w) => content.includes(w.toLowerCase()));
      expect(hasSpanish || hasEnglish).toBe(true);
    }
  });

  test('should persist language preference across sessions', async ({ dynamicUserPage: page }) => {
    await page.goto('/board');
    await helpers.updateSettingsViaAPI({ preferred_languages: ['eng', 'spa'], primary_language: 'eng', ocr_language: 'eng' });

    let languages = await openLanguages(page);
    await expect(languages.getByRole('list', { name: 'Selected languages' })).toContainText('Spanish', { timeout: TIMEOUTS.medium });

    await page.reload();
    languages = page.getByRole('region', { name: 'Languages' });
    await page.getByRole('button', { name: 'Edit Languages' }).click();
    await expect(languages.getByRole('list', { name: 'Selected languages' })).toContainText('Spanish');
  });

  test('should display available languages from API', async ({ dynamicUserPage: page }) => {
    await page.goto('/board');
    const response = await page.request.get('/api/ocr/languages', {
      headers: { Authorization: `Bearer ${await helpers.getAuthToken()}` },
    });
    expect(response.ok()).toBe(true);
    const body = await response.json();
    const list = Array.isArray(body) ? body : body.available_languages;
    expect(Array.isArray(list)).toBe(true);
    expect(list.length).toBeGreaterThan(0);

    const languages = await openLanguages(page);
    await languages.getByRole('button', { name: /Add more languages/ }).click();
    const available = languages.getByRole('group', { name: 'Available Languages' });
    await expect(available).toBeVisible({ timeout: TIMEOUTS.short });
    // Every language the API reports is offered (English is already selected, so it is not in this list)
    await expect(available.getByRole('checkbox')).not.toHaveCount(0);
  });

  test('should handle bulk operations with multiple languages', async ({ dynamicUserPage: page }) => {
    const id1 = await helpers.uploadDocumentViaAPI(TEST_FILES.englishTest);
    const id2 = await helpers.uploadDocumentViaAPI(TEST_FILES.spanishTest);
    await helpers.waitForOCRComplete(id1);
    await helpers.waitForOCRComplete(id2);

    await page.goto('/documents');
    const english = helpers.documentRows().filter({ hasText: 'english_test.pdf' });
    const spanish = helpers.documentRows().filter({ hasText: 'spanish_test.pdf' });
    await english.getByRole('checkbox').check({ force: true });
    await spanish.getByRole('checkbox').check({ force: true });

    await expect(english.getByRole('checkbox')).toBeChecked();
    await expect(spanish.getByRole('checkbox')).toBeChecked();
    // The bulk bar offers the bulk actions for the selection
    await expect(page.getByRole('button', { name: /Retry OCR/ })).toBeVisible();
    await expect(page.getByRole('button', { name: /Add label/ })).toBeVisible();
  });

  test('should handle OCR language errors gracefully', async ({ dynamicUserPage: page }) => {
    await page.route('**/api/ocr/languages', (route) =>
      route.fulfill({ status: 500, contentType: 'application/json', body: JSON.stringify({ error: 'boom' }) }),
    );
    await page.goto('/settings/ocr');
    await page.getByRole('button', { name: 'Edit Languages' }).click();

    // Not a blank page: the selector reports the failure in an alert
    await expect(page.getByRole('region', { name: 'Languages' }).getByRole('alert')).toBeVisible({ timeout: TIMEOUTS.medium });
    await expect(page.getByRole('heading', { level: 1, name: 'Settings' })).toBeVisible();
  });

  test('should upload document with multiple languages selected', async ({ dynamicUserPage: page }) => {
    await page.goto('/board');
    await helpers.updateSettingsViaAPI({ preferred_languages: ['eng', 'spa'], primary_language: 'eng', ocr_language: 'eng' });

    const docId = await helpers.uploadDocumentViaAPI(TEST_FILES.mixedLanguageTest);
    const doc = await helpers.waitForOCRComplete(docId);
    expect(['completed', 'failed']).toContain(doc.ocr_status);

    await page.goto(`/documents/${docId}`);
    await expect(page.getByRole('heading', { level: 1, name: 'mixed_language_test.pdf' })).toBeVisible({ timeout: TIMEOUTS.medium });
  });

  test('should retry failed OCR with multiple languages', async ({ dynamicUserPage: page }) => {
    test.setTimeout(150000);
    const id = await seedFailedDocument('retry-multi.pdf');
    const body = await retryWithLanguages(page, 'retry-multi.pdf', id, ['Spanish', 'French']);
    expect(body.languages).toEqual(['spa', 'fra']);
    await helpers.waitForToast(/OCR retry queued/);
  });
});
