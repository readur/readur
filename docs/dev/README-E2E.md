# Readur E2E Testing Guide

End-to-end tests drive the real UI in a browser with Playwright. They live in `frontend/e2e/` and are configured in `frontend/playwright.config.ts`.

## Prerequisites

- Node.js and npm (CI uses Node 26)
- Rust and Cargo, to build the backend
- PostgreSQL
- Tesseract with the languages the OCR tests use (CI installs eng, spa, fra, deu, ita and por), plus ocrmypdf and poppler-utils

Install the frontend dependencies and the browsers:

```bash
cd frontend
npm install
npx playwright install
```

## How the tests reach the server

- With `PLAYWRIGHT_BASE_URL` set, the tests run against that URL and Playwright starts nothing.
- Without it (and outside CI), Playwright starts the Vite dev server (`npm run dev`) on `http://localhost:5173`, or reuses one that is already running. The dev server proxies `/api` to `http://localhost:8000`, so a backend must be running there. (The proxy target can be changed with the `BACKEND_PORT` or `VITE_API_PROXY_TARGET` variables of `frontend/vite.config.ts`.)

The suite provisions its own test users through the API, logged in as the admin account the server creates at startup. It needs that admin's password: set E2E_ADMIN_PASSWORD (or `ADMIN_PASSWORD`) in the test environment to the `ADMIN_PASSWORD` the server was started with. The username defaults to `admin` (override with E2E_ADMIN_USERNAME). See `frontend/e2e/utils/test-auth-helper.ts`.

## Running locally

1. Start PostgreSQL and the backend with a known admin password. Migrations run automatically at startup.

   ```bash
   createdb readur_e2e
   DATABASE_URL=postgresql://postgres:postgres@localhost:5432/readur_e2e \
   ADMIN_PASSWORD=e2e-admin-password \
   cargo run --release --bin readur
   ```

2. In another terminal, run the tests (Playwright starts the dev server on port 5173):

   ```bash
   cd frontend
   env E2E_ADMIN_PASSWORD=e2e-admin-password npm run test:e2e
   ```

Alternatively, build the frontend (`npm run build` in `frontend/`): the backend serves `frontend/dist` itself, and the tests can run straight against it with `PLAYWRIGHT_BASE_URL` set to `http://localhost:8000`. This is what CI does.

### Scripts

| Command | What it does |
|---------|--------------|
| `npm run test:e2e` | Run all projects headless |
| `npm run test:e2e:headed` | Show the browser |
| `npm run test:e2e:debug` | Playwright inspector, step through tests |
| `npm run test:e2e:ui` | Playwright UI mode |

Extra Playwright arguments go after `--`, for example `npm run test:e2e -- --project=chromium e2e/search.spec.ts`.

`scripts/run-e2e-local.sh` and `./run-tests.sh e2e` are older wrappers. They do not currently work as written: `run-e2e-local.sh` calls a `migrate` binary that does not exist and polls `/health` instead of `/api/health`, and both start a dev server that relies on VITE_API_BASE_URL, which `vite.config.ts` does not read. Use the steps above.

## Configuration (`frontend/playwright.config.ts`)

- Test directory `./e2e`, 60 s timeout per test, 10 s for `expect`.
- Projects: `chromium` (all specs except `websocket-*.spec.ts`), `chromium-websocket` (only `websocket-*.spec.ts`, one worker), `firefox` and `webkit` (kept for local runs; CI runs only the two Chromium projects).
- Not fully parallel; 2 workers locally, 1 in CI. One retry locally, two in CI. `test.only` fails the run in CI.
- Reports: HTML in `test-results/e2e-report`, JSON in `test-results/e2e-results.json`, and the list reporter. Traces on the first retry, screenshots on failure, videos kept on failure, all under `test-results/e2e-artifacts`.
- `PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH` points the Chromium projects at a system browser, for hosts where the downloaded one cannot run (NixOS, for example).
- `CI` set: no dev server is started, retries and workers change as above.

## CI

`.github/workflows/test-e2e.yml` runs on pushes and pull requests to `main` and `master`. It starts PostgreSQL 18 as a service, installs Tesseract and the other OCR tools, builds the backend (`cargo build --release`) and the frontend (`npm run build`), starts `./target/release/readur` with a generated `JWT_SECRET` and a fixed admin password, waits for `GET /api/health`, installs Chromium, and runs:

```bash
npm run test:e2e -- --project=chromium --project=chromium-websocket
```

with `PLAYWRIGHT_BASE_URL` set to `http://localhost:8000`. Test results are uploaded as artifacts.

## Test layout

Specs in `frontend/e2e/`:

`auth.spec.ts`, `auth-system.spec.ts`, `command-palette.spec.ts`, `document-management.spec.ts`, `home.spec.ts`, `library.spec.ts`, `mobile-nav.spec.ts`, `navigation.spec.ts`, `no-horizontal-overflow.spec.ts`, `ocr-multiple-languages.spec.ts`, `ocr-retry-workflow.spec.ts`, `per-user-watch-feature-flag.spec.ts`, `redirects.spec.ts`, `search.spec.ts`, `settings.spec.ts`, `slideover-layering.spec.ts`, `sources.spec.ts`, `theme.spec.ts`, `toast-placement.spec.ts`, `upload.spec.ts`, `webdav-workflow.spec.ts`, `websocket-sync-progress.spec.ts`.

Helpers:

- `e2e/fixtures/auth.ts`: a Playwright `test` extended with `testUser` and `testAdmin` (users created through the API) and `dynamicUserPage` / `dynamicAdminPage` (a page already logged in as that user).
- `e2e/utils/test-auth-helper.ts`: `E2ETestAuthHelper`, which creates and logs in test users through the seeded admin. Test user passwords default to `testpass123` / `adminpass123` (override with E2E_USER_PASSWORD / E2E_ADMIN_TEST_PASSWORD).
- `e2e/utils/test-helpers.ts`: `TestHelpers`, with API helpers (`uploadDocumentViaAPI`, `waitForOCRComplete`, `deleteDocumentViaAPI`, `createWebdavSourceViaAPI`, ...) and UI helpers (`navigateToPage`, `waitForApiCall`, `waitForToast`, `waitForLoadingToComplete`, ...).
- `e2e/utils/test-data.ts`: test file paths, API paths and `TIMEOUTS` (`short` 5 s, `medium` 10 s, `long` 30 s, `upload` 60 s, `ocr` 120 s).

Sample files are in `frontend/test_data/` (for example `hello_ocr.png`, `multiline.png`, `numbers.png`, `sample.txt`, `test_file.docx`, and `multilingual/`).

## Writing a test

```typescript
import { test, expect } from './fixtures/auth';
import { TestHelpers } from './utils/test-helpers';

test.describe('Sources', () => {
  test('opens the Sources page', async ({ dynamicUserPage: page }) => {
    const helpers = new TestHelpers(page);
    await helpers.openIntake('connections');
    await helpers.waitForLoadingToComplete();
    await expect(page.getByRole('heading', { level: 1, name: 'Sources' })).toBeVisible();
  });
});
```

Guidelines:

- Prefer role- and label-based locators; add `data-testid` only where there is no accessible name.
- Create the data a test needs through the API helpers and delete it afterwards, so tests do not depend on each other.
- Wait for the API call or UI state you need (`waitForApiCall`, `waitForOCRComplete`) instead of fixed sleeps.
- Name WebSocket tests `websocket-*.spec.ts` so they run in the serial project.

## Debugging

- `npm run test:e2e:debug` or `npm run test:e2e:ui`, or add `await page.pause()` to a test and run headed.
- After a failure, open the HTML report: `npx playwright show-report test-results/e2e-report`.
- Traces of retried tests open with `npx playwright show-trace <path to trace.zip>`.
- In CI, download the uploaded artifacts from the workflow run.
- If test users cannot be created, check that the admin password given to the tests matches the server's `ADMIN_PASSWORD`; if the server was started without one, it generated a random password (written to `<UPLOAD_PATH>/.readur/initial-admin-password`).
