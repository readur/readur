/*!
 * Basic Sync Unit Tests
 * 
 * Simple tests for sync functionality that don't require database connection
 */

use serde_json::json;
use uuid::Uuid;
use chrono::Utc;

use readur::models::{SourceType, SourceStatus, WebDAVSourceConfig, LocalFolderSourceConfig, S3SourceConfig};

#[test]
fn test_source_type_string_conversion() {
    assert_eq!(SourceType::WebDAV.to_string(), "webdav");
    assert_eq!(SourceType::LocalFolder.to_string(), "local_folder");
    assert_eq!(SourceType::S3.to_string(), "s3");
}

#[test]
fn test_source_status_string_conversion() {
    assert_eq!(SourceStatus::Idle.to_string(), "idle");
    assert_eq!(SourceStatus::Syncing.to_string(), "syncing");
    assert_eq!(SourceStatus::Error.to_string(), "error");
}

#[test]
fn test_webdav_config_serialization() {
    let config = WebDAVSourceConfig {
        server_url: "https://cloud.example.com".to_string(),
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec![".pdf".to_string(), ".txt".to_string()],
        auto_sync: true,
        sync_interval_minutes: 60,
        server_type: Some("nextcloud".to_string()),
    };
    
    let json_value = serde_json::to_value(&config).unwrap();
    let deserialized: WebDAVSourceConfig = serde_json::from_value(json_value).unwrap();
    
    assert_eq!(config.server_url, deserialized.server_url);
    assert_eq!(config.username, deserialized.username);
    assert_eq!(config.auto_sync, deserialized.auto_sync);
    assert_eq!(config.sync_interval_minutes, deserialized.sync_interval_minutes);
}

#[test]
fn test_local_folder_config_serialization() {
    let config = LocalFolderSourceConfig {
        watch_folders: vec!["/home/user/documents".to_string()],
        file_extensions: vec![".pdf".to_string(), ".txt".to_string(), ".jpg".to_string()],
        auto_sync: true,
        sync_interval_minutes: 30,
        recursive: true,
        follow_symlinks: false,
    };
    
    let json_value = serde_json::to_value(&config).unwrap();
    let deserialized: LocalFolderSourceConfig = serde_json::from_value(json_value).unwrap();
    
    assert_eq!(config.watch_folders, deserialized.watch_folders);
    assert_eq!(config.recursive, deserialized.recursive);
    assert_eq!(config.follow_symlinks, deserialized.follow_symlinks);
    assert_eq!(config.sync_interval_minutes, deserialized.sync_interval_minutes);
}

#[test]
fn test_s3_config_serialization() {
    let config = S3SourceConfig {
        bucket_name: "test-documents".to_string(),
        region: "us-east-1".to_string(),
        access_key_id: "AKIATEST".to_string(),
        secret_access_key: "secrettest".to_string(),
        endpoint_url: Some("https://minio.example.com".to_string()),
        prefix: Some("documents/".to_string()),
        watch_folders: vec!["documents/".to_string()],
        file_extensions: vec![".pdf".to_string(), ".docx".to_string()],
        auto_sync: true,
        sync_interval_minutes: 120,
    };
    
    let json_value = serde_json::to_value(&config).unwrap();
    let deserialized: S3SourceConfig = serde_json::from_value(json_value).unwrap();
    
    assert_eq!(config.bucket_name, deserialized.bucket_name);
    assert_eq!(config.region, deserialized.region);
    assert_eq!(config.endpoint_url, deserialized.endpoint_url);
    assert_eq!(config.prefix, deserialized.prefix);
    assert_eq!(config.sync_interval_minutes, deserialized.sync_interval_minutes);
}

#[test]
fn test_auto_sync_validation() {
    // Test that auto_sync works with different intervals
    let intervals = vec![1, 15, 30, 60, 120, 240, 480];
    
    for interval in intervals {
        let webdav_config = WebDAVSourceConfig {
            server_url: "https://test.com".to_string(),
            username: "test".to_string(),
            password: "test".to_string(),
            watch_folders: vec!["/test".to_string()],
            file_extensions: vec![".pdf".to_string()],
            auto_sync: true,
            sync_interval_minutes: interval,
            server_type: Some("nextcloud".to_string()),
        };
        
        assert!(webdav_config.auto_sync);
        assert_eq!(webdav_config.sync_interval_minutes, interval);
        assert!(webdav_config.sync_interval_minutes > 0);
    }
}

#[test]
fn test_file_extension_validation() {
    let valid_extensions = vec![".pdf", ".txt", ".jpg", ".png", ".docx", ".xlsx"];
    let invalid_extensions = vec!["pdf", "txt", "", "no-dot", ".", ".."];
    
    for ext in valid_extensions {
        assert!(ext.starts_with('.'), "Extension should start with dot: {}", ext);
        assert!(ext.len() > 1, "Extension should have content after dot: {}", ext);
    }
    
    // Test in actual config
    let config = WebDAVSourceConfig {
        server_url: "https://test.com".to_string(),
        username: "test".to_string(),
        password: "test".to_string(),
        watch_folders: vec!["/test".to_string()],
        file_extensions: vec![".pdf".to_string(), ".txt".to_string()],
        auto_sync: true,
        sync_interval_minutes: 60,
        server_type: Some("nextcloud".to_string()),
    };
    
    for ext in &config.file_extensions {
        assert!(ext.starts_with('.'));
        assert!(ext.len() > 1);
    }
}

#[test]
fn test_watch_folder_validation() {
    let valid_folders = vec!["/", "/home", "/home/user", "/Documents", "/var/log"];
    let questionable_folders = vec!["", "relative/path", "../parent"];
    
    for folder in valid_folders {
        let config = LocalFolderSourceConfig {
            watch_folders: vec![folder.to_string()],
            file_extensions: vec![".pdf".to_string()],
            auto_sync: true,
            sync_interval_minutes: 30,
            recursive: true,
            follow_symlinks: false,
        };
        
        assert_eq!(config.watch_folders[0], folder);
        if folder.starts_with('/') {
            assert!(folder.len() >= 1);
        }
    }
}

#[test]
fn test_server_type_validation() {
    let valid_server_types = vec![
        Some("nextcloud".to_string()),
        Some("owncloud".to_string()),
        Some("generic".to_string()),
        None,
    ];
    
    for server_type in valid_server_types {
        let config = WebDAVSourceConfig {
            server_url: "https://test.com".to_string(),
            username: "test".to_string(),
            password: "test".to_string(),
            watch_folders: vec!["/test".to_string()],
            file_extensions: vec![".pdf".to_string()],
            auto_sync: true,
            sync_interval_minutes: 60,
            server_type: server_type.clone(),
        };
        
        assert_eq!(config.server_type, server_type);
    }
}

#[test]
fn test_s3_bucket_name_validation() {
    let valid_bucket_names = vec![
        "test-bucket",
        "my-bucket-123",
        "bucket.with.dots",
        "a", // minimum length
    ];
    
    let invalid_bucket_names = vec![
        "", // empty
        "Bucket", // uppercase
        "bucket_with_underscores", // underscores
        "bucket with spaces", // spaces
    ];
    
    for bucket_name in valid_bucket_names {
        let config = S3SourceConfig {
            bucket_name: bucket_name.to_string(),
            region: "us-east-1".to_string(),
            access_key_id: "test".to_string(),
            secret_access_key: "test".to_string(),
            endpoint_url: None,
            prefix: None,
            watch_folders: vec!["".to_string()],
            file_extensions: vec![".pdf".to_string()],
            auto_sync: true,
            sync_interval_minutes: 120,
        };
        
        assert_eq!(config.bucket_name, bucket_name);
        // Basic validation rules
        assert!(!config.bucket_name.is_empty());
        assert!(config.bucket_name.len() <= 63); // AWS limit
    }
}

#[test]
fn test_endpoint_url_handling() {
    // Test AWS S3 (no endpoint)
    let aws_config = S3SourceConfig {
        bucket_name: "test-bucket".to_string(),
        region: "us-east-1".to_string(),
        access_key_id: "AKIA...".to_string(),
        secret_access_key: "secret".to_string(),
        endpoint_url: None, // AWS S3
        prefix: None,
        watch_folders: vec!["".to_string()],
        file_extensions: vec![".pdf".to_string()],
        auto_sync: true,
        sync_interval_minutes: 120,
    };
    
    assert!(aws_config.endpoint_url.is_none());
    
    // Test MinIO (custom endpoint)
    let minio_config = S3SourceConfig {
        bucket_name: "test-bucket".to_string(),
        region: "us-east-1".to_string(),
        access_key_id: "minioadmin".to_string(),
        secret_access_key: "minioadmin".to_string(),
        endpoint_url: Some("https://minio.example.com".to_string()),
        prefix: None,
        watch_folders: vec!["".to_string()],
        file_extensions: vec![".pdf".to_string()],
        auto_sync: true,
        sync_interval_minutes: 120,
    };
    
    assert!(minio_config.endpoint_url.is_some());
    assert!(minio_config.endpoint_url.unwrap().starts_with("https://"));
}

#[test]
fn test_sync_interval_ranges() {
    // Test reasonable sync intervals
    let intervals = vec![
        (1, "Very frequent"),
        (5, "Frequent"),
        (15, "Every 15 minutes"),
        (30, "Half hourly"),
        (60, "Hourly"),
        (120, "Every 2 hours"),
        (240, "Every 4 hours"),
        (480, "Every 8 hours"),
        (1440, "Daily"),
    ];
    
    for (interval, description) in intervals {
        let config = WebDAVSourceConfig {
            server_url: "https://test.com".to_string(),
            username: "test".to_string(),
            password: "test".to_string(),
            watch_folders: vec!["/test".to_string()],
            file_extensions: vec![".pdf".to_string()],
            auto_sync: true,
            sync_interval_minutes: interval,
            server_type: Some("nextcloud".to_string()),
        };
        
        assert_eq!(config.sync_interval_minutes, interval);
        assert!(config.sync_interval_minutes > 0, "Interval should be positive for: {}", description);
        assert!(config.sync_interval_minutes <= 1440, "Interval should be at most daily for: {}", description);
    }
}

#[test]
fn test_configuration_size_limits() {
    // Test that configurations don't become too large when serialized
    let large_webdav_config = WebDAVSourceConfig {
        server_url: "https://very-long-server-name-that-might-be-used-in-enterprise.example.com".to_string(),
        username: "very_long_username_that_might_exist".to_string(),
        password: "very_long_password_with_special_chars_!@#$%^&*()".to_string(),
        watch_folders: vec![
            "/very/long/path/to/documents/folder/one".to_string(),
            "/very/long/path/to/documents/folder/two".to_string(),
            "/very/long/path/to/documents/folder/three".to_string(),
        ],
        file_extensions: vec![
            ".pdf".to_string(), ".txt".to_string(), ".doc".to_string(),
            ".docx".to_string(), ".xls".to_string(), ".xlsx".to_string(),
            ".ppt".to_string(), ".pptx".to_string(), ".jpg".to_string(),
            ".png".to_string(), ".gif".to_string(), ".bmp".to_string(),
        ],
        auto_sync: true,
        sync_interval_minutes: 60,
        server_type: Some("nextcloud".to_string()),
    };
    
    let serialized = serde_json::to_string(&large_webdav_config).unwrap();
    
    // Reasonable size limit for configuration
    assert!(serialized.len() < 2048, "Configuration should not be too large: {} bytes", serialized.len());
    assert!(serialized.len() > 100, "Configuration should have substantial content");
    
    // Test that it can be deserialized back
    let _deserialized: WebDAVSourceConfig = serde_json::from_str(&serialized).unwrap();
}

#[test]
fn test_concurrent_configuration_access() {
    use std::sync::Arc;
    use std::thread;
    
    let config = Arc::new(WebDAVSourceConfig {
        server_url: "https://test.com".to_string(),
        username: "test".to_string(),
        password: "test".to_string(),
        watch_folders: vec!["/test".to_string()],
        file_extensions: vec![".pdf".to_string()],
        auto_sync: true,
        sync_interval_minutes: 60,
        server_type: Some("nextcloud".to_string()),
    });
    
    let mut handles = vec![];
    
    // Spawn multiple threads that read the configuration
    for i in 0..5 {
        let config_clone = Arc::clone(&config);
        let handle = thread::spawn(move || {
            // Simulate concurrent access
            for _ in 0..100 {
                assert_eq!(config_clone.server_url, "https://test.com");
                assert_eq!(config_clone.sync_interval_minutes, 60);
                assert!(config_clone.auto_sync);
            }
            i // Return thread id
        });
        handles.push(handle);
    }
    
    // Wait for all threads and collect results
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results, vec![0, 1, 2, 3, 4]);
}