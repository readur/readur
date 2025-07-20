# Error System Documentation

This document describes the comprehensive error handling system implemented in readur for consistent, maintainable, and user-friendly error responses.

## Overview

The error system provides structured, typed error handling across all API endpoints with automatic integration into the existing error management and monitoring infrastructure.

## Architecture

### Core Components

1. **`src/errors/mod.rs`** - Central error module with shared traits and utilities
2. **Entity-specific error modules** - Dedicated error types for each domain:
   - `src/errors/user.rs` - User management errors
   - `src/errors/source.rs` - File source operation errors  
   - `src/errors/label.rs` - Label management errors
   - `src/errors/settings.rs` - Settings configuration errors
   - `src/errors/search.rs` - Search operation errors

### AppError Trait

All custom errors implement the `AppError` trait which provides:

```rust
pub trait AppError: std::error::Error + Send + Sync + 'static {
    fn status_code(&self) -> StatusCode;           // HTTP status code
    fn user_message(&self) -> String;              // User-friendly message
    fn error_code(&self) -> &'static str;          // Frontend error code
    fn error_category(&self) -> ErrorCategory;     // Error categorization
    fn error_severity(&self) -> ErrorSeverity;     // Severity level
    fn suppression_key(&self) -> Option<String>;   // For repeated error handling
    fn suggested_action(&self) -> Option<String>;  // Recovery suggestions
}
```

## Error Type Reference

The following table provides a comprehensive overview of all error types, when to use them, and their characteristics:

| Error Type | Module | When to Use | Common Scenarios | Example Error Codes | HTTP Status |
|------------|--------|-------------|------------------|-------------------|-------------|
| **ApiError** | `errors::mod` | Generic API errors when specific types don't apply | Rate limiting, payload validation, generic server errors | `BAD_REQUEST`, `NOT_FOUND`, `UNAUTHORIZED`, `INTERNAL_SERVER_ERROR` | 400, 401, 404, 500 |
| **UserError** | `errors::user` | User management and authentication operations | Registration, login, profile updates, permissions | `USER_NOT_FOUND`, `USER_DUPLICATE_USERNAME`, `USER_PERMISSION_DENIED` | 400, 401, 403, 404, 409 |
| **SourceError** | `errors::source` | File source operations (WebDAV, S3, Local Folder) | Source configuration, sync operations, connection issues | `SOURCE_CONNECTION_FAILED`, `SOURCE_AUTH_FAILED`, `SOURCE_SYNC_IN_PROGRESS` | 400, 401, 404, 409, 503 |
| **LabelError** | `errors::label` | Document labeling and label management | Creating/editing labels, label assignment, system labels | `LABEL_DUPLICATE_NAME`, `LABEL_IN_USE`, `LABEL_SYSTEM_MODIFICATION` | 400, 403, 404, 409 |
| **SettingsError** | `errors::settings` | Application and user settings validation | OCR configuration, preferences, validation | `SETTINGS_INVALID_LANGUAGE`, `SETTINGS_VALUE_OUT_OF_RANGE`, `SETTINGS_READ_ONLY` | 400, 403, 404 |
| **SearchError** | `errors::search` | Document search and indexing operations | Search queries, index operations, result processing | `SEARCH_QUERY_TOO_SHORT`, `SEARCH_INDEX_UNAVAILABLE`, `SEARCH_TOO_MANY_RESULTS` | 400, 503, 429 |
| **OcrError** | `ocr::error` | OCR processing operations | Tesseract operations, image processing, language detection | `OCR_NOT_INSTALLED`, `OCR_LANG_MISSING`, `OCR_TIMEOUT` | 400, 500, 503 |
| **DocumentError** | `routes::documents::crud` | Document CRUD operations | File upload, download, metadata operations | `DOCUMENT_NOT_FOUND`, `DOCUMENT_TOO_LARGE` | 400, 404, 413, 500 |

### Error Type Usage Guidelines

#### **ApiError** - Generic API Errors
- **Use when**: No specific error type applies
- **Best for**: Cross-cutting concerns, middleware errors, generic validation
- **Avoid when**: Domain-specific operations have dedicated error types
- **Example**: Request rate limiting, malformed JSON, generic server failures

#### **UserError** - User Management
- **Use when**: Operations involve user accounts, authentication, or authorization
- **Best for**: Registration, login, profile management, role/permission checks
- **Covers**: Account creation, credential validation, access control
- **Example**: Duplicate usernames, invalid passwords, insufficient permissions

#### **SourceError** - File Sources
- **Use when**: Operations involve external file sources (WebDAV, S3, etc.)
- **Best for**: Source configuration, sync operations, connection management
- **Covers**: Authentication to external systems, network connectivity, sync status
- **Example**: WebDAV connection failures, S3 credential issues, sync conflicts

#### **LabelError** - Label Management
- **Use when**: Operations involve document labels or label management
- **Best for**: Label CRUD operations, label assignment to documents
- **Covers**: Label validation, system label protection, usage tracking
- **Example**: Duplicate label names, modifying system labels, deleting used labels

#### **SettingsError** - Configuration & Settings
- **Use when**: Operations involve application or user settings
- **Best for**: Configuration validation, preference management, OCR settings
- **Covers**: Value validation, constraint checking, read-only setting protection
- **Example**: Invalid OCR languages, out-of-range values, conflicting settings

#### **SearchError** - Search Operations
- **Use when**: Operations involve document search or search index management
- **Best for**: Query validation, index operations, result processing
- **Covers**: Query syntax, performance limits, index availability
- **Example**: Short queries, syntax errors, index rebuilding, too many results

#### **OcrError** - OCR Processing
- **Use when**: Operations involve OCR processing with Tesseract
- **Best for**: OCR-specific failures, Tesseract configuration issues
- **Covers**: Installation checks, language data, processing errors
- **Example**: Missing Tesseract, language data not found, processing timeouts

#### **DocumentError** - Document Operations
- **Use when**: Operations involve document CRUD operations
- **Best for**: File upload/download, document metadata, storage operations
- **Covers**: File validation, storage limits, document lifecycle
- **Example**: File too large, unsupported format, document not found

### Error Severity Mapping

| Error Type | Typical Severity | Reasoning |
|------------|------------------|-----------|
| **ApiError** | Minor to Critical | Depends on specific error - server errors are Critical, validation is Minor |
| **UserError** | Minor to Important | Auth failures are Important, validation errors are Minor |
| **SourceError** | Minor to Important | Connection issues are Important, config errors are Minor |
| **LabelError** | Minor | Usually user input validation, rarely system-critical |
| **SettingsError** | Minor | Configuration errors, typically user-correctable |
| **SearchError** | Minor to Important | Index unavailable is Important, query errors are Minor |
| **OcrError** | Minor to Critical | Missing installation is Critical, processing errors are Minor |
| **DocumentError** | Minor to Important | Storage failures are Important, validation is Minor |

### Error Type Decision Tree

Use this decision tree to choose the appropriate error type:

```
Is this a user account/authentication operation?
├─ YES → UserError
└─ NO → Is this a file source operation (WebDAV, S3, Local)?
    ├─ YES → SourceError  
    └─ NO → Is this a search/indexing operation?
        ├─ YES → SearchError
        └─ NO → Is this an OCR/Tesseract operation?
            ├─ YES → OcrError
            └─ NO → Is this a document upload/download/CRUD operation?
                ├─ YES → DocumentError
                └─ NO → Is this a label management operation?
                    ├─ YES → LabelError
                    └─ NO → Is this a settings/configuration operation?
                        ├─ YES → SettingsError
                        └─ NO → Use ApiError for generic cases
```

### Quick Reference Checklist

**Before choosing an error type, ask:**

- [ ] Does the operation primarily involve user accounts, roles, or authentication? → **UserError**
- [ ] Does the operation connect to external file sources? → **SourceError** 
- [ ] Does the operation search documents or manage search index? → **SearchError**
- [ ] Does the operation use Tesseract for OCR processing? → **OcrError**
- [ ] Does the operation upload, download, or manage document files? → **DocumentError**
- [ ] Does the operation create, modify, or assign labels? → **LabelError**
- [ ] Does the operation validate or modify application settings? → **SettingsError**
- [ ] None of the above apply? → **ApiError**

### Common Error Mapping Patterns

| Operation Type | Recommended Error Type | Example Operations |
|----------------|----------------------|-------------------|
| User registration/login | UserError | `/api/auth/register`, `/api/auth/login` |
| File source sync | SourceError | `/api/sources/{id}/sync`, `/api/sources/{id}/test` |
| Document search | SearchError | `/api/search`, `/api/documents/search` |
| OCR processing | OcrError | `/api/documents/{id}/ocr`, `/api/ocr/languages` |
| Document management | DocumentError | `/api/documents`, `/api/documents/{id}` |
| Label operations | LabelError | `/api/labels`, `/api/documents/{id}/labels` |
| Settings management | SettingsError | `/api/settings`, `/api/users/{id}/settings` |
| Generic API operations | ApiError | Rate limiting, payload validation, CORS |

## Error Types

### UserError

Handles user management operations:

```rust
// Examples
UserError::NotFound
UserError::DuplicateUsername { username: "john_doe" }
UserError::PermissionDenied { reason: "Admin access required" }
UserError::InvalidCredentials
UserError::DeleteRestricted { id, reason: "Cannot delete your own account" }
```

**Error Codes**: `USER_NOT_FOUND`, `USER_DUPLICATE_USERNAME`, `USER_PERMISSION_DENIED`, etc.

### SourceError

Handles file source operations (WebDAV, Local Folder, S3):

```rust
// Examples  
SourceError::ConnectionFailed { details: "Network timeout" }
SourceError::InvalidPath { path: "/invalid/path" }
SourceError::AuthenticationFailed { name: "my-webdav", reason: "Invalid credentials" }
SourceError::SyncInProgress { name: "backup-source" }
SourceError::ConfigurationInvalid { details: "Missing server URL" }
```

**Error Codes**: `SOURCE_CONNECTION_FAILED`, `SOURCE_AUTH_FAILED`, `SOURCE_CONFIG_INVALID`, etc.

### LabelError

Handles label management:

```rust
// Examples
LabelError::DuplicateName { name: "Important" }
LabelError::SystemLabelModification { name: "system-label" }
LabelError::InvalidColor { color: "#gggggg" }
LabelError::LabelInUse { document_count: 42 }
LabelError::MaxLabelsReached { max_labels: 100 }
```

**Error Codes**: `LABEL_DUPLICATE_NAME`, `LABEL_SYSTEM_MODIFICATION`, `LABEL_IN_USE`, etc.

### SettingsError

Handles settings validation and management:

```rust
// Examples
SettingsError::InvalidLanguage { language: "xx", available_languages: "en,es,fr" }
SettingsError::ValueOutOfRange { setting_name: "timeout", value: 3600, min: 1, max: 300 }
SettingsError::InvalidOcrConfiguration { details: "DPI too high" }
SettingsError::ConflictingSettings { setting1: "auto_detect", setting2: "fixed_language" }
```

**Error Codes**: `SETTINGS_INVALID_LANGUAGE`, `SETTINGS_VALUE_OUT_OF_RANGE`, etc.

### SearchError

Handles search operations:

```rust
// Examples
SearchError::QueryTooShort { length: 1, min_length: 2 }
SearchError::TooManyResults { result_count: 15000, max_results: 10000 }
SearchError::IndexUnavailable { reason: "Rebuilding index" }
SearchError::InvalidPagination { offset: -1, limit: 0 }
```

**Error Codes**: `SEARCH_QUERY_TOO_SHORT`, `SEARCH_TOO_MANY_RESULTS`, etc.

## Integration Features

### Error Management System

All errors automatically integrate with the sophisticated error management system in `src/monitoring/error_management.rs`:

- **Categorization**: Errors are categorized (Auth, Database, Network, etc.)
- **Severity Levels**: Critical, Important, Minor, Expected
- **Intelligent Suppression**: Prevents spam from repeated errors
- **Structured Logging**: Consistent log format with context

### HTTP Response Format

All errors return consistent JSON responses:

```json
{
  "error": "User-friendly error message",
  "code": "ERROR_CODE_FOR_FRONTEND", 
  "status": 400
}
```

### Frontend Error Codes

Each error provides a stable error code for frontend handling:

```typescript
// Frontend can handle specific errors
switch (error.code) {
  case 'USER_DUPLICATE_USERNAME':
    showUsernameAlreadyExistsMessage();
    break;
  case 'SOURCE_CONNECTION_FAILED':
    showNetworkErrorDialog();
    break;
  case 'SEARCH_QUERY_TOO_SHORT':
    highlightSearchInput();
    break;
}
```

## Usage Examples

### In Route Handlers

```rust
use crate::errors::user::UserError;

async fn create_user(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(user_data): Json<CreateUser>,
) -> Result<Json<UserResponse>, UserError> {
    require_admin(&auth_user)?;
    
    let user = state
        .db
        .create_user(user_data)
        .await
        .map_err(|e| {
            let error_msg = e.to_string();
            if error_msg.contains("username") && error_msg.contains("unique") {
                UserError::duplicate_username(&user_data.username)
            } else if error_msg.contains("email") && error_msg.contains("unique") {
                UserError::duplicate_email(&user_data.email)
            } else {
                UserError::internal_server_error(format!("Database error: {}", e))
            }
        })?;

    Ok(Json(user.into()))
}
```

### Convenience Methods

All error types provide convenience constructors:

```rust
// Instead of verbose enum construction
UserError::DuplicateUsername { username: username.clone() }

// Use convenience methods
UserError::duplicate_username(&username)
UserError::permission_denied("Admin access required")
UserError::not_found_by_id(user_id)
```

### Error Context and Suggestions

Errors include contextual information and recovery suggestions:

```rust
LabelError::invalid_color("#gggggg") 
// Returns: "Invalid color format - use hex format like #0969da"
// Suggestion: "Use a valid hex color format like #0969da or #ff5722"

SourceError::rate_limit_exceeded("my-source", 300)
// Returns: "Rate limit exceeded, try again in 300 seconds" 
// Suggestion: "Wait 300 seconds before retrying"
```

## Best Practices

### 1. Use Specific Error Types

```rust
// Good
return Err(UserError::duplicate_username(&username));

// Avoid
return Err(UserError::BadRequest { message: "Username exists".to_string() });
```

### 2. Provide Context

```rust
// Good
SourceError::connection_failed(format!("Failed to connect to {}: {}", url, e))

// Less helpful
SourceError::connection_failed("Connection failed")
```

### 3. Handle Database Errors Thoughtfully

```rust
.map_err(|e| {
    let error_msg = e.to_string();
    if error_msg.contains("unique constraint") {
        UserError::duplicate_username(&username)
    } else if error_msg.contains("not found") {
        UserError::not_found()
    } else {
        UserError::internal_server_error(format!("Database error: {}", e))
    }
})?
```

### 4. Use Suppression Keys Wisely

```rust
impl AppError for SearchError {
    fn suppression_key(&self) -> Option<String> {
        match self {
            // Suppress repeated "no results" errors
            SearchError::NoResults => Some("search_no_results".to_string()),
            // Don't suppress validation errors - users need to see them
            SearchError::QueryTooShort { .. } => None,
            // Suppress by specific source for connection errors
            SearchError::IndexUnavailable { .. } => Some("search_index_unavailable".to_string()),
            _ => None,
        }
    }
}
```

## Migration from Generic Errors

When updating existing endpoints:

1. **Add error type import**:
   ```rust
   use crate::errors::user::UserError;
   ```

2. **Update function signature**:
   ```rust
   // Before
   async fn my_handler() -> Result<Json<Response>, StatusCode>
   
   // After  
   async fn my_handler() -> Result<Json<Response>, UserError>
   ```

3. **Replace generic error mapping**:
   ```rust
   // Before
   .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
   
   // After
   .map_err(|e| UserError::internal_server_error(format!("Operation failed: {}", e)))?
   ```

4. **Update tests** to expect new error format:
   ```rust
   // Before
   assert_eq!(response.status(), StatusCode::CONFLICT);
   
   // After - check JSON response
   let error: serde_json::Value = response.json().await;
   assert_eq!(error["code"], "USER_DELETE_RESTRICTED");
   ```

## Testing

The error system includes comprehensive testing to ensure:

- Correct HTTP status codes
- Proper JSON response format  
- Error code consistency
- Integration with error management
- Suppression behavior

Run error-specific tests:
```bash
cargo test user_error
cargo test source_error  
cargo test error_integration
```

## Error Code Conventions

All error types follow consistent naming conventions for error codes:

### Naming Pattern
```
{TYPE}_{SPECIFIC_ERROR}
```

### Examples by Type
| Error Type | Prefix | Example Codes |
|------------|--------|---------------|
| **ApiError** | None | `BAD_REQUEST`, `NOT_FOUND`, `UNAUTHORIZED` |
| **UserError** | `USER_` | `USER_NOT_FOUND`, `USER_DUPLICATE_USERNAME`, `USER_PERMISSION_DENIED` |
| **SourceError** | `SOURCE_` | `SOURCE_CONNECTION_FAILED`, `SOURCE_AUTH_FAILED`, `SOURCE_CONFIG_INVALID` |
| **LabelError** | `LABEL_` | `LABEL_DUPLICATE_NAME`, `LABEL_IN_USE`, `LABEL_SYSTEM_MODIFICATION` |
| **SettingsError** | `SETTINGS_` | `SETTINGS_INVALID_LANGUAGE`, `SETTINGS_VALUE_OUT_OF_RANGE` |
| **SearchError** | `SEARCH_` | `SEARCH_QUERY_TOO_SHORT`, `SEARCH_INDEX_UNAVAILABLE` |
| **OcrError** | `OCR_` | `OCR_NOT_INSTALLED`, `OCR_LANG_MISSING`, `OCR_TIMEOUT` |
| **DocumentError** | `DOCUMENT_` | `DOCUMENT_NOT_FOUND`, `DOCUMENT_TOO_LARGE` |

### Code Style Guidelines
- Use `SCREAMING_SNAKE_CASE` for all error codes
- Start with error type prefix (except ApiError)
- Be descriptive but concise
- Avoid abbreviations unless commonly understood
- Group related errors with consistent sub-prefixes

## Practical Examples

### Complete Implementation Examples

#### User Registration Endpoint
```rust
use crate::errors::user::UserError;

async fn register_user(
    State(state): State<Arc<AppState>>,
    Json(user_data): Json<CreateUser>,
) -> Result<Json<UserResponse>, UserError> {
    // Validate input
    if user_data.username.len() < 3 {
        return Err(UserError::invalid_username(
            &user_data.username, 
            "Username must be at least 3 characters"
        ));
    }
    
    // Create user
    let user = state.db.create_user(user_data).await
        .map_err(|e| {
            let error_msg = e.to_string();
            if error_msg.contains("username") && error_msg.contains("unique") {
                UserError::duplicate_username(&user_data.username)
            } else if error_msg.contains("email") && error_msg.contains("unique") {
                UserError::duplicate_email(&user_data.email)
            } else {
                UserError::internal_server_error(format!("Database error: {}", e))
            }
        })?;

    Ok(Json(user.into()))
}
```

#### WebDAV Source Test Endpoint
```rust
use crate::errors::source::SourceError;

async fn test_webdav_connection(
    State(state): State<Arc<AppState>>,
    Path(source_id): Path<Uuid>,
    auth_user: AuthUser,
) -> Result<Json<ConnectionTestResponse>, SourceError> {
    let source = state.db.get_source(source_id).await
        .map_err(|_| SourceError::not_found_by_id(source_id))?;
    
    // Check ownership
    if source.user_id != auth_user.user.id {
        return Err(SourceError::permission_denied(
            "You can only test your own sources"
        ));
    }
    
    // Test connection
    match test_webdav_connection_internal(&source).await {
        Ok(()) => Ok(Json(ConnectionTestResponse { success: true })),
        Err(e) if e.to_string().contains("authentication") => {
            Err(SourceError::authentication_failed(&source.name, &e.to_string()))
        },
        Err(e) if e.to_string().contains("timeout") => {
            Err(SourceError::network_timeout(&source.url, 30))
        },
        Err(e) => {
            Err(SourceError::connection_failed(format!("Connection test failed: {}", e)))
        }
    }
}
```

#### Search Query Endpoint
```rust
use crate::errors::search::SearchError;

async fn search_documents(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>,
    auth_user: AuthUser,
) -> Result<Json<SearchResponse>, SearchError> {
    // Validate query length
    if params.query.len() < 2 {
        return Err(SearchError::query_too_short(params.query.len(), 2));
    }
    
    if params.query.len() > 500 {
        return Err(SearchError::query_too_long(params.query.len(), 500));
    }
    
    // Check pagination
    if params.limit > 1000 {
        return Err(SearchError::invalid_pagination(params.offset, params.limit));
    }
    
    // Perform search
    let results = state.search_service.search(&params, auth_user.user.id).await
        .map_err(|e| match e {
            SearchServiceError::IndexUnavailable => {
                SearchError::index_unavailable("Search index is being rebuilt")
            },
            SearchServiceError::TooManyResults(count) => {
                SearchError::too_many_results(count, 10000)
            },
            SearchServiceError::InvalidSyntax(details) => {
                SearchError::invalid_syntax(details)
            },
            _ => SearchError::internal_error(format!("Search failed: {}", e)),
        })?;
    
    Ok(Json(results))
}
```

## Monitoring

Error metrics are automatically tracked:

- **Error rates** by type and endpoint
- **Suppression statistics** for repeated errors
- **Severity distribution** across the application
- **Recovery suggestions** utilization

View error dashboards in Grafana or check Prometheus metrics at `/metrics`.

## Future Enhancements

Planned improvements to the error system:

1. **Internationalization** - Multi-language error messages
2. **Error Analytics** - Advanced error pattern detection  
3. **Auto-Recovery** - Suggested API retry strategies
4. **Enhanced Suppression** - Time-based and pattern-based suppression
5. **Error Documentation** - Auto-generated API error documentation

## References

- [Error Management Documentation](./ERROR_MANAGEMENT.md)
- [API Error Response Standards](../api-reference.md#error-responses)
- [Frontend Error Handling Guide](../../frontend/ERROR_HANDLING.md)
- [Monitoring and Observability](./MONITORING.md)