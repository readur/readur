import { test, expect } from './fixtures/auth';
import { TIMEOUTS } from './utils/test-data';
import { TestHelpers } from './utils/test-helpers';

/**
 * WebDAV connections in Intake → Connections. The server address is one nothing listens
 * on, so sync and test calls fail fast without an external WebDAV server.
 */
test.describe('WebDAV Workflow', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ dynamicAdminPage }) => {
    helpers = new TestHelpers(dynamicAdminPage);
    await helpers.openIntake('connections');
  });

  test('should create and configure WebDAV source', async ({ dynamicAdminPage: page }) => {
    const name = helpers.uniqueName('Test WebDAV Source');
    await page.getByRole('button', { name: 'Add connection' }).first().click();
    const dialog = page.getByRole('dialog', { name: 'Add connection' });

    await dialog.getByRole('textbox', { name: 'Name', exact: true }).fill(name);
    await expect(dialog.getByRole('radio', { name: /^WebDAV/ })).toBeChecked();
    await dialog.getByRole('radiogroup', { name: 'Server type' }).getByText('Nextcloud', { exact: true }).click();
    await dialog.getByRole('textbox', { name: 'Server URL' }).fill('http://127.0.0.1:9/remote.php/dav/files/webdav_user/');
    await dialog.getByRole('textbox', { name: 'Username' }).fill('webdav_user');
    await dialog.getByRole('textbox', { name: 'Password' }).fill('webdav_pass');

    // Add a second folder and an extension
    const folders = dialog.getByRole('textbox', { name: 'Folders to monitor' });
    await folders.fill('/Scans');
    await folders.press('Enter');
    await expect(dialog.getByRole('list', { name: 'Folders to monitor' })).toContainText('/Scans');
    const extensions = dialog.getByRole('textbox', { name: 'File extensions' });
    await extensions.fill('docx');
    await extensions.press('Enter');
    await expect(dialog.getByRole('list', { name: 'File extensions' })).toContainText('docx');

    const create = page.waitForResponse((r) => /\/api\/sources$/.test(r.url()) && r.request().method() === 'POST');
    await dialog.getByRole('button', { name: 'Add connection' }).click();
    const response = await create;
    expect(response.ok()).toBe(true);
    const body = JSON.parse(response.request().postData() ?? '{}');
    expect(body.source_type).toBe('webdav');
    expect(body.config.server_type).toBe('nextcloud');
    expect(body.config.watch_folders).toEqual(expect.arrayContaining(['/Documents', '/Scans']));
    expect(body.config.file_extensions).toContain('docx');

    const panel = await helpers.openConnection(name);
    await expect(panel.getByRole('group', { name: 'Watched folders' })).toContainText('/Scans');
    await expect(panel.getByRole('group', { name: 'Connection' })).toContainText('webdav_user');
  });

  test('should test WebDAV connection', async ({ dynamicAdminPage: page }) => {
    const source = await helpers.createWebdavSourceViaAPI({ name: helpers.uniqueName('Testable') });
    await page.reload();

    const panel = await helpers.openConnection(source.name);
    const call = page.waitForResponse((r) => r.url().includes('/test') && r.request().method() === 'POST', {
      timeout: TIMEOUTS.long,
    });
    await panel.getByRole('button', { name: 'Test connection' }).click();
    await call;
    // Nothing listens at the address, so the result is a failure toast
    await helpers.waitForToast(/Connection failed|Could not test the connection/);
  });

  test('should initiate WebDAV sync', async ({ dynamicAdminPage: page }) => {
    const source = await helpers.createWebdavSourceViaAPI({ name: helpers.uniqueName('Syncing') });
    await page.reload();

    const panel = await helpers.openConnection(source.name);
    const sync = page.waitForResponse((r) => r.url().includes(`/api/sources/${source.id}/sync`) && r.request().method() === 'POST');
    await panel.getByRole('button', { name: 'Sync now' }).click();
    expect((await sync).ok()).toBe(true);
    await helpers.waitForToast(/Sync started/);

    // Deep scan is WebDAV-only and is offered here
    await expect(panel.getByRole('button', { name: 'Deep scan' })).toBeVisible();
  });

  test('should show WebDAV sync history', async ({ dynamicAdminPage: page }) => {
    const source = await helpers.createWebdavSourceViaAPI({ name: helpers.uniqueName('History') });
    await page.reload();

    const panel = await helpers.openConnection(source.name);
    await expect(panel.getByRole('group', { name: 'Schedule' }).getByRole('term').filter({ hasText: 'Last sync' })).toBeVisible();
    await expect(panel.getByRole('region', { name: 'Recent errors' })).toBeVisible();

    // The connection's ignored files are one click away
    await panel.getByRole('button', { name: 'Ignored files' }).click();
    await expect(page).toHaveURL(/section=ignored/);
    await expect(page).toHaveURL(new RegExp(`sourceId=${source.id}`));
  });

  test('should handle WebDAV source deletion', async ({ dynamicAdminPage: page }) => {
    const source = await helpers.createWebdavSourceViaAPI({ name: helpers.uniqueName('Doomed') });
    await page.reload();

    const panel = await helpers.openConnection(source.name);
    await panel.getByRole('button', { name: 'Delete', exact: true }).click();
    const confirm = page.getByRole('alertdialog');
    const del = page.waitForResponse((r) => r.url().includes(`/api/sources/${source.id}`) && r.request().method() === 'DELETE');
    await confirm.getByRole('button', { name: 'Delete connection' }).click();
    expect((await del).ok()).toBe(true);
    await expect(helpers.connectionRow(source.name)).toHaveCount(0);
  });
});
