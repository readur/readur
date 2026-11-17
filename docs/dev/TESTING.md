# Testing Guide

How Readur's tests are organised and how to run them. Browser tests are covered separately in the [E2E Testing Guide](README-E2E.md), and the shared test helpers in [Test Infrastructure](test-infrastructure.md).

## Overview

| Kind | Where | Run with |
|------|-------|----------|
| Rust unit tests | `#[cfg(test)]` modules next to the code, plus `src/tests/` | `cargo test --lib` |
| Rust integration tests | `tests/*.rs` (one test binary per file) | `cargo test --test '*'` |
| Frontend unit tests | `frontend/src/**/__tests__/*.test.ts(x)` | `npm run test:unit` |
| Frontend integration tests | `frontend/src/**/*.integration.test.tsx` | `npm run test:integration` |
| E2E tests | `frontend/e2e/*.spec.ts` | `npm run test:e2e` |

## Prerequisites

- A Rust toolchain. The default features include `ocr`, which links against Tesseract and Leptonica, so their development packages (and `clang`/`libclang` for bindgen) must be installed to build. CI installs `tesseract-ocr`, `libtesseract-dev`, `libleptonica-dev`, `pkg-config`, `libclang-dev` and `clang`, plus `ocrmypdf` and `poppler-utils` for the PDF tests.
- Docker, for tests that need PostgreSQL (a container is started automatically), or an existing PostgreSQL server (see [Test databases](#test-databases)).
- Node.js and npm for the frontend tests (CI uses Node 26).

## Rust tests

### Unit tests

```bash
cargo test --lib
```

Most unit tests sit in `#[cfg(test)] mod tests` blocks in the file they test. `src/tests/` holds a few cross-cutting modules: `config_tests`, `document_response_serialization_tests`, `layering_tests`, `ocr_tests`, `regression_tests` and `route_compilation_tests`. Filter by name as usual:

```bash
cargo test --lib priority          # every unit test whose path contains "priority"
cargo test --lib -- --nocapture    # show println! output
```

### Integration tests

Each file in `tests/` is its own test binary, named after the file:

```bash
cargo test --test '*'                              # all of them
cargo test --test integration_labels_tests         # one file
cargo test --test integration_labels_tests create  # matching tests in one file
```

The files follow a naming convention: `integration_*` for tests that exercise the app end to end, `unit_*` for isolated tests that need more setup than an inline module, `*migration*` for migration tests, and `webdav_*` for WebDAV tests. Supporting directories:

- `tests/support/`: shared helpers for the documentation tests (`api_routes.rs`, `api_reference_examples.rs`, `markdown.rs`).
- `tests/integration_labels/`: a submodule of the label tests.
- `tests/performance/`: WebDAV performance tests.
- `tests/fixtures/`, `tests/test_images/`, `tests/test_pdfs/`: test data.

`tests/webdav_stress_tests.rs` needs the `stress-testing` feature:

```bash
cargo test --test webdav_stress_tests --features stress-testing
```

The test helpers are compiled with the `test-utils` feature, which the `readur` dev-dependency in `Cargo.toml` enables for all tests, so no flag is needed for them.

Most integration tests build the app in process with `TestContext` (from `src/test_utils.rs`) and send requests to its router; they need only a database. A smaller group (for example `integration_integration_tests.rs`, `integration_ocr_retry_tests.rs`, `integration_prometheus_metrics_tests.rs`, `integration_mcp_live_server_tests.rs`) talks over HTTP to a separately running server: the API_URL variable (default `http://localhost:8000`) and `TEST_DATABASE_URL` (falling back to `DATABASE_URL`) must point at that server and its database. These fail if no server is running.

### Test databases

Tests that need PostgreSQL get their own database from `readur::test_db::TestDatabase` (used by `TestContext`). Each test process starts one server, runs the migrations once into a template database, and clones it per test with `CREATE DATABASE … TEMPLATE …`, which takes milliseconds. Tests of the migrations themselves use `TestDatabase::empty()`.

The server is a `postgres:18` testcontainer by default (`test_db::POSTGRES_TAG`). To use an existing server instead, for example when Docker cannot run containers, point `READUR_TEST_POSTGRES_URL` at any database on it whose role can `CREATEDB`:

```bash
export READUR_TEST_POSTGRES_URL=postgresql://readur:readur@localhost:5432/readur
```

The container, or on an existing server the per-test databases, are removed when the test process exits. The template is kept on an existing server and reused until the migrations change.

`TEST_REQUIRES_LARGER_POOL` (any value) gives each `TestContext` a pool of 15 connections instead of 5.

### What CI runs

- `.github/workflows/test-unit.yml`: `cargo test --lib --no-fail-fast`, then `cargo test --tests unit_tests --no-fail-fast` (every test whose name contains `unit_tests`), a check that the generated TypeScript bindings are current (`scripts/check-ts-bindings.sh`), and the frontend type checks and unit tests with coverage.
- `.github/workflows/test-integration.yml`: starts PostgreSQL and a Readur server, plus a WebDAV server for the WebDAV tests, then runs `cargo test --test '*' --features "test-utils stress-testing" --no-fail-fast -- --test-threads=1`. A separate job runs `npm run test:integration`.

## Frontend tests

```bash
cd frontend
npm install
npm run test:unit            # unit tests, once
npm run test:unit:watch      # unit tests, watch mode
npm run test:unit:coverage   # unit tests with coverage
npm run test:integration     # *.integration.test.tsx
npm run type-check           # tsc for the app
npm run type-check:test      # tsc for the tests
```

`npm test` runs Vitest with `vitest.config.ts`, which re-exports the unit config; add `--run` to run once instead of watching. To run one file: `npm run test:unit -- src/features/library/__tests__/LibraryPage.test.tsx` (any path or name filter works).

Configuration:

- `vitest.unit.config.ts`: jsdom environment, globals, setup file `src/test/setup.ts`; excludes `e2e/`, `*.e2e.test.*` and `*.integration.test.*`. Coverage uses the v8 provider over `src/**/*.{ts,tsx}` (excluding tests, `src/test/`, type declarations and `src/main.tsx`), reports text summary, JSON summary and HTML, and fails below the thresholds set in the file (statements 86, branches 80, functions 84, lines 88).
- `vitest.integration.config.ts`: same environment and setup, includes only `**/*.integration.test.{js,jsx,ts,tsx}`.

Tests live in `__tests__` directories next to the code they test, mostly under `frontend/src/features/` and `frontend/src/services/`.

## The Docker test runner

`./run-tests.sh` runs the suites against containers from `docker-compose.test.yml` (PostgreSQL on port 5433, Readur on port 8001), saving output under `test-results/`:

```bash
./run-tests.sh unit           # cargo test --lib against the test database
./run-tests.sh integration    # cargo test --test '*' against the containers
./run-tests.sh frontend       # Vitest in a node container
./run-tests.sh all
./run-tests.sh integration keep-running   # leave the containers up afterwards
```

The Readur container's admin password is `ADMIN_PASSWORD` from your shell (default `readur-test-admin-password`), while the script passes `ADMIN_PASSWORD` to the tests with a different default (`test-admin-password`). Export `ADMIN_PASSWORD` before running so both use the same value. The `e2e` mode does not currently work; see the [E2E Testing Guide](README-E2E.md).

The old `test_runner` binary has been removed; use the commands above.

## Debugging failures

```bash
RUST_BACKTRACE=1 cargo test --test integration_labels_tests -- --nocapture
cargo test --lib -- --test-threads=1      # run serially
RUST_LOG=readur=debug cargo test --test integration_labels_tests -- --nocapture
```

- `Failed to connect to test database` or testcontainer errors: Docker is not available. Start it, or set `READUR_TEST_POSTGRES_URL`.
- Live-server tests failing with connection errors: start a server and set API_URL and `TEST_DATABASE_URL` as described above.
- Build errors in `tesseract-sys` or `leptonica-sys`: install the Tesseract/Leptonica development packages and `libclang`.

## Adding tests

- Put unit tests in a `#[cfg(test)]` module next to the code.
- For API behaviour, add a file `tests/integration_<area>_tests.rs` that uses `TestContext` and its `TestAuthHelper` (see [Test Infrastructure](test-infrastructure.md)):

```rust
use readur::test_utils::TestContext;

#[tokio::test]
async fn lists_labels_for_a_new_user() {
    let ctx = TestContext::new().await;
    let auth_helper = ctx.auth_helper();
    let user = auth_helper.create_test_user().await;
    let token = auth_helper.login_user(&user.username, &user.password).await;

    let body = auth_helper
        .make_authenticated_request("GET", "/api/labels", None, &token)
        .await;
    let labels: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(labels.is_array());
}
```

- Frontend: add `*.test.tsx` in a `__tests__` directory next to the component; name it `*.integration.test.tsx` if it belongs to the integration suite.
