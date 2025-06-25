import { Page, expect } from '@playwright/test';

export class TestHelpers {
  constructor(private page: Page) {}

  async waitForApiCall(urlPattern: string | RegExp, timeout = 10000) {
    return this.page.waitForResponse(resp => 
      typeof urlPattern === 'string' 
        ? resp.url().includes(urlPattern)
        : urlPattern.test(resp.url()), 
      { timeout }
    );
  }

  async uploadFile(inputSelector: string, filePath: string) {
    const fileInput = this.page.locator(inputSelector);
    await fileInput.setInputFiles(filePath);
  }

  async clearAndType(selector: string, text: string) {
    await this.page.fill(selector, '');
    await this.page.type(selector, text);
  }

  async waitForToast(message?: string) {
    const toast = this.page.locator('[data-testid="toast"], .toast, [role="alert"]');
    await expect(toast).toBeVisible({ timeout: 5000 });
    
    if (message) {
      await expect(toast).toContainText(message);
    }
    
    return toast;
  }

  async waitForLoadingToComplete() {
    // Wait for any loading spinners to disappear
    await this.page.waitForFunction(() => 
      !document.querySelector('[data-testid="loading"], .loading, [aria-label*="loading" i]')
    );
  }

  async navigateToPage(path: string) {
    await this.page.goto(path);
    await this.waitForLoadingToComplete();
  }

  async takeScreenshotOnFailure(testName: string) {
    await this.page.screenshot({ 
      path: `test-results/screenshots/${testName}-${Date.now()}.png`,
      fullPage: true 
    });
  }

  async uploadTestDocument(fileName: string) {
    // Navigate to upload page
    await this.page.goto('/upload');
    
    // Look for file input
    const fileInput = this.page.locator('input[type="file"]');
    await expect(fileInput).toBeVisible();
    
    // Upload the test file
    await fileInput.setInputFiles(`../tests/test_images/${fileName}`);
    
    // Wait for upload button and click it
    const uploadButton = this.page.locator('button:has-text("Upload"), [data-testid="upload-button"]');
    if (await uploadButton.isVisible()) {
      await uploadButton.click();
    }
    
    // Wait for upload to complete
    await this.page.waitForTimeout(2000);
    
    // Return to documents page
    await this.page.goto('/documents');
    await this.waitForLoadingToComplete();
  }

  async ensureTestDocumentsExist() {
    // Check if there are any documents
    const documentCount = await this.page.locator('[data-testid="document-item"], .document-item, .document-card').count();
    
    if (documentCount === 0) {
      // Upload a test document
      await this.uploadTestDocument('test1.png');
    }
  }

  async ensureFailedOcrDocumentsExist() {
    // Navigate to failed OCR page and check if there are any failed documents
    await this.page.goto('/failed-ocr');
    await this.waitForLoadingToComplete();
    
    // Check for failed OCR documents
    const failedDocCount = await this.page.locator('tbody tr, [data-testid="failed-document"]').count();
    
    if (failedDocCount === 0) {
      // If no failed documents exist, we'll upload some test documents
      // that might fail OCR or at least create some documents for testing
      await this.uploadTestDocument('test1.png');
      await this.uploadTestDocument('test2.jpg');
      
      // Wait a bit for OCR processing to potentially fail
      await this.page.waitForTimeout(5000);
      
      // Return to failed OCR page
      await this.page.goto('/failed-ocr');
      await this.waitForLoadingToComplete();
    }
  }

  async waitForDownload(timeout = 10000) {
    return this.page.waitForEvent('download', { timeout });
  }

  async interceptAuthenticatedRequest(urlPattern: string | RegExp) {
    return this.page.route(urlPattern, async (route, request) => {
      const headers = request.headers();
      
      // Verify auth header exists
      if (!headers['authorization'] || !headers['authorization'].startsWith('Bearer ')) {
        throw new Error(`Missing or invalid authorization header for ${request.url()}`);
      }
      
      // Continue with the request
      await route.continue();
    });
  }
}