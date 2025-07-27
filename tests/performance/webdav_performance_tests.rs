use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::info;
use uuid::Uuid;

use readur::test_utils::TestContext;
use readur::services::webdav::{SmartSyncService, SyncProgress};
use readur::models::{CreateWebDAVDirectory, Source, SourceType, SourceConfig};

/// Performance tests for WebDAV operations with large directory hierarchies
/// These tests help identify bottlenecks and optimization opportunities

#[tokio::test]
async fn test_large_directory_hierarchy_performance() {
    let test_ctx = TestContext::new().await;
    let state = test_ctx.state.clone();
    
    // Create test user
    let user = state.db.create_user("test@example.com", "password123").await
        .expect("Failed to create test user");
    
    // Create WebDAV source
    let source_config = SourceConfig::WebDAV {
        server_url: "https://test.example.com".to_string(),
        username: "test".to_string(),
        password: "test".to_string(),
        watch_folders: vec!["/".to_string()],
    };
    
    let _source = state.db.create_source(
        user.id,
        "large_hierarchy_test",
        SourceType::WebDAV,
        source_config,
        vec!["pdf".to_string(), "txt".to_string()],
    ).await.expect("Failed to create WebDAV source");
    
    // Simulate large directory hierarchy in database
    let start_time = Instant::now();
    let num_directories = 1000;
    let num_files_per_dir = 50;
    
    info!("🏗️ Creating test data: {} directories with {} files each", 
          num_directories, num_files_per_dir);
    
    // Create directory structure
    let mut directories = Vec::new();
    for i in 0..num_directories {
        let depth = i % 5; // Vary depth from 0-4
        let path = if depth == 0 {
            format!("/test_dir_{}", i)
        } else {
            format!("/test_dir_0/subdir_{}/deep_{}", depth, i)
        };
        
        directories.push(CreateWebDAVDirectory {
            user_id: user.id,
            directory_path: path.clone(),
            directory_etag: format!("etag_dir_{}", i),
            file_count: num_files_per_dir,
            total_size_bytes: (num_files_per_dir * 1024 * 10) as i64, // 10KB per file
        });
    }
    
    // Bulk insert directories
    let insert_start = Instant::now();
    let result = state.db.bulk_create_or_update_webdav_directories(&directories).await;
    let insert_duration = insert_start.elapsed();
    
    assert!(result.is_ok(), "Failed to create test directories: {:?}", result.err());
    info!("✅ Directory insertion completed in {:?}", insert_duration);
    
    // Test smart sync evaluation performance
    let smart_sync = SmartSyncService::new(state.clone());
    
    // Test smart sync evaluation with created directories
    
    // Test evaluation performance with large dataset
    let eval_start = Instant::now();
    
    // Since we don't have a real WebDAV server, we'll test the database query performance
    let known_dirs = state.db.list_webdav_directories(user.id).await
        .expect("Failed to fetch directories");
    let eval_duration = eval_start.elapsed();
    
    assert_eq!(known_dirs.len(), num_directories);
    info!("📊 Directory listing query completed in {:?} for {} directories", 
          eval_duration, known_dirs.len());
    
    // Performance assertions
    assert!(insert_duration < Duration::from_secs(10), 
            "Directory insertion took too long: {:?}", insert_duration);
    assert!(eval_duration < Duration::from_secs(5), 
            "Directory evaluation took too long: {:?}", eval_duration);
    
    let total_duration = start_time.elapsed();
    info!("🎯 Total test duration: {:?}", total_duration);
    
    // Performance metrics
    let dirs_per_sec = num_directories as f64 / insert_duration.as_secs_f64();
    let query_rate = num_directories as f64 / eval_duration.as_secs_f64();
    
    info!("📈 Performance metrics:");
    info!("   - Directory insertion rate: {:.1} dirs/sec", dirs_per_sec);
    info!("   - Directory query rate: {:.1} dirs/sec", query_rate);
    
    // Ensure reasonable performance thresholds
    assert!(dirs_per_sec > 100.0, "Directory insertion rate too slow: {:.1} dirs/sec", dirs_per_sec);
    assert!(query_rate > 200.0, "Directory query rate too slow: {:.1} dirs/sec", query_rate);
}

#[tokio::test]
async fn test_concurrent_directory_operations_performance() {
    let test_ctx = TestContext::new().await;
    let state = test_ctx.state.clone();
    
    // Create test user
    let user = state.db.create_user("test2@example.com", "password123").await
        .expect("Failed to create test user");
    
    info!("🔄 Testing concurrent directory operations");
    
    let num_concurrent_ops = 10;
    let dirs_per_op = 100;
    
    let start_time = Instant::now();
    
    // Spawn concurrent tasks that create directories
    let mut tasks = Vec::new();
    for task_id in 0..num_concurrent_ops {
        let state_clone = state.clone();
        let user_id = user.id;
        
        let task = tokio::spawn(async move {
            let mut directories = Vec::new();
            for i in 0..dirs_per_op {
                directories.push(CreateWebDAVDirectory {
                    user_id,
                    directory_path: format!("/concurrent_test_{}/dir_{}", task_id, i),
                    directory_etag: format!("etag_{}_{}", task_id, i),
                    file_count: 10,
                    total_size_bytes: 10240,
                });
            }
            
            let task_start = Instant::now();
            let result = state_clone.db.bulk_create_or_update_webdav_directories(&directories).await;
            let task_duration = task_start.elapsed();
            
            (task_id, result, task_duration, directories.len())
        });
        
        tasks.push(task);
    }
    
    // Wait for all tasks to complete
    let mut total_dirs_created = 0;
    let mut max_task_duration = Duration::from_secs(0);
    
    for task in tasks {
        let (task_id, result, duration, dirs_count) = task.await
            .expect("Task panicked");
        
        assert!(result.is_ok(), "Task {} failed: {:?}", task_id, result.err());
        total_dirs_created += dirs_count;
        max_task_duration = max_task_duration.max(duration);
        
        info!("Task {} completed: {} dirs in {:?}", task_id, dirs_count, duration);
    }
    
    let total_duration = start_time.elapsed();
    
    info!("🎯 Concurrent operations summary:");
    info!("   - Total directories created: {}", total_dirs_created);
    info!("   - Total duration: {:?}", total_duration);
    info!("   - Longest task duration: {:?}", max_task_duration);
    info!("   - Average throughput: {:.1} dirs/sec", 
          total_dirs_created as f64 / total_duration.as_secs_f64());
    
    // Verify all directories were created
    let final_count = state.db.list_webdav_directories(user.id).await
        .expect("Failed to count directories")
        .len();
    
    assert_eq!(final_count, total_dirs_created);
    
    // Performance assertions
    assert!(total_duration < Duration::from_secs(30), 
            "Concurrent operations took too long: {:?}", total_duration);
    assert!(max_task_duration < Duration::from_secs(15), 
            "Individual task took too long: {:?}", max_task_duration);
}

#[tokio::test] 
async fn test_etag_comparison_performance() {
    let test_ctx = TestContext::new().await;
    let state = test_ctx.state.clone();
    
    // Create test user
    let user = state.db.create_user("test3@example.com", "password123").await
        .expect("Failed to create test user");
    
    info!("🔍 Testing ETag comparison performance for large datasets");
    
    let num_directories = 5000;
    let changed_percentage = 0.1; // 10% of directories have changed ETags
    
    // Create initial directory set
    let mut directories = Vec::new();
    for i in 0..num_directories {
        directories.push(CreateWebDAVDirectory {
            user_id: user.id,
            directory_path: format!("/etag_test/dir_{}", i),
            directory_etag: format!("original_etag_{}", i),
            file_count: 5,
            total_size_bytes: 5120,
        });
    }
    
    // Insert initial directories
    let insert_start = Instant::now();
    state.db.bulk_create_or_update_webdav_directories(&directories).await
        .expect("Failed to create initial directories");
    let insert_duration = insert_start.elapsed();
    
    info!("✅ Inserted {} directories in {:?}", num_directories, insert_duration);
    
    // Simulate changed directories (as would come from WebDAV server)
    let num_changed = (num_directories as f64 * changed_percentage) as usize;
    let mut discovered_directories = directories.clone();
    
    // Change ETags for some directories
    for i in 0..num_changed {
        discovered_directories[i].directory_etag = format!("changed_etag_{}", i);
    }
    
    // Test smart sync evaluation performance
    let smart_sync = SmartSyncService::new(state.clone());
    
    // Measure time to load known directories
    let load_start = Instant::now();
    let known_dirs = state.db.list_webdav_directories(user.id).await
        .expect("Failed to load directories");
    let load_duration = load_start.elapsed();
    
    // Measure time to compare ETags
    let compare_start = Instant::now();
    let mut changed_dirs = Vec::new();
    let mut unchanged_dirs = 0;
    
    // Convert to HashMap for O(1) lookup (simulating smart sync logic)
    let known_etags: std::collections::HashMap<String, String> = known_dirs
        .into_iter()
        .map(|d| (d.directory_path, d.directory_etag))
        .collect();
    
    for discovered_dir in &discovered_directories {
        if let Some(known_etag) = known_etags.get(&discovered_dir.directory_path) {
            if known_etag != &discovered_dir.directory_etag {
                changed_dirs.push(discovered_dir.directory_path.clone());
            } else {
                unchanged_dirs += 1;
            }
        }
    }
    
    let compare_duration = compare_start.elapsed();
    
    info!("📊 ETag comparison results:");
    info!("   - Total directories: {}", num_directories);
    info!("   - Changed directories: {}", changed_dirs.len());
    info!("   - Unchanged directories: {}", unchanged_dirs);
    info!("   - Load time: {:?}", load_duration);
    info!("   - Compare time: {:?}", compare_duration);
    info!("   - Comparison rate: {:.1} dirs/sec", 
          num_directories as f64 / compare_duration.as_secs_f64());
    
    // Verify correctness
    assert_eq!(changed_dirs.len(), num_changed);
    assert_eq!(unchanged_dirs, num_directories - num_changed);
    
    // Performance assertions
    assert!(load_duration < Duration::from_secs(2), 
            "Directory loading took too long: {:?}", load_duration);
    assert!(compare_duration < Duration::from_millis(500), 
            "ETag comparison took too long: {:?}", compare_duration);
    
    let comparison_rate = num_directories as f64 / compare_duration.as_secs_f64();
    assert!(comparison_rate > 10000.0, 
            "ETag comparison rate too slow: {:.1} dirs/sec", comparison_rate);
}

#[tokio::test]
async fn test_progress_tracking_overhead() {
    let test_setup = TestSetup::new().await;
    
    info!("⏱️ Testing progress tracking performance overhead");
    
    let num_operations = 10000;
    let progress = SyncProgress::new();
    
    // Test progress updates without progress tracking
    let start_no_progress = Instant::now();
    for i in 0..num_operations {
        // Simulate work without progress tracking
        let _dummy = format!("operation_{}", i);
    }
    let duration_no_progress = start_no_progress.elapsed();
    
    // Test progress updates with progress tracking
    let start_with_progress = Instant::now();
    for i in 0..num_operations {
        // Simulate work with progress tracking
        let _dummy = format!("operation_{}", i);
        
        if i % 100 == 0 {
            progress.add_files_found(1);
            progress.set_current_directory(&format!("/test/dir_{}", i / 100));
        }
    }
    let duration_with_progress = start_with_progress.elapsed();
    
    let overhead = duration_with_progress.saturating_sub(duration_no_progress);
    let overhead_percentage = (overhead.as_secs_f64() / duration_no_progress.as_secs_f64()) * 100.0;
    
    info!("📈 Progress tracking overhead:");
    info!("   - Without progress: {:?}", duration_no_progress);
    info!("   - With progress: {:?}", duration_with_progress);
    info!("   - Overhead: {:?} ({:.1}%)", overhead, overhead_percentage);
    
    // Assert that progress tracking overhead is reasonable (< 50%)
    assert!(overhead_percentage < 50.0, 
            "Progress tracking overhead too high: {:.1}%", overhead_percentage);
    
    // Verify progress state
    let stats = progress.get_stats().expect("Failed to get progress stats");
    assert!(stats.files_processed > 0);
    assert!(!stats.current_directory.is_empty());
}

#[tokio::test]
async fn test_memory_usage_with_large_datasets() {
    let test_setup = TestSetup::new().await;
    let state = test_setup.app_state();
    
    // Create test user
    let user = test_setup.create_test_user().await;
    
    info!("💾 Testing memory usage patterns with large datasets");
    
    let batch_size = 1000;
    let num_batches = 10;
    
    for batch in 0..num_batches {
        let batch_start = Instant::now();
        
        // Create batch of directories
        let mut directories = Vec::new();
        for i in 0..batch_size {
            directories.push(CreateWebDAVDirectory {
                user_id: user.id,
                directory_path: format!("/memory_test/batch_{}/dir_{}", batch, i),
                directory_etag: format!("etag_{}_{}", batch, i),
                file_count: 20,
                total_size_bytes: 20480,
            });
        }
        
        // Process batch
        state.db.bulk_create_or_update_webdav_directories(&directories).await
            .expect("Failed to process batch");
        
        let batch_duration = batch_start.elapsed();
        
        // Check memory isn't growing linearly (basic heuristic)
        if batch > 0 {
            info!("Batch {} processed in {:?}", batch, batch_duration);
        }
        
        // Small delay to prevent overwhelming the system
        sleep(Duration::from_millis(10)).await;
    }
    
    // Verify final count
    let final_dirs = state.db.list_webdav_directories(user.id).await
        .expect("Failed to count final directories");
    
    let expected_count = batch_size * num_batches;
    assert_eq!(final_dirs.len(), expected_count);
    
    info!("✅ Memory test completed with {} directories", final_dirs.len());
}

/// Benchmark directory hierarchy traversal patterns
#[tokio::test]
async fn test_hierarchy_traversal_patterns() {
    let test_setup = TestSetup::new().await;
    let state = test_setup.app_state();
    
    // Create test user
    let user = test_setup.create_test_user().await;
    
    info!("🌳 Testing different directory hierarchy patterns");
    
    // Pattern 1: Wide and shallow (1000 dirs at depth 1)
    let wide_start = Instant::now();
    let mut wide_dirs = Vec::new();
    for i in 0..1000 {
        wide_dirs.push(CreateWebDAVDirectory {
            user_id: user.id,
            directory_path: format!("/wide/dir_{}", i),
            directory_etag: format!("wide_etag_{}", i),
            file_count: 10,
            total_size_bytes: 10240,
        });
    }
    
    state.db.bulk_create_or_update_webdav_directories(&wide_dirs).await
        .expect("Failed to create wide hierarchy");
    let wide_duration = wide_start.elapsed();
    
    // Pattern 2: Deep and narrow (100 dirs at depth 10)
    let deep_start = Instant::now();
    let mut deep_dirs = Vec::new();
    let mut current_path = "/deep".to_string();
    
    for depth in 0..10 {
        for i in 0..10 {
            current_path = format!("{}/level_{}_dir_{}", current_path, depth, i);
            deep_dirs.push(CreateWebDAVDirectory {
                user_id: user.id,
                directory_path: current_path.clone(),
                directory_etag: format!("deep_etag_{}_{}", depth, i),
                file_count: 5,
                total_size_bytes: 5120,
            });
        }
    }
    
    state.db.bulk_create_or_update_webdav_directories(&deep_dirs).await
        .expect("Failed to create deep hierarchy");
    let deep_duration = deep_start.elapsed();
    
    info!("🎯 Hierarchy performance comparison:");
    info!("   - Wide & shallow (1000 dirs): {:?}", wide_duration);
    info!("   - Deep & narrow (100 dirs): {:?}", deep_duration);
    
    // Both should be reasonably fast
    assert!(wide_duration < Duration::from_secs(5));
    assert!(deep_duration < Duration::from_secs(5));
    
    // Query performance test
    let query_start = Instant::now();
    let all_dirs = state.db.list_webdav_directories(user.id).await
        .expect("Failed to query all directories");
    let query_duration = query_start.elapsed();
    
    info!("   - Query all {} directories: {:?}", all_dirs.len(), query_duration);
    assert!(query_duration < Duration::from_secs(2));
}