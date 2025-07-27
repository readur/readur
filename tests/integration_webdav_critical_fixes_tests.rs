use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;
use tokio;
use futures::future::join_all;
use readur::{
    models::{CreateWebDAVDirectory, CreateUser, UserRole},
    db::Database,
    test_utils::TestContext,
    AppState,
};

/// Integration test that validates the race condition fix
/// Tests that concurrent directory updates are atomic and consistent
#[tokio::test]
async fn test_race_condition_fix_atomic_updates() {
    let test_context = TestContext::new().await;
    let db = Arc::new(test_context.state.db.clone());
    
    // Create a test user first
    let create_user = CreateUser {
        username: "race_testuser".to_string(),
        email: "race@example.com".to_string(),
        password: "password123".to_string(),
        role: Some(UserRole::User),
    };
    let user = db.create_user(create_user).await
        .expect("Failed to create test user");
    let user_id = user.id;
    
    // Create initial directories
    let initial_directories = vec![
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/test/dir1".to_string(),
            directory_etag: "initial_etag1".to_string(),
            file_count: 5,
            total_size_bytes: 1024,
        },
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/test/dir2".to_string(),
            directory_etag: "initial_etag2".to_string(),
            file_count: 10,
            total_size_bytes: 2048,
        },
    ];
    
    let _ = db.bulk_create_or_update_webdav_directories(&initial_directories).await.unwrap();
    
    // Simulate race condition: multiple tasks trying to update directories simultaneously
    let mut handles = vec![];
    
    for i in 0..5 {
        let db_clone = Arc::clone(&db);
        let handle = tokio::spawn(async move {
            let updated_directories = vec![
                CreateWebDAVDirectory {
                    user_id,
                    directory_path: "/test/dir1".to_string(),
                    directory_etag: format!("race_etag1_{}", i),
                    file_count: 5 + i as i64,
                    total_size_bytes: 1024 + (i * 100) as i64,
                },
                CreateWebDAVDirectory {
                    user_id,
                    directory_path: "/test/dir2".to_string(),
                    directory_etag: format!("race_etag2_{}", i),
                    file_count: 10 + i as i64,
                    total_size_bytes: 2048 + (i * 200) as i64,
                },
                CreateWebDAVDirectory {
                    user_id,
                    directory_path: format!("/test/new_dir_{}", i),
                    directory_etag: format!("new_etag_{}", i),
                    file_count: i as i64,
                    total_size_bytes: (i * 512) as i64,
                },
            ];
            
            // Use the atomic sync operation
            db_clone.sync_webdav_directories(user_id, &updated_directories).await
        });
        handles.push(handle);
    }
    
    // Wait for all operations to complete
    let results: Vec<_> = join_all(handles).await;
    
    // All operations should succeed (transactions ensure atomicity)
    for result in results {
        assert!(result.is_ok());
        let sync_result = result.unwrap();
        assert!(sync_result.is_ok());
    }
    
    // Final state should be consistent
    let final_directories = db.list_webdav_directories(user_id).await.unwrap();
    
    // Should have 3 directories (dir1, dir2, and one of the new_dir_X)
    assert_eq!(final_directories.len(), 3);
    
    // All ETags should be from one consistent transaction
    let dir1 = final_directories.iter().find(|d| d.directory_path == "/test/dir1").unwrap();
    let dir2 = final_directories.iter().find(|d| d.directory_path == "/test/dir2").unwrap();
    
    // ETags should be from the same transaction (both should end with same number)
    let etag1_suffix = dir1.directory_etag.chars().last().unwrap();
    let etag2_suffix = dir2.directory_etag.chars().last().unwrap();
    assert_eq!(etag1_suffix, etag2_suffix, "ETags should be from same atomic transaction");
}

/// Test that validates directory deletion detection works correctly
#[tokio::test]
async fn test_deletion_detection_fix() {
    let test_context = TestContext::new().await;
    let db = &test_context.state.db;
    
    // Create a test user first
    let create_user = CreateUser {
        username: "deletion_testuser".to_string(),
        email: "deletion@example.com".to_string(),
        password: "password123".to_string(),
        role: Some(UserRole::User),
    };
    let user = db.create_user(create_user).await
        .expect("Failed to create test user");
    let user_id = user.id;
    
    // Create initial directories
    let initial_directories = vec![
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/documents/folder1".to_string(),
            directory_etag: "etag1".to_string(),
            file_count: 5,
            total_size_bytes: 1024,
        },
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/documents/folder2".to_string(),
            directory_etag: "etag2".to_string(),
            file_count: 3,
            total_size_bytes: 512,
        },
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/documents/folder3".to_string(),
            directory_etag: "etag3".to_string(),
            file_count: 8,
            total_size_bytes: 2048,
        },
    ];
    
    let _ = db.bulk_create_or_update_webdav_directories(&initial_directories).await.unwrap();
    
    // Verify all 3 directories exist
    let directories_before = db.list_webdav_directories(user_id).await.unwrap();
    assert_eq!(directories_before.len(), 3);
    
    // Simulate sync where folder2 and folder3 are deleted from WebDAV server
    let current_directories = vec![
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/documents/folder1".to_string(),
            directory_etag: "etag1_updated".to_string(), // Updated
            file_count: 6,
            total_size_bytes: 1200,
        },
        // folder2 and folder3 are missing (deleted from server)
    ];
    
    // Use atomic sync which should detect and remove deleted directories
    let (updated_directories, deleted_count) = db.sync_webdav_directories(user_id, &current_directories).await.unwrap();
    
    // Should have 1 updated directory and 2 deletions
    assert_eq!(updated_directories.len(), 1);
    assert_eq!(deleted_count, 2);
    
    // Verify only folder1 remains with updated ETag
    let final_directories = db.list_webdav_directories(user_id).await.unwrap();
    assert_eq!(final_directories.len(), 1);
    assert_eq!(final_directories[0].directory_path, "/documents/folder1");
    assert_eq!(final_directories[0].directory_etag, "etag1_updated");
    assert_eq!(final_directories[0].file_count, 6);
}

/// Test that validates proper ETag comparison handling
#[tokio::test]
async fn test_etag_comparison_fix() {
    use readur::webdav_xml_parser::{compare_etags, weak_compare_etags, strong_compare_etags};
    
    // Test weak vs strong ETag comparison
    let strong_etag = "\"abc123\"";
    let weak_etag = "W/\"abc123\"";
    let different_etag = "\"def456\"";
    
    // Smart comparison should handle weak/strong equivalence
    assert!(compare_etags(strong_etag, weak_etag), "Smart comparison should match weak and strong with same content");
    assert!(!compare_etags(strong_etag, different_etag), "Smart comparison should reject different content");
    
    // Weak comparison should match regardless of weak/strong
    assert!(weak_compare_etags(strong_etag, weak_etag), "Weak comparison should match");
    assert!(weak_compare_etags(weak_etag, strong_etag), "Weak comparison should be symmetrical");
    
    // Strong comparison should reject weak ETags
    assert!(!strong_compare_etags(strong_etag, weak_etag), "Strong comparison should reject weak ETags");
    assert!(!strong_compare_etags(weak_etag, strong_etag), "Strong comparison should reject weak ETags");
    assert!(strong_compare_etags(strong_etag, "\"abc123\""), "Strong comparison should match strong ETags");
    
    // Test case sensitivity (ETags should be case-sensitive per RFC)
    assert!(!compare_etags("\"ABC123\"", "\"abc123\""), "ETags should be case-sensitive");
    
    // Test various real-world formats
    let nextcloud_etag = "\"5f3e7e8a9b2c1d4\"";
    let apache_etag = "\"1234-567-890abcdef\"";
    let nginx_weak = "W/\"5f3e7e8a\"";
    
    assert!(!compare_etags(nextcloud_etag, apache_etag), "Different ETag values should not match");
    assert!(weak_compare_etags(nginx_weak, "\"5f3e7e8a\""), "Weak and strong with same content should match in weak comparison");
}

/// Test performance of bulk operations vs individual operations
#[tokio::test]
async fn test_bulk_operations_performance() {
    let test_context = TestContext::new().await;
    let db = &test_context.state.db;
    
    // Create a test user first
    let create_user = CreateUser {
        username: "perf_testuser".to_string(),
        email: "perf@example.com".to_string(),
        password: "password123".to_string(),
        role: Some(UserRole::User),
    };
    let user = db.create_user(create_user).await
        .expect("Failed to create test user");
    let user_id = user.id;
    
    // Create test data
    let test_directories: Vec<_> = (0..100).map(|i| CreateWebDAVDirectory {
        user_id,
        directory_path: format!("/test/perf/dir{}", i),
        directory_etag: format!("etag{}", i),
        file_count: i as i64,
        total_size_bytes: (i * 1024) as i64,
    }).collect();
    
    // Test individual operations (old way)
    let start_individual = Instant::now();
    for directory in &test_directories {
        let _ = db.create_or_update_webdav_directory(directory).await;
    }
    let individual_duration = start_individual.elapsed();
    
    // Clear data
    let _ = db.clear_webdav_directories(user_id).await;
    
    // Test bulk operation (new way)
    let start_bulk = Instant::now();
    let _ = db.bulk_create_or_update_webdav_directories(&test_directories).await;
    let bulk_duration = start_bulk.elapsed();
    
    // Bulk should be faster
    assert!(bulk_duration < individual_duration, 
           "Bulk operations should be faster than individual operations. Bulk: {:?}, Individual: {:?}", 
           bulk_duration, individual_duration);
    
    // Verify all data was saved correctly
    let saved_directories = db.list_webdav_directories(user_id).await.unwrap();
    assert_eq!(saved_directories.len(), 100);
}

/// Test transaction rollback behavior
#[tokio::test]
async fn test_transaction_rollback_consistency() {
    let test_context = TestContext::new().await;
    let db = &test_context.state.db;
    
    // Create a test user first
    let create_user = CreateUser {
        username: "rollback_testuser".to_string(),
        email: "rollback@example.com".to_string(),
        password: "password123".to_string(),
        role: Some(UserRole::User),
    };
    let user = db.create_user(create_user).await
        .expect("Failed to create test user");
    let user_id = user.id;
    
    // Create some initial data
    let initial_directory = CreateWebDAVDirectory {
        user_id,
        directory_path: "/test/initial".to_string(),
        directory_etag: "initial_etag".to_string(),
        file_count: 1,
        total_size_bytes: 100,
    };
    
    let _ = db.create_or_update_webdav_directory(&initial_directory).await.unwrap();
    
    // Try to create directories where one has invalid data that should cause rollback
    let directories_with_failure = vec![
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/test/valid1".to_string(),
            directory_etag: "valid_etag1".to_string(),
            file_count: 2,
            total_size_bytes: 200,
        },
        CreateWebDAVDirectory {
            user_id: Uuid::nil(), // This should cause a constraint violation
            directory_path: "/test/invalid".to_string(),
            directory_etag: "invalid_etag".to_string(),
            file_count: 3,
            total_size_bytes: 300,
        },
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/test/valid2".to_string(),
            directory_etag: "valid_etag2".to_string(),
            file_count: 4,
            total_size_bytes: 400,
        },
    ];
    
    // This should fail and rollback
    let result = db.bulk_create_or_update_webdav_directories(&directories_with_failure).await;
    assert!(result.is_err(), "Transaction should fail due to invalid user_id");
    
    // Verify that no partial changes were made - only initial directory should exist
    let final_directories = db.list_webdav_directories(user_id).await.unwrap();
    assert_eq!(final_directories.len(), 1);
    assert_eq!(final_directories[0].directory_path, "/test/initial");
    assert_eq!(final_directories[0].directory_etag, "initial_etag");
}

/// Integration test simulating real WebDAV sync scenario
#[tokio::test]
async fn test_full_sync_integration() {
    let test_context = TestContext::new().await;
    let app_state = &test_context.state;
    
    // Create a test user first
    let create_user = CreateUser {
        username: "sync_testuser".to_string(),
        email: "sync@example.com".to_string(),
        password: "password123".to_string(),
        role: Some(UserRole::User),
    };
    let user = app_state.db.create_user(create_user).await
        .expect("Failed to create test user");
    let user_id = user.id;
    
    // Simulate initial sync with some directories
    let initial_directories = vec![
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/documents".to_string(),
            directory_etag: "docs_etag_v1".to_string(),
            file_count: 10,
            total_size_bytes: 10240,
        },
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/pictures".to_string(),
            directory_etag: "pics_etag_v1".to_string(),
            file_count: 5,
            total_size_bytes: 51200,
        },
    ];
    
    let (saved_dirs, _) = app_state.db.sync_webdav_directories(user_id, &initial_directories).await.unwrap();
    assert_eq!(saved_dirs.len(), 2);
    
    // Simulate second sync with changes
    let updated_directories = vec![
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/documents".to_string(),
            directory_etag: "docs_etag_v2".to_string(), // Changed
            file_count: 12,
            total_size_bytes: 12288,
        },
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/videos".to_string(), // New directory
            directory_etag: "videos_etag_v1".to_string(),
            file_count: 3,
            total_size_bytes: 102400,
        },
        // /pictures directory was deleted from server
    ];
    
    let (updated_dirs, deleted_count) = app_state.db.sync_webdav_directories(user_id, &updated_directories).await.unwrap();
    
    // Should have 2 directories (updated documents + new videos) and 1 deletion (pictures)
    assert_eq!(updated_dirs.len(), 2);
    assert_eq!(deleted_count, 1);
    
    // Verify final state
    let final_dirs = app_state.db.list_webdav_directories(user_id).await.unwrap();
    assert_eq!(final_dirs.len(), 2);
    
    let docs_dir = final_dirs.iter().find(|d| d.directory_path == "/documents").unwrap();
    assert_eq!(docs_dir.directory_etag, "docs_etag_v2");
    assert_eq!(docs_dir.file_count, 12);
    
    let videos_dir = final_dirs.iter().find(|d| d.directory_path == "/videos").unwrap();
    assert_eq!(videos_dir.directory_etag, "videos_etag_v1");
    assert_eq!(videos_dir.file_count, 3);
}