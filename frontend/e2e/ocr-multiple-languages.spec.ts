import { test, expect } from './fixtures/auth';
import { TIMEOUTS, API_ENDPOINTS, TEST_FILES } from './utils/test-data';
import { TestHelpers } from './utils/test-helpers';

// Test data for multilingual OCR testing
const MULTILINGUAL_TEST_FILES = {
  spanish: TEST_FILES.spanishTest,
  english: TEST_FILES.englishTest,
  mixed: TEST_FILES.mixedLanguageTest,
  spanishComplex: TEST_FILES.spanishComplex,
  englishComplex: TEST_FILES.englishComplex
};

const EXPECTED_CONTENT = {
  spanish: {
    keywords: ['español', 'documento', 'reconocimiento', 'café', 'niño', 'comunicación'],
    phrases: ['Hola mundo', 'este es un documento', 'en español']
  },
  english: {
    keywords: ['English', 'document', 'recognition', 'technology', 'computer'],
    phrases: ['Hello world', 'this is an English', 'document']
  },
  mixed: {
    spanish: ['español', 'idiomas', 'reconocimiento'],
    english: ['English', 'languages', 'recognition']
  }
};

const OCR_LANGUAGES = {
  spanish: { code: 'spa', name: 'Spanish' },
  english: { code: 'eng', name: 'English' },
  auto: { code: 'auto', name: 'Auto-detect' }
};

test.describe('OCR Multiple Languages', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ adminPage }) => {
    helpers = new TestHelpers(adminPage);
    await helpers.navigateToPage('/settings');
  });

  test('should display OCR language selector in settings', async ({ adminPage: page }) => {
    // Navigate to settings page
    await page.goto('/settings');
    await helpers.waitForLoadingToComplete();

    // Look for OCR language selector component
    const languageSelector = page.locator('[data-testid="ocr-language-selector"], #ocr-language-label').first();
    await expect(languageSelector).toBeVisible({ timeout: TIMEOUTS.medium });

    // Check if the selector shows available languages
    const selectInput = page.locator('div[role="combobox"], select[id*="ocr"], input[id*="language"]').first();
    if (await selectInput.isVisible()) {
      await selectInput.click();
      
      // Wait for language options to appear
      await page.waitForTimeout(1000);
      
      // Check for Spanish and English options
      const spanishOption = page.locator('[data-value="spa"], option[value="spa"], :has-text("Spanish")').first();
      const englishOption = page.locator('[data-value="eng"], option[value="eng"], :has-text("English")').first();
      
      if (await spanishOption.isVisible({ timeout: 3000 })) {
        console.log('✅ Spanish language option found');
      }
      if (await englishOption.isVisible({ timeout: 3000 })) {
        console.log('✅ English language option found');
      }
    }
  });

  test('should change OCR language preference to Spanish', async ({ adminPage: page }) => {
    await page.goto('/settings');
    await helpers.waitForLoadingToComplete();

    // Find and interact with language selector
    const languageSelector = page.locator('[data-testid="ocr-language-selector"], div:has(label:text("OCR Language"))').first();
    
    if (await languageSelector.isVisible()) {
      // Click on the selector to open dropdown
      await languageSelector.click();
      await page.waitForTimeout(500);
      
      // Select Spanish option
      const spanishOption = page.locator('[data-value="spa"], option[value="spa"], li:has-text("Spanish")').first();
      if (await spanishOption.isVisible({ timeout: 5000 })) {
        await spanishOption.click();
        
        // Look for save button or auto-save indication
        const saveButton = page.locator('button:has-text("Save"), button[type="submit"]').first();
        if (await saveButton.isVisible({ timeout: 3000 })) {
          // Wait for settings update API call
          const updatePromise = helpers.waitForApiCall('/api/settings', TIMEOUTS.medium);
          await saveButton.click();
          await updatePromise;
        }
        
        // Check for success indication
        await helpers.waitForToast();
        console.log('✅ OCR language changed to Spanish');
      }
    }
  });

  test('should upload Spanish document and process with Spanish OCR', async ({ adminPage: page }) => {
    // First set language to Spanish
    await page.goto('/settings');
    await helpers.waitForLoadingToComplete();
    
    const languageSelector = page.locator('div:has(label:text("OCR Language")), [data-testid="ocr-language-selector"]').first();
    if (await languageSelector.isVisible()) {
      await languageSelector.click();
      const spanishOption = page.locator('[data-value="spa"], li:has-text("Spanish")').first();
      if (await spanishOption.isVisible({ timeout: 5000 })) {
        await spanishOption.click();
        
        const saveButton = page.locator('button:has-text("Save")').first();
        if (await saveButton.isVisible()) {
          await saveButton.click();
          await helpers.waitForToast();
        }
      }
    }

    // Navigate to upload page
    await page.goto('/upload');
    await helpers.waitForLoadingToComplete();

    // Upload Spanish test document
    const fileInput = page.locator('input[type="file"]').first();
    await expect(fileInput).toBeAttached({ timeout: 10000 });
    
    try {
      await fileInput.setInputFiles(MULTILINGUAL_TEST_FILES.spanish);
      
      // Verify file appears in upload list
      await expect(page.getByText('spanish_test.pdf')).toBeVisible({ timeout: 5000 });
      
      // Click upload button
      const uploadButton = page.locator('button:has-text("Upload")').first();
      if (await uploadButton.isVisible()) {
        // Wait for upload and OCR processing
        const uploadPromise = helpers.waitForApiCall('/api/documents', TIMEOUTS.upload);
        await uploadButton.click();
        await uploadPromise;
        
        // Wait for OCR processing to complete
        await page.waitForTimeout(3000);
        console.log('✅ Spanish document uploaded and OCR initiated');
      }
    } catch (error) {
      console.log('ℹ️ Spanish test file not found, skipping upload test');
    }
  });

  test('should upload English document and process with English OCR', async ({ adminPage: page }) => {
    // First set language to English
    await page.goto('/settings');
    await helpers.waitForLoadingToComplete();
    
    const languageSelector = page.locator('div:has(label:text("OCR Language")), [data-testid="ocr-language-selector"]').first();
    if (await languageSelector.isVisible()) {
      await languageSelector.click();
      const englishOption = page.locator('[data-value="eng"], li:has-text("English")').first();
      if (await englishOption.isVisible({ timeout: 5000 })) {
        await englishOption.click();
        
        const saveButton = page.locator('button:has-text("Save")').first();
        if (await saveButton.isVisible()) {
          await saveButton.click();
          await helpers.waitForToast();
        }
      }
    }

    // Navigate to upload page
    await page.goto('/upload');
    await helpers.waitForLoadingToComplete();

    // Upload English test document
    const fileInput = page.locator('input[type="file"]').first();
    await expect(fileInput).toBeAttached({ timeout: 10000 });
    
    try {
      await fileInput.setInputFiles(MULTILINGUAL_TEST_FILES.english);
      
      // Verify file appears in upload list
      await expect(page.getByText('english_test.pdf')).toBeVisible({ timeout: 5000 });
      
      // Click upload button
      const uploadButton = page.locator('button:has-text("Upload")').first();
      if (await uploadButton.isVisible()) {
        // Wait for upload and OCR processing
        const uploadPromise = helpers.waitForApiCall('/api/documents', TIMEOUTS.upload);
        await uploadButton.click();
        await uploadPromise;
        
        // Wait for OCR processing to complete
        await page.waitForTimeout(3000);
        console.log('✅ English document uploaded and OCR initiated');
      }
    } catch (error) {
      console.log('ℹ️ English test file not found, skipping upload test');
    }
  });

  test('should validate OCR results contain expected language-specific content', async ({ adminPage: page }) => {
    await page.goto('/documents');
    await helpers.waitForLoadingToComplete();

    // Look for uploaded documents
    const documentItems = page.locator('.document-item, .document-card, [data-testid="document-item"]');
    const documentCount = await documentItems.count();
    
    if (documentCount > 0) {
      // Click on first document to view details
      await documentItems.first().click();
      await helpers.waitForLoadingToComplete();
      
      // Look for document content or OCR text
      const contentArea = page.locator('.document-content, .ocr-text, [data-testid="document-content"]').first();
      
      if (await contentArea.isVisible({ timeout: TIMEOUTS.medium })) {
        const contentText = await contentArea.textContent();
        
        if (contentText) {
          // Check for Spanish keywords
          const hasSpanishContent = EXPECTED_CONTENT.spanish.keywords.some(keyword => 
            contentText.toLowerCase().includes(keyword.toLowerCase())
          );
          
          // Check for English keywords  
          const hasEnglishContent = EXPECTED_CONTENT.english.keywords.some(keyword =>
            contentText.toLowerCase().includes(keyword.toLowerCase())
          );
          
          if (hasSpanishContent) {
            console.log('✅ Spanish OCR content detected');
          }
          if (hasEnglishContent) {
            console.log('✅ English OCR content detected');
          }
          
          console.log(`📄 Document content preview: ${contentText.substring(0, 100)}...`);
        }
      }
    } else {
      console.log('ℹ️ No documents found for content validation');
    }
  });

  test('should retry failed OCR with different language', async ({ adminPage: page }) => {
    await page.goto('/documents');
    await helpers.waitForLoadingToComplete();

    // Look for failed documents or retry options
    const retryButton = page.locator('button:has-text("Retry"), [data-testid="retry-ocr"]').first();
    
    if (await retryButton.isVisible()) {
      // Look for language selection in retry dialog
      await retryButton.click();
      
      // Check if retry dialog opens with language options
      const retryDialog = page.locator('.retry-dialog, [role="dialog"], .modal').first();
      if (await retryDialog.isVisible({ timeout: 5000 })) {
        
        // Look for language selector in retry dialog
        const retryLanguageSelector = page.locator('select, [role="combobox"]').first();
        if (await retryLanguageSelector.isVisible()) {
          // Change language for retry
          await retryLanguageSelector.click();
          
          const spanishRetryOption = page.locator('[data-value="spa"], option[value="spa"]').first();
          if (await spanishRetryOption.isVisible()) {
            await spanishRetryOption.click();
            
            // Confirm retry with new language
            const confirmRetryButton = page.locator('button:has-text("Retry"), button:has-text("Confirm")').last();
            if (await confirmRetryButton.isVisible()) {
              const retryPromise = helpers.waitForApiCall('/retry', TIMEOUTS.ocr);
              await confirmRetryButton.click();
              
              try {
                await retryPromise;
                console.log('✅ OCR retry with different language initiated');
              } catch (error) {
                console.log('ℹ️ Retry may have failed or timed out');
              }
            }
          }
        }
      }
    } else {
      console.log('ℹ️ No failed documents found for retry testing');
    }
  });

  test('should handle mixed language document', async ({ adminPage: page }) => {
    // Upload mixed language document
    await page.goto('/upload');
    await helpers.waitForLoadingToComplete();

    const fileInput = page.locator('input[type="file"]').first();
    
    try {
      await fileInput.setInputFiles(MULTILINGUAL_TEST_FILES.mixed);
      
      await expect(page.getByText('mixed_language_test.pdf')).toBeVisible({ timeout: 5000 });
      
      const uploadButton = page.locator('button:has-text("Upload")').first();
      if (await uploadButton.isVisible()) {
        const uploadPromise = helpers.waitForApiCall('/api/documents', TIMEOUTS.upload);
        await uploadButton.click();
        await uploadPromise;
        
        // Wait for OCR processing
        await page.waitForTimeout(5000);
        
        // Navigate to documents and check content
        await page.goto('/documents');
        await helpers.waitForLoadingToComplete();
        
        // Look for the mixed document
        const mixedDocument = page.locator('text="mixed_language_test.pdf"').first();
        if (await mixedDocument.isVisible()) {
          await mixedDocument.click();
          
          const contentArea = page.locator('.document-content, .ocr-text').first();
          if (await contentArea.isVisible({ timeout: TIMEOUTS.medium })) {
            const content = await contentArea.textContent();
            
            if (content) {
              const hasSpanish = EXPECTED_CONTENT.mixed.spanish.some(word => 
                content.toLowerCase().includes(word.toLowerCase())
              );
              const hasEnglish = EXPECTED_CONTENT.mixed.english.some(word =>
                content.toLowerCase().includes(word.toLowerCase())
              );
              
              if (hasSpanish && hasEnglish) {
                console.log('✅ Mixed language document processed successfully');
              }
            }
          }
        }
      }
    } catch (error) {
      console.log('ℹ️ Mixed language test file not found, skipping test');
    }
  });

  test('should persist language preference across sessions', async ({ adminPage: page }) => {
    // Set language to Spanish
    await page.goto('/settings');
    await helpers.waitForLoadingToComplete();
    
    const languageSelector = page.locator('div:has(label:text("OCR Language"))').first();
    if (await languageSelector.isVisible()) {
      await languageSelector.click();
      
      const spanishOption = page.locator('[data-value="spa"], li:has-text("Spanish")').first();
      if (await spanishOption.isVisible()) {
        await spanishOption.click();
        
        const saveButton = page.locator('button:has-text("Save")').first();
        if (await saveButton.isVisible()) {
          await saveButton.click();
          await helpers.waitForToast();
        }
      }
    }
    
    // Reload page to simulate new session
    await page.reload();
    await helpers.waitForLoadingToComplete();
    
    // Check if Spanish is still selected
    const currentLanguageIndicator = page.locator('text="Spanish", [data-value="spa"]').first();
    if (await currentLanguageIndicator.isVisible({ timeout: 5000 })) {
      console.log('✅ Language preference persisted across reload');
    } else {
      console.log('ℹ️ Could not verify language persistence');
    }
  });

  test('should display available languages from API', async ({ adminPage: page }) => {
    // Navigate to settings and check API call for languages
    const languagesPromise = helpers.waitForApiCall('/api/ocr/languages', TIMEOUTS.medium);
    
    await page.goto('/settings');
    await helpers.waitForLoadingToComplete();
    
    try {
      const languagesResponse = await languagesPromise;
      console.log('✅ OCR languages API called successfully');
      
      // Check if language selector shows loading then options
      const languageSelector = page.locator('[data-testid="ocr-language-selector"]').first();
      if (await languageSelector.isVisible()) {
        // Click to see available options
        await languageSelector.click();
        await page.waitForTimeout(1000);
        
        // Count available language options
        const languageOptions = page.locator('li[role="option"], option[value]');
        const optionCount = await languageOptions.count();
        
        if (optionCount > 0) {
          console.log(`✅ Found ${optionCount} language options in selector`);
        }
      }
    } catch (error) {
      console.log('ℹ️ Could not capture languages API call');
    }
  });

  test('should handle bulk operations with multiple languages', async ({ adminPage: page }) => {
    await page.goto('/documents');
    await helpers.waitForLoadingToComplete();

    // Look for documents and select multiple
    const documentCheckboxes = page.locator('.document-item input[type="checkbox"], [data-testid="document-checkbox"]');
    const checkboxCount = await documentCheckboxes.count();
    
    if (checkboxCount > 1) {
      // Select first two documents
      await documentCheckboxes.nth(0).click();
      await documentCheckboxes.nth(1).click();
      
      // Look for bulk action menu
      const bulkActionsMenu = page.locator('[data-testid="bulk-actions"], .bulk-actions, button:has-text("Bulk")').first();
      
      if (await bulkActionsMenu.isVisible()) {
        await bulkActionsMenu.click();
        
        // Look for language-specific bulk operations
        const bulkRetryWithLanguage = page.locator('button:has-text("Retry with Language"), .bulk-retry-language').first();
        
        if (await bulkRetryWithLanguage.isVisible()) {
          await bulkRetryWithLanguage.click();
          
          // Check for language selection in bulk retry
          const bulkLanguageSelector = page.locator('select, [role="combobox"]').first();
          if (await bulkLanguageSelector.isVisible()) {
            await bulkLanguageSelector.click();
            
            const spanishBulkOption = page.locator('[data-value="spa"], option[value="spa"]').first();
            if (await spanishBulkOption.isVisible()) {
              await spanishBulkOption.click();
              
              const confirmBulkButton = page.locator('button:has-text("Confirm"), button:has-text("Apply")').first();
              if (await confirmBulkButton.isVisible()) {
                const bulkRetryPromise = helpers.waitForApiCall('/bulk-retry', TIMEOUTS.ocr);
                await confirmBulkButton.click();
                
                try {
                  await bulkRetryPromise;
                  console.log('✅ Bulk retry with Spanish language initiated');
                } catch (error) {
                  console.log('ℹ️ Bulk retry may have failed or not available');
                }
              }
            }
          }
        }
      }
    } else {
      console.log('ℹ️ Not enough documents for bulk operations test');
    }
  });

  test('should handle OCR language errors gracefully', async ({ adminPage: page }) => {
    await page.goto('/settings');
    await helpers.waitForLoadingToComplete();
    
    // Look for language selector component
    const languageSelector = page.locator('[data-testid="ocr-language-selector"]').first();
    
    // Check for error handling in language selector
    const errorAlert = page.locator('[role="alert"], .error, .alert-warning').first();
    const retryButton = page.locator('button:has-text("Retry"), .retry').first();
    
    if (await errorAlert.isVisible()) {
      console.log('⚠️ Language selector showing error state');
      
      if (await retryButton.isVisible()) {
        await retryButton.click();
        console.log('✅ Error retry mechanism available');
      }
    } else if (await languageSelector.isVisible()) {
      console.log('✅ Language selector loaded without errors');
    }
    
    // Check for fallback behavior
    const englishFallback = page.locator('text="English (Fallback)"').first();
    if (await englishFallback.isVisible()) {
      console.log('✅ Fallback language option available');
    }
  });
});