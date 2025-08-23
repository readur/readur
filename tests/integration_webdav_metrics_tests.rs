use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

use readur::{
    db::Database,
    models::webdav_metrics::*,
    models::{CreateUser, UserRole},
    services::webdav_metrics_tracker::WebDAVMetricsTracker,
    test_helpers::create_test_app_state,
};

/// Helper to create a test user using the proper models
async fn create_test_user(db: &Database) -> Result<Uuid> {
    let user_suffix = Uuid::new_v4().simple().to_string();
    let create_user = CreateUser {
        username: format!("testuser_{}", user_suffix),
        email: format!("test_{}@example.com", user_suffix),
        password: "test_password".to_string(),
        role: Some(UserRole::User),
    };
    
    let created_user = db.create_user(create_user).await?;
    Ok(created_user.id)
}

/// Helper to create a test WebDAV source
async fn create_test_source(db: &Database, user_id: Uuid) -> Result<Uuid> {
    let source_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO sources (id, user_id, name, source_type, config, enabled, created_at, updated_at) 
         VALUES ($1, $2, $3, 'webdav', $4, true, NOW(), NOW())"
    )
    .bind(source_id)
    .bind(user_id)
    .bind(format!("Test WebDAV Source {}", source_id))
    .bind(serde_json::json!({
        "server_url": "https://example.com/webdav",
        "username": "testuser",
        "password": "testpass",
        "watch_folders": ["/Documents"],
        "file_extensions": ["pdf", "txt", "doc", "docx"],
        "auto_sync": true,
        "sync_interval_minutes": 60
    }))
    .execute(&db.pool)
    .await?;
    
    Ok(source_id)
}

/// Test basic session creation and management
#[tokio::test]
async fn test_webdav_session_lifecycle() -> Result<()> {
    let app_state = create_test_app_state().await
        .map_err(|e| anyhow::anyhow!("Failed to create test app state: {}", e))?;
    let user_id = create_test_user(&app_state.db).await?;
    let source_id = Some(create_test_source(&app_state.db, user_id).await?);
    
    let metrics_tracker = WebDAVMetricsTracker::new(app_state.db.clone());
    
    // Start a sync session
    let session_id = metrics_tracker
        .start_session(
            user_id,
            source_id,
            "full_sync".to_string(),
            "/Documents".to_string(),
            Some(10),
        )
        .await?;
    
    // Update session counters
    metrics_tracker
        .update_session_counters(
            session_id,
            5,  // directories_discovered
            3,  // directories_processed  
            20, // files_discovered
            15, // files_processed
            1024 * 1024, // bytes_discovered (1MB)
            512 * 1024,  // bytes_processed (512KB)
        )
        .await?;
    
    // Record some skipped items
    metrics_tracker
        .record_skipped_items(
            session_id,
            1, // directories_skipped
            2, // files_skipped
            "permission_denied",
        )
        .await?;
    
    // Finish the session
    metrics_tracker
        .finish_session(
            session_id,
            WebDAVSyncStatus::Completed,
            None,
        )
        .await?;
    
    // Verify session was recorded correctly
    let session = metrics_tracker
        .get_session_details(session_id, user_id)
        .await?
        .expect("Session should exist");
    
    assert_eq!(session.user_id, user_id);
    assert_eq!(session.source_id, source_id);
    assert_eq!(session.sync_type, "full_sync");
    assert_eq!(session.root_path, "/Documents");
    assert_eq!(session.directories_discovered, 5);
    assert_eq!(session.directories_processed, 3);
    assert_eq!(session.files_discovered, 20);
    assert_eq!(session.files_processed, 15);
    assert_eq!(session.directories_skipped, 1);
    assert_eq!(session.files_skipped, 2);
    assert_eq!(session.status, "completed");
    assert!(session.duration_ms.is_some());
    
    println!("✅ Session lifecycle test passed");
    Ok(())
}

/// Test directory metrics tracking
#[tokio::test]
async fn test_directory_metrics_tracking() -> Result<()> {
    let app_state = create_test_app_state().await
        .map_err(|e| anyhow::anyhow!("Failed to create test app state: {}", e))?;
    let user_id = create_test_user(&app_state.db).await?;
    let source_id = Some(create_test_source(&app_state.db, user_id).await?);
    
    let metrics_tracker = WebDAVMetricsTracker::new(app_state.db.clone());
    
    // Start session
    let session_id = metrics_tracker
        .start_session(
            user_id,
            source_id,
            "incremental_sync".to_string(),
            "/Photos".to_string(),
            Some(5),
        )
        .await?;
    
    // Start directory scan
    let dir_metric_id = metrics_tracker
        .start_directory_scan(
            session_id,
            user_id,
            source_id,
            "/Photos/2023".to_string(),
            2,
            Some("/Photos".to_string()),
        )
        .await?;
    
    // Update directory counters
    metrics_tracker
        .update_directory_counters(
            dir_metric_id,
            10, // files_found
            2,  // subdirectories_found
            5 * 1024 * 1024, // size_bytes (5MB)
            8,  // files_processed
            1,  // files_skipped
            1,  // files_failed
        )
        .await?;
    
    // Record some errors and warnings
    metrics_tracker
        .record_directory_error(dir_metric_id, "timeout", false)
        .await?;
    
    metrics_tracker
        .record_directory_error(dir_metric_id, "large_file", true)
        .await?;
    
    // Record ETag results
    metrics_tracker
        .record_etag_result(dir_metric_id, true, true)
        .await?;
    
    metrics_tracker
        .record_etag_result(dir_metric_id, false, false)
        .await?;
    
    // Finish directory scan
    metrics_tracker
        .finish_directory_scan(
            dir_metric_id,
            "completed",
            None,
            None,
        )
        .await?;
    
    // Finish session
    metrics_tracker
        .finish_session(session_id, WebDAVSyncStatus::Completed, None)
        .await?;
    
    // Verify directory metrics
    let dir_metrics = metrics_tracker
        .get_directory_metrics(session_id, user_id)
        .await?;
    
    assert_eq!(dir_metrics.len(), 1);
    let dir_metric = &dir_metrics[0];
    
    assert_eq!(dir_metric.directory_path, "/Photos/2023");
    assert_eq!(dir_metric.directory_depth, 2);
    assert_eq!(dir_metric.files_found, 10);
    assert_eq!(dir_metric.subdirectories_found, 2);
    assert_eq!(dir_metric.files_processed, 8);
    assert_eq!(dir_metric.files_skipped, 1);
    assert_eq!(dir_metric.files_failed, 1);
    assert_eq!(dir_metric.errors_encountered, 1);
    assert_eq!(dir_metric.warnings_count, 1);
    assert_eq!(dir_metric.etag_matches, 1);
    assert_eq!(dir_metric.etag_mismatches, 1);
    assert_eq!(dir_metric.cache_hits, 1);
    assert_eq!(dir_metric.cache_misses, 1);
    assert!(dir_metric.scan_duration_ms.is_some());
    
    println!("✅ Directory metrics test passed");
    Ok(())
}

/// Test HTTP request metrics recording
#[tokio::test]
async fn test_http_request_metrics() -> Result<()> {
    let app_state = create_test_app_state().await
        .map_err(|e| anyhow::anyhow!("Failed to create test app state: {}", e))?;
    let user_id = create_test_user(&app_state.db).await?;
    let source_id = Some(create_test_source(&app_state.db, user_id).await?);
    
    let metrics_tracker = WebDAVMetricsTracker::new(app_state.db.clone());
    
    // Start session and directory
    let session_id = metrics_tracker
        .start_session(user_id, source_id, "test_sync".to_string(), "/".to_string(), None)
        .await?;
    
    let dir_metric_id = metrics_tracker
        .start_directory_scan(session_id, user_id, source_id, "/test".to_string(), 1, None)
        .await?;
    
    // Record successful PROPFIND request
    let request_id_1 = metrics_tracker
        .record_http_request(
            Some(session_id),
            Some(dir_metric_id),
            user_id,
            source_id,
            WebDAVRequestType::PropFind,
            WebDAVOperationType::Discovery,
            "/test".to_string(),
            Duration::from_millis(250),
            Some(512),
            Some(2048),
            Some(207), // Multi-Status
            true,
            0,
            None,
            None,
            None,
            Some("192.168.1.100".to_string()),
        )
        .await?;
    
    // Record failed GET request
    let request_id_2 = metrics_tracker
        .record_http_request(
            Some(session_id),
            Some(dir_metric_id),
            user_id,
            source_id,
            WebDAVRequestType::Get,
            WebDAVOperationType::Download,
            "/test/file.pdf".to_string(),
            Duration::from_millis(5000),
            None,
            None,
            Some(404),
            false,
            1, // retry attempt
            Some("not_found".to_string()),
            Some("File not found".to_string()),
            None,
            Some("192.168.1.100".to_string()),
        )
        .await?;
    
    // Finish directory and session
    metrics_tracker
        .finish_directory_scan(dir_metric_id, "completed", None, None)
        .await?;
    
    metrics_tracker
        .finish_session(session_id, WebDAVSyncStatus::Completed, None)
        .await?;
    
    // Verify request metrics
    let request_metrics = metrics_tracker
        .get_request_metrics(Some(session_id), None, user_id, Some(10))
        .await?;
    
    assert_eq!(request_metrics.len(), 2);
    
    // Find the PROPFIND request
    let propfind_request = request_metrics
        .iter()
        .find(|r| r.request_type == "PROPFIND")
        .expect("Should find PROPFIND request");
    
    assert_eq!(propfind_request.operation_type, "discovery");
    assert_eq!(propfind_request.target_path, "/test");
    assert_eq!(propfind_request.duration_ms, 250);
    assert_eq!(propfind_request.request_size_bytes, Some(512));
    assert_eq!(propfind_request.response_size_bytes, Some(2048));
    assert_eq!(propfind_request.http_status_code, Some(207));
    assert!(propfind_request.success);
    assert_eq!(propfind_request.retry_attempt, 0);
    
    // Find the GET request
    let get_request = request_metrics
        .iter()
        .find(|r| r.request_type == "GET")
        .expect("Should find GET request");
    
    assert_eq!(get_request.operation_type, "download");
    assert_eq!(get_request.target_path, "/test/file.pdf");
    assert_eq!(get_request.duration_ms, 5000);
    assert_eq!(get_request.http_status_code, Some(404));
    assert!(!get_request.success);
    assert_eq!(get_request.retry_attempt, 1);
    assert_eq!(get_request.error_type, Some("not_found".to_string()));
    
    println!("✅ HTTP request metrics test passed");
    Ok(())
}

/// Test metrics summary generation
#[tokio::test]
async fn test_metrics_summary() -> Result<()> {
    let app_state = create_test_app_state().await
        .map_err(|e| anyhow::anyhow!("Failed to create test app state: {}", e))?;
    let user_id = create_test_user(&app_state.db).await?;
    let source_id = Some(create_test_source(&app_state.db, user_id).await?);
    
    let metrics_tracker = WebDAVMetricsTracker::new(app_state.db.clone());
    
    // Create multiple sessions with various outcomes
    for i in 0..3 {
        let session_id = metrics_tracker
            .start_session(
                user_id,
                source_id,
                format!("test_sync_{}", i),
                format!("/test_{}", i),
                None,
            )
            .await?;
        
        // Update counters
        metrics_tracker
            .update_session_counters(
                session_id,
                5, 5, 10, 10,
                1024 * (i + 1) as i64, // Different sizes for each
                512 * (i + 1) as i64,
            )
            .await?;
        
        // Record some requests
        for j in 0..5 {
            let success = i != 2 || j < 3; // Make last session partially fail
            let status_code = if success { Some(200) } else { Some(500) };
            
            metrics_tracker
                .record_http_request(
                    Some(session_id),
                    None,
                    user_id,
                    source_id,
                    WebDAVRequestType::Get,
                    WebDAVOperationType::Download,
                    format!("/test_{}/file_{}", i, j),
                    Duration::from_millis(100 * (j + 1) as u64),
                    None,
                    Some(1024),
                    status_code,
                    success,
                    0,
                    if !success { Some("server_error".to_string()) } else { None },
                    if !success { Some("Internal server error".to_string()) } else { None },
                    None,
                    None,
                )
                .await?;
        }
        
        let status = if i == 2 { WebDAVSyncStatus::Failed } else { WebDAVSyncStatus::Completed };
        metrics_tracker
            .finish_session(session_id, status, None)
            .await?;
    }
    
    // Get metrics summary
    let query = WebDAVMetricsQuery {
        user_id: Some(user_id),
        source_id,
        start_time: Some(chrono::Utc::now() - chrono::Duration::hours(1)),
        end_time: Some(chrono::Utc::now()),
        limit: None,
        offset: None,
    };
    
    let summary = metrics_tracker
        .get_metrics_summary(&query)
        .await?
        .expect("Should have summary data");
    
    assert_eq!(summary.total_sessions, 3);
    assert_eq!(summary.successful_sessions, 2);
    assert_eq!(summary.failed_sessions, 1);
    assert_eq!(summary.total_files_processed, 30); // 10 files per session
    assert_eq!(summary.total_http_requests, 15); // 5 requests per session
    assert!(summary.request_success_rate > 0.0);
    assert!(summary.avg_request_duration_ms > 0.0);
    
    println!("✅ Metrics summary test passed");
    println!("Summary: {} total sessions, {} successful, {} failed", 
             summary.total_sessions, summary.successful_sessions, summary.failed_sessions);
    println!("Success rate: {:.1}%, Avg request time: {:.0}ms",
             summary.request_success_rate,
             summary.avg_request_duration_ms);
    
    Ok(())
}

/// Test performance insights generation
#[tokio::test]
async fn test_performance_insights() -> Result<()> {
    let app_state = create_test_app_state().await
        .map_err(|e| anyhow::anyhow!("Failed to create test app state: {}", e))?;
    let user_id = create_test_user(&app_state.db).await?;
    let source_id = Some(create_test_source(&app_state.db, user_id).await?);
    
    let metrics_tracker = WebDAVMetricsTracker::new(app_state.db.clone());
    
    // Create a session with detailed metrics
    let session_id = metrics_tracker
        .start_session(user_id, source_id, "performance_test".to_string(), "/perf".to_string(), None)
        .await?;
    
    // Create multiple directories with different performance characteristics
    let dir_paths = ["/perf/fast", "/perf/slow", "/perf/medium"];
    let scan_times = [100, 5000, 1000]; // milliseconds
    
    for (path, scan_time) in dir_paths.iter().zip(scan_times.iter()) {
        let dir_metric_id = metrics_tracker
            .start_directory_scan(session_id, user_id, source_id, path.to_string(), 2, Some("/perf".to_string()))
            .await?;
        
        // Simulate directory processing
        tokio::time::sleep(Duration::from_millis(*scan_time as u64 / 10)).await; // Reduce for test speed
        
        metrics_tracker
            .update_directory_counters(dir_metric_id, 5, 1, 2048, 5, 0, 0)
            .await?;
        
        // Record some requests for this directory
        for i in 0..3 {
            metrics_tracker
                .record_http_request(
                    Some(session_id),
                    Some(dir_metric_id),
                    user_id,
                    source_id,
                    if i == 0 { WebDAVRequestType::PropFind } else { WebDAVRequestType::Get },
                    if i == 0 { WebDAVOperationType::Discovery } else { WebDAVOperationType::Download },
                    format!("{}/file_{}", path, i),
                    Duration::from_millis(*scan_time as u64 / 3),
                    None,
                    Some(1024),
                    Some(200),
                    true,
                    0,
                    None,
                    None,
                    None,
                    None,
                )
                .await?;
        }
        
        metrics_tracker
            .finish_directory_scan(dir_metric_id, "completed", None, None)
            .await?;
    }
    
    metrics_tracker
        .finish_session(session_id, WebDAVSyncStatus::Completed, None)
        .await?;
    
    // Get performance insights
    let insights = metrics_tracker
        .get_performance_insights(session_id, user_id)
        .await?
        .expect("Should have performance insights");
    
    assert_eq!(insights.session_id, session_id);
    assert!(insights.avg_directory_scan_time_ms > 0.0);
    assert_eq!(insights.slowest_directories.len(), 3);
    
    // Verify slowest directory is at the top
    let slowest = &insights.slowest_directories[0];
    assert_eq!(slowest.path, "/perf/slow");
    
    // Verify request distribution
    assert_eq!(insights.request_distribution.total_count, 9); // 3 requests per directory
    assert_eq!(insights.request_distribution.propfind_count, 3); // 1 per directory
    assert_eq!(insights.request_distribution.get_count, 6); // 2 per directory
    
    println!("✅ Performance insights test passed");
    println!("Avg directory scan time: {:.1}ms", insights.avg_directory_scan_time_ms);
    println!("Slowest directory: {} ({}ms)", 
             slowest.path, slowest.scan_duration_ms);
    
    Ok(())
}

/// Integration test demonstrating the complete metrics collection workflow
#[tokio::test]
async fn test_complete_metrics_workflow() -> Result<()> {
    let app_state = create_test_app_state().await
        .map_err(|e| anyhow::anyhow!("Failed to create test app state: {}", e))?;
    let user_id = create_test_user(&app_state.db).await?;
    let source_id = Some(create_test_source(&app_state.db, user_id).await?);
    
    let metrics_tracker = WebDAVMetricsTracker::new(app_state.db.clone());
    
    println!("🚀 Starting complete WebDAV metrics workflow test");
    
    // Step 1: Start sync session
    let session_id = metrics_tracker
        .start_session(
            user_id,
            source_id,
            "complete_test".to_string(),
            "/Documents".to_string(),
            Some(10),
        )
        .await?;
    
    println!("📊 Session {} started", session_id);
    
    // Step 2: Simulate directory discovery and processing
    let directories = [
        ("/Documents", 0),
        ("/Documents/2023", 1),
        ("/Documents/2023/Reports", 2),
        ("/Documents/2024", 1),
    ];
    
    let mut total_files = 0;
    let mut total_bytes = 0i64;
    
    for (dir_path, depth) in directories.iter() {
        let parent = if *depth == 0 {
            None
        } else {
            dir_path.rfind('/').map(|pos| dir_path[..pos].to_string())
        };
        
        let dir_metric_id = metrics_tracker
            .start_directory_scan(
                session_id,
                user_id,
                source_id,
                dir_path.to_string(),
                *depth,
                parent,
            )
            .await?;
        
        // Simulate discovery request
        let discovery_duration = Duration::from_millis(150 + *depth as u64 * 50);
        let files_in_dir = 3 + *depth;
        let bytes_in_dir = (files_in_dir as i64) * 1024 * 256; // 256KB per file
        
        metrics_tracker
            .record_http_request(
                Some(session_id),
                Some(dir_metric_id),
                user_id,
                source_id,
                WebDAVRequestType::PropFind,
                WebDAVOperationType::Discovery,
                dir_path.to_string(),
                discovery_duration,
                Some(512),
                Some(2048),
                Some(207),
                true,
                0,
                None,
                None,
                None,
                None,
            )
            .await?;
        
        // Update directory counters with discovery results
        metrics_tracker
            .update_directory_counters(
                dir_metric_id,
                files_in_dir,
                1, // subdirectories
                bytes_in_dir,
                0, // files_processed (will update later)
                0, // files_skipped
                0, // files_failed
            )
            .await?;
        
        // Simulate file downloads
        for file_idx in 0..files_in_dir {
            let file_path = format!("{}/file_{}.pdf", dir_path, file_idx);
            let download_duration = Duration::from_millis(200 + file_idx as u64 * 100);
            let file_size = 256 * 1024; // 256KB
            
            let success = file_idx < files_in_dir - 1; // Last file fails
            let status_code = if success { Some(200) } else { Some(404) };
            
            metrics_tracker
                .record_http_request(
                    Some(session_id),
                    Some(dir_metric_id),
                    user_id,
                    source_id,
                    WebDAVRequestType::Get,
                    WebDAVOperationType::Download,
                    file_path,
                    download_duration,
                    None,
                    if success { Some(file_size) } else { None },
                    status_code,
                    success,
                    0,
                    if !success { Some("not_found".to_string()) } else { None },
                    if !success { Some("File not found".to_string()) } else { None },
                    None,
                    None,
                )
                .await?;
            
            if success {
                // Update counters for successful download
                metrics_tracker
                    .update_directory_counters(
                        dir_metric_id,
                        0, 0, 0, // no change to discovery counts
                        1, // files_processed
                        0, // files_skipped
                        0, // files_failed
                    )
                    .await?;
                
                total_files += 1;
                total_bytes += file_size;
            } else {
                // Update counters for failed download
                metrics_tracker
                    .update_directory_counters(
                        dir_metric_id,
                        0, 0, 0, // no change to discovery counts
                        0, // files_processed
                        0, // files_skipped
                        1, // files_failed
                    )
                    .await?;
                
                metrics_tracker
                    .record_directory_error(dir_metric_id, "file_not_found", false)
                    .await?;
            }
        }
        
        // Record ETag activity
        metrics_tracker
            .record_etag_result(dir_metric_id, true, true)
            .await?;
        
        // Finish directory scan
        metrics_tracker
            .finish_directory_scan(dir_metric_id, "completed", None, None)
            .await?;
        
        println!("📁 Processed directory {} with {} files", dir_path, files_in_dir);
    }
    
    // Step 3: Update session with final counts
    metrics_tracker
        .update_session_counters(
            session_id,
            directories.len() as i32,
            directories.len() as i32,
            total_files,
            total_files - 4, // Subtract failed files
            total_bytes,
            total_bytes - (4 * 256 * 1024), // Subtract failed file bytes
        )
        .await?;
    
    // Step 4: Finish session
    metrics_tracker
        .finish_session(session_id, WebDAVSyncStatus::Completed, None)
        .await?;
    
    println!("✅ Session completed successfully");
    
    // Step 5: Verify all metrics were recorded correctly
    
    // Check session details
    let session = metrics_tracker
        .get_session_details(session_id, user_id)
        .await?
        .expect("Session should exist");
    
    assert_eq!(session.status, "completed");
    assert!(session.duration_ms.is_some());
    assert!(session.total_http_requests > 0);
    assert!(session.successful_requests > 0);
    assert!(session.failed_requests > 0);
    
    // Check directory metrics
    let dir_metrics = metrics_tracker
        .get_directory_metrics(session_id, user_id)
        .await?;
    
    assert_eq!(dir_metrics.len(), directories.len());
    
    // Check request metrics
    let request_metrics = metrics_tracker
        .get_request_metrics(Some(session_id), None, user_id, None)
        .await?;
    
    assert!(request_metrics.len() > 0);
    let propfind_count = request_metrics.iter().filter(|r| r.request_type == "PROPFIND").count();
    let get_count = request_metrics.iter().filter(|r| r.request_type == "GET").count();
    assert_eq!(propfind_count, directories.len());
    assert!(get_count > 0);
    
    // Check performance insights
    let insights = metrics_tracker
        .get_performance_insights(session_id, user_id)
        .await?
        .expect("Should have insights");
    
    assert_eq!(insights.slowest_directories.len(), directories.len());
    assert!(insights.request_distribution.total_count > 0);
    assert!(insights.error_analysis.total_errors > 0);
    
    // Check summary metrics
    let query = WebDAVMetricsQuery {
        user_id: Some(user_id),
        source_id,
        start_time: Some(chrono::Utc::now() - chrono::Duration::hours(1)),
        end_time: Some(chrono::Utc::now()),
        limit: None,
        offset: None,
    };
    
    let summary = metrics_tracker
        .get_metrics_summary(&query)
        .await?
        .expect("Should have summary");
    
    assert_eq!(summary.total_sessions, 1);
    assert_eq!(summary.successful_sessions, 1);
    
    println!("📈 Metrics Summary:");
    println!("  - Sessions: {} total, {} successful", summary.total_sessions, summary.successful_sessions);
    println!("  - Files: {} processed", summary.total_files_processed);
    println!("  - Requests: {} total, {:.1}% success rate", 
             summary.total_http_requests, summary.request_success_rate);
    println!("  - Performance: {:.0}ms avg request time", 
             summary.avg_request_duration_ms);
    
    println!("🎉 Complete metrics workflow test passed!");
    
    Ok(())
}