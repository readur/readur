import { test, expect } from './fixtures/auth';
import { TIMEOUTS, API_ENDPOINTS } from './utils/test-data';
import { TestHelpers } from './utils/test-helpers';

test.describe('Source Management', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ adminPage }) => {
    helpers = new TestHelpers(adminPage);
    await helpers.navigateToPage('/sources');
  });

  test.skip('should display sources interface', async ({ adminPage: page }) => {
    // Check for sources page components
    await expect(page.locator('[data-testid="sources-list"], .sources-list, .sources-container')).toBeVisible();
    await expect(page.locator('button:has-text("Add Source"), [data-testid="add-source"]')).toBeVisible();
  });

  test.skip('should create a new local folder source', async ({ adminPage: page }) => {
    // Click add source button
    await page.click('button:has-text("Add Source"), [data-testid="add-source"]');
    
    // Should show add source form/modal
    await expect(page.locator('[data-testid="add-source-form"], .add-source-modal, .source-form')).toBeVisible();
    
    // Fill in source details
    await page.fill('input[name="name"], [data-testid="source-name"]', 'Test Local Folder');
    
    // Select source type
    const typeSelector = page.locator('select[name="type"], [data-testid="source-type"]');
    if (await typeSelector.isVisible()) {
      await typeSelector.selectOption('local_folder');
    }
    
    // Fill in folder path
    await page.fill('input[name="path"], [data-testid="folder-path"]', '/tmp/test-folder');
    
    // Wait for source creation API call
    const createResponse = helpers.waitForApiCall('/api/sources', TIMEOUTS.medium);
    
    // Submit form
    await page.click('button[type="submit"], button:has-text("Create"), [data-testid="create-source"]');
    
    // Verify source was created
    await createResponse;
    
    // Should show success message
    await helpers.waitForToast();
    
    // Should appear in sources list
    await expect(page.locator(':has-text("Test Local Folder")')).toBeVisible({ timeout: TIMEOUTS.medium });
  });

  test.skip('should create a new WebDAV source', async ({ adminPage: page }) => {
    await page.click('button:has-text("Add Source"), [data-testid="add-source"]');
    
    await expect(page.locator('[data-testid="add-source-form"], .add-source-modal, .source-form')).toBeVisible();
    
    // Fill in WebDAV source details
    await page.fill('input[name="name"], [data-testid="source-name"]', 'Test WebDAV');
    
    const typeSelector = page.locator('select[name="type"], [data-testid="source-type"]');
    if (await typeSelector.isVisible()) {
      await typeSelector.selectOption('webdav');
    }
    
    // Fill WebDAV specific fields
    await page.fill('input[name="url"], [data-testid="webdav-url"]', 'https://example.com/webdav');
    await page.fill('input[name="username"], [data-testid="webdav-username"]', 'testuser');
    await page.fill('input[name="password"], [data-testid="webdav-password"]', 'testpass');
    
    const createResponse = helpers.waitForApiCall('/api/sources');
    
    await page.click('button[type="submit"], button:has-text("Create"), [data-testid="create-source"]');
    
    await createResponse;
    await helpers.waitForToast();
    
    await expect(page.locator(':has-text("Test WebDAV")')).toBeVisible({ timeout: TIMEOUTS.medium });
  });

  test.skip('should create a new S3 source', async ({ adminPage: page }) => {
    await page.click('button:has-text("Add Source"), [data-testid="add-source"]');
    
    await expect(page.locator('[data-testid="add-source-form"], .add-source-modal, .source-form')).toBeVisible();
    
    // Fill in S3 source details
    await page.fill('input[name="name"], [data-testid="source-name"]', 'Test S3 Bucket');
    
    const typeSelector = page.locator('select[name="type"], [data-testid="source-type"]');
    if (await typeSelector.isVisible()) {
      await typeSelector.selectOption('s3');
    }
    
    // Fill S3 specific fields
    await page.fill('input[name="bucket"], [data-testid="s3-bucket"]', 'test-bucket');
    await page.fill('input[name="region"], [data-testid="s3-region"]', 'us-east-1');
    await page.fill('input[name="accessKey"], [data-testid="s3-access-key"]', 'AKIATEST');
    await page.fill('input[name="secretKey"], [data-testid="s3-secret-key"]', 'secretkey123');
    
    const createResponse = helpers.waitForApiCall('/api/sources');
    
    await page.click('button[type="submit"], button:has-text("Create"), [data-testid="create-source"]');
    
    await createResponse;
    await helpers.waitForToast();
    
    await expect(page.locator(':has-text("Test S3 Bucket")')).toBeVisible({ timeout: TIMEOUTS.medium });
  });

  test('should edit existing source', async ({ adminPage: page }) => {
    // Look for existing source to edit
    const firstSource = page.locator('[data-testid="source-item"], .source-item, .source-card').first();
    
    if (await firstSource.isVisible()) {
      // Click edit button
      const editButton = firstSource.locator('button:has-text("Edit"), [data-testid="edit-source"], .edit-button');
      if (await editButton.isVisible()) {
        await editButton.click();
        
        // Should show edit form
        await expect(page.locator('[data-testid="edit-source-form"], .edit-source-modal, .source-form')).toBeVisible();
        
        // Modify source name
        const nameInput = page.locator('input[name="name"], [data-testid="source-name"]');
        await nameInput.fill('Updated Source Name');
        
        const updateResponse = helpers.waitForApiCall('/api/sources');
        
        await page.click('button[type="submit"], button:has-text("Save"), [data-testid="save-source"]');
        
        await updateResponse;
        await helpers.waitForToast();
        
        // Should show updated name
        await expect(page.locator(':has-text("Updated Source Name")')).toBeVisible({ timeout: TIMEOUTS.medium });
      }
    }
  });

  test('should delete source', async ({ adminPage: page }) => {
    const firstSource = page.locator('[data-testid="source-item"], .source-item, .source-card').first();
    
    if (await firstSource.isVisible()) {
      const sourceName = await firstSource.locator('[data-testid="source-name"], .source-name, h3, h4').textContent();
      
      // Click delete button
      const deleteButton = firstSource.locator('button:has-text("Delete"), [data-testid="delete-source"], .delete-button');
      if (await deleteButton.isVisible()) {
        await deleteButton.click();
        
        // Should show confirmation dialog
        const confirmButton = page.locator('button:has-text("Confirm"), button:has-text("Yes"), [data-testid="confirm-delete"]');
        if (await confirmButton.isVisible()) {
          const deleteResponse = helpers.waitForApiCall('/api/sources');
          
          await confirmButton.click();
          
          await deleteResponse;
          await helpers.waitForToast();
          
          // Source should be removed from list
          if (sourceName) {
            await expect(page.locator(`:has-text("${sourceName}")`)).not.toBeVisible();
          }
        }
      }
    }
  });

  test.skip('should start source sync', async ({ adminPage: page }) => {
    const firstSource = page.locator('[data-testid="source-item"], .source-item, .source-card').first();
    
    if (await firstSource.isVisible()) {
      // Look for sync button
      const syncButton = firstSource.locator('button:has-text("Sync"), [data-testid="sync-source"], .sync-button');
      if (await syncButton.isVisible()) {
        const syncResponse = helpers.waitForApiCall('/api/sources/*/sync');
        
        await syncButton.click();
        
        await syncResponse;
        
        // Should show sync status
        await expect(firstSource.locator(':has-text("Syncing"), [data-testid="sync-status"], .sync-status')).toBeVisible({ 
          timeout: TIMEOUTS.medium 
        });
      }
    }
  });

  test('should stop source sync', async ({ adminPage: page }) => {
    const firstSource = page.locator('[data-testid="source-item"], .source-item, .source-card').first();
    
    if (await firstSource.isVisible()) {
      // First start sync if not running
      const syncButton = firstSource.locator('button:has-text("Sync"), [data-testid="sync-source"]');
      if (await syncButton.isVisible()) {
        await syncButton.click();
        await helpers.waitForLoadingToComplete();
      }
      
      // Look for stop button
      const stopButton = firstSource.locator('button:has-text("Stop"), [data-testid="stop-sync"], .stop-button');
      if (await stopButton.isVisible()) {
        const stopResponse = helpers.waitForApiCall('/api/sources/*/stop');
        
        await stopButton.click();
        
        await stopResponse;
        
        // Should show stopped status
        await expect(firstSource.locator(':has-text("Stopped"), :has-text("Idle")')).toBeVisible({ 
          timeout: TIMEOUTS.medium 
        });
      }
    }
  });

  test('should display source status and statistics', async ({ adminPage: page }) => {
    const firstSource = page.locator('[data-testid="source-item"]').first();
    
    if (await firstSource.isVisible()) {
      // Should show source status information - check for status chip with icons
      const statusChip = firstSource.locator('.MuiChip-root').filter({ hasText: /^(Idle|Syncing|Error)$/i });
      await expect(statusChip).toBeVisible();
      
      // Should show statistics cards within the source item
      await expect(firstSource.locator(':has-text("Documents Stored")')).toBeVisible();
      await expect(firstSource.locator(':has-text("OCR Processed")')).toBeVisible();
      await expect(firstSource.locator(':has-text("Last Sync")')).toBeVisible();
      await expect(firstSource.locator(':has-text("Total Size")')).toBeVisible();
    }
  });

  test.skip('should test source connection', async ({ adminPage: page }) => {
    await page.click('button:has-text("Add Source"), [data-testid="add-source"]');
    
    await expect(page.locator('[data-testid="add-source-form"], .add-source-modal')).toBeVisible();
    
    // Fill in source details
    await page.fill('input[name="name"], [data-testid="source-name"]', 'Test Connection');
    
    const typeSelector = page.locator('select[name="type"], [data-testid="source-type"]');
    if (await typeSelector.isVisible()) {
      await typeSelector.selectOption('webdav');
    }
    
    await page.fill('input[name="url"], [data-testid="webdav-url"]', 'https://example.com/webdav');
    await page.fill('input[name="username"], [data-testid="webdav-username"]', 'testuser');
    await page.fill('input[name="password"], [data-testid="webdav-password"]', 'testpass');
    
    // Look for test connection button
    const testButton = page.locator('button:has-text("Test"), [data-testid="test-connection"], .test-button');
    if (await testButton.isVisible()) {
      const testResponse = helpers.waitForApiCall('/api/sources/test');
      
      await testButton.click();
      
      await testResponse;
      
      // Should show test result
      await helpers.waitForToast();
    }
  });

  test('should filter sources by type', async ({ adminPage: page }) => {
    // Look for filter dropdown
    const filterDropdown = page.locator('[data-testid="source-filter"], select[name="filter"], .source-filter');
    if (await filterDropdown.isVisible()) {
      await filterDropdown.selectOption('webdav');
      
      await helpers.waitForLoadingToComplete();
      
      // Should show only WebDAV sources
      const sourceItems = page.locator('[data-testid="source-item"], .source-item');
      if (await sourceItems.count() > 0) {
        await expect(sourceItems.first().locator(':has-text("WebDAV"), .webdav-icon')).toBeVisible();
      }
    }
  });

  test('should display sync history', async ({ adminPage: page }) => {
    const firstSource = page.locator('[data-testid="source-item"], .source-item, .source-card').first();
    
    if (await firstSource.isVisible()) {
      await firstSource.click();
      
      // Look for sync history section
      const historySection = page.locator('[data-testid="sync-history"], .sync-history, .history-section');
      if (await historySection.isVisible()) {
        // Should show sync runs
        await expect(historySection.locator('[data-testid="sync-run"], .sync-run, .history-item')).toBeVisible();
      }
    }
  });

  test.skip('should validate required fields in source creation', async ({ adminPage: page }) => {
    await page.click('button:has-text("Add Source"), [data-testid="add-source"]');
    
    await expect(page.locator('[data-testid="add-source-form"], .add-source-modal')).toBeVisible();
    
    // Try to submit without filling required fields
    await page.click('button[type="submit"], button:has-text("Create"), [data-testid="create-source"]');
    
    // Should show validation errors
    const nameInput = page.locator('input[name="name"], [data-testid="source-name"]');
    await expect(nameInput).toBeVisible();
    
    // Check for validation messages
    const validationMessages = page.locator('.error, .validation-error, [data-testid="validation-error"]');
    if (await validationMessages.count() > 0) {
      await expect(validationMessages.first()).toBeVisible();
    }
  });

  test('should schedule automatic sync', async ({ adminPage: page }) => {
    const firstSource = page.locator('[data-testid="source-item"], .source-item, .source-card').first();
    
    if (await firstSource.isVisible()) {
      // Click settings or edit button
      const settingsButton = firstSource.locator('button:has-text("Settings"), button:has-text("Edit"), [data-testid="source-settings"]');
      if (await settingsButton.isVisible()) {
        await settingsButton.click();
        
        // Look for scheduling options
        const scheduleSection = page.locator('[data-testid="schedule-section"], .schedule-options');
        if (await scheduleSection.isVisible()) {
          // Enable automatic sync
          const enableSchedule = page.locator('input[type="checkbox"][name="enableSchedule"], [data-testid="enable-schedule"]');
          if (await enableSchedule.isVisible()) {
            await enableSchedule.check();
            
            // Set sync interval
            const intervalSelect = page.locator('select[name="interval"], [data-testid="sync-interval"]');
            if (await intervalSelect.isVisible()) {
              await intervalSelect.selectOption('daily');
            }
            
            const saveResponse = helpers.waitForApiCall('/api/sources');
            
            await page.click('button[type="submit"], button:has-text("Save")');
            
            await saveResponse;
            await helpers.waitForToast();
          }
        }
      }
    }
  });
});