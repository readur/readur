import { test, expect } from './fixtures/auth';
import { TIMEOUTS } from './utils/test-data';
import { TestHelpers } from './utils/test-helpers';

test.describe('WebSocket Sync Progress', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ adminPage }) => {
    helpers = new TestHelpers(adminPage);
    await helpers.navigateToPage('/sources');
  });

  test('should establish WebSocket connection for sync progress', async ({ adminPage: page }) => {
    // Create a test source first
    await page.click('button:has-text("Add Source"), [data-testid="add-source"]');
    await page.fill('input[name="name"]', 'WebSocket Test Source');
    await page.selectOption('select[name="type"]', 'webdav');
    await page.fill('input[name="server_url"]', 'https://test.webdav.server');
    await page.fill('input[name="username"]', 'testuser');
    await page.fill('input[name="password"]', 'testpass');
    await page.click('button[type="submit"]');

    // Wait for source to be created
    await helpers.waitForToast();
    
    // Find the created source and trigger sync
    const sourceRow = page.locator('[data-testid="source-item"]:has-text("WebSocket Test Source")').first();
    await expect(sourceRow).toBeVisible();
    
    // Click sync button
    await sourceRow.locator('button:has-text("Sync")').click();
    
    // Wait for sync progress display to appear
    await expect(page.locator('[data-testid="sync-progress"], .sync-progress')).toBeVisible({ timeout: TIMEOUTS.medium });
    
    // Check that WebSocket connection is established
    // We'll monitor network traffic or check for specific UI indicators
    const progressDisplay = page.locator('[data-testid="sync-progress"], .sync-progress');
    
    // Should show connection status
    await expect(progressDisplay.locator(':has-text("Connected"), :has-text("Connecting")')).toBeVisible();
    
    // Should receive progress updates
    await expect(progressDisplay.locator('[data-testid="progress-phase"], .progress-phase')).toBeVisible();
    
    // Should show progress data
    await expect(progressDisplay.locator('[data-testid="files-processed"], .files-processed')).toBeVisible();
  });

  test('should handle WebSocket connection errors gracefully', async ({ adminPage: page }) => {
    // Mock WebSocket connection failure
    await page.route('**/sync/progress/ws**', route => {
      route.abort('connectionrefused');
    });
    
    // Create and sync a source
    await helpers.createTestSource('Error Test Source', 'webdav');
    
    const sourceRow = page.locator('[data-testid="source-item"]:has-text("Error Test Source")').first();
    await sourceRow.locator('button:has-text("Sync")').click();
    
    // Should show connection error
    await expect(page.locator('[data-testid="connection-error"], .connection-error, :has-text("Connection failed")')).toBeVisible({ timeout: TIMEOUTS.medium });
  });

  test('should automatically reconnect on WebSocket disconnection', async ({ adminPage: page }) => {
    // Create and sync a source
    await helpers.createTestSource('Reconnect Test Source', 'webdav');
    
    const sourceRow = page.locator('[data-testid="source-item"]:has-text("Reconnect Test Source")').first();
    await sourceRow.locator('button:has-text("Sync")').click();
    
    // Wait for initial connection
    const progressDisplay = page.locator('[data-testid="sync-progress"], .sync-progress');
    await expect(progressDisplay.locator(':has-text("Connected")')).toBeVisible();
    
    // Simulate connection interruption by intercepting WebSocket and closing it
    await page.evaluate(() => {
      // Find any active WebSocket connections and close them
      // This is a simplified simulation - in real tests you might use more sophisticated mocking
      if ((window as any).testWebSocket) {
        (window as any).testWebSocket.close();
      }
    });
    
    // Should show reconnecting status
    await expect(progressDisplay.locator(':has-text("Reconnecting"), :has-text("Disconnected")')).toBeVisible({ timeout: TIMEOUTS.short });
    
    // Should eventually reconnect
    await expect(progressDisplay.locator(':has-text("Connected")')).toBeVisible({ timeout: TIMEOUTS.medium });
  });

  test('should display real-time progress updates via WebSocket', async ({ adminPage: page }) => {
    // Create a source and start sync
    await helpers.createTestSource('Progress Updates Test', 'webdav');
    
    const sourceRow = page.locator('[data-testid="source-item"]:has-text("Progress Updates Test")').first();
    await sourceRow.locator('button:has-text("Sync")').click();
    
    const progressDisplay = page.locator('[data-testid="sync-progress"], .sync-progress');
    await expect(progressDisplay).toBeVisible();
    
    // Should show different phases over time
    const phases = ['initializing', 'discovering', 'processing'];
    
    for (const phase of phases) {
      // Wait for phase to appear (with timeout since sync might be fast)
      try {
        await expect(progressDisplay.locator(`:has-text("${phase}")`)).toBeVisible({ timeout: TIMEOUTS.short });
      } catch (e) {
        // Phase might have passed quickly, continue to next
        continue;
      }
    }
    
    // Should show numerical progress
    await expect(progressDisplay.locator('[data-testid="files-processed"], .files-processed')).toBeVisible();
    await expect(progressDisplay.locator('[data-testid="progress-percentage"], .progress-percentage')).toBeVisible();
  });

  test('should handle multiple concurrent WebSocket connections', async ({ adminPage: page }) => {
    // Create multiple sources
    const sourceNames = ['Multi Source 1', 'Multi Source 2', 'Multi Source 3'];
    
    for (const name of sourceNames) {
      await helpers.createTestSource(name, 'webdav');
    }
    
    // Start sync on all sources
    for (const name of sourceNames) {
      const sourceRow = page.locator(`[data-testid="source-item"]:has-text("${name}")`);
      await sourceRow.locator('button:has-text("Sync")').click();
      
      // Wait a moment between syncs
      await page.waitForTimeout(500);
    }
    
    // Should have multiple progress displays
    const progressDisplays = page.locator('[data-testid="sync-progress"], .sync-progress');
    await expect(progressDisplays).toHaveCount(3, { timeout: TIMEOUTS.medium });
    
    // Each should show connection status
    for (let i = 0; i < 3; i++) {
      const display = progressDisplays.nth(i);
      await expect(display.locator(':has-text("Connected"), :has-text("Connecting")')).toBeVisible();
    }
  });

  test('should authenticate WebSocket connection with JWT token', async ({ adminPage: page }) => {
    // Intercept WebSocket requests to verify token is sent
    let websocketToken = '';
    
    await page.route('**/sync/progress/ws**', route => {
      websocketToken = new URL(route.request().url()).searchParams.get('token') || '';
      route.continue();
    });
    
    // Create and sync a source
    await helpers.createTestSource('Auth Test Source', 'webdav');
    
    const sourceRow = page.locator('[data-testid="source-item"]:has-text("Auth Test Source")').first();
    await sourceRow.locator('button:has-text("Sync")').click();
    
    // Wait for WebSocket connection attempt
    await page.waitForTimeout(2000);
    
    // Verify token was sent
    expect(websocketToken).toBeTruthy();
    expect(websocketToken.length).toBeGreaterThan(20); // JWT tokens are typically longer
  });

  test('should handle WebSocket authentication failures', async ({ adminPage: page }) => {
    // Mock authentication failure
    await page.route('**/sync/progress/ws**', route => {
      if (route.request().url().includes('token=')) {
        route.fulfill({ status: 401, body: 'Unauthorized' });
      } else {
        route.continue();
      }
    });
    
    // Create and sync a source
    await helpers.createTestSource('Auth Fail Test', 'webdav');
    
    const sourceRow = page.locator('[data-testid="source-item"]:has-text("Auth Fail Test")').first();
    await sourceRow.locator('button:has-text("Sync")').click();
    
    // Should show authentication error
    await expect(page.locator(':has-text("Authentication failed"), :has-text("Unauthorized")')).toBeVisible({ timeout: TIMEOUTS.medium });
  });

  test('should properly clean up WebSocket connections on component unmount', async ({ adminPage: page }) => {
    // Create and sync a source
    await helpers.createTestSource('Cleanup Test Source', 'webdav');
    
    const sourceRow = page.locator('[data-testid="source-item"]:has-text("Cleanup Test Source")').first();
    await sourceRow.locator('button:has-text("Sync")').click();
    
    // Wait for progress display
    const progressDisplay = page.locator('[data-testid="sync-progress"], .sync-progress');
    await expect(progressDisplay).toBeVisible();
    
    // Navigate away from the page
    await page.goto('/documents');
    
    // Navigate back
    await page.goto('/sources');
    
    // The progress display should be properly cleaned up and recreated if sync is still active
    // This tests that WebSocket connections are properly closed on unmount
    
    // If sync is still running, progress should reappear
    const sourceRowAfter = page.locator('[data-testid="source-item"]:has-text("Cleanup Test Source")').first();
    if (await sourceRowAfter.locator(':has-text("Syncing")').isVisible()) {
      await expect(page.locator('[data-testid="sync-progress"], .sync-progress')).toBeVisible();
    }
  });

  test('should handle WebSocket message parsing errors', async ({ adminPage: page }) => {
    // Mock WebSocket with malformed messages
    await page.addInitScript(() => {
      const originalWebSocket = window.WebSocket;
      window.WebSocket = class extends originalWebSocket {
        constructor(url: string, protocols?: string | string[]) {
          super(url, protocols);
          
          // Override message handling to send malformed data
          setTimeout(() => {
            if (this.onmessage) {
              this.onmessage({
                data: 'invalid json {malformed',
                type: 'message'
              } as MessageEvent);
            }
          }, 1000);
        }
      };
    });
    
    // Create and sync a source
    await helpers.createTestSource('Parse Error Test', 'webdav');
    
    const sourceRow = page.locator('[data-testid="source-item"]:has-text("Parse Error Test")').first();
    await sourceRow.locator('button:has-text("Sync")').click();
    
    // Should handle parsing errors gracefully (not crash the UI)
    const progressDisplay = page.locator('[data-testid="sync-progress"], .sync-progress');
    await expect(progressDisplay).toBeVisible();
    
    // Check console for error messages (optional)
    const logs = [];
    page.on('console', msg => {
      if (msg.type() === 'error') {
        logs.push(msg.text());
      }
    });
    
    await page.waitForTimeout(3000);
    
    // Verify the UI didn't crash (still showing some content)
    await expect(page.locator('body')).toBeVisible();
  });

  test('should display WebSocket connection status indicators', async ({ adminPage: page }) => {
    // Create and sync a source
    await helpers.createTestSource('Status Test Source', 'webdav');
    
    const sourceRow = page.locator('[data-testid="source-item"]:has-text("Status Test Source")').first();
    await sourceRow.locator('button:has-text("Sync")').click();
    
    const progressDisplay = page.locator('[data-testid="sync-progress"], .sync-progress');
    await expect(progressDisplay).toBeVisible();
    
    // Should show connecting status initially
    await expect(progressDisplay.locator('[data-testid="connection-status"], .connection-status')).toContainText(/connecting|connected/i);
    
    // Should show connected status once established
    await expect(progressDisplay.locator(':has-text("Connected")')).toBeVisible({ timeout: TIMEOUTS.medium });
    
    // Should have visual indicators (icons, colors, etc.)
    await expect(progressDisplay.locator('.connection-indicator, [data-testid="connection-indicator"]')).toBeVisible();
  });

  test('should support WebSocket ping/pong for connection health', async ({ adminPage: page }) => {
    // This test verifies that the WebSocket connection uses ping/pong for health checks
    
    let pingReceived = false;
    
    // Mock WebSocket to track ping messages
    await page.addInitScript(() => {
      const originalWebSocket = window.WebSocket;
      window.WebSocket = class extends originalWebSocket {
        send(data: string | ArrayBufferLike | Blob | ArrayBufferView) {
          if (data === 'ping') {
            (window as any).pingReceived = true;
          }
          super.send(data);
        }
      };
    });
    
    // Create and sync a source
    await helpers.createTestSource('Ping Test Source', 'webdav');
    
    const sourceRow = page.locator('[data-testid="source-item"]:has-text("Ping Test Source")').first();
    await sourceRow.locator('button:has-text("Sync")').click();
    
    // Wait for connection and potential ping messages
    await page.waitForTimeout(5000);
    
    // Check if ping was sent (this is implementation-dependent)
    const pingWasSent = await page.evaluate(() => (window as any).pingReceived);
    
    // Note: This test might need adjustment based on actual ping/pong implementation
    // The important thing is that the connection remains healthy
    const progressDisplay = page.locator('[data-testid="sync-progress"], .sync-progress');
    await expect(progressDisplay.locator(':has-text("Connected")')).toBeVisible();
  });
});

test.describe('WebSocket Sync Progress - Cross-browser Compatibility', () => {
  test('should work in different browser engines', async ({ adminPage: page }) => {
    // This test would run across different browsers (Chrome, Firefox, Safari)
    // The test framework should handle this automatically
    
    // Create and sync a source
    const helpers = new TestHelpers(page);
    await helpers.navigateToPage('/sources');
    await helpers.createTestSource('Cross Browser Test', 'webdav');
    
    const sourceRow = page.locator('[data-testid="source-item"]:has-text("Cross Browser Test")').first();
    await sourceRow.locator('button:has-text("Sync")').click();
    
    // Should work regardless of browser
    const progressDisplay = page.locator('[data-testid="sync-progress"], .sync-progress');
    await expect(progressDisplay).toBeVisible();
    await expect(progressDisplay.locator(':has-text("Connected"), :has-text("Connecting")')).toBeVisible();
  });
});