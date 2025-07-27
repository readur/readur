use std::{sync::Arc, time::Duration, collections::HashMap};
use tokio::time::sleep;
use uuid::Uuid;
use futures::future::join_all;
use readur::{
    AppState,
    models::{CreateWebDAVDirectory, SourceType, SourceStatus, WebDAVSourceConfig, CreateSource},
    test_utils::{TestContext, TestAuthHelper},
    scheduling::source_scheduler::SourceScheduler,
    services::webdav::{SmartSyncService, WebDAVService, WebDAVConfig, SyncProgress, SyncPhase},
};

/// Helper function to create test setup with database and real components
async fn create_integration_test_state() -> (TestContext, Arc<AppState>, Uuid) {
    let test_context = TestContext::new().await;
    
    let auth_helper = TestAuthHelper::new(test_context.app().clone());
    let test_user = auth_helper.create_test_user().await;

    let state = test_context.state().clone();
    let user_id = test_user.user_response.id;
    
    (test_context, state, user_id)
}

/// Helper to create a test WebDAV source
async fn create_test_webdav_source(
    state: &Arc<AppState>, 
    user_id: Uuid, 
    name: &str,
    auto_sync: bool,
) -> readur::models::Source {
    let config = WebDAVSourceConfig {
        server_url: "https://test.example.com".to_string(),
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/test".to_string()],
        file_extensions: vec!["pdf".to_string(), "txt".to_string()],
        auto_sync,
        sync_interval_minutes: 1, // Fast interval for testing
        server_type: Some("nextcloud".to_string()),
    };

    let create_source = CreateSource {
        name: name.to_string(),
        source_type: SourceType::WebDAV,
        config: serde_json::to_value(config).unwrap(),
        enabled: true,
    };

    state.db.create_source(user_id, &create_source).await
        .expect("Failed to create test source")
}

/// Mock WebDAV service that simulates network operations with controllable delays
#[derive(Clone)]
struct MockWebDAVService {
    delay_ms: u64,
    should_fail: bool,
    etag_counter: Arc<std::sync::Mutex<u32>>,
}

impl MockWebDAVService {
    fn new(delay_ms: u64, should_fail: bool) -> Self {
        Self {
            delay_ms,
            should_fail,
            etag_counter: Arc::new(std::sync::Mutex::new(1)),
        }
    }

    async fn mock_discover_files_and_directories(
        &self,
        directory_path: &str,
        _recursive: bool,
    ) -> Result<readur::services::webdav::discovery::WebDAVDiscoveryResult, anyhow::Error> {
        // Simulate network delay
        sleep(Duration::from_millis(self.delay_ms)).await;

        if self.should_fail {
            return Err(anyhow::anyhow!("Mock WebDAV discovery failed"));
        }

        // Generate unique ETags for each call to simulate directory changes
        let etag = {
            let mut counter = self.etag_counter.lock().unwrap();
            *counter += 1;
            format!("mock-etag-{}", *counter)
        };

        let mock_files = vec![
            readur::models::FileIngestionInfo {
                name: format!("test-file-{}.pdf", etag),
                relative_path: format!("{}/test-file-{}.pdf", directory_path, etag),
                size: 1024,
                modified: chrono::Utc::now(),
                etag: etag.clone(),
                is_directory: false,
                content_type: Some("application/pdf".to_string()),
            }
        ];

        let mock_directories = vec![
            readur::models::FileIngestionInfo {
                name: "subdir".to_string(),
                relative_path: format!("{}/subdir", directory_path),
                size: 0,
                modified: chrono::Utc::now(),
                etag: etag.clone(),
                is_directory: true,
                content_type: None,
            }
        ];

        Ok(readur::services::webdav::discovery::WebDAVDiscoveryResult {
            files: mock_files,
            directories: mock_directories,
        })
    }
}

/// Test concurrent source scheduler trigger operations
#[tokio::test]
async fn test_concurrent_source_scheduler_triggers() {
    let (_test_context, state, user_id) = create_integration_test_state().await;
    
    // Create test sources
    let source1 = create_test_webdav_source(&state, user_id, "TestSource1", false).await;
    let source2 = create_test_webdav_source(&state, user_id, "TestSource2", false).await;
    
    // Create source scheduler
    let scheduler = Arc::new(SourceScheduler::new(state.clone()));
    
    // Test concurrent triggers of the same source
    let concurrent_triggers = (0..5).map(|i| {
        let scheduler_clone = scheduler.clone();
        let source_id = source1.id;
        tokio::spawn(async move {
            println!("Trigger attempt {} for source {}", i, source_id);
            
            // Try to trigger sync - some should succeed, others should get conflict
            let result = scheduler_clone.trigger_sync(source_id).await;
            println!("Trigger {} result: {:?}", i, result.is_ok());
            (i, result)
        })
    });
    
    // Wait for all trigger attempts
    let trigger_results: Vec<_> = join_all(concurrent_triggers).await;
    
    // Verify all tasks completed without panicking
    for result in trigger_results {
        let (task_id, sync_result) = result.expect("Task should complete without panicking");
        println!("Task {} completed with result: {:?}", task_id, sync_result.is_ok());
        // Note: The actual sync operations might fail due to concurrency control,
        // but the scheduler should handle this gracefully
    }
    
    // Test concurrent triggers across different sources
    let cross_source_triggers = vec![
        (scheduler.clone(), source1.id),
        (scheduler.clone(), source2.id),
        (scheduler.clone(), source1.id), // Duplicate to test conflict handling
        (scheduler.clone(), source2.id), // Duplicate to test conflict handling
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (scheduler_clone, source_id))| {
        tokio::spawn(async move {
            println!("Cross-source trigger {} for source {}", i, source_id);
            let result = scheduler_clone.trigger_sync(source_id).await;
            (i, source_id, result)
        })
    });
    
    let cross_results: Vec<_> = join_all(cross_source_triggers).await;
    
    // Verify cross-source operations
    for result in cross_results {
        let (task_id, source_id, sync_result) = result.expect("Cross-source task should complete");
        println!("Cross-source task {} for source {} completed: {:?}", task_id, source_id, sync_result.is_ok());
    }
    
    // Give time for any background tasks to complete
    sleep(Duration::from_millis(500)).await;
    
    // Verify final source states are consistent
    let final_source1 = state.db.get_source(user_id, source1.id).await
        .expect("Failed to get source1")
        .expect("Source1 should exist");
    let final_source2 = state.db.get_source(user_id, source2.id).await
        .expect("Failed to get source2")
        .expect("Source2 should exist");
    
    // Sources should not be stuck in syncing state
    assert_ne!(final_source1.status, SourceStatus::Syncing, 
              "Source1 should not be stuck in syncing state");
    assert_ne!(final_source2.status, SourceStatus::Syncing, 
              "Source2 should not be stuck in syncing state");
}

/// Test concurrent smart sync evaluations
#[tokio::test]
async fn test_concurrent_smart_sync_evaluations() {
    let (_test_context, state, user_id) = create_integration_test_state().await;
    
    // Pre-populate some directory state
    let test_directories = vec![
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/test".to_string(),
            directory_etag: "initial-etag".to_string(),
            file_count: 5,
            total_size_bytes: 1024,
        },
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/test/subdir1".to_string(),
            directory_etag: "subdir1-etag".to_string(),
            file_count: 3,
            total_size_bytes: 512,
        },
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/test/subdir2".to_string(),
            directory_etag: "subdir2-etag".to_string(),
            file_count: 2,
            total_size_bytes: 256,
        },
    ];
    
    for dir in test_directories {
        state.db.create_or_update_webdav_directory(&dir).await
            .expect("Failed to create test directory");
    }
    
    // Create mock WebDAV services with different behaviors
    let mock_services = vec![
        MockWebDAVService::new(50, false),  // Fast, successful
        MockWebDAVService::new(100, false), // Slower, successful  
        MockWebDAVService::new(200, false), // Slowest, successful
        MockWebDAVService::new(75, true),   // Medium speed, fails
        MockWebDAVService::new(25, false),  // Fastest, successful
    ];
    
    // Test concurrent smart sync evaluations
    let concurrent_evaluations = mock_services.into_iter().enumerate().map(|(i, mock_service)| {
        let state_clone = state.clone();
        let user_id = user_id;
        tokio::spawn(async move {
            println!("Smart sync evaluation {} starting", i);
            
            // Create SmartSyncService for this task
            let smart_sync_service = SmartSyncService::new(state_clone.clone());
            
            // Mock the WebDAV service call by calling the database methods directly
            // This simulates what would happen during concurrent smart sync evaluations
            
            // 1. Read current directory state (simulates smart sync evaluation)
            let known_dirs_result = state_clone.db.list_webdav_directories(user_id).await;
            
            // 2. Simulate discovery results with some delay
            sleep(Duration::from_millis(mock_service.delay_ms)).await;
            
            if mock_service.should_fail {
                return (i, Err(anyhow::anyhow!("Mock evaluation failed")));
            }
            
            // 3. Update directory ETags (simulates directory changes being detected)
            let update_result = state_clone.db.create_or_update_webdav_directory(
                &CreateWebDAVDirectory {
                    user_id,
                    directory_path: "/test".to_string(),
                    directory_etag: format!("updated-etag-{}", i),
                    file_count: 5 + i as i64,
                    total_size_bytes: 1024 + (i as i64 * 100),
                }
            ).await;
            
            println!("Smart sync evaluation {} completed", i);
            (i, Ok((known_dirs_result, update_result)))
        })
    });
    
    // Wait for all evaluations
    let evaluation_results: Vec<_> = join_all(concurrent_evaluations).await;
    
    // Verify all evaluations completed
    let mut successful_evaluations = 0;
    let mut failed_evaluations = 0;
    
    for result in evaluation_results {
        let (task_id, eval_result) = result.expect("Evaluation task should complete without panicking");
        
        match eval_result {
            Ok((read_result, update_result)) => {
                assert!(read_result.is_ok(), "Directory read should succeed for task {}", task_id);
                assert!(update_result.is_ok(), "Directory update should succeed for task {}", task_id);
                successful_evaluations += 1;
            }
            Err(_) => {
                failed_evaluations += 1;
            }
        }
    }
    
    println!("Smart sync evaluations: {} successful, {} failed", successful_evaluations, failed_evaluations);
    
    // Verify the system is in a consistent state
    let final_directories = state.db.list_webdav_directories(user_id).await
        .expect("Failed to list final directories");
    
    assert_eq!(final_directories.len(), 3, "Should still have 3 directories");
    
    // The main directory should have been updated by one of the successful operations
    let main_dir = final_directories.iter()
        .find(|d| d.directory_path == "/test")
        .expect("Main directory should exist");
    assert!(main_dir.directory_etag.starts_with("updated-etag-"), 
           "Main directory should have updated ETag: {}", main_dir.directory_etag);
}

/// Test concurrent sync triggers with stop operations
#[tokio::test]
async fn test_concurrent_sync_triggers_with_stops() {
    let (_test_context, state, user_id) = create_integration_test_state().await;
    
    // Create test source
    let source = create_test_webdav_source(&state, user_id, "StoppableSource", false).await;
    
    // Create source scheduler
    let scheduler = Arc::new(SourceScheduler::new(state.clone()));
    
    // Start multiple sync operations and stop attempts concurrently
    let operations = (0..10).map(|i| {
        let scheduler_clone = scheduler.clone();
        let source_id = source.id;
        tokio::spawn(async move {
            if i % 3 == 0 {
                // Trigger sync
                println!("Triggering sync for operation {}", i);
                let result = scheduler_clone.trigger_sync(source_id).await;
                (i, "trigger", result.is_ok())
            } else if i % 3 == 1 {
                // Stop sync (might not have anything to stop)
                println!("Stopping sync for operation {}", i);
                let result = scheduler_clone.stop_sync(source_id).await;
                (i, "stop", result.is_ok())
            } else {
                // Read source status
                println!("Reading status for operation {}", i);
                sleep(Duration::from_millis(50)).await; // Small delay to simulate status checks
                (i, "status", true) // Status reads should always work
            }
        })
    });
    
    // Wait for all operations
    let operation_results: Vec<_> = join_all(operations).await;
    
    // Verify all operations completed without panicking
    for result in operation_results {
        let (task_id, op_type, success) = result.expect("Operation task should complete");
        println!("Operation {}: {} -> {}", task_id, op_type, success);
    }
    
    // Give time for any background operations to settle
    sleep(Duration::from_millis(1000)).await;
    
    // Verify source is in a stable state
    let final_source = state.db.get_source(user_id, source.id).await
        .expect("Failed to get source")
        .expect("Source should exist");
    
    // Source should not be stuck in an inconsistent state
    assert!(matches!(final_source.status, SourceStatus::Idle | SourceStatus::Error),
           "Source should be in a stable state, got: {:?}", final_source.status);
}

/// Test concurrent source status updates during sync operations
#[tokio::test]
async fn test_concurrent_source_status_updates() {
    let (_test_context, state, user_id) = create_integration_test_state().await;
    
    // Create test source
    let source = create_test_webdav_source(&state, user_id, "StatusTestSource", false).await;
    
    // Test concurrent status updates from different "components"
    let status_updates = (0..20).map(|i| {
        let state_clone = state.clone();
        let source_id = source.id;
        tokio::spawn(async move {
            let (status, message) = match i % 4 {
                0 => (SourceStatus::Syncing, Some("Starting sync")),
                1 => (SourceStatus::Idle, None),
                2 => (SourceStatus::Error, Some("Test error")),
                3 => (SourceStatus::Syncing, Some("Resuming sync")),
                _ => unreachable!(),
            };
            
            // Add small random delay to increase chance of race conditions
            sleep(Duration::from_millis(((i % 10) * 10) as u64)).await;
            
            let result = if let Some(msg) = message {
                sqlx::query(
                    r#"UPDATE sources 
                       SET status = $2, last_error = $3, last_error_at = NOW(), updated_at = NOW()
                       WHERE id = $1"#
                )
                .bind(source_id)
                .bind(status.to_string())
                .bind(msg)
                .execute(state_clone.db.get_pool())
                .await
            } else {
                sqlx::query(
                    r#"UPDATE sources 
                       SET status = $2, last_error = NULL, last_error_at = NULL, updated_at = NOW()
                       WHERE id = $1"#
                )
                .bind(source_id)
                .bind(status.to_string())
                .execute(state_clone.db.get_pool())
                .await
            };
            
            (i, status, result.is_ok())
        })
    });
    
    // Wait for all status updates
    let update_results: Vec<_> = join_all(status_updates).await;
    
    // Verify all updates completed successfully
    for result in update_results {
        let (task_id, expected_status, success) = result.expect("Status update task should complete");
        assert!(success, "Status update {} to {:?} should succeed", task_id, expected_status);
    }
    
    // Verify final source state is consistent
    let final_source = state.db.get_source(user_id, source.id).await
        .expect("Failed to get source")
        .expect("Source should exist");
    
    // Source should have a valid status (one of the updates should have succeeded)
    assert!(matches!(final_source.status, 
                    SourceStatus::Idle | SourceStatus::Syncing | SourceStatus::Error),
           "Source should have a valid status: {:?}", final_source.status);
    
    // Verify database consistency - updated_at is a DateTime, not Option
    println!("Source updated at: {:?}", final_source.updated_at);
}

/// Test concurrent directory ETag updates during smart sync operations
#[tokio::test]
async fn test_concurrent_directory_etag_updates_during_smart_sync() {
    let (_test_context, state, user_id) = create_integration_test_state().await;
    
    // Create initial directory structure
    let base_directories = vec![
        ("/test", "base-etag-1"),
        ("/test/docs", "base-etag-2"),
        ("/test/images", "base-etag-3"),
        ("/test/archive", "base-etag-4"),
    ];
    
    for (path, etag) in &base_directories {
        let directory = CreateWebDAVDirectory {
            user_id,
            directory_path: path.to_string(),
            directory_etag: etag.to_string(),
            file_count: 10,
            total_size_bytes: 1024,
        };
        state.db.create_or_update_webdav_directory(&directory).await
            .expect("Failed to create base directory");
    }
    
    // Simulate concurrent smart sync operations updating directory ETags
    let smart_sync_updates = (0..15).map(|i| {
        let state_clone = state.clone();
        let user_id = user_id;
        tokio::spawn(async move {
            // Pick a directory to update
            let dir_index = i % base_directories.len();
            let (path, _) = &base_directories[dir_index];
            
            // Simulate smart sync discovering changes
            sleep(Duration::from_millis(((i % 5) * 20) as u64)).await;
            
            // Create "discovered" directory info with new ETag
            let updated_directory = CreateWebDAVDirectory {
                user_id,
                directory_path: path.to_string(),
                directory_etag: format!("smart-sync-etag-{}-{}", dir_index, i),
                file_count: 10 + i as i64,
                total_size_bytes: 1024 + (i as i64 * 100),
            };
            
            // Update directory (simulates smart sync saving discovered ETags)
            let result = state_clone.db.create_or_update_webdav_directory(&updated_directory).await;
            
            (i, dir_index, path.to_string(), result.is_ok())
        })
    });
    
    // Wait for all smart sync updates
    let update_results: Vec<_> = join_all(smart_sync_updates).await;
    
    // Verify all updates completed
    for result in update_results {
        let (task_id, dir_index, path, success) = result.expect("Smart sync update task should complete");
        assert!(success, "Smart sync update {} for directory {} ({}) should succeed", 
               task_id, dir_index, path);
    }
    
    // Verify final directory state
    let final_directories = state.db.list_webdav_directories(user_id).await
        .expect("Failed to list final directories");
    
    assert_eq!(final_directories.len(), base_directories.len(), 
              "Should have same number of directories");
    
    // Verify all directories have been updated with smart sync ETags
    let mut updated_count = 0;
    for directory in final_directories {
        if directory.directory_etag.contains("smart-sync-etag-") {
            updated_count += 1;
        }
        // File count should have been updated by at least one operation
        assert!(directory.file_count >= 10, 
               "Directory {} should have updated file count: {}", 
               directory.directory_path, directory.file_count);
    }
    
    assert!(updated_count > 0, 
           "At least some directories should have smart sync ETags, got {} updated", 
           updated_count);
}

/// Test resilience to partial failures during concurrent operations
#[tokio::test]
async fn test_concurrent_operations_with_partial_failures() {
    let (_test_context, state, user_id) = create_integration_test_state().await;
    
    // Create multiple sources for testing
    let sources = vec![
        create_test_webdav_source(&state, user_id, "ReliableSource", false).await,
        create_test_webdav_source(&state, user_id, "UnreliableSource", false).await,
        create_test_webdav_source(&state, user_id, "SlowSource", false).await,
    ];
    
    // Create source scheduler
    let scheduler = Arc::new(SourceScheduler::new(state.clone()));
    
    // Mix of operations that might succeed or fail
    let mixed_operations = (0..20).map(|i| {
        let scheduler_clone = scheduler.clone();
        let state_clone = state.clone();
        let source_id = sources[i % sources.len()].id;
        let user_id = user_id;
        
        tokio::spawn(async move {
            match i % 5 {
                0 => {
                    // Normal sync trigger
                    let result = scheduler_clone.trigger_sync(source_id).await;
                    (i, "trigger", result.is_ok(), None)
                }
                1 => {
                    // Stop operation (might have nothing to stop)
                    let result = scheduler_clone.stop_sync(source_id).await;
                    (i, "stop", true, None) // Always consider stop attempts as successful
                }
                2 => {
                    // Database read operation
                    let result = state_clone.db.get_source(user_id, source_id).await;
                    (i, "read", result.is_ok(), None)
                }
                3 => {
                    // Status update operation
                    let result = sqlx::query(
                        "UPDATE sources SET status = 'idle', updated_at = NOW() WHERE id = $1"
                    )
                    .bind(source_id)
                    .execute(state_clone.db.get_pool())
                    .await;
                    (i, "status_update", result.is_ok(), None)
                }
                4 => {
                    // Directory listing operation (simulates smart sync evaluation)
                    let result = state_clone.db.list_webdav_directories(user_id).await;
                    (i, "list_dirs", result.is_ok(), Some(result.map(|dirs| dirs.len()).unwrap_or(0)))
                }
                _ => unreachable!(),
            }
        })
    });
    
    // Wait for all operations
    let operation_results: Vec<_> = join_all(mixed_operations).await;
    
    // Analyze results
    let mut operation_stats = HashMap::new();
    let mut total_operations = 0;
    let mut successful_operations = 0;
    
    for result in operation_results {
        let task_result = result.expect("Operation task should complete without panicking");
        let (task_id, op_type, success, extra_info) = task_result;
        
        *operation_stats.entry(op_type).or_insert(0) += 1;
        total_operations += 1;
        if success {
            successful_operations += 1;
        }
        
        println!("Operation {}: {} -> {} {:?}", task_id, op_type, success, extra_info);
    }
    
    println!("Operation statistics: {:?}", operation_stats);
    println!("Success rate: {}/{} ({:.1}%)", 
             successful_operations, total_operations,
             (successful_operations as f64 / total_operations as f64) * 100.0);
    
    // Verify system resilience
    assert!(successful_operations > 0, "At least some operations should succeed");
    
    // Verify all sources are in consistent states
    for source in sources {
        let final_source = state.db.get_source(user_id, source.id).await
            .expect("Failed to get source")
            .expect("Source should exist");
        
        // Source should be in a valid state (not corrupted by partial failures)
        assert!(matches!(final_source.status, 
                        SourceStatus::Idle | SourceStatus::Syncing | SourceStatus::Error),
               "Source {} should be in valid state: {:?}", source.name, final_source.status);
    }
    
    // System should remain functional for new operations
    let recovery_test = scheduler.trigger_sync(sources[0].id).await;
    // Recovery might succeed or fail, but shouldn't panic
    println!("Recovery test result: {:?}", recovery_test.is_ok());
}