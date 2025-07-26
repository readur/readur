use readur::models::FileIngestionInfo;
use readur::services::webdav::{WebDAVConfig, WebDAVUrlManager};

#[test]
fn test_nextcloud_directory_path_handling() {
    let config = WebDAVConfig {
        server_url: "https://nas.example.com".to_string(),
        username: "testuser".to_string(),
        password: "password".to_string(),
        watch_folders: vec!["/Photos".to_string()],
        file_extensions: vec!["jpg".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };

    let manager = WebDAVUrlManager::new(config);

    // Test a directory from Nextcloud WebDAV response
    let directory_info = FileIngestionInfo {
        relative_path: "TEMP".to_string(),
        full_path: "/remote.php/dav/files/testuser/Photos/Subfolder/".to_string(),
        #[allow(deprecated)]
        path: "/remote.php/dav/files/testuser/Photos/Subfolder/".to_string(),
        name: "Subfolder".to_string(),
        size: 0,
        mime_type: "".to_string(),
        last_modified: None,
        etag: "dir123".to_string(),
        is_directory: true,
        created_at: None,
        permissions: None,
        owner: None,
        group: None,
        metadata: None,
    };

    let processed = manager.process_file_info(directory_info);

    // The relative_path should be correct for subdirectory scanning
    assert_eq!(processed.relative_path, "/Photos/Subfolder/");
    assert_eq!(processed.full_path, "/remote.php/dav/files/testuser/Photos/Subfolder/");
    
    // The legacy path field should also be set to relative path for backward compatibility
    #[allow(deprecated)]
    assert_eq!(processed.path, "/Photos/Subfolder/");
}

#[test]
fn test_nextcloud_file_path_handling() {
    let config = WebDAVConfig {
        server_url: "https://nas.example.com".to_string(),
        username: "testuser".to_string(),
        password: "password".to_string(),
        watch_folders: vec!["/Photos".to_string()],
        file_extensions: vec!["jpg".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };

    let manager = WebDAVUrlManager::new(config);

    // Test a file from Nextcloud WebDAV response
    let file_info = FileIngestionInfo {
        relative_path: "TEMP".to_string(),
        full_path: "/remote.php/dav/files/testuser/Photos/image.jpg".to_string(),
        #[allow(deprecated)]
        path: "/remote.php/dav/files/testuser/Photos/image.jpg".to_string(),
        name: "image.jpg".to_string(),
        size: 1024,
        mime_type: "image/jpeg".to_string(),
        last_modified: None,
        etag: "file123".to_string(),
        is_directory: false,
        created_at: None,
        permissions: None,
        owner: None,
        group: None,
        metadata: None,
    };

    let processed = manager.process_file_info(file_info);

    // The relative_path should be correct for file processing
    assert_eq!(processed.relative_path, "/Photos/image.jpg");
    assert_eq!(processed.full_path, "/remote.php/dav/files/testuser/Photos/image.jpg");
    
    // The legacy path field should also be set to relative path for backward compatibility
    #[allow(deprecated)]
    assert_eq!(processed.path, "/Photos/image.jpg");
}

#[test]
fn test_webdav_root_path_handling() {
    let config = WebDAVConfig {
        server_url: "https://nas.example.com".to_string(),
        username: "testuser".to_string(),
        password: "password".to_string(),
        watch_folders: vec!["/".to_string()],
        file_extensions: vec!["jpg".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };

    let manager = WebDAVUrlManager::new(config);

    // Test root directory handling
    let root_info = FileIngestionInfo {
        relative_path: "TEMP".to_string(),
        full_path: "/remote.php/dav/files/testuser".to_string(),
        #[allow(deprecated)]
        path: "/remote.php/dav/files/testuser".to_string(),
        name: "testuser".to_string(),
        size: 0,
        mime_type: "".to_string(),
        last_modified: None,
        etag: "root123".to_string(),
        is_directory: true,
        created_at: None,
        permissions: None,
        owner: None,
        group: None,
        metadata: None,
    };

    let processed = manager.process_file_info(root_info);

    // Root should map to "/"
    assert_eq!(processed.relative_path, "/");
    assert_eq!(processed.full_path, "/remote.php/dav/files/testuser");
}

#[test]
fn test_url_construction_from_relative_path() {
    let config = WebDAVConfig {
        server_url: "https://nas.example.com".to_string(),
        username: "testuser".to_string(),
        password: "password".to_string(),
        watch_folders: vec!["/Photos".to_string()],
        file_extensions: vec!["jpg".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };

    let manager = WebDAVUrlManager::new(config);

    // Test URL construction for scanning subdirectories
    let subfolder_url = manager.relative_path_to_url("/Photos/Subfolder/");
    assert_eq!(subfolder_url, "https://nas.example.com/remote.php/dav/files/testuser/Photos/Subfolder/");

    let file_url = manager.relative_path_to_url("/Photos/image.jpg");
    assert_eq!(file_url, "https://nas.example.com/remote.php/dav/files/testuser/Photos/image.jpg");

    let root_url = manager.relative_path_to_url("/");
    assert_eq!(root_url, "https://nas.example.com/remote.php/dav/files/testuser");
}

#[test]
fn test_owncloud_path_handling() {
    let config = WebDAVConfig {
        server_url: "https://cloud.example.com".to_string(),
        username: "user123".to_string(),
        password: "password".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec!["pdf".to_string()],
        timeout_seconds: 30,
        server_type: Some("owncloud".to_string()),
    };

    let manager = WebDAVUrlManager::new(config);

    // Test ownCloud path conversion
    let file_info = FileIngestionInfo {
        relative_path: "TEMP".to_string(),
        full_path: "/remote.php/webdav/Documents/report.pdf".to_string(),
        #[allow(deprecated)]
        path: "/remote.php/webdav/Documents/report.pdf".to_string(),
        name: "report.pdf".to_string(),
        size: 2048,
        mime_type: "application/pdf".to_string(),
        last_modified: None,
        etag: "pdf456".to_string(),
        is_directory: false,
        created_at: None,
        permissions: None,
        owner: None,
        group: None,
        metadata: None,
    };

    let processed = manager.process_file_info(file_info);
    assert_eq!(processed.relative_path, "/Documents/report.pdf");
    assert_eq!(processed.full_path, "/remote.php/webdav/Documents/report.pdf");
}

#[test]
fn test_generic_webdav_path_handling() {
    let config = WebDAVConfig {
        server_url: "https://webdav.example.com".to_string(),
        username: "user".to_string(),
        password: "password".to_string(),
        watch_folders: vec!["/files".to_string()],
        file_extensions: vec!["txt".to_string()],
        timeout_seconds: 30,
        server_type: Some("generic".to_string()),
    };

    let manager = WebDAVUrlManager::new(config);

    // Test generic WebDAV path conversion
    let file_info = FileIngestionInfo {
        relative_path: "TEMP".to_string(),
        full_path: "/webdav/files/document.txt".to_string(),
        #[allow(deprecated)]
        path: "/webdav/files/document.txt".to_string(),
        name: "document.txt".to_string(),
        size: 512,
        mime_type: "text/plain".to_string(),
        last_modified: None,
        etag: "txt789".to_string(),
        is_directory: false,
        created_at: None,
        permissions: None,
        owner: None,
        group: None,
        metadata: None,
    };

    let processed = manager.process_file_info(file_info);
    assert_eq!(processed.relative_path, "/files/document.txt");
    assert_eq!(processed.full_path, "/webdav/files/document.txt");
}

/// Test download path resolution for WebDAV service compatibility
#[test]
fn test_download_path_resolution() {
    let config = WebDAVConfig {
        server_url: "https://nas.example.com".to_string(),
        username: "testuser".to_string(),
        password: "password".to_string(),
        watch_folders: vec!["/Photos".to_string()],
        file_extensions: vec!["jpg".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };

    let manager = WebDAVUrlManager::new(config);

    // Test that processed file info has correct paths for download operations
    let file_info = FileIngestionInfo {
        relative_path: "TEMP".to_string(),
        full_path: "/remote.php/dav/files/testuser/Photos/image.jpg".to_string(),
        #[allow(deprecated)]
        path: "/remote.php/dav/files/testuser/Photos/image.jpg".to_string(),
        name: "image.jpg".to_string(),
        size: 1024,
        mime_type: "image/jpeg".to_string(),
        last_modified: None,
        etag: "file123".to_string(),
        is_directory: false,
        created_at: None,
        permissions: None,
        owner: None,
        group: None,
        metadata: None,
    };

    let processed = manager.process_file_info(file_info);

    // The relative_path should be clean and usable for download operations
    assert_eq!(processed.relative_path, "/Photos/image.jpg");
    
    // The download URL should be correctly constructed from relative path
    let download_url = manager.relative_path_to_url(&processed.relative_path);
    assert_eq!(download_url, "https://nas.example.com/remote.php/dav/files/testuser/Photos/image.jpg");
    
    // The full_path should preserve the original server response
    assert_eq!(processed.full_path, "/remote.php/dav/files/testuser/Photos/image.jpg");
}

/// Test using the actual Nextcloud XML fixture to ensure our path handling works with real data
#[test]
fn test_with_nextcloud_fixture_data() {
    use readur::webdav_xml_parser::parse_propfind_response_with_directories;
    
    let config = WebDAVConfig {
        server_url: "https://nas.jonathonfuller.com".to_string(),
        username: "perf3ct".to_string(),
        password: "password".to_string(),
        watch_folders: vec!["/Photos".to_string()],
        file_extensions: vec!["jpg".to_string(), "jpeg".to_string(), "png".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };

    let manager = WebDAVUrlManager::new(config);

    // Load the real Nextcloud XML fixture
    let fixture_path = "tests/fixtures/webdav/nextcloud_photos_propfind_response.xml";
    let xml_content = std::fs::read_to_string(fixture_path)
        .expect("Should be able to read the Nextcloud fixture file");

    // Parse the XML
    let parsed_items = parse_propfind_response_with_directories(&xml_content)
        .expect("Should be able to parse the Nextcloud XML");

    // Process the items through url_manager
    let processed_items = manager.process_file_infos(parsed_items);

    // Verify that we got some items and they're properly processed
    assert!(!processed_items.is_empty(), "Should have parsed some items from the fixture");

    // Check that all items have proper relative paths (not the temp value)
    for item in &processed_items {
        assert_ne!(item.relative_path, "TEMP", "All items should have processed relative_path");
        assert!(item.relative_path.starts_with("/"), "Relative paths should start with /");
        
        // Relative paths should not contain the Nextcloud WebDAV prefix
        assert!(!item.relative_path.contains("/remote.php/dav/files/"), 
                "Relative path should not contain WebDAV prefix: {}", item.relative_path);
        
        // But full_path should contain the prefix
        assert!(item.full_path.contains("/remote.php/dav/files/"), 
                "Full path should contain WebDAV prefix: {}", item.full_path);
    }

    // Check for both files and directories
    let files: Vec<_> = processed_items.iter().filter(|item| !item.is_directory).collect();
    let directories: Vec<_> = processed_items.iter().filter(|item| item.is_directory).collect();
    
    println!("Parsed {} files and {} directories from fixture", files.len(), directories.len());
    
    // There should be at least some files and directories in the Photos folder
    assert!(!files.is_empty(), "Should have found some files");
    
    // Verify file relative paths look correct
    for file in files {
        assert!(file.relative_path.starts_with("/Photos/") || file.relative_path == "/Photos", 
                "File relative path should be under Photos: {}", file.relative_path);
    }
}