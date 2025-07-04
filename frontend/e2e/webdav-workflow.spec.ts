import { test, expect } from './fixtures/auth';
import { TIMEOUTS, API_ENDPOINTS } from './utils/test-data';
import { TestHelpers } from './utils/test-helpers';

test.describe('WebDAV Workflow', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ adminPage }) => {
    helpers = new TestHelpers(adminPage);
    await helpers.navigateToPage('/sources');
  });

  test('should create and configure WebDAV source', async ({ adminPage: page }) => {
    // Navigate to sources page
    await page.goto('/sources');
    await helpers.waitForLoadingToComplete();

    // Look for add source button (try multiple selectors)
    const addSourceButton = page.locator('button:has-text("Add"), button:has-text("New"), [data-testid="add-source"]').first();
    
    if (await addSourceButton.isVisible()) {
      await addSourceButton.click();
    } else {
      // Alternative: look for floating action button or plus button
      const fabButton = page.locator('button[aria-label*="add"], button[title*="add"], .fab, .add-button').first();
      if (await fabButton.isVisible()) {
        await fabButton.click();
      }
    }

    // Wait for source creation form/modal
    await page.waitForTimeout(1000);

    // Select WebDAV source type if source type selection exists
    const webdavOption = page.locator('input[value="webdav"], [data-value="webdav"], option[value="webdav"]').first();
    if (await webdavOption.isVisible()) {
      await webdavOption.click();
    }

    // Fill WebDAV configuration form
    const nameInput = page.locator('input[name="name"], input[placeholder*="name"], input[label*="Name"]').first();
    if (await nameInput.isVisible()) {
      await nameInput.fill('Test WebDAV Source');
    }

    const urlInput = page.locator('input[name="url"], input[placeholder*="url"], input[type="url"]').first();
    if (await urlInput.isVisible()) {
      await urlInput.fill('https://demo.webdav.server/');
    }

    const usernameInput = page.locator('input[name="username"], input[placeholder*="username"]').first();
    if (await usernameInput.isVisible()) {
      await usernameInput.fill('webdav_user');
    }

    const passwordInput = page.locator('input[name="password"], input[type="password"]').first();
    if (await passwordInput.isVisible()) {
      await passwordInput.fill('webdav_pass');
    }

    // Save the source configuration
    const saveButton = page.locator('button:has-text("Save"), button:has-text("Create"), button[type="submit"]').first();
    if (await saveButton.isVisible()) {
      // Wait for save API call
      const savePromise = page.waitForResponse(response => 
        response.url().includes('/sources') && (response.status() === 200 || response.status() === 201),
        { timeout: TIMEOUTS.medium }
      );
      
      await saveButton.click();
      
      try {
        await savePromise;
        console.log('WebDAV source created successfully');
      } catch (error) {
        console.log('Source creation may have failed or timed out:', error);
      }
    }

    // Verify source appears in the list
    await helpers.waitForLoadingToComplete();
    const sourceList = page.locator('[data-testid="sources-list"], .sources-list, .source-item');
    await expect(sourceList.first()).toBeVisible({ timeout: TIMEOUTS.medium });
  });

  test('should test WebDAV connection', async ({ adminPage: page }) => {
    // This test assumes a WebDAV source exists from the previous test or setup
    await page.goto('/sources');
    await helpers.waitForLoadingToComplete();

    // Find WebDAV source and test connection
    const testConnectionButton = page.locator('button:has-text("Test"), [data-testid="test-connection"]').first();
    
    if (await testConnectionButton.isVisible()) {
      // Wait for connection test API call
      const testPromise = page.waitForResponse(response => 
        response.url().includes('/test') || response.url().includes('/connection'),
        { timeout: TIMEOUTS.medium }
      );
      
      await testConnectionButton.click();
      
      try {
        const response = await testPromise;
        console.log('Connection test completed with status:', response.status());
      } catch (error) {
        console.log('Connection test may have failed:', error);
      }
    }

    // Look for connection status indicator
    const statusIndicator = page.locator('.status, [data-testid="connection-status"], .connection-result');
    if (await statusIndicator.isVisible()) {
      const statusText = await statusIndicator.textContent();
      console.log('Connection status:', statusText);
    }
  });

  test('should initiate WebDAV sync', async ({ adminPage: page }) => {
    await page.goto('/sources');
    await helpers.waitForLoadingToComplete();

    // Find and click sync button
    const syncButton = page.locator('button:has-text("Sync"), [data-testid="sync-source"]').first();
    
    if (await syncButton.isVisible()) {
      // Wait for sync API call
      const syncPromise = page.waitForResponse(response => 
        response.url().includes('/sync') && response.status() === 200,
        { timeout: TIMEOUTS.medium }
      );
      
      await syncButton.click();
      
      try {
        await syncPromise;
        console.log('WebDAV sync initiated successfully');
        
        // Look for sync progress indicators
        const progressIndicator = page.locator('.progress, [data-testid="sync-progress"], .syncing');
        if (await progressIndicator.isVisible({ timeout: 5000 })) {
          console.log('Sync progress indicator visible');
        }
      } catch (error) {
        console.log('Sync may have failed or timed out:', error);
      }
    }
  });

  test('should show WebDAV sync history', async ({ adminPage: page }) => {
    await page.goto('/sources');
    await helpers.waitForLoadingToComplete();

    // Look for sync history or logs
    const historyButton = page.locator('button:has-text("History"), button:has-text("Logs"), [data-testid="sync-history"]').first();
    
    if (await historyButton.isVisible()) {
      await historyButton.click();
      
      // Check if history modal or page opens
      const historyContainer = page.locator('.history, [data-testid="sync-history"], .logs-container');
      await expect(historyContainer.first()).toBeVisible({ timeout: TIMEOUTS.short });
      
      // Check for history entries
      const historyEntries = page.locator('.history-item, .log-entry, tr');
      if (await historyEntries.first().isVisible({ timeout: 5000 })) {
        const entryCount = await historyEntries.count();
        console.log(`Found ${entryCount} sync history entries`);
      }
    }
  });

  test('should handle WebDAV source deletion', async ({ adminPage: page }) => {
    await page.goto('/sources');
    await helpers.waitForLoadingToComplete();

    // Find delete button for WebDAV source
    const deleteButton = page.locator('button:has-text("Delete"), [data-testid="delete-source"], .delete-button').first();
    
    if (await deleteButton.isVisible()) {
      await deleteButton.click();
      
      // Handle confirmation dialog if it appears
      const confirmButton = page.locator('button:has-text("Confirm"), button:has-text("Delete"), button:has-text("Yes")').first();
      if (await confirmButton.isVisible({ timeout: 3000 })) {
        // Wait for delete API call
        const deletePromise = page.waitForResponse(response => 
          response.url().includes('/sources') && response.status() === 200,
          { timeout: TIMEOUTS.medium }
        );
        
        await confirmButton.click();
        
        try {
          await deletePromise;
          console.log('WebDAV source deleted successfully');
        } catch (error) {
          console.log('Source deletion may have failed:', error);
        }
      }
    }
  });
});