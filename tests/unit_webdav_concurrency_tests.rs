use std::{sync::Arc, time::Duration};
use tokio::time::sleep;
use uuid::Uuid;
use readur::{
    AppState,
    models::CreateWebDAVDirectory,
    test_utils::{TestContext, TestAuthHelper},
};
use futures;

/// Helper function to create test setup with database
async fn create_test_state() -> (TestContext, Arc<AppState>, Uuid) {
    let test_context = TestContext::new().await;
    
    let auth_helper = TestAuthHelper::new(test_context.app().clone());
    let test_user = auth_helper.create_test_user().await;

    let state = test_context.state().clone();
    let user_id = test_user.user_response.id;
    
    (test_context, state, user_id)
}


/// Test concurrent directory ETag updates to detect race conditions
#[tokio::test]
async fn test_concurrent_etag_updates() {
    let (_test_context, state, user_id) = create_test_state().await;
    
    // Create a base directory entry
    let directory = CreateWebDAVDirectory {
        user_id,
        directory_path: "/test/directory".to_string(),
        directory_etag: "etag-v1".to_string(),
        file_count: 10,
        total_size_bytes: 1024,
    };
    
    // Insert initial directory
    state.db.create_or_update_webdav_directory(&directory).await
        .expect("Failed to create initial directory");
    
    // Simulate concurrent updates from multiple "sync operations"
    let update_tasks = (0..10).map(|i| {
        let state_clone = state.clone();
        let user_id = user_id;
        tokio::spawn(async move {
            let updated_directory = CreateWebDAVDirectory {
                user_id,
                directory_path: "/test/directory".to_string(),
                directory_etag: format!("etag-v{}", i + 2),
                file_count: 10 + i,
                total_size_bytes: 1024 + (i * 100),
            };
            
            // Add small random delay to increase chance of race conditions
            sleep(Duration::from_millis(i as u64 * 10)).await;
            
            state_clone.db.create_or_update_webdav_directory(&updated_directory).await
        })
    });
    
    // Wait for all updates to complete
    let results: Vec<_> = futures::future::join_all(update_tasks).await;
    
    // Verify all updates succeeded (no database constraint violations)
    for result in results {
        assert!(result.is_ok(), "Concurrent update task failed");
        assert!(result.unwrap().is_ok(), "Database update failed");
    }
    
    // Verify final state is consistent
    let final_directories = state.db.list_webdav_directories(user_id).await
        .expect("Failed to list directories");
    
    assert_eq!(final_directories.len(), 1, "Should have exactly one directory");
    
    let final_dir = &final_directories[0];
    assert_eq!(final_dir.directory_path, "/test/directory");
    // ETag should be one of the updated values (database should handle concurrency)
    assert!(final_dir.directory_etag.starts_with("etag-v"), 
           "ETag should be updated: {}", final_dir.directory_etag);
}

/// Test concurrent sync operations on the same directory
#[tokio::test]
async fn test_concurrent_sync_operations() {
    let (_test_context, state, user_id) = create_test_state().await;
    
    // Pre-populate with some directories
    let directories = vec![
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/test/folder1".to_string(),
            directory_etag: "etag-1".to_string(),
            file_count: 5,
            total_size_bytes: 512,
        },
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/test/folder2".to_string(),
            directory_etag: "etag-2".to_string(),
            file_count: 3,
            total_size_bytes: 256,
        },
    ];
    
    for dir in directories {
        state.db.create_or_update_webdav_directory(&dir).await
            .expect("Failed to create test directory");
    }
    
    // Test that multiple sync evaluations can run concurrently without panicking
    let eval_tasks = (0..5).map(|i| {
        let state_clone = state.clone();
        let user_id = user_id;
        tokio::spawn(async move {
            // Note: This would normally require a WebDAV service, but we're testing
            // the database interaction logic for race conditions
            
            // Simulate concurrent reads of known directories
            let result = state_clone.db.list_webdav_directories(user_id).await;
            assert!(result.is_ok(), "Concurrent directory listing failed for task {}", i);
            
            let directories = result.unwrap();
            assert!(directories.len() >= 2, "Should have at least 2 directories");
            
            // Simulate directory updates
            if let Some(dir) = directories.first() {
                let updated_dir = CreateWebDAVDirectory {
                    user_id,
                    directory_path: dir.directory_path.clone(),
                    directory_etag: format!("{}-updated-{}", dir.directory_etag, i),
                    file_count: dir.file_count + 1,
                    total_size_bytes: dir.total_size_bytes + 100,
                };
                
                let update_result = state_clone.db.create_or_update_webdav_directory(&updated_dir).await;
                assert!(update_result.is_ok(), "Concurrent directory update failed for task {}", i);
            }
        })
    });
    
    // Wait for all tasks to complete
    let results: Vec<_> = futures::future::join_all(eval_tasks).await;
    
    // Verify all tasks completed successfully
    for (i, result) in results.into_iter().enumerate() {
        assert!(result.is_ok(), "Concurrent evaluation task {} failed", i);
    }
    
    // Verify database state is still consistent
    let final_directories = state.db.list_webdav_directories(user_id).await
        .expect("Failed to list final directories");
    
    assert_eq!(final_directories.len(), 2, "Should still have exactly 2 directories");
    
    // Verify at least some directories have been updated (due to concurrent access, all might not be updated)
    let updated_count = final_directories.iter()
        .filter(|dir| dir.directory_etag.contains("updated"))
        .count();
    
    assert!(updated_count > 0, "At least some directories should have been updated, got {} updated out of {}", 
           updated_count, final_directories.len());
}

/// Test ETag collision detection and handling
#[tokio::test]
async fn test_etag_collision_handling() {
    let (_test_context, state, user_id) = create_test_state().await;
    
    // Create directories with the same ETag (simulating ETag reuse)
    let dir1 = CreateWebDAVDirectory {
        user_id,
        directory_path: "/test/dir1".to_string(),
        directory_etag: "same-etag".to_string(),
        file_count: 5,
        total_size_bytes: 512,
    };
    
    let dir2 = CreateWebDAVDirectory {
        user_id,
        directory_path: "/test/dir2".to_string(),
        directory_etag: "same-etag".to_string(), // Same ETag, different path
        file_count: 3,
        total_size_bytes: 256,
    };
    
    // Insert both directories
    state.db.create_or_update_webdav_directory(&dir1).await
        .expect("Failed to create first directory");
    
    state.db.create_or_update_webdav_directory(&dir2).await
        .expect("Failed to create second directory");
    
    // Verify both directories exist with the same ETag
    let directories = state.db.list_webdav_directories(user_id).await
        .expect("Failed to list directories");
    
    assert_eq!(directories.len(), 2, "Should have 2 directories with same ETag");
    
    // Verify both have the same ETag but different paths
    let etags: Vec<_> = directories.iter().map(|d| &d.directory_etag).collect();
    assert!(etags.iter().all(|&etag| etag == "same-etag"), "All ETags should be the same");
    
    let paths: Vec<_> = directories.iter().map(|d| &d.directory_path).collect();
    assert!(paths.contains(&&"/test/dir1".to_string()), "Should contain dir1");
    assert!(paths.contains(&&"/test/dir2".to_string()), "Should contain dir2");
}

/// Test large-scale concurrent directory operations
#[tokio::test]
async fn test_large_scale_concurrent_operations() {
    let (_test_context, state, user_id) = create_test_state().await;
    
    let num_directories = 1000;
    let num_concurrent_operations = 10;
    
    // Create many directories concurrently
    let create_tasks = (0..num_concurrent_operations).map(|batch| {
        let state_clone = state.clone();
        let user_id = user_id;
        tokio::spawn(async move {
            let batch_size = num_directories / num_concurrent_operations;
            let start_idx = batch * batch_size;
            let end_idx = if batch == num_concurrent_operations - 1 {
                num_directories // Last batch gets remaining directories
            } else {
                start_idx + batch_size
            };
            
            for i in start_idx..end_idx {
                let directory = CreateWebDAVDirectory {
                    user_id,
                    directory_path: format!("/test/dir_{:04}", i),
                    directory_etag: format!("etag-{:04}", i),
                    file_count: i as i64,
                    total_size_bytes: (i * 1024) as i64,
                };
                
                let result = state_clone.db.create_or_update_webdav_directory(&directory).await;
                if result.is_err() {
                    eprintln!("Failed to create directory {}: {:?}", i, result.err());
                }
            }
        })
    });
    
    // Wait for all creation tasks
    let results: Vec<_> = futures::future::join_all(create_tasks).await;
    
    // Verify all creation tasks completed
    for (i, result) in results.into_iter().enumerate() {
        assert!(result.is_ok(), "Creation task {} failed", i);
    }
    
    // Verify all directories were created
    let directories = state.db.list_webdav_directories(user_id).await
        .expect("Failed to list directories");
    
    assert_eq!(directories.len(), num_directories, 
              "Should have created {} directories, got {}", num_directories, directories.len());
    
    // Now test concurrent reads and updates
    let update_tasks = (0..num_concurrent_operations).map(|batch| {
        let state_clone = state.clone();
        let user_id = user_id;
        tokio::spawn(async move {
            // Read all directories
            let directories = state_clone.db.list_webdav_directories(user_id).await
                .expect("Failed to read directories in concurrent update");
            
            assert_eq!(directories.len(), num_directories, "Concurrent read should see all directories");
            
            // Update some directories
            for (i, dir) in directories.iter().enumerate() {
                if i % 10 == batch { // Each task updates different directories
                    let updated_dir = CreateWebDAVDirectory {
                        user_id,
                        directory_path: dir.directory_path.clone(),
                        directory_etag: format!("{}-updated", dir.directory_etag),
                        file_count: dir.file_count + 1,
                        total_size_bytes: dir.total_size_bytes + 100,
                    };
                    
                    state_clone.db.create_or_update_webdav_directory(&updated_dir).await
                        .expect("Failed to update directory in concurrent operation");
                }
            }
        })
    });
    
    // Wait for all update tasks
    let update_results: Vec<_> = futures::future::join_all(update_tasks).await;
    
    // Verify all update tasks completed
    for (i, result) in update_results.into_iter().enumerate() {
        assert!(result.is_ok(), "Update task {} failed", i);
    }
    
    // Final verification
    let final_directories = state.db.list_webdav_directories(user_id).await
        .expect("Failed to list final directories");
    
    assert_eq!(final_directories.len(), num_directories, "Should still have all directories");
    
    // Count updated directories
    let updated_count = final_directories.iter()
        .filter(|d| d.directory_etag.contains("updated"))
        .count();
    
    // Due to concurrent access patterns, we expect some directories to be updated
    // Each task attempts to update every 10th directory, so there should be overlapping updates
    // The actual count will depend on race conditions, but should be more than 0 and less than total
    assert!(updated_count > 0 && updated_count <= num_directories,
           "Should have updated some directories (0 < {} <= {})", updated_count, num_directories);
}

/// Test directory deletion concurrency
#[tokio::test]
async fn test_concurrent_directory_deletion() {
    let (_test_context, state, user_id) = create_test_state().await;
    
    // Create test directories
    let directories = (0..50).map(|i| CreateWebDAVDirectory {
        user_id,
        directory_path: format!("/test/deleteme_{:02}", i),
        directory_etag: format!("etag-{:02}", i),
        file_count: i,
        total_size_bytes: (i * 100) as i64,
    }).collect::<Vec<_>>();
    
    // Insert all directories
    for dir in &directories {
        state.db.create_or_update_webdav_directory(dir).await
            .expect("Failed to create test directory");
    }
    
    // Verify initial count
    let initial_count = state.db.list_webdav_directories(user_id).await
        .expect("Failed to list initial directories")
        .len();
    assert_eq!(initial_count, 50, "Should have 50 initial directories");
    
    // Test concurrent deletion operations
    // Note: Since we don't have a direct delete method in the current API,
    // we simulate cleanup by testing concurrent create_or_update operations
    // that might conflict with each other
    
    let concurrent_tasks = (0..10).map(|task_id| {
        let state_clone = state.clone();
        let user_id = user_id;
        tokio::spawn(async move {
            // Each task updates a subset of directories
            for i in (task_id..50).step_by(10) {
                let updated_dir = CreateWebDAVDirectory {
                    user_id,
                    directory_path: format!("/test/deleteme_{:02}", i),
                    directory_etag: format!("etag-{:02}-task-{}", i, task_id),
                    file_count: i + 100, // Significant change
                    total_size_bytes: ((i + 100) * 100) as i64,
                };
                
                state_clone.db.create_or_update_webdav_directory(&updated_dir).await
                    .expect("Failed to update directory in concurrent deletion test");
            }
        })
    });
    
    // Wait for all tasks
    let results: Vec<_> = futures::future::join_all(concurrent_tasks).await;
    
    // Verify all tasks completed
    for (i, result) in results.into_iter().enumerate() {
        assert!(result.is_ok(), "Concurrent deletion task {} failed", i);
    }
    
    // Verify final state
    let final_directories = state.db.list_webdav_directories(user_id).await
        .expect("Failed to list final directories");
    
    assert_eq!(final_directories.len(), 50, "Should still have 50 directories");
    
    // Verify all directories were updated by at least one task
    for dir in final_directories {
        assert!(dir.directory_etag.contains("task"), 
               "Directory should have been updated: {}", dir.directory_etag);
        assert!(dir.file_count >= 100, 
               "Directory file count should be updated: {}", dir.file_count);
    }
}

/// Test concurrent operations with network failures (simulated)
#[tokio::test]
async fn test_concurrent_operations_with_simulated_failures() {
    let (_test_context, state, user_id) = create_test_state().await;
    
    // Create initial directory state
    let initial_directories = vec![
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/test/stable".to_string(),
            directory_etag: "stable-etag".to_string(),
            file_count: 10,
            total_size_bytes: 1024,
        },
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/test/changing".to_string(),
            directory_etag: "changing-etag-v1".to_string(),
            file_count: 5,
            total_size_bytes: 512,
        },
    ];
    
    for dir in initial_directories {
        state.db.create_or_update_webdav_directory(&dir).await
            .expect("Failed to create initial directory");
    }
    
    // Simulate concurrent operations where some fail and some succeed
    let mixed_tasks = (0..20).map(|i| {
        let state_clone = state.clone();
        let user_id = user_id;
        tokio::spawn(async move {
            if i % 3 == 0 {
                // Simulate "successful" operations - update directory
                let updated_dir = CreateWebDAVDirectory {
                    user_id,
                    directory_path: "/test/changing".to_string(),
                    directory_etag: format!("changing-etag-v{}", i + 2),
                    file_count: 5 + i,
                    total_size_bytes: 512 + (i * 50),
                };
                
                let result = state_clone.db.create_or_update_webdav_directory(&updated_dir).await;
                assert!(result.is_ok(), "Successful operation should work");
                
            } else if i % 3 == 1 {
                // Simulate "read-only" operations - just list directories
                let result = state_clone.db.list_webdav_directories(user_id).await;
                assert!(result.is_ok(), "Read operation should work");
                assert_eq!(result.unwrap().len(), 2, "Should always see 2 directories");
                
            } else {
                // Simulate "failed" operations - try to access non-existent directory
                // This tests that the system remains stable even with partial failures
                
                // Try to read a non-existent directory (this won't cause a database error,
                // but simulates the kind of inconsistent state that might occur during
                // network failures)
                let directories = state_clone.db.list_webdav_directories(user_id).await
                    .expect("Failed to list directories");
                
                // Verify system remains in consistent state
                assert!(directories.len() >= 2, "Should have at least 2 directories");
                
                // Try to update with potentially stale ETag
                if let Some(dir) = directories.iter().find(|d| d.directory_path == "/test/changing") {
                    let stale_update = CreateWebDAVDirectory {
                        user_id,
                        directory_path: dir.directory_path.clone(),
                        directory_etag: "stale-etag".to_string(), // Potentially stale
                        file_count: dir.file_count - 1, // Simulate old data
                        total_size_bytes: dir.total_size_bytes - 100,
                    };
                    
                    // This should still work (overwriting with stale data)
                    // In a real system, you'd want ETag validation to prevent this
                    let result = state_clone.db.create_or_update_webdav_directory(&stale_update).await;
                    assert!(result.is_ok(), "Even stale updates should complete");
                }
            }
        })
    });
    
    // Wait for all mixed operations
    let results: Vec<_> = futures::future::join_all(mixed_tasks).await;
    
    // Verify all tasks completed (even simulated failures should not panic)
    for (i, result) in results.into_iter().enumerate() {
        assert!(result.is_ok(), "Mixed operation task {} should complete", i);
    }
    
    // Verify system is still in a consistent state
    let final_directories = state.db.list_webdav_directories(user_id).await
        .expect("Failed to list final directories");
    
    assert_eq!(final_directories.len(), 2, "Should still have 2 directories");
    
    // Verify the stable directory remains unchanged
    let stable_dir = final_directories.iter()
        .find(|d| d.directory_path == "/test/stable")
        .expect("Stable directory should still exist");
    assert_eq!(stable_dir.directory_etag, "stable-etag", "Stable directory should be unchanged");
    
    // The changing directory should have been updated
    let changing_dir = final_directories.iter()
        .find(|d| d.directory_path == "/test/changing")
        .expect("Changing directory should still exist");
    // ETag could be anything depending on which operation completed last
    assert!(!changing_dir.directory_etag.is_empty(), "Changing directory should have an ETag");
}