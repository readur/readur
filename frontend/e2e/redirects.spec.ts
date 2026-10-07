import { test, expect } from './fixtures/auth';

/** Every pre-redesign URL lands on its new home (app/legacyRoutes.ts). */
const LEGACY: { from: string; to: RegExp; heading: string | null }[] = [
  { from: '/', to: /\/home$/, heading: null },
  { from: '/board', to: /\/home$/, heading: null },
  { from: '/dashboard', to: /\/home$/, heading: null },
  { from: '/upload', to: /\/intake\?section=upload$/, heading: 'Intake' },
  { from: '/watch', to: /\/sources\?section=watch$/, heading: 'Sources' },
  { from: '/intake?section=connections', to: /\/sources\?section=connections$/, heading: 'Sources' },
  { from: '/documents/management', to: /\/intake\?section=attention$/, heading: 'Intake' },
  { from: '/ignored-files', to: /\/intake\?section=ignored$/, heading: 'Intake' },
  { from: '/labels', to: /\/settings\/labels$/, heading: 'Settings' },
  { from: '/debug', to: /\/settings\/debug$/, heading: 'Settings' },
  { from: '/profile', to: /\/settings\/account$/, heading: 'Settings' },
];

test.describe('Legacy redirects', () => {
  test('each legacy URL lands on the right new place', async ({ dynamicAdminPage: page }) => {
    for (const { from, to, heading } of LEGACY) {
      await page.goto(from);
      await expect(page, `${from} should redirect`).toHaveURL(to);
      const h1 = heading ? page.getByRole('heading', { level: 1, name: heading }) : page.getByRole('heading', { level: 1 });
      await expect(h1).toBeVisible();
    }
  });

  test('the section the redirect picks is selected', async ({ dynamicAdminPage: page }) => {
    const tabs: [string, RegExp][] = [
      ['/upload', /^Add documents/],
      ['/intake?section=connections', /^Connections/],
      ['/watch', /^Watch folder/],
      ['/documents/management', /^Needs attention/],
      ['/ignored-files', /^Ignored/],
    ];
    for (const [from, tab] of tabs) {
      await page.goto(from);
      await expect(page.getByRole('tab', { name: tab, selected: true })).toBeVisible();
    }
  });

  test('an old document page link opens the document drawer instead', async ({ dynamicAdminPage: page }) => {
    // An unknown id is enough: the redirect does not depend on the document existing.
    const id = '00000000-0000-4000-8000-000000000000';
    await page.goto(`/documents/${id}`);
    await expect(page).toHaveURL(new RegExp(`/documents\\?document=${id}$`));
    await expect(page.getByRole('dialog').getByRole('heading', { name: 'Document not found' })).toBeVisible();
    await page.goto(`/documents/${id}?q=lease`);
    await expect(page).toHaveURL(new RegExp(`/search\\?q=lease&document=${id}$`));
  });

  test('extra query parameters and the hash are kept', async ({ dynamicAdminPage: page }) => {
    await page.goto('/watch?foo=bar#top');
    await expect(page).toHaveURL(/\/sources\?/);
    const url = new URL(page.url());
    expect(url.searchParams.get('section')).toBe('watch');
    expect(url.searchParams.get('foo')).toBe('bar');
    expect(url.hash).toBe('#top');
  });

  test('the target section wins over an incoming one', async ({ dynamicAdminPage: page }) => {
    await page.goto('/upload?section=ignored');
    await expect(page).toHaveURL(/section=upload/);
    await expect(page).not.toHaveURL(/section=ignored/);
  });

  test('/search is its own page and the old ?query= spelling becomes ?q=', async ({ dynamicAdminPage: page }) => {
    await page.goto('/search?query=receipt');
    await expect(page).toHaveURL(/\/search\?q=receipt$/);
    await expect(page.getByRole('navigation', { name: 'Main' }).getByRole('link', { name: 'Search' })).toHaveAttribute(
      'aria-current',
      'page',
    );
  });

  test('Library searches with ?q= still work', async ({ dynamicAdminPage: page }) => {
    await page.goto('/documents?q=invoice');
    await expect(page).toHaveURL(/\/documents\?q=invoice/);
  });

  test('signed-out visitors go to sign-in first', async ({ page }) => {
    await page.goto('/dashboard');
    await expect(page).toHaveURL(/\/login/);
  });
});
