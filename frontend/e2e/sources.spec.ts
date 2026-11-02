import { test, expect } from './fixtures/auth';
import type { Page } from '@playwright/test';
import { TIMEOUTS } from './utils/test-data';
import { TestHelpers } from './utils/test-helpers';

/**
 * Sources have their own page: /sources?section=connections (old Intake links redirect).
 * A row opens the connection's detail panel with its actions. Every test signs in as a fresh
 * user, so the board only holds connections the test creates. WebDAV connections point at
 * an address nothing listens on, so no test depends on an external server.
 */
test.describe('Source Management', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ dynamicUserPage }) => {
    helpers = new TestHelpers(dynamicUserPage);
    await helpers.openIntake('connections');
  });

  const addDialog = (page: Page) => page.getByRole('dialog', { name: 'Add connection' });

  test('should display sources interface', async ({ dynamicUserPage: page }) => {
    await page.goto('/sources');
    await expect(page.getByRole('heading', { level: 1, name: 'Sources' })).toBeVisible();
    await expect(page.getByRole('tab', { name: 'Connections', selected: true })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Add connection' }).first()).toBeVisible();
    await expect(page.getByRole('grid', { name: 'Connections' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'No connections yet' })).toBeVisible();
  });

  test('should refuse a local folder source for a regular user', async ({ dynamicUserPage: page }) => {
    // A local folder reads the server's own disk, so only admins may add one unless
    // LOCAL_SOURCE_ALLOWED_PATHS names folders users are allowed to watch.
    await page.getByRole('button', { name: 'Add connection' }).first().click();
    const dialog = addDialog(page);
    await dialog.getByRole('textbox', { name: 'Name', exact: true }).fill(helpers.uniqueName('Test Local Folder'));
    await dialog.getByRole('radio', { name: /^Local folder/ }).check({ force: true });

    const create = page.waitForResponse((r) => /\/api\/sources$/.test(r.url()) && r.request().method() === 'POST');
    await dialog.getByRole('button', { name: 'Add connection' }).click();
    expect((await create).status()).toBe(403);
    const toast = await helpers.waitForToast(/Could not save the connection/);
    await expect(toast).toContainText('Only an administrator can add local folders.');
    await expect(dialog).toBeVisible();
  });

  test('should create a new WebDAV source', async ({ dynamicUserPage: page }) => {
    const name = await helpers.createTestSource('Test WebDAV', 'webdav');
    await expect(helpers.connectionRow(name)).toContainText('WebDAV');
    await expect(helpers.connectionRow(name)).toContainText('never');
  });

  test('should create a new S3 source', async ({ dynamicUserPage: page }) => {
    const name = await helpers.createTestSource('Test S3 Bucket', 's3');
    await expect(helpers.connectionRow(name)).toContainText(/S3/);

    // The secret is never shown; the access key is masked
    const panel = await helpers.openConnection(name);
    await expect(panel).not.toContainText('test-secret-key');
    await expect(panel).not.toContainText('AKIATESTKEY0000');
  });

  test('should edit existing source', async ({ dynamicUserPage: page }) => {
    const source = await helpers.createWebdavSourceViaAPI({ name: helpers.uniqueName('Editable') });
    await page.reload();

    const panel = await helpers.openConnection(source.name);
    await panel.getByRole('button', { name: 'Edit', exact: true }).click();

    const dialog = page.getByRole('dialog', { name: 'Edit connection' });
    await expect(dialog).toBeVisible();
    const newName = helpers.uniqueName('Updated Source Name');
    await dialog.getByRole('textbox', { name: 'Name', exact: true }).fill(newName);

    const update = page.waitForResponse((r) => r.url().includes(`/api/sources/${source.id}`) && r.request().method() === 'PUT');
    await dialog.getByRole('button', { name: 'Save changes' }).click();
    expect((await update).ok()).toBe(true);
    await helpers.waitForToast(/Connection updated/);

    await expect(helpers.connectionRow(newName)).toBeVisible({ timeout: TIMEOUTS.medium });
  });

  test('should delete source', async ({ dynamicUserPage: page }) => {
    const source = await helpers.createWebdavSourceViaAPI({ name: helpers.uniqueName('Deletable') });
    await page.reload();

    const panel = await helpers.openConnection(source.name);
    await panel.getByRole('button', { name: 'Delete', exact: true }).click();

    const confirm = page.getByRole('alertdialog', { name: /Delete “.*”\?/ });
    await expect(confirm).toBeVisible();
    const del = page.waitForResponse((r) => r.url().includes(`/api/sources/${source.id}`) && r.request().method() === 'DELETE');
    await confirm.getByRole('button', { name: 'Delete connection' }).click();
    expect((await del).ok()).toBe(true);
    await helpers.waitForToast(/Connection deleted/);

    await expect(helpers.connectionRow(source.name)).toHaveCount(0);
    await expect(page.getByRole('heading', { name: 'No connections yet' })).toBeVisible();
  });

  test('should start source sync', async ({ dynamicUserPage: page }) => {
    const source = await helpers.createWebdavSourceViaAPI({ name: helpers.uniqueName('Syncable') });
    await page.reload();

    const panel = await helpers.openConnection(source.name);
    const sync = page.waitForResponse((r) => r.url().includes(`/api/sources/${source.id}/sync`) && r.request().method() === 'POST');
    await panel.getByRole('button', { name: 'Sync now' }).click();
    expect((await sync).ok()).toBe(true);
    await helpers.waitForToast(/Sync started/);
  });

  test('should stop source sync', async ({ dynamicUserPage: page }) => {
    const source = await helpers.createWebdavSourceViaAPI({ name: helpers.uniqueName('Stoppable') });
    await page.reload();

    // Pretend the connection is mid-sync so the panel offers "Stop sync"
    await page.route('**/api/sources', async (route) => {
      if (route.request().method() !== 'GET') return route.continue();
      const response = await route.fetch();
      const list = await response.json();
      for (const s of list) if (s.id === source.id) s.status = 'syncing';
      await route.fulfill({ response, json: list });
    });
    await page.route(`**/api/sources/${source.id}/sync/stop`, (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: '{}' }),
    );
    await page.reload();

    const panel = await helpers.openConnection(source.name);
    await expect(panel).toContainText('Syncing');
    const stop = page.waitForRequest((r) => r.url().includes(`/api/sources/${source.id}/sync/stop`) && r.method() === 'POST');
    await panel.getByRole('button', { name: 'Stop sync' }).click();
    await stop;
    await helpers.waitForToast(/Sync stopped/);
  });

  test('should display source status and statistics', async ({ dynamicUserPage: page }) => {
    const source = await helpers.createWebdavSourceViaAPI({ name: helpers.uniqueName('Stats') });
    await page.reload();

    const grid = page.getByRole('grid', { name: 'Connections' });
    for (const col of ['Name', 'Type', 'Status', 'Last sync', 'Files', 'Next']) {
      await expect(grid.getByRole('columnheader', { name: col })).toBeVisible();
    }
    const row = helpers.connectionRow(source.name);
    await expect(row).toContainText('Healthy');
    await expect(row).toContainText('never');

    const panel = await helpers.openConnection(source.name);
    for (const group of ['Connection', 'Schedule', 'Counts', 'Scope']) {
      await expect(panel.getByRole('group', { name: group })).toBeVisible();
    }
    const counts = panel.getByRole('group', { name: 'Counts' });
    for (const term of ['Documents', 'OCR done', 'Pending', 'Size']) {
      await expect(counts.getByRole('term').filter({ hasText: term })).toBeVisible();
    }
    // The password is never shown
    await expect(panel.getByRole('group', { name: 'Connection' })).toContainText('password set');
    await expect(panel).not.toContainText('e2e-password');
  });

  test('should test source connection', async ({ dynamicUserPage: page }) => {
    await page.getByRole('button', { name: 'Add connection' }).first().click();
    const dialog = addDialog(page);
    await expect(dialog.getByRole('button', { name: 'Test connection' })).toBeDisabled();

    await dialog.getByRole('textbox', { name: 'Name', exact: true }).fill('Test Connection');
    await dialog.getByRole('textbox', { name: 'Server URL' }).fill('http://127.0.0.1:9/dav/');
    await dialog.getByRole('textbox', { name: 'Username' }).fill('testuser');
    await dialog.getByRole('textbox', { name: 'Password' }).fill('testpass');

    const testCall = page.waitForResponse((r) => r.url().includes('/test') && r.request().method() === 'POST', {
      timeout: TIMEOUTS.long,
    });
    await dialog.getByRole('button', { name: 'Test connection' }).click();
    await testCall;
    // Nothing listens there, so the dialog reports the server's coarse failure category inline
    await expect(dialog.getByRole('alert')).toContainText(/Server is unreachable|Connection timed out/, {
      timeout: TIMEOUTS.long,
    });
  });

  test('should sort connections by name', async ({ dynamicUserPage: page }) => {
    // The old page's type filter is gone (Type is a column); the board sorts instead.
    const run = Math.random().toString(36).slice(2, 6);
    await helpers.createWebdavSourceViaAPI({ name: `b-conn-${run}` });
    await helpers.createWebdavSourceViaAPI({ name: `a-conn-${run}` });
    await page.reload();

    const rows = page.getByRole('grid', { name: 'Connections' }).getByRole('rowgroup').nth(1).getByRole('row');
    await expect(rows.first()).toContainText(`a-conn-${run}`);
    await page.getByRole('grid', { name: 'Connections' }).getByRole('columnheader', { name: 'Name' }).click();
    await expect(rows.first()).toContainText(`b-conn-${run}`);
  });

  test('should display sync history', async ({ dynamicUserPage: page }) => {
    // Per-connection history is the "Recent errors" list in the detail panel
    const source = await helpers.createWebdavSourceViaAPI({ name: helpers.uniqueName('History') });
    await page.reload();

    const panel = await helpers.openConnection(source.name);
    const errors = panel.getByRole('region', { name: 'Recent errors' });
    await expect(errors).toBeVisible();
    await expect(errors.getByText(/No recent errors|errors?/i).first()).toBeVisible({ timeout: TIMEOUTS.medium });
  });

  test('should validate required fields in source creation', async ({ dynamicUserPage: page }) => {
    await page.getByRole('button', { name: 'Add connection' }).first().click();
    const dialog = addDialog(page);

    let created = false;
    page.on('request', (r) => {
      if (/\/api\/sources$/.test(r.url()) && r.method() === 'POST') created = true;
    });
    await dialog.getByRole('button', { name: 'Add connection' }).click();

    await expect(dialog.getByText('Fix the highlighted fields before saving.')).toBeVisible();
    await expect(dialog.getByRole('textbox', { name: 'Name', exact: true })).toHaveAttribute('aria-invalid', 'true');
    await expect(dialog.getByRole('textbox', { name: 'Server URL' })).toHaveAttribute('aria-invalid', 'true');
    expect(created).toBe(false);
    await expect(dialog).toBeVisible();
  });

  test('should schedule automatic sync', async ({ dynamicUserPage: page }) => {
    const source = await helpers.createWebdavSourceViaAPI({ name: helpers.uniqueName('Scheduled') });
    await page.reload();

    const panel = await helpers.openConnection(source.name);
    await expect(panel.getByRole('group', { name: 'Schedule' })).toContainText('manual');
    await panel.getByRole('button', { name: 'Edit', exact: true }).click();

    const dialog = page.getByRole('dialog', { name: 'Edit connection' });
    await dialog.getByText('Automatic sync', { exact: true }).click();
    const interval = dialog.getByRole('textbox', { name: 'Sync interval (minutes)' }).or(
      dialog.getByRole('spinbutton', { name: 'Sync interval (minutes)' }),
    );
    await expect(interval).toBeVisible();
    await interval.fill('45');

    const update = page.waitForResponse((r) => r.url().includes(`/api/sources/${source.id}`) && r.request().method() === 'PUT');
    await dialog.getByRole('button', { name: 'Save changes' }).click();
    const response = await update;
    expect(response.ok()).toBe(true);
    const body = JSON.parse(response.request().postData() ?? '{}');
    expect(body.config.auto_sync).toBe(true);
    expect(body.config.sync_interval_minutes).toBe(45);

    await expect(page.getByRole('dialog', { name: source.name }).getByRole('group', { name: 'Schedule' })).toContainText('every 45 min', {
      timeout: TIMEOUTS.medium,
    });
  });
});

test.describe('Source Management (admin)', () => {
  test('should create a new local folder source', async ({ dynamicAdminPage: page }) => {
    const helpers = new TestHelpers(page);
    await helpers.openIntake('connections');
    // Admins may watch any folder that exists on the server
    const name = await helpers.createTestSource('Test Local Folder', 'local_folder', { watchFolder: '/tmp' });
    await expect(helpers.connectionRow(name)).toContainText('Local folder');
    await helpers.waitForToast(/Connection added/);
  });
});
