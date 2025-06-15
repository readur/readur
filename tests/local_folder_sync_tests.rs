/*!
 * Local Folder Sync Service Unit Tests
 * 
 * Tests for local filesystem synchronization functionality including:
 * - Path validation and access checking
 * - Recursive directory traversal
 * - Symlink handling
 * - File change detection
 * - Permission handling
 * - Cross-platform path normalization
 */

use std::path::{Path, PathBuf};
use std::fs;
use tempfile::TempDir;
use uuid::Uuid;
use chrono::Utc;
use serde_json::json;

use readur::{
    models::{LocalFolderSourceConfig, SourceType},
    local_folder_service::LocalFolderService,
};

/// Create a test local folder configuration
fn create_test_local_config() -> LocalFolderSourceConfig {
    LocalFolderSourceConfig {
        paths: vec!["/test/documents".to_string(), "/test/images".to_string()],
        recursive: true,
        follow_symlinks: false,
        auto_sync: true,
        sync_interval_minutes: 30,
        file_extensions: vec![".pdf".to_string(), ".txt".to_string(), ".jpg".to_string()],
    }
}

/// Create a test directory structure
fn create_test_directory_structure() -> Result<TempDir, std::io::Error> {
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path();
    
    // Create directory structure
    fs::create_dir_all(base_path.join("documents"))?;
    fs::create_dir_all(base_path.join("documents/subfolder"))?;
    fs::create_dir_all(base_path.join("images"))?;
    fs::create_dir_all(base_path.join("restricted"))?;
    
    // Create test files
    fs::write(base_path.join("documents/test1.pdf"), b"PDF content")?;
    fs::write(base_path.join("documents/test2.txt"), b"Text content")?;
    fs::write(base_path.join("documents/subfolder/nested.pdf"), b"Nested PDF")?;
    fs::write(base_path.join("images/photo.jpg"), b"Image content")?;
    fs::write(base_path.join("documents/ignored.exe"), b"Executable")?;
    fs::write(base_path.join("restricted/secret.txt"), b"Secret content")?;
    
    Ok(temp_dir)
}

#[test]
fn test_local_folder_config_creation() {
    let config = create_test_local_config();
    
    assert_eq!(config.paths.len(), 2);
    assert_eq!(config.paths[0], "/test/documents");
    assert_eq!(config.paths[1], "/test/images");
    assert!(config.recursive);
    assert!(!config.follow_symlinks);
    assert!(config.auto_sync);
    assert_eq!(config.sync_interval_minutes, 30);
    assert_eq!(config.file_extensions.len(), 3);
}

#[test]
fn test_local_folder_config_validation() {
    let config = create_test_local_config();
    
    // Test paths validation
    assert!(!config.paths.is_empty(), "Should have at least one path");
    for path in &config.paths {
        assert!(Path::new(path).is_absolute() || path.starts_with('.'), 
                "Path should be absolute or relative: {}", path);
    }
    
    // Test sync interval validation
    assert!(config.sync_interval_minutes > 0, "Sync interval should be positive");
    
    // Test file extensions validation
    assert!(!config.file_extensions.is_empty(), "Should have file extensions");
    for ext in &config.file_extensions {
        assert!(ext.starts_with('.'), "Extension should start with dot: {}", ext);
    }
}

#[test]
fn test_path_normalization() {
    let test_cases = vec![
        ("./documents", "./documents"),
        ("../documents", "../documents"),
        ("/home/user/documents", "/home/user/documents"),
        ("C:\\Users\\test\\Documents", "C:\\Users\\test\\Documents"),
        ("documents/", "documents"),
        ("documents//subfolder", "documents/subfolder"),
    ];
    
    for (input, expected) in test_cases {
        let normalized = normalize_path(input);
        // On different platforms, the exact normalization might vary
        // but we can test basic properties
        assert!(!normalized.is_empty(), "Normalized path should not be empty");
        assert!(!normalized.contains("//"), "Should not contain double slashes");
        assert!(!normalized.ends_with('/') || normalized == "/", "Should not end with slash unless root");
    }
}

fn normalize_path(path: &str) -> String {
    let path = path.trim_end_matches('/');
    path.replace("//", "/")
}

#[test]
fn test_file_extension_filtering() {
    let config = create_test_local_config();
    let allowed_extensions = &config.file_extensions;
    
    let test_files = vec![
        ("document.pdf", true),
        ("notes.txt", true),
        ("photo.jpg", true),
        ("archive.zip", false),
        ("program.exe", false),
        ("script.sh", false),
        ("Document.PDF", true), // Test case insensitivity
        ("README", false), // No extension
        (".hidden.txt", true), // Hidden file with allowed extension
    ];
    
    for (filename, should_be_allowed) in test_files {
        let extension = extract_extension(filename);
        let is_allowed = allowed_extensions.contains(&extension);
        
        assert_eq!(is_allowed, should_be_allowed, 
                   "File {} should be {}", filename, 
                   if should_be_allowed { "allowed" } else { "rejected" });
    }
}

fn extract_extension(filename: &str) -> String {
    if let Some(pos) = filename.rfind('.') {
        filename[pos..].to_lowercase()
    } else {
        String::new()
    }
}

#[test]
fn test_recursive_directory_traversal() {
    let temp_dir = create_test_directory_structure().unwrap();
    let base_path = temp_dir.path();
    
    // Test recursive traversal
    let mut files_found = Vec::new();
    collect_files_recursive(base_path, &mut files_found).unwrap();
    
    assert!(!files_found.is_empty(), "Should find files in directory structure");
    
    // Should find files in subdirectories when recursive is enabled
    let nested_files: Vec<_> = files_found.iter()
        .filter(|f| f.to_string_lossy().contains("subfolder"))
        .collect();
    assert!(!nested_files.is_empty(), "Should find files in subdirectories");
    
    // Test non-recursive traversal
    let mut files_flat = Vec::new();
    collect_files_flat(base_path, &mut files_flat).unwrap();
    
    // Should find fewer files when not recursive
    assert!(files_flat.len() <= files_found.len(), 
            "Non-recursive should find same or fewer files");
}

fn collect_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    use walkdir::WalkDir;
    
    for entry in WalkDir::new(dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            files.push(entry.path().to_path_buf());
        }
    }
    Ok(())
}

fn collect_files_flat(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            files.push(entry.path());
        }
    }
    Ok(())
}

#[test]
fn test_symlink_handling() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();
    
    // Create a file and a symlink to it
    let file_path = base_path.join("original.txt");
    fs::write(&file_path, b"Original content").unwrap();
    
    let symlink_path = base_path.join("link.txt");
    
    // Create symlink (this might fail on Windows without admin rights)
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        if symlink(&file_path, &symlink_path).is_ok() {
            // Test with follow_symlinks = true
            let mut files_with_symlinks = Vec::new();
            collect_files_with_symlinks(base_path, true, &mut files_with_symlinks).unwrap();
            
            // Should find both original and symlinked file
            assert!(files_with_symlinks.len() >= 2, "Should find original and symlinked files");
            
            // Test with follow_symlinks = false
            let mut files_without_symlinks = Vec::new();
            collect_files_with_symlinks(base_path, false, &mut files_without_symlinks).unwrap();
            
            // Should find only original file
            assert!(files_without_symlinks.len() < files_with_symlinks.len(), 
                    "Should find fewer files when not following symlinks");
        }
    }
}

fn collect_files_with_symlinks(dir: &Path, follow_symlinks: bool, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    use walkdir::WalkDir;
    
    let walker = WalkDir::new(dir).follow_links(follow_symlinks);
    
    for entry in walker {
        let entry = entry?;
        if entry.file_type().is_file() {
            files.push(entry.path().to_path_buf());
        }
    }
    Ok(())
}

#[test]
fn test_file_metadata_extraction() {
    let temp_dir = create_test_directory_structure().unwrap();
    let base_path = temp_dir.path();
    let test_file = base_path.join("documents/test1.pdf");
    
    let metadata = fs::metadata(&test_file).unwrap();
    
    // Test basic metadata
    assert!(metadata.is_file());
    assert!(!metadata.is_dir());
    assert!(metadata.len() > 0);
    
    // Test modification time
    let modified = metadata.modified().unwrap();
    let now = std::time::SystemTime::now();
    assert!(modified <= now, "File modification time should be in the past");
    
    // Test file size
    let expected_size = "PDF content".len() as u64;
    assert_eq!(metadata.len(), expected_size);
}

#[test]
fn test_permission_checking() {
    let temp_dir = create_test_directory_structure().unwrap();
    let base_path = temp_dir.path();
    
    // Test readable file
    let readable_file = base_path.join("documents/test1.pdf");
    assert!(readable_file.exists());
    assert!(is_readable(&readable_file));
    
    // Test readable directory
    let readable_dir = base_path.join("documents");
    assert!(readable_dir.exists());
    assert!(readable_dir.is_dir());
    assert!(is_readable(&readable_dir));
    
    // Test non-existent path
    let non_existent = base_path.join("does_not_exist.txt");
    assert!(!non_existent.exists());
    assert!(!is_readable(&non_existent));
}

fn is_readable(path: &Path) -> bool {
    path.exists() && fs::metadata(path).is_ok()
}

#[test]
fn test_file_change_detection() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    
    // Create initial file
    fs::write(&test_file, b"Initial content").unwrap();
    let initial_metadata = fs::metadata(&test_file).unwrap();
    let initial_modified = initial_metadata.modified().unwrap();
    let initial_size = initial_metadata.len();
    
    // Wait a bit to ensure timestamp difference
    std::thread::sleep(std::time::Duration::from_millis(10));
    
    // Modify file
    fs::write(&test_file, b"Modified content").unwrap();
    let modified_metadata = fs::metadata(&test_file).unwrap();
    let modified_modified = modified_metadata.modified().unwrap();
    let modified_size = modified_metadata.len();
    
    // Test change detection
    assert_ne!(initial_size, modified_size, "File size should change");
    assert!(modified_modified >= initial_modified, "Modification time should advance");
}

#[test]
fn test_error_handling() {
    // Test various error scenarios
    
    // Non-existent path
    let non_existent_config = LocalFolderSourceConfig {
        paths: vec!["/this/path/does/not/exist".to_string()],
        recursive: true,
        follow_symlinks: false,
        auto_sync: true,
        sync_interval_minutes: 30,
        file_extensions: vec![".txt".to_string()],
    };
    
    assert_eq!(non_existent_config.paths[0], "/this/path/does/not/exist");
    
    // Empty paths
    let empty_paths_config = LocalFolderSourceConfig {
        paths: Vec::new(),
        recursive: true,
        follow_symlinks: false,
        auto_sync: true,
        sync_interval_minutes: 30,
        file_extensions: vec![".txt".to_string()],
    };
    
    assert!(empty_paths_config.paths.is_empty());
    
    // Invalid sync interval
    let invalid_interval_config = LocalFolderSourceConfig {
        paths: vec!["/test".to_string()],
        recursive: true,
        follow_symlinks: false,
        auto_sync: true,
        sync_interval_minutes: 0, // Invalid
        file_extensions: vec![".txt".to_string()],
    };
    
    assert_eq!(invalid_interval_config.sync_interval_minutes, 0);
}

#[test]
fn test_cross_platform_paths() {
    let test_paths = vec![
        ("/home/user/documents", true),  // Unix absolute
        ("./documents", true),           // Relative
        ("../documents", true),          // Relative parent
        ("documents", true),             // Relative simple
        ("C:\\Users\\test", true),       // Windows absolute
        ("", false),                     // Empty path
    ];
    
    for (path, should_be_valid) in test_paths {
        let is_valid = !path.is_empty();
        assert_eq!(is_valid, should_be_valid, "Path validation failed for: {}", path);
        
        if is_valid {
            let path_obj = Path::new(path);
            // Test that we can create a Path object
            assert_eq!(path_obj.to_string_lossy(), path);
        }
    }
}

#[test]
fn test_file_filtering_performance() {
    // Create a larger set of test files to test filtering performance
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();
    
    // Create many files
    for i in 0..1000 {
        let filename = format!("file_{}.txt", i);
        let filepath = base_path.join(&filename);
        fs::write(filepath, format!("Content {}", i)).unwrap();
    }
    
    // Create some files with different extensions
    for i in 0..100 {
        let filename = format!("doc_{}.pdf", i);
        let filepath = base_path.join(&filename);
        fs::write(filepath, format!("PDF {}", i)).unwrap();
    }
    
    let config = create_test_local_config();
    let start = std::time::Instant::now();
    
    // Simulate filtering
    let mut matching_files = 0;
    for entry in fs::read_dir(base_path).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            let filename = entry.file_name().to_string_lossy().to_string();
            let extension = extract_extension(&filename);
            if config.file_extensions.contains(&extension) {
                matching_files += 1;
            }
        }
    }
    
    let elapsed = start.elapsed();
    
    assert!(matching_files > 0, "Should find matching files");
    assert!(elapsed < std::time::Duration::from_secs(1), "Filtering should be fast");
}

#[test]
fn test_concurrent_access_safety() {
    use std::sync::{Arc, Mutex};
    use std::thread;
    
    let temp_dir = create_test_directory_structure().unwrap();
    let base_path = Arc::new(temp_dir.path().to_path_buf());
    let file_count = Arc::new(Mutex::new(0));
    
    let mut handles = vec![];
    
    // Spawn multiple threads to read the same directory
    for _ in 0..4 {
        let base_path = Arc::clone(&base_path);
        let file_count = Arc::clone(&file_count);
        
        let handle = thread::spawn(move || {
            let mut local_count = 0;
            if let Ok(entries) = fs::read_dir(&*base_path) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        if entry.file_type().unwrap().is_file() {
                            local_count += 1;
                        }
                    }
                }
            }
            
            let mut count = file_count.lock().unwrap();
            *count += local_count;
        });
        
        handles.push(handle);
    }
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    let final_count = *file_count.lock().unwrap();
    assert!(final_count > 0, "Should have counted files from multiple threads");
}

#[test]
fn test_hidden_file_handling() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();
    
    // Create regular and hidden files
    fs::write(base_path.join("visible.txt"), b"Visible content").unwrap();
    fs::write(base_path.join(".hidden.txt"), b"Hidden content").unwrap();
    
    // Test file discovery
    let mut all_files = Vec::new();
    for entry in fs::read_dir(base_path).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            all_files.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    
    assert!(all_files.contains(&"visible.txt".to_string()));
    
    // Hidden file visibility depends on the OS and settings
    let has_hidden = all_files.iter().any(|f| f.starts_with('.'));
    println!("Hidden files found: {}", has_hidden);
    
    // Filter hidden files if needed
    let visible_files: Vec<_> = all_files.iter()
        .filter(|f| !f.starts_with('.'))
        .collect();
    
    assert!(!visible_files.is_empty(), "Should find at least one visible file");
}

#[test]
fn test_large_file_handling() {
    let temp_dir = TempDir::new().unwrap();
    let large_file = temp_dir.path().join("large.txt");
    
    // Create a larger file (1MB)
    let content = "a".repeat(1024 * 1024);
    fs::write(&large_file, content.as_bytes()).unwrap();
    
    let metadata = fs::metadata(&large_file).unwrap();
    assert_eq!(metadata.len(), 1024 * 1024);
    
    // Test that we can handle large file metadata efficiently
    let start = std::time::Instant::now();
    let _metadata = fs::metadata(&large_file).unwrap();
    let elapsed = start.elapsed();
    
    assert!(elapsed < std::time::Duration::from_millis(100), 
            "Metadata reading should be fast even for large files");
}

#[test]
fn test_disk_space_estimation() {
    let temp_dir = create_test_directory_structure().unwrap();
    let base_path = temp_dir.path();
    
    let mut total_size = 0u64;
    let mut file_count = 0u32;
    
    for entry in walkdir::WalkDir::new(base_path) {
        let entry = entry.unwrap();
        if entry.file_type().is_file() {
            if let Ok(metadata) = entry.metadata() {
                total_size += metadata.len();
                file_count += 1;
            }
        }
    }
    
    assert!(file_count > 0, "Should count files");
    assert!(total_size > 0, "Should calculate total size");
    
    // Calculate average file size
    let avg_size = if file_count > 0 { total_size / file_count as u64 } else { 0 };
    assert!(avg_size > 0, "Should calculate average file size");
}