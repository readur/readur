import { test, expect } from './fixtures/auth';
import type { Page } from '@playwright/test';
import { TIMEOUTS } from './utils/test-data';
import { TestHelpers } from './utils/test-helpers';

/**
 * Failed OCR now lives in Intake → Needs attention (/intake?section=attention; the old
 * /documents/management URL redirects there). Each test signs in as a fresh user and seeds
 * PDFs that are not really PDFs, which the OCR pipeline always fails.
 */
test.describe('OCR Retry Workflow', () => {
  let helpers: TestHelpers;

  async function seedFailedDocument(name: string): Promise<string> {
    const id = await helpers.uploadBufferViaAPI(name, Buffer.from(`%PDF-1.4\nnot a real pdf ${name} ${Math.random()}\n`), 'application/pdf');
    const doc = await helpers.waitForOCRComplete(id);
    expect(doc.ocr_status).toBe('failed');
    return id;
  }

  const failedGrid = (page: Page) => page.getByRole('grid', { name: 'Failed documents' });
  const failedRow = (page: Page, name: string) => failedGrid(page).getByRole('row', { name: new RegExp(name.replace('.', '\\.')) });

  async function openAttention(page: Page) {
    await helpers.openIntake('attention');
    await expect(page.getByRole('tab', { name: /Needs attention/, selected: true })).toBeVisible();
  }

  test.beforeEach(async ({ dynamicUserPage }) => {
    helpers = new TestHelpers(dynamicUserPage);
  });

  test('should display failed OCR documents', async ({ dynamicUserPage: page }) => {
    await seedFailedDocument('broken-a.pdf');

    // The legacy management URL lands here
    await page.goto('/documents/management');
    await expect(page).toHaveURL(/\/intake\?section=attention/);
    await expect(page.getByRole('radio', { name: 'Failed OCR' })).toBeChecked();

    const row = failedRow(page, 'broken-a.pdf');
    await expect(row).toBeVisible({ timeout: TIMEOUTS.medium });
    await expect(row).toContainText('Failed');
    // The tab carries the count
    await expect(page.getByRole('tab', { name: /Needs attention\s*,\s*1/ })).toBeVisible();
  });

  test('should retry individual failed OCR document', async ({ dynamicUserPage: page }) => {
    test.setTimeout(150000);
    const id = await seedFailedDocument('broken-b.pdf');
    await openAttention(page);

    await failedRow(page, 'broken-b.pdf').getByRole('rowheader').click();
    const panel = page.getByRole('dialog', { name: 'broken-b.pdf' });
    await expect(panel).toBeVisible();

    // Retry with an explicit language (the default-language path has its own test below)
    const languages = panel.getByRole('region', { name: 'Retry with languages' });
    await languages.getByRole('button', { name: /Select OCR languages/ }).click();
    await languages.getByRole('group', { name: 'Available Languages' }).getByText('English', { exact: true }).click();

    // The server keeps auto-retrying a failed job for a while and refuses a manual retry
    // (500, "queue item already exists") until it gives up, so press Retry until it is accepted.
    // The error toasts from refused attempts sit beside the panel, clear of its footer.
    await expect(async () => {
      const retry = page.waitForResponse(
        (r) => r.url().includes(`/api/documents/${id}/ocr/retry`) && r.request().method() === 'POST',
        { timeout: TIMEOUTS.medium },
      );
      await panel.getByRole('button', { name: 'Retry OCR' }).click();
      expect((await retry).ok()).toBe(true);
    }).toPass({ timeout: 90000, intervals: [5000] });
    await helpers.waitForToast(/OCR retry queued/);
  });

  test('should retry a failed OCR document with the default language', async ({ dynamicUserPage: page }) => {
    test.setTimeout(150000);
    const id = await seedFailedDocument('broken-h.pdf');
    await openAttention(page);
    await failedRow(page, 'broken-h.pdf').getByRole('rowheader').click();
    const panel = page.getByRole('dialog', { name: 'broken-h.pdf' });
    await expect(panel).toBeVisible();

    // No languages picked: the request still carries a JSON body (it used to be empty, which
    // the server refused with 415). The server may refuse a manual retry with 500 while its own
    // automatic retries are pending, so press Retry until it is accepted, as above.
    await expect(async () => {
      const retry = page.waitForResponse(
        (r) => r.url().includes(`/api/documents/${id}/ocr/retry`) && r.request().method() === 'POST',
        { timeout: TIMEOUTS.medium },
      );
      await panel.getByRole('button', { name: 'Retry OCR' }).click();
      const response = await retry;
      expect(response.status()).not.toBe(415);
      expect(response.request().headers()['content-type']).toMatch(/^application\/json/);
      expect(response.request().postDataJSON()).toEqual({});
      expect(response.ok()).toBe(true);
    }).toPass({ timeout: 90000, intervals: [5000] });
    await helpers.waitForToast(/OCR retry queued/);
  });

  test('should bulk retry multiple failed OCR documents', async ({ dynamicUserPage: page }) => {
    await seedFailedDocument('broken-c.pdf');
    await seedFailedDocument('broken-d.pdf');
    await openAttention(page);

    await failedRow(page, 'broken-c.pdf').getByRole('checkbox').check({ force: true });
    await failedRow(page, 'broken-d.pdf').getByRole('checkbox').check({ force: true });

    await page.getByRole('button', { name: 'Retry…' }).click();
    const modal = page.getByRole('dialog', { name: 'Bulk OCR retry' });
    await expect(modal).toBeVisible();
    await expect(modal.getByText(/Retry selected documents \(2 selected\)/)).toBeVisible();

    await modal.getByRole('button', { name: 'Preview' }).click();
    const execute = modal.getByRole('button', { name: /^Retry \d+ documents$/ });
    await expect(execute).toBeEnabled({ timeout: TIMEOUTS.medium });
    const bulk = helpers.waitForApiCall('/api/documents/ocr/retry/bulk', TIMEOUTS.long);
    await execute.click();
    expect((await bulk).ok()).toBe(true);
  });

  test('should show OCR retry history', async ({ dynamicUserPage: page }) => {
    await seedFailedDocument('broken-e.pdf');
    await openAttention(page);

    await failedRow(page, 'broken-e.pdf').getByRole('rowheader').click();
    const panel = page.getByRole('dialog', { name: 'broken-e.pdf' });
    const history = panel.getByRole('region', { name: 'OCR retry history' });
    await expect(history).toBeVisible();
    await expect(history.getByRole('button', { name: 'Refresh' })).toBeVisible();
  });

  test('should display OCR failure reasons', async ({ dynamicUserPage: page }) => {
    await seedFailedDocument('broken-f.pdf');
    await openAttention(page);

    const row = failedRow(page, 'broken-f.pdf');
    await expect(row.getByRole('gridcell').nth(2)).not.toBeEmpty();

    await row.getByRole('rowheader').click();
    const panel = page.getByRole('dialog', { name: 'broken-f.pdf' });
    await expect(panel.getByRole('group', { name: 'Failure' }).getByRole('term').filter({ hasText: 'Reason' })).toBeVisible();
    const message = panel.getByRole('region', { name: 'Error message' });
    await expect(message).toBeVisible();
    await expect(message).not.toBeEmpty();
    await expect(panel.getByRole('region', { name: 'Retry recommendations' })).toBeVisible();
  });

  test('should filter failed documents by failure type', async ({ dynamicUserPage: page }) => {
    await seedFailedDocument('broken-g.pdf');
    await openAttention(page);
    await expect(failedRow(page, 'broken-g.pdf')).toBeVisible();

    // Low-confidence documents are a separate view with their own threshold
    const show = page.getByRole('radiogroup', { name: 'Show' });
    await show.getByText('Low confidence', { exact: true }).click();
    await expect(page).toHaveURL(/view=/);
    await expect(failedGrid(page)).toHaveCount(0);
    await expect(page.getByRole('slider').first()).toBeVisible();

    await show.getByText('Failed OCR', { exact: true }).click();
    await expect(failedRow(page, 'broken-g.pdf')).toBeVisible();
  });
});
