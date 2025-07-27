use std::{sync::Arc, time::Duration, collections::HashMap};
use tokio::time::sleep;
use uuid::Uuid;
use futures::future::join_all;
use anyhow::Result;
use readur::{
    AppState,
    models::{CreateWebDAVDirectory, SourceType, SourceStatus, WebDAVSourceConfig, CreateSource, FileIngestionInfo},
    test_utils::{TestContext, TestAuthHelper},
    scheduling::source_scheduler::SourceScheduler,
    services::webdav::{
        SmartSyncService, 
        SmartSyncStrategy, 
        SyncProgress, 
        SyncPhase,
        WebDAVDiscoveryResult,
    },
};

/// Helper function to create full production test setup
async fn create_production_test_state() -> (TestContext, Arc<AppState>, Uuid) {
    let test_context = TestContext::new().await;
    
    let auth_helper = TestAuthHelper::new(test_context.app().clone());
    let test_user = auth_helper.create_test_user().await;

    let state = test_context.state().clone();
    let user_id = test_user.user_response.id;
    
    (test_context, state, user_id)
}

/// Helper to create a production-like WebDAV source
async fn create_production_webdav_source(
    state: &Arc<AppState>, 
    user_id: Uuid, 
    name: &str,
    folders: Vec<String>,
    auto_sync: bool,
) -> readur::models::Source {
    let config = WebDAVSourceConfig {
        server_url: "https://nextcloud.example.com".to_string(),
        username: "production_user".to_string(),
        password: "secure_password".to_string(),
        watch_folders: folders,
        file_extensions: vec!["pdf".to_string(), "docx".to_string(), "txt".to_string(), "md".to_string()],
        auto_sync,
        sync_interval_minutes: 5, // Realistic interval
        server_type: Some("nextcloud".to_string()),
    };

    let create_source = CreateSource {
        name: name.to_string(),
        source_type: SourceType::WebDAV,
        config: serde_json::to_value(config).unwrap(),
        enabled: Some(true),
    };

    state.db.create_source(user_id, &create_source).await
        .expect("Failed to create production source")
}

/// Production-like mock WebDAV service with realistic delays and behaviors
#[derive(Clone)]
struct ProductionMockWebDAVService {
    server_load_factor: f64, // 1.0 = normal, 2.0 = slow server, 0.5 = fast server
    failure_rate: f64,       // 0.0 = never fails, 0.1 = 10% failure rate
    directory_structure: Arc<std::sync::Mutex<HashMap<String, (String, Vec<FileIngestionInfo>)>>>, // path -> (etag, files)
    call_counter: Arc<std::sync::Mutex<u32>>,
}

impl ProductionMockWebDAVService {
    fn new(server_load_factor: f64, failure_rate: f64) -> Self {
        let mut structure = HashMap::new();
        
        // Create realistic directory structure
        structure.insert("/Documents".to_string(), (
            "docs-etag-v1".to_string(),
            vec![
                FileIngestionInfo {
                    name: "report.pdf".to_string(),
                    relative_path: "/Documents/report.pdf".to_string(),
                    full_path: "/remote.php/dav/files/user/Documents/report.pdf".to_string(),
                    #[allow(deprecated)]
                    path: "/Documents/report.pdf".to_string(),
                    size: 2048576, // 2MB
                    last_modified: Some(chrono::Utc::now() - chrono::Duration::hours(2)),
                    etag: "report-etag-1".to_string(),
                    is_directory: false,
                    mime_type: "application/pdf".to_string(),
                    created_at: None,
                    permissions: None,
                    owner: None,
                    group: None,
                    metadata: None,
                },
                FileIngestionInfo {
                    name: "notes.md".to_string(),
                    relative_path: "/Documents/notes.md".to_string(),
                    full_path: "/remote.php/dav/files/user/Documents/notes.md".to_string(),
                    #[allow(deprecated)]
                    path: "/Documents/notes.md".to_string(),
                    size: 4096, // 4KB
                    last_modified: Some(chrono::Utc::now() - chrono::Duration::minutes(30)),
                    etag: "notes-etag-1".to_string(),
                    is_directory: false,
                    mime_type: "text/markdown".to_string(),
                    created_at: None,
                    permissions: None,
                    owner: None,
                    group: None,
                    metadata: None,
                },
            ]
        ));
        
        structure.insert("/Projects".to_string(), (
            "projects-etag-v1".to_string(),
            vec![
                FileIngestionInfo {
                    name: "spec.docx".to_string(),
                    relative_path: "/Projects/spec.docx".to_string(),
                    full_path: "/remote.php/dav/files/user/Projects/spec.docx".to_string(),
                    #[allow(deprecated)]
                    path: "/Projects/spec.docx".to_string(),
                    size: 1024000, // 1MB
                    last_modified: Some(chrono::Utc::now() - chrono::Duration::days(1)),
                    etag: "spec-etag-1".to_string(),
                    is_directory: false,
                    mime_type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string(),
                    created_at: None,
                    permissions: None,
                    owner: None,
                    group: None,
                    metadata: None,
                },
            ]
        ));
        
        structure.insert("/Archive".to_string(), (
            "archive-etag-v1".to_string(),
            vec![] // Empty archive folder
        ));
        
        Self {
            server_load_factor,
            failure_rate,
            directory_structure: Arc::new(std::sync::Mutex::new(structure)),
            call_counter: Arc::new(std::sync::Mutex::new(0)),
        }
    }

    async fn mock_discover_with_realistic_behavior(
        &self,
        directory_path: &str,
        _recursive: bool,
    ) -> Result<WebDAVDiscoveryResult> {
        // Increment call counter
        {
            let mut counter = self.call_counter.lock().unwrap();
            *counter += 1;
        }

        // Simulate realistic network delays based on server load
        let base_delay = 200; // 200ms base delay
        let actual_delay = (base_delay as f64 * self.server_load_factor) as u64;
        sleep(Duration::from_millis(actual_delay)).await;

        // Simulate random failures
        if rand::random::<f64>() < self.failure_rate {
            return Err(anyhow::anyhow!("Simulated network failure for {}", directory_path));
        }

        // Get directory structure
        let structure = self.directory_structure.lock().unwrap();
        if let Some((etag, files)) = structure.get(directory_path) {
            // Create directory info for the path itself
            let directory_info = FileIngestionInfo {
                name: directory_path.split('/').last().unwrap_or("").to_string(),
                relative_path: directory_path.to_string(),
                full_path: format!("/remote.php/dav/files/user{}", directory_path),
                #[allow(deprecated)]
                path: directory_path.to_string(),
                size: 0,
                last_modified: Some(chrono::Utc::now()),
                etag: etag.clone(),
                is_directory: true,
                mime_type: "application/octet-stream".to_string(),
                created_at: None,
                permissions: None,
                owner: None,
                group: None,
                metadata: None,
            };

            Ok(WebDAVDiscoveryResult {
                files: files.clone(),
                directories: vec![directory_info],
            })
        } else {
            // Unknown directory
            Ok(WebDAVDiscoveryResult {
                files: vec![],
                directories: vec![],
            })
        }
    }

    fn get_call_count(&self) -> u32 {
        *self.call_counter.lock().unwrap()
    }

    fn update_directory_etag(&self, path: &str, new_etag: &str) {
        let mut structure = self.directory_structure.lock().unwrap();
        if let Some((etag, _)) = structure.get_mut(path) {
            *etag = new_etag.to_string();
        }
    }
}

/// Test full production sync flow with realistic concurrency scenarios
#[tokio::test]
async fn test_production_sync_flow_concurrent_sources() {
    let (_test_context, state, user_id) = create_production_test_state().await;
    
    // Create multiple sources like a real production setup
    let sources = vec![
        create_production_webdav_source(&state, user_id, "PersonalDocs", vec!["/Documents".to_string()], true).await,
        create_production_webdav_source(&state, user_id, "WorkProjects", vec!["/Projects".to_string()], true).await,
        create_production_webdav_source(&state, user_id, "Archive", vec!["/Archive".to_string()], false).await,
        create_production_webdav_source(&state, user_id, "MultiFolder", vec!["/Documents".to_string(), "/Projects".to_string()], true).await,
    ];
    
    // Create source scheduler
    let scheduler = SourceScheduler::new(state.clone());
    
    // Create production mock services with different characteristics
    let mock_services = vec![
        ProductionMockWebDAVService::new(1.0, 0.0),  // Normal server, reliable
        ProductionMockWebDAVService::new(2.0, 0.1),  // Slow server, occasional failures
        ProductionMockWebDAVService::new(0.8, 0.05), // Fast server, very reliable
        ProductionMockWebDAVService::new(1.5, 0.2),  // Slow server, unreliable
    ];
    
    // Simulate production workload: concurrent sync triggers from different sources
    let production_sync_operations: Vec<_> = sources.iter().zip(mock_services.iter()).enumerate().map(|(i, (source, mock_service))| {
        let state_clone = state.clone();
        let smart_sync_service = SmartSyncService::new(state_clone.clone());
        let source_id = source.id;
        let source_name = source.name.clone();
        let source_config = source.config.clone(); // Clone the config to avoid borrowing the source
        let mock_service = mock_service.clone();
        let user_id = user_id;
        
        tokio::spawn(async move {
            println!("🚀 Starting production sync for source: {}", source_name);
            
            // Create scheduler instance for this task
            let scheduler_local = SourceScheduler::new(state_clone.clone());
            
            // Step 1: Trigger sync via scheduler (Route Level simulation)
            let trigger_result = scheduler_local.trigger_sync(source_id).await;
            if trigger_result.is_err() {
                println!("❌ Failed to trigger sync for {}: {:?}", source_name, trigger_result);
                return (i, source_name, false, 0, 0);
            }
            
            // Step 2: Simulate smart sync evaluation and execution
            let config: WebDAVSourceConfig = serde_json::from_value(source_config).unwrap();
            let mut total_files_discovered = 0;
            let mut total_directories_processed = 0;
            
            for watch_folder in &config.watch_folders {
                println!("🔍 Processing watch folder: {} for source: {}", watch_folder, source_name);
                
                // Step 3: Simulate smart sync discovery (with mock WebDAV calls)
                match mock_service.mock_discover_with_realistic_behavior(watch_folder, true).await {
                    Ok(discovery_result) => {
                        total_files_discovered += discovery_result.files.len();
                        total_directories_processed += discovery_result.directories.len();
                        
                        // Step 4: Save discovered directory ETags (Database Level)
                        for dir_info in &discovery_result.directories {
                            let webdav_directory = CreateWebDAVDirectory {
                                user_id,
                                directory_path: dir_info.relative_path.clone(),
                                directory_etag: dir_info.etag.clone(),
                                file_count: discovery_result.files.len() as i64,
                                total_size_bytes: discovery_result.files.iter().map(|f| f.size).sum(),
                            };
                            
                            if let Err(e) = state_clone.db.create_or_update_webdav_directory(&webdav_directory).await {
                                println!("⚠️ Failed to save directory ETag for {}: {}", dir_info.relative_path, e);
                            }
                        }
                        
                        println!("✅ Discovered {} files and {} directories in {} for source: {}", 
                                discovery_result.files.len(), discovery_result.directories.len(), 
                                watch_folder, source_name);
                    }
                    Err(e) => {
                        println!("❌ Discovery failed for {} in source {}: {}", watch_folder, source_name, e);
                    }
                }
                
                // Small delay between folders to simulate realistic processing
                sleep(Duration::from_millis(100)).await;
            }
            
            println!("🎉 Production sync completed for source: {} ({} files, {} dirs)", 
                     source_name, total_files_discovered, total_directories_processed);
            
            (i, source_name, true, total_files_discovered, total_directories_processed)
        })
    }).collect();
    
    // Wait for all production sync operations
    let sync_results: Vec<_> = join_all(production_sync_operations).await;
    
    // Analyze production sync results
    let mut successful_syncs = 0;
    let mut total_files = 0;
    let mut total_dirs = 0;
    
    for result in sync_results {
        assert!(result.is_ok(), "Production sync task should complete");
        let (task_id, source_name, success, files, dirs) = result.unwrap();
        
        if success {
            successful_syncs += 1;
            total_files += files;
            total_dirs += dirs;
        }
        
        println!("Production sync {}: {} -> Success: {}, Files: {}, Dirs: {}", 
                 task_id, source_name, success, files, dirs);
    }
    
    println!("📊 Production sync summary: {}/{} sources successful, {} total files, {} total directories", 
             successful_syncs, sources.len(), total_files, total_dirs);
    
    // Verify production state consistency
    let final_directories = state.db.list_webdav_directories(user_id).await
        .expect("Failed to list final directories");
    
    println!("📁 Final directory count: {}", final_directories.len());
    
    // Should have directories from successful syncs
    assert!(final_directories.len() > 0, "Should have discovered some directories");
    
    // Verify all sources are in consistent states
    for source in sources {
        let final_source = state.db.get_source(user_id, source.id).await
            .expect("Failed to get source")
            .expect("Source should exist");
        
        // Source should not be stuck in syncing state
        assert_ne!(final_source.status, SourceStatus::Syncing,
                  "Source {} should not be stuck in syncing state", source.name);
    }
    
    // At least some syncs should succeed in a production environment
    assert!(successful_syncs > 0, "At least some production syncs should succeed");
}

/// Test production-like concurrent user actions
#[tokio::test]
async fn test_production_concurrent_user_actions() {
    let (_test_context, state, user_id) = create_production_test_state().await;
    
    // Create sources
    let source1 = create_production_webdav_source(&state, user_id, "UserDocs", vec!["/Documents".to_string()], false).await;
    let source2 = create_production_webdav_source(&state, user_id, "UserProjects", vec!["/Projects".to_string()], false).await;
    
    let scheduler = SourceScheduler::new(state.clone());
    
    // Simulate realistic user interaction patterns
    let user_actions = vec![
        // User rapidly clicks sync multiple times (common user behavior)
        (0, "trigger", source1.id, 0),
        (50, "trigger", source1.id, 0),   // 50ms later, another trigger
        (100, "trigger", source1.id, 0),  // Another trigger
        
        // User starts sync on source2, then immediately tries to stop it
        (200, "trigger", source2.id, 0),
        (250, "stop", source2.id, 0),
        
        // User checks status and triggers again
        (500, "trigger", source2.id, 0),
        
        // User tries to trigger both sources simultaneously
        (800, "trigger", source1.id, 0),
        (810, "trigger", source2.id, 0),
        
        // User stops everything
        (1200, "stop", source1.id, 0),
        (1210, "stop", source2.id, 0),
        
        // User waits and tries again
        (2000, "trigger", source1.id, 0),
    ];
    
    let user_action_tasks = user_actions.into_iter().map(|(delay_ms, action, source_id, _)| {
        let state_clone = state.clone();
        let action = action.to_string();
        tokio::spawn(async move {
            // Wait for scheduled time
            sleep(Duration::from_millis(delay_ms)).await;
            
            // Create scheduler instance for this task
            let scheduler_local = SourceScheduler::new(state_clone);
            
            let result = match action.as_str() {
                "trigger" => {
                    println!("🎯 User action: trigger sync for source {}", source_id);
                    scheduler_local.trigger_sync(source_id).await
                }
                "stop" => {
                    println!("🛑 User action: stop sync for source {}", source_id);
                    scheduler_local.stop_sync(source_id).await
                }
                _ => Ok(()),
            };
            
            (delay_ms, action, source_id, result.is_ok())
        })
    });
    
    // Execute all user actions concurrently
    let action_results: Vec<_> = join_all(user_action_tasks).await;
    
    // Analyze user action results
    let mut trigger_attempts = 0;
    let mut stop_attempts = 0;
    let mut successful_actions = 0;
    
    for result in action_results {
        assert!(result.is_ok(), "User action task should complete");
        let (delay, action, source_id, success) = result.unwrap();
        
        match action.as_str() {
            "trigger" => trigger_attempts += 1,
            "stop" => stop_attempts += 1,
            _ => {}
        }
        
        if success {
            successful_actions += 1;
        }
        
        println!("User action at {}ms: {} source {} -> {}", delay, action, source_id, success);
    }
    
    println!("📊 User actions: {} triggers, {} stops, {} successful", 
             trigger_attempts, stop_attempts, successful_actions);
    
    // Give time for any background operations to settle
    sleep(Duration::from_millis(3000)).await;
    
    // Verify final state after chaotic user interactions
    let final_source1 = state.db.get_source(user_id, source1.id).await
        .expect("Failed to get source1")
        .expect("Source1 should exist");
    let final_source2 = state.db.get_source(user_id, source2.id).await
        .expect("Failed to get source2")
        .expect("Source2 should exist");
    
    // Both sources should be in stable states (not stuck in syncing)
    assert!(matches!(final_source1.status, SourceStatus::Idle | SourceStatus::Error),
           "Source1 should be stable: {:?}", final_source1.status);
    assert!(matches!(final_source2.status, SourceStatus::Idle | SourceStatus::Error),
           "Source2 should be stable: {:?}", final_source2.status);
    
    // System should have handled the chaos gracefully
    assert!(successful_actions > 0, "Some user actions should have succeeded");
    
    // Final functionality test - system should still work
    let final_test = scheduler.trigger_sync(source1.id).await;
    println!("Final functionality test: {:?}", final_test.is_ok());
}

/// Test production memory and resource management under concurrent load
#[tokio::test]
async fn test_production_resource_management() {
    let (_test_context, state, user_id) = create_production_test_state().await;
    
    // Create many sources to simulate heavy production load
    let mut sources = Vec::new();
    for i in 0..20 {
        let source = create_production_webdav_source(
            &state, 
            user_id, 
            &format!("LoadTestSource{:02}", i),
            vec![format!("/Load{:02}", i)],
            true
        ).await;
        sources.push(source);
    }
    
    // Create extensive directory structure to test memory usage
    for i in 0..100 {
        let directory = CreateWebDAVDirectory {
            user_id,
            directory_path: format!("/memory-test-{:03}", i),
            directory_etag: format!("memory-etag-{:03}", i),
            file_count: (i as i64) * 10,
            total_size_bytes: (i as i64) * 1024 * 1024, // i MB each
        };
        state.db.create_or_update_webdav_directory(&directory).await
            .expect("Failed to create memory test directory");
    }
    
    let scheduler = SourceScheduler::new(state.clone());
    let smart_sync_service = SmartSyncService::new(state.clone());
    
    // Test concurrent operations under memory pressure
    let memory_stress_operations = (0..50).map(|i| {
        let smart_sync_clone = smart_sync_service.clone();
        let state_clone = state.clone();
        let source_id = sources[i % sources.len()].id;
        let user_id = user_id;
        
        tokio::spawn(async move {
            match i % 5 {
                0 => {
                    // Heavy database read operation
                    let dirs = state_clone.db.list_webdav_directories(user_id).await;
                    dirs.map(|d| d.len()).unwrap_or(0)
                }
                1 => {
                    // Sync trigger operation
                    let scheduler_local = SourceScheduler::new(state_clone.clone());
                    scheduler_local.trigger_sync(source_id).await.is_ok() as usize
                }
                2 => {
                    // Multiple directory updates
                    let mut updates = 0;
                    for j in 0..10 {
                        let dir = CreateWebDAVDirectory {
                            user_id,
                            directory_path: format!("/stress-{}-{}", i, j),
                            directory_etag: format!("stress-etag-{}-{}", i, j),
                            file_count: j as i64,
                            total_size_bytes: (j as i64) * 1024,
                        };
                        if state_clone.db.create_or_update_webdav_directory(&dir).await.is_ok() {
                            updates += 1;
                        }
                    }
                    updates
                }
                3 => {
                    // Stop operation
                    let scheduler_local = SourceScheduler::new(state_clone.clone());
                    scheduler_local.stop_sync(source_id).await.is_ok() as usize
                }
                4 => {
                    // Batch directory read and update
                    let dirs = state_clone.db.list_webdav_directories(user_id).await.unwrap_or_default();
                    let mut processed = 0;
                    for dir in dirs.iter().take(5) {
                        let updated = CreateWebDAVDirectory {
                            user_id,
                            directory_path: dir.directory_path.clone(),
                            directory_etag: format!("{}-batch-{}", dir.directory_etag, i),
                            file_count: dir.file_count + 1,
                            total_size_bytes: dir.total_size_bytes + 1024,
                        };
                        if state_clone.db.create_or_update_webdav_directory(&updated).await.is_ok() {
                            processed += 1;
                        }
                    }
                    processed
                }
                _ => unreachable!(),
            }
        })
    });
    
    // Execute all stress operations
    let stress_results: Vec<_> = join_all(memory_stress_operations).await;
    
    // Analyze resource management results
    let mut total_work_done = 0;
    for (i, result) in stress_results.into_iter().enumerate() {
        assert!(result.is_ok(), "Stress test task {} should complete", i);
        let work_units = result.unwrap();
        total_work_done += work_units;
    }
    
    println!("📊 Resource stress test completed: {} total work units", total_work_done);
    
    // Verify system is still functional after stress
    let final_directories = state.db.list_webdav_directories(user_id).await
        .expect("Database should still be functional");
    
    println!("📁 Final directory count after stress: {}", final_directories.len());
    
    // Should have handled the stress without corrupting data
    assert!(final_directories.len() >= 100, 
           "Should have at least the initial directories plus stress directories");
    
    // System should still be responsive
    let response_test_start = std::time::Instant::now();
    let response_test = state.db.list_webdav_directories(user_id).await;
    let response_time = response_test_start.elapsed();
    
    assert!(response_test.is_ok(), "System should still be responsive");
    assert!(response_time < Duration::from_secs(5), 
           "Response time should be reasonable: {:?}", response_time);
    
    println!("✅ System remains responsive after stress test: {:?}", response_time);
    
    // All sources should be in valid states
    for source in sources.iter().take(5) { // Check first 5 sources
        let final_source = state.db.get_source(user_id, source.id).await
            .expect("Failed to get source")
            .expect("Source should exist");
        
        assert!(matches!(final_source.status, 
                        SourceStatus::Idle | SourceStatus::Syncing | SourceStatus::Error),
               "Source {} should be in valid state: {:?}", source.name, final_source.status);
    }
}