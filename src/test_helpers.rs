/*!
 * Test Helpers and Utilities
 * 
 * This module provides utilities for creating test configurations and services
 * with sensible defaults. Tests can modify the returned objects as needed.
 */

use crate::{
    config::Config,
    db::Database,
    services::file_service::FileService,
    storage::{StorageConfig, factory::create_storage_backend},
    AppState,
    ocr::queue::OcrQueueService,
    services::sync_progress_tracker::SyncProgressTracker,
};
use std::sync::Arc;
use sqlx::PgPool;

/// Creates a test configuration with sensible defaults
/// All fields are populated to avoid compilation errors when new fields are added
pub fn create_test_config() -> Config {
    Config {
        database_url: std::env::var("TEST_DATABASE_URL")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .unwrap_or_else(|_| "postgresql://readur:readur@localhost:5432/readur".to_string()),
        server_address: "127.0.0.1:0".to_string(),
        jwt_secret: "test_jwt_secret_for_integration_tests".to_string(),
        upload_path: "/tmp/test_uploads".to_string(),
        watch_folder: "/tmp/test_watch".to_string(),
        user_watch_base_dir: "/tmp/user_watch".to_string(),
        enable_per_user_watch: false,
        allowed_file_types: vec!["pdf".to_string(), "png".to_string(), "jpg".to_string(), "txt".to_string()],
        watch_interval_seconds: Some(10),
        file_stability_check_ms: Some(500),
        max_file_age_hours: Some(24),
        
        // OCR Configuration
        ocr_language: "eng".to_string(),
        concurrent_ocr_jobs: 2,
        ocr_timeout_seconds: 60,
        max_file_size_mb: 50,
        
        // Performance
        memory_limit_mb: 256,
        cpu_priority: "normal".to_string(),
        
        // OIDC Configuration (disabled for tests)
        oidc_enabled: false,
        oidc_client_id: None,
        oidc_client_secret: None,
        oidc_issuer_url: None,
        oidc_redirect_uri: None,
        
        // S3 Configuration (disabled for tests by default)
        s3_enabled: false,
        s3_config: None,
    }
}

/// Creates a default test database URL
pub fn default_test_db_url() -> String {
    std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .unwrap_or_else(|_| "postgresql://readur:readur@localhost:5432/readur".to_string())
}

/// Creates a test FileService with local storage
pub async fn create_test_file_service(upload_path: Option<&str>) -> Arc<FileService> {
    let path = upload_path.unwrap_or("/tmp/test_uploads");
    let storage_config = StorageConfig::Local { 
        upload_path: path.to_string() 
    };
    let storage_backend = create_storage_backend(storage_config)
        .await
        .expect("Failed to create test storage backend");
    
    Arc::new(FileService::with_storage(path.to_string(), storage_backend))
}

/// Creates a test Database instance
pub async fn create_test_database() -> Database {
    let database_url = default_test_db_url();
    Database::new(&database_url)
        .await
        .expect("Failed to connect to test database")
}

/// Creates a test Database instance with custom pool configuration
pub async fn create_test_database_with_pool(max_connections: u32, min_connections: u32) -> Database {
    let database_url = default_test_db_url();
    Database::new_with_pool_config(&database_url, max_connections, min_connections)
        .await
        .expect("Failed to connect to test database with custom pool")
}

/// Creates a test OcrQueueService
pub fn create_test_queue_service(db: Database, pool: PgPool, file_service: Arc<FileService>) -> Arc<OcrQueueService> {
    Arc::new(OcrQueueService::new(db, pool, 2, file_service))
}

/// Creates a test AppState with default configuration and services
/// This provides a convenient way to get a fully configured AppState for testing
pub async fn create_test_app_state() -> Arc<AppState> {
    let config = create_test_config();
    create_test_app_state_with_config(config).await
}

/// Creates a test AppState with a custom configuration
/// This allows tests to customize config while still getting properly initialized services
pub async fn create_test_app_state_with_config(config: Config) -> Arc<AppState> {
    let db = create_test_database().await;
    let file_service = create_test_file_service(Some(&config.upload_path)).await;
    let pool = db.pool.clone();
    let queue_service = create_test_queue_service(db.clone(), pool, file_service.clone());
    let sync_progress_tracker = Arc::new(SyncProgressTracker::new());
    
    Arc::new(AppState {
        db,
        config,
        file_service,
        webdav_scheduler: None,
        source_scheduler: None,
        queue_service,
        oidc_client: None,
        sync_progress_tracker,
        user_watch_service: None,
    })
}

/// Creates a test AppState with custom upload path
/// Convenient for tests that need a specific upload directory
pub async fn create_test_app_state_with_upload_path(upload_path: &str) -> Arc<AppState> {
    let mut config = create_test_config();
    config.upload_path = upload_path.to_string();
    create_test_app_state_with_config(config).await
}

/// Creates a test AppState with user watch service enabled
/// Useful for tests that need per-user watch functionality
pub async fn create_test_app_state_with_user_watch(user_watch_base_dir: &str) -> Arc<AppState> {
    let mut config = create_test_config();
    config.enable_per_user_watch = true;
    config.user_watch_base_dir = user_watch_base_dir.to_string();
    
    let db = create_test_database().await;
    let file_service = create_test_file_service(Some(&config.upload_path)).await;
    let pool = db.pool.clone();
    let queue_service = create_test_queue_service(db.clone(), pool, file_service.clone());
    let sync_progress_tracker = Arc::new(SyncProgressTracker::new());
    
    // Create user watch service
    let user_watch_service = Some(Arc::new(crate::services::user_watch_service::UserWatchService::new(
        &config.user_watch_base_dir
    )));
    
    Arc::new(AppState {
        db,
        config,
        file_service,
        webdav_scheduler: None,
        source_scheduler: None,
        queue_service,
        oidc_client: None,
        sync_progress_tracker,
        user_watch_service,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_config() {
        let config = create_test_config();
        assert!(!config.database_url.is_empty());
        assert!(!config.s3_enabled); // Default should be false
        assert!(config.s3_config.is_none()); // Default should be None
        assert!(!config.oidc_enabled); // Default should be false
    }

    #[tokio::test]
    async fn test_create_test_file_service() {
        let file_service = create_test_file_service(None).await;
        // Just verify it was created successfully
        assert!(file_service.as_ref() as *const _ != std::ptr::null());
    }

    #[tokio::test]
    async fn test_create_test_database() {
        let db = create_test_database().await;
        // Just verify it was created successfully
        assert!(db.pool.is_closed() == false);
    }

    #[tokio::test]
    async fn test_create_test_app_state() {
        let state = create_test_app_state().await;
        // Verify all required fields are present
        assert!(!state.config.database_url.is_empty());
        assert!(!state.config.s3_enabled); // Default should be false
        assert!(!state.config.oidc_enabled); // Default should be false
        assert!(state.user_watch_service.is_none()); // Default should be None
    }

    #[tokio::test]
    async fn test_create_test_app_state_with_custom_config() {
        let mut config = create_test_config();
        config.upload_path = "/custom/test/path".to_string();
        config.s3_enabled = true;
        
        let state = create_test_app_state_with_config(config).await;
        assert_eq!(state.config.upload_path, "/custom/test/path");
        assert!(state.config.s3_enabled);
    }

    #[tokio::test]
    async fn test_create_test_app_state_with_user_watch() {
        let state = create_test_app_state_with_user_watch("/tmp/user_watch_test").await;
        assert!(state.config.enable_per_user_watch);
        assert_eq!(state.config.user_watch_base_dir, "/tmp/user_watch_test");
        assert!(state.user_watch_service.is_some());
    }
}