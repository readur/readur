/// Integration tests for source error database functions
/// Tests the complete lifecycle of source scan failure tracking including:
/// - Recording failures with proper enum type casting
/// - Querying failures with various filters  
/// - Resolving and resetting failures
/// - Statistics aggregation
///
/// These tests specifically ensure that PostgreSQL enum types are handled correctly
/// to prevent runtime SQL errors like "function resolve_source_scan_failure does not exist"
/// or "operator does not exist: source_error_source_type = text"

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use readur::test_utils::TestContext;
    use readur::models::{
        CreateSourceScanFailure, ErrorSourceType, SourceErrorType, 
        SourceErrorSeverity, ListFailuresQuery, SourceScanFailure
    };
    use uuid::Uuid;
    use serde_json::json;

    #[tokio::test]
    async fn test_record_source_scan_failure_with_enum_types() {
        let ctx = TestContext::new().await;
        
        let result: Result<()> = async {
            let db = &ctx.state.db;
            let user_id = Uuid::new_v4();
            
            // Create test user
            let unique_suffix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let username = format!("test_source_error_user_{}", unique_suffix);
            let email = format!("test_source_error_{}@example.com", unique_suffix);
            
            sqlx::query(
                "INSERT INTO users (id, username, email, password_hash, role) 
                 VALUES ($1, $2, $3, $4, $5)"
            )
            .bind(user_id)
            .bind(&username)
            .bind(&email)
            .bind("hash")
            .bind("user")
            .execute(db.get_pool())
            .await?;

            // Test recording a source scan failure with all enum types
            let failure = CreateSourceScanFailure {
                user_id,
                source_type: ErrorSourceType::WebDAV,
                source_id: None, // No source_id to avoid foreign key constraint
                resource_path: "/test/path/to/file.pdf".to_string(),
                error_type: SourceErrorType::PermissionDenied,
                error_message: "Access denied to resource".to_string(),
                error_code: Some("403".to_string()),
                http_status_code: Some(403),
                response_time_ms: Some(250),
                response_size_bytes: Some(1024),
                resource_size_bytes: Some(2048),
                diagnostic_data: Some(json!({
                    "headers": {
                        "content-type": "text/html"
                    }
                })),
            };

            // This should succeed with proper enum type casting
            let failure_id = db.record_source_scan_failure(&failure).await?;
            assert!(!failure_id.is_nil());

            // Test with different enum values
            let failure2 = CreateSourceScanFailure {
                user_id,
                source_type: ErrorSourceType::S3,
                source_id: None,
                resource_path: "bucket/key/object.txt".to_string(),
                error_type: SourceErrorType::Timeout,
                error_message: "Request timed out".to_string(),
                error_code: None,
                http_status_code: None,
                response_time_ms: Some(30000),
                response_size_bytes: None,
                resource_size_bytes: None,
                diagnostic_data: None,
            };

            let failure_id2 = db.record_source_scan_failure(&failure2).await?;
            assert!(!failure_id2.is_nil());

            Ok(())
        }.await;
        
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_list_source_scan_failures_with_enum_filters() {
        let ctx = TestContext::new().await;
        
        let result: Result<()> = async {
            let db = &ctx.state.db;
            let user_id = Uuid::new_v4();
            
            // Create test user
            let unique_suffix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let username = format!("test_list_failures_user_{}", unique_suffix);
            let email = format!("test_list_failures_{}@example.com", unique_suffix);
            
            sqlx::query(
                "INSERT INTO users (id, username, email, password_hash, role) 
                 VALUES ($1, $2, $3, $4, $5)"
            )
            .bind(user_id)
            .bind(&username)
            .bind(&email)
            .bind("hash")
            .bind("user")
            .execute(db.get_pool())
            .await?;

            // Create multiple failures with different enum values
            let failures_data = vec![
                (ErrorSourceType::WebDAV, SourceErrorType::PermissionDenied),
                (ErrorSourceType::WebDAV, SourceErrorType::Timeout),
                (ErrorSourceType::S3, SourceErrorType::NetworkError),
                (ErrorSourceType::Local, SourceErrorType::PathTooLong),
            ];

            for (source_type, error_type) in failures_data {
                let failure = CreateSourceScanFailure {
                    user_id,
                    source_type: source_type.clone(),
                    source_id: None,
                    resource_path: format!("/test/{}/{}", source_type, error_type),
                    error_type: error_type.clone(),
                    error_message: format!("Test error: {}", error_type),
                    error_code: None,
                    http_status_code: None,
                    response_time_ms: None,
                    response_size_bytes: None,
                    resource_size_bytes: None,
                    diagnostic_data: None,
                };
                db.record_source_scan_failure(&failure).await?;
            }

            // Test querying with source_type filter (tests enum comparison)
            let query = ListFailuresQuery {
                source_type: Some(ErrorSourceType::WebDAV),
                source_id: None,
                error_type: None,
                severity: None,
                include_resolved: Some(false),
                include_excluded: Some(false),
                ready_for_retry: None,
                limit: None,
                offset: None,
            };

            let webdav_failures = db.list_source_scan_failures(user_id, &query).await?;
            assert_eq!(webdav_failures.len(), 2);

            // Test querying with error_type filter
            let query = ListFailuresQuery {
                source_type: None,
                source_id: None,
                error_type: Some(SourceErrorType::Timeout),
                severity: None,
                include_resolved: Some(false),
                include_excluded: Some(false),
                ready_for_retry: None,
                limit: None,
                offset: None,
            };

            let timeout_failures = db.list_source_scan_failures(user_id, &query).await?;
            assert_eq!(timeout_failures.len(), 1);

            // Test querying with multiple filters
            let query = ListFailuresQuery {
                source_type: Some(ErrorSourceType::S3),
                source_id: None,
                error_type: Some(SourceErrorType::NetworkError),
                severity: None,
                include_resolved: Some(false),
                include_excluded: Some(false),
                ready_for_retry: None,
                limit: None,
                offset: None,
            };

            let s3_network_failures = db.list_source_scan_failures(user_id, &query).await?;
            assert_eq!(s3_network_failures.len(), 1);

            Ok(())
        }.await;
        
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_resolve_source_scan_failure_with_enum_casting() {
        let ctx = TestContext::new().await;
        
        let result: Result<()> = async {
            let db = &ctx.state.db;
            let user_id = Uuid::new_v4();
            
            // Create test user
            let unique_suffix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let username = format!("test_resolve_user_{}", unique_suffix);
            let email = format!("test_resolve_{}@example.com", unique_suffix);
            
            sqlx::query(
                "INSERT INTO users (id, username, email, password_hash, role) 
                 VALUES ($1, $2, $3, $4, $5)"
            )
            .bind(user_id)
            .bind(&username)
            .bind(&email)
            .bind("hash")
            .bind("user")
            .execute(db.get_pool())
            .await?;

            let source_id = None; // No source_id to avoid foreign key constraint
            let resource_path = "/test/resolve/file.pdf";

            // Create a failure
            let failure = CreateSourceScanFailure {
                user_id,
                source_type: ErrorSourceType::WebDAV,
                source_id,
                resource_path: resource_path.to_string(),
                error_type: SourceErrorType::ServerError,
                error_message: "Internal server error".to_string(),
                error_code: Some("500".to_string()),
                http_status_code: Some(500),
                response_time_ms: None,
                response_size_bytes: None,
                resource_size_bytes: None,
                diagnostic_data: None,
            };

            db.record_source_scan_failure(&failure).await?;

            // Test resolving the failure (this tests the function with enum type casting)
            let resolved = db.resolve_source_scan_failure(
                user_id,
                ErrorSourceType::WebDAV,
                source_id,
                resource_path,
                "manual"
            ).await?;
            
            assert!(resolved);

            // Verify it's marked as resolved
            let query = ListFailuresQuery {
                source_type: Some(ErrorSourceType::WebDAV),
                source_id,
                error_type: None,
                severity: None,
                include_resolved: Some(true), // Include resolved
                include_excluded: Some(false),
                ready_for_retry: None,
                limit: None,
                offset: None,
            };

            let failures = db.list_source_scan_failures(user_id, &query).await?;
            assert_eq!(failures.len(), 1);
            assert!(failures[0].resolved);

            Ok(())
        }.await;
        
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_reset_source_scan_failure_with_enum_casting() {
        let ctx = TestContext::new().await;
        
        let result: Result<()> = async {
            let db = &ctx.state.db;
            let user_id = Uuid::new_v4();
            
            // Create test user
            let unique_suffix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let username = format!("test_reset_user_{}", unique_suffix);
            let email = format!("test_reset_{}@example.com", unique_suffix);
            
            sqlx::query(
                "INSERT INTO users (id, username, email, password_hash, role) 
                 VALUES ($1, $2, $3, $4, $5)"
            )
            .bind(user_id)
            .bind(&username)
            .bind(&email)
            .bind("hash")
            .bind("user")
            .execute(db.get_pool())
            .await?;

            let resource_path = "/test/reset/file.pdf";

            // Create a failure
            let failure = CreateSourceScanFailure {
                user_id,
                source_type: ErrorSourceType::S3,
                source_id: None,
                resource_path: resource_path.to_string(),
                error_type: SourceErrorType::RateLimited,
                error_message: "Rate limit exceeded".to_string(),
                error_code: Some("429".to_string()),
                http_status_code: Some(429),
                response_time_ms: None,
                response_size_bytes: None,
                resource_size_bytes: None,
                diagnostic_data: None,
            };

            db.record_source_scan_failure(&failure).await?;

            // Test resetting the failure
            let reset = db.reset_source_scan_failure(
                user_id,
                ErrorSourceType::S3,
                None,
                resource_path
            ).await?;
            
            assert!(reset);

            Ok(())
        }.await;
        
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_is_source_known_failure_with_enum_comparison() {
        let ctx = TestContext::new().await;
        
        let result: Result<()> = async {
            let db = &ctx.state.db;
            let user_id = Uuid::new_v4();
            
            // Create test user
            let unique_suffix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let username = format!("test_known_failure_user_{}", unique_suffix);
            let email = format!("test_known_failure_{}@example.com", unique_suffix);
            
            sqlx::query(
                "INSERT INTO users (id, username, email, password_hash, role) 
                 VALUES ($1, $2, $3, $4, $5)"
            )
            .bind(user_id)
            .bind(&username)
            .bind(&email)
            .bind("hash")
            .bind("user")
            .execute(db.get_pool())
            .await?;

            let resource_path = "/test/known/file.pdf";

            // Create a critical failure that should be considered "known"
            let failure = CreateSourceScanFailure {
                user_id,
                source_type: ErrorSourceType::Local,
                source_id: None,
                resource_path: resource_path.to_string(),
                error_type: SourceErrorType::PathTooLong,
                error_message: "Path exceeds maximum length".to_string(),
                error_code: None,
                http_status_code: None,
                response_time_ms: None,
                response_size_bytes: None,
                resource_size_bytes: None,
                diagnostic_data: None,
            };

            db.record_source_scan_failure(&failure).await?;

            // Test checking if it's a known failure
            let is_known = db.is_source_known_failure(
                user_id,
                ErrorSourceType::Local,
                None,
                resource_path
            ).await?;
            
            // PathTooLong is a critical error, so it should be considered known
            assert!(is_known);

            // Test with different source type - should not be known
            let is_known_different = db.is_source_known_failure(
                user_id,
                ErrorSourceType::S3, // Different source type
                None,
                resource_path
            ).await?;
            
            assert!(!is_known_different);

            Ok(())
        }.await;
        
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_exclude_source_from_scan_with_enum_casting() {
        let ctx = TestContext::new().await;
        
        let result: Result<()> = async {
            let db = &ctx.state.db;
            let user_id = Uuid::new_v4();
            
            // Create test user
            let unique_suffix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let username = format!("test_exclude_user_{}", unique_suffix);
            let email = format!("test_exclude_{}@example.com", unique_suffix);
            
            sqlx::query(
                "INSERT INTO users (id, username, email, password_hash, role) 
                 VALUES ($1, $2, $3, $4, $5)"
            )
            .bind(user_id)
            .bind(&username)
            .bind(&email)
            .bind("hash")
            .bind("user")
            .execute(db.get_pool())
            .await?;

            let resource_path = "/test/exclude/file.pdf";

            // Create a failure
            let failure = CreateSourceScanFailure {
                user_id,
                source_type: ErrorSourceType::WebDAV,
                source_id: None,
                resource_path: resource_path.to_string(),
                error_type: SourceErrorType::InvalidCharacters,
                error_message: "Invalid characters in filename".to_string(),
                error_code: None,
                http_status_code: None,
                response_time_ms: None,
                response_size_bytes: None,
                resource_size_bytes: None,
                diagnostic_data: None,
            };

            db.record_source_scan_failure(&failure).await?;

            // Test excluding the source from scan
            let excluded = db.exclude_source_from_scan(
                user_id,
                ErrorSourceType::WebDAV,
                None,
                resource_path,
                Some("User requested exclusion")
            ).await?;
            
            assert!(excluded);

            // Verify it's marked as excluded
            let query = ListFailuresQuery {
                source_type: Some(ErrorSourceType::WebDAV),
                source_id: None,
                error_type: None,
                severity: None,
                include_resolved: Some(false),
                include_excluded: Some(true), // Include excluded
                ready_for_retry: None,
                limit: None,
                offset: None,
            };

            let failures = db.list_source_scan_failures(user_id, &query).await?;
            assert_eq!(failures.len(), 1);
            assert!(failures[0].user_excluded);

            Ok(())
        }.await;
        
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_get_source_retry_candidates_with_enum_filter() {
        let ctx = TestContext::new().await;
        
        let result: Result<()> = async {
            let db = &ctx.state.db;
            let user_id = Uuid::new_v4();
            
            // Create test user
            let unique_suffix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let username = format!("test_retry_candidates_user_{}", unique_suffix);
            let email = format!("test_retry_candidates_{}@example.com", unique_suffix);
            
            sqlx::query(
                "INSERT INTO users (id, username, email, password_hash, role) 
                 VALUES ($1, $2, $3, $4, $5)"
            )
            .bind(user_id)
            .bind(&username)
            .bind(&email)
            .bind("hash")
            .bind("user")
            .execute(db.get_pool())
            .await?;

            // Create failures with different source types
            for i in 0..3 {
                let failure = CreateSourceScanFailure {
                    user_id,
                    source_type: if i == 0 { ErrorSourceType::WebDAV } else { ErrorSourceType::S3 },
                    source_id: None,
                    resource_path: format!("/test/retry/{}.pdf", i),
                    error_type: SourceErrorType::NetworkError,
                    error_message: "Network error".to_string(),
                    error_code: None,
                    http_status_code: None,
                    response_time_ms: None,
                    response_size_bytes: None,
                    resource_size_bytes: None,
                    diagnostic_data: None,
                };
                db.record_source_scan_failure(&failure).await?;
                
                // Manually set next_retry_at to NOW() for testing
                sqlx::query(
                    "UPDATE source_scan_failures 
                     SET next_retry_at = NOW() 
                     WHERE user_id = $1 AND resource_path = $2"
                )
                .bind(user_id)
                .bind(format!("/test/retry/{}.pdf", i))
                .execute(db.get_pool())
                .await?;
            }

            // Test getting retry candidates filtered by source type
            let webdav_candidates = db.get_source_retry_candidates(
                user_id,
                Some(ErrorSourceType::WebDAV),
                10
            ).await?;
            
            assert_eq!(webdav_candidates.len(), 1);
            assert_eq!(webdav_candidates[0].source_type, ErrorSourceType::WebDAV);

            // Test getting all retry candidates
            let all_candidates = db.get_source_retry_candidates(
                user_id,
                None,
                10
            ).await?;
            
            assert_eq!(all_candidates.len(), 3);

            Ok(())
        }.await;
        
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_get_source_scan_failure_stats_with_enum_filter() {
        let ctx = TestContext::new().await;
        
        let result: Result<()> = async {
            let db = &ctx.state.db;
            let user_id = Uuid::new_v4();
            
            // Create test user
            let unique_suffix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let username = format!("test_stats_user_{}", unique_suffix);
            let email = format!("test_stats_{}@example.com", unique_suffix);
            
            sqlx::query(
                "INSERT INTO users (id, username, email, password_hash, role) 
                 VALUES ($1, $2, $3, $4, $5)"
            )
            .bind(user_id)
            .bind(&username)
            .bind(&email)
            .bind("hash")
            .bind("user")
            .execute(db.get_pool())
            .await?;

            // Create multiple failures with different characteristics
            let failures_to_create = vec![
                (ErrorSourceType::WebDAV, SourceErrorType::PermissionDenied, false),
                (ErrorSourceType::WebDAV, SourceErrorType::Timeout, false),
                (ErrorSourceType::S3, SourceErrorType::NetworkError, false),
                (ErrorSourceType::S3, SourceErrorType::ServerError, true), // This one will be resolved
                (ErrorSourceType::Local, SourceErrorType::PathTooLong, false),
            ];

            for (i, (source_type, error_type, should_resolve)) in failures_to_create.iter().enumerate() {
                let failure = CreateSourceScanFailure {
                    user_id,
                    source_type: source_type.clone(),
                    source_id: None,
                    resource_path: format!("/test/stats/{}.pdf", i),
                    error_type: error_type.clone(),
                    error_message: format!("Error: {}", error_type),
                    error_code: None,
                    http_status_code: None,
                    response_time_ms: None,
                    response_size_bytes: None,
                    resource_size_bytes: None,
                    diagnostic_data: None,
                };
                db.record_source_scan_failure(&failure).await?;
                
                if *should_resolve {
                    db.resolve_source_scan_failure(
                        user_id,
                        source_type.clone(),
                        None,
                        &format!("/test/stats/{}.pdf", i),
                        "test"
                    ).await?;
                }
            }

            // Test getting stats for all source types
            let all_stats = db.get_source_scan_failure_stats(user_id, None).await?;
            assert_eq!(all_stats.active_failures, 4); // 5 created, 1 resolved
            assert_eq!(all_stats.resolved_failures, 1);
            assert!(all_stats.by_source_type.contains_key("webdav"));
            assert!(all_stats.by_source_type.contains_key("s3"));
            assert!(all_stats.by_source_type.contains_key("local"));

            // Test getting stats filtered by source type
            let webdav_stats = db.get_source_scan_failure_stats(
                user_id, 
                Some(ErrorSourceType::WebDAV)
            ).await?;
            assert_eq!(webdav_stats.active_failures, 2); // 2 WebDAV failures, none resolved

            Ok(())
        }.await;
        
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_all_enum_values_are_supported() {
        let ctx = TestContext::new().await;
        
        let result: Result<()> = async {
            let db = &ctx.state.db;
            let user_id = Uuid::new_v4();
            
            // Create test user
            let unique_suffix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let username = format!("test_all_enums_user_{}", unique_suffix);
            let email = format!("test_all_enums_{}@example.com", unique_suffix);
            
            sqlx::query(
                "INSERT INTO users (id, username, email, password_hash, role) 
                 VALUES ($1, $2, $3, $4, $5)"
            )
            .bind(user_id)
            .bind(&username)
            .bind(&email)
            .bind("hash")
            .bind("user")
            .execute(db.get_pool())
            .await?;

            // Test all ErrorSourceType values
            let source_types = vec![
                ErrorSourceType::WebDAV,
                ErrorSourceType::S3,
                ErrorSourceType::Local,
            ];

            // Test all SourceErrorType values
            let error_types = vec![
                SourceErrorType::Timeout,
                SourceErrorType::PermissionDenied,
                SourceErrorType::NetworkError,
                SourceErrorType::ServerError,
                SourceErrorType::PathTooLong,
                SourceErrorType::InvalidCharacters,
                SourceErrorType::TooManyItems,
                SourceErrorType::DepthLimit,
                SourceErrorType::SizeLimit,
                SourceErrorType::XmlParseError,
                SourceErrorType::JsonParseError,
                SourceErrorType::QuotaExceeded,
                SourceErrorType::RateLimited,
                SourceErrorType::NotFound,
                SourceErrorType::Conflict,
                SourceErrorType::UnsupportedOperation,
                SourceErrorType::Unknown,
            ];

            // Create a failure for each combination to ensure all enum values work
            for source_type in &source_types {
                for (i, error_type) in error_types.iter().enumerate() {
                    let failure = CreateSourceScanFailure {
                        user_id,
                        source_type: source_type.clone(),
                        source_id: None,
                        resource_path: format!("/test/enum/{}/{}.pdf", source_type, i),
                        error_type: error_type.clone(),
                        error_message: format!("Testing {} with {}", source_type, error_type),
                        error_code: None,
                        http_status_code: None,
                        response_time_ms: None,
                        response_size_bytes: None,
                        resource_size_bytes: None,
                        diagnostic_data: None,
                    };
                    
                    // This should not panic or return an error
                    let result = db.record_source_scan_failure(&failure).await;
                    assert!(result.is_ok(), "Failed to record failure for {} / {}: {:?}", 
                            source_type, error_type, result);
                }
            }

            // Verify we can query all of them
            let all_failures = db.list_source_scan_failures(user_id, &ListFailuresQuery {
                source_type: None,
                source_id: None,
                error_type: None,
                severity: None,
                include_resolved: Some(false),
                include_excluded: Some(false),
                ready_for_retry: None,
                limit: None,
                offset: None,
            }).await?;

            assert_eq!(all_failures.len(), source_types.len() * error_types.len());

            Ok(())
        }.await;
        
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }
}