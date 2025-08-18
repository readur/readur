/**
 * Example E2E test using the mock API framework
 * Demonstrates Playwright integration with MSW
 */

import { test, expect, Page } from '@playwright/test'

// Setup mock API for browser environment
test.beforeEach(async ({ page }) => {
  // Initialize MSW in browser context
  await page.addInitScript(() => {
    // Enable MSW worker
    window.__MSW_ENABLED__ = true
    
    // Set default scenario
    window.__MOCK_API_SCENARIO__ = 'ACTIVE_SYSTEM'
  })

  // Navigate to app
  await page.goto('/')
  
  // Wait for mock service worker to be ready
  await page.waitForFunction(() => window.__MSW_READY__ === true, { timeout: 5000 })
})

test.describe('E2E Tests with Mock API', () => {
  test('complete user workflow - search and view documents', async ({ page }) => {
    // Step 1: Navigate to search
    await page.click('[data-testid="nav-search"]')
    await expect(page.locator('[data-testid="search-page"]')).toBeVisible()

    // Step 2: Perform search
    await page.fill('[data-testid="search-input"]', 'invoice')
    await page.click('[data-testid="search-button"]')

    // Step 3: Wait for search results
    await expect(page.locator('[data-testid="search-results"]')).toBeVisible()
    
    // Step 4: Verify results appear
    const resultItems = page.locator('[data-testid="search-result"]')
    await expect(resultItems).toHaveCount(5) // Mock data returns 5 PDF results

    // Step 5: Click on first result
    await resultItems.first().click()

    // Step 6: Verify document viewer opens
    await expect(page.locator('[data-testid="document-viewer"]')).toBeVisible()
    await expect(page.locator('[data-testid="document-title"]')).toContainText('.pdf')
  })

  test('document upload workflow', async ({ page }) => {
    // Step 1: Navigate to upload page
    await page.click('[data-testid="nav-upload"]')
    await expect(page.locator('[data-testid="upload-page"]')).toBeVisible()

    // Step 2: Upload a file
    const fileInput = page.locator('[data-testid="file-input"]')
    await fileInput.setInputFiles({
      name: 'test-document.pdf',
      mimeType: 'application/pdf',
      buffer: Buffer.from('fake pdf content')
    })

    // Step 3: Wait for upload to complete
    await expect(page.locator('[data-testid="upload-success"]')).toBeVisible({ timeout: 10000 })

    // Step 4: Verify document appears in recent uploads
    await expect(page.locator('[data-testid="recent-upload"]')).toContainText('test-document.pdf')

    // Step 5: Check OCR processing status
    const ocrStatus = page.locator('[data-testid="ocr-status"]')
    await expect(ocrStatus).toContainText('pending') // Initially pending
    
    // Wait for OCR completion (mocked)
    await expect(ocrStatus).toContainText('completed', { timeout: 5000 })
  })

  test('real-time sync progress monitoring', async ({ page }) => {
    // Step 1: Navigate to sources page
    await page.click('[data-testid="nav-sources"]')
    await expect(page.locator('[data-testid="sources-page"]')).toBeVisible()

    // Step 2: Find a source and start sync
    const firstSource = page.locator('[data-testid="source-item"]').first()
    await firstSource.locator('[data-testid="sync-button"]').click()

    // Step 3: Verify sync starts
    await expect(page.locator('[data-testid="sync-progress"]')).toBeVisible()

    // Step 4: Monitor progress updates via WebSocket
    const progressBar = page.locator('[data-testid="progress-bar"]')
    const progressText = page.locator('[data-testid="progress-text"]')

    // Initial state
    await expect(progressText).toContainText('discovery')

    // Progress updates
    await expect(progressText).toContainText('processing', { timeout: 3000 })
    await expect(progressBar).toHaveAttribute('value', /[1-9]/, { timeout: 3000 })

    // Completion
    await expect(progressText).toContainText('completed', { timeout: 10000 })
    await expect(progressBar).toHaveAttribute('value', '100')
  })

  test('error handling and offline behavior', async ({ page }) => {
    // Step 1: Simulate network error
    await page.evaluate(() => {
      window.__MOCK_API_SCENARIO__ = 'NETWORK_ERROR'
    })

    // Step 2: Try to perform search
    await page.goto('/search')
    await page.fill('[data-testid="search-input"]', 'test query')
    await page.click('[data-testid="search-button"]')

    // Step 3: Verify error message appears
    await expect(page.locator('[data-testid="error-message"]')).toBeVisible()
    await expect(page.locator('[data-testid="error-message"]')).toContainText('network error')

    // Step 4: Verify retry functionality
    await page.click('[data-testid="retry-button"]')
    
    // Step 5: Simulate network recovery
    await page.evaluate(() => {
      window.__MOCK_API_SCENARIO__ = 'ACTIVE_SYSTEM'
    })

    // Step 6: Retry should succeed
    await page.click('[data-testid="retry-button"]')
    await expect(page.locator('[data-testid="search-results"]')).toBeVisible()
  })

  test('authentication flow', async ({ page }) => {
    // Step 1: Start logged out
    await page.evaluate(() => {
      window.__MOCK_API_SCENARIO__ = 'LOGGED_OUT'
    })

    await page.goto('/')

    // Step 2: Should redirect to login
    await expect(page.locator('[data-testid="login-page"]')).toBeVisible()

    // Step 3: Perform login
    await page.fill('[data-testid="username-input"]', 'testuser')
    await page.fill('[data-testid="password-input"]', 'password')
    await page.click('[data-testid="login-button"]')

    // Step 4: Should redirect to dashboard
    await expect(page.locator('[data-testid="dashboard"]')).toBeVisible()

    // Step 5: Verify user info is displayed
    await expect(page.locator('[data-testid="user-menu"]')).toContainText('testuser')

    // Step 6: Test logout
    await page.click('[data-testid="user-menu"]')
    await page.click('[data-testid="logout-button"]')

    // Step 7: Should redirect back to login
    await expect(page.locator('[data-testid="login-page"]')).toBeVisible()
  })

  test('mobile responsive behavior', async ({ page }) => {
    // Step 1: Set mobile viewport
    await page.setViewportSize({ width: 375, height: 667 })

    // Step 2: Navigate to dashboard
    await page.goto('/dashboard')

    // Step 3: Verify mobile navigation
    const mobileMenu = page.locator('[data-testid="mobile-menu-button"]')
    await expect(mobileMenu).toBeVisible()

    // Step 4: Open mobile menu
    await mobileMenu.click()
    await expect(page.locator('[data-testid="mobile-nav"]')).toBeVisible()

    // Step 5: Navigate via mobile menu
    await page.click('[data-testid="mobile-nav-search"]')
    await expect(page.locator('[data-testid="search-page"]')).toBeVisible()

    // Step 6: Test mobile search interface
    const searchInput = page.locator('[data-testid="search-input"]')
    await expect(searchInput).toBeVisible()
    
    await searchInput.fill('mobile test')
    await page.click('[data-testid="search-button"]')

    // Step 7: Verify mobile search results layout
    await expect(page.locator('[data-testid="search-results"]')).toBeVisible()
    
    // Results should stack vertically on mobile
    const results = page.locator('[data-testid="search-result"]')
    const firstResult = results.first()
    const secondResult = results.nth(1)
    
    const firstBox = await firstResult.boundingBox()
    const secondBox = await secondResult.boundingBox()
    
    // Verify vertical stacking (second result should be below first)
    expect(secondBox!.y).toBeGreaterThan(firstBox!.y + firstBox!.height)
  })

  test('accessibility compliance', async ({ page }) => {
    // Step 1: Navigate to main pages
    const pages = ['/', '/dashboard', '/search', '/upload', '/sources']
    
    for (const url of pages) {
      await page.goto(url)
      
      // Step 2: Check for accessibility landmarks
      await expect(page.locator('main')).toBeVisible()
      await expect(page.locator('nav')).toBeVisible()
      
      // Step 3: Verify keyboard navigation
      await page.keyboard.press('Tab')
      const focusedElement = await page.locator(':focus').first()
      await expect(focusedElement).toBeVisible()
      
      // Step 4: Check for proper heading structure
      const h1Elements = page.locator('h1')
      const h1Count = await h1Elements.count()
      expect(h1Count).toBe(1) // Should have exactly one h1 per page
      
      // Step 5: Verify images have alt text
      const images = page.locator('img')
      const imageCount = await images.count()
      
      for (let i = 0; i < imageCount; i++) {
        const img = images.nth(i)
        const altText = await img.getAttribute('alt')
        expect(altText).toBeTruthy() // All images should have alt text
      }
    }
  })

  test('performance monitoring', async ({ page }) => {
    // Step 1: Start performance measurement
    const startTime = Date.now()

    // Step 2: Navigate to heavy data page
    await page.evaluate(() => {
      window.__MOCK_API_SCENARIO__ = 'SYSTEM_UNDER_LOAD'
    })

    await page.goto('/dashboard')

    // Step 3: Wait for page to be fully loaded
    await page.waitForLoadState('networkidle')

    // Step 4: Measure load time
    const loadTime = Date.now() - startTime
    expect(loadTime).toBeLessThan(5000) // Should load within 5 seconds

    // Step 5: Check for performance metrics
    const performanceMetrics = await page.evaluate(() => {
      const navigation = performance.getEntriesByType('navigation')[0] as PerformanceNavigationTiming
      return {
        domContentLoaded: navigation.domContentLoadedEventEnd - navigation.navigationStart,
        loadComplete: navigation.loadEventEnd - navigation.navigationStart,
        firstPaint: performance.getEntriesByName('first-paint')[0]?.startTime || 0,
        firstContentfulPaint: performance.getEntriesByName('first-contentful-paint')[0]?.startTime || 0,
      }
    })

    // Performance assertions
    expect(performanceMetrics.domContentLoaded).toBeLessThan(3000)
    expect(performanceMetrics.firstContentfulPaint).toBeLessThan(2000)

    console.log('Performance Metrics:', performanceMetrics)
  })
})

// Helper function to setup specific test scenarios
async function setupMockScenario(page: Page, scenario: string) {
  await page.evaluate((scenarioName) => {
    window.__MOCK_API_SCENARIO__ = scenarioName
  }, scenario)
}

// Helper function to wait for network requests to complete
async function waitForNetworkIdle(page: Page, timeout = 5000) {
  await page.waitForLoadState('networkidle', { timeout })
}

// Helper function to simulate slow network
async function simulateSlowNetwork(page: Page, delay = 1000) {
  await page.evaluate((delayMs) => {
    window.__MOCK_NETWORK_DELAY__ = delayMs
  }, delay)
}

// Test with different scenarios
test.describe('Scenario-based E2E Tests', () => {
  test('empty system scenario', async ({ page }) => {
    await setupMockScenario(page, 'EMPTY_SYSTEM')
    await page.goto('/dashboard')

    // Verify empty states
    await expect(page.locator('[data-testid="empty-documents"]')).toBeVisible()
    await expect(page.locator('[data-testid="empty-sources"]')).toBeVisible()
  })

  test('problematic system scenario', async ({ page }) => {
    await setupMockScenario(page, 'PROBLEMATIC_SYSTEM')
    await page.goto('/dashboard')

    // Verify error indicators
    await expect(page.locator('[data-testid="error-indicator"]')).toBeVisible()
    await expect(page.locator('[data-testid="failed-ocr-count"]')).toContainText(/[1-9]/)
  })

  test('slow network conditions', async ({ page }) => {
    await simulateSlowNetwork(page, 2000)
    await page.goto('/search')

    // Search should show loading state longer
    await page.fill('[data-testid="search-input"]', 'test')
    await page.click('[data-testid="search-button"]')

    // Verify loading indicator appears
    await expect(page.locator('[data-testid="search-loading"]')).toBeVisible()
    
    // Loading should persist for at least 1.5 seconds
    await page.waitForTimeout(1500)
    await expect(page.locator('[data-testid="search-loading"]')).toBeVisible()

    // Eventually results should appear
    await expect(page.locator('[data-testid="search-results"]')).toBeVisible({ timeout: 5000 })
  })
})