import { test, expect } from './fixtures/auth';

const DESTINATIONS = [
  { name: 'Board', path: '/board', heading: 'Board' },
  { name: 'Library', path: '/documents', heading: 'Library' },
  { name: 'Intake', path: '/intake', heading: 'Intake' },
  { name: 'Settings', path: '/settings', heading: 'Settings' },
];

test.describe('Navigation', () => {
  test('should reach every destination after login', async ({ dynamicAdminPage: page }) => {
    for (const d of DESTINATIONS) {
      await page.goto(d.path);
      await expect(page.getByRole('heading', { level: 1, name: d.heading })).toBeVisible();
      // Exactly one page title per destination
      await expect(page.getByRole('heading', { level: 1 })).toHaveCount(1);
    }

    // The upload section has a file input behind its "Choose files" button
    await page.goto('/intake?section=upload');
    await expect(page.locator('input[type="file"]')).toBeAttached();
    await expect(page.getByRole('button', { name: 'Choose files' }).first()).toBeVisible();
  });

  test('should navigate with the main navigation and mark the current page', async ({ dynamicAdminPage: page }) => {
    await page.goto('/board');
    const nav = page.getByRole('navigation', { name: 'Main' });

    for (const d of DESTINATIONS) {
      await nav.getByRole('link', { name: d.name }).click();
      await expect(page).toHaveURL(new RegExp(`${d.path}(\\?|$|/)`));
      await expect(page.getByRole('heading', { level: 1, name: d.heading })).toBeVisible();
      await expect(nav.getByRole('link', { name: d.name })).toHaveAttribute('aria-current', 'page');
    }

    // The wordmark goes home
    await page.getByRole('link', { name: 'Readur home' }).click();
    await expect(page).toHaveURL(/\/board/);
  });

  test('should offer a skip link to the main content', async ({ dynamicAdminPage: page }) => {
    await page.goto('/board');
    await expect(page.getByRole('heading', { level: 1, name: 'Board' })).toBeVisible();
    const skip = page.getByRole('link', { name: 'Skip to content' });
    await expect(skip).toHaveAttribute('href', '#main');
    await skip.focus();
    await expect(skip).toBeVisible();
    await page.keyboard.press('Enter');
    await expect(page.getByRole('main')).toBeFocused();
  });

  test('should send unknown paths to the board', async ({ dynamicAdminPage: page }) => {
    await page.goto('/no-such-page');
    await expect(page).toHaveURL(/\/board/);
  });
});
