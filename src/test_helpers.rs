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
use anyhow::Result;
use std::sync::Arc;
use sqlx::PgPool;

/// Error type for test helper operations
#[derive(Debug, thiserror::Error)]
pub enum TestHelperError {
    #[error("Database connection failed: {0}")]
    DatabaseConnection(String),
    
    #[error("Storage backend creation failed: {0}")]
    StorageBackend(String),
    
    #[error("Service initialization failed: {message}")]
    ServiceInitialization { message: String },
    
    #[error("Configuration error: {message}")]
    Configuration { message: String },
}

/// Options for creating a test AppState with customizable fields
#[derive(Debug, Clone)]
pub struct TestAppStateOptions {
    /// Custom database URL (defaults to TEST_DATABASE_URL or DATABASE_URL)
    pub database_url: Option<String>,
    
    /// Upload path for file storage (defaults to "/tmp/test_uploads")
    pub upload_path: Option<String>,
    
    /// Watch folder path (defaults to "/tmp/test_watch") 
    pub watch_folder: Option<String>,
    
    /// User watch base directory (defaults to "/tmp/user_watch")
    pub user_watch_base_dir: Option<String>,
    
    /// Enable per-user watch functionality (defaults to false)
    pub enable_per_user_watch: Option<bool>,
    
    /// Number of concurrent OCR jobs (defaults to 2 for tests)
    pub concurrent_ocr_jobs: Option<usize>,
    
    /// OCR timeout in seconds (defaults to 60)
    pub ocr_timeout_seconds: Option<u64>,
    
    /// Maximum file size in MB (defaults to 50)
    pub max_file_size_mb: Option<u64>,
    
    /// Enable S3 storage (defaults to false)
    pub s3_enabled: Option<bool>,
    
    /// S3 configuration (defaults to None)
    pub s3_config: Option<crate::models::S3SourceConfig>,
    
    /// Enable OIDC authentication (defaults to false)
    pub oidc_enabled: Option<bool>,
    
    /// OIDC client ID (defaults to None)
    pub oidc_client_id: Option<String>,
    
    /// Database pool max connections (defaults to 5 for tests)
    pub db_max_connections: Option<u32>,
    
    /// Database pool min connections (defaults to 1 for tests)
    pub db_min_connections: Option<u32>,
    
    /// Allowed file types (defaults to common test types)
    pub allowed_file_types: Option<Vec<String>>,
    
    /// OCR language (defaults to "eng")
    pub ocr_language: Option<String>,
    
    /// Memory limit in MB (defaults to 256 for tests)
    pub memory_limit_mb: Option<usize>,
}

impl Default for TestAppStateOptions {
    fn default() -> Self {
        Self {
            database_url: None,
            upload_path: None,
            watch_folder: None,
            user_watch_base_dir: None,
            enable_per_user_watch: None,
            concurrent_ocr_jobs: None,
            ocr_timeout_seconds: None,
            max_file_size_mb: None,
            s3_enabled: None,
            s3_config: None,
            oidc_enabled: None,
            oidc_client_id: None,
            db_max_connections: None,
            db_min_connections: None,
            allowed_file_types: None,
            ocr_language: None,
            memory_limit_mb: None,
        }
    }
}

impl TestAppStateOptions {
    /// Create new options with default values
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set database URL
    pub fn with_database_url<S: Into<String>>(mut self, url: S) -> Self {
        self.database_url = Some(url.into());
        self
    }
    
    /// Set upload path
    pub fn with_upload_path<S: Into<String>>(mut self, path: S) -> Self {
        self.upload_path = Some(path.into());
        self
    }
    
    /// Set watch folder
    pub fn with_watch_folder<S: Into<String>>(mut self, path: S) -> Self {
        self.watch_folder = Some(path.into());
        self
    }
    
    /// Enable per-user watch with base directory
    pub fn with_user_watch<S: Into<String>>(mut self, base_dir: S) -> Self {
        self.user_watch_base_dir = Some(base_dir.into());
        self.enable_per_user_watch = Some(true);
        self
    }
    
    /// Set concurrent OCR jobs
    pub fn with_concurrent_ocr_jobs(mut self, jobs: usize) -> Self {
        self.concurrent_ocr_jobs = Some(jobs);
        self
    }
    
    /// Enable S3 storage with config
    pub fn with_s3_config(mut self, config: crate::models::S3SourceConfig) -> Self {
        self.s3_config = Some(config);
        self.s3_enabled = Some(true);
        self
    }
    
    /// Enable OIDC with client ID
    pub fn with_oidc<S: Into<String>>(mut self, client_id: S) -> Self {
        self.oidc_client_id = Some(client_id.into());
        self.oidc_enabled = Some(true);
        self
    }
    
    /// Set database pool size
    pub fn with_db_pool_size(mut self, max_connections: u32, min_connections: u32) -> Self {
        self.db_max_connections = Some(max_connections);
        self.db_min_connections = Some(min_connections);
        self
    }
}

/// Creates a test configuration with sensible defaults
/// All fields are populated to avoid compilation errors when new fields are added
pub fn create_test_config() -> Config {
    Config {
        database_url: default_test_db_url(),
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

/// Creates a test configuration from options
pub fn create_test_config_from_options(options: &TestAppStateOptions) -> Config {
    let mut config = create_test_config();
    
    // Apply options overrides
    if let Some(ref database_url) = options.database_url {
        config.database_url = database_url.clone();
    }
    
    if let Some(ref upload_path) = options.upload_path {
        config.upload_path = upload_path.clone();
    }
    
    if let Some(ref watch_folder) = options.watch_folder {
        config.watch_folder = watch_folder.clone();
    }
    
    if let Some(ref user_watch_base_dir) = options.user_watch_base_dir {
        config.user_watch_base_dir = user_watch_base_dir.clone();
    }
    
    if let Some(enable_per_user_watch) = options.enable_per_user_watch {
        config.enable_per_user_watch = enable_per_user_watch;
    }
    
    if let Some(concurrent_ocr_jobs) = options.concurrent_ocr_jobs {
        config.concurrent_ocr_jobs = concurrent_ocr_jobs as usize;
    }
    
    if let Some(ocr_timeout_seconds) = options.ocr_timeout_seconds {
        config.ocr_timeout_seconds = ocr_timeout_seconds;
    }
    
    if let Some(max_file_size_mb) = options.max_file_size_mb {
        config.max_file_size_mb = max_file_size_mb;
    }
    
    if let Some(s3_enabled) = options.s3_enabled {
        config.s3_enabled = s3_enabled;
    }
    
    if let Some(ref s3_config) = options.s3_config {
        config.s3_config = Some(s3_config.clone());
        config.s3_enabled = true; // Automatically enable if config provided
    }
    
    if let Some(oidc_enabled) = options.oidc_enabled {
        config.oidc_enabled = oidc_enabled;
    }
    
    if let Some(ref oidc_client_id) = options.oidc_client_id {
        config.oidc_client_id = Some(oidc_client_id.clone());
        config.oidc_enabled = true; // Automatically enable if client ID provided
    }
    
    if let Some(ref allowed_file_types) = options.allowed_file_types {
        config.allowed_file_types = allowed_file_types.clone();
    }
    
    if let Some(ref ocr_language) = options.ocr_language {
        config.ocr_language = ocr_language.clone();
    }
    
    if let Some(memory_limit_mb) = options.memory_limit_mb {
        config.memory_limit_mb = memory_limit_mb as usize;
    }
    
    config
}

/// Creates a default test database URL
pub fn default_test_db_url() -> String {
    std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .unwrap_or_else(|_| "postgresql://readur:readur@localhost:5432/readur".to_string())
}

/// Creates a test FileService with local storage
pub async fn create_test_file_service(upload_path: Option<&str>) -> Result<Arc<FileService>, TestHelperError> {
    let path = upload_path.unwrap_or("/tmp/test_uploads");
    let storage_config = StorageConfig::Local { 
        upload_path: path.to_string() 
    };
    let storage_backend = create_storage_backend(storage_config)
        .await
        .map_err(|e| TestHelperError::StorageBackend(e.to_string()))?;
    
    Ok(Arc::new(FileService::with_storage(path.to_string(), storage_backend)))
}

/// Creates a test Database instance with test-optimized pool settings
/// Uses smaller connection pools and shorter timeouts suitable for testing
pub async fn create_test_database() -> Result<Database, TestHelperError> {
    let database_url = default_test_db_url();
    
    // Use test-optimized pool settings: smaller pools, faster timeouts
    Database::new_with_pool_config(&database_url, 5, 1)
        .await
        .map_err(|e| TestHelperError::DatabaseConnection(e.to_string()))
}

/// Creates a test Database instance with custom pool configuration
/// This version allows tests to specify their own pool settings
pub async fn create_test_database_with_pool(max_connections: u32, min_connections: u32) -> Result<Database, TestHelperError> {
    let database_url = default_test_db_url();
    Database::new_with_pool_config(&database_url, max_connections, min_connections)
        .await
        .map_err(|e| TestHelperError::DatabaseConnection(e.to_string()))
}

/// Creates a test Database instance from options
pub async fn create_test_database_from_options(options: &TestAppStateOptions) -> Result<Database, TestHelperError> {
    let default_url = default_test_db_url();
    let database_url = options.database_url.as_deref().unwrap_or(&default_url);
    let max_connections = options.db_max_connections.unwrap_or(5);
    let min_connections = options.db_min_connections.unwrap_or(1);
    
    Database::new_with_pool_config(database_url, max_connections, min_connections)
        .await
        .map_err(|e| TestHelperError::DatabaseConnection(e.to_string()))
}

/// Creates a test OcrQueueService with proper error handling
pub fn create_test_queue_service(db: Database, pool: PgPool, concurrent_jobs: usize, file_service: Arc<FileService>) -> Result<Arc<OcrQueueService>, TestHelperError> {
    Ok(Arc::new(OcrQueueService::new(db, pool, concurrent_jobs, file_service)))
}

/// Creates a test AppState with default configuration and services
/// This provides a convenient way to get a fully configured AppState for testing
pub async fn create_test_app_state() -> Result<Arc<AppState>, TestHelperError> {
    let options = TestAppStateOptions::default();
    create_test_app_state_with_options(options).await
}

/// Creates a test AppState with a custom configuration
/// This allows tests to customize config while still getting properly initialized services
/// DEPRECATED: Use create_test_app_state_with_options instead for better flexibility
pub async fn create_test_app_state_with_config(config: Config) -> Result<Arc<AppState>, TestHelperError> {
    let db = create_test_database().await?;
    let file_service = create_test_file_service(Some(&config.upload_path)).await?;
    let pool = db.pool.clone();
    let queue_service = create_test_queue_service(db.clone(), pool, config.concurrent_ocr_jobs, file_service.clone())?;
    let sync_progress_tracker = Arc::new(SyncProgressTracker::new());
    
    Ok(Arc::new(AppState {
        db,
        config,
        file_service,
        webdav_scheduler: None,
        source_scheduler: None,
        queue_service,
        oidc_client: None,
        webdav_metrics_collector: None,
        sync_progress_tracker,
        user_watch_service: None,
    }))
}

/// Creates a test AppState with customizable options
/// This is the recommended way to create test AppState instances
pub async fn create_test_app_state_with_options(options: TestAppStateOptions) -> Result<Arc<AppState>, TestHelperError> {
    let config = create_test_config_from_options(&options);
    let db = create_test_database_from_options(&options).await?;
    let file_service = create_test_file_service(Some(&config.upload_path)).await?;
    let pool = db.pool.clone();
    let queue_service = create_test_queue_service(
        db.clone(), 
        pool, 
        config.concurrent_ocr_jobs,
        file_service.clone()
    )?;
    let sync_progress_tracker = Arc::new(SyncProgressTracker::new());
    
    // Create user watch service if enabled
    let user_watch_service = if config.enable_per_user_watch {
        Some(Arc::new(
            crate::services::user_watch_service::UserWatchService::new(&config.user_watch_base_dir)
        ))
    } else {
        None
    };
    
    Ok(Arc::new(AppState {
        db,
        config,
        file_service,
        webdav_scheduler: None,
        source_scheduler: None,
        queue_service,
        webdav_metrics_collector: None,
        oidc_client: None,
        sync_progress_tracker,
        user_watch_service,
    }))
}

/// Creates a test AppState with custom upload path
/// Convenient for tests that need a specific upload directory
pub async fn create_test_app_state_with_upload_path(upload_path: &str) -> Result<Arc<AppState>, TestHelperError> {
    let options = TestAppStateOptions::new()
        .with_upload_path(upload_path);
    create_test_app_state_with_options(options).await
}

/// Creates a test AppState with user watch service enabled
/// Useful for tests that need per-user watch functionality
pub async fn create_test_app_state_with_user_watch(user_watch_base_dir: &str) -> Result<Arc<AppState>, TestHelperError> {
    let options = TestAppStateOptions::new()
        .with_user_watch(user_watch_base_dir);
    create_test_app_state_with_options(options).await
}

/// Backward compatibility wrapper that panics on error (to maintain existing test compatibility)
/// DEPRECATED: Tests should migrate to use the Result-returning versions
pub async fn create_test_app_state_legacy() -> Arc<AppState> {
    create_test_app_state().await
        .expect("Failed to create test app state - check database connection")
}

/// Backward compatibility wrapper that panics on error
/// DEPRECATED: Tests should migrate to use the Result-returning versions  
pub async fn create_test_app_state_with_config_legacy(config: Config) -> Arc<AppState> {
    create_test_app_state_with_config(config).await
        .expect("Failed to create test app state with config - check database connection")
}

/// Backward compatibility wrapper that panics on error
/// DEPRECATED: Tests should migrate to use the Result-returning versions
pub async fn create_test_app_state_with_upload_path_legacy(upload_path: &str) -> Arc<AppState> {
    create_test_app_state_with_upload_path(upload_path).await
        .expect("Failed to create test app state with upload path - check database connection")
}

/// Backward compatibility wrapper that panics on error
/// DEPRECATED: Tests should migrate to use the Result-returning versions
pub async fn create_test_app_state_with_user_watch_legacy(user_watch_base_dir: &str) -> Arc<AppState> {
    create_test_app_state_with_user_watch(user_watch_base_dir).await
        .expect("Failed to create test app state with user watch - check database connection")
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

    #[test]
    fn test_test_app_state_options_builder() {
        let options = TestAppStateOptions::new()
            .with_upload_path("/test/uploads")
            .with_concurrent_ocr_jobs(4)
            .with_db_pool_size(10, 2);
        
        assert_eq!(options.upload_path, Some("/test/uploads".to_string()));
        assert_eq!(options.concurrent_ocr_jobs, Some(4));
        assert_eq!(options.db_max_connections, Some(10));
        assert_eq!(options.db_min_connections, Some(2));
    }

    #[test]
    fn test_create_test_config_from_options() {
        let options = TestAppStateOptions::new()
            .with_upload_path("/custom/uploads")
            .with_concurrent_ocr_jobs(8)
            .with_oidc("test-client-id");
        
        let config = create_test_config_from_options(&options);
        assert_eq!(config.upload_path, "/custom/uploads");
        assert_eq!(config.concurrent_ocr_jobs, 8);
        assert!(config.oidc_enabled);
        assert_eq!(config.oidc_client_id, Some("test-client-id".to_string()));
    }

    #[tokio::test]
    async fn test_create_test_file_service() {
        let file_service = create_test_file_service(None).await;
        // Just verify it was created successfully
        match file_service {
            Ok(service) => assert!(service.as_ref() as *const _ != std::ptr::null()),
            Err(e) => panic!("Failed to create file service: {}", e),
        }
    }

    #[tokio::test]
    async fn test_create_test_database() {
        match create_test_database().await {
            Ok(db) => {
                assert!(!db.pool.is_closed());
                // Verify it uses test-optimized settings - size() returns the actual current size, not max
                // The pool starts with min_connections (1) and grows up to max_connections (5) as needed
                let pool_size = db.pool.size();
                println!("Database pool size: {}", pool_size);
                assert!(pool_size >= 1 && pool_size <= 5, "Pool size should be between 1 and 5, got {}", pool_size);
            },
            Err(TestHelperError::DatabaseConnection(_)) => {
                // This is expected in environments without a test database
                println!("Database connection failed - this is expected in CI without a test database");
            },
            Err(e) => panic!("Unexpected error creating database: {}", e),
        }
    }

    #[tokio::test]
    async fn test_create_test_app_state_with_options() {
        let temp_dir = std::env::temp_dir().join("test_custom_uploads");
        let temp_path = temp_dir.to_string_lossy().to_string();
        
        let options = TestAppStateOptions::new()
            .with_upload_path(&temp_path)
            .with_concurrent_ocr_jobs(3)
            .with_db_pool_size(3, 1);
        
        match create_test_app_state_with_options(options).await {
            Ok(state) => {
                assert_eq!(state.config.upload_path, temp_path);
                assert_eq!(state.config.concurrent_ocr_jobs, 3);
                assert!(!state.config.s3_enabled); // Default should be false
                assert!(!state.config.oidc_enabled); // Default should be false
                assert!(state.user_watch_service.is_none()); // Default should be None
            },
            Err(TestHelperError::DatabaseConnection(_)) => {
                // This is expected in environments without a test database
                println!("Database connection failed - this is expected in CI without a test database");
            },
            Err(e) => panic!("Unexpected error creating app state: {}", e),
        }
    }

    #[tokio::test]
    async fn test_create_test_app_state_with_user_watch() {
        let options = TestAppStateOptions::new()
            .with_user_watch("/tmp/user_watch_test");
        
        match create_test_app_state_with_options(options).await {
            Ok(state) => {
                assert!(state.config.enable_per_user_watch);
                assert_eq!(state.config.user_watch_base_dir, "/tmp/user_watch_test");
                assert!(state.user_watch_service.is_some());
            },
            Err(TestHelperError::DatabaseConnection(_)) => {
                // This is expected in environments without a test database
                println!("Database connection failed - this is expected in CI without a test database");
            },
            Err(e) => panic!("Unexpected error creating app state with user watch: {}", e),
        }
    }

    #[tokio::test]
    async fn test_backward_compatibility_functions() {
        // Test that the old API still works for existing code
        match create_test_app_state_with_upload_path("/tmp/compat/test").await {
            Ok(state) => {
                assert_eq!(state.config.upload_path, "/tmp/compat/test");
            },
            Err(TestHelperError::DatabaseConnection(_)) => {
                // This is expected in environments without a test database
                println!("Database connection failed - this is expected in CI without a test database");
            },
            Err(e) => panic!("Unexpected error in backward compatibility test: {}", e),
        }
    }

}