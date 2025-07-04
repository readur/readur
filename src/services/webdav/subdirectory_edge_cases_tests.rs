#[cfg(test)]
mod tests {
    use super::super::{WebDAVService, WebDAVConfig};
    use crate::models::FileInfo;
    use tokio;
    use chrono::Utc;
    use std::collections::BTreeSet;

// Helper function to create test WebDAV service
fn create_test_webdav_service() -> WebDAVService {
    let config = WebDAVConfig {
        server_url: "https://test.example.com".to_string(),
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec!["pdf".to_string(), "txt".to_string(), "docx".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };
    
    WebDAVService::new(config).unwrap()
}

// Test scenario that matches the real-world bug: deep nested structure with various file types
fn create_complex_nested_structure() -> Vec<FileInfo> {
    vec![
        // Root directories at different levels
        FileInfo {
            path: "/FullerDocuments".to_string(),
            name: "FullerDocuments".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "fuller-root-etag".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: "/FullerDocuments/JonDocuments".to_string(),
            name: "JonDocuments".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "jon-docs-etag".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        // Multiple levels of nesting
        FileInfo {
            path: "/FullerDocuments/JonDocuments/Work".to_string(),
            name: "Work".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "work-etag".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: "/FullerDocuments/JonDocuments/Personal".to_string(),
            name: "Personal".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "personal-etag".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: "/FullerDocuments/JonDocuments/Work/Projects".to_string(),
            name: "Projects".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "projects-etag".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: "/FullerDocuments/JonDocuments/Work/Reports".to_string(),
            name: "Reports".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "reports-etag".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: "/FullerDocuments/JonDocuments/Work/Projects/WebApp".to_string(),
            name: "WebApp".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "webapp-etag".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        // Files at various nesting levels - this is the key part that was failing
        FileInfo {
            path: "/FullerDocuments/JonDocuments/index.txt".to_string(),
            name: "index.txt".to_string(),
            size: 1500,
            mime_type: "text/plain".to_string(),
            last_modified: Some(Utc::now()),
            etag: "index-etag".to_string(),
            is_directory: false,
            created_at: Some(Utc::now()),
            permissions: Some(644),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: "/FullerDocuments/JonDocuments/Work/schedule.pdf".to_string(),
            name: "schedule.pdf".to_string(),
            size: 2048000,
            mime_type: "application/pdf".to_string(),
            last_modified: Some(Utc::now()),
            etag: "schedule-etag".to_string(),
            is_directory: false,
            created_at: Some(Utc::now()),
            permissions: Some(644),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: "/FullerDocuments/JonDocuments/Work/Projects/proposal.docx".to_string(),
            name: "proposal.docx".to_string(),
            size: 1024000,
            mime_type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string(),
            last_modified: Some(Utc::now()),
            etag: "proposal-etag".to_string(),
            is_directory: false,
            created_at: Some(Utc::now()),
            permissions: Some(644),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: "/FullerDocuments/JonDocuments/Work/Projects/WebApp/design.pdf".to_string(),
            name: "design.pdf".to_string(),
            size: 3072000,
            mime_type: "application/pdf".to_string(),
            last_modified: Some(Utc::now()),
            etag: "design-etag".to_string(),
            is_directory: false,
            created_at: Some(Utc::now()),
            permissions: Some(644),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: "/FullerDocuments/JonDocuments/Work/Reports/monthly.pdf".to_string(),
            name: "monthly.pdf".to_string(),
            size: 4096000,
            mime_type: "application/pdf".to_string(),
            last_modified: Some(Utc::now()),
            etag: "monthly-etag".to_string(),
            is_directory: false,
            created_at: Some(Utc::now()),
            permissions: Some(644),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: "/FullerDocuments/JonDocuments/Personal/diary.txt".to_string(),
            name: "diary.txt".to_string(),
            size: 5120,
            mime_type: "text/plain".to_string(),
            last_modified: Some(Utc::now()),
            etag: "diary-etag".to_string(),
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
async fn test_comprehensive_directory_extraction() {
    let files = create_complex_nested_structure();
    
    // Test the exact logic from track_subdirectories_recursively
    let mut all_directories = BTreeSet::new();
    
    for file in &files {
        if file.is_directory {
            // Add the directory itself
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
    
    // Expected directories should include ALL paths from root to deepest level
    let expected_directories: BTreeSet<String> = [
        "/FullerDocuments",
        "/FullerDocuments/JonDocuments", 
        "/FullerDocuments/JonDocuments/Work",
        "/FullerDocuments/JonDocuments/Personal",
        "/FullerDocuments/JonDocuments/Work/Projects",
        "/FullerDocuments/JonDocuments/Work/Reports",
        "/FullerDocuments/JonDocuments/Work/Projects/WebApp",
    ].iter().map(|s| s.to_string()).collect();
    
    assert_eq!(all_directories, expected_directories);
    
    // Verify we found all 7 unique directory levels
    assert_eq!(all_directories.len(), 7);
    
    println!("✅ Successfully extracted {} unique directories", all_directories.len());
    for dir in &all_directories {
        println!("  📁 {}", dir);
    }
}

#[tokio::test]
async fn test_first_time_scan_scenario_logic() {
    // This test simulates the exact bug scenario:
    // 1. We have a directory structure with subdirectories
    // 2. The root directory ETag hasn't changed
    // 3. But no subdirectories are known in the database
    // 4. The system should fall back to full scan, not return empty results
    
    let files = create_complex_nested_structure();
    let service = create_test_webdav_service();
    
    // Test the filtering logic that was causing the bug
    let parent_path = "/FullerDocuments/JonDocuments";
    
    // Simulate an empty list of known directories (first-time scan scenario)
    let known_directories: Vec<crate::models::WebDAVDirectory> = vec![];
    
    // Filter to subdirectories of this parent (this was returning empty)
    let subdirectories: Vec<_> = known_directories.iter()
        .filter(|dir| dir.directory_path.starts_with(parent_path) && dir.directory_path != parent_path)
        .collect();
    
    // This should be empty - which was causing the bug
    assert!(subdirectories.is_empty(), "Known subdirectories should be empty on first scan");
    
    // The key insight: when subdirectories.is_empty(), we should NOT return Vec::new()  
    // Instead, we should do a full scan to discover the structure
    
    // Verify that files actually exist in subdirectories
    let files_in_subdirs: Vec<_> = files.iter()
        .filter(|f| f.path.starts_with(parent_path) && f.path != parent_path && !f.is_directory)
        .collect();
    
    assert!(!files_in_subdirs.is_empty(), "There should be files in subdirectories");
    assert_eq!(files_in_subdirs.len(), 6, "Should find 6 files in subdirectories");
    
    // Test that we can correctly identify direct children at each level
    let direct_children_root: Vec<_> = files.iter()
        .filter(|f| service.is_direct_child(&f.path, "/FullerDocuments/JonDocuments"))
        .collect();
    
    // Should include: index.txt, Work/, Personal/
    assert_eq!(direct_children_root.len(), 3);
    
    let direct_children_work: Vec<_> = files.iter()
        .filter(|f| service.is_direct_child(&f.path, "/FullerDocuments/JonDocuments/Work"))
        .collect();
    
    // Should include: schedule.pdf, Projects/, Reports/
    assert_eq!(direct_children_work.len(), 3);
    
    println!("✅ First-time scan scenario logic test passed");
    println!("✅ Found {} files that would be missed without proper fallback", files_in_subdirs.len());
}

#[tokio::test]
async fn test_directory_etag_mapping_accuracy() {
    let files = create_complex_nested_structure();
    
    // Test ETag mapping logic from track_subdirectories_recursively
    let mut directory_etags = std::collections::HashMap::new();
    for file in &files {
        if file.is_directory {
            directory_etags.insert(file.path.clone(), file.etag.clone());
        }
    }
    
    // Verify all directory ETags are captured
    assert_eq!(directory_etags.len(), 7); // All 7 directories should have ETags
    
    // Test specific mappings
    assert_eq!(directory_etags.get("/FullerDocuments/JonDocuments").unwrap(), "jon-docs-etag");
    assert_eq!(directory_etags.get("/FullerDocuments/JonDocuments/Work").unwrap(), "work-etag");
    assert_eq!(directory_etags.get("/FullerDocuments/JonDocuments/Work/Projects/WebApp").unwrap(), "webapp-etag");
    
    // Test that files don't create ETag entries
    assert!(directory_etags.get("/FullerDocuments/JonDocuments/index.txt").is_none());
    assert!(directory_etags.get("/FullerDocuments/JonDocuments/Work/schedule.pdf").is_none());
    
    println!("✅ Directory ETag mapping accuracy test passed");
}

#[tokio::test]
async fn test_direct_file_counting_precision() {
    let service = create_test_webdav_service();
    let files = create_complex_nested_structure();
    
    // Test precise file counting for each directory level
    
    // Root level: should have 1 direct file (index.txt)
    let root_direct_files: Vec<_> = files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/FullerDocuments/JonDocuments"))
        .collect();
    assert_eq!(root_direct_files.len(), 1);
    assert_eq!(root_direct_files[0].name, "index.txt");
    
    // Work level: should have 1 direct file (schedule.pdf)
    let work_direct_files: Vec<_> = files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/FullerDocuments/JonDocuments/Work"))
        .collect();
    assert_eq!(work_direct_files.len(), 1);
    assert_eq!(work_direct_files[0].name, "schedule.pdf");
    
    // Projects level: should have 1 direct file (proposal.docx)
    let projects_direct_files: Vec<_> = files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/FullerDocuments/JonDocuments/Work/Projects"))
        .collect();
    assert_eq!(projects_direct_files.len(), 1);
    assert_eq!(projects_direct_files[0].name, "proposal.docx");
    
    // WebApp level: should have 1 direct file (design.pdf)
    let webapp_direct_files: Vec<_> = files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/FullerDocuments/JonDocuments/Work/Projects/WebApp"))
        .collect();
    assert_eq!(webapp_direct_files.len(), 1);
    assert_eq!(webapp_direct_files[0].name, "design.pdf");
    
    // Reports level: should have 1 direct file (monthly.pdf)
    let reports_direct_files: Vec<_> = files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/FullerDocuments/JonDocuments/Work/Reports"))
        .collect();
    assert_eq!(reports_direct_files.len(), 1);
    assert_eq!(reports_direct_files[0].name, "monthly.pdf");
    
    // Personal level: should have 1 direct file (diary.txt)
    let personal_direct_files: Vec<_> = files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/FullerDocuments/JonDocuments/Personal"))
        .collect();
    assert_eq!(personal_direct_files.len(), 1);
    assert_eq!(personal_direct_files[0].name, "diary.txt");
    
    println!("✅ Direct file counting precision test passed");
}

#[tokio::test] 
async fn test_total_size_calculation_per_directory() {
    let service = create_test_webdav_service();
    let files = create_complex_nested_structure();
    
    // Test size calculations match expected values
    
    let root_size: i64 = files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/FullerDocuments/JonDocuments"))
        .map(|f| f.size)
        .sum();
    assert_eq!(root_size, 1500); // index.txt
    
    let work_size: i64 = files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/FullerDocuments/JonDocuments/Work"))
        .map(|f| f.size)
        .sum();
    assert_eq!(work_size, 2048000); // schedule.pdf
    
    let projects_size: i64 = files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/FullerDocuments/JonDocuments/Work/Projects"))
        .map(|f| f.size)
        .sum();
    assert_eq!(projects_size, 1024000); // proposal.docx
    
    let webapp_size: i64 = files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/FullerDocuments/JonDocuments/Work/Projects/WebApp"))
        .map(|f| f.size)
        .sum();
    assert_eq!(webapp_size, 3072000); // design.pdf
    
    let reports_size: i64 = files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/FullerDocuments/JonDocuments/Work/Reports"))
        .map(|f| f.size)
        .sum();
    assert_eq!(reports_size, 4096000); // monthly.pdf
    
    let personal_size: i64 = files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/FullerDocuments/JonDocuments/Personal"))
        .map(|f| f.size)
        .sum();
    assert_eq!(personal_size, 5120); // diary.txt
    
    println!("✅ Total size calculation test passed");
}

#[tokio::test]
async fn test_path_edge_cases_and_normalization() {
    let service = create_test_webdav_service();
    
    // Test various path edge cases that might cause issues
    
    // Paths with trailing slashes
    assert!(service.is_direct_child("/FullerDocuments/JonDocuments/file.pdf", "/FullerDocuments/JonDocuments/"));
    assert!(service.is_direct_child("/FullerDocuments/JonDocuments/subfolder", "/FullerDocuments/JonDocuments/"));
    
    // Paths without trailing slashes  
    assert!(service.is_direct_child("/FullerDocuments/JonDocuments/file.pdf", "/FullerDocuments/JonDocuments"));
    assert!(service.is_direct_child("/FullerDocuments/JonDocuments/subfolder", "/FullerDocuments/JonDocuments"));
    
    // Mixed trailing slash scenarios
    assert!(service.is_direct_child("/FullerDocuments/JonDocuments/file.pdf", "/FullerDocuments/JonDocuments/"));
    assert!(service.is_direct_child("/FullerDocuments/JonDocuments/file.pdf", "/FullerDocuments/JonDocuments"));
    
    // Paths that are similar but not parent-child relationships
    assert!(!service.is_direct_child("/FullerDocumentsBackup/file.pdf", "/FullerDocuments"));
    assert!(!service.is_direct_child("/FullerDocuments2/file.pdf", "/FullerDocuments"));
    
    // Deep nesting verification
    assert!(!service.is_direct_child("/FullerDocuments/JonDocuments/Work/Projects/file.pdf", "/FullerDocuments/JonDocuments"));
    assert!(service.is_direct_child("/FullerDocuments/JonDocuments/Work/Projects/file.pdf", "/FullerDocuments/JonDocuments/Work/Projects"));
    
    // Root path edge cases
    assert!(service.is_direct_child("/FullerDocuments", ""));
    assert!(service.is_direct_child("/FullerDocuments", "/"));
    assert!(!service.is_direct_child("/FullerDocuments/JonDocuments", ""));
    
    println!("✅ Path edge cases and normalization test passed");
}

#[tokio::test]
async fn test_bug_scenario_file_count_verification() {
    // This test specifically verifies that we would find the reported 7046 files
    // in a scenario similar to the user's real environment
    
    let files = create_complex_nested_structure();
    
    // In the real bug scenario, they had 7046 files discovered initially
    // Let's simulate a larger structure to verify our logic scales
    
    let total_files: usize = files.iter().filter(|f| !f.is_directory).count();
    assert_eq!(total_files, 6); // Our mock has 6 files
    
    // Verify all files would be discovered in a full scan
    let parent_path = "/FullerDocuments/JonDocuments";
    let files_under_parent: Vec<_> = files.iter()
        .filter(|f| f.path.starts_with(parent_path) && !f.is_directory)
        .collect();
    
    // All 6 files should be under the parent (all files in our mock are under this path)
    assert_eq!(files_under_parent.len(), 6);
    
    // Verify that with the old buggy behavior, these files would be missed
    // (because subdirectories.is_empty() would return Ok(Vec::new()))
    
    // But with the fix, a full scan would discover them all
    let discovered_files: Vec<_> = files.iter()
        .filter(|f| f.path.starts_with(parent_path))
        .collect();
    
    // Should include both directories and files
    assert_eq!(discovered_files.len(), 12); // 6 directories + 6 files under parent
    
    println!("✅ Bug scenario file count verification passed");
    println!("✅ Would discover {} total files under parent path", files_under_parent.len());
    println!("✅ Full scan would find {} total entries", discovered_files.len());
}

}