import { test, expect } from './fixtures/auth';

/** Under 720px the top-bar navigation becomes a bottom tab bar. */
test.describe('Mobile navigation', () => {
  test.use({ viewport: { width: 390, height: 844 }, hasTouch: true });

  test('the bottom tab bar navigates between destinations', async ({ dynamicUserPage: page }) => {
    await page.goto('/board');
    await expect(page.getByRole('heading', { level: 1, name: 'Board' })).toBeVisible();

    // One navigation, outside the header, pinned to the bottom of the screen
    const nav = page.getByRole('navigation', { name: 'Main' });
    await expect(nav).toHaveCount(1);
    await expect(page.getByRole('banner').getByRole('navigation', { name: 'Main' })).toHaveCount(0);
    const box = await nav.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.y + box!.height).toBeGreaterThan(844 - 100);
    expect(box!.width).toBeLessThanOrEqual(390);

    const destinations: [string, RegExp, string][] = [
      ['Library', /\/documents$/, 'Library'],
      ['Intake', /\/intake/, 'Intake'],
      ['Settings', /\/settings/, 'Settings'],
      ['Board', /\/board$/, 'Board'],
    ];
    for (const [name, url, heading] of destinations) {
      const link = nav.getByRole('link', { name });
      await link.tap();
      await expect(page).toHaveURL(url);
      await expect(page.getByRole('heading', { level: 1, name: heading })).toBeVisible();
      await expect(link).toHaveAttribute('aria-current', 'page');
    }
  });

  test('tab targets are at least 44px tall', async ({ dynamicUserPage: page }) => {
    await page.goto('/board');
    const links = page.getByRole('navigation', { name: 'Main' }).getByRole('link');
    await expect(links).toHaveCount(4);
    for (const link of await links.all()) {
      const box = await link.boundingBox();
      expect(box!.height).toBeGreaterThanOrEqual(44);
    }
  });

  test('search becomes an icon button that opens the palette', async ({ dynamicUserPage: page }) => {
    await page.goto('/board');
    await page.getByRole('button', { name: 'Search documents' }).tap();
    await expect(page.getByRole('dialog', { name: 'Command palette' })).toBeVisible();
  });

  // App bug (see the Task 13 report): at 390px the Library grid (/documents), the Intake tab
  // row and the Board's arrivals table widen the page, so it scrolls sideways (up to ~570px).
  test.fixme('the page does not scroll sideways', async ({ dynamicUserPage: page }) => {
    for (const path of ['/board', '/documents', '/intake?section=upload', '/settings']) {
      await page.goto(path);
      await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
      const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
      expect(overflow, `${path} overflows horizontally`).toBeLessThanOrEqual(0);
    }
  });
});
