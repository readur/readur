import { test, expect } from './fixtures/auth';
import { E2ETestAuthHelper } from './utils/test-auth-helper';

test.describe('E2E Auth System', () => {
  test('should create and login dynamic test user', async ({ page, testUser }) => {
    // The testUser fixture should have created a user and logged them in via dynamicUserPage
    expect(testUser.credentials.username).toMatch(/^e2e_user_\d+_\d+_[a-z0-9]+$/);
    expect(testUser.userResponse.role).toBe('user');
    
  });

  test('should create and login dynamic admin user', async ({ page, testAdmin }) => {
    // The testAdmin fixture should have created an admin user
    expect(testAdmin.credentials.username).toMatch(/^e2e_admin_\d+_\d+_[a-z0-9]+$/);
    expect(testAdmin.userResponse.role).toBe('admin');
    
  });

  test('should login dynamic user via browser UI', async ({ page, testUser }) => {
    const authHelper = new E2ETestAuthHelper(page);
    
    // Ensure we're logged out first
    await authHelper.ensureLoggedOut();
    
    // Login with the dynamic user
    const loginSuccess = await authHelper.loginUser(testUser.credentials);
    expect(loginSuccess).toBe(true);
    
    // Verify we're on Home
    await expect(page).toHaveURL(/\/home/);
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
    
  });

  test('should login dynamic admin via browser UI', async ({ page, testAdmin }) => {
    const authHelper = new E2ETestAuthHelper(page);
    
    // Ensure we're logged out first
    await authHelper.ensureLoggedOut();
    
    // Login with the dynamic admin
    const loginSuccess = await authHelper.loginUser(testAdmin.credentials);
    expect(loginSuccess).toBe(true);
    
    // Verify we're on Home
    await expect(page).toHaveURL(/\/home/);
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
    
  });

  test('should support API login for dynamic users', async ({ page, testUser }) => {
    const authHelper = new E2ETestAuthHelper(page);
    
    // Login via API
    const token = await authHelper.loginUserAPI(testUser.credentials);
    expect(token).toBeTruthy();
    expect(typeof token).toBe('string');
    
  });

  test('should create unique users for each test', async ({ page }) => {
    const authHelper = new E2ETestAuthHelper(page);
    
    // Create multiple users to ensure uniqueness
    const user1 = await authHelper.createTestUser();
    const user2 = await authHelper.createTestUser();
    
    // Should have different usernames and IDs
    expect(user1.credentials.username).not.toBe(user2.credentials.username);
    expect(user1.userResponse.id).not.toBe(user2.userResponse.id);
    
  });

  test('dynamic admin should have admin permissions', async ({ dynamicAdminPage }) => {
    // The dynamicAdminPage fixture should have created and logged in an admin user
    
    // The legacy /debug URL lands on the admin-only Debug settings section
    await dynamicAdminPage.goto('/debug');
    await expect(dynamicAdminPage).toHaveURL(/\/settings\/debug/);
    await expect(dynamicAdminPage.getByRole('heading', { level: 2, name: 'Debug' })).toBeVisible({ timeout: 10000 });

    // Admin-only sections are listed in the settings nav
    const nav = dynamicAdminPage.getByRole('navigation', { name: 'Settings sections' });
    await expect(nav.getByRole('link', { name: 'Users' })).toBeVisible();
    await expect(nav.getByRole('link', { name: 'Debug' })).toBeVisible();

  });

  test('dynamic user should have user permissions', async ({ dynamicUserPage }) => {
    // The dynamicUserPage fixture should have created and logged in a regular user
    
    // Home works for every user (the legacy /dashboard URL redirects there)
    await dynamicUserPage.goto('/dashboard');
    await expect(dynamicUserPage).toHaveURL(/\/home/);
    await expect(dynamicUserPage.getByRole('heading', { level: 1 })).toBeVisible();

    // Admin-only settings sections are hidden from regular users
    await dynamicUserPage.goto('/settings');
    const nav = dynamicUserPage.getByRole('navigation', { name: 'Settings sections' });
    await expect(nav.getByRole('link', { name: 'General' })).toBeVisible();
    await expect(nav.getByRole('link', { name: 'Users' })).toHaveCount(0);
    await expect(nav.getByRole('link', { name: 'Debug' })).toHaveCount(0);

  });
});