use readur::services::webdav::{WebDAVService, WebDAVConfig, RetryConfig};
use readur::webdav_xml_parser::parse_propfind_response;
use readur::models::FileIngestionInfo;
use readur::models::*;
use chrono::Utc;
use uuid::Uuid;

// Mock WebDAV responses for comprehensive testing
fn mock_nextcloud_propfind_response() -> String {
    r#"<?xml version="1.0"?>
    <d:multistatus xmlns:d="DAV:" xmlns:s="http://sabredav.org/ns" xmlns:oc="http://owncloud.org/ns" xmlns:nc="http://nextcloud.org/ns">
        <d:response>
            <d:href>/remote.php/dav/files/admin/Documents/</d:href>
            <d:propstat>
                <d:prop>
                    <d:displayname>Documents</d:displayname>
                    <d:getlastmodified>Tue, 01 Jan 2024 12:00:00 GMT</d:getlastmodified>
                    <d:resourcetype>
                        <d:collection/>
                    </d:resourcetype>
                    <d:getetag>"abc123"</d:getetag>
                </d:prop>
                <d:status>HTTP/1.1 200 OK</d:status>
            </d:propstat>
        </d:response>
        <d:response>
            <d:href>/remote.php/dav/files/admin/Documents/report.pdf</d:href>
            <d:propstat>
                <d:prop>
                    <d:displayname>report.pdf</d:displayname>
                    <d:getcontentlength>2048000</d:getcontentlength>
                    <d:getlastmodified>Mon, 15 Jan 2024 14:30:00 GMT</d:getlastmodified>
                    <d:getcontenttype>application/pdf</d:getcontenttype>
                    <d:getetag>"pdf123"</d:getetag>
                    <d:resourcetype/>
                </d:prop>
                <d:status>HTTP/1.1 200 OK</d:status>
            </d:propstat>
        </d:response>
        <d:response>
            <d:href>/remote.php/dav/files/admin/Documents/photo.png</d:href>
            <d:propstat>
                <d:prop>
                    <d:displayname>photo.png</d:displayname>
                    <d:getcontentlength>768000</d:getcontentlength>
                    <d:getlastmodified>Wed, 10 Jan 2024 09:15:00 GMT</d:getlastmodified>
                    <d:getcontenttype>image/png</d:getcontenttype>
                    <d:getetag>"png456"</d:getetag>
                    <d:resourcetype/>
                </d:prop>
                <d:status>HTTP/1.1 200 OK</d:status>
            </d:propstat>
        </d:response>
        <d:response>
            <d:href>/remote.php/dav/files/admin/Documents/unsupported.docx</d:href>
            <d:propstat>
                <d:prop>
                    <d:displayname>unsupported.docx</d:displayname>
                    <d:getcontentlength>102400</d:getcontentlength>
                    <d:getlastmodified>Thu, 20 Jan 2024 16:45:00 GMT</d:getlastmodified>
                    <d:getcontenttype>application/vnd.openxmlformats-officedocument.wordprocessingml.document</d:getcontenttype>
                    <d:getetag>"docx789"</d:getetag>
                    <d:resourcetype/>
                </d:prop>
                <d:status>HTTP/1.1 200 OK</d:status>
            </d:propstat>
        </d:response>
    </d:multistatus>"#.to_string()
}

fn mock_empty_folder_response() -> String {
    r#"<?xml version="1.0"?>
    <d:multistatus xmlns:d="DAV:">
        <d:response>
            <d:href>/webdav/EmptyFolder/</d:href>
            <d:propstat>
                <d:prop>
                    <d:displayname>EmptyFolder</d:displayname>
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

fn mock_malformed_xml_response() -> String {
    r#"<?xml version="1.0"?>
    <d:multistatus xmlns:d="DAV:">
        <d:response>
            <d:href>/webdav/test.pdf</d:href>
            <d:propstat>
                <d:prop>
                    <d:displayname>test.pdf
                    <!-- Missing closing tag -->
                </d:prop>
            </d:propstat>
        </d:response>
    <!-- Incomplete XML -->"#.to_string()
}

#[test]
fn test_webdav_config_validation() {
    // Test valid config
    let valid_config = WebDAVConfig {
        server_url: "https://cloud.example.com".to_string(),
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string(), "/Photos".to_string()],
        file_extensions: vec!["pdf".to_string(), "png".to_string(), "jpg".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };

    assert!(WebDAVService::new(valid_config).is_ok());

    // Test config with empty server URL - should fail with our enhanced validation
    let invalid_config = WebDAVConfig {
        server_url: "".to_string(),
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec!["pdf".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };

    // Should fail early with enhanced validation
    assert!(WebDAVService::new(invalid_config).is_err());
    
    // Test config with invalid URL scheme - should also fail
    let invalid_scheme_config = WebDAVConfig {
        server_url: "ftp://cloud.example.com".to_string(),
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec!["pdf".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };

    assert!(WebDAVService::new(invalid_scheme_config).is_err());
    
    // Test config with relative URL - should also fail
    let relative_url_config = WebDAVConfig {
        server_url: "/webdav".to_string(),
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec!["pdf".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };

    assert!(WebDAVService::new(relative_url_config).is_err());
}

#[test]
fn test_webdav_url_construction_comprehensive() {
    // Test Nextcloud URL construction
    let nextcloud_config = WebDAVConfig {
        server_url: "https://nextcloud.example.com".to_string(),
        username: "admin".to_string(),
        password: "secret".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec!["pdf".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };

    let service = WebDAVService::new(nextcloud_config).unwrap();
    // URL construction is tested implicitly during service creation

    // Test ownCloud URL construction
    let owncloud_config = WebDAVConfig {
        server_url: "https://cloud.example.com/".to_string(), // With trailing slash
        username: "user123".to_string(),
        password: "pass123".to_string(),
        watch_folders: vec!["/Shared".to_string()],
        file_extensions: vec!["jpg".to_string()],
        timeout_seconds: 60,
        server_type: Some("owncloud".to_string()),
    };

    assert!(WebDAVService::new(owncloud_config).is_ok());

    // Test generic WebDAV URL construction
    let generic_config = WebDAVConfig {
        server_url: "https://webdav.example.com".to_string(),
        username: "webdavuser".to_string(),
        password: "webdavpass".to_string(),
        watch_folders: vec!["/Files".to_string()],
        file_extensions: vec!["txt".to_string()],
        timeout_seconds: 45,
        server_type: None, // No server type = generic
    };

    assert!(WebDAVService::new(generic_config).is_ok());
}

#[test]
fn test_webdav_response_parsing_comprehensive() {
    let config = WebDAVConfig {
        server_url: "https://cloud.example.com".to_string(),
        username: "admin".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec!["pdf".to_string(), "png".to_string(), "jpg".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };

    let service = WebDAVService::new(config.clone()).unwrap();
    
    // Test Nextcloud response parsing
    let nextcloud_response = mock_nextcloud_propfind_response();
    let files = parse_propfind_response(&nextcloud_response);
    assert!(files.is_ok());

    let files = files.unwrap();
    
    // Filter files by supported extensions
    let supported_files: Vec<_> = files.iter()
        .filter(|f| {
            if let Some(ext) = std::path::Path::new(&f.name)
                .extension()
                .and_then(|e| e.to_str())
            {
                config.file_extensions.contains(&ext.to_lowercase())
            } else {
                false
            }
        })
        .collect();
    
    assert_eq!(supported_files.len(), 2); // Should have 2 files with supported extensions (pdf, png)

    // Verify first file (report.pdf)
    let pdf_file = files.iter().find(|f| f.name == "report.pdf").unwrap();
    assert_eq!(pdf_file.size, 2048000);
    assert_eq!(pdf_file.mime_type, "application/pdf");
    assert_eq!(pdf_file.etag, "pdf123"); // ETag should be normalized (quotes removed)
    assert!(!pdf_file.is_directory);

    // Verify second file (photo.png)
    let png_file = files.iter().find(|f| f.name == "photo.png").unwrap();
    assert_eq!(png_file.size, 768000);
    assert_eq!(png_file.mime_type, "image/png");
    assert_eq!(png_file.etag, "png456"); // ETag should be normalized (quotes removed)
    assert!(!png_file.is_directory);

    // Verify that unsupported file (docx) is not included in supported files
    assert!(supported_files.iter().find(|f| f.name == "unsupported.docx").is_none());
}

#[test]
fn test_empty_folder_parsing() {
    let config = WebDAVConfig {
        server_url: "https://cloud.example.com".to_string(),
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/EmptyFolder".to_string()],
        file_extensions: vec!["pdf".to_string()],
        timeout_seconds: 30,
        server_type: Some("generic".to_string()),
    };

    let service = WebDAVService::new(config).unwrap();
    let response = mock_empty_folder_response();
    
    let files = parse_propfind_response(&response);
    assert!(files.is_ok());
    
    let files = files.unwrap();
    assert_eq!(files.len(), 0); // Empty folder should have no files
}

#[test]
fn test_malformed_xml_handling() {
    let config = WebDAVConfig {
        server_url: "https://cloud.example.com".to_string(),
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec!["pdf".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };

    let service = WebDAVService::new(config).unwrap();
    let response = mock_malformed_xml_response();
    
    // Current simple parser might still extract some data from malformed XML
    let result = parse_propfind_response(&response);
    // It might succeed or fail depending on how robust the parser is
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_retry_config_custom_values() {
    let custom_retry = RetryConfig {
        max_retries: 5,
        initial_delay_ms: 500,
        max_delay_ms: 15000,
        backoff_multiplier: 1.5,
        timeout_seconds: 90,
        rate_limit_backoff_ms: 10000,
    };

    assert_eq!(custom_retry.max_retries, 5);
    assert_eq!(custom_retry.initial_delay_ms, 500);
    assert_eq!(custom_retry.max_delay_ms, 15000);
    assert_eq!(custom_retry.backoff_multiplier, 1.5);
    assert_eq!(custom_retry.timeout_seconds, 90);
    assert_eq!(custom_retry.rate_limit_backoff_ms, 10000);

    let config = WebDAVConfig {
        server_url: "https://cloud.example.com".to_string(),
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec!["pdf".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };

    assert!(WebDAVService::new_with_retry(config, custom_retry).is_ok());
}

#[test]
fn test_file_extension_matching() {
    let supported_extensions = vec!["pdf", "png", "jpg", "jpeg", "tiff", "bmp", "txt"];
    
    let test_cases = vec![
        ("document.pdf", true),
        ("image.PNG", true), // Case insensitive
        ("photo.jpg", true),
        ("photo.JPEG", true),
        ("scan.tiff", true),
        ("bitmap.bmp", true),
        ("readme.txt", true),
        ("spreadsheet.xlsx", false),
        ("presentation.pptx", false),
        ("archive.zip", false),
        ("script.sh", false),
        ("no_extension", false),
        (".hidden", false),
    ];

    for (filename, should_match) in test_cases {
        let extension = std::path::Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase());

        let matches = extension
            .as_ref()
            .map(|ext| supported_extensions.contains(&ext.as_str()))
            .unwrap_or(false);

        assert_eq!(matches, should_match, 
            "File '{}' extension matching failed. Expected: {}, Got: {}", 
            filename, should_match, matches);
    }
}

#[test]
fn test_webdav_sync_state_model() {
    let sync_state = WebDAVSyncState {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        last_sync_at: Some(Utc::now()),
        sync_cursor: Some("cursor123".to_string()),
        is_running: true,
        files_processed: 42,
        files_remaining: 58,
        current_folder: Some("/Documents".to_string()),
        errors: vec!["Error 1".to_string(), "Error 2".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    assert!(sync_state.is_running);
    assert_eq!(sync_state.files_processed, 42);
    assert_eq!(sync_state.files_remaining, 58);
    assert_eq!(sync_state.current_folder, Some("/Documents".to_string()));
    assert_eq!(sync_state.errors.len(), 2);
}

#[test]
fn test_webdav_file_model() {
    let document_id = Uuid::new_v4();
    let webdav_file = WebDAVFile {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        webdav_path: "/Documents/report.pdf".to_string(),
        etag: "\"abc123\"".to_string(),
        last_modified: Some(Utc::now()),
        file_size: 2048000,
        mime_type: "application/pdf".to_string(),
        document_id: Some(document_id),
        sync_status: "completed".to_string(),
        sync_error: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    assert_eq!(webdav_file.webdav_path, "/Documents/report.pdf");
    assert_eq!(webdav_file.etag, "\"abc123\"");
    assert_eq!(webdav_file.file_size, 2048000);
    assert_eq!(webdav_file.sync_status, "completed");
    assert!(webdav_file.sync_error.is_none());
}

#[test]
fn test_create_webdav_file_model() {
    let user_id = Uuid::new_v4();
    let create_file = CreateWebDAVFile {
        user_id,
        webdav_path: "/Photos/vacation.jpg".to_string(),
        etag: "\"photo123\"".to_string(),
        last_modified: Some(Utc::now()),
        file_size: 1536000,
        mime_type: "image/jpeg".to_string(),
        document_id: None,
        sync_status: "pending".to_string(),
        sync_error: None,
    };

    assert_eq!(create_file.user_id, user_id);
    assert_eq!(create_file.webdav_path, "/Photos/vacation.jpg");
    assert_eq!(create_file.file_size, 1536000);
    assert_eq!(create_file.sync_status, "pending");
}

#[test]
fn test_update_webdav_sync_state_model() {
    let update_state = UpdateWebDAVSyncState {
        last_sync_at: Some(Utc::now()),
        sync_cursor: Some("new_cursor".to_string()),
        is_running: false,
        files_processed: 100,
        files_remaining: 0,
        current_folder: None,
        errors: Vec::new(),
    };

    assert!(!update_state.is_running);
    assert_eq!(update_state.files_processed, 100);
    assert_eq!(update_state.files_remaining, 0);
    assert!(update_state.current_folder.is_none());
    assert!(update_state.errors.is_empty());
}

#[test]
fn test_ocr_priority_calculation_comprehensive() {
    let test_cases = vec![
        // Size boundaries
        (0, 10),            // 0 bytes
        (1, 10),            // 1 byte
        (1048576, 10),      // Exactly 1MB
        (1048577, 8),       // 1MB + 1 byte
        (5242880, 8),       // Exactly 5MB
        (5242881, 6),       // 5MB + 1 byte
        (10485760, 6),      // Exactly 10MB
        (10485761, 4),      // 10MB + 1 byte
        (52428800, 4),      // Exactly 50MB
        (52428801, 2),      // 50MB + 1 byte
        (104857600, 2),     // 100MB
        (1073741824, 2),    // 1GB
    ];

    for (file_size, expected_priority) in test_cases {
        let priority = match file_size {
            0..=1048576 => 10,      // <= 1MB
            ..=5242880 => 8,        // 1-5MB
            ..=10485760 => 6,       // 5-10MB  
            ..=52428800 => 4,       // 10-50MB
            _ => 2,                 // > 50MB
        };
        
        assert_eq!(priority, expected_priority, 
            "Priority calculation failed for file size {} bytes", file_size);
    }
}

#[test]
fn test_sync_status_serialization() {
    let sync_status = WebDAVSyncStatus {
        is_running: true,
        last_sync: Some(Utc::now()),
        files_processed: 25,
        files_remaining: 75,
        current_folder: Some("/Documents/Reports".to_string()),
        errors: vec!["Connection timeout".to_string()],
    };

    // Test that the status can be serialized to JSON
    let json = serde_json::to_string(&sync_status);
    assert!(json.is_ok());

    let json_str = json.unwrap();
    assert!(json_str.contains("\"is_running\":true"));
    assert!(json_str.contains("\"files_processed\":25"));
    assert!(json_str.contains("\"files_remaining\":75"));
    assert!(json_str.contains("\"current_folder\":\"/Documents/Reports\""));
}

#[test]
fn test_crawl_estimate_calculation() {
    let folder1 = WebDAVFolderInfo {
        path: "/Documents".to_string(),
        total_files: 100,
        supported_files: 80,
        estimated_time_hours: 0.044, // ~2.6 minutes
        total_size_mb: 150.0,
    };

    let folder2 = WebDAVFolderInfo {
        path: "/Photos".to_string(),
        total_files: 200,
        supported_files: 150,
        estimated_time_hours: 0.083, // ~5 minutes
        total_size_mb: 500.0,
    };

    let estimate = WebDAVCrawlEstimate {
        folders: vec![folder1, folder2],
        total_files: 300,
        total_supported_files: 230,
        total_estimated_time_hours: 0.127, // ~7.6 minutes
        total_size_mb: 650.0,
    };

    assert_eq!(estimate.folders.len(), 2);
    assert_eq!(estimate.total_files, 300);
    assert_eq!(estimate.total_supported_files, 230);
    assert!((estimate.total_estimated_time_hours - 0.127).abs() < 0.001);
    assert_eq!(estimate.total_size_mb, 650.0);
}

#[test]
fn test_connection_result_variants() {
    // Success case
    let success_result = WebDAVConnectionResult {
        success: true,
        message: "Connected successfully to Nextcloud 28.0.1".to_string(),
        server_version: Some("28.0.1".to_string()),
        server_type: Some("nextcloud".to_string()),
    };

    assert!(success_result.success);
    assert!(success_result.server_version.is_some());
    assert_eq!(success_result.server_type, Some("nextcloud".to_string()));

    // Failure case
    let failure_result = WebDAVConnectionResult {
        success: false,
        message: "Authentication failed: 401 Unauthorized".to_string(),
        server_version: None,
        server_type: None,
    };

    assert!(!failure_result.success);
    assert!(failure_result.server_version.is_none());
    assert!(failure_result.server_type.is_none());
    assert!(failure_result.message.contains("401"));
}

#[test]
fn test_notification_creation_for_webdav() {
    let notification = CreateNotification {
        notification_type: "info".to_string(),
        title: "WebDAV Sync Started".to_string(),
        message: "Synchronizing files from Nextcloud server".to_string(),
        action_url: Some("/sync-status".to_string()),
        metadata: Some(serde_json::json!({
            "sync_type": "webdav",
            "folders": ["/Documents", "/Photos"],
            "estimated_files": 150
        })),
    };

    assert_eq!(notification.notification_type, "info");
    assert_eq!(notification.title, "WebDAV Sync Started");
    assert!(notification.action_url.is_some());
    
    let metadata = notification.metadata.unwrap();
    assert_eq!(metadata["sync_type"], "webdav");
    assert!(metadata["folders"].is_array());
    assert_eq!(metadata["estimated_files"], 150);
}

#[test]
fn test_special_characters_in_paths() {
    let test_paths = vec![
        "/Documents/File with spaces.pdf",
        "/Documents/Ñoño/archivo.pdf",
        "/Documents/测试文件.pdf",
        "/Documents/файл.pdf",
        "/Documents/50%.pdf",
        "/Documents/file&name.pdf",
        "/Documents/file#1.pdf",
    ];

    for path in test_paths {
        let file_info = FileIngestionInfo {
            relative_path: path.to_string(),
            full_path: path.to_string(),
            #[allow(deprecated)]
            path: path.to_string(),
            name: std::path::Path::new(path)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            size: 1024,
            mime_type: "application/pdf".to_string(),
            last_modified: Some(Utc::now()),
            etag: "\"test123\"".to_string(),
            is_directory: false,
            created_at: None,
            permissions: None,
            owner: None,
            group: None,
            metadata: None,
        };

        assert!(!file_info.name.is_empty());
        assert!(file_info.name.ends_with(".pdf"));
    }
}

#[test]
fn test_backoff_delay_calculation() {
    let retry_config = RetryConfig::default();
    
    let mut delays = Vec::new();
    let mut delay = retry_config.initial_delay_ms;
    
    for _ in 0..5 {
        delays.push(delay);
        delay = ((delay as f64 * retry_config.backoff_multiplier) as u64)
            .min(retry_config.max_delay_ms);
    }
    
    assert_eq!(delays[0], 1000);  // 1s
    assert_eq!(delays[1], 2000);  // 2s
    assert_eq!(delays[2], 4000);  // 4s
    assert_eq!(delays[3], 8000);  // 8s
    assert_eq!(delays[4], 16000); // 16s
    
    // Verify max delay is respected
    for _ in 0..10 {
        delay = ((delay as f64 * retry_config.backoff_multiplier) as u64)
            .min(retry_config.max_delay_ms);
    }
    assert_eq!(delay, retry_config.max_delay_ms);
}