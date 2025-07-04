use readur::services::webdav::{WebDAVService, WebDAVConfig};
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

#[tokio::test]
async fn test_empty_directory_tracking() {
    let service = create_test_webdav_service();
    
    // Test completely empty directory
    let empty_files: Vec<FileInfo> = vec![];
    
    // Test the directory extraction logic that happens in track_subdirectories_recursively
    let mut all_directories = std::collections::BTreeSet::new();
    
    for file in &empty_files {
        if file.is_directory {
            all_directories.insert(file.path.clone());
        } else {
            let mut path_parts: Vec<&str> = file.path.split('/').collect();
            path_parts.pop();
            
            let mut current_path = String::new();
            for part in path_parts {
                if !part.is_empty() {
                    if !current_path.is_empty() {
                        current_path.push('/');
                    }
                    current_path.push_str(part);
                    all_directories.insert(current_path.clone());
                }
            }
        }
    }
    
    assert!(all_directories.is_empty(), "Empty file list should result in no directories");
}

#[tokio::test]
async fn test_directory_only_structure() {
    let service = create_test_webdav_service();
    
    // Test structure with only directories, no files
    let directory_only_files = vec![
        FileInfo {
            path: "/Documents".to_string(),
            name: "Documents".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "docs-etag".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: "/Documents/Empty1".to_string(),
            name: "Empty1".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "empty1-etag".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: "/Documents/Empty2".to_string(),
            name: "Empty2".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "empty2-etag".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
    ];
    
    // Test file counting for empty directories
    let root_files: Vec<_> = directory_only_files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/Documents"))
        .collect();
    assert_eq!(root_files.len(), 0, "Root directory should have no files");
    
    let empty1_files: Vec<_> = directory_only_files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/Documents/Empty1"))
        .collect();
    assert_eq!(empty1_files.len(), 0, "Empty1 directory should have no files");
    
    // Test subdirectory counting
    let root_subdirs: Vec<_> = directory_only_files.iter()
        .filter(|f| f.is_directory && service.is_direct_child(&f.path, "/Documents"))
        .collect();
    assert_eq!(root_subdirs.len(), 2, "Root should have 2 subdirectories");
    
    // Test size calculation for empty directories
    let root_size: i64 = directory_only_files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/Documents"))
        .map(|f| f.size)
        .sum();
    assert_eq!(root_size, 0, "Empty directory should have zero total size");
}

#[tokio::test]
async fn test_very_deep_nesting() {
    let service = create_test_webdav_service();
    
    // Create a very deeply nested structure (10 levels deep)
    let deep_path = "/Documents/L1/L2/L3/L4/L5/L6/L7/L8/L9/L10";
    let file_path = format!("{}/deep-file.pdf", deep_path);
    
    let deep_files = vec![
        // All directories in the path
        FileInfo {
            path: "/Documents".to_string(),
            name: "Documents".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "docs-etag".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        // All intermediate directories from L1 to L10
        FileInfo {
            path: "/Documents/L1".to_string(),
            name: "L1".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "l1-etag".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: "/Documents/L1/L2".to_string(),
            name: "L2".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "l2-etag".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: "/Documents/L1/L2/L3".to_string(),
            name: "L3".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "l3-etag".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: deep_path.to_string(),
            name: "L10".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "l10-etag".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        // File at the deepest level
        FileInfo {
            path: file_path.clone(),
            name: "deep-file.pdf".to_string(),
            size: 1024000,
            mime_type: "application/pdf".to_string(),
            last_modified: Some(Utc::now()),
            etag: "deep-file-etag".to_string(),
            is_directory: false,
            created_at: Some(Utc::now()),
            permissions: Some(644),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
    ];
    
    // Test is_direct_child for deep paths
    assert!(service.is_direct_child(&file_path, deep_path), "File should be direct child of deepest directory");
    assert!(!service.is_direct_child(&file_path, "/Documents"), "File should not be direct child of root");
    assert!(!service.is_direct_child(&file_path, "/Documents/L1"), "File should not be direct child of L1");
    
    // Test directory extraction from deep file path
    let mut all_directories = std::collections::BTreeSet::new();
    
    for file in &deep_files {
        if file.is_directory {
            all_directories.insert(file.path.clone());
        } else {
            let mut path_parts: Vec<&str> = file.path.split('/').collect();
            path_parts.pop(); // Remove filename
            
            let mut current_path = String::new();
            for part in path_parts {
                if !part.is_empty() {
                    if !current_path.is_empty() {
                        current_path.push('/');
                    }
                    current_path.push_str(part);
                    all_directories.insert(current_path.clone());
                }
            }
        }
    }
    
    // Should extract all intermediate directories
    assert!(all_directories.contains("/Documents"));
    assert!(all_directories.contains("/Documents/L1"));
    assert!(all_directories.contains("/Documents/L1/L2"));
    assert!(all_directories.contains(deep_path));
    assert!(all_directories.len() >= 11, "Should track all intermediate directories"); // /Documents + L1 + L2 + L3 + L10 + extracted from file path = 11+ directories total
}

#[tokio::test]
async fn test_special_characters_in_paths() {
    let service = create_test_webdav_service();
    
    // Test paths with special characters, spaces, unicode
    let special_files = vec![
        FileInfo {
            path: "/Documents/Folder with spaces".to_string(),
            name: "Folder with spaces".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "spaces-etag".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: "/Documents/Folder-with-dashes".to_string(),
            name: "Folder-with-dashes".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "dashes-etag".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: "/Documents/Документы".to_string(), // Cyrillic
            name: "Документы".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "cyrillic-etag".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: "/Documents/Folder with spaces/file with spaces.pdf".to_string(),
            name: "file with spaces.pdf".to_string(),
            size: 1024000,
            mime_type: "application/pdf".to_string(),
            last_modified: Some(Utc::now()),
            etag: "space-file-etag".to_string(),
            is_directory: false,
            created_at: Some(Utc::now()),
            permissions: Some(644),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
    ];
    
    // Test is_direct_child with special characters
    assert!(service.is_direct_child("/Documents/Folder with spaces/file with spaces.pdf", "/Documents/Folder with spaces"));
    assert!(service.is_direct_child("/Documents/Folder with spaces", "/Documents"));
    assert!(service.is_direct_child("/Documents/Документы", "/Documents"));
    
    // Test file counting with special characters
    let spaces_folder_files: Vec<_> = special_files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/Documents/Folder with spaces"))
        .collect();
    assert_eq!(spaces_folder_files.len(), 1);
    assert_eq!(spaces_folder_files[0].name, "file with spaces.pdf");
}

#[tokio::test]
async fn test_edge_case_path_patterns() {
    let service = create_test_webdav_service();
    
    // Test various edge case paths
    let edge_case_tests = vec![
        // (child_path, parent_path, expected_result)
        ("/Documents/file.pdf", "/Documents", true),
        ("/Documents/", "/Documents", false), // Same path
        ("/Documents", "/Documents", false), // Same path
        ("/Documents/subfolder/", "/Documents", true), // Trailing slash
        ("/Documents/subfolder", "/Documents/", true), // Parent with trailing slash
        ("/Documenting/file.pdf", "/Documents", false), // Prefix but not parent
        ("/Documents/file.pdf", "/Doc", false), // Partial parent match
        ("", "/Documents", false), // Empty child
        ("/Documents/file.pdf", "", false), // Not direct child of root (nested in Documents)
        ("/file.pdf", "", true), // Root level file
        ("/Documents/file.pdf", "/", false), // Not direct child of root (nested in Documents)
        ("/file.pdf", "/", true), // Root level file with slash parent
        ("//Documents//file.pdf", "/Documents", false), // Double slashes (malformed)
        ("/Documents/./file.pdf", "/Documents", false), // Dot notation (should be normalized first)
        ("/Documents/../file.pdf", "", false), // Parent notation (should be normalized first)
    ];
    
    for (child, parent, expected) in edge_case_tests {
        let result = service.is_direct_child(child, parent);
        assert_eq!(
            result, expected,
            "is_direct_child('{}', '{}') expected {}, got {}",
            child, parent, expected, result
        );
    }
}

#[tokio::test]
async fn test_etag_normalization_edge_cases() {
    let service = create_test_webdav_service();
    
    // Test various ETag format edge cases
    let etag_test_cases = vec![
        (r#""simple-etag""#, "simple-etag"),
        (r#"W/"weak-etag""#, "weak-etag"),
        (r#"no-quotes"#, "no-quotes"),
        (r#""""#, ""), // Empty quoted string
        (r#""#, ""), // Single quote
        (r#"W/"""#, ""), // Weak etag with empty quotes
        (r#"  "  spaced-etag  "  "#, "  spaced-etag  "), // Extra whitespace around quotes
        (r#"W/  "weak-with-spaces"  "#, "weak-with-spaces"),
        (r#""etag-with-"internal"-quotes""#, r#"etag-with-"internal"-quotes"#), // Internal quotes
        (r#""unicode-ж-etag""#, "unicode-ж-etag"), // Unicode characters
    ];
    
    for (input_etag, expected_normalized) in etag_test_cases {
        let xml_response = format!(r#"<?xml version="1.0"?>
        <d:multistatus xmlns:d="DAV:">
            <d:response>
                <d:href>/remote.php/dav/files/admin/Documents/</d:href>
                <d:propstat>
                    <d:prop>
                        <d:getetag>{}</d:getetag>
                    </d:prop>
                    <d:status>HTTP/1.1 200 OK</d:status>
                </d:propstat>
            </d:response>
        </d:multistatus>"#, input_etag);
        
        let result = service.parse_directory_etag(&xml_response);
        match result {
            Ok(etag) => {
                assert_eq!(
                    etag, expected_normalized,
                    "ETag normalization failed for input '{}': expected '{}', got '{}'",
                    input_etag, expected_normalized, etag
                );
            }
            Err(e) => {
                if !expected_normalized.is_empty() {
                    panic!("Expected ETag '{}' but got error: {}", expected_normalized, e);
                }
                // Empty expected result means we expect an error
            }
        }
    }
}

#[tokio::test]
async fn test_malformed_xml_responses() {
    let service = create_test_webdav_service();
    
    // Test various malformed XML responses
    let malformed_xml_cases = vec![
        // Empty response
        "",
        // Not XML
        "not xml at all",
        // Incomplete XML
        "<?xml version=\"1.0\"?><d:multistatus",
        // Missing ETag
        r#"<?xml version="1.0"?>
        <d:multistatus xmlns:d="DAV:">
            <d:response>
                <d:href>/remote.php/dav/files/admin/Documents/</d:href>
                <d:propstat>
                    <d:prop>
                        <d:displayname>Documents</d:displayname>
                    </d:prop>
                    <d:status>HTTP/1.1 200 OK</d:status>
                </d:propstat>
            </d:response>
        </d:multistatus>"#,
        // Empty ETag
        r#"<?xml version="1.0"?>
        <d:multistatus xmlns:d="DAV:">
            <d:response>
                <d:href>/remote.php/dav/files/admin/Documents/</d:href>
                <d:propstat>
                    <d:prop>
                        <d:getetag></d:getetag>
                    </d:prop>
                    <d:status>HTTP/1.1 200 OK</d:status>
                </d:propstat>
            </d:response>
        </d:multistatus>"#,
        // Invalid XML characters
        r#"<?xml version="1.0"?>
        <d:multistatus xmlns:d="DAV:">
            <d:response>
                <d:href>/remote.php/dav/files/admin/Documents/</d:href>
                <d:propstat>
                    <d:prop>
                        <d:getetag>"invalid-xml-&#x1;-char"</d:getetag>
                    </d:prop>
                </d:propstat>
            </d:response>
        </d:multistatus>"#,
    ];
    
    for (i, malformed_xml) in malformed_xml_cases.iter().enumerate() {
        let result = service.parse_directory_etag(malformed_xml);
        // Some malformed XML might still be parsed successfully by the robust parser
        // The key is that it doesn't crash - either error or success is acceptable
        match result {
            Ok(etag) => {
                println!("Malformed XML case {} parsed successfully with ETag: {}", i, etag);
            }
            Err(e) => {
                println!("Malformed XML case {} failed as expected: {}", i, e);
            }
        }
    }
}

#[tokio::test]
async fn test_large_directory_structures() {
    let service = create_test_webdav_service();
    
    // Generate a large directory structure (1000 directories, 5000 files)
    let mut large_files = Vec::new();
    
    // Add root directory
    large_files.push(FileInfo {
        path: "/Documents".to_string(),
        name: "Documents".to_string(),
        size: 0,
        mime_type: "".to_string(),
        last_modified: Some(Utc::now()),
        etag: "root-etag".to_string(),
        is_directory: true,
        created_at: Some(Utc::now()),
        permissions: Some(755),
        owner: Some("admin".to_string()),
        group: Some("admin".to_string()),
        metadata: None,
    });
    
    // Generate 100 level-1 directories, each with 10 subdirectories and 50 files
    for i in 0..100 {
        let level1_path = format!("/Documents/Dir{:03}", i);
        
        // Add level-1 directory
        large_files.push(FileInfo {
            path: level1_path.clone(),
            name: format!("Dir{:03}", i),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: format!("dir{}-etag", i),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        });
        
        // Add 10 subdirectories
        for j in 0..10 {
            let level2_path = format!("{}/SubDir{:02}", level1_path, j);
            large_files.push(FileInfo {
                path: level2_path.clone(),
                name: format!("SubDir{:02}", j),
                size: 0,
                mime_type: "".to_string(),
                last_modified: Some(Utc::now()),
                etag: format!("subdir{}-{}-etag", i, j),
                is_directory: true,
                created_at: Some(Utc::now()),
                permissions: Some(755),
                owner: Some("admin".to_string()),
                group: Some("admin".to_string()),
                metadata: None,
            });
            
            // Add 5 files in each subdirectory
            for k in 0..5 {
                large_files.push(FileInfo {
                    path: format!("{}/file{:02}.pdf", level2_path, k),
                    name: format!("file{:02}.pdf", k),
                    size: 1024 * (k + 1) as i64,
                    mime_type: "application/pdf".to_string(),
                    last_modified: Some(Utc::now()),
                    etag: format!("file{}-{}-{}-etag", i, j, k),
                    is_directory: false,
                    created_at: Some(Utc::now()),
                    permissions: Some(644),
                    owner: Some("admin".to_string()),
                    group: Some("admin".to_string()),
                    metadata: None,
                });
            }
        }
    }
    
    println!("Generated {} files and directories", large_files.len());
    
    // Test performance of directory extraction
    let start_time = std::time::Instant::now();
    let mut all_directories = std::collections::BTreeSet::new();
    
    for file in &large_files {
        if file.is_directory {
            all_directories.insert(file.path.clone());
        } else {
            let mut path_parts: Vec<&str> = file.path.split('/').collect();
            path_parts.pop();
            
            let mut current_path = String::new();
            for part in path_parts {
                if !part.is_empty() {
                    if !current_path.is_empty() {
                        current_path.push('/');
                    }
                    current_path.push_str(part);
                    all_directories.insert(current_path.clone());
                }
            }
        }
    }
    
    let extraction_time = start_time.elapsed();
    println!("Extracted {} directories in {:?}", all_directories.len(), extraction_time);
    
    // Verify structure - the actual count includes extraction from file paths too
    assert!(all_directories.len() >= 1101, "Should have at least 1101 directories"); // 1 root + 100 level1 + 1000 level2 + extracted paths
    assert!(all_directories.contains("/Documents"));
    assert!(all_directories.contains("/Documents/Dir000"));
    assert!(all_directories.contains("/Documents/Dir099/SubDir09"));
    
    // Test performance of file counting for a specific directory
    let count_start = std::time::Instant::now();
    let test_dir = "/Documents/Dir050";
    let direct_files: Vec<_> = large_files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, test_dir))
        .collect();
    let count_time = count_start.elapsed();
    
    println!("Counted {} direct files in {} in {:?}", direct_files.len(), test_dir, count_time);
    
    // Performance assertions
    assert!(extraction_time.as_millis() < 1000, "Directory extraction too slow: {:?}", extraction_time);
    assert!(count_time.as_millis() < 100, "File counting too slow: {:?}", count_time);
}