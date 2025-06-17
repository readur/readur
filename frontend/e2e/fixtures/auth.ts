import { test as base, expect } from '@playwright/test';
import type { Page } from '@playwright/test';

export interface AuthFixture {
  authenticatedPage: Page;
}

export const test = base.extend<AuthFixture>({
  authenticatedPage: async ({ page }, use) => {
    await page.goto('/');
    
    // Check if already logged in by looking for username input (login page)
    const usernameInput = await page.locator('input[name="username"]').isVisible().catch(() => false);
    
    if (usernameInput) {
      // Fill login form with demo credentials
      await page.fill('input[name="username"]', 'admin');
      await page.fill('input[name="password"]', 'readur2024');
      await page.click('button[type="submit"]');
      
      // Wait for navigation away from login page
      await page.waitForURL(/\/dashboard|\//, { timeout: 10000 });
    }
    
    await use(page);
  },
});

export { expect };