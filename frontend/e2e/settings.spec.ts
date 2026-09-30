import { test, expect } from './fixtures/auth';
import type { Page } from '@playwright/test';
import { TIMEOUTS } from './utils/test-data';
import { TestHelpers } from './utils/test-helpers';
import { E2ETestAuthHelper } from './utils/test-auth-helper';

/**
 * Settings sections live at /settings/<section>. Each group folds open with "Edit <group>";
 * Save/Cancel appear once a field changes and Save sends only the changed keys.
 */
test.describe('Settings Management', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ dynamicAdminPage }) => {
    helpers = new TestHelpers(dynamicAdminPage);
  });

  const sectionNav = (page: Page) => page.getByRole('navigation', { name: 'Settings sections' });

  async function saveGroup(page: Page) {
    const put = page.waitForResponse((r) => r.url().includes('/api/settings') && r.request().method() === 'PUT', {
      timeout: TIMEOUTS.medium,
    });
    await page.getByRole('main').getByRole('button', { name: 'Save', exact: true }).click();
    const response = await put;
    expect(response.ok()).toBe(true);
    await helpers.waitForToast(/Settings updated successfully/);
    return JSON.parse(response.request().postData() ?? '{}');
  }

  test('should display settings interface', async ({ dynamicAdminPage: page }) => {
    await page.goto('/settings');
    await expect(page.getByRole('heading', { level: 1, name: 'Settings' })).toBeVisible();
    await expect(page.getByRole('heading', { level: 2, name: 'General' })).toBeVisible();

    const nav = sectionNav(page);
    for (const name of ['General', 'OCR', 'Users', 'Server', 'API keys', 'Labels', 'Debug', 'Appearance']) {
      await expect(nav.getByRole('link', { name, exact: true })).toBeVisible();
    }
    await expect(nav.getByRole('link', { name: 'General' })).toHaveAttribute('aria-current', 'page');

    // Each section has its own URL
    await nav.getByRole('link', { name: 'OCR' }).click();
    await expect(page).toHaveURL(/\/settings\/ocr/);
    await expect(page.getByRole('heading', { level: 2, name: 'OCR' })).toBeVisible();

    // Unknown sections fall back to /settings
    await page.goto('/settings/nope');
    await expect(page).toHaveURL(/\/settings$/);
  });

  test('should update OCR settings', async ({ dynamicAdminPage: page }) => {
    await page.goto('/settings/ocr');
    await page.getByRole('button', { name: 'Edit Languages' }).click();
    const languages = page.getByRole('region', { name: 'Languages' });

    await languages.getByRole('button', { name: /Add more languages/ }).click();
    await languages.getByRole('group', { name: 'Available Languages' }).getByText('Spanish', { exact: true }).click();
    await expect(languages.getByRole('list', { name: 'Selected languages' })).toContainText('Spanish');

    const body = await saveGroup(page);
    expect(body.preferred_languages).toEqual(['eng', 'spa']);

    // Persisted: a reload shows both languages
    await page.reload();
    await page.getByRole('button', { name: 'Edit Languages' }).click();
    await expect(page.getByRole('list', { name: 'Selected languages' })).toContainText('Spanish');
  });

  test('should show the watch folder configuration', async ({ dynamicAdminPage: page }) => {
    // The server watch folder is read-only configuration (Settings → Server);
    // the watch queue itself is Intake → Watch folder.
    await page.goto('/settings/server');
    await page.getByRole('button', { name: 'Edit Watch Folder Configuration' }).click();
    const watch = page.getByRole('region', { name: 'Watch Folder Configuration' });
    await expect(watch.getByRole('term').filter({ hasText: 'Watch Interval' })).toBeVisible();
    await expect(page.getByRole('region', { name: 'File Upload Configuration' }).getByRole('term').filter({ hasText: 'Watch Folder' })).toBeAttached();
  });

  test('should update search settings', async ({ dynamicAdminPage: page }) => {
    await page.goto('/settings');
    await page.getByRole('button', { name: 'Edit Search Configuration' }).click();
    const group = page.getByRole('region', { name: 'Search Configuration' });

    await group.getByRole('spinbutton', { name: 'Snippet Length' }).fill('240');
    await expect(page.getByRole('main').getByRole('button', { name: 'Cancel', exact: true })).toBeVisible();

    const body = await saveGroup(page);
    // Only the changed key is sent
    expect(Object.keys(body)).toEqual(['search_snippet_length']);
    expect(body.search_snippet_length).toBe(240);

    await page.reload();
    await expect(page.getByText(/240 chars/)).toBeVisible();
  });

  test('should validate settings before saving', async ({ dynamicAdminPage: page }) => {
    await page.goto('/settings');
    await page.getByRole('button', { name: 'Edit File Processing' }).click();
    const field = page.getByRole('spinbutton', { name: 'Max File Size (MB)' });

    let saved = false;
    page.on('request', (r) => {
      if (r.url().includes('/api/settings') && r.method() === 'PUT') saved = true;
    });

    await field.fill('0');
    await page.getByRole('main').getByRole('button', { name: 'Save', exact: true }).click();
    await expect(field).toHaveAttribute('aria-invalid', 'true');
    expect(saved).toBe(false);

    // Cancel restores the saved value
    await page.getByRole('main').getByRole('button', { name: 'Cancel', exact: true }).click();
    await expect(field).toHaveValue('50');
  });

  test('should display current system status', async ({ dynamicAdminPage: page }) => {
    await page.goto('/settings/server');
    await expect(page.getByRole('heading', { level: 2, name: 'Server' })).toBeVisible();
    for (const group of ['File Upload Configuration', 'OCR Processing Configuration', 'Server Information']) {
      await expect(page.getByRole('heading', { level: 3, name: group })).toBeVisible();
    }
    await page.getByRole('button', { name: 'Edit Server Information' }).click();
    await expect(page.getByRole('region', { name: 'Server Information' }).getByRole('term').filter({ hasText: 'Version' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Refresh Configuration' })).toBeVisible();
  });

  test('should test OCR functionality', async ({ dynamicAdminPage: page }) => {
    // The Debug section runs a file through the OCR pipeline step by step
    await page.goto('/settings/debug');
    await expect(page.getByRole('heading', { level: 2, name: 'Debug' })).toBeVisible();
    await expect(page.getByRole('tab', { name: /Upload/ })).toBeVisible();
    await expect(page.getByRole('tab', { name: /Search Existing/ })).toBeVisible();
  });

  test('should update user profile', async ({ dynamicAdminPage: page, testAdmin }) => {
    // Profile edits are an admin action in Settings → Users
    const other = await new E2ETestAuthHelper(page).createTestUser();
    await page.goto('/settings/users');
    const name = other.credentials.username;
    await page.getByRole('button', { name: `Edit ${name}` }).click();

    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    const newEmail = `updated_${Date.now()}@example.com`;
    await dialog.getByRole('textbox', { name: /email/i }).fill(newEmail);

    const put = helpers.waitForApiCall(`/api/users/${other.userResponse.id}`);
    await dialog.getByRole('button', { name: /update/i }).click();
    expect((await put).ok()).toBe(true);
    await expect(dialog).toBeHidden();
    await expect(page.getByRole('grid', { name: 'User Management' }).getByRole('row', { name })).toContainText(newEmail);
    expect(testAdmin.credentials.username).not.toBe(name);
  });

  test('should change password', async ({ dynamicAdminPage: page }) => {
    const auth = new E2ETestAuthHelper(page);
    const other = await auth.createTestUser();
    await page.goto('/settings/users');
    await page.getByRole('button', { name: `Edit ${other.credentials.username}` }).click();

    const dialog = page.getByRole('dialog');
    await dialog.getByLabel(/password/i).fill('changedpass456');
    const put = helpers.waitForApiCall(`/api/users/${other.userResponse.id}`);
    await dialog.getByRole('button', { name: /update/i }).click();
    expect((await put).ok()).toBe(true);

    // The new password works, the old one does not
    await expect(auth.loginUserAPI({ username: other.credentials.username, password: 'changedpass456' })).resolves.toBeTruthy();
    await expect(auth.loginUserAPI(other.credentials)).rejects.toThrow();
  });
});
