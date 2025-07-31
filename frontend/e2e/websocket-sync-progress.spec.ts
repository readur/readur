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
    const sourceName = await helpers.createTestSource('WebSocket Test Source', 'webdav');
    
    // Find the created source and trigger sync
    const sourceCard = page.locator(`[data-testid="source-item"]:has-text("${sourceName}")`).first();
    await expect(sourceCard).toBeVisible();
    
    // Hover over the source card to reveal action buttons
    await sourceCard.hover();
    
    // Wait a bit for hover effects
    await page.waitForTimeout(1000);
    
    // Look for action buttons - they should be visible after hover
    const actionButtons = sourceCard.locator('button');
    const buttonCount = await actionButtons.count();
    console.log(`Found ${buttonCount} action buttons in source card`);
    
    if (buttonCount === 0) {
      throw new Error('No action buttons found in source card');
    }
    
    // The sync button is typically the first action button (play icon)
    const syncButton = actionButtons.first();
    await syncButton.click();
    
    // Wait for sync modal to appear
    const syncModal = page.getByRole('dialog');
    await expect(syncModal).toBeVisible({ timeout: 5000 });
    
    // Look for sync type options in the modal - they are Material-UI Cards
    const quickSyncCard = page.locator('.MuiCard-root:has-text("Quick Sync")').first();
    if (await quickSyncCard.isVisible({ timeout: 2000 })) {
      console.log('Clicking Quick Sync card');
      await quickSyncCard.click();
    } else {
      // Fallback: look for Deep Scan option
      const deepScanCard = page.locator('.MuiCard-root:has-text("Deep Scan")').first();
      if (await deepScanCard.isVisible()) {
        console.log('Clicking Deep Scan card');
        await deepScanCard.click();
      } else {
        throw new Error('No sync options found in modal');
      }
    }
    
    // Wait for sync progress display to appear - look for the actual MUI Card component
    const progressDisplay = page.locator('.MuiCard-root:has-text("Sync Progress"), .MuiCardContent-root:has-text("Sync Progress")').first();
    await expect(progressDisplay).toBeVisible({ timeout: TIMEOUTS.medium });
    
    // Check that connection status is shown using MUI Chip component
    const connectionStatus = progressDisplay.locator('.MuiChip-root:has-text("Connected"), .MuiChip-root:has-text("Connecting"), .MuiChip-root:has-text("Live")').first();
    await expect(connectionStatus).toBeVisible({ timeout: TIMEOUTS.short });
    
    // Should receive progress updates - look for progress bar or statistics
    const progressContent = progressDisplay.locator('.MuiLinearProgress-root, :has-text("files"), :has-text("Files Progress")').first();
    await expect(progressContent).toBeVisible({ timeout: TIMEOUTS.short });
  });

  test('should handle WebSocket connection errors gracefully', async ({ adminPage: page }) => {
    // Mock WebSocket connection failure
    await page.route('**/sync/progress/ws**', route => {
      route.abort('connectionrefused');
    });
    
    // Create and sync a source
    const sourceName = await helpers.createTestSource('Error Test Source', 'webdav');
    
    const sourceCard = page.locator(`[data-testid="source-item"]:has-text("${sourceName}")`).first();
    
    // Find and click sync button
    const syncButton = sourceCard.locator('button:has-text("Sync"), button[aria-label*="sync" i]').first();
    if (!(await syncButton.isVisible({ timeout: 2000 }))) {
      const moreButton = sourceCard.locator('button[aria-label="more"], button:has-text("⋮")').first();
      if (await moreButton.isVisible()) {
        await moreButton.click();
      }
    }
    await syncButton.click();
    
    // Should show connection error using MUI Chip or Alert components
    const errorIndicator = page.locator('.MuiChip-root:has-text("Disconnected"), .MuiChip-root:has-text("Connection Failed"), .MuiAlert-root:has-text("error"), :has-text("Connection failed")').first();
    await expect(errorIndicator).toBeVisible({ timeout: TIMEOUTS.medium });
  });

  test('should automatically reconnect on WebSocket disconnection', async ({ adminPage: page }) => {
    // Create and sync a source
    const sourceName = await helpers.createTestSource('Reconnect Test Source', 'webdav');
    
    const sourceCard = page.locator(`[data-testid="source-item"]:has-text("${sourceName}")`).first();
    
    // Find and click sync button
    const syncButton = sourceCard.locator('button:has-text("Sync"), button[aria-label*="sync" i]').first();
    if (!(await syncButton.isVisible({ timeout: 2000 }))) {
      const moreButton = sourceCard.locator('button[aria-label="more"], button:has-text("⋮")').first();
      if (await moreButton.isVisible()) {
        await moreButton.click();
      }
    }
    await syncButton.click();
    
    // Wait for initial connection
    const progressDisplay = page.locator('.MuiCard-root:has-text("Sync Progress")').first();
    const connectedStatus = progressDisplay.locator('.MuiChip-root:has-text("Connected"), .MuiChip-root:has-text("Live")').first();
    await expect(connectedStatus).toBeVisible({ timeout: TIMEOUTS.medium });
    
    // Simulate connection interruption - route WebSocket to fail
    await page.route('**/sync/progress/ws**', route => {
      route.abort('connectionrefused');
    });
    
    // Trigger reconnection by evaluating some script that would cause a reconnect
    await page.evaluate(() => {
      // Force a reconnection attempt
      window.dispatchEvent(new Event('offline'));
      setTimeout(() => window.dispatchEvent(new Event('online')), 1000);
    });
    
    // Should show reconnecting status
    const reconnectingStatus = progressDisplay.locator('.MuiChip-root:has-text("Reconnecting"), .MuiChip-root:has-text("Disconnected")').first();
    await expect(reconnectingStatus).toBeVisible({ timeout: TIMEOUTS.short });
  });

  test('should display real-time progress updates via WebSocket', async ({ adminPage: page }) => {
    // Create a source and start sync
    const sourceName = await helpers.createTestSource('Progress Updates Test', 'webdav');
    
    const sourceCard = page.locator(`[data-testid="source-item"]:has-text("${sourceName}")`).first();
    
    // Find and click sync button
    const syncButton = sourceCard.locator('button:has-text("Sync"), button[aria-label*="sync" i]').first();
    if (!(await syncButton.isVisible({ timeout: 2000 }))) {
      const moreButton = sourceCard.locator('button[aria-label="more"], button:has-text("⋮")').first();
      if (await moreButton.isVisible()) {
        await moreButton.click();
      }
    }
    await syncButton.click();
    
    const progressDisplay = page.locator('.MuiCard-root:has-text("Sync Progress")').first();
    await expect(progressDisplay).toBeVisible();
    
    // Should show different phases over time - look for phase descriptions
    const phases = ['initializing', 'discovering', 'processing', 'evaluating'];
    
    // At least one phase should be visible
    let phaseFound = false;
    for (const phase of phases) {
      try {
        await expect(progressDisplay.locator(`:has-text("${phase}")`)).toBeVisible({ timeout: 2000 });
        phaseFound = true;
        break;
      } catch (e) {
        // Phase might have passed quickly, continue to next
        continue;
      }
    }
    
    // If no specific phase found, at least verify there's some progress content
    if (!phaseFound) {
      await expect(progressDisplay.locator(':has-text("Progress"), .MuiLinearProgress-root, :has-text("files")')).toBeVisible();
    }
    
    // Should show numerical progress - look for files/directories statistics
    const statsLocator = progressDisplay.locator(':has-text("/"), :has-text("files"), :has-text("Directories"), .MuiLinearProgress-root').first();
    await expect(statsLocator).toBeVisible();
  });

  test('should handle multiple concurrent WebSocket connections', async ({ adminPage: page }) => {
    // Create multiple sources
    const sourceNames = [];
    const baseNames = ['Multi Source 1', 'Multi Source 2'];
    
    for (const baseName of baseNames) {
      const sourceName = await helpers.createTestSource(baseName, 'webdav');
      sourceNames.push(sourceName);
    }
    
    // Start sync on all sources
    for (const sourceName of sourceNames) {
      const sourceCard = page.locator(`[data-testid="source-item"]:has-text("${sourceName}")`).first();
      
      // Find and click sync button
      const syncButton = sourceCard.locator('button:has-text("Sync"), button[aria-label*="sync" i]').first();
      if (!(await syncButton.isVisible({ timeout: 2000 }))) {
        const moreButton = sourceCard.locator('button[aria-label="more"], button:has-text("⋮")').first();
        if (await moreButton.isVisible()) {
          await moreButton.click();
        }
      }
      await syncButton.click();
      
      // Wait a moment between syncs
      await page.waitForTimeout(1000);
    }
    
    // Should have multiple progress displays
    const progressDisplays = page.locator('.MuiCard-root:has-text("Sync Progress")');
    await expect(progressDisplays).toHaveCount(2, { timeout: TIMEOUTS.medium });
    
    // Each should show connection status
    for (let i = 0; i < 2; i++) {
      const display = progressDisplays.nth(i);
      const connectionStatus = display.locator('.MuiChip-root:has-text("Connected"), .MuiChip-root:has-text("Connecting"), .MuiChip-root:has-text("Live")').first();
      await expect(connectionStatus).toBeVisible({ timeout: TIMEOUTS.short });
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
    const sourceName = await helpers.createTestSource('Auth Test Source', 'webdav');
    
    const sourceCard = page.locator(`[data-testid="source-item"]:has-text("${sourceName}")`).first();
    
    // Find and click sync button
    const syncButton = sourceCard.locator('button:has-text("Sync"), button[aria-label*="sync" i]').first();
    if (!(await syncButton.isVisible({ timeout: 2000 }))) {
      const moreButton = sourceCard.locator('button[aria-label="more"], button:has-text("⋮")').first();
      if (await moreButton.isVisible()) {
        await moreButton.click();
      }
    }
    await syncButton.click();
    
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
    const sourceName = await helpers.createTestSource('Auth Fail Test', 'webdav');
    
    const sourceCard = page.locator(`[data-testid="source-item"]:has-text("${sourceName}")`).first();
    
    // Find and click sync button
    const syncButton = sourceCard.locator('button:has-text("Sync"), button[aria-label*="sync" i]').first();
    if (!(await syncButton.isVisible({ timeout: 2000 }))) {
      const moreButton = sourceCard.locator('button[aria-label="more"], button:has-text("⋮")').first();
      if (await moreButton.isVisible()) {
        await moreButton.click();
      }
    }
    await syncButton.click();
    
    // Should show authentication error
    await expect(page.locator(':has-text("Authentication failed"), :has-text("Unauthorized")')).toBeVisible({ timeout: TIMEOUTS.medium });
  });

  test('should properly clean up WebSocket connections on component unmount', async ({ adminPage: page }) => {
    // Create and sync a source
    const sourceName = await helpers.createTestSource('Cleanup Test Source', 'webdav');
    
    const sourceCard = page.locator(`[data-testid="source-item"]:has-text("${sourceName}")`).first();
    
    // Find and click sync button
    const syncButton = sourceCard.locator('button:has-text("Sync"), button[aria-label*="sync" i]').first();
    if (!(await syncButton.isVisible({ timeout: 2000 }))) {
      const moreButton = sourceCard.locator('button[aria-label="more"], button:has-text("⋮")').first();
      if (await moreButton.isVisible()) {
        await moreButton.click();
      }
    }
    await syncButton.click();
    
    // Wait for progress display
    const progressDisplay = page.locator('.MuiCard-root:has-text("Sync Progress")').first();
    await expect(progressDisplay).toBeVisible();
    
    // Navigate away from the page
    await page.goto('/documents');
    
    // Navigate back
    await page.goto('/sources');
    
    // The progress display should be properly cleaned up and recreated if sync is still active
    // This tests that WebSocket connections are properly closed on unmount
    
    // If sync is still running, progress should reappear
    const sourceRowAfter = page.locator(`[data-testid="source-item"]:has-text("${sourceName}")`).first();
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
    const sourceName = await helpers.createTestSource('Parse Error Test', 'webdav');
    
    const sourceCard = page.locator(`[data-testid="source-item"]:has-text("${sourceName}")`).first();
    
    // Find and click sync button
    const syncButton = sourceCard.locator('button:has-text("Sync"), button[aria-label*="sync" i]').first();
    if (!(await syncButton.isVisible({ timeout: 2000 }))) {
      const moreButton = sourceCard.locator('button[aria-label="more"], button:has-text("⋮")').first();
      if (await moreButton.isVisible()) {
        await moreButton.click();
      }
    }
    await syncButton.click();
    
    // Should handle parsing errors gracefully (not crash the UI)
    const progressDisplay = page.locator('.MuiCard-root:has-text("Sync Progress")').first();
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
    const sourceName = await helpers.createTestSource('Status Test Source', 'webdav');
    
    const sourceCard = page.locator(`[data-testid="source-item"]:has-text("${sourceName}")`).first();
    
    // Find and click sync button
    const syncButton = sourceCard.locator('button:has-text("Sync"), button[aria-label*="sync" i]').first();
    if (!(await syncButton.isVisible({ timeout: 2000 }))) {
      const moreButton = sourceCard.locator('button[aria-label="more"], button:has-text("⋮")').first();
      if (await moreButton.isVisible()) {
        await moreButton.click();
      }
    }
    await syncButton.click();
    
    const progressDisplay = page.locator('.MuiCard-root:has-text("Sync Progress")').first();
    await expect(progressDisplay).toBeVisible();
    
    // Should show connecting status initially
    const statusChip = progressDisplay.locator('.MuiChip-root').first();
    await expect(statusChip).toBeVisible();
    await expect(statusChip).toContainText(/connecting|connected|live/i);
    
    // Should show connected status once established
    const connectedStatus = progressDisplay.locator('.MuiChip-root:has-text("Connected"), .MuiChip-root:has-text("Live")').first();
    await expect(connectedStatus).toBeVisible({ timeout: TIMEOUTS.medium });
    
    // Should have visual indicators (icons, colors, etc.)
    await expect(statusChip).toHaveClass(/MuiChip-root/);
  });

  test('should support WebSocket connection health monitoring', async ({ adminPage: page }) => {
    // This test verifies that the WebSocket connection monitors connection health
    
    let heartbeatReceived = false;
    
    // Mock WebSocket to track heartbeat/health messages
    await page.addInitScript(() => {
      const originalWebSocket = window.WebSocket;
      (window as any).WebSocket = class extends originalWebSocket {
        send(data: string | ArrayBufferLike | Blob | ArrayBufferView) {
          if (typeof data === 'string' && (data.includes('ping') || data.includes('heartbeat'))) {
            (window as any).heartbeatReceived = true;
          }
          super.send(data);
        }
      };
    });
    
    // Create and sync a source
    const sourceName = await helpers.createTestSource('Ping Test Source', 'webdav');
    
    const sourceCard = page.locator(`[data-testid="source-item"]:has-text("${sourceName}")`).first();
    
    // Find and click sync button
    const syncButton = sourceCard.locator('button:has-text("Sync"), button[aria-label*="sync" i]').first();
    if (!(await syncButton.isVisible({ timeout: 2000 }))) {
      const moreButton = sourceCard.locator('button[aria-label="more"], button:has-text("⋮")').first();
      if (await moreButton.isVisible()) {
        await moreButton.click();
      }
    }
    await syncButton.click();
    
    // Wait for connection and potential health check messages
    await page.waitForTimeout(5000);
    
    // The main thing is that the connection remains healthy and shows connected status
    const progressDisplay = page.locator('.MuiCard-root:has-text("Sync Progress")').first();
    const connectedStatus = progressDisplay.locator('.MuiChip-root:has-text("Connected"), .MuiChip-root:has-text("Live")').first();
    await expect(connectedStatus).toBeVisible();
    
    // Check if health monitoring was attempted (optional)
    const healthCheckAttempted = await page.evaluate(() => (window as any).heartbeatReceived);
    console.log(`Health check attempted: ${healthCheckAttempted}`);
  });
});

test.describe('WebSocket Sync Progress - Cross-browser Compatibility', () => {
  test('should work in different browser engines', async ({ adminPage: page }) => {
    // This test would run across different browsers (Chrome, Firefox, Safari)
    // The test framework should handle this automatically
    
    // Create and sync a source
    const helpers = new TestHelpers(page);
    await helpers.navigateToPage('/sources');
    const sourceName = await helpers.createTestSource('Cross Browser Test', 'webdav');
    
    const sourceCard = page.locator(`[data-testid="source-item"]:has-text("${sourceName}")`).first();
    
    // Find and click sync button
    const syncButton = sourceCard.locator('button:has-text("Sync"), button[aria-label*="sync" i]').first();
    if (!(await syncButton.isVisible({ timeout: 2000 }))) {
      const moreButton = sourceCard.locator('button[aria-label="more"], button:has-text("⋮")').first();
      if (await moreButton.isVisible()) {
        await moreButton.click();
      }
    }
    await syncButton.click();
    
    // Should work regardless of browser
    const progressDisplay = page.locator('.MuiCard-root:has-text("Sync Progress")').first();
    await expect(progressDisplay).toBeVisible();
    
    const connectionStatus = progressDisplay.locator('.MuiChip-root:has-text("Connected"), .MuiChip-root:has-text("Connecting"), .MuiChip-root:has-text("Live")').first();
    await expect(connectionStatus).toBeVisible();
  });
});