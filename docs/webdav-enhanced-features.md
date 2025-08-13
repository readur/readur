# WebDAV Enhanced Features Documentation

This document describes the critical WebDAV features that have been implemented to provide comprehensive WebDAV protocol support.

## Table of Contents
1. [WebDAV File Locking (LOCK/UNLOCK)](#webdav-file-locking)
2. [Partial Content/Resume Support](#partial-content-support)
3. [Directory Operations (MKCOL)](#directory-operations)
4. [Enhanced Status Code Handling](#status-code-handling)

## WebDAV File Locking

### Overview
WebDAV locking prevents concurrent modification issues by allowing clients to lock resources before modifying them. This implementation supports both exclusive and shared locks with configurable timeouts.

### Features
- **LOCK Method**: Acquire exclusive or shared locks on resources
- **UNLOCK Method**: Release previously acquired locks
- **Lock Tokens**: Opaque lock tokens in the format `opaquelocktoken:UUID`
- **Lock Refresh**: Extend lock timeout before expiration
- **Depth Support**: Lock individual resources or entire directory trees
- **Automatic Cleanup**: Expired locks are automatically removed

### Usage

#### Acquiring a Lock
```rust
use readur::services::webdav::{WebDAVService, LockScope};

// Acquire an exclusive lock
let lock_info = service.lock_resource(
    "/documents/important.docx",
    LockScope::Exclusive,
    Some("user@example.com".to_string()), // owner
    Some(3600), // timeout in seconds
).await?;

println!("Lock token: {}", lock_info.token);
```

#### Checking Lock Status
```rust
// Check if a resource is locked
if service.is_locked("/documents/important.docx").await {
    println!("Resource is locked");
}

// Get all locks on a resource
let locks = service.get_lock_info("/documents/important.docx").await;
for lock in locks {
    println!("Lock: {} (expires: {:?})", lock.token, lock.expires_at);
}
```

#### Refreshing a Lock
```rust
// Refresh lock before it expires
let refreshed = service.refresh_lock(&lock_info.token, Some(7200)).await?;
println!("Lock extended until: {:?}", refreshed.expires_at);
```

#### Releasing a Lock
```rust
// Release the lock when done
service.unlock_resource("/documents/important.docx", &lock_info.token).await?;
```

### Lock Types
- **Exclusive Lock**: Only one client can hold an exclusive lock
- **Shared Lock**: Multiple clients can hold shared locks simultaneously

### Error Handling
- **423 Locked**: Resource is already locked by another process
- **412 Precondition Failed**: Lock token is invalid or expired
- **409 Conflict**: Lock conflicts with existing locks

## Partial Content Support

### Overview
Partial content support enables reliable downloads with resume capability, essential for large files or unreliable connections. The implementation follows RFC 7233 for HTTP Range Requests.

### Features
- **Range Headers**: Support for byte-range requests
- **206 Partial Content**: Handle partial content responses
- **Resume Capability**: Continue interrupted downloads
- **Chunked Downloads**: Download large files in manageable chunks
- **Progress Tracking**: Monitor download progress in real-time

### Usage

#### Downloading a Specific Range
```rust
use readur::services::webdav::ByteRange;

// Download bytes 0-1023 (first 1KB)
let chunk = service.download_file_range(
    "/videos/large_file.mp4",
    0,
    Some(1023)
).await?;

// Download from byte 1024 to end of file
let rest = service.download_file_range(
    "/videos/large_file.mp4",
    1024,
    None
).await?;
```

#### Download with Resume Support
```rust
use std::path::PathBuf;

// Download with automatic resume on failure
let local_path = PathBuf::from("/downloads/large_file.mp4");
let content = service.download_file_with_resume(
    "/videos/large_file.mp4",
    local_path
).await?;
```

#### Monitoring Download Progress
```rust
// Get progress of a specific download
if let Some(progress) = service.get_download_progress("/videos/large_file.mp4").await {
    println!("Downloaded: {} / {} bytes ({:.1}%)",
        progress.bytes_downloaded,
        progress.total_size,
        progress.percentage_complete()
    );
}

// List all active downloads
let downloads = service.list_active_downloads().await;
for download in downloads {
    println!("{}: {:.1}% complete", 
        download.resource_path,
        download.percentage_complete()
    );
}
```

#### Canceling a Download
```rust
// Cancel an active download
service.cancel_download("/videos/large_file.mp4").await?;
```

### Range Format
- `bytes=0-1023` - First 1024 bytes
- `bytes=1024-` - From byte 1024 to end
- `bytes=-500` - Last 500 bytes
- `bytes=0-500,1000-1500` - Multiple ranges

## Directory Operations

### Overview
Comprehensive directory management using WebDAV-specific methods, including the MKCOL method for creating collections (directories).

### Features
- **MKCOL Method**: Create directories with proper WebDAV semantics
- **Recursive Creation**: Create entire directory trees
- **MOVE Method**: Move or rename directories
- **COPY Method**: Copy directories with depth control
- **DELETE Method**: Delete directories recursively
- **Directory Properties**: Set custom properties on directories

### Usage

#### Creating Directories
```rust
use readur::services::webdav::CreateDirectoryOptions;

// Create a single directory
let result = service.create_directory(
    "/projects/new_project",
    CreateDirectoryOptions::default()
).await?;

// Create with parent directories
let options = CreateDirectoryOptions {
    create_parents: true,
    fail_if_exists: false,
    properties: None,
};
let result = service.create_directory(
    "/projects/2024/january/reports",
    options
).await?;

// Create entire path recursively
let results = service.create_directory_recursive(
    "/projects/2024/january/reports"
).await?;
```

#### Checking Directory Existence
```rust
if service.directory_exists("/projects/2024").await? {
    println!("Directory exists");
}
```

#### Listing Directory Contents
```rust
let contents = service.list_directory("/projects").await?;
for item in contents {
    println!("  {}", item);
}
```

#### Moving Directories
```rust
// Move (rename) a directory
service.move_directory(
    "/projects/old_name",
    "/projects/new_name",
    false // don't overwrite if exists
).await?;
```

#### Copying Directories
```rust
// Copy directory recursively
service.copy_directory(
    "/projects/template",
    "/projects/new_project",
    false, // don't overwrite
    Some("infinity") // recursive copy
).await?;
```

#### Deleting Directories
```rust
// Delete empty directory
service.delete_directory("/projects/old", false).await?;

// Delete directory and all contents
service.delete_directory("/projects/old", true).await?;
```

## Status Code Handling

### Overview
Enhanced error handling for WebDAV-specific status codes, providing detailed error information and automatic retry logic.

### WebDAV Status Codes

#### Success Codes
- **207 Multi-Status**: Response contains multiple status codes
- **208 Already Reported**: Members already enumerated

#### Client Error Codes
- **422 Unprocessable Entity**: Request contains semantic errors
- **423 Locked**: Resource is locked
- **424 Failed Dependency**: Related operation failed

#### Server Error Codes
- **507 Insufficient Storage**: Server storage full
- **508 Loop Detected**: Infinite loop in request

### Error Information
Each error includes:
- Status code and description
- Resource path affected
- Lock token (if applicable)
- Suggested resolution action
- Retry information
- Server-provided details

### Usage

#### Enhanced Error Handling
```rust
use readur::services::webdav::StatusCodeHandler;

// Perform operation with enhanced error handling
let response = service.authenticated_request_enhanced(
    Method::GET,
    &url,
    None,
    None,
    &[200, 206] // expected status codes
).await?;
```

#### Smart Retry Logic
```rust
// Automatic retry with exponential backoff
let result = service.with_smart_retry(
    || Box::pin(async {
        // Your operation here
        service.download_file("/path/to/file").await
    }),
    3 // max attempts
).await?;
```

#### Error Details
```rust
match service.lock_resource(path, scope, owner, timeout).await {
    Ok(lock) => println!("Locked: {}", lock.token),
    Err(e) => {
        // Error includes WebDAV-specific information:
        // - Status code (e.g., 423)
        // - Lock owner information
        // - Suggested actions
        // - Retry recommendations
        println!("Lock failed: {}", e);
    }
}
```

### Retry Strategy
The system automatically determines if errors are retryable:

| Status Code | Retryable | Default Delay | Backoff |
|------------|-----------|---------------|---------|
| 423 Locked | Yes | 10s | Exponential |
| 429 Too Many Requests | Yes | 60s | Exponential |
| 503 Service Unavailable | Yes | 30s | Exponential |
| 409 Conflict | Yes | 5s | Exponential |
| 500-599 Server Errors | Yes | 30s | Exponential |
| 400-499 Client Errors | No | - | - |

## Integration with Existing Code

All new features are fully integrated with the existing WebDAV service:

```rust
use readur::services::webdav::{
    WebDAVService, WebDAVConfig,
    LockManager, PartialContentManager,
    CreateDirectoryOptions, ByteRange,
    WebDAVStatusCode, WebDAVError
};

// Create service as usual
let config = WebDAVConfig { /* ... */ };
let service = WebDAVService::new(config)?;

// All new features are available through the service
// - Locking: service.lock_resource(), unlock_resource()
// - Partial: service.download_file_range(), download_file_with_resume()
// - Directories: service.create_directory(), delete_directory()
// - Errors: Automatic enhanced error handling
```

## Testing

All features include comprehensive test coverage:

```bash
# Run all tests
cargo test --lib

# Run specific feature tests
cargo test locking_tests
cargo test partial_content_tests
cargo test directory_ops_tests

# Run integration tests (requires WebDAV server)
cargo test --ignored
```

## Performance Considerations

1. **Lock Management**: Locks are stored in memory with automatic cleanup of expired locks
2. **Partial Downloads**: Configurable chunk size (default 1MB) for optimal performance
3. **Directory Operations**: Batch operations use concurrent processing with semaphore control
4. **Error Handling**: Smart retry with exponential backoff prevents server overload

## Security Considerations

1. **Lock Tokens**: Use cryptographically secure UUIDs
2. **Authentication**: All operations use HTTP Basic Auth (configure HTTPS in production)
3. **Timeouts**: Configurable timeouts prevent resource exhaustion
4. **Rate Limiting**: Respect server rate limits with automatic backoff

## Compatibility

The implementation follows these standards:
- RFC 4918 (WebDAV)
- RFC 7233 (HTTP Range Requests)
- RFC 2518 (WebDAV Locking)

Tested with:
- Nextcloud
- ownCloud
- Apache mod_dav
- Generic WebDAV servers

## Migration Guide

For existing code using the WebDAV service:

1. **No Breaking Changes**: All existing methods continue to work
2. **New Features Are Opt-In**: Use new methods only when needed
3. **Enhanced Error Information**: Errors now include more details but maintain backward compatibility
4. **Automatic Benefits**: Some improvements (like better error handling) apply automatically

## Troubleshooting

### Lock Issues
- **423 Locked Error**: Another client holds a lock. Wait or use lock token
- **Lock Token Invalid**: Lock may have expired. Acquire a new lock
- **Locks Not Released**: Implement proper cleanup in error paths

### Partial Content Issues
- **Server Doesn't Support Ranges**: Falls back to full download automatically
- **Resume Fails**: File may have changed. Restart download
- **Slow Performance**: Adjust chunk size based on network conditions

### Directory Operation Issues
- **409 Conflict**: Parent directory doesn't exist. Use `create_parents: true`
- **405 Method Not Allowed**: Directory may already exist or server doesn't support MKCOL
- **507 Insufficient Storage**: Server storage full. Contact administrator

## Future Enhancements

Potential future improvements:
- WebDAV SEARCH method support
- Advanced property management (PROPPATCH)
- Access control (WebDAV ACL)
- Versioning support (DeltaV)
- Collection synchronization (WebDAV Sync)