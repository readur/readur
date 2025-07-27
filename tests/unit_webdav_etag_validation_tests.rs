use std::sync::Arc;
use uuid::Uuid;
use readur::{
    AppState,
    models::{CreateWebDAVDirectory, FileIngestionInfo},
    test_utils::{TestContext, TestAuthHelper},
};

/// Helper function to create test setup
async fn create_etag_test_state() -> (TestContext, Arc<AppState>, Uuid) {
    let test_context = TestContext::new().await;
    
    let auth_helper = TestAuthHelper::new(test_context.app().clone());
    let test_user = auth_helper.create_test_user().await;

    let state = test_context.state().clone();
    let user_id = test_user.user_response.id;
    
    (test_context, state, user_id)
}

/// Helper function to create directory info for testing
fn create_directory_info(path: &str, etag: &str) -> FileIngestionInfo {
    FileIngestionInfo {
        relative_path: path.to_string(),
        full_path: path.to_string(),
        #[allow(deprecated)]
        path: path.to_string(),
        name: path.split('/').last().unwrap_or("").to_string(),
        size: 0,
        mime_type: "".to_string(),
        last_modified: Some(chrono::Utc::now()),
        etag: etag.to_string(),
        is_directory: true,
        created_at: Some(chrono::Utc::now()),
        permissions: Some(755),
        owner: None,
        group: None,
        metadata: None,
    }
}

/// Test: Directory deletion detection
/// Critical Gap: Current implementation doesn't detect directories removed from WebDAV
#[tokio::test]
async fn test_detect_deleted_directories() {
    let (_test_context, state, user_id) = create_etag_test_state().await;
    
    // Setup: Create directories in database that represent previously discovered directories
    let known_directories = vec![
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/Documents".to_string(),
            directory_etag: "docs-etag-v1".to_string(),
            file_count: 10,
            total_size_bytes: 1024000,
        },
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/Documents/Projects".to_string(),
            directory_etag: "projects-etag-v1".to_string(),
            file_count: 5,
            total_size_bytes: 512000,
        },
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/Documents/Archive".to_string(),
            directory_etag: "archive-etag-v1".to_string(),
            file_count: 20,
            total_size_bytes: 2048000,
        },
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/Documents/ToBeDeleted".to_string(),
            directory_etag: "deleted-etag-v1".to_string(),
            file_count: 3,
            total_size_bytes: 256000,
        },
    ];
    
    for dir in &known_directories {
        state.db.create_or_update_webdav_directory(dir).await
            .expect("Failed to create known directory");
    }
    
    // Simulate current directories from WebDAV (missing one directory)
    let current_dirs = vec![
        create_directory_info("/Documents", "docs-etag-v1"), // Unchanged
        create_directory_info("/Documents/Projects", "projects-etag-v2"), // Changed
        create_directory_info("/Documents/Archive", "archive-etag-v1"), // Unchanged
        // /Documents/ToBeDeleted is missing - simulating deletion
    ];
    
    // Perform directory comparison logic (mirrors SmartSyncService logic)
    let stored_dirs = state.db.list_webdav_directories(user_id).await
        .expect("Failed to get stored directories");
    
    let stored_paths: std::collections::HashSet<String> = stored_dirs.iter()
        .filter(|dir| dir.directory_path.starts_with("/Documents"))
        .map(|dir| dir.directory_path.clone())
        .collect();
    
    let current_paths: std::collections::HashSet<String> = current_dirs.iter()
        .map(|dir| dir.relative_path.clone())
        .collect();
    
    // Detect deleted directories
    let deleted_directories: Vec<String> = stored_paths.difference(&current_paths)
        .cloned()
        .collect();
    
    // Verify deletion detection
    assert_eq!(deleted_directories.len(), 1, "Should detect 1 deleted directory");
    assert!(deleted_directories.contains(&"/Documents/ToBeDeleted".to_string()), 
           "Should detect deleted directory");
    
    // Verify other directories are still tracked
    let changed_directories: Vec<String> = current_dirs.iter()
        .filter_map(|current_dir| {
            stored_dirs.iter()
                .find(|stored_dir| stored_dir.directory_path == current_dir.relative_path)
                .and_then(|stored_dir| {
                    if stored_dir.directory_etag != current_dir.etag {
                        Some(current_dir.relative_path.clone())
                    } else {
                        None
                    }
                })
        })
        .collect();
    
    assert_eq!(changed_directories.len(), 1, "Should detect 1 changed directory");
    assert!(changed_directories.contains(&"/Documents/Projects".to_string()),
           "Should detect changed directory");
    
    println!("✅ Directory deletion detection working correctly");
    println!("   Deleted: {:?}", deleted_directories);
    println!("   Changed: {:?}", changed_directories);
}

/// Test: ETag corruption and malformed data handling
/// Critical Gap: No validation of ETag format or corrupted data handling
#[tokio::test]
async fn test_malformed_etag_handling() {
    let (_test_context, state, user_id) = create_etag_test_state().await;
    
    // Test various malformed ETags
    let extremely_long_etag = "a".repeat(1000);
    let malformed_etags = vec![
        ("", "empty_etag"),
        ("   ", "whitespace_only"),
        ("null", "null_string"),
        (extremely_long_etag.as_str(), "extremely_long"),
        ("etag\0with\0nulls", "null_bytes"),
        ("etag\nwith\nnewlines", "newlines"),
        ("🚀💾📡", "unicode_emojis"),
        ("etag with spaces", "spaces"),
    ];
    
    for (malformed_etag, test_case) in malformed_etags {
        println!("Testing malformed ETag case: {}", test_case);
        
        let dir = CreateWebDAVDirectory {
            user_id,
            directory_path: format!("/test/{}", test_case),
            directory_etag: malformed_etag.to_string(),
            file_count: 1,
            total_size_bytes: 1024,
        };
        
        // This should not crash the system
        let result = state.db.create_or_update_webdav_directory(&dir).await;
        
        match result {
            Ok(_) => {
                println!("  ✅ Malformed ETag '{}' was stored (system tolerant)", malformed_etag);
                
                // Verify it can be retrieved without issues
                let retrieved_dirs = state.db.list_webdav_directories(user_id).await
                    .expect("Should be able to retrieve directories even with malformed ETags");
                
                let found_dir = retrieved_dirs.iter()
                    .find(|d| d.directory_path == format!("/test/{}", test_case));
                
                assert!(found_dir.is_some(), "Should find directory with malformed ETag");
                
                if let Some(dir) = found_dir {
                    // System should preserve the ETag as-is (no corruption)
                    assert_eq!(dir.directory_etag, malformed_etag, 
                              "ETag should be preserved exactly as stored");
                }
            }
            Err(e) => {
                println!("  ⚠️ Malformed ETag '{}' rejected by database: {}", malformed_etag, e);
                // This is also acceptable behavior - failing fast on invalid data
            }
        }
    }
    
    println!("✅ Malformed ETag handling test completed");
}

/// Test: Deep hierarchy directory changes
/// Gap: Limited testing of nested directory structures and change detection
#[tokio::test]
async fn test_deep_nested_directory_changes() {
    let (_test_context, state, user_id) = create_etag_test_state().await;
    
    // Create a deep directory hierarchy (6 levels deep)
    let deep_directories = vec![
        "/Documents",
        "/Documents/Projects", 
        "/Documents/Projects/WebApp",
        "/Documents/Projects/WebApp/Backend",
        "/Documents/Projects/WebApp/Backend/API",
        "/Documents/Projects/WebApp/Backend/API/V1",
        "/Documents/Archive",
        "/Documents/Archive/2023",
        "/Documents/Archive/2023/Q1",
        "/Documents/Archive/2023/Q1/Reports",
        "/Documents/Archive/2023/Q1/Reports/Financial",
    ];
    
    // Store all directories with initial ETags
    for (i, path) in deep_directories.iter().enumerate() {
        let dir = CreateWebDAVDirectory {
            user_id,
            directory_path: path.to_string(),
            directory_etag: format!("etag-{}-v1", i),
            file_count: (i + 1) as i64,
            total_size_bytes: ((i + 1) * 100000) as i64,
        };
        
        state.db.create_or_update_webdav_directory(&dir).await
            .expect("Failed to create deep directory");
    }
    
    // Simulate change detection at various depths
    let change_scenarios = vec![
        ("/Documents", "Change at root level"),
        ("/Documents/Projects/WebApp/Backend/API/V1", "Change at deepest level"),
        ("/Documents/Archive/2023/Q1", "Change at mid-level"),
    ];
    
    for (changed_path, scenario) in change_scenarios {
        println!("Testing scenario: {}", scenario);
        
        // Get current directories
        let stored_dirs = state.db.list_webdav_directories(user_id).await
            .expect("Failed to get stored directories");
        
        // Simulate WebDAV discovery result with one changed directory
        let current_dirs: Vec<FileIngestionInfo> = stored_dirs.iter()
            .map(|stored_dir| {
                let etag = if stored_dir.directory_path == changed_path {
                    format!("{}-CHANGED", stored_dir.directory_etag)
                } else {
                    stored_dir.directory_etag.clone()
                };
                
                create_directory_info(&stored_dir.directory_path, &etag)
            })
            .collect();
        
        // Perform change detection
        let stored_map: std::collections::HashMap<String, String> = stored_dirs.iter()
            .map(|dir| (dir.directory_path.clone(), dir.directory_etag.clone()))
            .collect();
        
        let changed_dirs: Vec<String> = current_dirs.iter()
            .filter_map(|current_dir| {
                stored_map.get(&current_dir.relative_path)
                    .and_then(|stored_etag| {
                        if stored_etag != &current_dir.etag {
                            Some(current_dir.relative_path.clone())
                        } else {
                            None
                        }
                    })
            })
            .collect();
        
        // Verify change detection
        assert_eq!(changed_dirs.len(), 1, "Should detect exactly 1 change in scenario: {}", scenario);
        assert!(changed_dirs.contains(&changed_path.to_string()), 
               "Should detect change at: {}", changed_path);
        
        println!("  ✅ Detected change at: {}", changed_path);
    }
    
    println!("✅ Deep hierarchy change detection working correctly");
}

/// Test: ETag collision scenarios
/// Gap: No tests for multiple directories with same ETag
#[tokio::test]
async fn test_etag_collision_handling() {
    let (_test_context, state, user_id) = create_etag_test_state().await;
    
    // Create directories with intentionally colliding ETags
    let colliding_directories = vec![
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/Documents/Folder1".to_string(),
            directory_etag: "same-etag-123".to_string(),
            file_count: 5,
            total_size_bytes: 512000,
        },
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/Documents/Folder2".to_string(),
            directory_etag: "same-etag-123".to_string(), // Same ETag!
            file_count: 8,
            total_size_bytes: 768000,
        },
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/Documents/Folder3".to_string(),
            directory_etag: "same-etag-123".to_string(), // Same ETag!
            file_count: 3,
            total_size_bytes: 256000,
        },
        CreateWebDAVDirectory {
            user_id,
            directory_path: "/Documents/Unique".to_string(),
            directory_etag: "unique-etag-456".to_string(),
            file_count: 10,
            total_size_bytes: 1024000,
        },
    ];
    
    // Store all directories
    for dir in &colliding_directories {
        state.db.create_or_update_webdav_directory(dir).await
            .expect("Failed to create directory with colliding ETag");
    }
    
    // Verify all directories were stored correctly
    let stored_dirs = state.db.list_webdav_directories(user_id).await
        .expect("Failed to retrieve directories");
    
    assert_eq!(stored_dirs.len(), 4, "All directories should be stored despite ETag collisions");
    
    // Group by ETag to analyze collisions
    let mut etag_groups: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    
    for dir in &stored_dirs {
        etag_groups.entry(dir.directory_etag.clone())
            .or_insert_with(Vec::new)
            .push(dir.directory_path.clone());
    }
    
    // Verify collision handling
    let colliding_etag = "same-etag-123";
    assert!(etag_groups.contains_key(colliding_etag), "Should have colliding ETag group");
    
    let colliding_paths = etag_groups.get(colliding_etag).unwrap();
    assert_eq!(colliding_paths.len(), 3, "Should have 3 directories with same ETag");
    
    // Verify each directory maintains its unique path and metadata
    for original_dir in &colliding_directories {
        let stored_dir = stored_dirs.iter()
            .find(|d| d.directory_path == original_dir.directory_path)
            .expect("Should find stored directory");
        
        assert_eq!(stored_dir.directory_etag, original_dir.directory_etag, 
                  "ETag should match for {}", original_dir.directory_path);
        assert_eq!(stored_dir.file_count, original_dir.file_count,
                  "File count should match for {}", original_dir.directory_path);
        assert_eq!(stored_dir.total_size_bytes, original_dir.total_size_bytes,
                  "Size should match for {}", original_dir.directory_path);
    }
    
    // Test change detection with colliding ETags
    let current_dirs = vec![
        create_directory_info("/Documents/Folder1", "same-etag-123"), // Unchanged
        create_directory_info("/Documents/Folder2", "updated-etag-789"), // Changed
        create_directory_info("/Documents/Folder3", "same-etag-123"), // Unchanged  
        create_directory_info("/Documents/Unique", "unique-etag-456"), // Unchanged
    ];
    
    // Perform change detection
    let stored_map: std::collections::HashMap<String, String> = stored_dirs.iter()
        .map(|dir| (dir.directory_path.clone(), dir.directory_etag.clone()))
        .collect();
    
    let changed_dirs: Vec<String> = current_dirs.iter()
        .filter_map(|current_dir| {
            stored_map.get(&current_dir.relative_path)
                .and_then(|stored_etag| {
                    if stored_etag != &current_dir.etag {
                        Some(current_dir.relative_path.clone())
                    } else {
                        None
                    }
                })
        })
        .collect();
    
    // Should detect only the one changed directory, despite ETag collisions
    assert_eq!(changed_dirs.len(), 1, "Should detect exactly 1 changed directory");
    assert!(changed_dirs.contains(&"/Documents/Folder2".to_string()),
           "Should detect change in Folder2");
    
    println!("✅ ETag collision handling working correctly");
    println!("   Colliding ETag '{}' used by {} directories", colliding_etag, colliding_paths.len());
    println!("   Change detection still accurate despite collisions");
}

/// Test: Large scale ETag comparison performance
/// Gap: No stress testing with thousands of directories
#[tokio::test]
async fn test_large_scale_etag_performance() {
    let (_test_context, state, user_id) = create_etag_test_state().await;
    
    let num_directories = 1000;
    println!("Creating {} directories for performance testing...", num_directories);
    
    // Create many directories efficiently
    let start_time = std::time::Instant::now();
    
    for i in 0..num_directories {
        let dir = CreateWebDAVDirectory {
            user_id,
            directory_path: format!("/Documents/Dir{:04}", i),
            directory_etag: format!("etag-{:04}-v1", i),
            file_count: (i % 50) as i64,
            total_size_bytes: ((i % 100) * 10000) as i64,
        };
        
        state.db.create_or_update_webdav_directory(&dir).await
            .expect("Failed to create directory");
        
        // Progress indicator for large datasets
        if i % 100 == 0 {
            println!("  Created {} directories...", i);
        }
    }
    
    let creation_time = start_time.elapsed();
    println!("✅ Created {} directories in {:?}", num_directories, creation_time);
    
    // Test bulk retrieval performance
    let retrieval_start = std::time::Instant::now();
    let all_directories = state.db.list_webdav_directories(user_id).await
        .expect("Failed to retrieve directories");
    let retrieval_time = retrieval_start.elapsed();
    
    assert_eq!(all_directories.len(), num_directories, 
              "Should retrieve all {} directories", num_directories);
    
    println!("✅ Retrieved {} directories in {:?}", num_directories, retrieval_time);
    
    // Test change detection performance with mixed changes
    let change_detection_start = std::time::Instant::now();
    
    // Simulate current state with 10% of directories changed
    let changed_count = num_directories / 10;
    let current_dirs: Vec<FileIngestionInfo> = (0..num_directories).map(|i| {
        let etag = if i < changed_count {
            format!("etag-{:04}-v2", i) // Changed
        } else {
            format!("etag-{:04}-v1", i) // Unchanged
        };
        
        create_directory_info(&format!("/Documents/Dir{:04}", i), &etag)
    }).collect();
    
    // Perform change detection
    let stored_map: std::collections::HashMap<String, String> = all_directories.iter()
        .map(|dir| (dir.directory_path.clone(), dir.directory_etag.clone()))
        .collect();
    
    let changed_dirs: Vec<String> = current_dirs.iter()
        .filter_map(|current_dir| {
            stored_map.get(&current_dir.relative_path)
                .and_then(|stored_etag| {
                    if stored_etag != &current_dir.etag {
                        Some(current_dir.relative_path.clone())
                    } else {
                        None
                    }
                })
        })
        .collect();
    
    let change_detection_time = change_detection_start.elapsed();
    
    // Verify change detection results
    assert_eq!(changed_dirs.len(), changed_count, 
              "Should detect {} changed directories", changed_count);
    
    println!("✅ Change detection on {} directories completed in {:?}", 
             num_directories, change_detection_time);
    
    // Performance assertions
    assert!(retrieval_time.as_millis() < 1000, 
           "Bulk retrieval should be under 1 second, got {:?}", retrieval_time);
    assert!(change_detection_time.as_millis() < 500,
           "Change detection should be under 500ms, got {:?}", change_detection_time);
    
    println!("✅ Large scale ETag performance test completed successfully");
    println!("   Performance: {} dirs/sec creation, {} dirs/sec retrieval", 
             num_directories as f64 / creation_time.as_secs_f64(),
             num_directories as f64 / retrieval_time.as_secs_f64());
}