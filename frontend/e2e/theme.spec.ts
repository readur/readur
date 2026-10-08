import { test, expect } from './fixtures/auth';

/** Light/dark theme: the sidebar toggle and Settings → Appearance, persisted across reloads. */
test.describe('Theme', () => {
  test.use({ colorScheme: 'light' });

  test('the toggle switches theme and it persists across a reload', async ({ dynamicUserPage: page }) => {
    await page.goto('/home');
    const html = page.locator('html');
    await expect(html).toHaveAttribute('data-theme', 'light');

    await page.getByRole('button', { name: 'Switch to dark mode' }).click();
    await expect(html).toHaveAttribute('data-theme', 'dark');
    await expect(page.getByRole('button', { name: 'Switch to light mode' })).toBeVisible();

    await page.reload();
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
    await expect(html).toHaveAttribute('data-theme', 'dark');
    await expect(page.getByRole('button', { name: 'Switch to light mode' })).toBeVisible();

    // And back
    await page.getByRole('button', { name: 'Switch to light mode' }).click();
    await page.reload();
    await expect(html).toHaveAttribute('data-theme', 'light');
  });

  test('the page and the sidebar take the theme colours', async ({ dynamicUserPage: page }) => {
    await page.goto('/home');
    const sidebar = page.getByRole('banner');
    await expect(sidebar).toBeVisible();
    const colours = async () => ({
      body: await page.evaluate(() => getComputedStyle(document.body).backgroundColor),
      sidebar: await sidebar.evaluate((el) => getComputedStyle(el).backgroundColor),
    });
    const light = await colours();
    expect(light.body).toBe('rgb(240, 241, 247)');
    expect(light.sidebar).toBe('rgb(255, 255, 255)');

    await page.getByRole('button', { name: 'Switch to dark mode' }).click();
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
    const dark = await colours();
    expect(dark.body).toBe('rgb(16, 18, 28)');
    expect(dark.sidebar).toBe('rgb(23, 26, 40)');
  });

  test('Settings → Appearance sets the theme', async ({ dynamicUserPage: page }) => {
    await page.goto('/settings/appearance');
    await expect(page.getByRole('heading', { level: 2, name: 'Appearance' })).toBeVisible();

    await page.getByRole('button', { name: 'Edit Theme' }).click();
    const theme = page.getByRole('radiogroup', { name: 'Theme' });
    await theme.getByText('Dark', { exact: true }).click();
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
    await expect(theme.getByRole('radio', { name: /^Dark/ })).toBeChecked();

    await page.reload();
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
    await page.getByRole('button', { name: 'Edit Theme' }).click();
    await expect(page.getByRole('radiogroup', { name: 'Theme' }).getByRole('radio', { name: /^Dark/ })).toBeChecked();
  });
});
