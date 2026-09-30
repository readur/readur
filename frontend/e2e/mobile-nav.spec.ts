import { test, expect } from './fixtures/auth';
import { TestHelpers } from './utils/test-helpers';

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

  test('the bulk action bar sits above the tab bar, which stays usable', async ({ dynamicUserPage: page }) => {
    const helpers = new TestHelpers(page);
    const name = `dock-${Math.random().toString(36).slice(2, 6)}.txt`;
    await helpers.uploadBufferViaAPI(name, Buffer.from(`Dock check ${name} ${Math.random()}`), 'text/plain');

    await page.goto('/documents');
    const row = helpers.documentRows().filter({ hasText: name });
    await row.getByRole('checkbox').check({ force: true });
    const dock = page.getByRole('toolbar', { name: 'Bulk actions' });
    await expect(dock).toBeVisible();

    const nav = page.getByRole('navigation', { name: 'Main' });
    const dockBox = (await dock.boundingBox())!;
    const navBox = (await nav.boundingBox())!;
    expect(dockBox.y + dockBox.height).toBeLessThanOrEqual(navBox.y);

    // Every tab is still the topmost element at its centre, so taps reach it.
    for (const link of await nav.getByRole('link').all()) {
      const box = (await link.boundingBox())!;
      const onTop = await link.evaluate(
        (el, [x, y]) => {
          const hit = document.elementFromPoint(x, y);
          return hit !== null && (el === hit || el.contains(hit));
        },
        [box.x + box.width / 2, box.y + box.height / 2],
      );
      expect(onTop, `${await link.textContent()} is covered`).toBe(true);
    }
    await nav.getByRole('link', { name: 'Intake' }).tap();
    await expect(page).toHaveURL(/\/intake/);
  });

  // Populated pages are covered in no-horizontal-overflow.spec.ts.
  test('the page does not scroll sideways', async ({ dynamicUserPage: page }) => {
    for (const path of ['/board', '/documents', '/intake?section=upload', '/settings']) {
      await page.goto(path);
      await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
      const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
      expect(overflow, `${path} overflows horizontally`).toBeLessThanOrEqual(0);
    }
  });
});
