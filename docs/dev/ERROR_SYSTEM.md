# Error System Documentation

How Readur's API handlers report errors: the typed error enums in `src/errors/`, the JSON they produce, and the complete list of error codes.

## Overview

Most route handlers return `Result<_, SomeError>`, where `SomeError` is one of the typed enums below. Each enum maps every variant to an HTTP status, a message shown to the client, and a stable error code. Handlers that have no typed error yet still return a bare `StatusCode` or ad-hoc JSON.

| Type | Module | Used by |
|------|--------|---------|
| `ApiError` | `src/errors/mod.rs` | Generic errors; defined but not currently used by any route |
| `UserError` | `src/errors/user.rs` | `src/routes/users.rs` |
| `SourceError` | `src/errors/source.rs` | `src/routes/sources/`, source error tracking, WebDAV sync |
| `LabelError` | `src/errors/label.rs` | `src/routes/labels.rs`, MCP label tools |
| `SettingsError` | `src/errors/settings.rs` | `src/routes/settings.rs` |
| `SearchError` | `src/errors/search.rs` | `src/routes/search.rs`, `src/services/search.rs`, MCP search tool |
| `CommentError` | `src/errors/comment.rs` | `src/routes/comments.rs` |
| `ApiKeyError` | `src/errors/api_key.rs` | `src/routes/api_keys.rs` |
| `SharedLinkError` | `src/errors/shared_link.rs` | `src/routes/shared_links.rs` |
| `DocumentError` | `src/errors/document.rs` | `src/services/documents.rs`, `src/routes/documents/`, MCP document tools |

`OcrError` (`src/ocr/error.rs`) is separate: it is an internal error for the OCR engine. It has its own `error_code()` method (used in logs and tests), but it does not implement `AppError` and is never returned as an HTTP error body.

## The AppError trait

Every type except `DocumentError` implements `AppError` (`src/errors/mod.rs`):

```rust
pub trait AppError: std::error::Error + Send + Sync + 'static {
    fn status_code(&self) -> StatusCode;
    fn user_message(&self) -> String;
    fn error_code(&self) -> &'static str;
    fn error_category(&self) -> ErrorCategory;
    fn error_severity(&self) -> ErrorSeverity;
    fn suppression_key(&self) -> Option<String> { None }
    fn suggested_action(&self) -> Option<String> { None }
    fn to_managed_error(&self) -> ManagedError { /* built from the methods above */ }
}
```

The `impl_into_response!` macro gives each of these types an axum `IntoResponse` implementation. Converting an error into a response does two things:

1. It passes a `ManagedError` to the error manager (`src/monitoring/error_management.rs`) on a background task. The manager logs it at a level chosen by severity: `Critical` at error, `Important` at warn, `Minor` at info, `Expected` at debug, with the category and code as fields. If the error has a suppression key and that key has been seen more than 3 times in the last 5 minutes, the log line is dropped (logged at debug instead).
2. It returns the HTTP response.

Categories (`ErrorCategory`): `PdfProcessing`, `OcrProcessing`, `Database`, `Network`, `FileSystem`, `Auth`, `Config`. Severities (`ErrorSeverity`): `Critical`, `Important`, `Minor`, `Expected`.

## Response bodies

`AppError` types:

```json
{
  "error": "Username 'john_doe' already exists",
  "code": "USER_DUPLICATE_USERNAME",
  "status": 409
}
```

`error` is `user_message()`, which never contains internal details for server-side failures (for example `ApiError::InternalServerError` always says `An internal error occurred`).

`DocumentError` has its own `IntoResponse` with a different shape: the code is in `error_code`, and a timestamp and a random request ID are added:

```json
{
  "error": "Document not found",
  "status": 404,
  "error_code": "UPLOAD_NOT_FOUND",
  "timestamp": "2026-03-01T10:00:00+00:00",
  "request_id": "6f1c2a52-3d1e-4c8e-9a3b-0f8b2c6d7e41"
}
```

Its server-side variants take a `&'static str`, so a raw database or storage error cannot be formatted into the message; log the underlying error where it is caught.

## Using an error type in a handler

Each type has convenience constructors. From `src/routes/users.rs`:

```rust
validate_email(email).map_err(|_| UserError::invalid_email(email))?;
validate_password(password).map_err(UserError::invalid_password)?;
```

and mapping a unique-constraint violation by constraint name:

```rust
Some(USERS_USERNAME_UNIQUE) => UserError::duplicate_username(username),
Some(USERS_EMAIL_UNIQUE) => UserError::duplicate_email(email),
_ => UserError::internal_server_error(format!("Failed to save user: {}", e)),
```

The handler signature is `Result<Json<...>, UserError>`. When adding a new variant, add it to `status_code`, `user_message`, `error_code`, `error_category` and `error_severity`, and add the code to the list below.

## Error code conventions

- Codes are upper-case with underscores.
- Each type uses its own prefix: USER_, SOURCE_, LABEL_, SETTINGS_, SEARCH_, COMMENT_, API_KEY_, SHARED_LINK_. `ApiError` codes have no prefix. `DocumentError` codes start with UPLOAD_, except INVALID_QUERY.
- A code is part of the API contract: clients may match on it, so do not rename existing codes.

## Error code reference

The complete list, taken from the `error_code` and `status_code` implementations.

### ApiError (`src/errors/mod.rs`)

| Code | HTTP status |
|------|-------------|

### UserError (`src/errors/user.rs`)

| Code | HTTP status |
|------|-------------|
| `USER_NOT_FOUND` | 404 |
| `USER_NOT_FOUND_BY_ID` | 404 |
| `USER_DUPLICATE_USERNAME` | 409 |
| `USER_DUPLICATE_EMAIL` | 409 |
| `USER_INVALID_ROLE` | 400 |
| `USER_PERMISSION_DENIED` | 403 |
| `USER_INVALID_CREDENTIALS` | 401 |
| `USER_ACCOUNT_DISABLED` | 403 |
| `USER_INVALID_PASSWORD` | 400 |
| `USER_INVALID_USERNAME` | 400 |
| `USER_INVALID_EMAIL` | 400 |
| `USER_DELETE_RESTRICTED` | 403 |
| `USER_OIDC_AUTH_FAILED` | 401 |
| `USER_AUTH_PROVIDER_NOT_CONFIGURED` | 400 |
| `USER_TOKEN_EXPIRED` | 401 |
| `USER_INVALID_TOKEN` | 401 |
| `USER_SESSION_EXPIRED` | 401 |
| `USER_INTERNAL_SERVER_ERROR` | 500 |

### SourceError (`src/errors/source.rs`)

| Code | HTTP status |
|------|-------------|
| `SOURCE_NOT_FOUND` | 404 |
| `SOURCE_NOT_FOUND_BY_ID` | 404 |
| `SOURCE_DUPLICATE_NAME` | 409 |
| `SOURCE_INVALID_PATH` | 400 |
| `SOURCE_CONNECTION_FAILED` | 502 |
| `SOURCE_AUTH_FAILED` | 401 |
| `SOURCE_SYNC_IN_PROGRESS` | 409 |
| `SOURCE_CONFIG_INVALID` | 400 |
| `SOURCE_ACCESS_DENIED` | 403 |
| `SOURCE_DISABLED` | 403 |
| `SOURCE_INVALID_TYPE` | 400 |
| `SOURCE_NETWORK_TIMEOUT` | 504 |
| `SOURCE_CAPACITY_EXCEEDED` | 507 |
| `SOURCE_SERVER_ERROR` | 502 |
| `SOURCE_CERTIFICATE_ERROR` | 502 |
| `SOURCE_UNSUPPORTED_VERSION` | 501 |
| `SOURCE_RATE_LIMITED` | 429 |
| `SOURCE_FILE_NOT_FOUND` | 404 |
| `SOURCE_DIRECTORY_NOT_FOUND` | 404 |
| `SOURCE_VALIDATION_FAILED` | 400 |
| `SOURCE_SYNC_FAILED` | 500 |
| `SOURCE_DELETE_RESTRICTED` | 409 |

### LabelError (`src/errors/label.rs`)

| Code | HTTP status |
|------|-------------|
| `LABEL_NOT_FOUND` | 404 |
| `LABEL_NOT_FOUND_BY_ID` | 404 |
| `LABEL_DUPLICATE_NAME` | 409 |
| `LABEL_INVALID_COLOR` | 400 |
| `LABEL_INVALID_NAME` | 400 |
| `LABEL_IN_USE` | 409 |
| `LABEL_INVALID_ICON` | 400 |
| `LABEL_MAX_REACHED` | 409 |
| `LABEL_PERMISSION_DENIED` | 403 |
| `LABEL_COLOR_CONFLICT` | 400 |
| `LABEL_DESCRIPTION_TOO_LONG` | 400 |
| `LABEL_DELETE_RESTRICTED` | 409 |
| `LABEL_INVALID_ASSIGNMENT` | 400 |
| `LABEL_RESERVED_NAME` | 409 |

### SettingsError (`src/errors/settings.rs`)

| Code | HTTP status |
|------|-------------|
| `SETTINGS_NOT_FOUND` | 404 |
| `SETTINGS_NOT_FOUND_FOR_USER` | 404 |
| `SETTINGS_INVALID_LANGUAGE` | 400 |
| `SETTINGS_INVALID_VALUE` | 400 |
| `SETTINGS_READ_ONLY` | 403 |
| `SETTINGS_VALIDATION_FAILED` | 400 |
| `SETTINGS_INVALID_OCR_CONFIG` | 400 |
| `SETTINGS_INVALID_FILE_TYPE` | 400 |
| `SETTINGS_VALUE_OUT_OF_RANGE` | 400 |
| `SETTINGS_INVALID_CPU_PRIORITY` | 400 |
| `SETTINGS_MEMORY_LIMIT_TOO_LOW` | 400 |
| `SETTINGS_MEMORY_LIMIT_TOO_HIGH` | 400 |
| `SETTINGS_INVALID_TIMEOUT` | 400 |
| `SETTINGS_INVALID_DPI` | 400 |
| `SETTINGS_INVALID_CONFIDENCE` | 400 |
| `SETTINGS_INVALID_CHARACTER_LIST` | 400 |
| `SETTINGS_CONFLICTING` | 409 |
| `SETTINGS_PERMISSION_DENIED` | 403 |
| `SETTINGS_SYSTEM_RESET_DENIED` | 403 |
| `SETTINGS_INVALID_SEARCH_CONFIG` | 400 |

### SearchError (`src/errors/search.rs`)

| Code | HTTP status |
|------|-------------|
| `SEARCH_QUERY_TOO_SHORT` | 400 |
| `SEARCH_QUERY_TOO_LONG` | 400 |
| `SEARCH_INDEX_UNAVAILABLE` | 503 |
| `SEARCH_INVALID_SYNTAX` | 400 |
| `SEARCH_TOO_MANY_RESULTS` | 413 |
| `SEARCH_TIMEOUT` | 408 |
| `SEARCH_INVALID_MODE` | 400 |
| `SEARCH_INVALID_MIME_TYPE` | 400 |
| `SEARCH_INVALID_PAGINATION` | 400 |
| `SEARCH_BOOLEAN_SYNTAX_ERROR` | 400 |
| `SEARCH_INVALID_FUZZY_THRESHOLD` | 400 |
| `SEARCH_INDEX_REBUILDING` | 503 |
| `SEARCH_CANCELLED` | 408 |
| `SEARCH_NO_RESULTS` | 404 |
| `SEARCH_INVALID_SNIPPET_LENGTH` | 400 |
| `SEARCH_QUOTA_EXCEEDED` | 429 |
| `SEARCH_INVALID_TAG_FILTER` | 400 |
| `SEARCH_INDEX_CORRUPTION` | 500 |
| `SEARCH_PERMISSION_DENIED` | 403 |
| `SEARCH_DISABLED` | 503 |

### CommentError (`src/errors/comment.rs`)

| Code | HTTP status |
|------|-------------|
| `COMMENT_NOT_FOUND` | 404 |
| `COMMENT_DOCUMENT_NOT_FOUND` | 404 |
| `COMMENT_PERMISSION_DENIED` | 403 |
| `COMMENT_CONTENT_EMPTY` | 400 |
| `COMMENT_CONTENT_TOO_LONG` | 400 |
| `COMMENT_PARENT_NOT_FOUND` | 404 |
| `COMMENT_NESTING_TOO_DEEP` | 400 |
| `COMMENT_RATE_LIMITED` | 429 |
| `COMMENT_INTERNAL_ERROR` | 500 |

### ApiKeyError (`src/errors/api_key.rs`)

| Code | HTTP status |
|------|-------------|
| `API_KEY_NOT_FOUND` | 404 |
| `API_KEY_INVALID_REQUEST` | 400 |
| `API_KEY_MAX_REACHED` | 409 |
| `API_KEY_PERMISSION_DENIED` | 403 |
| `API_KEY_RATE_LIMITED` | 429 |
| `API_KEY_INTERNAL_ERROR` | 500 |

### SharedLinkError (`src/errors/shared_link.rs`)

| Code | HTTP status |
|------|-------------|
| `SHARED_LINK_NOT_FOUND` | 404 |
| `SHARED_LINK_EXPIRED` | 410 |
| `SHARED_LINK_REVOKED` | 410 |
| `SHARED_LINK_PASSWORD_REQUIRED` | 401 |
| `SHARED_LINK_INVALID_PASSWORD` | 403 |
| `SHARED_LINK_MAX_VIEWS` | 410 |
| `SHARED_LINK_PERMISSION_DENIED` | 403 |
| `SHARED_LINK_RATE_LIMITED` | 429 |
| `SHARED_LINK_DOCUMENT_NOT_FOUND` | 404 |
| `SHARED_LINK_INTERNAL_ERROR` | 500 |

### DocumentError (`src/errors/document.rs`)

| Code | HTTP status |
|------|-------------|
| UPLOAD_BAD_REQUEST | 400 |
| INVALID_QUERY | 400 |
| UPLOAD_NOT_FOUND | 404 |
| UPLOAD_CONFLICT | 409 |
| UPLOAD_TOO_LARGE | 413 |
| UPLOAD_INTERNAL_ERROR | 500 |
| UPLOAD_TIMEOUT | 408 |
| UPLOAD_DB_CONSTRAINT | 409 |
| UPLOAD_OCR_ERROR | 422 |
| UPLOAD_FILE_PROCESSING_ERROR | 422 |
| UPLOAD_CONCURRENT_ERROR | 429 |

## Testing

Error codes are asserted in unit tests next to the code that produces them (for example the `tests` module in `src/routes/users.rs` checks `USER_DUPLICATE_USERNAME` and `USER_DUPLICATE_EMAIL`), and response bodies in integration tests such as `tests/integration_upload_error_detail_tests.rs` and `tests/integration_error_handling_edge_cases_tests.rs`. See [Testing](TESTING.md) for how to run them.
