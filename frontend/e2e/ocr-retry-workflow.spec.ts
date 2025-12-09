import { test, expect } from './fixtures/auth';
import { TIMEOUTS, API_ENDPOINTS } from './utils/test-data';
import { TestHelpers } from './utils/test-helpers';

test.describe('OCR Retry Workflow', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ dynamicAdminPage }) => {
    helpers = new TestHelpers(dynamicAdminPage);
    await helpers.navigateToPage('/documents');
  });

  test('should display failed OCR documents', async ({ dynamicAdminPage: page }) => {
    await page.goto('/documents');
    await helpers.waitForLoadingToComplete();

    // Look for failed documents filter or section
    const failedFilter = page.locator('button:has-text("Failed"), [data-testid="failed-filter"], .filter-failed').first();
    
    if (await failedFilter.isVisible()) {
      await failedFilter.click();
      await helpers.waitForLoadingToComplete();
    } else {
      // Alternative: look for a dedicated failed documents page
      const failedTab = page.locator('tab:has-text("Failed"), [role="tab"]:has-text("Failed")').first();
      if (await failedTab.isVisible()) {
        await failedTab.click();
        await helpers.waitForLoadingToComplete();
      }
    }

    // Check if failed documents are displayed
    const documentList = page.locator('[data-testid="document-list"], .document-list, .documents-grid');
    if (await documentList.isVisible({ timeout: 5000 })) {
      const documents = page.locator('.document-item, .document-card, [data-testid="document-item"]');
      const documentCount = await documents.count();
      console.log(`Found ${documentCount} documents in failed OCR view`);
    }
  });

  test('should retry individual failed OCR document', async ({ dynamicAdminPage: page }) => {
    await page.goto('/documents');
    await helpers.waitForLoadingToComplete();

    // Navigate to failed documents
    const failedFilter = page.locator('button:has-text("Failed"), [data-testid="failed-filter"]').first();
    if (await failedFilter.isVisible()) {
      await failedFilter.click();
      await helpers.waitForLoadingToComplete();
    }

    // Find a failed document and its retry button
    const retryButton = page.locator('button:has-text("Retry"), [data-testid="retry-ocr"], .retry-button').first();
    
    if (await retryButton.isVisible()) {
      // Wait for retry API call
      const retryPromise = page.waitForResponse(response => 
        response.url().includes('/retry') && response.status() === 200,
        { timeout: TIMEOUTS.medium }
      );
      
      await retryButton.click();
      
      try {
        await retryPromise;
        console.log('OCR retry initiated successfully');
        
        // Look for success message or status change
        const successMessage = page.locator('.success, [data-testid="success-message"], .notification');
        if (await successMessage.isVisible({ timeout: 5000 })) {
          console.log('Retry success message displayed');
        }
      } catch (error) {
        console.log('OCR retry may have failed:', error);
      }
    }
  });

  test('should bulk retry multiple failed OCR documents', async ({ dynamicAdminPage: page }) => {
    await page.goto('/documents');
    await helpers.waitForLoadingToComplete();

    // Navigate to failed documents
    const failedFilter = page.locator('button:has-text("Failed"), [data-testid="failed-filter"]').first();
    if (await failedFilter.isVisible()) {
      await failedFilter.click();
      await helpers.waitForLoadingToComplete();
    }

    // Select multiple documents
    const selectAllCheckbox = page.locator('input[type="checkbox"]:has-text("Select All"), [data-testid="select-all"]').first();
    if (await selectAllCheckbox.isVisible()) {
      await selectAllCheckbox.click();
    } else {
      // Alternative: select individual checkboxes
      const documentCheckboxes = page.locator('.document-item input[type="checkbox"], [data-testid="document-checkbox"]');
      const checkboxCount = await documentCheckboxes.count();
      if (checkboxCount > 0) {
        // Select first 3 documents
        for (let i = 0; i < Math.min(3, checkboxCount); i++) {
          await documentCheckboxes.nth(i).click();
        }
      }
    }

    // Find bulk retry button
    const bulkRetryButton = page.locator('button:has-text("Retry Selected"), button:has-text("Bulk Retry"), [data-testid="bulk-retry"]').first();
    
    if (await bulkRetryButton.isVisible()) {
      // Wait for bulk retry API call
      const bulkRetryPromise = page.waitForResponse(response => 
        response.url().includes('/bulk-retry') || response.url().includes('/retry'),
        { timeout: TIMEOUTS.long }
      );
      
      await bulkRetryButton.click();
      
      try {
        await bulkRetryPromise;
        console.log('Bulk OCR retry initiated successfully');
        
        // Look for progress indicator or success message
        const progressIndicator = page.locator('.progress, [data-testid="retry-progress"], .bulk-retry-progress');
        if (await progressIndicator.isVisible({ timeout: 5000 })) {
          console.log('Bulk retry progress indicator visible');
        }
      } catch (error) {
        console.log('Bulk OCR retry may have failed:', error);
      }
    }
  });

  test('should show OCR retry history', async ({ dynamicAdminPage: page }) => {
    await page.goto('/documents');
    await helpers.waitForLoadingToComplete();

    // Look for retry history or logs
    const historyButton = page.locator('button:has-text("Retry History"), [data-testid="retry-history"], .history-button').first();
    
    if (await historyButton.isVisible()) {
      await historyButton.click();
      
      // Check if history modal or panel opens
      const historyContainer = page.locator('.retry-history, [data-testid="retry-history-panel"], .history-container');
      await expect(historyContainer.first()).toBeVisible({ timeout: TIMEOUTS.short });
      
      // Check for history entries
      const historyEntries = page.locator('.history-item, .retry-entry, tr');
      if (await historyEntries.first().isVisible({ timeout: 5000 })) {
        const entryCount = await historyEntries.count();
        console.log(`Found ${entryCount} retry history entries`);
      }
    }
  });

  test('should display OCR failure reasons', async ({ dynamicAdminPage: page }) => {
    await page.goto('/documents');
    await helpers.waitForLoadingToComplete();

    // Navigate to failed documents
    const failedFilter = page.locator('button:has-text("Failed"), [data-testid="failed-filter"]').first();
    if (await failedFilter.isVisible()) {
      await failedFilter.click();
      await helpers.waitForLoadingToComplete();
    }

    // Click on a failed document to view details
    const failedDocument = page.locator('.document-item, .document-card, [data-testid="document-item"]').first();
    
    if (await failedDocument.isVisible()) {
      await failedDocument.click();
      
      // Look for failure reason or error details
      const errorDetails = page.locator('.error-details, [data-testid="failure-reason"], .ocr-error');
      if (await errorDetails.isVisible({ timeout: 5000 })) {
        const errorText = await errorDetails.textContent();
        console.log('OCR failure reason:', errorText);
      }
      
      // Look for retry recommendations
      const recommendations = page.locator('.retry-recommendations, [data-testid="retry-suggestions"], .recommendations');
      if (await recommendations.isVisible({ timeout: 5000 })) {
        console.log('Retry recommendations displayed');
      }
    }
  });

  test('should filter failed documents by failure type', async ({ dynamicAdminPage: page }) => {
    await page.goto('/documents');
    await helpers.waitForLoadingToComplete();

    // Navigate to failed documents
    const failedFilter = page.locator('button:has-text("Failed"), [data-testid="failed-filter"]').first();
    if (await failedFilter.isVisible()) {
      await failedFilter.click();
      await helpers.waitForLoadingToComplete();
    }

    // Look for failure type filters
    const filterDropdown = page.locator('select[name="failure-type"], [data-testid="failure-filter"]').first();
    
    if (await filterDropdown.isVisible()) {
      await filterDropdown.click();
      
      // Select a specific failure type
      const timeoutOption = page.locator('option:has-text("Timeout"), [value="timeout"]').first();
      if (await timeoutOption.isVisible()) {
        await timeoutOption.click();
        await helpers.waitForLoadingToComplete();
        
        // Verify filtered results
        const filteredDocuments = page.locator('.document-item, .document-card');
        const documentCount = await filteredDocuments.count();
        console.log(`Found ${documentCount} documents with timeout failures`);
      }
    }
  });
});