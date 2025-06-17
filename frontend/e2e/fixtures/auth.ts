import { test as base, expect } from '@playwright/test';
import type { Page } from '@playwright/test';

export interface AuthFixture {
  authenticatedPage: Page;
}

export const test = base.extend<AuthFixture>({
  authenticatedPage: async ({ page }, use) => {
    await page.goto('/');
    
    // Wait a bit for the page to load
    await page.waitForLoadState('networkidle');
    
    // Check if already logged in by looking for username input (login page)
    const usernameInput = await page.locator('input[name="username"]').isVisible().catch(() => false);
    
    if (usernameInput) {
      console.log('Found login form, attempting to login...');
      
      // Fill login form with demo credentials
      await page.fill('input[name="username"]', 'admin');
      await page.fill('input[name="password"]', 'readur2024');
      
      // Wait for the login API call response
      const loginPromise = page.waitForResponse(response => 
        response.url().includes('/auth/login') && response.status() === 200,
        { timeout: 10000 }
      );
      
      await page.click('button[type="submit"]');
      
      try {
        await loginPromise;
        console.log('Login API call successful');
        
        // Wait for redirect or URL change
        await page.waitForFunction(() => 
          !window.location.pathname.includes('/login'),
          { timeout: 10000 }
        );
        
        console.log('Redirected to:', page.url());
      } catch (error) {
        console.log('Login failed or timeout:', error);
      }
    } else {
      console.log('Already logged in or no login form found');
    }
    
    await use(page);
  },
});

export { expect };