import { test, expect } from './fixtures/auth';
import { SEARCH_QUERIES, TIMEOUTS, API_ENDPOINTS } from './utils/test-data';
import { TestHelpers } from './utils/test-helpers';

test.describe('Search Functionality', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ authenticatedPage }) => {
    helpers = new TestHelpers(authenticatedPage);
    await helpers.navigateToPage('/search');
    // Ensure we have test documents for search functionality
    await helpers.ensureTestDocumentsExist();
  });

  test.skip('should display search interface', async ({ authenticatedPage: page }) => {
    // Check for search components
    await expect(page.locator('input[type="search"], input[placeholder*="search" i], [data-testid="search-input"]')).toBeVisible();
    await expect(page.locator('button:has-text("Search"), [data-testid="search-button"]')).toBeVisible();
  });

  test.skip('should perform basic search', async ({ authenticatedPage: page }) => {
    const searchInput = page.locator('input[type="search"], input[placeholder*="search" i], [data-testid="search-input"]').first();
    
    // Search for known OCR content from test images
    await searchInput.fill(SEARCH_QUERIES.simple);  // "Test 1"
    
    // Wait for search API call
    const searchResponse = helpers.waitForApiCall(API_ENDPOINTS.search);
    
    // Press Enter or click search button
    await searchInput.press('Enter');
    
    // Verify search was performed
    await searchResponse;
    
    // Should show search results
    await expect(page.locator('[data-testid="search-results"], .search-results')).toBeVisible({ 
      timeout: TIMEOUTS.medium 
    });
  });

  test.skip('should show search suggestions', async ({ authenticatedPage: page }) => {
    const searchInput = page.locator('input[type="search"], input[placeholder*="search" i], [data-testid="search-input"]').first();
    
    // Start typing "Test" to trigger suggestions based on OCR content
    await searchInput.type('Test', { delay: 100 });
    
    // Should show suggestion dropdown
    await expect(page.locator('[data-testid="search-suggestions"], .suggestions, .autocomplete')).toBeVisible({ 
      timeout: TIMEOUTS.short 
    });
  });

  test.skip('should filter search results', async ({ authenticatedPage: page }) => {
    const searchInput = page.locator('input[type="search"], input[placeholder*="search" i], [data-testid="search-input"]').first();
    
    // Search for content that should match multiple test images
    await searchInput.fill(SEARCH_QUERIES.content);  // "some text from text"
    await searchInput.press('Enter');
    
    await helpers.waitForLoadingToComplete();
    
    // Apply filters
    const filterButton = page.locator('[data-testid="filters"], button:has-text("Filter"), .filter-toggle');
    if (await filterButton.isVisible()) {
      await filterButton.click();
      
      // Select image type filter (since our test files are images)
      const imageFilter = page.locator('input[type="checkbox"][value="image"], input[type="checkbox"][value="png"], label:has-text("Image")');
      if (await imageFilter.isVisible()) {
        await imageFilter.check();
        
        // Should update search results
        await helpers.waitForApiCall(API_ENDPOINTS.search);
      }
    }
  });

  test.skip('should perform advanced search', async ({ authenticatedPage: page }) => {
    // Look for advanced search toggle
    const advancedToggle = page.locator('[data-testid="advanced-search"], button:has-text("Advanced"), .advanced-toggle');
    
    if (await advancedToggle.isVisible()) {
      await advancedToggle.click();
      
      // Fill advanced search fields
      await page.fill('[data-testid="title-search"], input[name="title"]', SEARCH_QUERIES.advanced.title);
      await page.fill('[data-testid="content-search"], input[name="content"]', SEARCH_QUERIES.advanced.content);
      
      // Set date filters if available
      const dateFromInput = page.locator('[data-testid="date-from"], input[name="dateFrom"], input[type="date"]').first();
      if (await dateFromInput.isVisible()) {
        await dateFromInput.fill(SEARCH_QUERIES.advanced.dateFrom);
      }
      
      // Perform advanced search
      await page.click('button:has-text("Search"), [data-testid="search-button"]');
      
      // Verify search results
      await expect(page.locator('[data-testid="search-results"], .search-results')).toBeVisible({ 
        timeout: TIMEOUTS.medium 
      });
    }
  });

  test.skip('should handle empty search results', async ({ authenticatedPage: page }) => {
    const searchInput = page.locator('input[type="search"], input[placeholder*="search" i], [data-testid="search-input"]').first();
    
    // Search for something that doesn't exist
    await searchInput.fill(SEARCH_QUERIES.noResults);
    await searchInput.press('Enter');
    
    await helpers.waitForLoadingToComplete();
    
    // Should show no results message
    await expect(page.locator(':has-text("No results"), :has-text("not found"), [data-testid="no-results"]')).toBeVisible({ 
      timeout: TIMEOUTS.medium 
    });
  });

  test.skip('should navigate to document from search results', async ({ authenticatedPage: page }) => {
    const searchInput = page.locator('input[type="search"], input[placeholder*="search" i], [data-testid="search-input"]').first();
    
    // Perform search
    await searchInput.fill(SEARCH_QUERIES.simple);
    await searchInput.press('Enter');
    
    await helpers.waitForLoadingToComplete();
    
    // Click on first search result
    const firstResult = page.locator('[data-testid="search-results"] > *, .search-result').first();
    if (await firstResult.isVisible()) {
      await firstResult.click();
      
      // Should navigate to document details
      await page.waitForURL(/\/documents\/[^\/]+/, { timeout: TIMEOUTS.medium });
    }
  });

  test.skip('should preserve search state on page reload', async ({ authenticatedPage: page }) => {
    const searchInput = page.locator('input[type="search"], input[placeholder*="search" i], [data-testid="search-input"]').first();
    
    // Perform search
    await searchInput.fill(SEARCH_QUERIES.simple);
    await searchInput.press('Enter');
    
    await helpers.waitForLoadingToComplete();
    
    // Reload page
    await page.reload();
    
    // Should preserve search query and results
    await expect(searchInput).toHaveValue(SEARCH_QUERIES.simple);
    await expect(page.locator('[data-testid="search-results"], .search-results')).toBeVisible({ 
      timeout: TIMEOUTS.medium 
    });
  });

  test.skip('should sort search results', async ({ authenticatedPage: page }) => {
    const searchInput = page.locator('input[type="search"], input[placeholder*="search" i], [data-testid="search-input"]').first();
    
    // Perform search
    await searchInput.fill(SEARCH_QUERIES.simple);
    await searchInput.press('Enter');
    
    await helpers.waitForLoadingToComplete();
    
    // Look for sort options
    const sortDropdown = page.locator('[data-testid="sort"], select[name="sort"], .sort-selector');
    if (await sortDropdown.isVisible()) {
      await sortDropdown.selectOption('date-desc');
      
      // Should update search results order
      await helpers.waitForApiCall(API_ENDPOINTS.search);
    }
  });

  test.skip('should paginate search results', async ({ authenticatedPage: page }) => {
    const searchInput = page.locator('input[type="search"], input[placeholder*="search" i], [data-testid="search-input"]').first();
    
    // Perform search
    await searchInput.fill(SEARCH_QUERIES.simple);
    await searchInput.press('Enter');
    
    await helpers.waitForLoadingToComplete();
    
    // Look for pagination
    const nextPageButton = page.locator('[data-testid="next-page"], button:has-text("Next"), .pagination button:last-child');
    if (await nextPageButton.isVisible()) {
      await nextPageButton.click();
      
      // Should load next page of results
      await helpers.waitForApiCall(API_ENDPOINTS.search);
      await expect(page.locator('[data-testid="search-results"], .search-results')).toBeVisible({ 
        timeout: TIMEOUTS.medium 
      });
    }
  });

  test.skip('should highlight search terms in results', async ({ authenticatedPage: page }) => {
    const searchInput = page.locator('input[type="search"], input[placeholder*="search" i], [data-testid="search-input"]').first();
    
    // Perform search with specific term
    await searchInput.fill('test');
    await searchInput.press('Enter');
    
    await helpers.waitForLoadingToComplete();
    
    // Should highlight search terms in results
    await expect(page.locator('.highlight, mark, .search-highlight')).toBeVisible({ 
      timeout: TIMEOUTS.medium 
    });
  });

  test.skip('should clear search results', async ({ authenticatedPage: page }) => {
    const searchInput = page.locator('input[type="search"], input[placeholder*="search" i], [data-testid="search-input"]').first();
    
    // Perform search
    await searchInput.fill(SEARCH_QUERIES.simple);
    await searchInput.press('Enter');
    
    await helpers.waitForLoadingToComplete();
    
    // Clear search
    const clearButton = page.locator('[data-testid="clear-search"], button:has-text("Clear"), .clear-button');
    if (await clearButton.isVisible()) {
      await clearButton.click();
    } else {
      // Clear by emptying input
      await searchInput.clear();
      await searchInput.press('Enter');
    }
    
    // Should clear results
    await expect(page.locator('[data-testid="search-results"], .search-results')).not.toBeVisible();
  });
});