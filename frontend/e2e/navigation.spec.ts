import { test, expect } from './fixtures/auth';

/** `heading: null` means the page names itself (Home greets, Search shows the query). */
const DESTINATIONS: { name: string; path: string; heading: string | null }[] = [
  { name: 'Home', path: '/home', heading: null },
  { name: 'Advanced search', path: '/search', heading: null },
  { name: 'Library', path: '/documents', heading: 'Library' },
  { name: 'Intake', path: '/intake', heading: 'Intake' },
  { name: 'Settings', path: '/settings', heading: 'Settings' },
];

const title = (page: import('@playwright/test').Page, heading: string | null) =>
  heading ? page.getByRole('heading', { level: 1, name: heading }) : page.getByRole('heading', { level: 1 });

test.describe('Navigation', () => {
  test('should reach every destination after login', async ({ dynamicAdminPage: page }) => {
    for (const d of DESTINATIONS) {
      await page.goto(d.path);
      await expect(title(page, d.heading)).toBeVisible();
      // Exactly one page title per destination
      await expect(page.getByRole('heading', { level: 1 })).toHaveCount(1);
    }

    // The upload section has a file input behind its "Choose files" button
    await page.goto('/intake?section=upload');
    await expect(page.locator('input[type="file"]')).toBeAttached();
    await expect(page.getByRole('button', { name: 'Choose files' }).first()).toBeVisible();
  });

  test('should navigate with the main navigation and mark the current page', async ({ dynamicAdminPage: page }) => {
    await page.goto('/home');
    const nav = page.getByRole('navigation', { name: 'Main' });

    for (const d of DESTINATIONS) {
      await nav.getByRole('link', { name: d.name }).click();
      await expect(page).toHaveURL(new RegExp(`${d.path}(\\?|$|/)`));
      await expect(title(page, d.heading)).toBeVisible();
      await expect(nav.getByRole('link', { name: d.name })).toHaveAttribute('aria-current', 'page');
    }

    // The wordmark goes home
    await page.getByRole('link', { name: 'Readur home' }).click();
    await expect(page).toHaveURL(/\/home$/);
  });

  test('the sidebar lists collections and sources and links into them', async ({ dynamicAdminPage: page }) => {
    await page.goto('/home');
    const collections = page.getByRole('navigation', { name: 'Collections' });
    await expect(collections.getByRole('link', { name: 'All collections' })).toHaveAttribute('href', '/settings/labels');

    const sources = page.getByRole('navigation', { name: 'Sources' });
    await sources.getByRole('link', { name: 'Watch folder' }).click();
    await expect(page).toHaveURL(/\/sources\?section=watch/);
    await expect(sources.getByRole('link', { name: 'Watch folder' })).toHaveAttribute('aria-current', 'page');
    await sources.getByRole('link', { name: 'Uploads' }).click();
    await expect(page).toHaveURL(/\/intake\?section=upload/);
  });

  test('should offer a skip link to the main content', async ({ dynamicAdminPage: page }) => {
    await page.goto('/home');
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
    const skip = page.getByRole('link', { name: 'Skip to content' });
    await expect(skip).toHaveAttribute('href', '#main');
    await skip.focus();
    await expect(skip).toBeVisible();
    await page.keyboard.press('Enter');
    await expect(page.getByRole('main')).toBeFocused();
  });

  test('should send unknown paths home', async ({ dynamicAdminPage: page }) => {
    await page.goto('/no-such-page');
    await expect(page).toHaveURL(/\/home$/);
  });
});
