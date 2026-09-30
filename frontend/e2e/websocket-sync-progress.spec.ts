import { test, expect } from './fixtures/auth';
import type { Locator, Page, WebSocketRoute } from '@playwright/test';
import { TIMEOUTS } from './utils/test-data';
import { TestHelpers } from './utils/test-helpers';

/**
 * Live sync progress in Intake → Connections → a connection's detail panel. The panel shows
 * "<name> sync progress" while the connection is syncing, fed by
 * /api/sources/:id/sync/progress/ws. The connection word (Connecting…/Connected/Live/
 * Reconnecting…/Disconnected/Connection failed) is a role=status inside that region.
 *
 * Most tests route the WebSocket (page.routeWebSocket) so progress is deterministic; the
 * first one talks to the real server.
 */

const WS_PATTERN = /\/api\/sources\/[^/]+\/sync\/progress\/ws/;

function progress(sourceId: string, overrides: Record<string, unknown> = {}) {
  return JSON.stringify({
    type: 'progress',
    data: {
      source_id: sourceId,
      phase: 'processing_files',
      phase_description: 'Processing files',
      elapsed_time_secs: 12,
      directories_found: 4,
      directories_processed: 2,
      files_found: 10,
      files_processed: 3,
      bytes_processed: 2048,
      processing_rate_files_per_sec: 1.5,
      files_progress_percent: 30,
      estimated_time_remaining_secs: 20,
      current_directory: '/Documents/2026',
      current_file: 'invoice.pdf',
      errors: 0,
      warnings: 0,
      is_active: true,
      ...overrides,
    },
  });
}

test.describe('WebSocket Sync Progress', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ dynamicAdminPage }) => {
    helpers = new TestHelpers(dynamicAdminPage);
    await dynamicAdminPage.goto('/board');
  });

  /** Report the given connections as syncing so their panels show live progress. */
  async function markSyncing(page: Page, ids: string[]) {
    await page.route('**/api/sources', async (route) => {
      if (route.request().method() !== 'GET') return route.continue();
      const response = await route.fetch();
      const list = await response.json();
      for (const s of list) if (ids.includes(s.id)) s.status = 'syncing';
      await route.fulfill({ response, json: list });
    });
  }

  /** Create a syncing connection and open its panel. Returns the progress region. */
  async function openSyncingConnection(page: Page, base: string): Promise<{ id: string; name: string; progressRegion: Locator; panel: Locator }> {
    const source = await helpers.createWebdavSourceViaAPI({ name: helpers.uniqueName(base) });
    await markSyncing(page, [source.id]);
    await helpers.openIntake('connections');
    const panel = await helpers.openConnection(source.name);
    return { ...source, panel, progressRegion: panel.getByRole('region', { name: `${source.name} sync progress` }) };
  }

  const statusWord = (region: Locator) => region.getByRole('status').first();

  test('should establish WebSocket connection for sync progress', async ({ dynamicAdminPage: page }) => {
    const sockets: string[] = [];
    page.on('websocket', (ws) => sockets.push(ws.url()));

    const source = await helpers.createWebdavSourceViaAPI({ name: helpers.uniqueName('WebSocket Test Source') });
    await helpers.openIntake('connections');
    const panel = await helpers.openConnection(source.name);

    // A real sync against the real server
    const sync = page.waitForResponse((r) => r.url().includes(`/api/sources/${source.id}/sync`) && r.request().method() === 'POST');
    await panel.getByRole('button', { name: 'Sync now' }).click();
    expect((await sync).ok()).toBe(true);

    const region = panel.getByRole('region', { name: `${source.name} sync progress` });
    await expect(region).toBeVisible({ timeout: TIMEOUTS.medium });
    await expect(statusWord(region)).toHaveText(/Connecting…|Connected|Live/);
    expect(sockets.some((u) => u.includes(`/api/sources/${source.id}/sync/progress/ws`))).toBe(true);
    await expect(statusWord(region)).toHaveText(/Connected|Live/, { timeout: TIMEOUTS.medium });
  });

  test('should handle WebSocket connection errors gracefully', async ({ dynamicAdminPage: page }) => {
    await page.routeWebSocket(WS_PATTERN, (ws) => {
      const id = ws.url().split('/sources/')[1].split('/')[0];
      ws.send(progress(id));
      setTimeout(() => ws.close({ code: 4000, reason: 'test failure' }), 300);
    });
    const { progressRegion } = await openSyncingConnection(page, 'Error Test Source');

    await expect(progressRegion).toBeVisible();
    await expect(statusWord(progressRegion)).toHaveText(/Reconnecting…|Disconnected|Connection failed/, { timeout: TIMEOUTS.medium });
    // The last progress stays on screen
    await expect(progressRegion.getByRole('group', { name: 'Sync figures' })).toBeVisible();
  });

  test('should automatically reconnect on WebSocket disconnection', async ({ dynamicAdminPage: page }) => {
    let connections = 0;
    await page.routeWebSocket(WS_PATTERN, (ws) => {
      connections += 1;
      const id = ws.url().split('/sources/')[1].split('/')[0];
      if (connections === 1) {
        ws.send(progress(id, { is_active: false }));
        setTimeout(() => ws.close({ code: 4001, reason: 'dropped' }), 300);
      } else {
        ws.send(progress(id, { files_processed: 7, files_progress_percent: 70 }));
      }
    });
    const { progressRegion } = await openSyncingConnection(page, 'Reconnect Test Source');

    await expect(progressRegion).toBeVisible();
    await expect.poll(() => connections, { timeout: TIMEOUTS.medium }).toBeGreaterThanOrEqual(2);
    await expect(statusWord(progressRegion)).toHaveText(/Live$/, { timeout: TIMEOUTS.medium });
    await expect(progressRegion).toContainText('7 / 10 files (70.0%)');
  });

  test('should display real-time progress updates via WebSocket', async ({ dynamicAdminPage: page }) => {
    let socket: WebSocketRoute | null = null;
    await page.routeWebSocket(WS_PATTERN, (ws) => {
      socket = ws;
      ws.send(progress(ws.url().split('/sources/')[1].split('/')[0], { phase_description: 'Discovering directories', files_processed: 0, files_found: 0, files_progress_percent: 0 }));
    });
    const { id, progressRegion } = await openSyncingConnection(page, 'Progress Updates Test');

    const figures = progressRegion.getByRole('group', { name: 'Sync figures' });
    await expect(figures).toContainText('Discovering directories');

    socket!.send(progress(id));
    await expect(figures).toContainText('Processing files');
    await expect(figures).toContainText('3 / 10 files (30.0%)');
    await expect(figures).toContainText('2 / 4');
    await expect(progressRegion.getByRole('progressbar', { name: 'Files progress' })).toBeVisible();
    await expect(progressRegion).toContainText('/Documents/2026');
    await expect(progressRegion).toContainText('invoice.pdf');

    socket!.send(progress(id, { files_processed: 10, files_progress_percent: 100, errors: 2, warnings: 1 }));
    await expect(figures).toContainText('10 / 10 files (100.0%)');
    await expect(progressRegion).toContainText('2 errors');
    await expect(progressRegion).toContainText('1 warning');
  });

  test('should handle multiple concurrent WebSocket connections', async ({ dynamicAdminPage: page }) => {
    const urls = new Set<string>();
    await page.routeWebSocket(WS_PATTERN, (ws) => {
      urls.add(ws.url());
      ws.send(progress(ws.url().split('/sources/')[1].split('/')[0]));
    });
    const a = await helpers.createWebdavSourceViaAPI({ name: helpers.uniqueName('Multi Source 1') });
    const b = await helpers.createWebdavSourceViaAPI({ name: helpers.uniqueName('Multi Source 2') });
    await markSyncing(page, [a.id, b.id]);
    await helpers.openIntake('connections');

    // Both rows report syncing
    await expect(helpers.connectionRow(a.name)).toContainText('SYNCING');
    await expect(helpers.connectionRow(b.name)).toContainText('SYNCING');

    for (const s of [a, b]) {
      const panel = await helpers.openConnection(s.name);
      await expect(panel.getByRole('region', { name: `${s.name} sync progress` }).getByRole('status').first()).toHaveText(/Live$/);
      await panel.getByRole('button', { name: 'Close' }).click();
      await expect(panel).toBeHidden();
    }
    expect([...urls].some((u) => u.includes(a.id))).toBe(true);
    expect([...urls].some((u) => u.includes(b.id))).toBe(true);
  });

  test('should authenticate WebSocket connection with JWT token', async ({ dynamicAdminPage: page }) => {
    await page.routeWebSocket(WS_PATTERN, (ws) => ws.send(progress(ws.url().split('/sources/')[1].split('/')[0])));
    // Record the subprotocols the app offers (installed after the route so it wraps it)
    await page.addInitScript(() => {
      const Current = window.WebSocket;
      (window as any).__wsProtocols = [];
      (window as any).WebSocket = class extends Current {
        constructor(url: string | URL, protocols?: string | string[]) {
          (window as any).__wsProtocols.push(protocols ?? null);
          super(url, protocols);
        }
      };
    });
    const token = await helpers.getAuthToken();
    expect(token.split('.')).toHaveLength(3);

    const { progressRegion } = await openSyncingConnection(page, 'Auth Test Source');
    await expect(statusWord(progressRegion)).toHaveText(/Live$/);

    const protocols = await page.evaluate(() => (window as any).__wsProtocols as (string[] | null)[]);
    expect(protocols.flat()).toContain(`bearer.${token}`);
  });

  test('should handle WebSocket authentication failures', async ({ dynamicAdminPage: page }) => {
    let connections = 0;
    await page.routeWebSocket(WS_PATTERN, (ws) => {
      connections += 1;
      ws.send(progress(ws.url().split('/sources/')[1].split('/')[0], { is_active: false }));
      // 1008 = policy violation, what the server sends for a bad token
      setTimeout(() => ws.close({ code: 1008, reason: 'Unauthorized' }), 200);
    });
    const { progressRegion } = await openSyncingConnection(page, 'Auth Fail Test');

    // The panel says the feed is down instead of pretending to be live, and keeps the last figures
    await expect(statusWord(progressRegion)).toHaveText(/Reconnecting…$|Disconnected$|Connection failed$/, { timeout: TIMEOUTS.medium });
    await expect(progressRegion.getByRole('group', { name: 'Sync figures' })).toBeVisible();
    expect(connections).toBeGreaterThanOrEqual(1);
  });

  test('should properly clean up WebSocket connections on component unmount', async ({ dynamicAdminPage: page }) => {
    let closedByClient = false;
    await page.routeWebSocket(WS_PATTERN, (ws) => {
      ws.onClose(() => {
        closedByClient = true;
      });
      ws.send(progress(ws.url().split('/sources/')[1].split('/')[0]));
    });
    const { panel, progressRegion } = await openSyncingConnection(page, 'Cleanup Test');
    await expect(statusWord(progressRegion)).toHaveText(/Live$/);

    await panel.getByRole('button', { name: 'Close' }).click();
    await expect(panel).toBeHidden();
    await expect.poll(() => closedByClient, { timeout: TIMEOUTS.short }).toBe(true);
  });

  test('should handle WebSocket message parsing errors', async ({ dynamicAdminPage: page }) => {
    const errors: string[] = [];
    page.on('pageerror', (e) => errors.push(e.message));
    await page.routeWebSocket(WS_PATTERN, (ws) => {
      ws.send('invalid json {malformed');
      ws.send(progress(ws.url().split('/sources/')[1].split('/')[0]));
    });
    const { progressRegion } = await openSyncingConnection(page, 'Parse Error Test');

    // The bad frame is ignored; the good one still renders
    await expect(progressRegion.getByRole('group', { name: 'Sync figures' })).toContainText('3 / 10 files (30.0%)');
    expect(errors).toEqual([]);
  });

  test('should display WebSocket connection status indicators', async ({ dynamicAdminPage: page }) => {
    let socket: WebSocketRoute | null = null;
    await page.routeWebSocket(WS_PATTERN, (ws) => {
      socket = ws;
    });
    const { id, progressRegion } = await openSyncingConnection(page, 'Status Test Source');

    // Open but no progress yet: "Connected" with a waiting note
    await expect(statusWord(progressRegion)).toHaveText(/Connected$/);
    await expect(progressRegion).toContainText('Waiting for sync progress information…');

    // An active sync turns it "Live" and marks the region SYNCING
    socket!.send(progress(id));
    await expect(statusWord(progressRegion)).toHaveText(/Live$/);
    await expect(progressRegion).toContainText('SYNCING');

    // The region folds away and back
    const collapse = progressRegion.getByRole('button', { name: 'Collapse' });
    await collapse.click();
    await expect(progressRegion.getByRole('group', { name: 'Sync figures' })).toBeHidden();
    await progressRegion.getByRole('button', { name: 'Expand' }).click();
    await expect(progressRegion.getByRole('group', { name: 'Sync figures' })).toBeVisible();
  });

  test('should support WebSocket connection health monitoring', async ({ dynamicAdminPage: page }) => {
    let socket: WebSocketRoute | null = null;
    await page.routeWebSocket(WS_PATTERN, (ws) => {
      socket = ws;
      ws.send(progress(ws.url().split('/sources/')[1].split('/')[0]));
    });
    const { progressRegion } = await openSyncingConnection(page, 'Ping Test Source');
    await expect(statusWord(progressRegion)).toHaveText(/Live$/);

    // Heartbeats keep the connection healthy without changing the figures
    for (let i = 0; i < 3; i++) {
      socket!.send(JSON.stringify({ type: 'heartbeat', data: { source_id: 'x', is_active: true, timestamp: Date.now() } }));
      await page.waitForTimeout(300);
    }
    await expect(statusWord(progressRegion)).toHaveText(/Live$/);
    await expect(progressRegion.getByRole('group', { name: 'Sync figures' })).toContainText('3 / 10 files (30.0%)');

    // A heartbeat saying the sync is no longer active clears the figures
    socket!.send(JSON.stringify({ type: 'heartbeat', data: { source_id: 'x', is_active: false, timestamp: Date.now() } }));
    await expect(progressRegion).toContainText('Waiting for sync progress information…');
  });
});

test.describe('WebSocket Sync Progress - Cross-browser Compatibility', () => {
  test('should work in different browser engines', async ({ dynamicAdminPage: page }) => {
    const helpers = new TestHelpers(page);
    await page.goto('/board');
    const source = await helpers.createWebdavSourceViaAPI({ name: helpers.uniqueName('Cross Browser Test') });
    await page.route('**/api/sources', async (route) => {
      if (route.request().method() !== 'GET') return route.continue();
      const response = await route.fetch();
      const list = await response.json();
      for (const s of list) if (s.id === source.id) s.status = 'syncing';
      await route.fulfill({ response, json: list });
    });
    await page.routeWebSocket(WS_PATTERN, (ws) => ws.send(progress(source.id)));

    await helpers.openIntake('connections');
    const panel = await helpers.openConnection(source.name);
    const region = panel.getByRole('region', { name: `${source.name} sync progress` });
    await expect(region).toBeVisible({ timeout: TIMEOUTS.medium });
    await expect(region.getByRole('status').first()).toHaveText(/Connected|Connecting…|Live/);
  });
});
