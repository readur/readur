# Readur E2E Testing Guide

This guide covers the end-to-end (E2E) testing setup for Readur using Playwright.

## Overview

The E2E test suite covers:
- User authentication flows
- Document upload and processing
- Search functionality
- Document management
- Complete user workflows

## Setup

### Prerequisites

- Node.js 18+ and npm
- Rust and Cargo
- PostgreSQL
- Git

### Installation

1. **Install Playwright dependencies**:
   ```bash
   cd frontend
   npm install
   npx playwright install
   ```

2. **Set up test database**:
   ```bash
   # Create test database
   createdb readur_e2e_test
   
   # Add vector extension (if available)
   psql -d readur_e2e_test -c "CREATE EXTENSION IF NOT EXISTS vector;"
   ```

## Running Tests

### Local Development

#### Quick Start
Use the provided script for automated setup:
```bash
./scripts/run-e2e-local.sh
```

#### Manual Setup
If you prefer manual control:

1. **Start backend server**:
   ```bash
   DATABASE_URL="postgresql://postgres:postgres@localhost:5432/readur_e2e_test" \
   TEST_MODE=true \
   ROCKET_PORT=8001 \
   cargo run --release
   ```

2. **Start frontend dev server**:
   ```bash
   cd frontend
   VITE_API_BASE_URL="http://localhost:8001" \
   npm run dev -- --port 5174
   ```

3. **Run tests**:
   ```bash
   cd frontend
   npm run test:e2e
   ```

### Test Options

- **Headless mode** (default): `npm run test:e2e`
- **Headed mode** (show browser): `npm run test:e2e:headed`
- **Debug mode**: `npm run test:e2e:debug`
- **UI mode**: `npm run test:e2e:ui`

### Using the Local Script

The `run-e2e-local.sh` script provides additional options:

```bash
# Run tests normally
./scripts/run-e2e-local.sh

# Run in headed mode
./scripts/run-e2e-local.sh --headed

# Run in debug mode
./scripts/run-e2e-local.sh --debug

# Run with Playwright UI
./scripts/run-e2e-local.sh --ui

# Show help
./scripts/run-e2e-local.sh --help
```

## GitHub Actions

The E2E tests automatically run in GitHub Actions on:
- Push to `master`/`main` branch
- Pull requests to `master`/`main` branch

The workflow:
1. Sets up PostgreSQL database
2. Builds and starts the backend server
3. Starts the frontend dev server
4. Runs all E2E tests
5. Uploads test reports and artifacts

## Test Structure

### Test Files

- `e2e/auth.spec.ts` - Authentication flows
- `e2e/upload.spec.ts` - Document upload functionality
- `e2e/search.spec.ts` - Search workflows
- `e2e/document-management.spec.ts` - Document management

### Utilities

- `e2e/fixtures/auth.ts` - Authentication fixture for logged-in tests
- `e2e/utils/test-helpers.ts` - Common helper functions
- `e2e/utils/test-data.ts` - Test data and configuration

### Configuration

- `playwright.config.ts` - Playwright configuration
- `.github/workflows/e2e-tests.yml` - GitHub Actions workflow

## Test Data

Tests use sample files from:
- `frontend/test_data/hello_ocr.png` - Sample image for OCR
- `frontend/test_data/multiline.png` - Multi-line text image
- `frontend/test_data/numbers.png` - Numbers image

Add additional test files to `frontend/test_data/` as needed.

## Writing Tests

### Basic Test Structure

```typescript
import { test, expect } from '@playwright/test';
import { TestHelpers } from './utils/test-helpers';

test.describe('Feature Name', () => {
  let helpers: TestHelpers;

  test.beforeEach(async ({ page }) => {
    helpers = new TestHelpers(page);
    await helpers.navigateToPage('/your-page');
  });

  test('should do something', async ({ page }) => {
    // Your test logic here
    await expect(page.locator('[data-testid="element"]')).toBeVisible();
  });
});
```

### Using Authentication Fixture

For tests requiring authentication:

```typescript
import { test, expect } from './fixtures/auth';

test.describe('Authenticated Feature', () => {
  test('should work when logged in', async ({ authenticatedPage }) => {
    // Page is already authenticated
    await authenticatedPage.goto('/protected-page');
  });
});
```

### Best Practices

1. **Use data-testid attributes** for reliable element selection
2. **Wait for API calls** using `helpers.waitForApiCall()`
3. **Handle loading states** with `helpers.waitForLoadingToComplete()`
4. **Use meaningful test descriptions** that describe user actions
5. **Clean up test data** when necessary
6. **Use timeouts appropriately** from `TIMEOUTS` constants

## Debugging

### Local Debugging

1. **Run with --debug flag**:
   ```bash
   npm run test:e2e:debug
   ```

2. **Use Playwright UI**:
   ```bash
   npm run test:e2e:ui
   ```

3. **Add debugging code**:
   ```typescript
   await page.pause(); // Pauses execution
   await page.screenshot({ path: 'debug.png' }); // Take screenshot
   ```

### CI Debugging

- Check uploaded test artifacts in GitHub Actions
- Review test reports in the workflow summary
- Examine screenshots and videos from failed tests

## Configuration

### Environment Variables

- `PLAYWRIGHT_BASE_URL` - Base URL for tests (default: http://localhost:5173)
- `CI` - Set to true in CI environment
- `TEST_MODE` - Set to true for backend test mode

### Timeouts

Configure timeouts in `utils/test-data.ts`:
- `TIMEOUTS.short` (5s) - Quick operations
- `TIMEOUTS.medium` (10s) - Normal operations  
- `TIMEOUTS.long` (30s) - Slow operations
- `TIMEOUTS.upload` (60s) - File uploads
- `TIMEOUTS.ocr` (120s) - OCR processing

## Troubleshooting

### Common Issues

1. **Tests timing out**:
   - Increase timeouts in configuration
   - Check if services are running properly
   - Verify database connectivity

2. **Authentication failures**:
   - Ensure test user exists in database
   - Check authentication fixture implementation
   - Verify API endpoints are correct

3. **File upload failures**:
   - Ensure test files exist in `test_data/`
   - Check file permissions
   - Verify upload API is working

4. **Database issues**:
   - Ensure PostgreSQL is running
   - Check database migrations
   - Verify test database exists

### Getting Help

1. Check logs in `backend.log` and `frontend.log`
2. Review Playwright documentation
3. Examine existing test implementations
4. Use browser dev tools in headed mode

## Contributing

When adding new features:

1. **Add E2E tests** for new user workflows
2. **Update test data** if needed
3. **Add data-testid attributes** to new UI elements
4. **Update this documentation** if test setup changes

Ensure tests:
- Are reliable and not flaky
- Test realistic user scenarios
- Have good error messages
- Clean up after themselves