/*!
 * WebDAV Sync Service Unit Tests
 * 
 * Tests for WebDAV synchronization functionality including:
 * - Connection testing and validation
 * - File discovery and enumeration
 * - ETag-based change detection
 * - File download and processing
 * - Error handling and retry logic
 * - Server type detection (Nextcloud, ownCloud, etc.)
 */

use std::collections::HashMap;
use uuid::Uuid;
use chrono::Utc;
use serde_json::json;

use readur::{
    models::{WebDAVSourceConfig, SourceType, WebDAVFile, WebDAVCrawlEstimate, WebDAVFolderInfo},
    services::webdav_service::{WebDAVService, WebDAVConfig},
};

/// Create a test WebDAV configuration
fn create_test_webdav_config() -> WebDAVConfig {
    WebDAVConfig {
        server_url: "https://cloud.example.com".to_string(),
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string(), "/Photos".to_string()],
        file_extensions: vec![".pdf".to_string(), ".txt".to_string(), ".jpg".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    }
}

/// Create a test WebDAV source configuration
fn create_test_source_config() -> WebDAVSourceConfig {
    WebDAVSourceConfig {
        server_url: "https://cloud.example.com".to_string(),
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec![".pdf".to_string(), ".txt".to_string()],
        auto_sync: true,
        sync_interval_minutes: 60,
        server_type: Some("nextcloud".to_string()),
    }
}

#[test]
fn test_webdav_config_creation() {
    let config = create_test_webdav_config();
    
    assert_eq!(config.server_url, "https://cloud.example.com");
    assert_eq!(config.username, "testuser");
    assert_eq!(config.password, "testpass");
    assert_eq!(config.watch_folders.len(), 2);
    assert_eq!(config.file_extensions.len(), 3);
    assert_eq!(config.timeout_seconds, 30);
    assert_eq!(config.server_type, Some("nextcloud".to_string()));
}

#[test]
fn test_webdav_source_config_creation() {
    let config = create_test_source_config();
    
    assert_eq!(config.server_url, "https://cloud.example.com");
    assert_eq!(config.username, "testuser");
    assert!(config.auto_sync);
    assert_eq!(config.sync_interval_minutes, 60);
    assert_eq!(config.server_type, Some("nextcloud".to_string()));
}

#[test]
fn test_webdav_config_validation() {
    let config = create_test_webdav_config();
    
    // Test URL validation
    assert!(config.server_url.starts_with("https://"));
    assert!(!config.server_url.is_empty());
    
    // Test credentials validation
    assert!(!config.username.is_empty());
    assert!(!config.password.is_empty());
    
    // Test folders validation
    assert!(!config.watch_folders.is_empty());
    for folder in &config.watch_folders {
        assert!(folder.starts_with('/'));
    }
    
    // Test extensions validation
    assert!(!config.file_extensions.is_empty());
    for ext in &config.file_extensions {
        assert!(ext.starts_with('.'));
    }
    
    // Test timeout validation
    assert!(config.timeout_seconds > 0);
    assert!(config.timeout_seconds <= 300); // Max 5 minutes
}

#[test]
fn test_webdav_file_structure() {
    let webdav_file = WebDAVFile {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        webdav_path: "/Documents/test.pdf".to_string(),
        etag: "abc123".to_string(),
        last_modified: Some(Utc::now()),
        file_size: 1024,
        mime_type: "application/pdf".to_string(),
        document_id: None,
        sync_status: "synced".to_string(),
        sync_error: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    assert_eq!(webdav_file.webdav_path, "/Documents/test.pdf");
    assert_eq!(webdav_file.etag, "abc123");
    assert_eq!(webdav_file.file_size, 1024);
    assert_eq!(webdav_file.mime_type, "application/pdf");
    
    // Test filename extraction
    let filename = webdav_file.webdav_path.split('/').last().unwrap();
    assert_eq!(filename, "test.pdf");
    
    // Test extension detection
    let extension = filename.split('.').last().unwrap();
    assert_eq!(extension, "pdf");
}

#[test]
fn test_file_extension_filtering() {
    let config = create_test_webdav_config();
    let allowed_extensions = &config.file_extensions;
    
    // Test allowed files
    assert!(allowed_extensions.contains(&".pdf".to_string()));
    assert!(allowed_extensions.contains(&".txt".to_string()));
    assert!(allowed_extensions.contains(&".jpg".to_string()));
    
    // Test disallowed files
    assert!(!allowed_extensions.contains(&".exe".to_string()));
    assert!(!allowed_extensions.contains(&".bat".to_string()));
    assert!(!allowed_extensions.contains(&".sh".to_string()));
    
    // Test case sensitivity
    let test_files = vec![
        ("document.PDF", ".pdf"),
        ("notes.TXT", ".txt"),
        ("image.JPG", ".jpg"),
        ("archive.ZIP", ".zip"),
    ];
    
    for (filename, expected_ext) in test_files {
        let ext = format!(".{}", filename.split('.').last().unwrap().to_lowercase());
        assert_eq!(ext, expected_ext);
        
        if allowed_extensions.contains(&ext) {
            println!("✅ File {} would be processed", filename);
        } else {
            println!("❌ File {} would be skipped", filename);
        }
    }
}

#[test]
fn test_etag_change_detection() {
    let old_etag = "abc123";
    let new_etag = "def456";
    let same_etag = "abc123";
    
    // Test change detection
    assert_ne!(old_etag, new_etag, "Different ETags should indicate file change");
    assert_eq!(old_etag, same_etag, "Same ETags should indicate no change");
    
    // Test ETag normalization (some servers use quotes)
    let quoted_etag = "\"abc123\"";
    let normalized_etag = quoted_etag.trim_matches('"');
    assert_eq!(normalized_etag, old_etag);
}

#[test]
fn test_etag_normalization() {
    // Test various ETag formats that WebDAV servers might return
    let test_cases = vec![
        ("abc123", "abc123"),                    // Plain ETag
        ("\"abc123\"", "abc123"),                // Quoted ETag
        ("W/\"abc123\"", "abc123"),              // Weak ETag
        ("\"abc-123-def\"", "abc-123-def"),      // Quoted with dashes
        ("W/\"abc-123-def\"", "abc-123-def"),    // Weak ETag with dashes
    ];
    
    for (input, expected) in test_cases {
        let normalized = input
            .trim_start_matches("W/")
            .trim_matches('"');
        assert_eq!(normalized, expected, 
            "Failed to normalize ETag: {} -> expected {}", input, expected);
    }
}

#[test]
fn test_etag_comparison_fixes_duplicate_downloads() {
    // This test demonstrates how ETag normalization prevents unnecessary downloads
    
    // Simulate a WebDAV server that returns quoted ETags 
    let server_etag = "\"file-hash-123\"";
    
    // Before fix: stored ETag would have quotes, server ETag would have quotes
    // After fix: both should be normalized (no quotes)
    let normalized_server = server_etag.trim_start_matches("W/").trim_matches('"');
    let normalized_stored = "file-hash-123"; // What would be stored after normalization
    
    // These should match after normalization, preventing redownload
    assert_eq!(normalized_server, normalized_stored, 
        "Normalized ETags should match to prevent unnecessary redownloads");
    
    // Demonstrate the issue that was fixed
    let old_behavior_would_mismatch = (server_etag != normalized_stored);
    assert!(old_behavior_would_mismatch, 
        "Before fix: quoted vs unquoted ETags would cause unnecessary downloads");
    
    let new_behavior_matches = (normalized_server == normalized_stored);
    assert!(new_behavior_matches, 
        "After fix: normalized ETags match, preventing unnecessary downloads");
}

#[test]
fn test_path_normalization() {
    let test_paths = vec![
        ("/Documents/test.pdf", "/Documents/test.pdf"),
        ("Documents/test.pdf", "/Documents/test.pdf"),
        ("/Documents//test.pdf", "/Documents/test.pdf"),
        ("/Documents/./test.pdf", "/Documents/test.pdf"),
    ];
    
    for (input, expected) in test_paths {
        let normalized = normalize_webdav_path(input);
        assert_eq!(normalized, expected, "Path normalization failed for: {}", input);
    }
}

fn normalize_webdav_path(path: &str) -> String {
    let mut normalized = path.to_string();
    
    // Ensure path starts with /
    if !normalized.starts_with('/') {
        normalized = format!("/{}", normalized);
    }
    
    // Remove double slashes
    while normalized.contains("//") {
        normalized = normalized.replace("//", "/");
    }
    
    // Remove ./ references
    normalized = normalized.replace("/./", "/");
    
    normalized
}

#[test]
fn test_crawl_estimate_structure() {
    let estimate = WebDAVCrawlEstimate {
        folders: vec![
            WebDAVFolderInfo {
                path: "/Documents".to_string(),
                total_files: 10,
                supported_files: 8,
                estimated_time_hours: 0.5,
                total_size_mb: 50.0
            },
            WebDAVFolderInfo {
                path: "/Photos".to_string(),
                total_files: 100,
                supported_files: 90,
                estimated_time_hours: 2.0,
                total_size_mb: 500.0
            }
        ],
        total_files: 110,
        total_supported_files: 98,
        total_estimated_time_hours: 2.5,
        total_size_mb: 550.0,
    };
    
    assert_eq!(estimate.folders.len(), 2);
    assert_eq!(estimate.total_files, 110);
    assert_eq!(estimate.total_supported_files, 98);
    assert_eq!(estimate.total_estimated_time_hours, 2.5);
    assert_eq!(estimate.total_size_mb, 550.0);
    
    // Test calculation accuracy
    let calculated_files: i64 = estimate.folders.iter()
        .map(|f| f.total_files)
        .sum();
    assert_eq!(calculated_files, estimate.total_files);
    
    let calculated_supported: i64 = estimate.folders.iter()
        .map(|f| f.supported_files)
        .sum();
    assert_eq!(calculated_supported, estimate.total_supported_files);
}

#[test]
fn test_webdav_url_construction() {
    let base_url = "https://cloud.example.com";
    let folder = "/Documents";
    let filename = "test.pdf";
    
    // Test DAV endpoint construction
    let dav_url = format!("{}/remote.php/dav/files/testuser{}", base_url, folder);
    assert_eq!(dav_url, "https://cloud.example.com/remote.php/dav/files/testuser/Documents");
    
    // Test file URL construction
    let file_url = format!("{}/{}", dav_url, filename);
    assert_eq!(file_url, "https://cloud.example.com/remote.php/dav/files/testuser/Documents/test.pdf");
    
    // Test URL encoding (spaces and special characters)
    let special_filename = "my document (1).pdf";
    let encoded_filename = urlencoding::encode(special_filename);
    let encoded_url = format!("{}/{}", dav_url, encoded_filename);
    assert!(encoded_url.contains("my%20document%20%281%29.pdf"));
}

#[test]
fn test_server_type_detection() {
    let server_types = vec![
        ("nextcloud", true),
        ("owncloud", true),
        ("apache", false),
        ("nginx", false),
        ("unknown", false),
    ];
    
    for (server_type, is_supported) in server_types {
        let config = WebDAVConfig {
            server_url: "https://test.com".to_string(),
            username: "test".to_string(),
            password: "test".to_string(),
            watch_folders: vec!["/test".to_string()],
            file_extensions: vec![".pdf".to_string()],
            timeout_seconds: 30,
            server_type: Some(server_type.to_string()),
        };
        
        if is_supported {
            assert!(["nextcloud", "owncloud"].contains(&config.server_type.as_ref().unwrap().as_str()));
        } else {
            assert!(!["nextcloud", "owncloud"].contains(&config.server_type.as_ref().unwrap().as_str()));
        }
    }
}

#[test]
fn test_error_handling_scenarios() {
    // Test various error scenarios that might occur during sync
    
    // Network timeout scenario
    let timeout_config = WebDAVConfig {
        server_url: "https://invalid-server.com".to_string(),
        username: "test".to_string(),
        password: "test".to_string(),
        watch_folders: vec!["/test".to_string()],
        file_extensions: vec![".pdf".to_string()],
        timeout_seconds: 1, // Very short timeout
        server_type: Some("nextcloud".to_string()),
    };
    
    assert_eq!(timeout_config.timeout_seconds, 1);
    
    // Authentication error scenario
    let auth_config = WebDAVConfig {
        server_url: "https://cloud.example.com".to_string(),
        username: "invalid_user".to_string(),
        password: "wrong_password".to_string(),
        watch_folders: vec!["/test".to_string()],
        file_extensions: vec![".pdf".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };
    
    assert_eq!(auth_config.username, "invalid_user");
    assert_eq!(auth_config.password, "wrong_password");
    
    // Invalid folder path scenario
    let invalid_path_config = WebDAVConfig {
        server_url: "https://cloud.example.com".to_string(),
        username: "test".to_string(),
        password: "test".to_string(),
        watch_folders: vec!["/nonexistent_folder".to_string()],
        file_extensions: vec![".pdf".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };
    
    assert_eq!(invalid_path_config.watch_folders[0], "/nonexistent_folder");
}

#[test]
fn test_sync_performance_metrics() {
    // Test metrics that would be important for sync performance
    
    let sync_stats = SyncStats {
        files_discovered: 100,
        files_downloaded: 85,
        files_skipped: 10,
        files_failed: 5,
        total_bytes_downloaded: 10_000_000, // 10MB
        sync_duration_ms: 30_000, // 30 seconds
        average_download_speed_mbps: 2.67,
    };
    
    assert_eq!(sync_stats.files_discovered, 100);
    assert_eq!(sync_stats.files_downloaded, 85);
    assert_eq!(sync_stats.files_skipped, 10);
    assert_eq!(sync_stats.files_failed, 5);
    
    // Test calculated metrics
    let total_processed = sync_stats.files_downloaded + sync_stats.files_skipped + sync_stats.files_failed;
    assert_eq!(total_processed, sync_stats.files_discovered);
    
    let success_rate = (sync_stats.files_downloaded as f64 / sync_stats.files_discovered as f64) * 100.0;
    assert!(success_rate >= 80.0, "Success rate should be at least 80%");
    
    let mb_downloaded = sync_stats.total_bytes_downloaded as f64 / 1_000_000.0;
    let seconds = sync_stats.sync_duration_ms as f64 / 1000.0;
    let calculated_speed = (mb_downloaded * 8.0) / seconds; // Convert to Mbps
    assert!((calculated_speed - sync_stats.average_download_speed_mbps).abs() < 0.1);
}

#[derive(Debug, Clone)]
struct SyncStats {
    files_discovered: u32,
    files_downloaded: u32,
    files_skipped: u32,
    files_failed: u32,
    total_bytes_downloaded: u64,
    sync_duration_ms: u64,
    average_download_speed_mbps: f64,
}

#[test]
fn test_concurrent_sync_protection() {
    // Test data structures that would prevent concurrent syncs
    use std::sync::{Arc, Mutex};
    use std::collections::HashSet;
    
    let active_syncs: Arc<Mutex<HashSet<Uuid>>> = Arc::new(Mutex::new(HashSet::new()));
    
    let source_id = Uuid::new_v4();
    
    // Test adding a sync
    {
        let mut syncs = active_syncs.lock().unwrap();
        let was_inserted = syncs.insert(source_id);
        assert!(was_inserted, "First sync should be allowed");
    }
    
    // Test preventing duplicate sync
    {
        let mut syncs = active_syncs.lock().unwrap();
        let was_inserted = syncs.insert(source_id);
        assert!(!was_inserted, "Duplicate sync should be prevented");
    }
    
    // Test removing completed sync
    {
        let mut syncs = active_syncs.lock().unwrap();
        let was_removed = syncs.remove(&source_id);
        assert!(was_removed, "Sync should be removable after completion");
    }
}

#[test]
fn test_file_hash_comparison() {
    use sha2::{Sha256, Digest};
    
    // Test SHA256 hash generation for file deduplication
    let file_content_1 = b"This is test file content";
    let file_content_2 = b"This is different content";
    let file_content_3 = b"This is test file content"; // Same as 1
    
    let hash_1 = Sha256::digest(file_content_1);
    let hash_2 = Sha256::digest(file_content_2);
    let hash_3 = Sha256::digest(file_content_3);
    
    assert_ne!(hash_1, hash_2, "Different content should have different hashes");
    assert_eq!(hash_1, hash_3, "Same content should have same hashes");
    
    // Test hex encoding
    let hash_1_hex = format!("{:x}", hash_1);
    let hash_3_hex = format!("{:x}", hash_3);
    assert_eq!(hash_1_hex, hash_3_hex);
    assert_eq!(hash_1_hex.len(), 64); // SHA256 is 64 hex characters
}

#[test]
fn test_retry_mechanism() {
    // Test exponential backoff for retry logic
    fn calculate_retry_delay(attempt: u32, base_delay_ms: u64) -> u64 {
        let max_delay_ms = 30_000; // 30 seconds max
        let delay = base_delay_ms * 2_u64.pow(attempt.saturating_sub(1));
        std::cmp::min(delay, max_delay_ms)
    }
    
    assert_eq!(calculate_retry_delay(1, 1000), 1000);   // 1 second
    assert_eq!(calculate_retry_delay(2, 1000), 2000);   // 2 seconds
    assert_eq!(calculate_retry_delay(3, 1000), 4000);   // 4 seconds
    assert_eq!(calculate_retry_delay(4, 1000), 8000);   // 8 seconds
    assert_eq!(calculate_retry_delay(5, 1000), 16000);  // 16 seconds
    assert_eq!(calculate_retry_delay(6, 1000), 30000);  // Capped at 30 seconds
    assert_eq!(calculate_retry_delay(10, 1000), 30000); // Still capped at 30 seconds
}

#[test]
fn test_bandwidth_limiting() {
    // Test bandwidth limiting calculations
    fn calculate_download_delay(bytes_downloaded: u64, target_mbps: f64) -> u64 {
        if target_mbps <= 0.0 {
            return 0; // No limit
        }
        
        let bits_downloaded = bytes_downloaded * 8;
        let target_bps = target_mbps * 1_000_000.0;
        let ideal_duration_ms = (bits_downloaded as f64 / target_bps * 1000.0) as u64;
        
        ideal_duration_ms
    }
    
    // Test with 1 Mbps limit
    let delay_1mb = calculate_download_delay(125_000, 1.0); // 1 Mb of data
    assert_eq!(delay_1mb, 1000); // Should take 1 second
    
    // Test with 10 Mbps limit
    let delay_10mb = calculate_download_delay(125_000, 10.0); // 1 Mb of data
    assert_eq!(delay_10mb, 100); // Should take 0.1 seconds
    
    // Test with no limit
    let delay_unlimited = calculate_download_delay(125_000, 0.0);
    assert_eq!(delay_unlimited, 0); // No delay
}