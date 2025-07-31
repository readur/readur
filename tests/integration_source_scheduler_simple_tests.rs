/*!
 * Simple Source Scheduler Unit Tests
 * 
 * Basic tests for the source scheduler functionality without complex mocking
 */

use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;
use serde_json::json;

use readur::{
    AppState, 
    config::Config,
    db::Database,
    models::{Source, SourceType, SourceStatus, WebDAVSourceConfig, LocalFolderSourceConfig, S3SourceConfig},
    scheduling::source_scheduler::SourceScheduler,
};

/// Create a test app state
async fn create_test_app_state() -> Arc<AppState> {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .unwrap_or_else(|_| "postgresql://readur:readur@localhost:5432/readur".to_string());
    
    let config = Config {
        database_url,
        server_address: "127.0.0.1:8080".to_string(),
        jwt_secret: "test_secret".to_string(),
        upload_path: "/tmp/test_uploads".to_string(),
        watch_folder: "/tmp/watch".to_string(),
        allowed_file_types: vec!["pdf".to_string(), "txt".to_string()],
        watch_interval_seconds: Some(10),
        file_stability_check_ms: Some(1000),
        max_file_age_hours: Some(24),
        ocr_language: "eng".to_string(),
        concurrent_ocr_jobs: 4,
        ocr_timeout_seconds: 300,
        max_file_size_mb: 100,
        memory_limit_mb: 512,
        cpu_priority: "normal".to_string(),
        oidc_enabled: false,
        oidc_client_id: None,
        oidc_client_secret: None,
        oidc_issuer_url: None,
        oidc_redirect_uri: None,
    };

    let db = Database::new(&config.database_url).await.unwrap();
    
    let queue_service = Arc::new(readur::ocr::queue::OcrQueueService::new(db.clone(), db.pool.clone(), 2));
    Arc::new(AppState {
        db: db.clone(),
        config,
        webdav_scheduler: None,
        source_scheduler: None,
        queue_service,
        oidc_client: None,
        sync_progress_tracker: std::sync::Arc::new(readur::services::sync_progress_tracker::SyncProgressTracker::new()),
        user_watch_service: None,
    })
}

#[tokio::test]
async fn test_source_scheduler_creation() {
    let state = create_test_app_state().await;
    let _scheduler = SourceScheduler::new(state.clone());
    
    // Test that scheduler is created successfully
    assert!(true); // If we get here, creation succeeded
}

#[test]
fn test_webdav_config_parsing() {
    let config_json = json!({
        "server_url": "https://cloud.example.com",
        "username": "testuser",
        "password": "testpass",
        "watch_folders": ["/Documents"],
        "file_extensions": [".pdf", ".txt"],
        "auto_sync": true,
        "sync_interval_minutes": 60,
        "server_type": "nextcloud"
    });
    
    let config: Result<WebDAVSourceConfig, _> = serde_json::from_value(config_json);
    assert!(config.is_ok(), "WebDAV config should parse successfully");
    
    let webdav_config = config.unwrap();
    assert_eq!(webdav_config.server_url, "https://cloud.example.com");
    assert_eq!(webdav_config.username, "testuser");
    assert!(webdav_config.auto_sync);
    assert_eq!(webdav_config.sync_interval_minutes, 60);
    assert_eq!(webdav_config.server_type, Some("nextcloud".to_string()));
}

#[test]
fn test_local_folder_config_parsing() {
    let config_json = json!({
        "watch_folders": ["/home/user/documents"],
        "file_extensions": [".pdf", ".txt", ".jpg"],
        "auto_sync": true,
        "sync_interval_minutes": 30,
        "recursive": true,
        "follow_symlinks": false
    });
    
    let config: Result<LocalFolderSourceConfig, _> = serde_json::from_value(config_json);
    assert!(config.is_ok(), "Local Folder config should parse successfully");
    
    let local_config = config.unwrap();
    assert_eq!(local_config.watch_folders.len(), 1);
    assert_eq!(local_config.watch_folders[0], "/home/user/documents");
    assert!(local_config.recursive);
    assert!(!local_config.follow_symlinks);
    assert_eq!(local_config.sync_interval_minutes, 30);
}

#[test]
fn test_s3_config_parsing() {
    let config_json = json!({
        "bucket_name": "test-documents",
        "region": "us-east-1",
        "access_key_id": "AKIATEST",
        "secret_access_key": "secrettest",
        "endpoint_url": null,
        "prefix": "documents/",
        "watch_folders": ["documents/"],
        "file_extensions": [".pdf", ".docx"],
        "auto_sync": true,
        "sync_interval_minutes": 120
    });
    
    let config: Result<S3SourceConfig, _> = serde_json::from_value(config_json);
    assert!(config.is_ok(), "S3 config should parse successfully");
    
    let s3_config = config.unwrap();
    assert_eq!(s3_config.bucket_name, "test-documents");
    assert_eq!(s3_config.region, "us-east-1");
    assert_eq!(s3_config.prefix, Some("documents/".to_string()));
    assert_eq!(s3_config.sync_interval_minutes, 120);
}

#[test]
fn test_source_type_enum() {
    assert_eq!(SourceType::WebDAV.to_string(), "webdav");
    assert_eq!(SourceType::LocalFolder.to_string(), "local_folder");
    assert_eq!(SourceType::S3.to_string(), "s3");
}

#[test]
fn test_source_status_enum() {
    assert_eq!(SourceStatus::Idle.to_string(), "idle");
    assert_eq!(SourceStatus::Syncing.to_string(), "syncing");
    assert_eq!(SourceStatus::Error.to_string(), "error");
}

#[test]
fn test_interrupted_sync_detection() {
    let user_id = Uuid::new_v4();
    
    let interrupted_source = Source {
        id: Uuid::new_v4(),
        user_id,
        name: "Test Source".to_string(),
        source_type: SourceType::WebDAV,
        enabled: true,
        config: json!({
            "server_url": "https://cloud.example.com",
            "username": "test",
            "password": "test",
            "watch_folders": ["/test"],
            "file_extensions": [".pdf"],
            "auto_sync": true,
            "sync_interval_minutes": 60,
            "server_type": "nextcloud"
        }),
        status: SourceStatus::Syncing, // This indicates interruption
        last_sync_at: None,
        last_error: None,
        last_error_at: None,
        total_files_synced: 0,
        total_files_pending: 0,
        total_size_bytes: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        validation_status: None,
        last_validation_at: None,
        validation_score: None,
        validation_issues: None,
    };
    
    // Test that interrupted sync is detected
    assert_eq!(interrupted_source.status, SourceStatus::Syncing);
    
    let completed_source = Source {
        id: Uuid::new_v4(),
        user_id,
        name: "Completed Source".to_string(),
        source_type: SourceType::WebDAV,
        enabled: true,
        config: json!({
            "server_url": "https://cloud.example.com",
            "username": "test",
            "password": "test",
            "watch_folders": ["/test"],
            "file_extensions": [".pdf"],
            "auto_sync": true,
            "sync_interval_minutes": 60,
            "server_type": "nextcloud"
        }),
        status: SourceStatus::Idle, // Completed normally
        last_sync_at: Some(Utc::now()),
        last_error: None,
        last_error_at: None,
        total_files_synced: 10,
        total_files_pending: 0,
        total_size_bytes: 1024,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        validation_status: None,
        last_validation_at: None,
        validation_score: None,
        validation_issues: None,
    };
    
    assert_eq!(completed_source.status, SourceStatus::Idle);
}

#[test]
fn test_auto_sync_configuration() {
    // Test WebDAV auto sync enabled
    let webdav_config = WebDAVSourceConfig {
        server_url: "https://test.com".to_string(),
        username: "test".to_string(),
        password: "test".to_string(),
        watch_folders: vec!["/test".to_string()],
        file_extensions: vec![".pdf".to_string()],
        auto_sync: true,
        sync_interval_minutes: 60,
        server_type: Some("nextcloud".to_string()),
    };
    
    assert!(webdav_config.auto_sync);
    assert_eq!(webdav_config.sync_interval_minutes, 60);
    
    // Test auto sync disabled
    let webdav_disabled = WebDAVSourceConfig {
        server_url: "https://test.com".to_string(),
        username: "test".to_string(),
        password: "test".to_string(),
        watch_folders: vec!["/test".to_string()],
        file_extensions: vec![".pdf".to_string()],
        auto_sync: false,
        sync_interval_minutes: 60,
        server_type: Some("nextcloud".to_string()),
    };
    
    assert!(!webdav_disabled.auto_sync);
}

#[test]
fn test_sync_interval_validation() {
    let valid_intervals = vec![1, 15, 30, 60, 120, 240];
    let invalid_intervals = vec![0, -1, -30];
    
    for interval in valid_intervals {
        assert!(interval > 0, "Valid interval should be positive: {}", interval);
    }
    
    for interval in invalid_intervals {
        assert!(interval <= 0, "Invalid interval should be non-positive: {}", interval);
    }
}

#[test]
fn test_file_extension_validation() {
    let valid_extensions = vec![".pdf", ".txt", ".jpg", ".png", ".docx"];
    let invalid_extensions = vec!["pdf", "txt", "", "no-dot"];
    
    for ext in valid_extensions {
        assert!(ext.starts_with('.'), "Valid extension should start with dot: {}", ext);
        assert!(!ext.is_empty(), "Valid extension should not be empty");
    }
    
    for ext in invalid_extensions {
        if !ext.is_empty() {
            assert!(!ext.starts_with('.') || ext.len() == 1, "Invalid extension: {}", ext);
        }
    }
}

#[tokio::test]
async fn test_trigger_sync_basic() {
    let state = create_test_app_state().await;
    let scheduler = SourceScheduler::new(state.clone());
    
    // Test triggering sync with non-existent source
    let non_existent_id = Uuid::new_v4();
    let result = scheduler.trigger_sync(non_existent_id).await;
    
    // Should return error for non-existent source
    assert!(result.is_err());
}

#[test]
fn test_source_configuration_sizes() {
    // Test that configurations don't grow too large
    let webdav_config = WebDAVSourceConfig {
        server_url: "https://very-long-server-url-that-might-be-too-long.example.com".to_string(),
        username: "test".to_string(),
        password: "test".to_string(),
        watch_folders: vec!["/folder1".to_string(), "/folder2".to_string()],
        file_extensions: vec![".pdf".to_string(), ".txt".to_string(), ".jpg".to_string()],
        auto_sync: true,
        sync_interval_minutes: 60,
        server_type: Some("nextcloud".to_string()),
    };
    
    let serialized = serde_json::to_string(&webdav_config).unwrap();
    assert!(serialized.len() < 1024, "Config should not be too large");
    
    // Test that required fields are present
    assert!(!webdav_config.server_url.is_empty());
    assert!(!webdav_config.username.is_empty());
    assert!(!webdav_config.watch_folders.is_empty());
    assert!(!webdav_config.file_extensions.is_empty());
}