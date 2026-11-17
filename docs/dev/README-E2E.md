# Readur E2E Testing Guide

End-to-end tests drive the real UI in a browser with Playwright. They live in `frontend/e2e/` and are configured in `frontend/playwright.config.ts`.

## Prerequisites

- Node.js and npm (CI uses Node 26)
- Rust and Cargo, to build the backend
- PostgreSQL. The migrations need only `pg_trgm` from the standard contrib package, no vector or other third-party extension. The wrapper below creates its scratch database on `localhost:5432` as `postgres`/`postgres` unless E2E_DB_HOST, E2E_DB_PORT, E2E_DB_USER and E2E_DB_PASSWORD say otherwise (E2E_DB_NAME names the database, default `readur_e2e_test`; it is dropped and recreated).
- `psql`, `createdb` and `dropdb` (the PostgreSQL client tools) and `curl`, for the wrapper. It starts the backend and the dev server in their own process groups with `setsid` where available (Linux) and with `perl` otherwise (macOS).
- Tesseract with the languages the OCR tests use (CI installs eng, spa, fra, deu, ita and por), plus ocrmypdf and poppler-utils

Install the frontend dependencies (the wrapper does this itself when `frontend/node_modules` is missing):

```bash
cd frontend
npm ci
```

## Quick start

From the repository root:

```bash
scripts/run-e2e-local.sh
```

It creates the scratch database, builds the backend (release profile) and starts it on `127.0.0.1:8001` with a known admin password (the server runs its migrations on startup), starts the Vite dev server on port 5174 proxying `/api` (HTTP and WebSocket) to that backend, checks the proxy reaches it, installs Playwright's Chromium, and runs the two Chromium projects as CI does. On exit it stops both servers, drops the database and deletes its upload directory; the logs stay in `frontend/test-results/e2e-local/`. It refuses to start when port 8001 or 5174 is already taken.

### Script options

```bash
scripts/run-e2e-local.sh --headed     # show the browser
scripts/run-e2e-local.sh --debug      # Playwright inspector
scripts/run-e2e-local.sh --ui         # Playwright UI mode
scripts/run-e2e-local.sh -- e2e/ocr-retry-workflow.spec.ts --project=chromium --workers=1
```

Arguments after `--` go to Playwright unchanged, replacing the default project selection. Settings come from the environment, all optional:

| Variable | Default | Meaning |
|----------|---------|---------|
| E2E_DB_HOST, E2E_DB_PORT | localhost, 5432 | PostgreSQL server for the scratch database |
| E2E_DB_USER, E2E_DB_PASSWORD | postgres, postgres | PostgreSQL account that creates and drops it |
| E2E_DB_NAME | readur_e2e_test | Scratch database (dropped and recreated) |
| E2E_BACKEND_PORT | 8001 | Backend port |
| E2E_FRONTEND_PORT | 5174 | Vite dev server port |
| E2E_CARGO_PROFILE | release | Cargo profile of the backend build (`dev` reuses a debug build) |
| E2E_ADMIN_USERNAME, E2E_ADMIN_PASSWORD | admin, readur-e2e-admin-password | Admin the backend seeds and the suite creates its test users through |
| CARGO | cargo | Cargo command (may include a wrapper) |
| CARGO_TARGET_DIR | target | Where the built binary is looked for |
| PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH | (unset) | Use a system Chromium instead of downloading one |

For example, against a PostgreSQL on another port with a system Chromium (NixOS needs a writable PLAYWRIGHT_BROWSERS_PATH as well, for the ffmpeg Playwright records videos with):

```bash
env E2E_DB_HOST=127.0.0.1 E2E_DB_PORT=55432 \
    PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH="$(command -v chromium)" PLAYWRIGHT_BROWSERS_PATH=/tmp/pw-browsers \
    scripts/run-e2e-local.sh -- e2e/search.spec.ts --project=chromium
```

On NixOS the backend binary also needs the OpenSSL, Leptonica and Tesseract libraries on the dynamic loader's search path: run the wrapper inside `nix-shell -p pkg-config leptonica tesseract openssl postgresql` with the loader path pointing at their `lib` directories.

`./run-tests.sh e2e` is the Docker variant: it starts the containers of `docker-compose.test.yml` (Readur on port 8001 and a Vite dev server container on port 5174 that proxies to it) and runs Playwright on the host against the dev server. It seeds the Readur container and runs the suite with the same admin password, `ADMIN_PASSWORD` from your shell or `readur-test-admin-password`, the compose file's default.

## How the tests reach the server

- With `PLAYWRIGHT_BASE_URL` set, the tests run against that URL and Playwright starts nothing.
- Without it (and outside CI), Playwright starts the Vite dev server (`npm run dev`) on `http://localhost:5173`, or reuses one that is already running. The dev server proxies `/api`, WebSockets included, to `http://localhost:8000`, so a backend must be running there. (The proxy target can be changed with the `BACKEND_PORT` or `VITE_API_PROXY_TARGET` variables of `frontend/vite.config.ts`.)

The backend only accepts a WebSocket whose `Origin` matches the host the browser addressed. Behind the dev server that host arrives in the `X-Forwarded-Host` header the proxy sets, and the backend honours it only from a peer listed in `TRUSTED_PROXIES` (its default trusts loopback and private networks). If you set `TRUSTED_PROXIES` yourself, include `127.0.0.1` and `::1`, or list the dev server's origin in `CORS_ALLOWED_ORIGINS`; otherwise WebSocket specs fail with the sync panel stuck on "Reconnecting". `scripts/run-e2e-local.sh` starts its backend with `TRUSTED_PROXIES="127.0.0.1,::1"`.

The suite provisions its own test users through the API, logged in as the admin account the server creates at startup. It needs that admin's password: set E2E_ADMIN_PASSWORD (or `ADMIN_PASSWORD`) in the test environment to the `ADMIN_PASSWORD` the server was started with. The username defaults to `admin` (override with E2E_ADMIN_USERNAME). See `frontend/e2e/utils/test-auth-helper.ts`.

## Manual setup

To keep a backend running between runs, start the pieces yourself. Ports 8001 and 5174 keep clear of a development server on 8000 and 5173.

1. Create a database and start the backend with a known admin password. Migrations run automatically at startup.

   ```bash
   createdb readur_e2e
   DATABASE_URL=postgresql://postgres:postgres@localhost:5432/readur_e2e \
   SERVER_ADDRESS=127.0.0.1:8001 \
   ADMIN_PASSWORD=e2e-admin-password \
   cargo run --release --bin readur
   ```

2. In a second terminal, start the dev server proxying to that backend:

   ```bash
   cd frontend
   VITE_API_PROXY_TARGET=http://127.0.0.1:8001 npm run dev -- --port 5174
   ```

3. In a third, run the tests against the dev server:

   ```bash
   cd frontend
   env PLAYWRIGHT_BASE_URL=http://localhost:5174 E2E_ADMIN_PASSWORD=e2e-admin-password npm run test:e2e -- --project=chromium
   ```

Alternatively, build the frontend (`npm run build` in `frontend/`): the backend serves `frontend/dist` itself, and the tests can run straight against it with `PLAYWRIGHT_BASE_URL` set to the backend's URL. This is what CI does.

### npm scripts

| Command | What it does |
|---------|--------------|
| `npm run test:e2e` | Run all projects headless |
| `npm run test:e2e:headed` | Show the browser |
| `npm run test:e2e:debug` | Playwright inspector, step through tests |
| `npm run test:e2e:ui` | Playwright UI mode |

Extra Playwright arguments go after `--`, for example `npm run test:e2e -- --project=chromium e2e/search.spec.ts`.

## Configuration (`frontend/playwright.config.ts`)

- Test directory `./e2e`, 60 s timeout per test, 10 s for `expect`.
- Projects: `chromium` (all specs except `websocket-*.spec.ts`), `chromium-websocket` (only `websocket-*.spec.ts`, one worker), `firefox` and `webkit` (kept for local runs; CI runs only the two Chromium projects).
- Not fully parallel; 2 workers locally, 1 in CI. One retry locally, two in CI. `test.only` fails the run in CI.
- Reports: HTML in `test-results/e2e-report`, JSON in `test-results/e2e-results.json`, and the list reporter. Traces on the first retry, screenshots on failure, videos kept on failure, all under `test-results/e2e-artifacts`.
- `PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH` points the Chromium projects at a system browser, for hosts where the downloaded one cannot run (NixOS, for example).
- `CI` set: no dev server is started, retries and workers change as above.

## CI

`.github/workflows/test-e2e.yml` runs on pushes and pull requests to `main` and `master`. It starts PostgreSQL 18 as a service, installs Tesseract and the other OCR tools, builds the backend (`cargo build --release`) and the frontend (`npm run build`), and starts `./target/release/readur` on port 8000 with a generated `JWT_SECRET` and a fixed admin password; the backend serves the built frontend itself. It then waits up to a minute for `GET /api/health`, failing the job with the tail of the server log if the backend exits or never answers, installs Chromium, and runs:

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
