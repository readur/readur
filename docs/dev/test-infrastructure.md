# Test Infrastructure Documentation

The shared helpers Rust tests use: per-test databases, `TestContext`, `TestAuthHelper` and the document builders. How to run the suites is in the [Testing Guide](TESTING.md).

## Table of Contents

- [Test Architecture Overview](#test-architecture-overview)
- [TestContext Pattern](#testcontext-pattern)
- [Test Utilities](#test-utilities)
- [Test Isolation and Environment Variables](#test-isolation-and-environment-variables)
- [Common Patterns](#common-patterns)
- [Troubleshooting](#troubleshooting)
- [Best Practices](#best-practices)

## Test Architecture Overview

| Module | Contents |
|--------|----------|
| `src/test_db.rs` | `TestDatabase`: a fresh PostgreSQL database per test, cloned from a migrated template |
| `src/test_utils.rs` | `TestContext`, `TestAuthHelper`, `TestUser`, `document_helpers`, test image helpers, live-server helpers |
| `src/test_utils_config.rs` | `TestConfigBuilder`, the configuration a `TestContext` runs with |
| `src/test_utils_mcp.rs` | MCP helpers: JSON-RPC requests to `/api/mcp`, MCP config switches |
| `src/test_helpers.rs` | Lower-level builders: `create_test_config`, `create_test_app_state`, `create_test_queue_service`, `TestAppStateOptions` |

These modules are compiled under `#[cfg(any(test, feature = "test-utils"))]`. The `readur` dev-dependency in `Cargo.toml` enables `test-utils`, so integration tests in `tests/` can use them without extra flags.

## TestContext Pattern

`TestContext` builds the full application in process: a new database, an `AppState`, and the Axum router with every API route. Requests go straight to the router, with no network and no separate server.

```rust
use readur::test_utils::TestContext;

#[tokio::test]
async fn health_is_ok() {
    let ctx = TestContext::new().await;

    let request = axum::http::Request::builder()
        .uri("/api/health")
        .body(axum::body::Body::empty())
        .unwrap();
    let response = tower::ServiceExt::oneshot(ctx.app().clone(), request).await.unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
}
```

### How TestContext Works

1. **Database**: `TestDatabase::from_template()` creates a database with all migrations applied (see [Per-test databases](#per-test-databases)).
2. **Pool**: a connection pool of 5 connections (15 with `TEST_REQUIRES_LARGER_POOL` set).
3. **Upload directory**: unless `with_upload_path` is given, a fresh directory outside the repository, removed when the context is dropped.
4. **App**: an `AppState` and the router, available as `ctx.app()` (also the public field `ctx.app`) and `ctx.state()`.

Databases are dropped when the test process exits. `ctx.cleanup_and_close().await` closes the pool earlier, which helps tests that open many contexts.

### Custom Configuration

```rust
use readur::test_utils::{TestConfigBuilder, TestContext};

#[tokio::test]
async fn small_upload_limit() {
    let config = TestConfigBuilder::default()
        .with_max_file_size_mb(1)
        .with_concurrent_ocr_jobs(1);
    let ctx = TestContext::with_config(config).await;
    // ...
}
```

`TestConfigBuilder` methods: `with_upload_path`, `with_watch_folder`, `with_concurrent_ocr_jobs`, `with_oidc_enabled`, `with_max_file_size_mb`, `with_registration_requires_approval`, `with_mcp_enabled`, `with_mcp_allow_delete`.

## Test Utilities

### TestAuthHelper

Creates users and sends authenticated requests to the in-process app. Get one bound to the context's app and database with `ctx.auth_helper()` (`TestAuthHelper::new(app)` works too, but cannot create admins).

```rust
let auth_helper = ctx.auth_helper();

// Regular user, registered through POST /api/auth/register.
// Username: testuser_<pid>_<thread>_<nanos>, password: password123
let mut user = auth_helper.create_test_user().await;

// Admin user, created directly in the database (needs ctx.auth_helper()).
// Username: adminuser_<pid>_<uuid>, password: adminpass123
let admin = auth_helper.create_admin_user().await;

// Log in: either through the user, which caches the token...
let token = user.login(&auth_helper).await.unwrap().to_string();
// ...or directly
let admin_token = auth_helper.login_user(&admin.username, &admin.password).await;

// Request with a bearer token; returns the response body as bytes
let body = auth_helper
    .make_authenticated_request("GET", "/api/documents", None, &token)
    .await;
```

`TestUser` has the fields `user_response`, `username`, `password` and `token`, and the methods `id()` and `is_admin()`.

### Document Helpers

`readur::test_utils::document_helpers` builds `Document` values for database-level tests:

```rust
use readur::test_utils::document_helpers::*;

let doc = create_test_document(user_id);
let doc = create_test_document_with_hash(user_id, "test.pdf", "abc123".to_string());
let doc = create_low_confidence_document(user_id, 45.0);
let doc = create_document_without_ocr(user_id);
let doc = create_document_with_ocr_error(user_id);
```

### Test images

`get_test_images()`, `get_test_image(n)`, `load_test_image(filename)` and `test_images_available()` give access to the images in `tests/test_images/` together with the text OCR should find in them.

### Live-server helpers

For the tests that talk to a separately running server over HTTP: `live_server_database_url()` (the `TEST_DATABASE_URL` or `DATABASE_URL` value, default `postgresql://readur:readur@localhost:5432/readur`), `create_live_server_user(...)` to create an active account directly in that database, and `test_password()` for a random password. Public registration cannot be used for this because self-registered accounts start inactive.

## Test Isolation and Environment Variables

### Per-test databases

Each test process uses one PostgreSQL server: a `postgres:18` testcontainer started on first use, or the server `READUR_TEST_POSTGRES_URL` points to (any database there whose role may `CREATEDB`). The migrations run once into a template database named after a hash of the migrations; each test then gets `readur_test_<pid>_<n>`, created with `CREATE DATABASE … TEMPLATE …`. `TestDatabase::empty()` gives a database without migrations, for migration tests.

At process exit the container is removed or, on an external server, the per-test databases are dropped. The template stays on an external server and is reused while the migrations are unchanged.

### Environment variables

Environment variables are shared by every test thread in a process, so a test that sets one can affect tests running in parallel. Prefer passing settings through `TestConfigBuilder` or `TestAppStateOptions` (in `src/test_helpers.rs`). If a test must change the process environment, run that test binary with `-- --test-threads=1`.

## Common Patterns

### Authenticated request

```rust
#[tokio::test]
async fn me_returns_the_user() {
    let ctx = TestContext::new().await;
    let auth_helper = ctx.auth_helper();
    let user = auth_helper.create_test_user().await;
    let token = auth_helper.login_user(&user.username, &user.password).await;

    let body = auth_helper
        .make_authenticated_request("GET", "/api/auth/me", None, &token)
        .await;
    let me: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(me["username"], user.username);
}
```

### Direct database access

```rust
let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
    .fetch_one(ctx.state().db.get_pool())
    .await
    .unwrap();
```

### Checking the status code

`make_authenticated_request` returns only the body. To check the status, build the request yourself and send it with `oneshot` as in the [TestContext example](#testcontext-pattern), adding an `Authorization: Bearer <token>` header.

## Troubleshooting

| Symptom | Cause and fix |
|---------|---------------|
| Testcontainer or `Failed to connect to test database` errors | Docker is not running or not reachable. Start it, or set `READUR_TEST_POSTGRES_URL` to an existing server |
| `permission denied to create database` | The role in `READUR_TEST_POSTGRES_URL` lacks `CREATEDB` |
| `TestAuthHelper::create_admin_user needs database access` | The helper was built with `TestAuthHelper::new`; use `ctx.auth_helper()` |
| Connection refused to `localhost:8000` | A live-server test without a running server; see the [Testing Guide](TESTING.md#integration-tests) |
| Pool timeouts in tests that open many connections | Set `TEST_REQUIRES_LARGER_POOL`, or call `ctx.cleanup_and_close()` when done |

Useful commands:

```bash
RUST_LOG=readur=debug cargo test --test integration_labels_tests -- --nocapture
cargo test --test integration_labels_tests -- --test-threads=1
RUST_BACKTRACE=1 cargo test --lib
```

## Best Practices

- Use `TestContext` for API behaviour; it needs only a database and no running server.
- Create the users and documents a test needs inside the test. Each `TestContext` has its own database, so tests do not see each other's data.
- Test the failure paths too: no token (401), another user's resource (404 or 403), invalid input (400).
- Parse response bodies into the real model types (`serde_json::from_slice::<DocumentResponse>(&body)`) so assertions break when the API changes.
- Keep temporary files in `tempfile::TempDir`, which deletes them on drop.

## Test Organization

```text
readur/
├── src/
│   ├── tests/            # cross-cutting unit tests
│   ├── test_db.rs        # per-test databases
│   ├── test_utils*.rs    # TestContext, TestAuthHelper, builders
│   └── test_helpers.rs   # config and AppState builders
├── tests/                # integration tests, one binary per file
│   ├── integration_*.rs
│   ├── unit_*.rs
│   ├── support/          # helpers for the documentation tests
│   ├── test_images/      # OCR test images
│   └── test_pdfs/
└── frontend/
    ├── src/**/__tests__/ # Vitest tests
    └── e2e/              # Playwright tests
```

File naming: `integration_<area>_tests.rs` for integration tests, `unit_<area>_tests.rs` for isolated tests in `tests/`.
