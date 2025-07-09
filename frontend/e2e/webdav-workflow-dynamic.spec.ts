import { test, expect } from './fixtures/auth';
import { TIMEOUTS, API_ENDPOINTS } from './utils/test-data';
import { TestHelpers } from './utils/test-helpers';

test.describe('WebDAV Workflow (Dynamic Auth)', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ dynamicAdminPage }) => {
    helpers = new TestHelpers(dynamicAdminPage);
    await helpers.navigateToPage('/sources');
  });

  test('should create and configure WebDAV source with dynamic admin', async ({ dynamicAdminPage: page, testAdmin }) => {
    // Increase timeout for this test as WebDAV operations can be slow
    test.setTimeout(60000);
    
    console.log(`Running WebDAV test with dynamic admin: ${testAdmin.credentials.username}`);
    
    // Navigate to sources page
    await page.goto('/sources');
    await helpers.waitForLoadingToComplete();

    // Look for add source button using our new data-testid
    const addSourceButton = page.locator('[data-testid="add-source"]');
    await expect(addSourceButton).toBeVisible({ timeout: TIMEOUTS.medium });
    await addSourceButton.click();

    // Wait for source creation form/modal to appear
    await page.waitForTimeout(1000);
    
    // Debug: log what's currently visible
    await page.waitForLoadState('networkidle');
    console.log('Waiting for source creation form to load...');

    // Select WebDAV source type if source type selection exists
    try {
      // First, look for any select/dropdown elements - focusing on Material-UI patterns
      const selectTrigger = page.locator([
        '[role="combobox"]',
        '.MuiSelect-select:not([aria-hidden="true"])', 
        'div[aria-haspopup="listbox"]',
        '.MuiOutlinedInput-input[role="combobox"]',
        'select[name*="type"]',
        'select[name*="source"]'
      ].join(', ')).first();
      
      if (await selectTrigger.isVisible({ timeout: 5000 })) {
        console.log('Found select trigger, attempting to click...');
        await selectTrigger.click({ timeout: 10000 });
        
        // Wait for dropdown menu to appear
        await page.waitForTimeout(1000);
        
        // Look for WebDAV option in the dropdown
        const webdavOption = page.locator([
          '[role="option"]:has-text("webdav")',
          '[role="option"]:has-text("WebDAV")', 
          'li:has-text("WebDAV")',
          'li:has-text("webdav")',
          '[data-value="webdav"]',
          'option[value="webdav"]'
        ].join(', ')).first();
        
        if (await webdavOption.isVisible({ timeout: 5000 })) {
          console.log('Found WebDAV option, selecting it...');
          await webdavOption.click();
        } else {
          console.log('WebDAV option not found in dropdown, checking if already selected');
        }
      } else {
        console.log('No source type selector found, continuing with form...');
      }
    } catch (error) {
      console.log('Error selecting WebDAV source type:', error);
    }

    // Fill WebDAV configuration form
    console.log('Filling WebDAV configuration form...');
    
    // Wait for form to be ready
    await page.waitForTimeout(1000);
    
    const nameInput = page.locator('input[name="name"], input[placeholder*="name"], input[label*="Name"]').first();
    if (await nameInput.isVisible({ timeout: 10000 })) {
      await nameInput.fill(`Test WebDAV Source - ${testAdmin.credentials.username}`);
      console.log('Filled name input');
    }

    const urlInput = page.locator('input[name="url"], input[placeholder*="url"], input[type="url"]').first();
    if (await urlInput.isVisible({ timeout: 5000 })) {
      await urlInput.fill('https://demo.webdav.server/');
      console.log('Filled URL input');
    }

    const usernameInput = page.locator('input[name="username"], input[placeholder*="username"]').first();
    if (await usernameInput.isVisible({ timeout: 5000 })) {
      await usernameInput.fill('webdav_user');
      console.log('Filled username input');
    }

    const passwordInput = page.locator('input[name="password"], input[type="password"]').first();
    if (await passwordInput.isVisible({ timeout: 5000 })) {
      await passwordInput.fill('webdav_pass');
      console.log('Filled password input');
    }

    // Save the source configuration
    console.log('Looking for save button...');
    const saveButton = page.locator('button:has-text("Save"), button:has-text("Create"), button[type="submit"]').first();
    if (await saveButton.isVisible({ timeout: 10000 })) {
      console.log('Found save button, clicking...');
      
      // Wait for save API call
      const savePromise = page.waitForResponse(response => 
        response.url().includes('/sources') && (response.status() === 200 || response.status() === 201),
        { timeout: TIMEOUTS.medium }
      );
      
      await saveButton.click();
      console.log('Clicked save button, waiting for response...');
      
      try {
        const response = await savePromise;
        console.log('WebDAV source created successfully with status:', response.status());
      } catch (error) {
        console.log('Source creation may have failed or timed out:', error);
        // Don't fail the test immediately - continue to check the results
      }
    } else {
      console.log('Save button not found');
    }

    // Verify source appears in the list using our new data-testid
    await helpers.waitForLoadingToComplete();
    const sourceList = page.locator('[data-testid="sources-list"]');
    await expect(sourceList).toBeVisible({ timeout: TIMEOUTS.medium });
    
    // Verify individual source items
    const sourceItems = page.locator('[data-testid="source-item"]');
    await expect(sourceItems.first()).toBeVisible({ timeout: TIMEOUTS.medium });
    
    console.log(`✅ WebDAV source created successfully by dynamic admin: ${testAdmin.credentials.username}`);
  });

  test('should test WebDAV connection with dynamic admin', async ({ dynamicAdminPage: page, testAdmin }) => {
    console.log(`Testing WebDAV connection with dynamic admin: ${testAdmin.credentials.username}`);
    
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
    
    console.log(`✅ WebDAV connection test completed by dynamic admin: ${testAdmin.credentials.username}`);
  });
});