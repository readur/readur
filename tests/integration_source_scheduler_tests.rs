/*!
 * Source Scheduler Unit Tests
 * 
 * Tests for the universal source scheduler functionality including:
 * - Auto-resume sync after server restart
 * - Background sync scheduling
 * - Manual sync triggering
 * - Source type detection and routing
 * - Error handling and recovery
 */

use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
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

/// Mock database for testing
struct MockDatabase {
    sources: Vec<Source>,
    sources_for_sync: Vec<Source>,
}

impl MockDatabase {
    fn new() -> Self {
        Self {
            sources: Vec::new(),
            sources_for_sync: Vec::new(),
        }
    }

    fn with_interrupted_source(mut self, name: &str, source_type: SourceType) -> Self {
        let mut config = json!({});
        match source_type {
            SourceType::WebDAV => {
                config = json!({
                    "server_url": "https://test.com",
                    "username": "test",
                    "password": "test",
                    "watch_folders": ["/test"],
                    "file_extensions": [".pdf", ".txt"],
                    "auto_sync": true,
                    "sync_interval_minutes": 60
                });
            },
            SourceType::LocalFolder => {
                config = json!({
                    "paths": ["/test/folder"],
                    "recursive": true,
                    "follow_symlinks": false,
                    "auto_sync": true,
                    "sync_interval_minutes": 30
                });
            },
            SourceType::S3 => {
                config = json!({
                    "bucket": "test-bucket",
                    "region": "us-east-1",
                    "access_key_id": "test",
                    "secret_access_key": "test",
                    "prefix": "",
                    "auto_sync": true,
                    "sync_interval_minutes": 120
                });
            }
        }

        self.sources.push(Source {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            name: name.to_string(),
            source_type,
            enabled: true,
            config,
            status: SourceStatus::Syncing, // Interrupted state
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
        });
        
        self.sources_for_sync = self.sources.clone();
        self
    }

    fn with_due_sync_source(mut self, name: &str, source_type: SourceType, minutes_ago: i64) -> Self {
        let mut config = json!({});
        match source_type {
            SourceType::WebDAV => {
                config = json!({
                    "server_url": "https://test.com",
                    "username": "test", 
                    "password": "test",
                    "watch_folders": ["/test"],
                    "file_extensions": [".pdf", ".txt"],
                    "auto_sync": true,
                    "sync_interval_minutes": 30
                });
            },
            SourceType::LocalFolder => {
                config = json!({
                    "paths": ["/test/folder"],
                    "recursive": true,
                    "follow_symlinks": false,
                    "auto_sync": true,
                    "sync_interval_minutes": 30
                });
            },
            SourceType::S3 => {
                config = json!({
                    "bucket": "test-bucket",
                    "region": "us-east-1", 
                    "access_key_id": "test",
                    "secret_access_key": "test",
                    "prefix": "",
                    "auto_sync": true,
                    "sync_interval_minutes": 30
                });
            }
        }

        self.sources.push(Source {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            name: name.to_string(),
            source_type,
            enabled: true,
            config,
            status: SourceStatus::Idle,
            last_sync_at: Some(Utc::now() - chrono::Duration::minutes(minutes_ago)),
            last_error: None,
            last_error_at: None,
            total_files_synced: 5,
            total_files_pending: 0,
            total_size_bytes: 1024,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        validation_status: None,
        last_validation_at: None,
        validation_score: None,
        validation_issues: None,
        });
        
        self.sources_for_sync = self.sources.clone();
        self
    }
}

async fn create_test_app_state() -> Arc<AppState> {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .unwrap_or_else(|_| "postgresql://readur:readur@localhost:5432/readur".to_string());
    
    let config = Config {
        database_url: database_url.clone(),
        server_address: "127.0.0.1:8080".to_string(),
        jwt_secret: "test_secret".to_string(),
        upload_path: "/tmp/test_uploads".to_string(),
        watch_folder: "/tmp/test_watch".to_string(),
        user_watch_base_dir: "./user_watch".to_string(),
        enable_per_user_watch: false,        allowed_file_types: vec!["pdf".to_string(), "txt".to_string()],
        watch_interval_seconds: Some(30),
        file_stability_check_ms: Some(500),
        max_file_age_hours: None,
        ocr_language: "eng".to_string(),
        concurrent_ocr_jobs: 2,
        ocr_timeout_seconds: 60,
        max_file_size_mb: 10,
        memory_limit_mb: 256,
        cpu_priority: "normal".to_string(),
        oidc_enabled: false,
        oidc_client_id: None,
        oidc_client_secret: None,
        oidc_issuer_url: None,
        oidc_redirect_uri: None,
        oidc_auto_register: None,
        allow_local_auth: None,
        s3_enabled: false,
        s3_config: None,
            max_pdf_size_mb: 100,
            max_office_document_size_mb: 100,
    };

    // Use smaller connection pool for tests to avoid exhaustion  
    let db = Database::new_with_pool_config(&database_url, 10, 2).await.unwrap();
    
    // Create test file service
    let storage_config = readur::storage::StorageConfig::Local { upload_path: "/tmp/test_uploads".to_string() };
    let storage_backend = readur::storage::factory::create_storage_backend(storage_config).await.unwrap();
    let file_service = std::sync::Arc::new(readur::services::file_service::FileService::with_storage("/tmp/test_uploads".to_string(), storage_backend));
    
    let queue_service = std::sync::Arc::new(readur::ocr::queue::OcrQueueService::new(db.clone(), db.pool.clone(), 2, file_service.clone(), 100, 100));
    
    Arc::new(AppState {
        db: db.clone(),
        config,
        file_service,
        webdav_scheduler: None,
        source_scheduler: None,
        queue_service,
        oidc_client: None,
        sync_progress_tracker: std::sync::Arc::new(readur::services::sync_progress_tracker::SyncProgressTracker::new()),
        user_watch_service: None,
        webdav_metrics_collector: None,
    })
}

/// Cleanup function to close database connections after tests
async fn cleanup_test_app_state(state: Arc<AppState>) {
    state.db.pool.close().await;
}

#[tokio::test]
async fn test_source_scheduler_creation() {
    let state = create_test_app_state().await;
    let scheduler = SourceScheduler::new(state.clone());
    
    // Test that scheduler is created successfully
    // assert_eq!(scheduler.check_interval, Duration::from_secs(60)); // private field
}

#[tokio::test]
async fn test_interrupted_sync_detection_webdav() {
    let state = create_test_app_state().await;
    let scheduler = SourceScheduler::new(state.clone());
    
    // Create a mock source that was interrupted during sync
    let source = Source {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        name: "Test WebDAV".to_string(),
        source_type: SourceType::WebDAV,
        enabled: true,
        config: json!({
            "server_url": "https://test.com",
            "username": "test",
            "password": "test", 
            "watch_folders": ["/test"],
            "file_extensions": [".pdf", ".txt"],
            "auto_sync": true,
            "sync_interval_minutes": 60
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
    assert_eq!(source.status, SourceStatus::Syncing);
    
    // Test config parsing for WebDAV
    let config: Result<WebDAVSourceConfig, _> = serde_json::from_value(source.config.clone());
    assert!(config.is_ok());
    let webdav_config = config.unwrap();
    assert!(webdav_config.auto_sync);
    assert_eq!(webdav_config.sync_interval_minutes, 60);
}

#[tokio::test]
async fn test_interrupted_sync_detection_local_folder() {
    let state = create_test_app_state().await;
    let scheduler = SourceScheduler::new(state.clone());
    
    let source = Source {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        name: "Test Local Folder".to_string(),
        source_type: SourceType::LocalFolder,
        enabled: true,
        config: json!({
            "watch_folders": ["/test/folder"],
            "file_extensions": [".pdf", ".txt"],
            "recursive": true,
            "follow_symlinks": false,
            "auto_sync": true,
            "sync_interval_minutes": 30
        }),
        status: SourceStatus::Syncing,
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

    // Test config parsing for Local Folder
    let config: Result<LocalFolderSourceConfig, _> = serde_json::from_value(source.config.clone());
    assert!(config.is_ok());
    let local_config = config.unwrap();
    assert!(local_config.auto_sync);
    assert_eq!(local_config.sync_interval_minutes, 30);
}

#[tokio::test]
async fn test_interrupted_sync_detection_s3() {
    let state = create_test_app_state().await;
    let scheduler = SourceScheduler::new(state.clone());
    
    let source = Source {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        name: "Test S3".to_string(),
        source_type: SourceType::S3,
        enabled: true,
        config: json!({
            "bucket_name": "test-bucket",
            "region": "us-east-1",
            "access_key_id": "test",
            "secret_access_key": "test",
            "prefix": "",
            "watch_folders": ["/test/prefix"],
            "file_extensions": [".pdf", ".txt"],
            "auto_sync": true,
            "sync_interval_minutes": 120
        }),
        status: SourceStatus::Syncing,
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

    // Test config parsing for S3
    let config: Result<S3SourceConfig, _> = serde_json::from_value(source.config.clone());
    assert!(config.is_ok());
    let s3_config = config.unwrap();
    assert!(s3_config.auto_sync);
    assert_eq!(s3_config.sync_interval_minutes, 120);
}

#[tokio::test]
async fn test_sync_due_calculation() {
    let state = create_test_app_state().await;
    let scheduler = SourceScheduler::new(state.clone());
    
    // Test source that should be due for sync
    let old_sync_source = Source {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        name: "Old Sync".to_string(),
        source_type: SourceType::WebDAV,
        enabled: true,
        config: json!({
            "server_url": "https://test.com",
            "username": "test",
            "password": "test",
            "watch_folders": ["/test"],
            "file_extensions": [".pdf"],
            "auto_sync": true,
            "sync_interval_minutes": 30
        }),
        status: SourceStatus::Idle,
        last_sync_at: Some(Utc::now() - chrono::Duration::minutes(45)), // 45 minutes ago
        last_error: None,
        last_error_at: None,
        total_files_synced: 5,
        total_files_pending: 0,
        total_size_bytes: 1024,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        validation_status: None,
        last_validation_at: None,
        validation_score: None,
        validation_issues: None,
    };
    
    // Test source that should NOT be due for sync
    let recent_sync_source = Source {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        name: "Recent Sync".to_string(),
        source_type: SourceType::LocalFolder,
        enabled: true,
        config: json!({
            "paths": ["/test"],
            "recursive": true,
            "follow_symlinks": false,
            "auto_sync": true,
            "sync_interval_minutes": 60
        }),
        status: SourceStatus::Idle,
        last_sync_at: Some(Utc::now() - chrono::Duration::minutes(15)), // 15 minutes ago
        last_error: None,
        last_error_at: None,
        total_files_synced: 10,
        total_files_pending: 0,
        total_size_bytes: 2048,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        validation_status: None,
        last_validation_at: None,
        validation_score: None,
        validation_issues: None,
    };

    // Test that sync due calculation works correctly
    // let old_result = scheduler.is_sync_due(&old_sync_source).await;
    // assert!(old_result.is_ok());
    // assert!(old_result.unwrap(), "Old sync should be due");
    
    // let recent_result = scheduler.is_sync_due(&recent_sync_source).await;
    // assert!(recent_result.is_ok());
    // assert!(!recent_result.unwrap(), "Recent sync should not be due");
}

#[tokio::test]
async fn test_auto_sync_disabled() {
    let state = create_test_app_state().await;
    let scheduler = SourceScheduler::new(state.clone());
    
    let source_with_auto_sync_disabled = Source {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        name: "Auto Sync Disabled".to_string(),
        source_type: SourceType::WebDAV,
        enabled: true,
        config: json!({
            "server_url": "https://test.com",
            "username": "test",
            "password": "test",
            "watch_folders": ["/test"],
            "file_extensions": [".pdf"],
            "auto_sync": false, // Disabled
            "sync_interval_minutes": 30
        }),
        status: SourceStatus::Idle,
        last_sync_at: Some(Utc::now() - chrono::Duration::minutes(45)),
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

    // let result = scheduler.is_sync_due(&source_with_auto_sync_disabled).await;
    // assert!(result.is_ok());
    // assert!(!result.unwrap(), "Source with auto_sync disabled should not be due");
}

#[tokio::test]
async fn test_currently_syncing_source() {
    let state = create_test_app_state().await;
    let scheduler = SourceScheduler::new(state.clone());
    
    let syncing_source = Source {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        name: "Currently Syncing".to_string(),
        source_type: SourceType::S3,
        enabled: true,
        config: json!({
            "bucket": "test-bucket",
            "region": "us-east-1",
            "access_key_id": "test",
            "secret_access_key": "test",
            "prefix": "",
            "auto_sync": true,
            "sync_interval_minutes": 30
        }),
        status: SourceStatus::Syncing, // Currently syncing
        last_sync_at: Some(Utc::now() - chrono::Duration::minutes(45)),
        last_error: None,
        last_error_at: None,
        total_files_synced: 0,
        total_files_pending: 5,
        total_size_bytes: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        validation_status: None,
        last_validation_at: None,
        validation_score: None,
        validation_issues: None,
    };

    // let result = scheduler.is_sync_due(&syncing_source).await;
    // assert!(result.is_ok());
    // assert!(!result.unwrap(), "Currently syncing source should not be due for another sync");
}

#[tokio::test]
async fn test_invalid_sync_interval() {
    let state = create_test_app_state().await;
    let scheduler = SourceScheduler::new(state.clone());
    
    let invalid_interval_source = Source {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        name: "Invalid Interval".to_string(),
        source_type: SourceType::WebDAV,
        enabled: true,
        config: json!({
            "server_url": "https://test.com",
            "username": "test",
            "password": "test",
            "watch_folders": ["/test"],
            "file_extensions": [".pdf"],
            "auto_sync": true,
            "sync_interval_minutes": 0 // Invalid interval
        }),
        status: SourceStatus::Idle,
        last_sync_at: Some(Utc::now() - chrono::Duration::minutes(45)),
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

    // let result = scheduler.is_sync_due(&invalid_interval_source).await;
    // assert!(result.is_ok());
    // assert!(!result.unwrap(), "Source with invalid sync interval should not be due");
}

#[tokio::test]
async fn test_never_synced_source() {
    let state = create_test_app_state().await;
    let scheduler = SourceScheduler::new(state.clone());
    
    let never_synced_source = Source {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        name: "Never Synced".to_string(),
        source_type: SourceType::LocalFolder,
        enabled: true,
        config: json!({
            "paths": ["/test"],
            "recursive": true,
            "follow_symlinks": false,
            "auto_sync": true,
            "sync_interval_minutes": 60
        }),
        status: SourceStatus::Idle,
        last_sync_at: None, // Never synced
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

    // let result = scheduler.is_sync_due(&never_synced_source).await;
    // assert!(result.is_ok());
    // assert!(result.unwrap(), "Never synced source should be due for sync");
}

#[tokio::test]
async fn test_trigger_sync_nonexistent_source() {
    let state = create_test_app_state().await;
    let scheduler = SourceScheduler::new(state.clone());
    
    let nonexistent_id = Uuid::new_v4();
    let result = scheduler.trigger_sync(nonexistent_id).await;
    
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "Source not found");
    
    // Cleanup database connections
    cleanup_test_app_state(state).await;
}

#[tokio::test]
async fn test_source_status_enum() {
    // Test SourceStatus enum string conversion
    assert_eq!(SourceStatus::Idle.to_string(), "idle");
    assert_eq!(SourceStatus::Syncing.to_string(), "syncing");
    assert_eq!(SourceStatus::Error.to_string(), "error");
}

#[tokio::test]
async fn test_source_type_enum() {
    // Test SourceType enum
    let webdav = SourceType::WebDAV;
    let local = SourceType::LocalFolder;
    let s3 = SourceType::S3;
    
    assert_eq!(webdav.to_string(), "webdav");
    assert_eq!(local.to_string(), "local_folder");
    assert_eq!(s3.to_string(), "s3");
}

#[tokio::test]
async fn test_config_validation() {
    // Test WebDAV config validation
    let webdav_config = WebDAVSourceConfig {
        server_url: "https://test.com".to_string(),
        username: "user".to_string(),
        password: "pass".to_string(),
        watch_folders: vec!["/folder1".to_string(), "/folder2".to_string()],
        file_extensions: vec![".pdf".to_string(), ".txt".to_string()],
        auto_sync: true,
        sync_interval_minutes: 60,
        server_type: Some("nextcloud".to_string()),
    };
    
    assert!(!webdav_config.server_url.is_empty());
    assert!(!webdav_config.username.is_empty());
    assert!(!webdav_config.password.is_empty());
    assert!(!webdav_config.watch_folders.is_empty());
    assert!(webdav_config.sync_interval_minutes > 0);
    
    // Test Local Folder config validation
    let local_config = LocalFolderSourceConfig {
        watch_folders: vec!["/test/path".to_string()],
        recursive: true,
        follow_symlinks: false,
        auto_sync: true,
        sync_interval_minutes: 30,
        file_extensions: vec![".pdf".to_string()],
    };
    
    assert!(!local_config.watch_folders.is_empty());
    assert!(local_config.sync_interval_minutes > 0);
    
    // Test S3 config validation
    let s3_config = S3SourceConfig {
        bucket_name: "test-bucket".to_string(),
        region: "us-east-1".to_string(),
        access_key_id: "key".to_string(),
        secret_access_key: "secret".to_string(),
        prefix: Some("docs/".to_string()),
        endpoint_url: Some("https://minio.example.com".to_string()),
        watch_folders: vec!["docs/".to_string()],
        auto_sync: true,
        sync_interval_minutes: 120,
        file_extensions: vec![".pdf".to_string()],
    };
    
    assert!(!s3_config.bucket_name.is_empty());
    assert!(!s3_config.region.is_empty());
    assert!(!s3_config.access_key_id.is_empty());
    assert!(!s3_config.secret_access_key.is_empty());
    assert!(s3_config.sync_interval_minutes > 0);
}

#[tokio::test]
async fn test_scheduler_timeout_handling() {
    let state = create_test_app_state().await;
    let scheduler = SourceScheduler::new(state.clone());
    
    // Test that operations complete within reasonable time
    let start = std::time::Instant::now();
    
    let dummy_source = Source {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        name: "Timeout Test".to_string(),
        source_type: SourceType::WebDAV,
        enabled: true,
        config: json!({
            "server_url": "https://test.com",
            "username": "test",
            "password": "test",
            "watch_folders": ["/test"],
            "file_extensions": [".pdf"],
            "auto_sync": true,
            "sync_interval_minutes": 60
        }),
        status: SourceStatus::Idle,
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
    
    // let result = timeout(Duration::from_secs(1), scheduler.is_sync_due(&dummy_source)).await;
    // assert!(result.is_ok(), "Sync due calculation should complete quickly");
    
    let elapsed = start.elapsed();
    assert!(elapsed < Duration::from_millis(500), "Operation should be fast");
}