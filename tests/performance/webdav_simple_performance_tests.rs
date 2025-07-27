use std::time::{Duration, Instant};
use tracing::info;

use readur::test_utils::TestContext;
use readur::services::webdav::{SmartSyncService, SyncProgress};
use readur::models::{CreateWebDAVDirectory, CreateUser, UserRole};

/// Simplified performance tests for WebDAV operations
/// These tests establish baseline performance metrics for large-scale operations

#[tokio::test]
async fn test_directory_insertion_performance() {
    let test_ctx = TestContext::new().await;
    let state = test_ctx.state.clone();
    
    // Create test user
    let user_data = CreateUser {
        username: "perf_test".to_string(),
        email: "perf_test@example.com".to_string(),
        password: "password123".to_string(),
        role: Some(UserRole::User),
    };
    let user = state.db.create_user(user_data).await
        .expect("Failed to create test user");
    
    println!("🏗️ Testing directory insertion performance");
    
    let num_directories = 1000;
    let start_time = Instant::now();
    
    // Create directory structure
    let mut directories = Vec::new();
    for i in 0..num_directories {
        directories.push(CreateWebDAVDirectory {
            user_id: user.id,
            directory_path: format!("/perf_test/dir_{}", i),
            directory_etag: format!("etag_{}", i),
            file_count: 10,
            total_size_bytes: 10240,
        });
    }
    
    // Bulk insert directories
    let insert_start = Instant::now();
    let result = state.db.bulk_create_or_update_webdav_directories(&directories).await;
    let insert_duration = insert_start.elapsed();
    
    assert!(result.is_ok(), "Failed to create directories: {:?}", result.err());
    
    // Test directory listing performance
    let query_start = Instant::now();
    let fetched_dirs = state.db.list_webdav_directories(user.id).await
        .expect("Failed to fetch directories");
    let query_duration = query_start.elapsed();
    
    let total_duration = start_time.elapsed();
    
    // Performance metrics
    let insert_rate = num_directories as f64 / insert_duration.as_secs_f64();
    let query_rate = fetched_dirs.len() as f64 / query_duration.as_secs_f64();
    
    println!("📊 Directory performance results:");
    println!("   - Directories created: {}", num_directories);
    println!("   - Directories fetched: {}", fetched_dirs.len());
    println!("   - Insert time: {:?} ({:.1} dirs/sec)", insert_duration, insert_rate);
    println!("   - Query time: {:?} ({:.1} dirs/sec)", query_duration, query_rate);
    println!("   - Total time: {:?}", total_duration);
    
    // Verify correctness
    assert_eq!(fetched_dirs.len(), num_directories);
    
    // Performance assertions (reasonable thresholds)
    assert!(insert_duration < Duration::from_secs(5), 
            "Insert took too long: {:?}", insert_duration);
    assert!(query_duration < Duration::from_secs(2), 
            "Query took too long: {:?}", query_duration);
    assert!(insert_rate > 200.0, 
            "Insert rate too slow: {:.1} dirs/sec", insert_rate);
    assert!(query_rate > 500.0, 
            "Query rate too slow: {:.1} dirs/sec", query_rate);
}

#[tokio::test]
async fn test_etag_comparison_performance() {
    let test_ctx = TestContext::new().await;
    let state = test_ctx.state.clone();
    
    // Create test user
    let user_data = CreateUser {
        username: "etag_test".to_string(),
        email: "etag_test@example.com".to_string(),
        password: "password123".to_string(),
        role: Some(UserRole::User),
    };
    let user = state.db.create_user(user_data).await
        .expect("Failed to create test user");
    
    println!("🔍 Testing ETag comparison performance");
    
    let num_directories = 2000;
    let changed_count = 200; // 10% changed
    
    // Create initial directories
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
    
    // Insert directories
    state.db.bulk_create_or_update_webdav_directories(&directories).await
        .expect("Failed to insert directories");
    
    // Load directories for comparison
    let load_start = Instant::now();
    let known_dirs = state.db.list_webdav_directories(user.id).await
        .expect("Failed to load directories");
    let load_duration = load_start.elapsed();
    
    // Create comparison data (simulating discovered directories)
    let mut discovered_dirs = directories.clone();
    for i in 0..changed_count {
        discovered_dirs[i].directory_etag = format!("changed_etag_{}", i);
    }
    
    // Perform ETag comparison
    let compare_start = Instant::now();
    
    // Convert to HashMap for efficient lookup
    let known_etags: std::collections::HashMap<String, String> = known_dirs
        .into_iter()
        .map(|d| (d.directory_path, d.directory_etag))
        .collect();
    
    let mut changed_dirs = 0;
    let mut unchanged_dirs = 0;
    
    for discovered in &discovered_dirs {
        if let Some(known_etag) = known_etags.get(&discovered.directory_path) {
            if known_etag != &discovered.directory_etag {
                changed_dirs += 1;
            } else {
                unchanged_dirs += 1;
            }
        }
    }
    
    let compare_duration = compare_start.elapsed();
    
    println!("📊 ETag comparison results:");
    println!("   - Total directories: {}", num_directories);
    println!("   - Changed detected: {}", changed_dirs);
    println!("   - Unchanged detected: {}", unchanged_dirs);
    println!("   - Load time: {:?}", load_duration);
    println!("   - Compare time: {:?}", compare_duration);
    println!("   - Comparison rate: {:.1} dirs/sec", 
          num_directories as f64 / compare_duration.as_secs_f64());
    
    // Verify correctness
    assert_eq!(changed_dirs, changed_count);
    assert_eq!(unchanged_dirs, num_directories - changed_count);
    
    // Performance assertions
    assert!(load_duration < Duration::from_secs(2));
    assert!(compare_duration < Duration::from_millis(100)); // Very fast operation
    
    let comparison_rate = num_directories as f64 / compare_duration.as_secs_f64();
    assert!(comparison_rate > 20000.0, 
            "Comparison rate too slow: {:.1} dirs/sec", comparison_rate);
}

#[tokio::test]
async fn test_progress_tracking_performance() {
    println!("⏱️ Testing progress tracking performance overhead");
    
    let num_operations = 5000;
    let progress = SyncProgress::new();
    
    // Test without progress tracking
    let start_no_progress = Instant::now();
    for i in 0..num_operations {
        let _work = format!("operation_{}", i);
    }
    let duration_no_progress = start_no_progress.elapsed();
    
    // Test with progress tracking
    let start_with_progress = Instant::now();
    for i in 0..num_operations {
        let _work = format!("operation_{}", i);
        
        if i % 50 == 0 {
            progress.add_files_found(1);
            progress.set_current_directory(&format!("/test/dir_{}", i / 50));
        }
    }
    let duration_with_progress = start_with_progress.elapsed();
    
    let overhead = duration_with_progress.saturating_sub(duration_no_progress);
    let overhead_percentage = if duration_no_progress.as_nanos() > 0 {
        (overhead.as_nanos() as f64 / duration_no_progress.as_nanos() as f64) * 100.0
    } else {
        0.0
    };
    
    println!("📈 Progress tracking overhead analysis:");
    println!("   - Operations: {}", num_operations);
    println!("   - Without progress: {:?}", duration_no_progress);
    println!("   - With progress: {:?}", duration_with_progress);
    println!("   - Overhead: {:?} ({:.1}%)", overhead, overhead_percentage);
    
    // Verify progress was tracked
    let stats = progress.get_stats().expect("Failed to get progress stats");
    assert!(stats.files_found > 0);
    
    // Performance assertion - overhead should be minimal
    assert!(overhead_percentage < 100.0, 
            "Progress tracking overhead too high: {:.1}%", overhead_percentage);
}

#[tokio::test] 
async fn test_smart_sync_evaluation_performance() {
    let test_ctx = TestContext::new().await;
    let state = test_ctx.state.clone();
    
    // Create test user
    let user_data = CreateUser {
        username: "smart_sync_test".to_string(),
        email: "smart_sync_test@example.com".to_string(),
        password: "password123".to_string(),
        role: Some(UserRole::User),
    };
    let user = state.db.create_user(user_data).await
        .expect("Failed to create test user");
    
    println!("🧠 Testing smart sync evaluation performance");
    
    let num_directories = 3000;
    
    // Create directory structure
    let mut directories = Vec::new();
    for i in 0..num_directories {
        let depth = i % 4; // Vary depth
        let path = if depth == 0 {
            format!("/smart_test/dir_{}", i)
        } else {
            format!("/smart_test/level_{}/dir_{}", depth, i)
        };
        
        directories.push(CreateWebDAVDirectory {
            user_id: user.id,
            directory_path: path,
            directory_etag: format!("etag_{}", i),
            file_count: (i % 20) as i64, // Vary file counts
            total_size_bytes: ((i % 20) * 1024) as i64,
        });
    }
    
    // Insert directories
    let insert_start = Instant::now();
    state.db.bulk_create_or_update_webdav_directories(&directories).await
        .expect("Failed to insert directories");
    let insert_duration = insert_start.elapsed();
    
    // Test smart sync service performance
    let smart_sync = SmartSyncService::new(state.clone());
    
    // Test directory filtering performance (simulating smart sync logic)
    let filter_start = Instant::now();
    let known_dirs = state.db.list_webdav_directories(user.id).await
        .expect("Failed to fetch directories");
    
    // Filter directories by path prefix (common smart sync operation)
    let prefix = "/smart_test/";
    let filtered_dirs: Vec<_> = known_dirs
        .into_iter()
        .filter(|d| d.directory_path.starts_with(prefix))
        .collect();
    
    let filter_duration = filter_start.elapsed();
    
    println!("📊 Smart sync evaluation results:");
    println!("   - Total directories: {}", num_directories);
    println!("   - Filtered directories: {}", filtered_dirs.len());
    println!("   - Insert time: {:?}", insert_duration);
    println!("   - Filter time: {:?}", filter_duration);
    println!("   - Filter rate: {:.1} dirs/sec", 
          filtered_dirs.len() as f64 / filter_duration.as_secs_f64());
    
    // Verify filtering worked correctly
    assert_eq!(filtered_dirs.len(), num_directories);
    
    // Performance assertions
    assert!(insert_duration < Duration::from_secs(10));
    assert!(filter_duration < Duration::from_millis(500));
    
    let filter_rate = filtered_dirs.len() as f64 / filter_duration.as_secs_f64();
    assert!(filter_rate > 6000.0, 
            "Filter rate too slow: {:.1} dirs/sec", filter_rate);
}