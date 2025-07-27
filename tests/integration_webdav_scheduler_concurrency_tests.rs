use std::{sync::Arc, time::Duration};
use tokio::time::sleep;
use uuid::Uuid;
use futures::future::join_all;
use readur::{
    AppState,
    models::{CreateWebDAVDirectory, SourceType, SourceStatus, WebDAVSourceConfig, CreateSource},
    test_utils::{TestContext, TestAuthHelper},
    scheduling::source_scheduler::SourceScheduler,
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
        enabled: Some(true),
    };

    state.db.create_source(user_id, &create_source).await.unwrap()
}

/// Test: Multiple concurrent sync triggers for the same source
/// This tests the SourceScheduler's ability to prevent duplicate syncs
#[tokio::test]
async fn test_concurrent_sync_triggers_same_source() {
    let (_test_context, state, user_id) = create_integration_test_state().await;
    
    // Create a WebDAV source for testing
    let source = create_test_webdav_source(&state, user_id, "test_source", false).await;
    
    // Create SourceScheduler (this is what the real server uses)
    let scheduler = SourceScheduler::new(state.clone());
    
    // Trigger multiple concurrent syncs for the same source
    let sync_tasks = (0..5).map(|i| {
        let state_clone = state.clone();
        let source_id = source.id;
        tokio::spawn(async move {
            let scheduler = SourceScheduler::new(state_clone);
            let result = scheduler.trigger_sync(source_id).await;
            (i, result)
        })
    });
    
    let results = join_all(sync_tasks).await;
    
    // Verify results
    let mut success_count = 0;
    let mut already_running_count = 0;
    
    for result in results {
        let (task_id, sync_result) = result.unwrap();
        match sync_result {
            Ok(()) => {
                success_count += 1;
                println!("Task {} successfully triggered sync", task_id);
            }
            Err(e) if e.to_string().contains("already") => {
                already_running_count += 1;
                println!("Task {} found sync already running: {}", task_id, e);
            }
            Err(e) => {
                println!("Task {} failed with unexpected error: {}", task_id, e);
            }
        }
    }
    
    // We expect exactly one sync to succeed and others to be rejected
    assert_eq!(success_count, 1, "Exactly one sync should succeed");
    assert!(already_running_count >= 3, "Multiple tasks should find sync already running");
    
    // Verify source status was updated
    let updated_source = state.db.get_source(user_id, source.id).await.unwrap().unwrap();
    // Note: Status might be Syncing or back to Idle depending on timing
    println!("Final source status: {:?}", updated_source.status);
}

/// Test: Concurrent sync operations with source stop requests
/// This tests the interaction between sync triggers and stop operations
#[tokio::test]
async fn test_concurrent_sync_trigger_and_stop() {
    let (_test_context, state, user_id) = create_integration_test_state().await;
    
    let source = create_test_webdav_source(&state, user_id, "stop_test_source", false).await;
    
    // Start sync operations and stop operations concurrently
    let source_id = source.id;
    let sync_and_stop_tasks = vec![
        // Trigger sync tasks
        tokio::spawn({
            let state_clone = state.clone();
            async move { 
                let scheduler = SourceScheduler::new(state_clone);
                let result = scheduler.trigger_sync(source_id).await;
                ("trigger_1", result.is_ok())
            }
        }),
        tokio::spawn({
            let state_clone = state.clone();
            async move { 
                sleep(Duration::from_millis(10)).await; // Small delay
                let scheduler = SourceScheduler::new(state_clone);
                let result = scheduler.trigger_sync(source_id).await;
                ("trigger_2", result.is_ok())
            }
        }),
        // Stop sync tasks
        tokio::spawn({
            let state_clone = state.clone();
            async move { 
                sleep(Duration::from_millis(5)).await; // Small delay
                let scheduler = SourceScheduler::new(state_clone);
                let result = scheduler.stop_sync(source_id).await;
                ("stop_1", result.is_ok())
            }
        }),
        tokio::spawn({
            let state_clone = state.clone();
            async move { 
                sleep(Duration::from_millis(15)).await; // Small delay
                let scheduler = SourceScheduler::new(state_clone);
                let result = scheduler.stop_sync(source_id).await;
                ("stop_2", result.is_ok())
            }
        }),
    ];
    
    let results = join_all(sync_and_stop_tasks).await;
    
    // Verify all operations completed without panicking
    for result in results {
        let (operation, success) = result.unwrap();
        println!("Operation {} completed: {}", operation, success);
        // Note: Success/failure depends on timing, but no operation should panic
    }
    
    // Final source should be in a consistent state (not stuck in "Syncing")
    sleep(Duration::from_millis(100)).await; // Allow operations to complete
    let final_source = state.db.get_source(user_id, source.id).await.unwrap().unwrap();
    println!("Final source status after concurrent operations: {:?}", final_source.status);
    
    // The source should not be permanently stuck in Syncing state
    assert_ne!(final_source.status, SourceStatus::Syncing, 
               "Source should not be stuck in syncing state after concurrent operations");
}

/// Test: Source status consistency during concurrent operations
/// This tests that source status updates remain consistent under concurrent access
#[tokio::test]
async fn test_source_status_consistency() {
    let (_test_context, state, user_id) = create_integration_test_state().await;
    
    // Create multiple sources to test concurrent status updates
    let sources = vec![
        create_test_webdav_source(&state, user_id, "status_test_1", false).await,
        create_test_webdav_source(&state, user_id, "status_test_2", false).await,
        create_test_webdav_source(&state, user_id, "status_test_3", false).await,
    ];
    
    // Create concurrent operations that update source status
    let status_update_tasks = sources.iter().flat_map(|source| {
        let source_id = source.id;
        let state_ref = state.clone();
        
        // For each source, create multiple concurrent operations
        (0..3).map(move |i| {
            let state_clone = state_ref.clone();
            tokio::spawn(async move {
                let scheduler = SourceScheduler::new(state_clone);
                // Simulate rapid start/stop cycles
                let trigger_result = scheduler.trigger_sync(source_id).await;
                sleep(Duration::from_millis(i * 5)).await; // Stagger operations
                let stop_result = scheduler.stop_sync(source_id).await;
                
                (source_id, trigger_result.is_ok(), stop_result.is_ok())
            })
        })
    }).collect::<Vec<_>>();
    
    let results = join_all(status_update_tasks).await;
    
    // Verify all operations completed
    for result in results {
        let (source_id, trigger_ok, stop_ok) = result.unwrap();
        println!("Source {}: trigger={}, stop={}", source_id, trigger_ok, stop_ok);
    }
    
    // Verify all sources are in consistent states
    for source in &sources {
        let updated_source = state.db.get_source(user_id, source.id).await.unwrap().unwrap();
        println!("Source {} final status: {:?}", source.name, updated_source.status);
        
        // Source should be in a valid state (not corrupted)
        assert!(
            matches!(updated_source.status, SourceStatus::Idle | SourceStatus::Error | SourceStatus::Syncing),
            "Source {} should be in a valid status, got {:?}", source.name, updated_source.status
        );
    }
}

/// Test: Directory ETag consistency during smart sync operations
/// This tests the core WebDAV ETag tracking under concurrent conditions
#[tokio::test]
async fn test_directory_etag_consistency_during_smart_sync() {
    let (_test_context, state, user_id) = create_integration_test_state().await;
    
    // Pre-populate with some directories that smart sync would discover
    let base_directories = vec![
        ("/test/docs", "etag-docs-v1"),
        ("/test/photos", "etag-photos-v1"),
        ("/test/archives", "etag-archives-v1"),
        ("/test/shared", "etag-shared-v1"),
    ];
    
    for (path, etag) in &base_directories {
        let dir = CreateWebDAVDirectory {
            user_id,
            directory_path: path.to_string(),
            directory_etag: etag.to_string(),
            file_count: 10,
            total_size_bytes: 1024000,
        };
        state.db.create_or_update_webdav_directory(&dir).await.unwrap();
    }
    
    // Create multiple SmartSyncService instances (as would happen in real concurrent syncs)
    let smart_sync_updates = (0..15).map(|i| {
        let state_clone = state.clone();
        let base_dirs = base_directories.clone(); // Clone for each task
        tokio::spawn(async move {
            // Pick a directory to update
            let dir_index = i % base_dirs.len();
            let (path, _) = &base_dirs[dir_index];
            
            // Simulate smart sync service updating directory ETags
            let updated_dir = CreateWebDAVDirectory {
                user_id,
                directory_path: path.to_string(),
                directory_etag: format!("etag-updated-{}-{}", dir_index, i),
                file_count: 10 + i as i64,
                total_size_bytes: 1024000 + (i as i64 * 50000),
            };
            
            let result = state_clone.db.create_or_update_webdav_directory(&updated_dir).await;
            (i, dir_index, path.to_string(), result.is_ok())
        })
    });
    
    let update_results = join_all(smart_sync_updates).await;
    
    // Verify all updates completed successfully
    for result in update_results {
        let (task_id, dir_index, path, success) = result.unwrap();
        assert!(success, "Task {} failed to update directory {} ({})", task_id, dir_index, path);
    }
    
    // Verify final directory state is consistent
    let final_directories = state.db.list_webdav_directories(user_id).await.unwrap();
    assert_eq!(final_directories.len(), base_directories.len(), 
               "Should still have exactly {} directories", base_directories.len());
    
    // Verify all directories have been updated (ETags should be different from originals)
    for dir in final_directories {
        assert!(dir.directory_etag.contains("updated"), 
               "Directory {} should have updated ETag: {}", dir.directory_path, dir.directory_etag);
        assert!(dir.file_count >= 10, "Directory should have updated file count");
    }
    
    println!("✅ Directory ETag consistency test completed successfully");
}

/// Test: Scheduler resilience under partial failures
/// This tests system behavior when some sync operations fail
#[tokio::test]
async fn test_scheduler_resilience_with_failures() {
    let (_test_context, state, user_id) = create_integration_test_state().await;
    
    // Create multiple sources, some valid, some with invalid config
    let valid_sources = vec![
        create_test_webdav_source(&state, user_id, "valid_source_1", false).await,
        create_test_webdav_source(&state, user_id, "valid_source_2", false).await,
    ];
    
    // Create invalid source by manually inserting bad config
    let invalid_config = serde_json::json!({
        "server_url": "", // Invalid empty URL
        "username": "test",
        "password": "test"
        // Missing required fields
    });
    
    let invalid_source = {
        let create_source = CreateSource {
            name: "invalid_source".to_string(),
            source_type: SourceType::WebDAV,
            config: invalid_config,
            enabled: Some(true),
        };
        state.db.create_source(user_id, &create_source).await.unwrap()
    };
    
    // Try to sync all sources concurrently
    let all_sources = [valid_sources, vec![invalid_source]].concat();
    let sync_tasks = all_sources.iter().map(|source| {
        let state_clone = state.clone();
        let source_id = source.id;
        let source_name = source.name.clone();
        tokio::spawn(async move {
            let scheduler = SourceScheduler::new(state_clone);
            let result = scheduler.trigger_sync(source_id).await;
            (source_name, result)
        })
    });
    
    let sync_results = join_all(sync_tasks).await;
    
    // Verify results
    let mut valid_sync_attempts = 0;
    let mut failed_sync_attempts = 0;
    
    for result in sync_results {
        let (source_name, sync_result) = result.unwrap();
        match sync_result {
            Ok(()) => {
                valid_sync_attempts += 1;
                println!("✅ Source '{}' sync triggered successfully", source_name);
            }
            Err(e) => {
                failed_sync_attempts += 1;
                println!("❌ Source '{}' sync failed: {}", source_name, e);
            }
        }
    }
    
    // We expect some syncs to fail (invalid source) but system should remain stable
    assert!(valid_sync_attempts > 0, "At least some valid sources should sync");
    assert!(failed_sync_attempts > 0, "Invalid source should fail");
    
    // Test recovery - try to sync a valid source after failures
    let recovery_scheduler = SourceScheduler::new(state.clone());
    let recovery_test = recovery_scheduler.trigger_sync(all_sources[0].id).await;
    assert!(recovery_test.is_ok() || recovery_test.unwrap_err().to_string().contains("already"), 
           "Scheduler should recover after partial failures");
    
    println!("✅ Scheduler resilience test completed successfully");
}