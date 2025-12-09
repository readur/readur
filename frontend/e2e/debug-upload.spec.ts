import { test, expect } from './fixtures/auth';
import { TEST_FILES, TIMEOUTS } from './utils/test-data';
import { TestHelpers } from './utils/test-helpers';

test.describe('Debug Upload', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ dynamicAdminPage }) => {
    helpers = new TestHelpers(dynamicAdminPage);
    await dynamicAdminPage.goto('/upload');
    await helpers.waitForLoadingToComplete();
  });

  test('should debug upload workflow', async ({ dynamicAdminPage: page }) => {
    console.log('Starting upload debug test...');
    
    // Find file input
    const fileInput = page.locator('input[type="file"]').first();
    console.log('Found file input');
    
    // Upload a file
    await fileInput.setInputFiles(TEST_FILES.test1);
    console.log('File added to input');
    
    // Wait a moment for file to be processed by dropzone
    await page.waitForTimeout(1000);
    
    // Log all button text on the page
    const allButtons = await page.locator('button').allTextContents();
    console.log('All buttons on page:', allButtons);
    
    // Log all text content that might indicate upload state
    const uploadTexts = await page.locator(':has-text("Upload"), :has-text("File"), :has-text("Progress")').allTextContents();
    console.log('Upload-related text:', uploadTexts);
    
    // Look for upload button specifically
    const uploadButton = page.locator('button:has-text("Upload All"), button:has-text("Upload")');
    const uploadButtonCount = await uploadButton.count();
    console.log('Upload button count:', uploadButtonCount);
    
    if (uploadButtonCount > 0) {
      const uploadButtonText = await uploadButton.first().textContent();
      console.log('Upload button text:', uploadButtonText);
      
      // Click the upload button
      console.log('Clicking upload button...');
      await uploadButton.first().click();
      
      // Wait and log state changes
      for (let i = 0; i < 10; i++) {
        await page.waitForTimeout(1000);
        
        const currentTexts = await page.locator('body').textContent();
        console.log(`After ${i+1}s: Page contains "progress": ${currentTexts?.toLowerCase().includes('progress')}`);
        console.log(`After ${i+1}s: Page contains "success": ${currentTexts?.toLowerCase().includes('success')}`);
        console.log(`After ${i+1}s: Page contains "complete": ${currentTexts?.toLowerCase().includes('complete')}`);
        console.log(`After ${i+1}s: Page contains "uploaded": ${currentTexts?.toLowerCase().includes('uploaded')}`);
        
        // Check for any status changes in specific areas
        const uploadArea = page.locator('[role="main"], .upload-area, .dropzone').first();
        if (await uploadArea.count() > 0) {
          const uploadAreaText = await uploadArea.textContent();
          console.log(`Upload area content: ${uploadAreaText?.substring(0, 200)}...`);
        }
      }
    } else {
      console.log('No upload button found!');
    }
  });
});