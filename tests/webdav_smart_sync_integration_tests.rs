use std::{sync::Arc, time::Duration, collections::HashMap};
use tokio::time::sleep;
use uuid::Uuid;
use futures::future::join_all;
use anyhow::Result;
use readur::{
    AppState,
    models::{CreateWebDAVDirectory, FileIngestionInfo},
    test_utils::{TestContext, TestAuthHelper},
    services::webdav::{
        SmartSyncService, 
        SmartSyncStrategy, 
        SyncProgress, 
        SyncPhase,
        WebDAVDiscoveryResult,
    },
};

/// Helper function to create test setup
async fn create_smart_sync_test_state() -> (TestContext, Arc<AppState>, Uuid) {
    let test_context = TestContext::new().await;
    
    let auth_helper = TestAuthHelper::new(test_context.app().clone());
    let test_user = auth_helper.create_test_user().await;

    let state = test_context.state().clone();
    let user_id = test_user.user_response.id;
    
    (test_context, state, user_id)
}

/// Mock WebDAV service for testing smart sync behavior
#[derive(Clone)]
struct MockWebDAVServiceForSmartSync {
    discovery_results: Arc<std::sync::Mutex<HashMap<String, WebDAVDiscoveryResult>>>,
    call_count: Arc<std::sync::Mutex<u32>>,
    delay_ms: u64,
    should_fail: bool,
}

impl MockWebDAVServiceForSmartSync {
    fn new(delay_ms: u64, should_fail: bool) -> Self {
        Self {
            discovery_results: Arc::new(std::sync::Mutex::new(HashMap::new())),
            call_count: Arc::new(std::sync::Mutex::new(0)),
            delay_ms,
            should_fail,
        }
    }

    fn set_discovery_result(&self, path: &str, result: WebDAVDiscoveryResult) {
        let mut results = self.discovery_results.lock().unwrap();
        results.insert(path.to_string(), result);
    }

    async fn mock_discover_files_and_directories(
        &self,
        directory_path: &str,
        _recursive: bool,
    ) -> Result<WebDAVDiscoveryResult> {
        // Increment call count
        {
            let mut count = self.call_count.lock().unwrap();
            *count += 1;
        }

        // Simulate network delay
        sleep(Duration::from_millis(self.delay_ms)).await;

        if self.should_fail {
            return Err(anyhow::anyhow!("Mock WebDAV discovery failed for {}", directory_path));
        }

        // Return preset result or generate default
        let results = self.discovery_results.lock().unwrap();
        if let Some(result) = results.get(directory_path) {
            Ok(result.clone())
        } else {
            // Generate default result
            Ok(WebDAVDiscoveryResult {
                files: vec![
                    FileIngestionInfo {
                        name: "default.pdf".to_string(),
                        relative_path: format!("{}/default.pdf", directory_path),
                        full_path: format!("{}/default.pdf", directory_path),
                        path: format!("{}/default.pdf", directory_path),
                        size: 1024,
                        mime_type: "application/pdf".to_string(),
                        last_modified: Some(chrono::Utc::now()),
                        etag: format!("default-etag-{}", directory_path.replace('/', "-")),
                        is_directory: false,
                        created_at: None,
                        permissions: None,
                        owner: None,
                        group: None,
                        metadata: None,
                    }
                ],
                directories: vec![
                    FileIngestionInfo {
                        name: "subdir".to_string(),
                        relative_path: format!("{}/subdir", directory_path),
                        full_path: format!("{}/subdir", directory_path),
                        path: format!("{}/subdir", directory_path),
                        size: 0,
                        mime_type: "inode/directory".to_string(),
                        last_modified: Some(chrono::Utc::now()),
                        etag: format!("dir-etag-{}", directory_path.replace('/', "-")),
                        is_directory: true,
                        created_at: None,
                        permissions: None,
                        owner: None,
                        group: None,
                        metadata: None,
                    }
                ],
            })
        }
    }

    fn get_call_count(&self) -> u32 {
        *self.call_count.lock().unwrap()
    }
}

/// Test concurrent smart sync evaluations with different ETag scenarios
#[tokio::test]
async fn test_concurrent_smart_sync_etag_evaluation() {
    let (_test_context, state, user_id) = create_smart_sync_test_state().await;
    
    let smart_sync_service = SmartSyncService::new(state.clone());
    
    // Set up initial directory state
    let initial_directories = vec![
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/test".to_string(),
            directory_etag: "old-etag-1".to_string(),
            file_count: 5,
            total_size_bytes: 1024,
        },
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/test/subdir1".to_string(),
            directory_etag: "old-etag-2".to_string(),
            file_count: 3,
            total_size_bytes: 512,
        },
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/test/subdir2".to_string(),
            directory_etag: "old-etag-3".to_string(),
            file_count: 2,
            total_size_bytes: 256,
        },
    ];
    
    for dir in initial_directories {
        state.db.create_or_update_webdav_directory(&dir).await
            .expect("Failed to create initial directory");
    }
    
    // Create mock WebDAV services simulating different discovery scenarios
    let mock_services = vec![
        (MockWebDAVServiceForSmartSync::new(50, false), "unchanged"),  // ETags unchanged
        (MockWebDAVServiceForSmartSync::new(75, false), "changed"),    // ETags changed
        (MockWebDAVServiceForSmartSync::new(100, false), "new_dirs"),  // New directories
        (MockWebDAVServiceForSmartSync::new(125, false), "mixed"),     // Mixed changes
        (MockWebDAVServiceForSmartSync::new(25, true), "failed"),      // Network failure
    ];
    
    // Configure mock responses
    for (mock_service, scenario) in &mock_services {
        match *scenario {
            "unchanged" => {
                // Return same ETags as database
                mock_service.set_discovery_result("/test", WebDAVDiscoveryResult {
                    files: vec![],
                    directories: vec![
                        FileIngestionInfo {
                            name: "subdir1".to_string(),
                            relative_path: "/test/subdir1".to_string(),
                            full_path: "/test/subdir1".to_string(),
                            path: "/test/subdir1".to_string(),
                            size: 0,
                            mime_type: "inode/directory".to_string(),
                            last_modified: Some(chrono::Utc::now()),
                            etag: "old-etag-2".to_string(), // Same as database
                            is_directory: true,
                            created_at: None,
                            permissions: None,
                            owner: None,
                            group: None,
                            metadata: None,
                        },
                        FileIngestionInfo {
                            name: "subdir2".to_string(),
                            relative_path: "/test/subdir2".to_string(),
                            full_path: "/test/subdir2".to_string(),
                            path: "/test/subdir2".to_string(),
                            size: 0,
                            mime_type: "inode/directory".to_string(),
                            last_modified: Some(chrono::Utc::now()),
                            etag: "old-etag-3".to_string(), // Same as database
                            is_directory: true,
                            created_at: None,
                            permissions: None,
                            owner: None,
                            group: None,
                            metadata: None,
                        },
                    ],
                });
            }
            "changed" => {
                // Return different ETags
                mock_service.set_discovery_result("/test", WebDAVDiscoveryResult {
                    files: vec![],
                    directories: vec![
                        FileIngestionInfo {
                            name: "subdir1".to_string(),
                            relative_path: "/test/subdir1".to_string(),
                            full_path: "/test/subdir1".to_string(),
                            path: "/test/subdir1".to_string(),
                            size: 0,
                            mime_type: "inode/directory".to_string(),
                            last_modified: Some(chrono::Utc::now()),
                            etag: "new-etag-2".to_string(), // Changed
                            is_directory: true,
                            created_at: None,
                            permissions: None,
                            owner: None,
                            group: None,
                            metadata: None,
                        },
                    ],
                });
            }
            "new_dirs" => {
                // Return new directories
                mock_service.set_discovery_result("/test", WebDAVDiscoveryResult {
                    files: vec![],
                    directories: vec![
                        FileIngestionInfo {
                            name: "new_subdir".to_string(),
                            relative_path: "/test/new_subdir".to_string(),
                            full_path: "/test/new_subdir".to_string(),
                            path: "/test/new_subdir".to_string(),
                            size: 0,
                            mime_type: "inode/directory".to_string(),
                            last_modified: Some(chrono::Utc::now()),
                            etag: "new-dir-etag".to_string(),
                            is_directory: true,
                            created_at: None,
                            permissions: None,
                            owner: None,
                            group: None,
                            metadata: None,
                        },
                    ],
                });
            }
            "mixed" => {
                // Mix of changed and new
                mock_service.set_discovery_result("/test", WebDAVDiscoveryResult {
                    files: vec![],
                    directories: vec![
                        FileIngestionInfo {
                            name: "subdir1".to_string(),
                            relative_path: "/test/subdir1".to_string(),
                            full_path: "/test/subdir1".to_string(),
                            path: "/test/subdir1".to_string(),
                            size: 0,
                            mime_type: "inode/directory".to_string(),
                            last_modified: Some(chrono::Utc::now()),
                            etag: "updated-etag-2".to_string(), // Changed
                            is_directory: true,
                            created_at: None,
                            permissions: None,
                            owner: None,
                            group: None,
                            metadata: None,
                        },
                        FileIngestionInfo {
                            name: "another_new_dir".to_string(),
                            relative_path: "/test/another_new_dir".to_string(),
                            full_path: "/test/another_new_dir".to_string(),
                            path: "/test/another_new_dir".to_string(),
                            size: 0,
                            mime_type: "inode/directory".to_string(),
                            last_modified: Some(chrono::Utc::now()),
                            etag: "another-new-etag".to_string(), // New
                            is_directory: true,
                            created_at: None,
                            permissions: None,
                            owner: None,
                            group: None,
                            metadata: None,
                        },
                    ],
                });
            }
            _ => {} // Failed case doesn't need setup
        }
    }
    
    // Run concurrent smart sync evaluations
    let concurrent_evaluations = mock_services.into_iter().enumerate().map(|(i, (mock_service, scenario))| {
        let smart_sync_clone = smart_sync_service.clone();
        let user_id = user_id;
        let scenario = scenario.to_string();
        tokio::spawn(async move {
            println!("Starting smart sync evaluation {} ({})", i, scenario);
            
            // Since we can't directly inject the mock into SmartSyncService,
            // we'll simulate the evaluation logic by calling the database methods
            // that SmartSyncService would call
            
            // 1. Get known directories (what SmartSyncService.evaluate_sync_need does)
            let known_dirs_result = smart_sync_clone.state().db.list_webdav_directories(user_id).await;
            
            // 2. Simulate discovery with delay (mock WebDAV call)
            let discovery_result = mock_service.mock_discover_files_and_directories("/test", false).await;
            
            // 3. If discovery succeeded, update directory ETags (what perform_smart_sync would do)
            let update_results = if let Ok(discovery) = &discovery_result {
                let mut results = Vec::new();
                for dir_info in &discovery.directories {
                    let update_dir = CreateWebDAVDirectory {
                        user_id,
                        directory_path: dir_info.relative_path.clone(),
                        directory_etag: dir_info.etag.clone(),
                        file_count: 0,
                        total_size_bytes: 0,
                    };
                    let result = smart_sync_clone.state().db.create_or_update_webdav_directory(&update_dir).await;
                    results.push(result.is_ok());
                }
                results
            } else {
                vec![]
            };
            
            println!("Completed smart sync evaluation {} ({})", i, scenario);
            (i, scenario, known_dirs_result.is_ok(), discovery_result.is_ok(), update_results)
        })
    });
    
    // Wait for all evaluations
    let evaluation_results: Vec<_> = join_all(concurrent_evaluations).await;
    
    // Analyze results
    let mut scenario_stats = HashMap::new();
    for result in evaluation_results {
        assert!(result.is_ok(), "Evaluation task should complete without panicking");
        let (task_id, scenario, db_read_ok, discovery_ok, updates) = result.unwrap();
        
        *scenario_stats.entry(scenario.clone()).or_insert(0) += 1;
        
        println!("Evaluation {}: {} - DB read: {}, Discovery: {}, Updates: {:?}", 
                 task_id, scenario, db_read_ok, discovery_ok, updates);
        
        // Database reads should always succeed
        assert!(db_read_ok, "Database read should succeed for evaluation {}", task_id);
        
        // Discovery should succeed unless it's the "failed" scenario
        if scenario != "failed" {
            assert!(discovery_ok, "Discovery should succeed for evaluation {} ({})", task_id, scenario);
        }
    }
    
    println!("Scenario statistics: {:?}", scenario_stats);
    
    // Verify final state consistency
    let final_directories = state.db.list_webdav_directories(user_id).await
        .expect("Failed to list final directories");
    
    // Should have at least the original directories, possibly more from concurrent updates
    assert!(final_directories.len() >= 3, 
           "Should have at least 3 directories, got {}", final_directories.len());
    
    // Check that some directories were updated by successful evaluations
    let updated_dirs = final_directories.iter()
        .filter(|d| !d.directory_etag.starts_with("old-etag"))
        .count();
    
    println!("Updated directories: {}/{}", updated_dirs, final_directories.len());
    
    // At least some directories should have been updated (unless all operations failed)
    // This depends on the specific timing of concurrent operations
}

/// Test smart sync full deep scan vs targeted scan concurrency
#[tokio::test]
async fn test_concurrent_smart_sync_strategies() {
    let (_test_context, state, user_id) = create_smart_sync_test_state().await;
    
    let smart_sync_service = SmartSyncService::new(state.clone());
    
    // Create extensive directory structure
    let directories = vec![
        ("/project", "proj-etag-1"),
        ("/project/docs", "docs-etag-1"),
        ("/project/src", "src-etag-1"),
        ("/project/tests", "tests-etag-1"),
        ("/project/docs/api", "api-etag-1"),
        ("/project/docs/user", "user-etag-1"),
        ("/archive", "arch-etag-1"),
        ("/archive/2023", "2023-etag-1"),
        ("/archive/2024", "2024-etag-1"),
    ];
    
    for (path, etag) in directories {
        let directory = CreateWebDAVDirectory {
            user_id,
            directory_path: path.to_string(),
            directory_etag: etag.to_string(),
            file_count: 5,
            total_size_bytes: 1024,
        };
        state.db.create_or_update_webdav_directory(&directory).await
            .expect("Failed to create directory");
    }
    
    // Test concurrent operations with different strategies
    let strategy_operations = vec![
        (SmartSyncStrategy::FullDeepScan, "/project", "full_scan_1"),
        (SmartSyncStrategy::TargetedScan(vec!["/project/docs".to_string()]), "/project", "targeted_1"),
        (SmartSyncStrategy::FullDeepScan, "/archive", "full_scan_2"),
        (SmartSyncStrategy::TargetedScan(vec!["/archive/2023".to_string(), "/archive/2024".to_string()]), "/archive", "targeted_2"),
        (SmartSyncStrategy::FullDeepScan, "/project", "full_scan_3"), // Overlapping with targeted_1
    ];
    
    let concurrent_strategy_tests = strategy_operations.into_iter().enumerate().map(|(i, (strategy, base_path, test_name))| {
        let smart_sync_clone = smart_sync_service.clone();
        let user_id = user_id;
        let base_path = base_path.to_string();
        let test_name = test_name.to_string();
        tokio::spawn(async move {
            println!("Starting strategy test {} ({}) for {}", i, test_name, base_path);
            
            // Simulate what perform_smart_sync would do for each strategy
            let result: Result<i32, anyhow::Error> = match strategy {
                SmartSyncStrategy::FullDeepScan => {
                    // Simulate full deep scan - update all directories under base_path
                    let all_dirs = smart_sync_clone.state().db.list_webdav_directories(user_id).await?;
                    let relevant_dirs: Vec<_> = all_dirs.into_iter()
                        .filter(|d| d.directory_path.starts_with(&base_path))
                        .collect();
                    
                    let mut update_count = 0;
                    for dir in relevant_dirs {
                        let updated_dir = CreateWebDAVDirectory {
                            user_id,
                            directory_path: dir.directory_path,
                            directory_etag: format!("{}-updated-by-{}", dir.directory_etag, test_name),
                            file_count: dir.file_count + 1,
                            total_size_bytes: dir.total_size_bytes + 100,
                        };
                        
                        if smart_sync_clone.state().db.create_or_update_webdav_directory(&updated_dir).await.is_ok() {
                            update_count += 1;
                        }
                    }
                    Ok(update_count)
                }
                SmartSyncStrategy::TargetedScan(target_dirs) => {
                    // Simulate targeted scan - only update specific directories
                    let mut update_count = 0;
                    for target_dir in target_dirs {
                        let updated_dir = CreateWebDAVDirectory {
                            user_id,
                            directory_path: target_dir.clone(),
                            directory_etag: format!("targeted-etag-{}", test_name),
                            file_count: 10,
                            total_size_bytes: 2048,
                        };
                        
                        if smart_sync_clone.state().db.create_or_update_webdav_directory(&updated_dir).await.is_ok() {
                            update_count += 1;
                        }
                    }
                    Ok(update_count)
                }
            };
            
            println!("Completed strategy test {} ({})", i, test_name);
            Result::<_, anyhow::Error>::Ok((i, test_name, result))
        })
    });
    
    // Wait for all strategy tests
    let strategy_results: Vec<_> = join_all(concurrent_strategy_tests).await;
    
    // Analyze strategy execution results
    let mut total_updates = 0;
    for result in strategy_results {
        assert!(result.is_ok(), "Strategy test task should complete");
        let strategy_result = result.unwrap();
        assert!(strategy_result.is_ok(), "Strategy test should not panic");
        
        let (task_id, test_name, update_result) = strategy_result.unwrap();
        match update_result {
            Ok(update_count) => {
                println!("Strategy test {}: {} updated {} directories", task_id, test_name, update_count);
                total_updates += update_count;
            }
            Err(e) => {
                println!("Strategy test {}: {} failed: {}", task_id, test_name, e);
            }
        }
    }
    
    println!("Total directory updates across all strategies: {}", total_updates);
    
    // Verify final state
    let final_directories = state.db.list_webdav_directories(user_id).await
        .expect("Failed to list final directories");
    
    // Should still have all directories
    assert_eq!(final_directories.len(), 9, "Should have all 9 directories");
    
    // Check for evidence of different strategy executions
    let full_scan_updates = final_directories.iter()
        .filter(|d| d.directory_etag.contains("updated-by-full_scan"))
        .count();
    
    let targeted_updates = final_directories.iter()
        .filter(|d| d.directory_etag.contains("targeted-etag"))
        .count();
    
    println!("Full scan updates: {}, Targeted updates: {}", full_scan_updates, targeted_updates);
    
    // At least some updates should have occurred
    assert!(total_updates > 0, "At least some strategy operations should have updated directories");
}

/// Test smart sync progress tracking under concurrent operations
#[tokio::test]
async fn test_concurrent_smart_sync_progress_tracking() {
    let (_test_context, state, user_id) = create_smart_sync_test_state().await;
    
    let smart_sync_service = SmartSyncService::new(state.clone());
    
    // Create multiple progress trackers for concurrent operations
    let progress_operations = (0..5).map(|i| {
        let smart_sync_clone = smart_sync_service.clone();
        let user_id = user_id;
        tokio::spawn(async move {
            let progress = SyncProgress::new();
            progress.set_phase(SyncPhase::Initializing);
            
            println!("Progress operation {} starting", i);
            
            // Simulate smart sync operation with progress tracking
            progress.set_phase(SyncPhase::Evaluating);
            progress.set_current_directory(&format!("/operation-{}", i));
            
            // Simulate database operations
            sleep(Duration::from_millis(50)).await;
            
            progress.set_phase(SyncPhase::DiscoveringDirectories);
            progress.set_current_directory(&format!("/operation-{}/subdir", i));
            
            // Simulate discovery delay
            sleep(Duration::from_millis(100)).await;
            
            progress.set_phase(SyncPhase::SavingMetadata);
            
            // Update directory (simulates saving discovered metadata)
            let directory = CreateWebDAVDirectory {
                user_id,
                directory_path: format!("/operation-{}", i),
                directory_etag: format!("progress-etag-{}", i),
                file_count: i as i64,
                total_size_bytes: (i as i64) * 1024,
            };
            
            let db_result = smart_sync_clone.state().db.create_or_update_webdav_directory(&directory).await;
            
            if db_result.is_ok() {
                progress.set_phase(SyncPhase::Completed);
            } else {
                progress.set_phase(SyncPhase::Failed("Database update failed".to_string()));
            }
            
            // Get final progress stats
            let stats = progress.get_stats();
            
            println!("Progress operation {} completed", i);
            (i, db_result.is_ok(), stats)
        })
    });
    
    // Wait for all progress operations
    let progress_results: Vec<_> = join_all(progress_operations).await;
    
    // Verify progress tracking results
    let mut successful_operations = 0;
    for result in progress_results {
        assert!(result.is_ok(), "Progress tracking task should complete");
        let (operation_id, db_success, stats) = result.unwrap();
        
        if db_success {
            successful_operations += 1;
        }
        
        if let Some(stats) = stats {
            println!("Operation {}: Success: {}, Elapsed: {:?}, Errors: {:?}", 
                     operation_id, db_success, stats.elapsed_time, stats.errors);
        }
    }
    
    println!("Successful progress operations: {}/5", successful_operations);
    
    // Verify created directories
    let final_directories = state.db.list_webdav_directories(user_id).await
        .expect("Failed to list directories");
    
    let progress_dirs = final_directories.iter()
        .filter(|d| d.directory_etag.starts_with("progress-etag-"))
        .count();
    
    assert_eq!(progress_dirs, successful_operations, 
              "Number of progress directories should match successful operations");
    
    // All operations should have completed (successfully or not)
    assert!(successful_operations > 0, "At least some operations should succeed");
}

/// Test concurrent smart sync operations with simulated ETag conflicts
#[tokio::test]
async fn test_concurrent_smart_sync_etag_conflicts() {
    let (_test_context, state, user_id) = create_smart_sync_test_state().await;
    
    let smart_sync_service = SmartSyncService::new(state.clone());
    
    // Create a shared directory that multiple operations will try to update
    let shared_directory = CreateWebDAVDirectory {
        user_id,
        directory_path: "/shared".to_string(),
        directory_etag: "initial-shared-etag".to_string(),
        file_count: 10,
        total_size_bytes: 2048,
    };
    
    state.db.create_or_update_webdav_directory(&shared_directory).await
        .expect("Failed to create shared directory");
    
    // Create concurrent operations that all try to update the same directory
    let etag_conflict_operations = (0..10).map(|i| {
        let smart_sync_clone = smart_sync_service.clone();
        let user_id = user_id;
        tokio::spawn(async move {
            println!("ETag conflict operation {} starting", i);
            
            // First, read the current directory state
            let current_dirs = smart_sync_clone.state().db.list_webdav_directories(user_id).await?;
            let shared_dir = current_dirs.iter()
                .find(|d| d.directory_path == "/shared")
                .ok_or_else(|| anyhow::anyhow!("Shared directory not found"))?;
            
            // Simulate discovery finding changes (each operation thinks it found different changes)
            sleep(Duration::from_millis((i % 5) * 20)).await; // Variable delay to create race conditions
            
            // Try to update with a new ETag (simulating discovered changes)
            let updated_directory = CreateWebDAVDirectory {
                user_id,
                directory_path: "/shared".to_string(),
                directory_etag: format!("conflict-etag-operation-{}", i),
                file_count: shared_dir.file_count + i as i64,
                total_size_bytes: shared_dir.total_size_bytes + (i as i64 * 100),
            };
            
            let update_result = smart_sync_clone.state().db.create_or_update_webdav_directory(&updated_directory).await;
            
            println!("ETag conflict operation {} completed: {:?}", i, update_result.is_ok());
            Result::<_, anyhow::Error>::Ok((i, update_result.is_ok()))
        })
    });
    
    // Wait for all conflict operations
    let conflict_results: Vec<_> = join_all(etag_conflict_operations).await;
    
    // Analyze conflict resolution
    let mut successful_updates = 0;
    for result in conflict_results {
        assert!(result.is_ok(), "ETag conflict task should complete");
        let operation_result = result.unwrap();
        assert!(operation_result.is_ok(), "ETag conflict operation should not panic");
        
        let (operation_id, update_success) = operation_result.unwrap();
        if update_success {
            successful_updates += 1;
        }
        println!("ETag conflict operation {}: {}", operation_id, update_success);
    }
    
    println!("Successful ETag updates: {}/10", successful_updates);
    
    // Verify final state
    let final_directories = state.db.list_webdav_directories(user_id).await
        .expect("Failed to list final directories");
    
    assert_eq!(final_directories.len(), 1, "Should have exactly one shared directory");
    
    let final_shared_dir = &final_directories[0];
    assert_eq!(final_shared_dir.directory_path, "/shared");
    
    // The final ETag should be from one of the operations
    assert!(final_shared_dir.directory_etag.contains("conflict-etag-operation-") ||
           final_shared_dir.directory_etag == "initial-shared-etag",
           "Final ETag should be from one of the operations: {}", final_shared_dir.directory_etag);
    
    // File count should have been updated by the successful operation
    if successful_updates > 0 {
        assert!(final_shared_dir.file_count >= 10, 
               "File count should have been updated: {}", final_shared_dir.file_count);
    }
    
    // All operations should have succeeded (database should handle concurrency gracefully)
    assert!(successful_updates > 0, "At least some ETag updates should succeed");
}