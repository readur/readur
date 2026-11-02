import type { Locator, Page } from '@playwright/test';
import { test, expect } from './fixtures/auth';
import { TestHelpers } from './utils/test-helpers';

/** The slideout sits above the app bar and tab bar: its header and footer actions are reachable. */
async function isOnTop(el: Locator): Promise<boolean> {
  return el.evaluate((node) => {
    const r = node.getBoundingClientRect();
    const top = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
    return Boolean(top && node.contains(top));
  });
}

async function openSlideout(page: Page): Promise<Locator> {
  const helpers = new TestHelpers(page);
  const name = `layer-${Math.random().toString(36).slice(2, 6)}.txt`;
  await helpers.uploadBufferViaAPI(name, Buffer.from(`Layering ${Math.random()}`), 'text/plain');
  await page.goto('/documents');
  return helpers.openDocumentCard(name);
}

for (const [label, size] of [
  ['desktop', { width: 1440, height: 900 }],
  ['phone', { width: 390, height: 844 }],
] as const) {
  test(`slideout close and action buttons are visible and on top (${label})`, async ({ dynamicUserPage: page }) => {
    await page.setViewportSize(size);
    const panel = await openSlideout(page);

    const close = panel.getByRole('button', { name: /close/i }).first();
    await expect(close).toBeVisible();
    // Poll: the panel is still sliding in for the first frames.
    await expect.poll(() => isOnTop(close), { message: 'close button is covered' }).toBe(true);

    const download = panel.getByRole('button', { name: 'Download', exact: true });
    await expect(download).toBeVisible();
    await expect.poll(() => isOnTop(download), { message: 'Download action is covered' }).toBe(true);
    const box = (await download.boundingBox())!;
    expect(box.y + box.height).toBeLessThanOrEqual(size.height);
  });
}
