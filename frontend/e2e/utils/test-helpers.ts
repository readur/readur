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
    const toast = this.page.locator('[data-testid="toast"], .toast, [role="alert"]').first();
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

  async waitForWebKitStability() {
    const browserName = await this.page.evaluate(() => navigator.userAgent);
    const isWebKit = browserName.includes('WebKit') && !browserName.includes('Chrome');
    
    if (isWebKit) {
      console.log('WebKit stability waiting initiated...');
      
      // Wait for network to be completely idle
      await this.page.waitForLoadState('networkidle');
      await this.page.waitForTimeout(3000);
      
      // Wait for JavaScript to finish executing
      await this.page.waitForFunction(() => {
        return document.readyState === 'complete' && 
               typeof window !== 'undefined';
      }, { timeout: 15000 });
      
      // Extra stability wait
      await this.page.waitForTimeout(2000);
      console.log('WebKit stability waiting completed');
    }
  }

  async waitForBrowserStability() {
    const browserName = await this.page.context().browser()?.browserType().name() || '';
    
    switch (browserName) {
      case 'webkit':
        await this.waitForWebKitStability();
        break;
      case 'firefox':
        // Firefox-specific stability wait
        console.log('Firefox stability waiting initiated...');
        await this.page.waitForLoadState('networkidle');
        await this.page.waitForTimeout(2000);
        // Firefox sometimes needs extra time for form validation
        await this.page.waitForFunction(() => {
          return document.readyState === 'complete' && 
                 typeof window !== 'undefined' &&
                 !document.querySelector('.MuiCircularProgress-root');
        }, { timeout: 15000 });
        console.log('Firefox stability waiting completed');
        break;
      default:
        // Chromium and others
        await this.page.waitForLoadState('networkidle');
        await this.page.waitForTimeout(500);
        break;
    }
  }

  async navigateToPage(path: string) {
    await this.page.goto(path);
    await this.waitForLoadingToComplete();
    
    // WebKit-specific stability waiting
    const browserName = await this.page.evaluate(() => navigator.userAgent);
    const isWebKit = browserName.includes('WebKit') && !browserName.includes('Chrome');
    
    if (isWebKit) {
      console.log('WebKit detected - adding stability waiting for page:', path);
      
      // Wait for network to be completely idle
      await this.page.waitForLoadState('networkidle');
      await this.page.waitForTimeout(3000);
      
      // Wait for JavaScript to finish executing and ensure we're not stuck on login
      await this.page.waitForFunction(() => {
        return document.readyState === 'complete' && 
               typeof window !== 'undefined' && 
               !window.location.href.includes('/login') &&
               !window.location.pathname.includes('/login');
      }, { timeout: 20000 });
      
      // Extra stability wait
      await this.page.waitForTimeout(2000);
      console.log('WebKit stability waiting completed for:', path);
    }
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

  async createTestSource(baseName: string, type: string, options?: {
    mockResponse?: boolean;
    responseData?: any;
    uniqueSuffix?: string;
  }) {
    // Generate unique source name to avoid conflicts in concurrent tests
    const timestamp = Date.now();
    const randomSuffix = options?.uniqueSuffix || Math.random().toString(36).substring(7);
    const sourceName = `${baseName}_${timestamp}_${randomSuffix}`;
    
    // Set up mock if requested
    if (options?.mockResponse) {
      const responseData = options.responseData || {
        id: `source_${timestamp}`,
        name: sourceName,
        type: type,
        status: 'active',
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString()
      };
      
      // Mock the POST request to create source
      await this.page.route('**/api/sources', async (route, request) => {
        if (request.method() === 'POST') {
          await route.fulfill({
            status: 200,
            contentType: 'application/json',
            body: JSON.stringify(responseData)
          });
        } else {
          await route.continue();
        }
      });
    }
    
    // Click the add source button
    await this.page.click('button:has-text("Add Source"), [data-testid="add-source"]');
    
    // Wait for dialog to appear - target the specific dialog paper element
    await expect(this.page.getByRole('dialog')).toBeVisible();
    
    // Fill in source details - use more reliable selectors
    const nameInput = this.page.getByLabel('Source Name');
    await nameInput.fill(sourceName);
    
    // For Material-UI Select, we need to click and then select the option
    const typeSelect = this.page.getByLabel('Source Type');
    if (await typeSelect.isVisible()) {
      await typeSelect.click();
      await this.page.getByRole('option', { name: new RegExp(type, 'i') }).click();
    }
    
    // Add type-specific fields using label-based selectors
    if (type === 'webdav') {
      await this.page.getByLabel('Server URL').fill('https://test.webdav.server');
      await this.page.getByLabel('Username').fill('testuser');
      await this.page.getByLabel('Password').fill('testpass');
    } else if (type === 's3') {
      await this.page.getByLabel('Bucket Name').fill('test-bucket');
      await this.page.getByLabel('Region').fill('us-east-1');
      await this.page.getByLabel('Access Key ID').fill('test-access-key');
      await this.page.getByLabel('Secret Access Key').fill('test-secret-key');
    } else if (type === 'local_folder') {
      // For local folder, we need to add a directory path
      const addFolderInput = this.page.getByLabel(/Add.*Path/i);
      if (await addFolderInput.isVisible()) {
        await addFolderInput.fill('/test/path');
        await this.page.getByRole('button', { name: /Add.*Folder/i }).click();
      }
    }
    
    // Submit the form
    const createPromise = this.waitForApiCall('/api/sources', 10000);
    await this.page.getByRole('button', { name: /Create|Save/i }).click();
    
    // Wait for source to be created
    await createPromise;
    await this.waitForToast();
    
    // Verify the source appears in the list
    await expect(this.page.locator(`[data-testid="source-item"]:has-text("${sourceName}")`)).toBeVisible();
    
    // Return the generated source name so tests can reference it
    return sourceName;
  }
}