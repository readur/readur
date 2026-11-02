import type { Locator, Page } from '@playwright/test';
import { test, expect } from './fixtures/auth';
import { TIMEOUTS } from './utils/test-data';
import { TestHelpers } from './utils/test-helpers';

/**
 * A toast raised from a SlideOver must not land on the panel's footer: beside the panel on a
 * wide screen, above its footer on a phone. The footer's buttons stay pressable while the toast
 * is up (a toast under the pointer also pauses its own timer, so it would never go away).
 */
test.describe('Toast placement with an open SlideOver', () => {
  async function openFailedDocument(page: Page, name: string): Promise<Locator> {
    const helpers = new TestHelpers(page);
    const id = await helpers.uploadBufferViaAPI(name, Buffer.from(`%PDF-1.4\nnot a real pdf ${Math.random()}\n`), 'application/pdf');
    expect((await helpers.waitForOCRComplete(id)).ocr_status).toBe('failed');
    await helpers.openIntake('attention');
    const grid = page.getByRole('grid', { name: 'Failed documents' });
    await grid.getByRole('row', { name: new RegExp(name.replace('.', '\\.')) }).getByRole('rowheader').click();
    const panel = page.getByRole('dialog', { name });
    await expect(panel).toBeVisible();
    return panel;
  }

  const overlaps = (a: { x: number; y: number; width: number; height: number }, b: typeof a) =>
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height;

  async function expectFooterClear(page: Page, panel: Locator) {
    const retry = panel.getByRole('button', { name: 'Retry OCR' });
    const toasts = page.getByRole('region', { name: 'Notifications' }).getByRole('alertdialog');

    await retry.click();
    await expect(toasts.first()).toBeVisible({ timeout: TIMEOUTS.medium });

    const button = (await retry.boundingBox())!;
    for (const toast of await toasts.all()) {
      const box = (await toast.boundingBox())!;
      expect(overlaps(box, button), 'toast covers the Retry OCR button').toBe(false);
    }
    // Nothing (not even the toast region's box) sits over the button's centre.
    const hit = await retry.evaluate((el) => {
      const r = el.getBoundingClientRect();
      const top = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
      return Boolean(top && el.contains(top));
    });
    expect(hit, 'Retry OCR is the element under its own centre').toBe(true);

    // And it can be pressed again straight away, with the toast still up.
    const again = page.waitForResponse((r) => /\/ocr\/retry$/.test(r.url()) && r.request().method() === 'POST');
    await retry.click({ timeout: TIMEOUTS.short });
    await again;
  }

  test('beside the panel on a wide screen', async ({ dynamicUserPage: page }) => {
    await page.setViewportSize({ width: 1280, height: 800 });
    const panel = await openFailedDocument(page, 'toast-wide.pdf');
    await expectFooterClear(page, panel);

    // The toasts sit left of the panel.
    const panelBox = (await panel.boundingBox())!;
    const toast = (await page.getByRole('region', { name: 'Notifications' }).getByRole('alertdialog').first().boundingBox())!;
    expect(toast.x + toast.width).toBeLessThanOrEqual(panelBox.x);
  });

  test('above the footer on a phone', async ({ dynamicUserPage: page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    const panel = await openFailedDocument(page, 'toast-narrow.pdf');
    await expectFooterClear(page, panel);
  });
});
