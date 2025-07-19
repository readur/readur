# Test Infrastructure Documentation

This document provides a comprehensive guide to the test infrastructure in Readur, including test patterns, utilities, common issues, and best practices.

## 📋 Table of Contents

- [Test Architecture Overview](#test-architecture-overview)
- [TestContext Pattern](#testcontext-pattern)
- [Test Utilities](#test-utilities)
- [Test Isolation and Environment Variables](#test-isolation-and-environment-variables)
- [Common Patterns](#common-patterns)
- [Troubleshooting](#troubleshooting)
- [Best Practices](#best-practices)

## Test Architecture Overview

Readur uses a three-tier testing approach:

1. **Unit Tests** (`src/tests/`) - Fast, isolated component tests
2. **Integration Tests** (`tests/`) - Full system tests with database
3. **Frontend Tests** (`frontend/src/__tests__/`) - React component and API tests

### Test Execution Flow

```
┌─────────────────┐
│   Unit Tests    │ ← No external dependencies
│  (cargo test)   │ ← Milliseconds execution
└────────┬────────┘
         │
┌────────▼────────┐
│Integration Tests│ ← Real database (PostgreSQL)
│ (TestContext)   │ ← In-memory app instance
└────────┬────────┘
         │
┌────────▼────────┐
│ Frontend Tests  │ ← Mocked API responses
│   (Vitest)      │ ← Component isolation
└─────────────────┘
```

## TestContext Pattern

The `TestContext` is the cornerstone of integration testing in Readur. It provides an isolated test environment with a real database.

### Basic Usage

```rust
use readur::test_utils::{TestContext, TestAuthHelper};

#[tokio::test]
async fn test_document_workflow() {
    // Create a new test context with default configuration
    let ctx = TestContext::new().await;
    
    // Access the app router for making requests
    let app = ctx.app();
    
    // Access the application state
    let state = ctx.state();
    
    // Test runs with isolated database
}
```

### How TestContext Works

1. **Database Setup**: Spins up a PostgreSQL container using testcontainers
2. **Migrations**: Runs all SQLx migrations automatically
3. **App Instance**: Creates an in-memory Axum router with full API routes
4. **Isolation**: Each test gets its own database container

### Custom Configuration

```rust
use readur::test_utils::{TestContext, TestConfigBuilder};

#[tokio::test]
async fn test_with_custom_config() {
    let config = TestConfigBuilder::default()
        .with_concurrent_ocr_jobs(4)
        .with_upload_path("./test-uploads")
        .with_oidc_enabled(false);
    
    let ctx = TestContext::with_config(config).await;
}
```

### Making Requests

```rust
use axum::http::{Request, StatusCode};
use axum::body::Body;
use tower::ServiceExt;

// Direct request to the test app
let request = Request::builder()
    .method("GET")
    .uri("/api/health")
    .body(Body::empty())
    .unwrap();

let response = ctx.app().clone().oneshot(request).await.unwrap();
assert_eq!(response.status(), StatusCode::OK);
```

## Test Utilities

### TestAuthHelper

Handles user creation and authentication in tests:

```rust
let auth_helper = TestAuthHelper::new(ctx.app().clone());

// Create a regular user
let mut test_user = auth_helper.create_test_user().await;
// Generates unique username: testuser_<pid>_<thread>_<nanos>

// Create an admin user
let admin_user = auth_helper.create_admin_user().await;

// Login and get token
let token = test_user.login(&auth_helper).await.unwrap();

// Make authenticated request
let response = auth_helper.make_authenticated_request(
    "GET",
    "/api/documents",
    None,
    &token
).await;
```

### Document Helpers

Test data builders for consistent document creation:

```rust
use readur::test_utils::document_helpers::*;

// Basic test document
let doc = create_test_document(user_id);

// Document with specific hash
let doc = create_test_document_with_hash(
    user_id,
    "test.pdf",
    "abc123".to_string()
);

// Low confidence OCR document
let doc = create_low_confidence_document(user_id, 45.0);

// Document with OCR error
let doc = create_document_with_ocr_error(user_id);
```

### Test User Pattern

Each test creates unique users to avoid conflicts:

```rust
// Unique username pattern: testuser_<process_id>_<thread_id>_<timestamp_nanos>
// Example: testuser_12345_2_1752870966778668050

// This prevents "Username already exists" errors in parallel tests
```

## Test Isolation and Environment Variables

### The TESSDATA_PREFIX Problem

One of the most challenging issues in the test suite was related to OCR language validation and environment variables.

#### The Issue

1. Tests set `TESSDATA_PREFIX` environment variable to point to temporary directories
2. Environment variables are **global** and shared across all threads
3. When tests run in parallel, they overwrite each other's `TESSDATA_PREFIX`
4. This caused 400 errors when validating OCR languages

#### The Solution

Modified the OCR retry endpoint to use custom tessdata paths:

```rust
// In src/routes/documents/ocr.rs
let health_checker = if let Ok(tessdata_path) = std::env::var("TESSDATA_PREFIX") {
    crate::ocr::health::OcrHealthChecker::new_with_path(tessdata_path)
} else {
    crate::ocr::health::OcrHealthChecker::new()
};
```

#### Test Setup Example

```rust
#[tokio::test]
async fn test_retry_ocr_with_language() {
    // Create temporary directory for tessdata
    let temp_dir = TempDir::new().unwrap();
    let tessdata_path = temp_dir.path();
    
    // Create mock language files
    fs::write(tessdata_path.join("eng.traineddata"), "mock").unwrap();
    fs::write(tessdata_path.join("spa.traineddata"), "mock").unwrap();
    
    // Set environment variable (careful with parallel tests!)
    let tessdata_str = tessdata_path.to_string_lossy().to_string();
    std::env::set_var("TESSDATA_PREFIX", &tessdata_str);
    
    let ctx = TestContext::new().await;
    // ... rest of test
}
```

### Best Practices for Environment Variables

1. **Avoid Global State**: Prefer passing configuration through constructors
2. **Use TestContext**: It provides isolation for most test scenarios
3. **Serial Execution**: For tests that must modify environment variables:
   ```rust
   #[tokio::test]
   #[serial]  // Using serial_test crate
   async fn test_that_modifies_env() {
       // This test runs in isolation
   }
   ```

## Common Patterns

### Authentication Test Pattern

```rust
#[tokio::test]
async fn test_authenticated_endpoint() {
    let ctx = TestContext::new().await;
    let auth_helper = TestAuthHelper::new(ctx.app().clone());
    
    // Create and login user
    let mut user = auth_helper.create_test_user().await;
    let token = user.login(&auth_helper).await.unwrap();
    
    // Make authenticated request
    let request = Request::builder()
        .method("GET")
        .uri("/api/protected")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    
    let response = ctx.app().clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
```

### Document Upload Pattern

```rust
#[tokio::test]
async fn test_document_upload() {
    let ctx = TestContext::new().await;
    let auth_helper = TestAuthHelper::new(ctx.app().clone());
    let mut user = auth_helper.create_test_user().await;
    let token = user.login(&auth_helper).await.unwrap();
    
    // Create multipart form
    let form = multipart::Form::new()
        .text("tags", "test,document")
        .part("file", multipart::Part::bytes(b"test content")
            .file_name("test.txt")
            .mime_str("text/plain").unwrap());
    
    // Upload document
    let response = reqwest::Client::new()
        .post("http://localhost:8000/api/documents")
        .header("Authorization", format!("Bearer {}", token))
        .multipart(form)
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 201);
}
```

### Database Direct Access Pattern

```rust
#[tokio::test]
async fn test_database_operations() {
    let ctx = TestContext::new().await;
    let user_id = Uuid::new_v4();
    
    // Direct database access
    sqlx::query!(
        "INSERT INTO users (id, username, email, password_hash, role) 
         VALUES ($1, $2, $3, $4, $5)",
        user_id,
        "testuser",
        "test@example.com",
        "hash",
        "user"
    )
    .execute(&ctx.state().db.pool)
    .await
    .unwrap();
    
    // Verify through API
    // ...
}
```

## Troubleshooting

### Common Test Failures

#### 1. "Username already exists" Error

**Cause**: Parallel tests creating users with same username

**Solution**: TestAuthHelper now generates unique usernames with timestamps

```rust
// Automatic unique username generation
let username = format!("testuser_{}_{}_{}",
    std::process::id(),
    thread_id,
    timestamp_nanos
);
```

#### 2. "Server is not running" (Integration Tests)

**Cause**: Tests expecting external server on localhost:8000

**Solution**: Use TestContext instead of external HTTP requests

```rust
// ❌ Wrong - expects external server
let response = reqwest::get("http://localhost:8000/api/health").await;

// ✅ Correct - uses TestContext
let response = ctx.app().clone()
    .oneshot(Request::builder()
        .uri("/api/health")
        .body(Body::empty())
        .unwrap())
    .await
    .unwrap();
```

#### 3. OCR Language Validation Failures (400 errors)

**Cause**: TESSDATA_PREFIX environment variable conflicts

**Solution**: Use new_with_path() for custom tessdata directories

#### 4. Database Connection Errors

**Cause**: PostgreSQL container not ready or migrations failed

**Debug Steps**:
```bash
# Check if tests can connect to database
RUST_LOG=debug cargo test

# Run single test with output
cargo test test_name -- --nocapture

# Check Docker containers
docker ps
```

### Debugging Techniques

#### Enable Detailed Logging

```bash
# Full debug output
RUST_LOG=debug cargo test -- --nocapture

# Specific module logging
RUST_LOG=readur::routes=debug cargo test

# With backtrace
RUST_BACKTRACE=1 cargo test
```

#### Run Tests Serially

```bash
# Avoid parallel execution issues
cargo test -- --test-threads=1
```

#### Inspect Test Database

```rust
// Add debug queries in test
let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
    .fetch_one(&ctx.state().db.pool)
    .await
    .unwrap();
println!("User count: {}", count);
```

## Best Practices

### 1. Use Unique Identifiers

Always use timestamps or UUIDs for test data:

```rust
let unique_id = Uuid::new_v4();
let unique_email = format!("test_{}@example.com", unique_id);
```

### 2. Clean Test State

TestContext automatically provides isolated databases, but clean up external resources:

```rust
// TempDir automatically cleans up
let temp_dir = TempDir::new().unwrap();
// Directory deleted when temp_dir drops
```

### 3. Test Both Success and Failure Cases

```rust
#[tokio::test]
async fn test_endpoint_success() {
    // Happy path test
}

#[tokio::test]
async fn test_endpoint_unauthorized() {
    // No auth token - expect 401
}

#[tokio::test]
async fn test_endpoint_not_found() {
    // Invalid ID - expect 404
}
```

### 4. Use Type-Safe Assertions

```rust
// Parse response to proper types
let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
    .await
    .unwrap();
let document: DocumentResponse = serde_json::from_slice(&body_bytes).unwrap();

// Now assertions are type-safe
assert_eq!(document.filename, "test.pdf");
```

### 5. Document Test Purpose

```rust
#[tokio::test]
async fn test_ocr_retry_with_multiple_languages() {
    // Tests that OCR retry endpoint accepts multiple language codes
    // and validates them against available tessdata files.
    // This ensures multi-language OCR support works correctly.
}
```

### 6. Avoid External Dependencies

- Use TestContext instead of external servers
- Mock external services when possible
- Use in-memory databases for unit tests
- Create test fixtures instead of relying on external files

### 7. Handle Async Properly

```rust
// Use tokio::test for async tests
#[tokio::test]
async fn test_async_operation() {
    // Can use .await here
}

// For timeout handling
use tokio::time::{timeout, Duration};

let result = timeout(
    Duration::from_secs(30),
    long_running_operation()
).await;
```

## Test Organization

### Directory Structure

```
readur/
├── src/
│   └── tests/          # Unit tests
│       ├── mod.rs
│       ├── auth_tests.rs
│       ├── db_tests.rs
│       └── ...
├── tests/              # Integration tests
│   ├── integration_ocr_language_endpoints.rs
│   ├── integration_settings_tests.rs
│   └── ...
└── frontend/
    └── src/
        └── __tests__/  # Frontend tests
            ├── components/
            └── pages/
```

### Naming Conventions

- Unit tests: `test_<component>_<behavior>`
- Integration tests: `test_<workflow>_<scenario>`
- Test files: `integration_<feature>_tests.rs`

## Summary

The test infrastructure in Readur provides:

1. **Isolation**: Each test runs in its own environment
2. **Realism**: Integration tests use real databases and full app instances
3. **Speed**: Parallel execution with proper isolation
4. **Reliability**: Unique identifiers prevent conflicts
5. **Maintainability**: Clear patterns and utilities

Key takeaways:
- Always use TestContext for integration tests
- Generate unique test data to avoid conflicts
- Be careful with environment variables in parallel tests
- Use the provided test utilities for common operations
- Test both success and failure scenarios

For more examples, see the existing test files in `tests/` directory.