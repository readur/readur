use readur::services::webdav_service::{WebDAVService, WebDAVConfig};
use readur::models::FileInfo;
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

// Mock XML response for directory ETag check
fn mock_directory_etag_response(etag: &str) -> String {
    format!(r#"<?xml version="1.0"?>
    <d:multistatus xmlns:d="DAV:">
        <d:response>
            <d:href>/remote.php/dav/files/admin/Documents/</d:href>
            <d:propstat>
                <d:prop>
                    <d:getetag>"{}"</d:getetag>
                </d:prop>
                <d:status>HTTP/1.1 200 OK</d:status>
            </d:propstat>
        </d:response>
    </d:multistatus>"#, etag)
}

// Mock complex nested directory structure
fn mock_nested_directory_files() -> Vec<FileInfo> {
    vec![
        // Root directory
        FileInfo {
            path: "/Documents".to_string(),
            name: "Documents".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "root-etag-123".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        // Level 1 directories
        FileInfo {
            path: "/Documents/2024".to_string(),
            name: "2024".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "2024-etag-456".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: "/Documents/Archive".to_string(),
            name: "Archive".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "archive-etag-789".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        // Level 2 directories
        FileInfo {
            path: "/Documents/2024/Q1".to_string(),
            name: "Q1".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "q1-etag-101".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: "/Documents/2024/Q2".to_string(),
            name: "Q2".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "q2-etag-102".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        // Level 3 directory
        FileInfo {
            path: "/Documents/2024/Q1/Reports".to_string(),
            name: "Reports".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "reports-etag-201".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        // Files at various levels
        FileInfo {
            path: "/Documents/root-file.pdf".to_string(),
            name: "root-file.pdf".to_string(),
            size: 1024000,
            mime_type: "application/pdf".to_string(),
            last_modified: Some(Utc::now()),
            etag: "root-file-etag".to_string(),
            is_directory: false,
            created_at: Some(Utc::now()),
            permissions: Some(644),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: "/Documents/2024/annual-report.pdf".to_string(),
            name: "annual-report.pdf".to_string(),
            size: 2048000,
            mime_type: "application/pdf".to_string(),
            last_modified: Some(Utc::now()),
            etag: "annual-report-etag".to_string(),
            is_directory: false,
            created_at: Some(Utc::now()),
            permissions: Some(644),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: "/Documents/2024/Q1/q1-summary.pdf".to_string(),
            name: "q1-summary.pdf".to_string(),
            size: 512000,
            mime_type: "application/pdf".to_string(),
            last_modified: Some(Utc::now()),
            etag: "q1-summary-etag".to_string(),
            is_directory: false,
            created_at: Some(Utc::now()),
            permissions: Some(644),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: "/Documents/2024/Q1/Reports/detailed-report.pdf".to_string(),
            name: "detailed-report.pdf".to_string(),
            size: 4096000,
            mime_type: "application/pdf".to_string(),
            last_modified: Some(Utc::now()),
            etag: "detailed-report-etag".to_string(),
            is_directory: false,
            created_at: Some(Utc::now()),
            permissions: Some(644),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: "/Documents/Archive/old-document.pdf".to_string(),
            name: "old-document.pdf".to_string(),
            size: 256000,
            mime_type: "application/pdf".to_string(),
            last_modified: Some(Utc::now()),
            etag: "old-document-etag".to_string(),
            is_directory: false,
            created_at: Some(Utc::now()),
            permissions: Some(644),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
    ]
}

#[tokio::test]
async fn test_parse_directory_etag() {
    let service = create_test_webdav_service();
    
    // Test parsing a simple directory ETag response
    let xml_response = mock_directory_etag_response("test-etag-123");
    let etag = service.parse_directory_etag(&xml_response).unwrap();
    
    assert_eq!(etag, "test-etag-123");
}

#[tokio::test]
async fn test_parse_directory_etag_with_quotes() {
    let service = create_test_webdav_service();
    
    // Test ETag normalization (removing quotes)
    let xml_response = r#"<?xml version="1.0"?>
    <d:multistatus xmlns:d="DAV:">
        <d:response>
            <d:href>/remote.php/dav/files/admin/Documents/</d:href>
            <d:propstat>
                <d:prop>
                    <d:getetag>"quoted-etag-456"</d:getetag>
                </d:prop>
                <d:status>HTTP/1.1 200 OK</d:status>
            </d:propstat>
        </d:response>
    </d:multistatus>"#;
    
    let etag = service.parse_directory_etag(xml_response).unwrap();
    assert_eq!(etag, "quoted-etag-456");
}

#[tokio::test]
async fn test_parse_directory_etag_weak_etag() {
    let service = create_test_webdav_service();
    
    // Test weak ETag normalization
    let xml_response = r#"<?xml version="1.0"?>
    <d:multistatus xmlns:d="DAV:">
        <d:response>
            <d:href>/remote.php/dav/files/admin/Documents/</d:href>
            <d:propstat>
                <d:prop>
                    <d:getetag>W/"weak-etag-789"</d:getetag>
                </d:prop>
                <d:status>HTTP/1.1 200 OK</d:status>
            </d:propstat>
        </d:response>
    </d:multistatus>"#;
    
    let etag = service.parse_directory_etag(xml_response).unwrap();
    assert_eq!(etag, "weak-etag-789");
}

#[tokio::test]
async fn test_is_direct_child() {
    let service = create_test_webdav_service();
    
    // Test direct child detection
    assert!(service.is_direct_child("/Documents/file.pdf", "/Documents"));
    assert!(service.is_direct_child("/Documents/subfolder", "/Documents"));
    
    // Test non-direct children (nested deeper)
    assert!(!service.is_direct_child("/Documents/2024/file.pdf", "/Documents"));
    assert!(!service.is_direct_child("/Documents/2024/Q1/file.pdf", "/Documents"));
    
    // Test root directory edge case
    assert!(service.is_direct_child("/Documents", ""));
    assert!(service.is_direct_child("/Documents", "/"));
    assert!(!service.is_direct_child("/Documents/file.pdf", ""));
    
    // Test non-matching paths
    assert!(!service.is_direct_child("/Other/file.pdf", "/Documents"));
    assert!(!service.is_direct_child("/Documenting/file.pdf", "/Documents")); // prefix but not child
}

#[tokio::test]
async fn test_track_subdirectories_recursively_structure() {
    // This test verifies the directory extraction logic without database operations
    let files = mock_nested_directory_files();
    
    // Extract directories that should be tracked
    let mut expected_directories = std::collections::BTreeSet::new();
    expected_directories.insert("/Documents".to_string());
    expected_directories.insert("/Documents/2024".to_string());
    expected_directories.insert("/Documents/Archive".to_string());
    expected_directories.insert("/Documents/2024/Q1".to_string());
    expected_directories.insert("/Documents/2024/Q2".to_string());
    expected_directories.insert("/Documents/2024/Q1/Reports".to_string());
    
    // This tests the directory extraction logic that happens in track_subdirectories_recursively
    let mut all_directories = std::collections::BTreeSet::new();
    
    for file in &files {
        if file.is_directory {
            all_directories.insert(file.path.clone());
        } else {
            // Extract all parent directories from file paths
            let mut path_parts: Vec<&str> = file.path.split('/').collect();
            path_parts.pop(); // Remove the filename
            
            // Build directory paths from root down to immediate parent
            let mut current_path = String::new();
            for part in path_parts {
                if !part.is_empty() {
                    if !current_path.is_empty() {
                        current_path.push('/');
                    } else {
                        // Start with leading slash for absolute paths
                        current_path.push('/');
                    }
                    current_path.push_str(part);
                    all_directories.insert(current_path.clone());
                }
            }
        }
    }
    
    assert_eq!(all_directories, expected_directories);
}

#[tokio::test]
async fn test_direct_file_counting() {
    let service = create_test_webdav_service();
    let files = mock_nested_directory_files();
    
    // Test counting direct files in root directory
    let direct_files_root: Vec<_> = files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/Documents"))
        .collect();
    assert_eq!(direct_files_root.len(), 1); // Only root-file.pdf
    assert_eq!(direct_files_root[0].name, "root-file.pdf");
    
    // Test counting direct files in /Documents/2024
    let direct_files_2024: Vec<_> = files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/Documents/2024"))
        .collect();
    assert_eq!(direct_files_2024.len(), 1); // Only annual-report.pdf
    assert_eq!(direct_files_2024[0].name, "annual-report.pdf");
    
    // Test counting direct files in /Documents/2024/Q1
    let direct_files_q1: Vec<_> = files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/Documents/2024/Q1"))
        .collect();
    assert_eq!(direct_files_q1.len(), 1); // Only q1-summary.pdf
    assert_eq!(direct_files_q1[0].name, "q1-summary.pdf");
    
    // Test counting direct files in deep directory
    let direct_files_reports: Vec<_> = files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/Documents/2024/Q1/Reports"))
        .collect();
    assert_eq!(direct_files_reports.len(), 1); // Only detailed-report.pdf
    assert_eq!(direct_files_reports[0].name, "detailed-report.pdf");
    
    // Test empty directory
    let direct_files_q2: Vec<_> = files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/Documents/2024/Q2"))
        .collect();
    assert_eq!(direct_files_q2.len(), 0); // No direct files in Q2
}

#[tokio::test]
async fn test_direct_subdirectory_counting() {
    let service = create_test_webdav_service();
    let files = mock_nested_directory_files();
    
    // Test counting direct subdirectories in root
    let direct_subdirs_root: Vec<_> = files.iter()
        .filter(|f| f.is_directory && service.is_direct_child(&f.path, "/Documents"))
        .collect();
    assert_eq!(direct_subdirs_root.len(), 2); // 2024 and Archive
    
    // Test counting direct subdirectories in /Documents/2024
    let direct_subdirs_2024: Vec<_> = files.iter()
        .filter(|f| f.is_directory && service.is_direct_child(&f.path, "/Documents/2024"))
        .collect();
    assert_eq!(direct_subdirs_2024.len(), 2); // Q1 and Q2
    
    // Test counting direct subdirectories in /Documents/2024/Q1
    let direct_subdirs_q1: Vec<_> = files.iter()
        .filter(|f| f.is_directory && service.is_direct_child(&f.path, "/Documents/2024/Q1"))
        .collect();
    assert_eq!(direct_subdirs_q1.len(), 1); // Reports
    
    // Test leaf directory (no subdirectories)
    let direct_subdirs_reports: Vec<_> = files.iter()
        .filter(|f| f.is_directory && service.is_direct_child(&f.path, "/Documents/2024/Q1/Reports"))
        .collect();
    assert_eq!(direct_subdirs_reports.len(), 0); // No subdirectories in Reports
}

#[tokio::test]
async fn test_size_calculation_per_directory() {
    let service = create_test_webdav_service();
    let files = mock_nested_directory_files();
    
    // Calculate total size for each directory's direct files
    let root_size: i64 = files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/Documents"))
        .map(|f| f.size)
        .sum();
    assert_eq!(root_size, 1024000); // root-file.pdf
    
    let q1_size: i64 = files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/Documents/2024/Q1"))
        .map(|f| f.size)
        .sum();
    assert_eq!(q1_size, 512000); // q1-summary.pdf
    
    let reports_size: i64 = files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/Documents/2024/Q1/Reports"))
        .map(|f| f.size)
        .sum();
    assert_eq!(reports_size, 4096000); // detailed-report.pdf
    
    let archive_size: i64 = files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/Documents/Archive"))
        .map(|f| f.size)
        .sum();
    assert_eq!(archive_size, 256000); // old-document.pdf
}

#[tokio::test]
async fn test_edge_cases() {
    let service = create_test_webdav_service();
    
    // Test empty paths
    assert!(!service.is_direct_child("", "/Documents"));
    assert!(service.is_direct_child("/Documents", ""));
    
    // Test identical paths
    assert!(!service.is_direct_child("/Documents", "/Documents"));
    
    // Test path with trailing slashes
    assert!(service.is_direct_child("/Documents/file.pdf", "/Documents/"));
    
    // Test paths that are prefix but not parent
    assert!(!service.is_direct_child("/DocumentsBackup/file.pdf", "/Documents"));
    
    // Test deeply nested paths
    let deep_path = "/Documents/a/b/c/d/e/f/g/h/i/j/file.pdf";
    assert!(!service.is_direct_child(deep_path, "/Documents"));
    assert!(!service.is_direct_child(deep_path, "/Documents/a"));
    assert!(service.is_direct_child(deep_path, "/Documents/a/b/c/d/e/f/g/h/i/j"));
}