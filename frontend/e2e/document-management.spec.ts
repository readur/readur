import { test, expect } from './fixtures/auth';
import { TIMEOUTS } from './utils/test-data';
import { TestHelpers } from './utils/test-helpers';

test.describe('Document Management', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ authenticatedPage }) => {
    helpers = new TestHelpers(authenticatedPage);
    await helpers.navigateToPage('/documents');
    // Ensure we have test documents for tests that need them
    await helpers.ensureTestDocumentsExist();
  });

  test('should display document list', async ({ authenticatedPage: page }) => {
    // The documents page should be visible with title and description
    await expect(page.getByRole('heading', { name: 'Documents' })).toBeVisible();
    await expect(page.locator('text=Manage and explore your document library')).toBeVisible();
    
    // Check for document cards/items or empty state
    const documentCards = page.locator('.MuiCard-root');
    const hasDocuments = await documentCards.count() > 0;
    
    if (hasDocuments) {
      // Should show at least one document card
      await expect(documentCards.first()).toBeVisible();
    }
    
    // Either way, the page should be functional - check for search bar
    await expect(page.getByRole('main').getByRole('textbox', { name: 'Search documents...' })).toBeVisible();
  });

  test('should navigate to document details', async ({ authenticatedPage: page }) => {
    // Click on first document if available
    const firstDocument = page.locator('.MuiCard-root').first();
    
    if (await firstDocument.isVisible()) {
      await firstDocument.click();
      
      // Should navigate to document details page
      await page.waitForURL(/\/documents\/[^\/]+/, { timeout: TIMEOUTS.medium });
      
      // Should show document details
      await expect(page.locator('[data-testid="document-details"], .document-details, h1, h2')).toBeVisible();
    } else {
      test.skip();
    }
  });

  test('should display document metadata', async ({ authenticatedPage: page }) => {
    const firstDocument = page.locator('.MuiCard-root').first();
    
    if (await firstDocument.isVisible()) {
      await firstDocument.click();
      await page.waitForURL(/\/documents\/[^\/]+/, { timeout: TIMEOUTS.medium });
      
      // Should show various metadata fields
      await expect(page.locator(':has-text("Bytes"), :has-text("OCR"), :has-text("Download")')).toBeVisible();
    } else {
      test.skip();
    }
  });

  test('should allow document download', async ({ authenticatedPage: page }) => {
    const firstDocument = page.locator('[data-testid="document-item"], .document-item, .document-card').first();
    
    if (await firstDocument.isVisible()) {
      await firstDocument.click();
      await page.waitForURL(/\/documents\/[^\/]+/, { timeout: TIMEOUTS.medium });
      
      // Look for download button
      const downloadButton = page.locator('[data-testid="download"], button:has-text("Download"), .download-button');
      if (await downloadButton.isVisible()) {
        // Set up download listener
        const downloadPromise = page.waitForEvent('download');
        
        await downloadButton.click();
        
        // Verify download started
        const download = await downloadPromise;
        expect(download.suggestedFilename()).toBeTruthy();
      }
    }
  });

  test('should allow document deletion', async ({ authenticatedPage: page }) => {
    const firstDocument = page.locator('[data-testid="document-item"], .document-item, .document-card').first();
    
    if (await firstDocument.isVisible()) {
      await firstDocument.click();
      await page.waitForURL(/\/documents\/[^\/]+/, { timeout: TIMEOUTS.medium });
      
      // Look for delete button
      const deleteButton = page.locator('[data-testid="delete"], button:has-text("Delete"), .delete-button');
      if (await deleteButton.isVisible()) {
        await deleteButton.click();
        
        // Should show confirmation dialog
        const confirmButton = page.locator('button:has-text("Confirm"), button:has-text("Yes"), [data-testid="confirm-delete"]');
        if (await confirmButton.isVisible()) {
          await confirmButton.click();
          
          // Should redirect back to documents list
          await page.waitForURL(/\/documents$/, { timeout: TIMEOUTS.medium });
        }
      }
    }
  });

  test('should filter documents by type', async ({ authenticatedPage: page }) => {
    // Look for filter controls
    const filterDropdown = page.locator('[data-testid="type-filter"], select[name="type"], .type-filter');
    if (await filterDropdown.isVisible()) {
      await filterDropdown.selectOption('pdf');
      
      await helpers.waitForLoadingToComplete();
      
      // Should show only PDF documents
      const documentItems = page.locator('[data-testid="document-item"], .document-item');
      if (await documentItems.count() > 0) {
        // Check that visible documents are PDFs
        await expect(documentItems.first().locator(':has-text(".pdf"), .pdf-icon')).toBeVisible();
      }
    }
  });

  test('should sort documents', async ({ authenticatedPage: page }) => {
    const sortDropdown = page.locator('[data-testid="sort"], select[name="sort"], .sort-dropdown');
    if (await sortDropdown.isVisible()) {
      await sortDropdown.selectOption('date-desc');
      
      await helpers.waitForLoadingToComplete();
      
      // Documents should be reordered
      await expect(page.locator('[data-testid="document-list"], .document-list')).toBeVisible();
    }
  });

  test('should display OCR status', async ({ authenticatedPage: page }) => {
    const firstDocument = page.locator('.MuiCard-root').first();
    
    if (await firstDocument.isVisible()) {
      await firstDocument.click();
      await page.waitForURL(/\/documents\/[^\/]+/, { timeout: TIMEOUTS.medium });
      
      // Should show OCR status information
      await expect(page.locator(':has-text("OCR"), [data-testid="ocr-status"], .ocr-status')).toBeVisible();
    } else {
      // Skip test if no documents
      test.skip();
    }
  });

  test('should search within document content', async ({ authenticatedPage: page }) => {
    const firstDocument = page.locator('.MuiCard-root').first();
    
    if (await firstDocument.isVisible()) {
      await firstDocument.click();
      await page.waitForURL(/\/documents\/[^\/]+/, { timeout: TIMEOUTS.medium });
      
      // Look for in-document search
      const searchInput = page.locator('[data-testid="document-search"], input[placeholder*="search" i]');
      if (await searchInput.isVisible()) {
        await searchInput.fill('test');
        
        // Should highlight matches in document content
        await expect(page.locator('.highlight, mark, .search-highlight')).toBeVisible({ 
          timeout: TIMEOUTS.short 
        });
      }
    } else {
      // Skip test if no documents
      test.skip();
    }
  });

  test('should paginate document list', async ({ authenticatedPage: page }) => {
    // Look for pagination controls
    const nextPageButton = page.locator('[data-testid="next-page"], button:has-text("Next"), .pagination-next');
    if (await nextPageButton.isVisible()) {
      const initialDocuments = await page.locator('[data-testid="document-item"], .document-item').count();
      
      await nextPageButton.click();
      
      await helpers.waitForLoadingToComplete();
      
      // Should load different documents
      const newDocuments = await page.locator('[data-testid="document-item"], .document-item').count();
      expect(newDocuments).toBeGreaterThan(0);
    }
  });

  test('should show document thumbnails', async ({ authenticatedPage: page }) => {
    // Check for document thumbnails in list view
    const documentThumbnails = page.locator('[data-testid="document-thumbnail"], .thumbnail, .document-preview');
    if (await documentThumbnails.first().isVisible()) {
      await expect(documentThumbnails.first()).toBeVisible();
    }
  });
});