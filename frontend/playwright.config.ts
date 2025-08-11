import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  timeout: 60 * 1000,
  expect: {
    timeout: 10000,
  },
  fullyParallel: false, // Disable full parallelism for WebSocket tests stability
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 1, // Allow one retry for flaky WebSocket tests
  workers: process.env.CI ? 1 : 2, // Limit concurrency to reduce WebSocket conflicts
  reporter: [
    ['html', { outputFolder: 'test-results/e2e-report' }],
    ['json', { outputFile: 'test-results/e2e-results.json' }],
    ['list']
  ],
  outputDir: 'test-results/e2e-artifacts',
  use: {
    baseURL: process.env.PLAYWRIGHT_BASE_URL || 'http://localhost:5173',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
      testMatch: '**/!(websocket-*)*.spec.ts', // Regular tests
    },
    {
      name: 'chromium-websocket',
      use: { ...devices['Desktop Chrome'] },
      testMatch: '**/websocket-*.spec.ts', // WebSocket tests run separately
      fullyParallel: false,
      workers: 1, // Force WebSocket tests to run serially
    },
    {
      name: 'firefox',
      use: { ...devices['Desktop Firefox'] },
      testMatch: '**/!(websocket-*)*.spec.ts',
    },
    {
      name: 'webkit',
      use: { ...devices['Desktop Safari'] },
      testMatch: '**/!(websocket-*)*.spec.ts',
    },
  ],
  webServer: process.env.CI ? undefined : {
    command: 'npm run dev',
    url: 'http://localhost:5173',
    reuseExistingServer: !process.env.CI,
    timeout: 120 * 1000,
  },
});