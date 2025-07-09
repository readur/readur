import { test, expect } from './fixtures/auth';
import { E2ETestAuthHelper } from './utils/test-auth-helper';

test.describe('E2E Auth System', () => {
  test('should create and login dynamic test user', async ({ page, testUser }) => {
    // The testUser fixture should have created a user and logged them in via dynamicUserPage
    expect(testUser.credentials.username).toMatch(/^e2e_user_\d+_\d+_[a-z0-9]+$/);
    expect(testUser.userResponse.role).toBe('User');
    
    console.log(`Test user created: ${testUser.credentials.username} (${testUser.userResponse.id})`);
  });

  test('should create and login dynamic admin user', async ({ page, testAdmin }) => {
    // The testAdmin fixture should have created an admin user
    expect(testAdmin.credentials.username).toMatch(/^e2e_admin_\d+_\d+_[a-z0-9]+$/);
    expect(testAdmin.userResponse.role).toBe('Admin');
    
    console.log(`Test admin created: ${testAdmin.credentials.username} (${testAdmin.userResponse.id})`);
  });

  test('should login dynamic user via browser UI', async ({ page, testUser }) => {
    const authHelper = new E2ETestAuthHelper(page);
    
    // Ensure we're logged out first
    await authHelper.ensureLoggedOut();
    
    // Login with the dynamic user
    const loginSuccess = await authHelper.loginUser(testUser.credentials);
    expect(loginSuccess).toBe(true);
    
    // Verify we're on the dashboard
    await expect(page).toHaveURL(/.*\/dashboard.*/);
    await expect(page.locator('h4:has-text("Welcome back,")')).toBeVisible();
    
    console.log(`Successfully logged in dynamic user: ${testUser.credentials.username}`);
  });

  test('should login dynamic admin via browser UI', async ({ page, testAdmin }) => {
    const authHelper = new E2ETestAuthHelper(page);
    
    // Ensure we're logged out first
    await authHelper.ensureLoggedOut();
    
    // Login with the dynamic admin
    const loginSuccess = await authHelper.loginUser(testAdmin.credentials);
    expect(loginSuccess).toBe(true);
    
    // Verify we're on the dashboard
    await expect(page).toHaveURL(/.*\/dashboard.*/);
    await expect(page.locator('h4:has-text("Welcome back,")')).toBeVisible();
    
    console.log(`Successfully logged in dynamic admin: ${testAdmin.credentials.username}`);
  });

  test('should support API login for dynamic users', async ({ page, testUser }) => {
    const authHelper = new E2ETestAuthHelper(page);
    
    // Login via API
    const token = await authHelper.loginUserAPI(testUser.credentials);
    expect(token).toBeTruthy();
    expect(typeof token).toBe('string');
    
    console.log(`Successfully got API token for: ${testUser.credentials.username}`);
  });

  test('should create unique users for each test', async ({ page }) => {
    const authHelper = new E2ETestAuthHelper(page);
    
    // Create multiple users to ensure uniqueness
    const user1 = await authHelper.createTestUser();
    const user2 = await authHelper.createTestUser();
    
    // Should have different usernames and IDs
    expect(user1.credentials.username).not.toBe(user2.credentials.username);
    expect(user1.userResponse.id).not.toBe(user2.userResponse.id);
    
    console.log(`Created unique users: ${user1.credentials.username} and ${user2.credentials.username}`);
  });

  test('dynamic admin should have admin permissions', async ({ dynamicAdminPage }) => {
    // The dynamicAdminPage fixture should have created and logged in an admin user
    
    // Navigate to a page that requires admin access (users management)
    await dynamicAdminPage.goto('/users');
    
    // Should not be redirected to dashboard (would happen for non-admin users)
    await expect(dynamicAdminPage).toHaveURL(/.*\/users.*/);
    
    // Should see admin-only content
    await expect(dynamicAdminPage.locator('h1, h2, h3, h4, h5, h6')).toContainText(['Users', 'User Management'], { timeout: 10000 });
    
    console.log('✅ Dynamic admin user has admin permissions');
  });

  test('dynamic user should have user permissions', async ({ dynamicUserPage }) => {
    // The dynamicUserPage fixture should have created and logged in a regular user
    
    // Try to navigate to an admin-only page
    await dynamicUserPage.goto('/users');
    
    // Should be redirected to dashboard or get access denied
    await dynamicUserPage.waitForLoadState('networkidle');
    
    // Should either be redirected to dashboard or see access denied
    const currentUrl = dynamicUserPage.url();
    const isDashboard = currentUrl.includes('/dashboard');
    const isAccessDenied = await dynamicUserPage.locator(':has-text("Access denied"), :has-text("Unauthorized"), :has-text("403")').isVisible().catch(() => false);
    
    expect(isDashboard || isAccessDenied).toBe(true);
    
    console.log('✅ Dynamic user has restricted permissions');
  });
});