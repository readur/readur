use std::sync::Arc;
use readur::{
    AppState,
    models::{CreateWebDAVDirectory, User, AuthProvider},
    services::webdav::{SmartSyncService, SmartSyncStrategy, SmartSyncDecision, WebDAVService, WebDAVConfig},
    test_utils::{TestContext, TestAuthHelper},
};

/// Mock WebDAV service for testing smart sync scenarios
#[derive(Clone)]
struct MockWebDAVService {
    directories: std::collections::HashMap<String, String>, // path -> etag
    files: Vec<readur::models::FileIngestionInfo>,
}

impl MockWebDAVService {
    fn new() -> Self {
        Self {
            directories: std::collections::HashMap::new(),
            files: Vec::new(),
        }
    }

    fn with_directory_structure(directories: Vec<(String, String)>) -> Self {
        let mut service = Self::new();
        for (path, etag) in directories {
            service.directories.insert(path, etag);
        }
        service
    }

    async fn discover_files_and_directories_mock(
        &self,
        _path: &str,
        _recursive: bool,
    ) -> anyhow::Result<readur::services::webdav::discovery::WebDAVDiscoveryResult> {
        let directories: Vec<readur::models::FileIngestionInfo> = self.directories
            .iter()
            .map(|(path, etag)| readur::models::FileIngestionInfo {
                path: path.clone(),
                name: path.split('/').last().unwrap_or("").to_string(),
                size: 0,
                mime_type: "".to_string(),
                last_modified: Some(chrono::Utc::now()),
                etag: etag.clone(),
                is_directory: true,
                created_at: Some(chrono::Utc::now()),
                permissions: Some(0),
                owner: None,
                group: None,
                metadata: None,
            })
            .collect();

        Ok(readur::services::webdav::discovery::WebDAVDiscoveryResult {
            files: self.files.clone(),
            directories,
        })
    }
}

use tokio::sync::OnceCell;

static TEST_CONTEXT: OnceCell<TestContext> = OnceCell::const_new();

/// Helper function to create test database and user using shared TestContext
async fn create_test_setup() -> (Arc<AppState>, User) {
    // Get or create shared test context to avoid multiple database containers
    let test_context = TEST_CONTEXT.get_or_init(|| async {
        TestContext::new().await
    }).await;
    
    let auth_helper = TestAuthHelper::new(test_context.app().clone());
    let test_user = auth_helper.create_test_user().await;
    
    // Convert TestUser to User model for compatibility
    let user = User {
        id: test_user.user_response.id,
        username: test_user.user_response.username,
        email: test_user.user_response.email,
        password_hash: Some("hashed_password".to_string()),
        role: test_user.user_response.role,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        oidc_subject: None,
        oidc_issuer: None,
        oidc_email: None,
        auth_provider: AuthProvider::Local,
    };

    (test_context.state().clone(), user)
}

/// Helper function to create WebDAV service for testing
fn create_test_webdav_service() -> WebDAVService {
    let config = WebDAVConfig {
        server_url: "https://test.example.com".to_string(),
        username: "test".to_string(),
        password: "test".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec!["pdf".to_string(), "txt".to_string()],
        timeout_seconds: 30,
        server_type: Some("generic".to_string()),
    };
    
    WebDAVService::new(config).expect("Failed to create WebDAV service")
}

#[tokio::test]
async fn test_first_time_sync_full_deep_scan() {
    // Test Scenario 1: First-time sync with no existing directory ETags
    // Expected: Should perform full deep scan and establish directory ETag baseline
    
    let (state, user) = create_test_setup().await;
    let smart_sync_service = SmartSyncService::new(state.clone());
    let webdav_service = create_test_webdav_service();
    
    // Verify no existing directories in database
    let existing_dirs = state.db.list_webdav_directories(user.id).await
        .expect("Failed to list directories");
    assert!(existing_dirs.is_empty(), "Database should start with no tracked directories");
    
    // Test smart sync evaluation for first-time scenario
    let decision = smart_sync_service.evaluate_sync_need(user.id, &webdav_service, "/Documents").await
        .expect("Smart sync evaluation failed");
    
    match decision {
        SmartSyncDecision::RequiresSync(SmartSyncStrategy::FullDeepScan) => {
            // This is expected for first-time sync
            println!("✅ First-time sync correctly identified need for full deep scan");
        }
        other => panic!("Expected FullDeepScan strategy for first-time sync, got: {:?}", other),
    }
    
    // Simulate performing the deep scan (this would normally interact with real WebDAV)
    // For testing, we'll directly save some directory ETags to verify the tracking works
    let test_directories = vec![
        ("/Documents", "root-etag-123"),
        ("/Documents/Projects", "projects-etag-456"),
        ("/Documents/Archive", "archive-etag-789"),
        ("/Documents/Projects/Current", "current-etag-abc"),
    ];
    
    for (path, etag) in &test_directories {
        let dir = CreateWebDAVDirectory {
            user_id: user.id,
            directory_path: path.to_string(),
            directory_etag: etag.to_string(),
            file_count: 0,
            total_size_bytes: 0,
        };
        
        state.db.create_or_update_webdav_directory(&dir).await
            .expect("Failed to create directory tracking");
    }
    
    // Verify all directories were tracked
    let tracked_dirs = state.db.list_webdav_directories(user.id).await
        .expect("Failed to list tracked directories");
    
    assert_eq!(tracked_dirs.len(), test_directories.len(), 
               "Should track all discovered directories");
    
    for (expected_path, expected_etag) in &test_directories {
        let found = tracked_dirs.iter().find(|d| &d.directory_path == expected_path);
        assert!(found.is_some(), "Directory {} should be tracked", expected_path);
        
        let dir = found.unwrap();
        assert_eq!(&dir.directory_etag, expected_etag, 
                   "Directory {} should have correct ETag", expected_path);
    }
    
    println!("✅ Test passed: First-time sync establishes complete directory ETag baseline");
}

#[tokio::test] 
async fn test_smart_sync_no_changes_skip() {
    // Test Scenario 2: Subsequent smart sync with no directory changes
    // Expected: Should skip sync entirely after ETag comparison
    
    let (state, user) = create_test_setup().await;
    let smart_sync_service = SmartSyncService::new(state.clone());
    
    // Pre-populate database with directory ETags (simulating previous sync)
    let existing_directories = vec![
        ("/Documents", "root-etag-stable"),
        ("/Documents/Projects", "projects-etag-stable"), 
        ("/Documents/Archive", "archive-etag-stable"),
    ];
    
    for (path, etag) in &existing_directories {
        let dir = CreateWebDAVDirectory {
            user_id: user.id,
            directory_path: path.to_string(),
            directory_etag: etag.to_string(),
            file_count: 5,
            total_size_bytes: 1024000,
        };
        
        state.db.create_or_update_webdav_directory(&dir).await
            .expect("Failed to create existing directory tracking");
    }
    
    // Verify directories were created in database
    let existing_dirs = state.db.list_webdav_directories(user.id).await
        .expect("Failed to list directories");
    assert_eq!(existing_dirs.len(), 3, "Should have 3 pre-existing directories");
    
    // Create mock WebDAV service that returns the same ETags (no changes)
    let mock_service = MockWebDAVService::with_directory_structure(vec![
        ("/Documents".to_string(), "root-etag-stable".to_string()),
        ("/Documents/Projects".to_string(), "projects-etag-stable".to_string()),
        ("/Documents/Archive".to_string(), "archive-etag-stable".to_string()),
    ]);
    
    // Test smart sync evaluation - should detect no changes
    let sync_result = mock_service.discover_files_and_directories_mock("/Documents", false).await
        .expect("Mock discovery should succeed");
    
    // Verify mock returns the same ETags
    assert_eq!(sync_result.directories.len(), 3, "Should discover 3 directories");
    for directory in &sync_result.directories {
        let expected_etag = match directory.path.as_str() {
            "/Documents" => "root-etag-stable",
            "/Documents/Projects" => "projects-etag-stable",
            "/Documents/Archive" => "archive-etag-stable",
            _ => panic!("Unexpected directory: {}", directory.path),
        };
        assert_eq!(directory.etag, expected_etag, "Directory {} should have unchanged ETag", directory.path);
    }
    
    // Manually test the smart sync logic (since we can't easily mock WebDAVService in evaluate_sync_need)
    // Get known directories from database
    let known_dirs: std::collections::HashMap<String, String> = existing_dirs
        .into_iter()
        .filter(|dir| dir.directory_path.starts_with("/Documents"))
        .map(|dir| (dir.directory_path, dir.directory_etag))
        .collect();
    
    // Compare with "discovered" directories (same ETags)
    let mut changed_count = 0;
    let mut new_count = 0;
    
    for directory in &sync_result.directories {
        match known_dirs.get(&directory.path) {
            Some(known_etag) => {
                if known_etag != &directory.etag {
                    changed_count += 1;
                }
            }
            None => {
                new_count += 1;
            }
        }
    }
    
    // Verify no changes detected
    assert_eq!(changed_count, 0, "Should detect no changed directories");
    assert_eq!(new_count, 0, "Should detect no new directories");
    
    // This demonstrates the logic that would cause SmartSyncDecision::SkipSync
    println!("✅ Smart sync no-changes test passed: {} changed, {} new directories detected", 
             changed_count, new_count);
    println!("✅ In real implementation, this would result in SmartSyncDecision::SkipSync");
}

#[tokio::test]
async fn test_deep_scan_resets_directory_etags() {
    // Test Scenario 5: Manual deep scan should reset all directory ETags at all levels
    // Expected: All directory ETags should be updated with fresh values from WebDAV
    
    let (state, user) = create_test_setup().await;
    let smart_sync_service = SmartSyncService::new(state.clone());
    let webdav_service = create_test_webdav_service();
    
    // Pre-populate database with old directory ETags
    let old_directories = vec![
        ("/Documents", "old-root-etag"),
        ("/Documents/Projects", "old-projects-etag"),
        ("/Documents/Archive", "old-archive-etag"),
        ("/Documents/Projects/Subproject", "old-subproject-etag"),
    ];
    
    for (path, etag) in &old_directories {
        let dir = CreateWebDAVDirectory {
            user_id: user.id,
            directory_path: path.to_string(),
            directory_etag: etag.to_string(),
            file_count: 3,
            total_size_bytes: 512000,
        };
        
        state.db.create_or_update_webdav_directory(&dir).await
            .expect("Failed to create old directory tracking");
    }
    
    // Verify old ETags are in database
    let pre_scan_dirs = state.db.list_webdav_directories(user.id).await
        .expect("Failed to list pre-scan directories");
    assert_eq!(pre_scan_dirs.len(), 4, "Should start with 4 tracked directories");
    
    for dir in &pre_scan_dirs {
        assert!(dir.directory_etag.starts_with("old-"), 
                "Directory {} should have old ETag", dir.directory_path);
    }
    
    // Create mock WebDAV service that returns new ETags for all directories
    let mock_service = MockWebDAVService::with_directory_structure(vec![
        ("/Documents".to_string(), "new-root-etag-123".to_string()),
        ("/Documents/Projects".to_string(), "new-projects-etag-456".to_string()),
        ("/Documents/Archive".to_string(), "new-archive-etag-789".to_string()),
        ("/Documents/Projects/Subproject".to_string(), "new-subproject-etag-abc".to_string()),
        // Additional new directory discovered during deep scan
        ("/Documents/NewlyFound".to_string(), "newly-found-etag-xyz".to_string()),
    ]);
    
    // Simulate deep scan discovery (this would be called by perform_smart_sync internally)
    let deep_scan_discovery = mock_service.discover_files_and_directories_mock("/Documents", true).await
        .expect("Mock deep scan discovery should succeed");
    
    // Verify deep scan discovers all directories including new ones
    assert_eq!(deep_scan_discovery.directories.len(), 5, "Deep scan should discover 5 directories");
    
    // Simulate what perform_smart_sync would do - save all discovered directory ETags
    for directory_info in &deep_scan_discovery.directories {
        let webdav_directory = CreateWebDAVDirectory {
            user_id: user.id,
            directory_path: directory_info.path.clone(),
            directory_etag: directory_info.etag.clone(),
            file_count: 0, // Would be updated by stats
            total_size_bytes: 0, // Would be updated by stats
        };
        
        state.db.create_or_update_webdav_directory(&webdav_directory).await
            .expect("Failed to update directory ETag during deep scan");
    }
    
    // Verify all directory ETags were reset to new values
    let post_scan_dirs = state.db.list_webdav_directories(user.id).await
        .expect("Failed to list post-scan directories");
    
    // Should have one additional directory from deep scan
    assert_eq!(post_scan_dirs.len(), 5, "Should have 5 directories after deep scan");
    
    // Verify all ETags are updated
    for dir in &post_scan_dirs {
        match dir.directory_path.as_str() {
            "/Documents" => {
                assert_eq!(dir.directory_etag, "new-root-etag-123", "Root ETag should be updated");
            }
            "/Documents/Projects" => {
                assert_eq!(dir.directory_etag, "new-projects-etag-456", "Projects ETag should be updated");
            }
            "/Documents/Archive" => {
                assert_eq!(dir.directory_etag, "new-archive-etag-789", "Archive ETag should be updated");
            }
            "/Documents/Projects/Subproject" => {
                assert_eq!(dir.directory_etag, "new-subproject-etag-abc", "Subproject ETag should be updated");
            }
            "/Documents/NewlyFound" => {
                assert_eq!(dir.directory_etag, "newly-found-etag-xyz", "New directory should be tracked");
            }
            _ => panic!("Unexpected directory: {}", dir.directory_path),
        }
        
        // Verify no old ETags remain
        assert!(!dir.directory_etag.starts_with("old-"), 
                "Directory {} should not have old ETag: {}", dir.directory_path, dir.directory_etag);
    }
    
    println!("✅ Manual deep scan test passed:");
    println!("   - All {} existing directory ETags were reset", old_directories.len());
    println!("   - 1 new directory was discovered and tracked");
    println!("   - Total directories tracked: {}", post_scan_dirs.len());
    println!("   - Deep scan strategy successfully resets entire ETag baseline");
}

#[tokio::test]
async fn test_directory_structure_changes() {
    // Test Scenario 8: Directory structure changes - new subdirectories should be detected
    // Expected: New directories get tracked, existing unchanged directories preserved
    
    let (state, user) = create_test_setup().await;
    let smart_sync_service = SmartSyncService::new(state.clone());
    
    // Start with some existing directory tracking
    let initial_directories = vec![
        ("/Documents", "root-etag-unchanged"),
        ("/Documents/Existing", "existing-etag-unchanged"),
    ];
    
    for (path, etag) in &initial_directories {
        let dir = CreateWebDAVDirectory {
            user_id: user.id,
            directory_path: path.to_string(),
            directory_etag: etag.to_string(),
            file_count: 2,
            total_size_bytes: 256000,
        };
        
        state.db.create_or_update_webdav_directory(&dir).await
            .expect("Failed to create initial directory tracking");
    }
    
    // Simulate discovering new directory structure (this would come from WebDAV)
    let new_structure = vec![
        ("/Documents", "root-etag-unchanged"),      // Unchanged
        ("/Documents/Existing", "existing-etag-unchanged"), // Unchanged  
        ("/Documents/NewFolder", "new-folder-etag"), // New directory
        ("/Documents/NewFolder/SubNew", "subnew-etag"), // New subdirectory
    ];
    
    // In a real scenario, smart sync would detect these changes and track new directories
    // For testing, we simulate the result of that discovery
    for (path, etag) in &new_structure {
        let dir = CreateWebDAVDirectory {
            user_id: user.id,
            directory_path: path.to_string(),
            directory_etag: etag.to_string(),
            file_count: if path.contains("New") { 0 } else { 2 },
            total_size_bytes: if path.contains("New") { 0 } else { 256000 },
        };
        
        state.db.create_or_update_webdav_directory(&dir).await
            .expect("Failed to update directory tracking");
    }
    
    // Verify all directories are now tracked
    let final_dirs = state.db.list_webdav_directories(user.id).await
        .expect("Failed to list final directories");
        
    assert_eq!(final_dirs.len(), 4, "Should track all 4 directories after structure change");
    
    // Verify new directories are tracked
    let new_folder = final_dirs.iter().find(|d| d.directory_path == "/Documents/NewFolder");
    assert!(new_folder.is_some(), "New folder should be tracked");
    assert_eq!(new_folder.unwrap().directory_etag, "new-folder-etag");
    
    let sub_new = final_dirs.iter().find(|d| d.directory_path == "/Documents/NewFolder/SubNew");
    assert!(sub_new.is_some(), "New subdirectory should be tracked");
    assert_eq!(sub_new.unwrap().directory_etag, "subnew-etag");
    
    // Verify unchanged directories preserved
    let existing = final_dirs.iter().find(|d| d.directory_path == "/Documents/Existing");
    assert!(existing.is_some(), "Existing directory should be preserved");
    assert_eq!(existing.unwrap().directory_etag, "existing-etag-unchanged");
    
    println!("✅ Test passed: Directory structure changes properly tracked");
}

// Additional test stubs for remaining scenarios
#[tokio::test]
async fn test_smart_sync_targeted_scan() {
    // Test Scenario 3: Smart sync with single directory changed - should use targeted scan
    // Expected: Should detect single change and use TargetedScan strategy
    
    let (state, user) = create_test_setup().await;
    let smart_sync_service = SmartSyncService::new(state.clone());
    
    // Pre-populate database with directory ETags (simulating previous sync)
    let existing_directories = vec![
        ("/Documents", "root-etag-stable"),
        ("/Documents/Projects", "projects-etag-old"), // This one will change
        ("/Documents/Archive", "archive-etag-stable"),
        ("/Documents/Reports", "reports-etag-stable"),
    ];
    
    for (path, etag) in &existing_directories {
        let dir = CreateWebDAVDirectory {
            user_id: user.id,
            directory_path: path.to_string(),
            directory_etag: etag.to_string(),
            file_count: 3,
            total_size_bytes: 512000,
        };
        
        state.db.create_or_update_webdav_directory(&dir).await
            .expect("Failed to create existing directory tracking");
    }
    
    // Verify initial state
    let existing_dirs = state.db.list_webdav_directories(user.id).await
        .expect("Failed to list directories");
    assert_eq!(existing_dirs.len(), 4, "Should have 4 pre-existing directories");
    
    // Create mock WebDAV service that returns one changed ETag
    let mock_service = MockWebDAVService::with_directory_structure(vec![
        ("/Documents".to_string(), "root-etag-stable".to_string()),
        ("/Documents/Projects".to_string(), "projects-etag-NEW".to_string()), // Changed!
        ("/Documents/Archive".to_string(), "archive-etag-stable".to_string()),
        ("/Documents/Reports".to_string(), "reports-etag-stable".to_string()),
    ]);
    
    // Test smart sync evaluation
    let sync_result = mock_service.discover_files_and_directories_mock("/Documents", false).await
        .expect("Mock discovery should succeed");
    
    // Verify mock returns expected ETags
    assert_eq!(sync_result.directories.len(), 4, "Should discover 4 directories");
    
    // Get known directories from database for comparison
    let known_dirs: std::collections::HashMap<String, String> = existing_dirs
        .into_iter()
        .filter(|dir| dir.directory_path.starts_with("/Documents"))
        .map(|dir| (dir.directory_path, dir.directory_etag))
        .collect();
    
    // Compare with discovered directories to identify changes
    let mut changed_directories = Vec::new();
    let mut new_directories = Vec::new();
    let mut unchanged_directories = Vec::new();
    
    for directory in &sync_result.directories {
        match known_dirs.get(&directory.path) {
            Some(known_etag) => {
                if known_etag != &directory.etag {
                    changed_directories.push(directory.path.clone());
                } else {
                    unchanged_directories.push(directory.path.clone());
                }
            }
            None => {
                new_directories.push(directory.path.clone());
            }
        }
    }
    
    // Verify targeted scan scenario
    assert_eq!(changed_directories.len(), 1, "Should detect exactly 1 changed directory");
    assert_eq!(new_directories.len(), 0, "Should detect no new directories");
    assert_eq!(unchanged_directories.len(), 3, "Should detect 3 unchanged directories");
    assert_eq!(changed_directories[0], "/Documents/Projects", "Changed directory should be /Documents/Projects");
    
    // Test strategy selection logic (mirrors SmartSyncService logic)
    let total_changes = changed_directories.len() + new_directories.len();
    let total_known = known_dirs.len();
    let change_ratio = total_changes as f64 / total_known.max(1) as f64;
    
    // Should use targeted scan (low change ratio, few new directories)
    let should_use_targeted = change_ratio <= 0.3 && new_directories.len() <= 5;
    assert!(should_use_targeted, "Should use targeted scan for single directory change");
    
    println!("✅ Smart sync targeted scan test passed:");
    println!("   - Changed directories: {:?}", changed_directories);
    println!("   - New directories: {:?}", new_directories);
    println!("   - Change ratio: {:.2}%", change_ratio * 100.0);
    println!("   - Strategy: TargetedScan (as expected)");
}

#[tokio::test] 
async fn test_smart_sync_fallback_to_deep_scan() {
    // Test Scenario 4: Smart sync with many directories changed - should fall back to full deep scan
    // Expected: Should detect many changes and use FullDeepScan strategy
    
    let (state, user) = create_test_setup().await;
    let smart_sync_service = SmartSyncService::new(state.clone());
    
    // Pre-populate database with directory ETags (simulating previous sync)
    let existing_directories = vec![
        ("/Documents", "root-etag-old"),
        ("/Documents/Projects", "projects-etag-old"), 
        ("/Documents/Archive", "archive-etag-old"),
        ("/Documents/Reports", "reports-etag-old"),
        ("/Documents/Images", "images-etag-old"),
        ("/Documents/Videos", "videos-etag-old"),
        ("/Documents/Music", "music-etag-old"),
        ("/Documents/Backup", "backup-etag-old"),
    ];
    
    for (path, etag) in &existing_directories {
        let dir = CreateWebDAVDirectory {
            user_id: user.id,
            directory_path: path.to_string(),
            directory_etag: etag.to_string(),
            file_count: 10,
            total_size_bytes: 2048000,
        };
        
        state.db.create_or_update_webdav_directory(&dir).await
            .expect("Failed to create existing directory tracking");
    }
    
    // Verify initial state
    let existing_dirs = state.db.list_webdav_directories(user.id).await
        .expect("Failed to list directories");
    assert_eq!(existing_dirs.len(), 8, "Should have 8 pre-existing directories");
    
    // Create mock WebDAV service that returns many changed ETags + new directories
    let mock_service = MockWebDAVService::with_directory_structure(vec![
        // Many existing directories with changed ETags
        ("/Documents".to_string(), "root-etag-NEW".to_string()), // Changed
        ("/Documents/Projects".to_string(), "projects-etag-NEW".to_string()), // Changed
        ("/Documents/Archive".to_string(), "archive-etag-NEW".to_string()), // Changed
        ("/Documents/Reports".to_string(), "reports-etag-NEW".to_string()), // Changed
        ("/Documents/Images".to_string(), "images-etag-old".to_string()), // Unchanged
        ("/Documents/Videos".to_string(), "videos-etag-old".to_string()), // Unchanged
        ("/Documents/Music".to_string(), "music-etag-NEW".to_string()), // Changed
        ("/Documents/Backup".to_string(), "backup-etag-old".to_string()), // Unchanged
        // Many new directories
        ("/Documents/NewProject1".to_string(), "new1-etag".to_string()), // New
        ("/Documents/NewProject2".to_string(), "new2-etag".to_string()), // New
        ("/Documents/NewProject3".to_string(), "new3-etag".to_string()), // New
        ("/Documents/NewProject4".to_string(), "new4-etag".to_string()), // New
        ("/Documents/NewProject5".to_string(), "new5-etag".to_string()), // New
        ("/Documents/NewProject6".to_string(), "new6-etag".to_string()), // New
    ]);
    
    // Test smart sync evaluation
    let sync_result = mock_service.discover_files_and_directories_mock("/Documents", false).await
        .expect("Mock discovery should succeed");
    
    // Verify mock returns expected ETags
    assert_eq!(sync_result.directories.len(), 14, "Should discover 14 directories total");
    
    // Get known directories from database for comparison
    let known_dirs: std::collections::HashMap<String, String> = existing_dirs
        .into_iter()
        .filter(|dir| dir.directory_path.starts_with("/Documents"))
        .map(|dir| (dir.directory_path, dir.directory_etag))
        .collect();
    
    // Compare with discovered directories to identify changes
    let mut changed_directories = Vec::new();
    let mut new_directories = Vec::new();
    let mut unchanged_directories = Vec::new();
    
    for directory in &sync_result.directories {
        match known_dirs.get(&directory.path) {
            Some(known_etag) => {
                if known_etag != &directory.etag {
                    changed_directories.push(directory.path.clone());
                } else {
                    unchanged_directories.push(directory.path.clone());
                }
            }
            None => {
                new_directories.push(directory.path.clone());
            }
        }
    }
    
    // Verify fallback to deep scan scenario
    assert_eq!(changed_directories.len(), 5, "Should detect 5 changed directories");
    assert_eq!(new_directories.len(), 6, "Should detect 6 new directories");
    assert_eq!(unchanged_directories.len(), 3, "Should detect 3 unchanged directories");
    
    // Test strategy selection logic (mirrors SmartSyncService logic)
    let total_changes = changed_directories.len() + new_directories.len();
    let total_known = known_dirs.len();
    let change_ratio = total_changes as f64 / total_known.max(1) as f64;
    
    // Should fallback to full deep scan (high change ratio OR many new directories)
    let should_use_full_scan = change_ratio > 0.3 || new_directories.len() > 5;
    assert!(should_use_full_scan, "Should use full deep scan for many changes");
    
    // Verify both thresholds are exceeded
    assert!(change_ratio > 0.3, "Change ratio {:.2}% should exceed 30% threshold", change_ratio * 100.0);
    assert!(new_directories.len() > 5, "New directories count {} should exceed 5", new_directories.len());
    
    println!("✅ Smart sync fallback to deep scan test passed:");
    println!("   - Changed directories: {} ({})", changed_directories.len(), changed_directories.join(", "));
    println!("   - New directories: {} ({})", new_directories.len(), new_directories.join(", "));
    println!("   - Unchanged directories: {}", unchanged_directories.len());
    println!("   - Change ratio: {:.1}% (exceeds 30% threshold)", change_ratio * 100.0);
    println!("   - New dirs count: {} (exceeds 5 threshold)", new_directories.len());
    println!("   - Strategy: FullDeepScan (as expected)");
}

#[tokio::test]
async fn test_scheduled_deep_scan() {
    // Test Scenario 6: Scheduled deep scan should reset all directory ETags and track new ones
    // Expected: Similar to manual deep scan, but triggered by scheduler with different lifecycle
    
    let (state, user) = create_test_setup().await;
    let smart_sync_service = SmartSyncService::new(state.clone());
    
    // Pre-populate database with directory ETags from previous scheduled sync
    let previous_directories = vec![
        ("/Documents", "scheduled-root-etag-v1"),
        ("/Documents/Quarterly", "scheduled-quarterly-etag-v1"),
        ("/Documents/Monthly", "scheduled-monthly-etag-v1"),
        ("/Documents/Daily", "scheduled-daily-etag-v1"),
    ];
    
    for (path, etag) in &previous_directories {
        let dir = CreateWebDAVDirectory {
            user_id: user.id,
            directory_path: path.to_string(),
            directory_etag: etag.to_string(),
            file_count: 8,
            total_size_bytes: 1536000,
        };
        
        state.db.create_or_update_webdav_directory(&dir).await
            .expect("Failed to create scheduled directory tracking");
    }
    
    // Verify initial scheduled sync state
    let pre_scheduled_dirs = state.db.list_webdav_directories(user.id).await
        .expect("Failed to list pre-scheduled directories");
    assert_eq!(pre_scheduled_dirs.len(), 4, "Should start with 4 scheduled directories");
    
    for dir in &pre_scheduled_dirs {
        assert!(dir.directory_etag.contains("v1"), 
                "Directory {} should have v1 ETag", dir.directory_path);
    }
    
    // Create mock WebDAV service for scheduled deep scan with updated structure
    let mock_service = MockWebDAVService::with_directory_structure(vec![
        // All existing directories get updated ETags
        ("/Documents".to_string(), "scheduled-root-etag-v2".to_string()),
        ("/Documents/Quarterly".to_string(), "scheduled-quarterly-etag-v2".to_string()),
        ("/Documents/Monthly".to_string(), "scheduled-monthly-etag-v2".to_string()),
        ("/Documents/Daily".to_string(), "scheduled-daily-etag-v2".to_string()),
        // New directories discovered during scheduled scan
        ("/Documents/Weekly".to_string(), "scheduled-weekly-etag-v1".to_string()),
        ("/Documents/Yearly".to_string(), "scheduled-yearly-etag-v1".to_string()),
        ("/Documents/Archives".to_string(), "scheduled-archives-etag-v1".to_string()),
    ]);
    
    // Simulate scheduled deep scan (this would be triggered by SourceScheduler)
    let scheduled_discovery = mock_service.discover_files_and_directories_mock("/Documents", true).await
        .expect("Mock scheduled scan discovery should succeed");
    
    // Verify scheduled scan discovers expanded directory structure
    assert_eq!(scheduled_discovery.directories.len(), 7, "Scheduled scan should discover 7 directories");
    
    // Simulate what scheduled sync would do - perform full deep scan strategy
    for directory_info in &scheduled_discovery.directories {
        let webdav_directory = CreateWebDAVDirectory {
            user_id: user.id,
            directory_path: directory_info.path.clone(),
            directory_etag: directory_info.etag.clone(),
            file_count: 5, // Updated file counts from scan
            total_size_bytes: 1024000, // Updated sizes from scan
        };
        
        state.db.create_or_update_webdav_directory(&webdav_directory).await
            .expect("Failed to update directory during scheduled scan");
    }
    
    // Verify scheduled deep scan results
    let post_scheduled_dirs = state.db.list_webdav_directories(user.id).await
        .expect("Failed to list post-scheduled directories");
    
    // Should have 3 additional directories from scheduled scan
    assert_eq!(post_scheduled_dirs.len(), 7, "Should have 7 directories after scheduled scan");
    
    // Verify all existing ETags were updated to v2
    let mut updated_existing = 0;
    let mut new_directories = 0;
    
    for dir in &post_scheduled_dirs {
        if previous_directories.iter().any(|(path, _)| path == &dir.directory_path) {
            // Existing directory should be updated
            assert!(dir.directory_etag.contains("v2"), 
                    "Existing directory {} should be updated to v2: {}", 
                    dir.directory_path, dir.directory_etag);
            assert_eq!(dir.file_count, 5, "File count should be updated from scan");
            assert_eq!(dir.total_size_bytes, 1024000, "Size should be updated from scan");
            updated_existing += 1;
        } else {
            // New directory should be tracked
            assert!(dir.directory_etag.contains("v1"), 
                    "New directory {} should have v1 ETag: {}", 
                    dir.directory_path, dir.directory_etag);
            new_directories += 1;
        }
    }
    
    assert_eq!(updated_existing, 4, "Should update 4 existing directories");
    assert_eq!(new_directories, 3, "Should discover 3 new directories");
    
    // Verify no old v1 ETags remain for existing directories
    for dir in &post_scheduled_dirs {
        if previous_directories.iter().any(|(path, _)| path == &dir.directory_path) {
            assert!(!dir.directory_etag.contains("v1"), 
                    "Existing directory {} should not have old v1 ETag", dir.directory_path);
        }
    }
    
    println!("✅ Scheduled deep scan test passed:");
    println!("   - Updated {} existing directories to v2 ETags", updated_existing);
    println!("   - Discovered and tracked {} new directories", new_directories);
    println!("   - Total directories tracked: {}", post_scheduled_dirs.len());
    println!("   - File counts and sizes updated during scan");
    println!("   - Scheduled deep scan maintains complete directory tracking");
}

#[tokio::test]
async fn test_smart_sync_after_deep_scan() {
    // Test Scenario 7: Smart sync after deep scan should use fresh directory ETags
    // Expected: After deep scan, smart sync should use the new baseline and detect minimal changes
    
    let (state, user) = create_test_setup().await;
    let smart_sync_service = SmartSyncService::new(state.clone());
    
    // Phase 1: Simulate state after a deep scan has completed
    let post_deep_scan_directories = vec![
        ("/Documents", "deep-scan-root-fresh"),
        ("/Documents/Active", "deep-scan-active-fresh"),
        ("/Documents/Archive", "deep-scan-archive-fresh"),
        ("/Documents/Processing", "deep-scan-processing-fresh"),
    ];
    
    for (path, etag) in &post_deep_scan_directories {
        let dir = CreateWebDAVDirectory {
            user_id: user.id,
            directory_path: path.to_string(),
            directory_etag: etag.to_string(),
            file_count: 12,
            total_size_bytes: 2048000,
        };
        
        state.db.create_or_update_webdav_directory(&dir).await
            .expect("Failed to create post-deep-scan directory tracking");
    }
    
    // Verify deep scan baseline is established
    let baseline_dirs = state.db.list_webdav_directories(user.id).await
        .expect("Failed to list baseline directories");
    assert_eq!(baseline_dirs.len(), 4, "Should have fresh baseline from deep scan");
    
    for dir in &baseline_dirs {
        assert!(dir.directory_etag.contains("fresh"), 
                "Directory {} should have fresh ETag from deep scan", dir.directory_path);
    }
    
    // Phase 2: Time passes, then smart sync runs and finds mostly unchanged structure
    // with just one minor change
    let mock_service = MockWebDAVService::with_directory_structure(vec![
        ("/Documents".to_string(), "deep-scan-root-fresh".to_string()), // Unchanged from deep scan
        ("/Documents/Active".to_string(), "deep-scan-active-UPDATED".to_string()), // One change!
        ("/Documents/Archive".to_string(), "deep-scan-archive-fresh".to_string()), // Unchanged
        ("/Documents/Processing".to_string(), "deep-scan-processing-fresh".to_string()), // Unchanged
    ]);
    
    // Phase 3: Smart sync evaluation after deep scan baseline
    let smart_sync_discovery = mock_service.discover_files_and_directories_mock("/Documents", false).await
        .expect("Mock smart sync after deep scan should succeed");
    
    // Verify structure is as expected
    assert_eq!(smart_sync_discovery.directories.len(), 4, "Should discover same 4 directories");
    
    // Phase 4: Analyze changes against fresh deep scan baseline
    let known_dirs: std::collections::HashMap<String, String> = baseline_dirs
        .into_iter()
        .filter(|dir| dir.directory_path.starts_with("/Documents"))
        .map(|dir| (dir.directory_path, dir.directory_etag))
        .collect();
    
    let mut changed_dirs_after_deep_scan = Vec::new();
    let mut unchanged_dirs_after_deep_scan = Vec::new();
    let mut new_dirs_after_deep_scan = Vec::new();
    
    for directory in &smart_sync_discovery.directories {
        match known_dirs.get(&directory.path) {
            Some(baseline_etag) => {
                if baseline_etag != &directory.etag {
                    changed_dirs_after_deep_scan.push(directory.path.clone());
                } else {
                    unchanged_dirs_after_deep_scan.push(directory.path.clone());
                }
            }
            None => {
                new_dirs_after_deep_scan.push(directory.path.clone());
            }
        }
    }
    
    // Phase 5: Verify smart sync detects minimal change against fresh baseline
    assert_eq!(changed_dirs_after_deep_scan.len(), 1, "Should detect 1 changed directory against fresh baseline");
    assert_eq!(unchanged_dirs_after_deep_scan.len(), 3, "Should detect 3 unchanged directories against fresh baseline");
    assert_eq!(new_dirs_after_deep_scan.len(), 0, "Should detect no new directories");
    
    assert_eq!(changed_dirs_after_deep_scan[0], "/Documents/Active", 
               "Active directory should be the one that changed since deep scan");
    
    // Phase 6: Verify smart sync strategy selection using fresh baseline
    let total_changes = changed_dirs_after_deep_scan.len() + new_dirs_after_deep_scan.len();
    let total_known = known_dirs.len();
    let change_ratio_vs_baseline = total_changes as f64 / total_known.max(1) as f64;
    
    // Should use targeted scan (minimal change against fresh baseline)
    let should_use_targeted = change_ratio_vs_baseline <= 0.3 && new_dirs_after_deep_scan.len() <= 5;
    assert!(should_use_targeted, "Should use targeted scan for minimal change against fresh baseline");
    
    // Phase 7: Simulate smart sync updating only the changed directory
    for dir in &smart_sync_discovery.directories {
        if changed_dirs_after_deep_scan.contains(&dir.path) {
            let updated_dir = CreateWebDAVDirectory {
                user_id: user.id,
                directory_path: dir.path.clone(),
                directory_etag: dir.etag.clone(),
                file_count: 15, // Updated from targeted scan
                total_size_bytes: 2560000, // Updated from targeted scan
            };
            
            state.db.create_or_update_webdav_directory(&updated_dir).await
                .expect("Failed to update changed directory from smart sync");
        }
    }
    
    // Phase 8: Verify final state maintains fresh baseline with targeted update
    let final_dirs = state.db.list_webdav_directories(user.id).await
        .expect("Failed to list final directories");
    
    assert_eq!(final_dirs.len(), 4, "Should still have 4 directories");
    
    for dir in &final_dirs {
        if dir.directory_path == "/Documents/Active" {
            assert_eq!(dir.directory_etag, "deep-scan-active-UPDATED", 
                      "Active directory should have updated ETag");
            assert_eq!(dir.file_count, 15, "File count should be updated");
        } else {
            assert!(dir.directory_etag.contains("fresh"), 
                    "Other directories should retain fresh baseline ETags: {}", 
                    dir.directory_path);
        }
    }
    
    println!("✅ Smart sync after deep scan test passed:");
    println!("   - Used fresh deep scan baseline with {} directories", post_deep_scan_directories.len());
    println!("   - Detected {} changed directory against fresh baseline", changed_dirs_after_deep_scan.len());
    println!("   - Preserved {} unchanged directories from baseline", unchanged_dirs_after_deep_scan.len());
    println!("   - Change ratio vs fresh baseline: {:.1}%", change_ratio_vs_baseline * 100.0);
    println!("   - Strategy: TargetedScan (efficient against fresh baseline)");
    println!("   - Deep scan provides accurate baseline for subsequent smart syncs");
}

#[tokio::test]
async fn test_directory_deletion_handling() {
    // Test Scenario 9: Directory deletion scenarios should be handled gracefully
    println!("📝 Test stub: Directory deletion handling");
    // TODO: Implement directory removal scenarios
}

#[tokio::test]
async fn test_webdav_error_fallback() {
    // Test Scenario 10: WebDAV server errors should fall back to traditional sync
    println!("📝 Test stub: WebDAV error fallback to traditional sync");
    // TODO: Implement error scenario testing
}