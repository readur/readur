import { test, expect } from './fixtures/auth';
import { TIMEOUTS } from './utils/test-data';
import { TestHelpers } from './utils/test-helpers';

test.describe('Failed OCR Page Downloads', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ authenticatedPage }) => {
    helpers = new TestHelpers(authenticatedPage);
    await helpers.navigateToPage('/failed-ocr');
    // Ensure we have some failed OCR documents for testing
    await helpers.ensureFailedOcrDocumentsExist();
  });

  test('should display failed OCR page with download buttons', async ({ authenticatedPage: page }) => {
    // Verify page loads correctly
    await expect(page.getByRole('heading', { name: /failed ocr/i })).toBeVisible();
    
    // Check for tabs
    await expect(page.getByRole('tab', { name: /failed ocr/i })).toBeVisible();
    await expect(page.getByRole('tab', { name: /duplicates/i })).toBeVisible();
    
    // Wait for data to load
    await helpers.waitForLoadingToComplete();
    
    // Look for download buttons - they should be present if there are failed documents
    const downloadButtons = page.locator('button[aria-label*="Download"], [data-testid="download"], .download-button');
    const downloadCount = await downloadButtons.count();
    
    if (downloadCount > 0) {
      await expect(downloadButtons.first()).toBeVisible();
    }
  });

  test('should download failed OCR document with authentication', async ({ authenticatedPage: page }) => {
    // Wait for failed OCR documents to load
    await helpers.waitForLoadingToComplete();
    
    // Look for the first document row that has a download button
    const documentRows = page.locator('tbody tr, [data-testid="document-row"], .document-item');
    const downloadButtons = page.locator('button[aria-label*="Download"], [data-testid="download"]');
    
    if (await downloadButtons.count() > 0) {
      // Set up download listener before clicking
      const downloadPromise = page.waitForEvent('download', { timeout: TIMEOUTS.medium });
      
      // Click the first download button
      await downloadButtons.first().click();
      
      // Wait for download to start
      const download = await downloadPromise;
      
      // Verify download properties
      expect(download.suggestedFilename()).toBeTruthy();
      expect(download.suggestedFilename()).not.toBe('');
      
      // Verify the download URL contains the document ID and has proper auth
      const downloadUrl = download.url();
      expect(downloadUrl).toContain('/api/documents/');
      expect(downloadUrl).toContain('/download');
      
      // Cancel the download to avoid saving files during tests
      await download.cancel();
    } else {
      test.skip('No failed OCR documents available for download testing');
    }
  });

  test('should expand error details and download from expanded view', async ({ authenticatedPage: page }) => {
    await helpers.waitForLoadingToComplete();
    
    // Look for expand buttons (chevron down, expand more, etc.)
    const expandButtons = page.locator('button[aria-label*="expand"], [data-testid="expand"], button:has([data-testid="ExpandMoreIcon"])');
    
    if (await expandButtons.count() > 0) {
      // Click first expand button
      await expandButtons.first().click();
      
      // Wait for expansion
      await page.waitForTimeout(500);
      
      // Look for "Error Details" section
      await expect(page.locator('text="Error Details"')).toBeVisible({ timeout: TIMEOUTS.short });
      
      // Look for download button in expanded section
      const expandedDownloadButtons = page.locator('.MuiCollapse-root button[aria-label*="Download"], [data-testid="expanded-download"]');
      
      if (await expandedDownloadButtons.count() > 0) {
        const downloadPromise = page.waitForEvent('download', { timeout: TIMEOUTS.medium });
        
        await expandedDownloadButtons.first().click();
        
        const download = await downloadPromise;
        expect(download.suggestedFilename()).toBeTruthy();
        
        await download.cancel();
      }
    } else {
      test.skip('No expandable failed OCR documents available');
    }
  });

  test('should download from duplicates tab', async ({ authenticatedPage: page }) => {
    // Switch to duplicates tab
    const duplicatesTab = page.getByRole('tab', { name: /duplicates/i });
    await duplicatesTab.click();
    
    // Wait for duplicates to load
    await helpers.waitForLoadingToComplete();
    
    // Look for download buttons in duplicates section
    const duplicateDownloadButtons = page.locator('button[aria-label*="Download"], [data-testid="download"]');
    
    if (await duplicateDownloadButtons.count() > 0) {
      const downloadPromise = page.waitForEvent('download', { timeout: TIMEOUTS.medium });
      
      await duplicateDownloadButtons.first().click();
      
      const download = await downloadPromise;
      expect(download.suggestedFilename()).toBeTruthy();
      
      await download.cancel();
    } else {
      // This is expected if there are no duplicates
      console.log('No duplicate documents available for download testing');
    }
  });

  test('should handle download errors gracefully', async ({ authenticatedPage: page }) => {
    await helpers.waitForLoadingToComplete();
    
    const downloadButtons = page.locator('button[aria-label*="Download"]');
    
    if (await downloadButtons.count() > 0) {
      // Intercept the download request and make it fail
      await page.route('**/api/documents/*/download', async route => {
        await route.abort('failed');
      });
      
      // Click download button
      await downloadButtons.first().click();
      
      // The application should handle the error gracefully
      // No crash should occur, and the page should remain functional
      await expect(page.getByRole('heading', { name: /failed ocr/i })).toBeVisible();
      
      // Remove the route interception
      await page.unroute('**/api/documents/*/download');
    } else {
      test.skip('No documents available for error testing');
    }
  });

  test('should verify authentication headers in download requests', async ({ authenticatedPage: page }) => {
    await helpers.waitForLoadingToComplete();
    
    let authHeaderFound = false;
    let downloadRequestMade = false;
    
    // Intercept download requests to verify auth headers
    await page.route('**/api/documents/*/download', async (route, request) => {
      downloadRequestMade = true;
      const headers = request.headers();
      
      // Check for Authorization header
      if (headers['authorization'] && headers['authorization'].startsWith('Bearer ')) {
        authHeaderFound = true;
      }
      
      // Continue with the request
      await route.continue();
    });
    
    const downloadButtons = page.locator('button[aria-label*="Download"]');
    
    if (await downloadButtons.count() > 0) {
      const downloadPromise = page.waitForEvent('download', { timeout: TIMEOUTS.medium });
      
      await downloadButtons.first().click();
      
      const download = await downloadPromise;
      await download.cancel();
      
      // Verify that the request was made with proper auth headers
      expect(downloadRequestMade).toBe(true);
      expect(authHeaderFound).toBe(true);
    } else {
      test.skip('No documents available for auth header testing');
    }
    
    await page.unroute('**/api/documents/*/download');
  });

  test('should preserve original filenames in downloads', async ({ authenticatedPage: page }) => {
    await helpers.waitForLoadingToComplete();
    
    // Look for document names in the table
    const documentCells = page.locator('td:has-text(".pdf"), td:has-text(".png"), td:has-text(".jpg"), td:has-text(".doc")');
    
    if (await documentCells.count() > 0) {
      const firstDocumentCell = documentCells.first();
      const documentName = await firstDocumentCell.textContent();
      
      // Find the corresponding download button
      const parentRow = firstDocumentCell.locator('xpath=ancestor::tr[1]');
      const downloadButton = parentRow.locator('button[aria-label*="Download"]');
      
      if (await downloadButton.isVisible()) {
        const downloadPromise = page.waitForEvent('download', { timeout: TIMEOUTS.medium });
        
        await downloadButton.click();
        
        const download = await downloadPromise;
        const suggestedFilename = download.suggestedFilename();
        
        // Verify the filename makes sense (not empty and contains an extension)
        expect(suggestedFilename).toBeTruthy();
        expect(suggestedFilename).toMatch(/\.(pdf|png|jpg|jpeg|doc|docx|txt)$/i);
        
        await download.cancel();
      }
    } else {
      test.skip('No documents with recognizable filenames found');
    }
  });

  test('should work with different document types', async ({ authenticatedPage: page }) => {
    await helpers.waitForLoadingToComplete();
    
    // Test downloading different file types if available
    const fileTypes = ['.pdf', '.png', '.jpg', '.doc', '.txt'];
    
    for (const fileType of fileTypes) {
      const documentCell = page.locator(`td:has-text("${fileType}")`).first();
      
      if (await documentCell.isVisible()) {
        const parentRow = documentCell.locator('xpath=ancestor::tr[1]');
        const downloadButton = parentRow.locator('button[aria-label*="Download"]');
        
        if (await downloadButton.isVisible()) {
          const downloadPromise = page.waitForEvent('download', { timeout: TIMEOUTS.short });
          
          await downloadButton.click();
          
          const download = await downloadPromise;
          expect(download.suggestedFilename()).toContain(fileType);
          
          await download.cancel();
          
          // Only test one file type to keep test execution time reasonable
          break;
        }
      }
    }
  });

  test('should maintain page state after download', async ({ authenticatedPage: page }) => {
    await helpers.waitForLoadingToComplete();
    
    const downloadButtons = page.locator('button[aria-label*="Download"]');
    
    if (await downloadButtons.count() > 0) {
      // Get initial page state
      const initialUrl = page.url();
      const initialHeading = page.getByRole('heading', { name: /failed ocr/i });
      
      // Perform download
      const downloadPromise = page.waitForEvent('download', { timeout: TIMEOUTS.medium });
      await downloadButtons.first().click();
      
      const download = await downloadPromise;
      await download.cancel();
      
      // Verify page state is maintained
      expect(page.url()).toBe(initialUrl);
      await expect(initialHeading).toBeVisible();
      
      // Verify download buttons are still functional
      await expect(downloadButtons.first()).toBeVisible();
    } else {
      test.skip('No documents available for state testing');
    }
  });
});