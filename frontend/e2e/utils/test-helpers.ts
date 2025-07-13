import { Page, expect } from '@playwright/test';
import { TEST_FILES } from './test-data';

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

  async uploadTestDocument(fileName: string = 'test1.png') {
    try {
      console.log(`Uploading test document: ${fileName}`);
      
      // Navigate to upload page
      await this.page.goto('/upload');
      await this.waitForLoadingToComplete();
      
      // Look for file input - react-dropzone creates hidden inputs
      const fileInput = this.page.locator('input[type="file"]').first();
      await expect(fileInput).toBeAttached({ timeout: 10000 });
      
      // Upload the test file using the proper path from TEST_FILES
      const filePath = fileName === 'test1.png' ? TEST_FILES.test1 : `../tests/test_images/${fileName}`;
      await fileInput.setInputFiles(filePath);
      
      // Verify file is added to the list by looking for the filename
      await expect(this.page.getByText(fileName)).toBeVisible({ timeout: 5000 });
      
      // Look for the "Upload All" button which appears after files are selected
      const uploadButton = this.page.locator('button:has-text("Upload All"), button:has-text("Upload")');
      if (await uploadButton.isVisible({ timeout: 5000 })) {
        // Wait for upload API call
        const uploadPromise = this.waitForApiCall('/api/documents', 30000);
        
        await uploadButton.click();
        
        // Wait for upload to complete
        await uploadPromise;
        console.log('Upload completed successfully');
      } else {
        console.log('Upload button not found, file may have been uploaded automatically');
      }
      
      // Return to documents page
      await this.page.goto('/documents');
      await this.waitForLoadingToComplete();
      
      console.log('Returned to documents page after upload');
    } catch (error) {
      console.error('Error uploading test document:', error);
      // Return to documents page even if upload failed
      await this.page.goto('/documents');
      await this.waitForLoadingToComplete();
    }
  }

  async ensureTestDocumentsExist() {
    try {
      // Give the page time to load before checking for documents
      await this.waitForLoadingToComplete();
      
      // Check if there are any documents - use multiple selectors to be safe
      const documentSelectors = [
        '[data-testid="document-item"]',
        '.document-item', 
        '.document-card',
        '.MuiCard-root', // Material-UI cards commonly used for documents
        '[role="article"]' // Semantic role for document items
      ];
      
      let documentCount = 0;
      for (const selector of documentSelectors) {
        const count = await this.page.locator(selector).count();
        if (count > 0) {
          documentCount = count;
          break;
        }
      }
      
      console.log(`Found ${documentCount} documents on the page`);
      
      if (documentCount === 0) {
        console.log('No documents found, attempting to upload a test document...');
        // Upload a test document
        await this.uploadTestDocument('test1.png');
      }
    } catch (error) {
      console.log('Error checking for test documents:', error);
      // Don't fail the test if document check fails, just log it
    }
  }
}