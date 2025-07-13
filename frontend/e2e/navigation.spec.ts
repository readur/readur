import { test, expect } from './fixtures/auth';
import { TestHelpers } from './utils/test-helpers';

test.describe('Navigation', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ authenticatedPage }) => {
    helpers = new TestHelpers(authenticatedPage);
  });

  test('should check available routes after login', async ({ authenticatedPage: page }) => {
    // Check current URL after login
    console.log('Current URL after login:', page.url());
    
    // Try to navigate to various pages and see what works
    const routes = ['/dashboard', '/upload', '/search', '/documents', '/sources', '/settings'];
    
    for (const route of routes) {
      console.log(`\nTesting route: ${route}`);
      
      try {
        await page.goto(route);
        await page.waitForLoadState('networkidle', { timeout: 5000 });
        
        const title = await page.title();
        const currentUrl = page.url();
        console.log(`✅ ${route} -> ${currentUrl} (title: ${title})`);
        
        // Check if there are any obvious error messages
        const errorElements = page.locator(':has-text("Error"), :has-text("Not found"), :has-text("404")');
        const hasError = await errorElements.count() > 0;
        if (hasError) {
          console.log(`⚠️  Possible error on ${route}`);
        }
        
        // Check for file input on upload page
        if (route === '/upload') {
          const fileInputs = await page.locator('input[type="file"]').count();
          const dropzones = await page.locator(':has-text("Drag"), :has-text("Choose"), [role="button"]').count();
          console.log(`  File inputs: ${fileInputs}, Dropzones: ${dropzones}`);
          
          // Get page content for debugging
          const bodyText = await page.locator('body').textContent();
          console.log(`  Upload page content preview: ${bodyText?.substring(0, 200)}...`);
        }
        
      } catch (error) {
        console.log(`❌ ${route} failed: ${error}`);
      }
    }
  });

  test('should check what elements are on dashboard', async ({ authenticatedPage: page }) => {
    await page.goto('/dashboard');
    await page.waitForLoadState('networkidle', { timeout: 5000 });
    
    console.log('Dashboard URL:', page.url());
    
    // Check for welcome message
    const welcomeMessage = await page.locator('h4:has-text("Welcome back,")').isVisible();
    console.log('Welcome message present:', welcomeMessage);
    
    // Check for common navigation elements
    const navLinks = await page.locator('a, button').allTextContents();
    console.log('Navigation elements:', navLinks);
    
    // Check for any upload-related elements on dashboard
    const uploadElements = await page.locator(':has-text("Upload"), :has-text("File"), input[type="file"]').count();
    console.log('Upload elements on dashboard:', uploadElements);
    
    if (uploadElements > 0) {
      const uploadTexts = await page.locator(':has-text("Upload"), :has-text("File")').allTextContents();
      console.log('Upload-related text:', uploadTexts);
    }
    
    // Verify we're properly logged in
    await expect(page.locator('h4:has-text("Welcome back,")')).toBeVisible();
  });
});