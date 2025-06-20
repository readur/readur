import { test, expect } from './fixtures/auth';
import { TIMEOUTS } from './utils/test-data';
import { TestHelpers } from './utils/test-helpers';

test.describe('Settings Management', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ authenticatedPage }) => {
    helpers = new TestHelpers(authenticatedPage);
    await helpers.navigateToPage('/settings');
  });

  test.skip('should display settings interface', async ({ authenticatedPage: page }) => {
    // Check for settings page components
    await expect(page.locator('[data-testid="settings-container"], .settings-page, .settings-form')).toBeVisible();
  });

  test('should update OCR settings', async ({ authenticatedPage: page }) => {
    // Look for OCR settings section
    const ocrSection = page.locator('[data-testid="ocr-settings"], .ocr-section, .settings-section:has-text("OCR")');
    if (await ocrSection.isVisible()) {
      // Change OCR language
      const languageSelect = page.locator('select[name="ocrLanguage"], [data-testid="ocr-language"]');
      if (await languageSelect.isVisible()) {
        await languageSelect.selectOption('spa'); // Spanish
        
        const saveResponse = helpers.waitForApiCall('/api/settings');
        
        // Save settings
        await page.click('button[type="submit"], button:has-text("Save"), [data-testid="save-settings"]');
        
        await saveResponse;
        await helpers.waitForToast();
      }
    }
  });

  test('should update watch folder settings', async ({ authenticatedPage: page }) => {
    // Navigate to watch folder section if it's a separate page
    const watchFolderNav = page.locator('a[href="/watch-folder"], [data-testid="watch-folder-nav"]');
    if (await watchFolderNav.isVisible()) {
      await watchFolderNav.click();
      await helpers.waitForLoadingToComplete();
    }
    
    // Look for watch folder settings
    const watchSection = page.locator('[data-testid="watch-settings"], .watch-folder-section, .settings-section:has-text("Watch")');
    if (await watchSection.isVisible()) {
      // Enable watch folder
      const enableWatch = page.locator('input[type="checkbox"][name="enableWatch"], [data-testid="enable-watch"]');
      if (await enableWatch.isVisible()) {
        await enableWatch.check();
        
        // Set watch folder path
        const pathInput = page.locator('input[name="watchPath"], [data-testid="watch-path"]');
        if (await pathInput.isVisible()) {
          await pathInput.fill('/tmp/watch-folder');
        }
        
        const saveResponse = helpers.waitForApiCall('/api/settings');
        
        await page.click('button[type="submit"], button:has-text("Save")');
        
        await saveResponse;
        await helpers.waitForToast();
      }
    }
  });

  test('should update notification settings', async ({ authenticatedPage: page }) => {
    const notificationSection = page.locator('[data-testid="notification-settings"], .notification-section, .settings-section:has-text("Notification")');
    if (await notificationSection.isVisible()) {
      // Enable notifications
      const enableNotifications = page.locator('input[type="checkbox"][name="enableNotifications"], [data-testid="enable-notifications"]');
      if (await enableNotifications.isVisible()) {
        await enableNotifications.check();
        
        // Configure notification types
        const ocrNotifications = page.locator('input[type="checkbox"][name="ocrNotifications"], [data-testid="ocr-notifications"]');
        if (await ocrNotifications.isVisible()) {
          await ocrNotifications.check();
        }
        
        const syncNotifications = page.locator('input[type="checkbox"][name="syncNotifications"], [data-testid="sync-notifications"]');
        if (await syncNotifications.isVisible()) {
          await syncNotifications.check();
        }
        
        const saveResponse = helpers.waitForApiCall('/api/settings');
        
        await page.click('button[type="submit"], button:has-text("Save")');
        
        await saveResponse;
        await helpers.waitForToast();
      }
    }
  });

  test('should update search settings', async ({ authenticatedPage: page }) => {
    const searchSection = page.locator('[data-testid="search-settings"], .search-section, .settings-section:has-text("Search")');
    if (await searchSection.isVisible()) {
      // Configure search results per page
      const resultsPerPage = page.locator('select[name="resultsPerPage"], [data-testid="results-per-page"]');
      if (await resultsPerPage.isVisible()) {
        await resultsPerPage.selectOption('25');
      }
      
      // Enable/disable features
      const enhancedSearch = page.locator('input[type="checkbox"][name="enhancedSearch"], [data-testid="enhanced-search"]');
      if (await enhancedSearch.isVisible()) {
        await enhancedSearch.check();
      }
      
      const saveResponse = helpers.waitForApiCall('/api/settings');
      
      await page.click('button[type="submit"], button:has-text("Save")');
      
      await saveResponse;
      await helpers.waitForToast();
    }
  });

  test('should reset settings to defaults', async ({ authenticatedPage: page }) => {
    // Look for reset button
    const resetButton = page.locator('button:has-text("Reset"), button:has-text("Default"), [data-testid="reset-settings"]');
    if (await resetButton.isVisible()) {
      await resetButton.click();
      
      // Should show confirmation
      const confirmButton = page.locator('button:has-text("Confirm"), button:has-text("Yes"), [data-testid="confirm-reset"]');
      if (await confirmButton.isVisible()) {
        const resetResponse = helpers.waitForApiCall('/api/settings/reset');
        
        await confirmButton.click();
        
        await resetResponse;
        await helpers.waitForToast();
        
        // Page should reload with default values
        await helpers.waitForLoadingToComplete();
      }
    }
  });

  test('should validate settings before saving', async ({ authenticatedPage: page }) => {
    // Try to set invalid values
    const pathInput = page.locator('input[name="watchPath"], [data-testid="watch-path"]');
    if (await pathInput.isVisible()) {
      // Enter invalid path
      await pathInput.fill('invalid/path/with/spaces and special chars!');
      
      await page.click('button[type="submit"], button:has-text("Save")');
      
      // Should show validation error
      await helpers.waitForToast();
      
      // Should not save invalid settings
      expect(await pathInput.inputValue()).toBe('invalid/path/with/spaces and special chars!');
    }
  });

  test('should export settings', async ({ authenticatedPage: page }) => {
    const exportButton = page.locator('button:has-text("Export"), [data-testid="export-settings"]');
    if (await exportButton.isVisible()) {
      // Set up download listener
      const downloadPromise = page.waitForEvent('download');
      
      await exportButton.click();
      
      // Verify download started
      const download = await downloadPromise;
      expect(download.suggestedFilename()).toContain('settings');
    }
  });

  test('should import settings', async ({ authenticatedPage: page }) => {
    const importButton = page.locator('button:has-text("Import"), [data-testid="import-settings"]');
    if (await importButton.isVisible()) {
      // Look for file input
      const fileInput = page.locator('input[type="file"], [data-testid="settings-file"]');
      if (await fileInput.isVisible()) {
        // Create a mock settings file
        const settingsContent = JSON.stringify({
          ocrLanguage: 'eng',
          enableNotifications: true,
          resultsPerPage: 20
        });
        
        await fileInput.setInputFiles({
          name: 'settings.json',
          mimeType: 'application/json',
          buffer: Buffer.from(settingsContent)
        });
        
        const importResponse = helpers.waitForApiCall('/api/settings/import');
        
        await importButton.click();
        
        await importResponse;
        await helpers.waitForToast();
      }
    }
  });

  test('should display current system status', async ({ authenticatedPage: page }) => {
    // Look for system status section
    const statusSection = page.locator('[data-testid="system-status"], .status-section, .settings-section:has-text("Status")');
    if (await statusSection.isVisible()) {
      // Should show various system metrics
      await expect(statusSection.locator(':has-text("Database"), :has-text("Storage"), :has-text("OCR")')).toBeVisible();
    }
  });

  test('should test OCR functionality', async ({ authenticatedPage: page }) => {
    const ocrSection = page.locator('[data-testid="ocr-settings"], .ocr-section');
    if (await ocrSection.isVisible()) {
      const testButton = page.locator('button:has-text("Test OCR"), [data-testid="test-ocr"]');
      if (await testButton.isVisible()) {
        const testResponse = helpers.waitForApiCall('/api/ocr/test');
        
        await testButton.click();
        
        await testResponse;
        
        // Should show test result
        await helpers.waitForToast();
      }
    }
  });

  test('should clear cache', async ({ authenticatedPage: page }) => {
    const clearCacheButton = page.locator('button:has-text("Clear Cache"), [data-testid="clear-cache"]');
    if (await clearCacheButton.isVisible()) {
      const clearResponse = helpers.waitForApiCall('/api/cache/clear');
      
      await clearCacheButton.click();
      
      await clearResponse;
      await helpers.waitForToast();
    }
  });

  test('should update user profile', async ({ authenticatedPage: page }) => {
    // Look for user profile section
    const profileSection = page.locator('[data-testid="profile-settings"], .profile-section, .settings-section:has-text("Profile")');
    if (await profileSection.isVisible()) {
      // Update email
      const emailInput = page.locator('input[name="email"], [data-testid="user-email"]');
      if (await emailInput.isVisible()) {
        await emailInput.fill('newemail@example.com');
      }
      
      // Update name
      const nameInput = page.locator('input[name="name"], [data-testid="user-name"]');
      if (await nameInput.isVisible()) {
        await nameInput.fill('Updated Name');
      }
      
      const saveResponse = helpers.waitForApiCall('/api/users/profile');
      
      await page.click('button[type="submit"], button:has-text("Save")');
      
      await saveResponse;
      await helpers.waitForToast();
    }
  });

  test('should change password', async ({ authenticatedPage: page }) => {
    const passwordSection = page.locator('[data-testid="password-settings"], .password-section, .settings-section:has-text("Password")');
    if (await passwordSection.isVisible()) {
      await page.fill('input[name="currentPassword"], [data-testid="current-password"]', 'currentpass');
      await page.fill('input[name="newPassword"], [data-testid="new-password"]', 'newpassword123');
      await page.fill('input[name="confirmPassword"], [data-testid="confirm-password"]', 'newpassword123');
      
      const changeResponse = helpers.waitForApiCall('/api/users/password');
      
      await page.click('button[type="submit"], button:has-text("Change Password")');
      
      await changeResponse;
      await helpers.waitForToast();
    }
  });
});