import * as fs from 'fs';
import type { Page } from '@playwright/test';
import { test, expect } from './fixtures/auth';
import { TEST_FILES, TIMEOUTS } from './utils/test-data';
import { TestHelpers, resolveTestFile } from './utils/test-helpers';

/**
 * At phone width (390×844) no page may scroll sideways: wide tables scroll inside their own
 * box, low-priority columns drop out, and tab rows scroll in place. Pages are checked with data
 * in them, because empty tables and boards hide the problem.
 */
test.describe('No horizontal overflow at 390px', () => {
  test.use({ viewport: { width: 390, height: 844 }, hasTouch: true });

  const LONG_NAME = 'quarterly-statement-with-a-very-long-descriptive-file-name-2026.png';

  async function expectNoOverflow(page: Page, label: string) {
    const { scrollWidth, clientWidth } = await page.evaluate(() => ({
      scrollWidth: document.documentElement.scrollWidth,
      clientWidth: document.documentElement.clientWidth,
    }));
    expect(clientWidth, 'viewport width').toBe(390);
    expect(scrollWidth, `${label} scrolls sideways`).toBeLessThanOrEqual(390);
  }

  async function visit(page: Page, path: string, ready?: (page: Page) => Promise<void>) {
    await page.goto(path);
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible({ timeout: TIMEOUTS.medium });
    if (ready) await ready(page);
    await expectNoOverflow(page, path);
  }

  test('populated pages stay within the screen', async ({ dynamicUserPage: page }) => {
    test.setTimeout(240000);
    const helpers = new TestHelpers(page);

    // One readable document with a long name, one that always fails OCR (feeds Needs attention).
    const readable = await helpers.uploadBufferViaAPI(LONG_NAME, fs.readFileSync(resolveTestFile(TEST_FILES.test1)), 'image/png');
    const broken = await helpers.uploadBufferViaAPI(
      'broken-overflow-check.pdf',
      Buffer.from(`%PDF-1.4\nnot a real pdf ${Math.random()}\n`),
      'application/pdf',
    );
    await helpers.waitForOCRComplete(readable);
    expect((await helpers.waitForOCRComplete(broken)).ocr_status).toBe('failed');

    await visit(page, '/home', async (p) => {
      await expect(p.getByText(LONG_NAME).first()).toBeVisible({ timeout: TIMEOUTS.medium });
    });

    await visit(page, '/documents', async () => {
      await expect(helpers.documentRows()).toHaveCount(2, { timeout: TIMEOUTS.medium });
    });
    // Columns dropped on a phone fold into the row: a visible mono meta line of values
    // (PNG · 2.0 KB · 3 min ago · Upload), announced with their labels as the row's description.
    const row = helpers.documentRows().filter({ hasText: LONG_NAME });
    await expect(row.getByText(/^PNG$/).filter({ visible: true })).toBeVisible();
    await expect(row.getByText(/^\d+(\.\d+)? (B|KB|MB)$/).filter({ visible: true })).toBeVisible();
    await expect(row).toHaveAccessibleDescription(/Type ?: PNG ?; Status ?: .+; Source ?: .+; Labels ?: .*; Size ?: .+; Added ?: .+;/);

    // The stacked row stays compact, and the name gets the width: its first line shows a real
    // stretch of the filename, not "t." or "tes…".
    const rowBox = await row.boundingBox();
    expect(rowBox!.height, 'Library row height at 390px').toBeLessThanOrEqual(80);
    const firstLineChars = await row.getByText(LONG_NAME, { exact: true }).evaluate((el) => {
      const text = el.firstChild;
      if (!text || text.nodeType !== Node.TEXT_NODE) return 0;
      const range = document.createRange();
      range.setStart(text, 0);
      range.setEnd(text, 1);
      const top = range.getBoundingClientRect().top;
      let count = 0;
      for (let i = 0; i < (text.textContent ?? '').length; i += 1) {
        range.setStart(text, i);
        range.setEnd(text, i + 1);
        const rect = range.getBoundingClientRect();
        if (Math.abs(rect.top - top) > 2 || rect.width === 0) break;
        count += 1;
      }
      return count;
    });
    expect(firstLineChars, 'characters on the first line of the name').toBeGreaterThanOrEqual(12);

    await visit(page, `/documents/${readable}`);

    for (const section of ['upload', 'connections', 'watch', 'attention', 'ignored'] as const) {
      await visit(page, `/intake?section=${section}`, async (p) => {
        // The selected tab is scrolled into view within the tab row.
        const tab = await p.getByRole('tab', { selected: true }).boundingBox();
        expect(tab!.x).toBeGreaterThanOrEqual(0);
        expect(tab!.x + tab!.width).toBeLessThanOrEqual(390);
        if (section === 'attention') {
          await expect(p.getByRole('grid', { name: 'Failed documents' }).getByText('broken-overflow-check.pdf')).toBeVisible({
            timeout: TIMEOUTS.medium,
          });
        }
      });
    }
    for (const view of ['lowConfidence', 'duplicates', 'cleanup']) {
      await visit(page, `/intake?section=attention&view=${view}`);
    }

    for (const path of ['/settings', '/settings/ocr', '/settings/labels', '/settings/api-keys', '/settings/appearance']) {
      await visit(page, path);
    }
  });

  test('admin settings stay within the screen', async ({ dynamicAdminPage: page }) => {
    for (const path of ['/settings/users', '/settings/server', '/settings/debug']) {
      await visit(page, path);
    }
  });
});
