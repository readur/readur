import { test, expect } from './fixtures/auth';
import { TestHelpers } from './utils/test-helpers';

/** Under 900px the sidebar becomes a drawer behind a slim top bar, and a tab bar holds the main destinations. */
test.describe('Mobile navigation', () => {
  test.use({ viewport: { width: 390, height: 844 }, hasTouch: true });

  test('the bottom tab bar navigates between destinations', async ({ dynamicUserPage: page }) => {
    await page.goto('/home');
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();

    // One navigation, outside the header, pinned to the bottom of the screen
    const nav = page.getByRole('navigation', { name: 'Main' });
    await expect(nav).toHaveCount(1);
    await expect(page.getByRole('banner').getByRole('navigation', { name: 'Main' })).toHaveCount(0);
    const box = await nav.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.y + box!.height).toBeGreaterThan(844 - 100);
    expect(box!.width).toBeLessThanOrEqual(390);

    const destinations: [string, RegExp][] = [
      ['Library', /\/documents$/],
      ['Intake', /\/intake/],
      ['Search', /\/search/],
      ['Home', /\/home$/],
    ];
    for (const [name, url] of destinations) {
      const link = nav.getByRole('link', { name });
      await link.tap();
      await expect(page).toHaveURL(url);
      await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
      await expect(link).toHaveAttribute('aria-current', 'page');
    }
  });

  test('the menu button opens the sidebar as a drawer, which closes on navigation', async ({ dynamicUserPage: page }) => {
    await page.goto('/home');
    await page.getByRole('button', { name: 'Open menu' }).tap();
    const drawer = page.getByRole('dialog', { name: 'Menu' });
    await expect(drawer).toBeVisible();
    await expect(drawer.getByRole('navigation', { name: 'Collections' })).toBeVisible();
    await expect(drawer.getByRole('navigation', { name: 'Sources' })).toBeVisible();
    await drawer.getByRole('link', { name: 'Settings' }).tap();
    await expect(page).toHaveURL(/\/settings/);
    await expect(drawer).toBeHidden();

    await page.getByRole('button', { name: 'Open menu' }).tap();
    await page.keyboard.press('Escape');
    await expect(drawer).toBeHidden();
  });

  test('tab targets are at least 44px tall', async ({ dynamicUserPage: page }) => {
    await page.goto('/home');
    const links = page.getByRole('navigation', { name: 'Main' }).getByRole('link');
    await expect(links).toHaveCount(4);
    for (const link of await links.all()) {
      const box = await link.boundingBox();
      expect(box!.height).toBeGreaterThanOrEqual(44);
    }
  });

  test('search becomes an icon button that opens the palette', async ({ dynamicUserPage: page }) => {
    await page.goto('/home');
    await page.getByRole('button', { name: 'Search documents' }).tap();
    await expect(page.getByRole('dialog', { name: 'Command palette' })).toBeVisible();
  });

  test('the bulk action bar sits above the tab bar, which stays usable', async ({ dynamicUserPage: page }) => {
    const helpers = new TestHelpers(page);
    const name = `dock-${Math.random().toString(36).slice(2, 6)}.txt`;
    await helpers.uploadBufferViaAPI(name, Buffer.from(`Dock check ${name} ${Math.random()}`), 'text/plain');

    await page.goto('/documents');
    await page.getByRole('checkbox', { name: `Select ${name}` }).check({ force: true });
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
    for (const path of ['/home', '/search', '/documents', '/intake?section=upload', '/sources', '/settings']) {
      await page.goto(path);
      await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
      const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
      expect(overflow, `${path} overflows horizontally`).toBeLessThanOrEqual(0);
    }
  });
});
