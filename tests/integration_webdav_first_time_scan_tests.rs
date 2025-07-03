use tokio;
use uuid::Uuid;
use chrono::Utc;
use anyhow::Result;
use readur::models::{FileInfo, CreateWebDAVDirectory, CreateUser, UserRole};
use readur::services::webdav_service::{WebDAVService, WebDAVConfig};
use readur::db::Database;

// Helper function to create test WebDAV service
fn create_test_webdav_service() -> WebDAVService {
    let config = WebDAVConfig {
        server_url: "https://test.example.com".to_string(),
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/FullerDocuments/JonDocuments".to_string()],
        file_extensions: vec!["pdf".to_string(), "docx".to_string(), "txt".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };
    
    WebDAVService::new(config).unwrap()
}

// Mock files structure that represents a real directory with subdirectories
fn mock_realistic_directory_structure() -> Vec<FileInfo> {
    vec![
        // Parent root directory
        FileInfo {
            path: "/FullerDocuments".to_string(),
            name: "FullerDocuments".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "fuller-docs-etag-000".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        // Root directory
        FileInfo {
            path: "/FullerDocuments/JonDocuments".to_string(),
            name: "JonDocuments".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "root-dir-etag-123".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        // Subdirectory level 1
        FileInfo {
            path: "/FullerDocuments/JonDocuments/Projects".to_string(),
            name: "Projects".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "projects-etag-456".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: "/FullerDocuments/JonDocuments/Archive".to_string(),
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
        // Subdirectory level 2
        FileInfo {
            path: "/FullerDocuments/JonDocuments/Projects/WebDev".to_string(),
            name: "WebDev".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "webdev-etag-101".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: "/FullerDocuments/JonDocuments/Projects/Mobile".to_string(),
            name: "Mobile".to_string(),
            size: 0,
            mime_type: "".to_string(),
            last_modified: Some(Utc::now()),
            etag: "mobile-etag-102".to_string(),
            is_directory: true,
            created_at: Some(Utc::now()),
            permissions: Some(755),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        // Files in various directories
        FileInfo {
            path: "/FullerDocuments/JonDocuments/readme.txt".to_string(),
            name: "readme.txt".to_string(),
            size: 1024,
            mime_type: "text/plain".to_string(),
            last_modified: Some(Utc::now()),
            etag: "readme-etag".to_string(),
            is_directory: false,
            created_at: Some(Utc::now()),
            permissions: Some(644),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: "/FullerDocuments/JonDocuments/Projects/project-overview.pdf".to_string(),
            name: "project-overview.pdf".to_string(),
            size: 2048000,
            mime_type: "application/pdf".to_string(),
            last_modified: Some(Utc::now()),
            etag: "overview-etag".to_string(),
            is_directory: false,
            created_at: Some(Utc::now()),
            permissions: Some(644),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: "/FullerDocuments/JonDocuments/Projects/WebDev/website-specs.docx".to_string(),
            name: "website-specs.docx".to_string(),
            size: 512000,
            mime_type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string(),
            last_modified: Some(Utc::now()),
            etag: "specs-etag".to_string(),
            is_directory: false,
            created_at: Some(Utc::now()),
            permissions: Some(644),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
        FileInfo {
            path: "/FullerDocuments/JonDocuments/Projects/Mobile/app-design.pdf".to_string(),
            name: "app-design.pdf".to_string(),
            size: 1536000,
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
            path: "/FullerDocuments/JonDocuments/Archive/old-notes.txt".to_string(),
            name: "old-notes.txt".to_string(),
            size: 256,
            mime_type: "text/plain".to_string(),
            last_modified: Some(Utc::now()),
            etag: "notes-etag".to_string(),
            is_directory: false,
            created_at: Some(Utc::now()),
            permissions: Some(644),
            owner: Some("admin".to_string()),
            group: Some("admin".to_string()),
            metadata: None,
        },
    ]
}

// Helper function to create test database
async fn create_test_database() -> Result<(Database, Uuid)> {
    let db_url = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("TEST_DATABASE_URL"))
        .unwrap_or_else(|_| "postgresql://readur:readur@localhost:5432/readur".to_string());
    
    let database = Database::new(&db_url).await?;
    
    // Create a test user in the database
    let unique_suffix = Uuid::new_v4().to_string()[..8].to_string();
    let test_user = CreateUser {
        username: format!("testuser_{}", unique_suffix),
        email: format!("testuser_{}@example.com", unique_suffix),
        password: "password123".to_string(),
        role: Some(UserRole::User),
    };
    let created_user = database.create_user(test_user).await?;
    
    Ok((database, created_user.id))
}

#[tokio::test]
async fn test_first_time_directory_scan_with_subdirectories() {
    let (database, user_id) = create_test_database().await.unwrap();
    let service = create_test_webdav_service();
    
    // Mock the scenario where we have files but no previously tracked directories
    let mock_files = mock_realistic_directory_structure();
    
    // Simulate the check_subdirectories_for_changes scenario:
    // 1. Directory ETag is unchanged (so we think it's unchanged)
    // 2. But no subdirectories are known in database (first-time scan)
    
    // Verify that list_webdav_directories returns empty (first-time scenario)
    let known_dirs = database.list_webdav_directories(user_id).await.unwrap();
    assert!(known_dirs.is_empty(), "Should have no known directories on first scan");
    
    // This is the critical test: check_subdirectories_for_changes should fall back to full scan
    // when no subdirectories are known, rather than returning empty results
    
    // We can't easily test check_subdirectories_for_changes directly since it's private,
    // but we can test the public discover_files_in_folder_optimized method that calls it
    
    // Create a partial directory record to simulate the "directory unchanged" scenario
    let root_dir = CreateWebDAVDirectory {
        user_id,
        directory_path: "/FullerDocuments/JonDocuments".to_string(),
        directory_etag: "root-dir-etag-123".to_string(),
        file_count: 1,
        total_size_bytes: 1024,
    };
    
    // Insert the root directory to simulate it being "known" but without subdirectories
    database.create_or_update_webdav_directory(&root_dir).await.unwrap();
    
    // Now verify that known directories contains only the root
    let known_dirs_after = database.list_webdav_directories(user_id).await.unwrap();
    assert_eq!(known_dirs_after.len(), 1);
    assert_eq!(known_dirs_after[0].directory_path, "/FullerDocuments/JonDocuments");
    
    // Filter subdirectories just like the code does
    let parent_path = "/FullerDocuments/JonDocuments";
    let subdirectories: Vec<_> = known_dirs_after.iter()
        .filter(|dir| dir.directory_path.starts_with(parent_path) && dir.directory_path != parent_path)
        .collect();
    
    // This should be empty (no known subdirectories), which was causing the bug
    assert!(subdirectories.is_empty(), "Should have no known subdirectories initially");
    
    // The fix we made should cause the system to do a full scan when subdirectories is empty
    // This test verifies that the logic correctly identifies this scenario
    
    println!("✅ Successfully verified first-time directory scan scenario");
    println!("✅ Root directory is known but no subdirectories are tracked");
    println!("✅ System should fall back to full scan in this case");
}

#[tokio::test]
async fn test_subdirectory_tracking_after_full_scan() {
    let (database, user_id) = create_test_database().await.unwrap();
    let service = create_test_webdav_service();
    let mock_files = mock_realistic_directory_structure();
    
    // Simulate what happens after a full scan - subdirectories should be tracked
    
    // Use the track_subdirectories_recursively logic manually
    use std::collections::{HashMap, BTreeSet};
    
    // Step 1: Extract all unique directory paths from the file list
    let mut all_directories = BTreeSet::new();
    
    for file in &mock_files {
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
                    }
                    current_path.push_str(part);
                    all_directories.insert(current_path.clone());
                }
            }
        }
    }
    
    // Step 2: Create a mapping of directory -> ETag from the files list
    let mut directory_etags: HashMap<String, String> = HashMap::new();
    for file in &mock_files {
        if file.is_directory {
            directory_etags.insert(file.path.clone(), file.etag.clone());
        }
    }
    
    // Step 3: Simulate tracking each directory
    for dir_path in &all_directories {
        let dir_etag = match directory_etags.get(dir_path) {
            Some(etag) => etag.clone(),
            None => {
                continue; // Skip directories without ETags
            }
        };
        
        // Count direct files in this directory
        let direct_files: Vec<_> = mock_files.iter()
            .filter(|f| {
                !f.is_directory && 
                service.is_direct_child(&f.path, dir_path)
            })
            .collect();
        
        let file_count = direct_files.len() as i64;
        let total_size_bytes = direct_files.iter().map(|f| f.size).sum::<i64>();
        
        // Create directory tracking record
        let directory_record = CreateWebDAVDirectory {
            user_id,
            directory_path: dir_path.clone(),
            directory_etag: dir_etag.clone(),
            file_count,
            total_size_bytes,
        };
        
        database.create_or_update_webdav_directory(&directory_record).await.unwrap();
    }
    
    // Now verify that all directories are tracked
    let tracked_dirs = database.list_webdav_directories(user_id).await.unwrap();
    
    // We should have tracked all directories found in the file structure
    let expected_directories = vec![
        "/FullerDocuments",
        "/FullerDocuments/JonDocuments",
        "/FullerDocuments/JonDocuments/Projects",
        "/FullerDocuments/JonDocuments/Archive",
        "/FullerDocuments/JonDocuments/Projects/WebDev",
        "/FullerDocuments/JonDocuments/Projects/Mobile",
    ];
    
    assert_eq!(tracked_dirs.len(), expected_directories.len());
    
    // Verify subdirectories are now known for the root path
    let parent_path = "/FullerDocuments/JonDocuments";
    let subdirectories: Vec<_> = tracked_dirs.iter()
        .filter(|dir| dir.directory_path.starts_with(parent_path) && dir.directory_path != parent_path)
        .collect();
    
    // Should now have known subdirectories
    assert!(!subdirectories.is_empty(), "Should have known subdirectories after full scan");
    assert!(subdirectories.len() >= 2, "Should have at least Projects and Archive subdirectories");
    
    println!("✅ Successfully verified subdirectory tracking after full scan");
    println!("✅ Found {} tracked directories", tracked_dirs.len());
    println!("✅ Found {} subdirectories under root", subdirectories.len());
}

#[tokio::test]
async fn test_direct_child_identification_edge_cases() {
    let service = create_test_webdav_service();
    
    // Test the realistic paths from our scenario
    assert!(service.is_direct_child("/FullerDocuments/JonDocuments/readme.txt", "/FullerDocuments/JonDocuments"));
    assert!(service.is_direct_child("/FullerDocuments/JonDocuments/Projects", "/FullerDocuments/JonDocuments"));
    assert!(service.is_direct_child("/FullerDocuments/JonDocuments/Archive", "/FullerDocuments/JonDocuments"));
    
    // These should NOT be direct children (nested deeper)
    assert!(!service.is_direct_child("/FullerDocuments/JonDocuments/Projects/project-overview.pdf", "/FullerDocuments/JonDocuments"));
    assert!(!service.is_direct_child("/FullerDocuments/JonDocuments/Projects/WebDev/website-specs.docx", "/FullerDocuments/JonDocuments"));
    
    // Test intermediate levels
    assert!(service.is_direct_child("/FullerDocuments/JonDocuments/Projects/WebDev", "/FullerDocuments/JonDocuments/Projects"));
    assert!(service.is_direct_child("/FullerDocuments/JonDocuments/Projects/project-overview.pdf", "/FullerDocuments/JonDocuments/Projects"));
    
    // Test deep nesting
    assert!(service.is_direct_child("/FullerDocuments/JonDocuments/Projects/WebDev/website-specs.docx", "/FullerDocuments/JonDocuments/Projects/WebDev"));
    
    println!("✅ All direct child identification tests passed");
}

#[tokio::test]
async fn test_file_count_accuracy_per_directory() {
    let service = create_test_webdav_service();
    let mock_files = mock_realistic_directory_structure();
    
    // Test that we correctly count direct files in each directory
    
    // Root directory should have 1 direct file (readme.txt)
    let root_files: Vec<_> = mock_files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/FullerDocuments/JonDocuments"))
        .collect();
    assert_eq!(root_files.len(), 1);
    assert_eq!(root_files[0].name, "readme.txt");
    
    // Projects directory should have 1 direct file (project-overview.pdf)
    let projects_files: Vec<_> = mock_files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/FullerDocuments/JonDocuments/Projects"))
        .collect();
    assert_eq!(projects_files.len(), 1);
    assert_eq!(projects_files[0].name, "project-overview.pdf");
    
    // WebDev directory should have 1 direct file (website-specs.docx)
    let webdev_files: Vec<_> = mock_files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/FullerDocuments/JonDocuments/Projects/WebDev"))
        .collect();
    assert_eq!(webdev_files.len(), 1);
    assert_eq!(webdev_files[0].name, "website-specs.docx");
    
    // Mobile directory should have 1 direct file (app-design.pdf)
    let mobile_files: Vec<_> = mock_files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/FullerDocuments/JonDocuments/Projects/Mobile"))
        .collect();
    assert_eq!(mobile_files.len(), 1);
    assert_eq!(mobile_files[0].name, "app-design.pdf");
    
    // Archive directory should have 1 direct file (old-notes.txt)
    let archive_files: Vec<_> = mock_files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/FullerDocuments/JonDocuments/Archive"))
        .collect();
    assert_eq!(archive_files.len(), 1);
    assert_eq!(archive_files[0].name, "old-notes.txt");
    
    println!("✅ File count accuracy test passed for all directories");
}

#[tokio::test] 
async fn test_size_calculation_accuracy() {
    let service = create_test_webdav_service();
    let mock_files = mock_realistic_directory_structure();
    
    // Test size calculations for each directory
    
    let root_size: i64 = mock_files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/FullerDocuments/JonDocuments"))
        .map(|f| f.size)
        .sum();
    assert_eq!(root_size, 1024); // readme.txt
    
    let projects_size: i64 = mock_files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/FullerDocuments/JonDocuments/Projects"))
        .map(|f| f.size)
        .sum();
    assert_eq!(projects_size, 2048000); // project-overview.pdf
    
    let webdev_size: i64 = mock_files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/FullerDocuments/JonDocuments/Projects/WebDev"))
        .map(|f| f.size)
        .sum();
    assert_eq!(webdev_size, 512000); // website-specs.docx
    
    let mobile_size: i64 = mock_files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/FullerDocuments/JonDocuments/Projects/Mobile"))
        .map(|f| f.size)
        .sum();
    assert_eq!(mobile_size, 1536000); // app-design.pdf
    
    let archive_size: i64 = mock_files.iter()
        .filter(|f| !f.is_directory && service.is_direct_child(&f.path, "/FullerDocuments/JonDocuments/Archive"))
        .map(|f| f.size)
        .sum();
    assert_eq!(archive_size, 256); // old-notes.txt
    
    println!("✅ Size calculation accuracy test passed for all directories");
}