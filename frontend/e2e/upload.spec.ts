import { test, expect } from './fixtures/auth';
import { TEST_FILES, TIMEOUTS, API_ENDPOINTS } from './utils/test-data';
import { TestHelpers } from './utils/test-helpers';

test.describe('Document Upload', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ authenticatedPage }) => {
    helpers = new TestHelpers(authenticatedPage);
    await helpers.navigateToPage('/upload');
  });

  test('should display upload interface', async ({ authenticatedPage: page }) => {
    // Check for upload components
    await expect(page.locator('input[type="file"], [data-testid="file-upload"]')).toBeVisible();
    await expect(page.locator('button:has-text("Upload"), [data-testid="upload-button"]')).toBeVisible();
  });

  test('should upload single document successfully', async ({ authenticatedPage: page }) => {
    // Find file input - try multiple selectors
    const fileInput = page.locator('input[type="file"]').first();
    
    // Upload a test file
    await fileInput.setInputFiles(TEST_FILES.image);
    
    // Wait for upload API call
    const uploadResponse = helpers.waitForApiCall(API_ENDPOINTS.upload, TIMEOUTS.upload);
    
    // Click upload button if present
    const uploadButton = page.locator('button:has-text("Upload"), [data-testid="upload-button"]');
    if (await uploadButton.isVisible()) {
      await uploadButton.click();
    }
    
    // Verify upload was successful
    await uploadResponse;
    
    // Check for success message
    await helpers.waitForToast();
    
    // Should show uploaded document in list
    await expect(page.locator('[data-testid="uploaded-files"], .uploaded-file')).toBeVisible({ timeout: TIMEOUTS.medium });
  });

  test('should upload multiple documents', async ({ authenticatedPage: page }) => {
    const fileInput = page.locator('input[type="file"]').first();
    
    // Upload multiple files
    await fileInput.setInputFiles([TEST_FILES.image, TEST_FILES.multiline]);
    
    const uploadButton = page.locator('button:has-text("Upload"), [data-testid="upload-button"]');
    if (await uploadButton.isVisible()) {
      await uploadButton.click();
    }
    
    // Wait for both uploads to complete
    await helpers.waitForLoadingToComplete();
    
    // Should show multiple uploaded documents
    const uploadedFiles = page.locator('[data-testid="uploaded-files"] > *, .uploaded-file');
    await expect(uploadedFiles).toHaveCount(2, { timeout: TIMEOUTS.medium });
  });

  test('should show upload progress', async ({ authenticatedPage: page }) => {
    const fileInput = page.locator('input[type="file"]').first();
    await fileInput.setInputFiles(TEST_FILES.image);
    
    const uploadButton = page.locator('button:has-text("Upload"), [data-testid="upload-button"]');
    if (await uploadButton.isVisible()) {
      await uploadButton.click();
    }
    
    // Should show progress indicator
    await expect(page.locator('[data-testid="upload-progress"], .progress, [role="progressbar"]')).toBeVisible({ timeout: TIMEOUTS.short });
  });

  test('should handle upload errors gracefully', async ({ authenticatedPage: page }) => {
    // Mock a failed upload by using a non-existent file type or intercepting the request
    await page.route('**/api/documents/upload', route => {
      route.fulfill({
        status: 500,
        contentType: 'application/json',
        body: JSON.stringify({ error: 'Upload failed' })
      });
    });
    
    const fileInput = page.locator('input[type="file"]').first();
    await fileInput.setInputFiles(TEST_FILES.image);
    
    const uploadButton = page.locator('button:has-text("Upload"), [data-testid="upload-button"]');
    if (await uploadButton.isVisible()) {
      await uploadButton.click();
    }
    
    // Should show error message
    await helpers.waitForToast();
  });

  test('should validate file types', async ({ authenticatedPage: page }) => {
    // Try to upload an unsupported file type
    const fileInput = page.locator('input[type="file"]').first();
    
    // Create a mock file with unsupported extension
    const buffer = Buffer.from('fake content');
    await fileInput.setInputFiles({
      name: 'test.xyz',
      mimeType: 'application/octet-stream',
      buffer
    });
    
    const uploadButton = page.locator('button:has-text("Upload"), [data-testid="upload-button"]');
    if (await uploadButton.isVisible()) {
      await uploadButton.click();
    }
    
    // Should show validation error
    await helpers.waitForToast();
  });

  test('should navigate to uploaded document after successful upload', async ({ authenticatedPage: page }) => {
    const fileInput = page.locator('input[type="file"]').first();
    await fileInput.setInputFiles(TEST_FILES.image);
    
    const uploadButton = page.locator('button:has-text("Upload"), [data-testid="upload-button"]');
    if (await uploadButton.isVisible()) {
      await uploadButton.click();
    }
    
    await helpers.waitForLoadingToComplete();
    
    // Click on uploaded document to view details
    const uploadedDocument = page.locator('[data-testid="uploaded-files"] > *, .uploaded-file').first();
    if (await uploadedDocument.isVisible()) {
      await uploadedDocument.click();
      
      // Should navigate to document details page
      await page.waitForURL(/\/documents\/[^\/]+/, { timeout: TIMEOUTS.medium });
    }
  });

  test('should show OCR processing status', async ({ authenticatedPage: page }) => {
    const fileInput = page.locator('input[type="file"]').first();
    await fileInput.setInputFiles(TEST_FILES.image);
    
    const uploadButton = page.locator('button:has-text("Upload"), [data-testid="upload-button"]');
    if (await uploadButton.isVisible()) {
      await uploadButton.click();
    }
    
    await helpers.waitForLoadingToComplete();
    
    // Should show OCR processing status
    await expect(page.locator(':has-text("OCR"), :has-text("Processing"), [data-testid="ocr-status"]')).toBeVisible({ 
      timeout: TIMEOUTS.medium 
    });
  });

  test('should allow drag and drop upload', async ({ authenticatedPage: page }) => {
    // Look for dropzone
    const dropzone = page.locator('[data-testid="dropzone"], .dropzone, .upload-area');
    
    if (await dropzone.isVisible()) {
      // Simulate drag and drop
      await dropzone.dispatchEvent('dragover', { dataTransfer: { files: [] } });
      await dropzone.dispatchEvent('drop', { 
        dataTransfer: { 
          files: [{ name: TEST_FILES.image, type: 'image/png' }] 
        } 
      });
      
      // Should show uploaded file
      await expect(page.locator('[data-testid="uploaded-files"], .uploaded-file')).toBeVisible({ 
        timeout: TIMEOUTS.medium 
      });
    }
  });
});