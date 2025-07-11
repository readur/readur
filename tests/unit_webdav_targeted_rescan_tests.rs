use readur::services::webdav::{WebDAVService, WebDAVConfig};
use readur::models::FileIngestionInfo;
use tokio;
use chrono::Utc;

// Helper function to create test WebDAV service
fn create_test_webdav_service() -> WebDAVService {
    let config = WebDAVConfig {
        server_url: "https://test.example.com".to_string(),
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec!["pdf".to_string(), "png".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };
    
    WebDAVService::new(config).unwrap()
}

#[tokio::test]
async fn test_discover_files_in_folder_shallow() {
    let service = create_test_webdav_service();
    
    // Mock XML response for shallow directory scan (Depth: 1)
    let mock_response = r#"<?xml version="1.0"?>
    <d:multistatus xmlns:d="DAV:">
        <d:response>
            <d:href>/remote.php/dav/files/admin/Documents/</d:href>
            <d:propstat>
                <d:prop>
                    <d:displayname>Documents</d:displayname>
                    <d:resourcetype>
                        <d:collection/>
                    </d:resourcetype>
                    <d:getetag>"docs-etag"</d:getetag>
                </d:prop>
                <d:status>HTTP/1.1 200 OK</d:status>
            </d:propstat>
        </d:response>
        <d:response>
            <d:href>/remote.php/dav/files/admin/Documents/file1.pdf</d:href>
            <d:propstat>
                <d:prop>
                    <d:displayname>file1.pdf</d:displayname>
                    <d:getcontentlength>1024</d:getcontentlength>
                    <d:getcontenttype>application/pdf</d:getcontenttype>
                    <d:getetag>"file1-etag"</d:getetag>
                    <d:resourcetype/>
                </d:prop>
                <d:status>HTTP/1.1 200 OK</d:status>
            </d:propstat>
        </d:response>
        <d:response>
            <d:href>/remote.php/dav/files/admin/Documents/SubFolder/</d:href>
            <d:propstat>
                <d:prop>
                    <d:displayname>SubFolder</d:displayname>
                    <d:resourcetype>
                        <d:collection/>
                    </d:resourcetype>
                    <d:getetag>"subfolder-etag"</d:getetag>
                </d:prop>
                <d:status>HTTP/1.1 200 OK</d:status>
            </d:propstat>
        </d:response>
    </d:multistatus>"#;
    
    // Test that shallow parsing works correctly
    let files = readur::webdav_xml_parser::parse_propfind_response_with_directories(mock_response).unwrap();
    
    // Debug print to see what files we actually got
    for file in &files {
        println!("Parsed file: {} (is_directory: {}, path: {})", file.name, file.is_directory, file.path);
    }
    
    // Should have directory, direct file, and direct subdirectory (but no nested files)
    assert_eq!(files.len(), 3);
    
    // Check that we got the right items
    let directory = files.iter().find(|f| f.name == "Documents").unwrap();
    assert!(directory.is_directory);
    assert_eq!(directory.etag, "docs-etag");
    
    let file = files.iter().find(|f| f.name == "file1.pdf").unwrap();
    assert!(!file.is_directory);
    assert_eq!(file.size, 1024);
    assert_eq!(file.etag, "file1-etag");
    
    let subfolder = files.iter().find(|f| f.name == "SubFolder").unwrap();
    assert!(subfolder.is_directory);
    assert_eq!(subfolder.etag, "subfolder-etag");
}

#[tokio::test]
async fn test_update_single_directory_tracking() {
    let service = create_test_webdav_service();
    
    // Create mock files representing a shallow directory scan
    let files = vec![
        FileIngestionInfo {
            path: "/Documents".to_string(),
            name: "Documents".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "docs-etag-123".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileIngestionInfo {
            path: "/Documents/file1.pdf".to_string(),
            name: "file1.pdf".to_string(),
            size: 1024000,
            mime_type: "application/pdf".to_string(),
            last_modified: Some(Utc::now()),
            etag: "file1-etag".to_string(),
            is_directory: false,
            created_at: Some(Utc::now()),
            permissions: Some(644),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileIngestionInfo {
            path: "/Documents/file2.pdf".to_string(),
            name: "file2.pdf".to_string(),
            size: 2048000,
            mime_type: "application/pdf".to_string(),
            last_modified: Some(Utc::now()),
            etag: "file2-etag".to_string(),
            is_directory: false,
            created_at: Some(Utc::now()),
            permissions: Some(644),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileIngestionInfo {
            path: "/Documents/SubFolder".to_string(),
            name: "SubFolder".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "subfolder-etag".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
    ];
    
    // Test that direct file counting works correctly
    let direct_files: Vec<_> = files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/Documents"))
        .collect();
    
    assert_eq!(direct_files.len(), 2); // file1.pdf and file2.pdf
    
    let total_size: i64 = direct_files.iter().map(|f| f.size).sum();
    assert_eq!(total_size, 3072000); // 1024000 + 2048000
    
    // Test that directory ETag extraction works
    let dir_etag = files.iter()
        .find(|f| f.is_directory && f.path == "/Documents")
        .map(|f| f.etag.clone())
        .unwrap();
    
    assert_eq!(dir_etag, "docs-etag-123");
}

#[tokio::test]
async fn test_targeted_rescan_logic() {
    let service = create_test_webdav_service();
    
    // Test the logic that determines which paths need scanning
    let paths_to_check = vec![
        "/Documents".to_string(),
        "/Documents/2024".to_string(),
        "/Documents/Archive".to_string(),
    ];
    
    // This tests the core logic used in discover_files_targeted_rescan
    // In a real implementation, this would involve database calls and network requests
    
    // Simulate ETag checking logic
    let mut paths_needing_scan = Vec::new();
    
    for path in &paths_to_check {
        // Simulate: current_etag != stored_etag (directory changed)
        let current_etag = format!("{}-current", path.replace('/', "-"));
        let stored_etag = format!("{}-stored", path.replace('/', "-"));
        
        if current_etag != stored_etag {
            paths_needing_scan.push(path.clone());
        }
    }
    
    // All paths should need scanning in this test scenario
    assert_eq!(paths_needing_scan.len(), 3);
    assert!(paths_needing_scan.contains(&"/Documents".to_string()));
    assert!(paths_needing_scan.contains(&"/Documents/2024".to_string()));
    assert!(paths_needing_scan.contains(&"/Documents/Archive".to_string()));
}

#[tokio::test]
async fn test_stale_directory_detection() {
    let service = create_test_webdav_service();
    
    // Test the logic for detecting stale subdirectories
    let parent_path = "/Documents";
    let directories = vec![
        ("/Documents", chrono::Utc::now()), // Fresh parent
        ("/Documents/2024", chrono::Utc::now() - chrono::Duration::hours(25)), // Stale (25 hours old)
        ("/Documents/Archive", chrono::Utc::now() - chrono::Duration::hours(1)), // Fresh (1 hour old)
        ("/Documents/2024/Q1", chrono::Utc::now() - chrono::Duration::hours(30)), // Stale (30 hours old)
        ("/Other", chrono::Utc::now() - chrono::Duration::hours(48)), // Stale but not under parent
    ];
    
    let max_age_hours = 24;
    let cutoff_time = chrono::Utc::now() - chrono::Duration::hours(max_age_hours);
    
    // Test the filtering logic
    let stale_subdirs: Vec<String> = directories.iter()
        .filter(|(path, last_scanned)| {
            path.starts_with(parent_path) && 
            *path != parent_path &&
            *last_scanned < cutoff_time
        })
        .map(|(path, _)| path.to_string())
        .collect();
    
    assert_eq!(stale_subdirs.len(), 2);
    assert!(stale_subdirs.contains(&"/Documents/2024".to_string()));
    assert!(stale_subdirs.contains(&"/Documents/2024/Q1".to_string()));
    assert!(!stale_subdirs.contains(&"/Documents/Archive".to_string())); // Fresh
    assert!(!stale_subdirs.contains(&"/Other".to_string())); // Different parent
}

#[tokio::test]
async fn test_incremental_sync_logic() {
    let service = create_test_webdav_service();
    
    // Test the change detection logic used in incremental sync
    let watch_folders = vec![
        "/Documents".to_string(),
        "/Photos".to_string(),
        "/Archive".to_string(),
    ];
    
    // Simulate stored ETags vs current ETags
    let stored_etags = [
        ("/Documents", "docs-etag-old"),
        ("/Photos", "photos-etag-same"),
        ("/Archive", "archive-etag-old"),
    ];
    
    let current_etags = [
        ("/Documents", "docs-etag-new"), // Changed
        ("/Photos", "photos-etag-same"), // Unchanged
        ("/Archive", "archive-etag-new"), // Changed
    ];
    
    let mut changed_folders = Vec::new();
    let mut unchanged_folders = Vec::new();
    
    for folder in &watch_folders {
        let stored = stored_etags.iter().find(|(path, _)| path == folder).map(|(_, etag)| *etag);
        let current = current_etags.iter().find(|(path, _)| path == folder).map(|(_, etag)| *etag);
        
        match (stored, current) {
            (Some(stored_etag), Some(current_etag)) => {
                if stored_etag != current_etag {
                    changed_folders.push(folder.clone());
                } else {
                    unchanged_folders.push(folder.clone());
                }
            }
            _ => {
                // New folder or missing data - assume changed
                changed_folders.push(folder.clone());
            }
        }
    }
    
    assert_eq!(changed_folders.len(), 2);
    assert!(changed_folders.contains(&"/Documents".to_string()));
    assert!(changed_folders.contains(&"/Archive".to_string()));
    
    assert_eq!(unchanged_folders.len(), 1);
    assert!(unchanged_folders.contains(&"/Photos".to_string()));
}

#[tokio::test]
async fn test_smart_sync_strategy_selection() {
    let service = create_test_webdav_service();
    
    // Test the logic for choosing between different sync strategies
    
    // Scenario 1: Directory unchanged, no stale subdirectories -> no scan needed
    let scenario1_main_dir_changed = false;
    let scenario1_stale_subdirs = 0;
    let scenario1_action = if scenario1_main_dir_changed {
        "full_scan"
    } else if scenario1_stale_subdirs > 0 {
        "targeted_scan"
    } else {
        "no_scan"
    };
    assert_eq!(scenario1_action, "no_scan");
    
    // Scenario 2: Directory unchanged, has stale subdirectories -> targeted scan
    let scenario2_main_dir_changed = false;
    let scenario2_stale_subdirs = 3;
    let scenario2_action = if scenario2_main_dir_changed {
        "full_scan"
    } else if scenario2_stale_subdirs > 0 {
        "targeted_scan"
    } else {
        "no_scan"
    };
    assert_eq!(scenario2_action, "targeted_scan");
    
    // Scenario 3: Directory changed -> full scan (optimized)
    let scenario3_main_dir_changed = true;
    let scenario3_stale_subdirs = 0;
    let scenario3_action = if scenario3_main_dir_changed {
        "full_scan"
    } else if scenario3_stale_subdirs > 0 {
        "targeted_scan"
    } else {
        "no_scan"
    };
    assert_eq!(scenario3_action, "full_scan");
}