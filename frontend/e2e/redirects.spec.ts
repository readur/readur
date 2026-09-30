import { test, expect } from './fixtures/auth';

/** Every pre-redesign URL lands on its new home (app/legacyRoutes.ts). */
const LEGACY: { from: string; to: RegExp; heading: string }[] = [
  { from: '/', to: /\/board$/, heading: 'Board' },
  { from: '/dashboard', to: /\/board$/, heading: 'Board' },
  { from: '/upload', to: /\/intake\?section=upload$/, heading: 'Intake' },
  { from: '/sources', to: /\/intake\?section=connections$/, heading: 'Intake' },
  { from: '/watch', to: /\/intake\?section=watch$/, heading: 'Intake' },
  { from: '/documents/management', to: /\/intake\?section=attention$/, heading: 'Intake' },
  { from: '/ignored-files', to: /\/intake\?section=ignored$/, heading: 'Intake' },
  { from: '/labels', to: /\/settings\/labels$/, heading: 'Settings' },
  { from: '/debug', to: /\/settings\/debug$/, heading: 'Settings' },
  { from: '/profile', to: /\/settings$/, heading: 'Settings' },
  { from: '/search', to: /\/documents$/, heading: 'Library' },
];

test.describe('Legacy redirects', () => {
  test('each legacy URL lands on the right new place', async ({ dynamicAdminPage: page }) => {
    for (const { from, to, heading } of LEGACY) {
      await page.goto(from);
      await expect(page, `${from} should redirect`).toHaveURL(to);
      await expect(page.getByRole('heading', { level: 1, name: heading })).toBeVisible();
    }
  });

  test('the Intake section the redirect picks is selected', async ({ dynamicAdminPage: page }) => {
    const tabs: [string, RegExp][] = [
      ['/upload', /^Add documents/],
      ['/sources', /^Connections/],
      ['/watch', /^Watch folder/],
      ['/documents/management', /^Needs attention/],
      ['/ignored-files', /^Ignored/],
    ];
    for (const [from, tab] of tabs) {
      await page.goto(from);
      await expect(page.getByRole('tab', { name: tab, selected: true })).toBeVisible();
    }
  });

  test('extra query parameters and the hash are kept', async ({ dynamicAdminPage: page }) => {
    await page.goto('/sources?foo=bar#top');
    await expect(page).toHaveURL(/\/intake\?/);
    const url = new URL(page.url());
    expect(url.searchParams.get('section')).toBe('connections');
    expect(url.searchParams.get('foo')).toBe('bar');
    expect(url.hash).toBe('#top');
  });

  test('the target section wins over an incoming one', async ({ dynamicAdminPage: page }) => {
    await page.goto('/upload?section=ignored');
    await expect(page).toHaveURL(/section=upload/);
    await expect(page).not.toHaveURL(/section=ignored/);
  });

  test('old search URLs become Library searches', async ({ dynamicAdminPage: page }) => {
    await page.goto('/search?q=invoice');
    await expect(page).toHaveURL(/\/documents\?q=invoice/);
    await expect(page.getByRole('searchbox', { name: 'Search documents' })).toHaveValue('invoice');

    await page.goto('/search?query=receipt');
    await expect(page).toHaveURL(/\/documents\?q=receipt/);
  });

  test('signed-out visitors go to sign-in first', async ({ page }) => {
    await page.goto('/dashboard');
    await expect(page).toHaveURL(/\/login/);
  });
});
