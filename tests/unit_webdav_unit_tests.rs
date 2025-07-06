use readur::services::webdav::{WebDAVService, WebDAVConfig};
use readur::models::FileInfo;
use readur::models::*;
use tokio;

// Mock WebDAV server responses for testing
fn mock_propfind_response() -> String {
    r#"<?xml version="1.0" encoding="utf-8"?>
    <d:multistatus xmlns:d="DAV:">
        <d:response>
            <d:href>/webdav/Documents/test.pdf</d:href>
            <d:propstat>
                <d:prop>
                    <d:displayname>test.pdf</d:displayname>
                    <d:getcontentlength>1024000</d:getcontentlength>
                    <d:getlastmodified>Fri, 01 Jan 2024 12:00:00 GMT</d:getlastmodified>
                    <d:getcontenttype>application/pdf</d:getcontenttype>
                    <d:getetag>"abc123"</d:getetag>
                    <d:resourcetype/>
                </d:prop>
                <d:status>HTTP/1.1 200 OK</d:status>
            </d:propstat>
        </d:response>
        <d:response>
            <d:href>/webdav/Documents/image.png</d:href>
            <d:propstat>
                <d:prop>
                    <d:displayname>image.png</d:displayname>
                    <d:getcontentlength>512000</d:getcontentlength>
                    <d:getlastmodified>Fri, 01 Jan 2024 12:00:00 GMT</d:getlastmodified>
                    <d:getcontenttype>image/png</d:getcontenttype>
                    <d:getetag>"def456"</d:getetag>
                    <d:resourcetype/>
                </d:prop>
                <d:status>HTTP/1.1 200 OK</d:status>
            </d:propstat>
        </d:response>
        <d:response>
            <d:href>/webdav/Documents/folder/</d:href>
            <d:propstat>
                <d:prop>
                    <d:displayname>folder</d:displayname>
                    <d:getlastmodified>Fri, 01 Jan 2024 12:00:00 GMT</d:getlastmodified>
                    <d:resourcetype>
                        <d:collection/>
                    </d:resourcetype>
                </d:prop>
                <d:status>HTTP/1.1 200 OK</d:status>
            </d:propstat>
        </d:response>
    </d:multistatus>"#.to_string()
}

#[test]
fn test_webdav_config_creation() {
    let config = WebDAVConfig {
        server_url: "https://cloud.example.com".to_string(),
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec!["pdf".to_string(), "png".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };

    assert_eq!(config.server_url, "https://cloud.example.com");
    assert_eq!(config.username, "testuser");
    assert_eq!(config.password, "testpass");
    assert_eq!(config.watch_folders.len(), 1);
    assert_eq!(config.file_extensions.len(), 2);
    assert_eq!(config.timeout_seconds, 30);
    assert_eq!(config.server_type, Some("nextcloud".to_string()));
}

#[test]
fn test_webdav_service_creation() {
    let config = WebDAVConfig {
        server_url: "https://cloud.example.com".to_string(),
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec!["pdf".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };

    let result = WebDAVService::new(config);
    assert!(result.is_ok());
}

#[test]
fn test_webdav_response_parsing() {
    let config = WebDAVConfig {
        server_url: "https://cloud.example.com".to_string(),
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec!["pdf".to_string(), "png".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };

    let service = WebDAVService::new(config).unwrap();
    let response = mock_propfind_response();
    
    let files = readur::webdav_xml_parser::parse_propfind_response(&response);
    assert!(files.is_ok());

    let files = files.unwrap();
    assert_eq!(files.len(), 2); // Should have 2 files (excluding directory)

    // Check first file (test.pdf)
    let pdf_file = &files[0];
    assert_eq!(pdf_file.name, "test.pdf");
    assert_eq!(pdf_file.size, 1024000);
    assert_eq!(pdf_file.mime_type, "application/pdf");
    assert!(!pdf_file.is_directory);

    // Check second file (image.png)
    let png_file = &files[1];
    assert_eq!(png_file.name, "image.png");
    assert_eq!(png_file.size, 512000);
    assert_eq!(png_file.mime_type, "image/png");
    assert!(!png_file.is_directory);
}

#[test]
fn test_webdav_models() {
    // Test WebDAVFolderInfo
    let folder_info = WebDAVFolderInfo {
        path: "/Documents".to_string(),
        total_files: 100,
        supported_files: 75,
        estimated_time_hours: 2.5,
        total_size_mb: 250.0,
    };

    assert_eq!(folder_info.path, "/Documents");
    assert_eq!(folder_info.total_files, 100);
    assert_eq!(folder_info.supported_files, 75);
    assert_eq!(folder_info.estimated_time_hours, 2.5);
    assert_eq!(folder_info.total_size_mb, 250.0);

    // Test WebDAVCrawlEstimate
    let estimate = WebDAVCrawlEstimate {
        folders: vec![folder_info],
        total_files: 100,
        total_supported_files: 75,
        total_estimated_time_hours: 2.5,
        total_size_mb: 250.0,
    };

    assert_eq!(estimate.folders.len(), 1);
    assert_eq!(estimate.total_files, 100);
    assert_eq!(estimate.total_supported_files, 75);

    // Test WebDAVTestConnection
    let test_config = WebDAVTestConnection {
        server_url: "https://cloud.example.com".to_string(),
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        server_type: Some("nextcloud".to_string()),
    };

    assert_eq!(test_config.server_url, "https://cloud.example.com");
    assert_eq!(test_config.username, "testuser");
    assert_eq!(test_config.password, "testpass");
    assert_eq!(test_config.server_type, Some("nextcloud".to_string()));

    // Test WebDAVConnectionResult
    let result = WebDAVConnectionResult {
        success: true,
        message: "Connection successful".to_string(),
        server_version: Some("28.0.1".to_string()),
        server_type: Some("nextcloud".to_string()),
    };

    assert!(result.success);
    assert_eq!(result.message, "Connection successful");
    assert_eq!(result.server_version, Some("28.0.1".to_string()));
    assert_eq!(result.server_type, Some("nextcloud".to_string()));

    // Test WebDAVSyncStatus
    let sync_status = WebDAVSyncStatus {
        is_running: false,
        last_sync: None,
        files_processed: 42,
        files_remaining: 58,
        current_folder: Some("/Documents".to_string()),
        errors: vec!["Test error".to_string()],
    };

    assert!(!sync_status.is_running);
    assert_eq!(sync_status.files_processed, 42);
    assert_eq!(sync_status.files_remaining, 58);
    assert_eq!(sync_status.current_folder, Some("/Documents".to_string()));
    assert_eq!(sync_status.errors.len(), 1);
}

#[test]
fn test_file_extension_filtering() {
    let supported_extensions: std::collections::HashSet<String> = 
        vec!["pdf".to_string(), "png".to_string(), "jpg".to_string()].into_iter().collect();

    // Test supported extensions
    assert!(supported_extensions.contains("pdf"));
    assert!(supported_extensions.contains("png"));
    assert!(supported_extensions.contains("jpg"));

    // Test unsupported extensions
    assert!(!supported_extensions.contains("txt"));
    assert!(!supported_extensions.contains("doc"));
    assert!(!supported_extensions.contains("mp4"));

    // Test case sensitivity
    let filename = "document.PDF";
    let extension = std::path::Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase());
    
    assert_eq!(extension, Some("pdf".to_string()));
    assert!(supported_extensions.contains(&extension.unwrap()));
}

#[test]
fn test_time_estimation() {
    let files_count = 1000;
    let seconds_per_file = 2.0; // 2 seconds per file for OCR
    let estimated_seconds = files_count as f32 * seconds_per_file;
    let estimated_hours = estimated_seconds / 3600.0;

    assert_eq!(estimated_seconds, 2000.0);
    assert!((estimated_hours - 0.5556).abs() < 0.001); // Approximately 0.5556 hours
}

#[test]
fn test_size_calculation() {
    let size_bytes = vec![1024000i64, 512000i64, 2048000i64]; // 1MB, 0.5MB, 2MB
    let total_bytes: i64 = size_bytes.iter().sum();
    let total_mb = total_bytes as f64 / (1024.0 * 1024.0);

    assert_eq!(total_bytes, 3584000);
    assert!((total_mb - 3.4180).abs() < 0.001); // Approximately 3.418 MB
}

#[test]
fn test_settings_integration() {
    let settings = Settings::default();
    
    // Test default WebDAV settings
    assert!(!settings.webdav_enabled);
    assert_eq!(settings.webdav_server_url, None);
    assert_eq!(settings.webdav_username, None);
    assert_eq!(settings.webdav_password, None);
    assert_eq!(settings.webdav_watch_folders, vec!["/Documents".to_string()]);
    assert!(!settings.webdav_auto_sync);
    assert_eq!(settings.webdav_sync_interval_minutes, 60);

    // Test that WebDAV file extensions match default OCR extensions
    let expected_extensions = vec![
        "pdf".to_string(),
        "png".to_string(), 
        "jpg".to_string(),
        "jpeg".to_string(),
        "tiff".to_string(),
        "bmp".to_string(),
        "txt".to_string(),
    ];
    assert_eq!(settings.webdav_file_extensions, expected_extensions);
}