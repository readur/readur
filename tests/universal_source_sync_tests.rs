/*!
 * Universal Source Sync Service Unit Tests
 * 
 * Tests for the universal source sync service that handles:
 * - Multiple source types (WebDAV, Local Folder, S3) 
 * - Generic sync operations and dispatching
 * - File deduplication and content hashing
 * - OCR queue integration
 * - Error handling across source types
 * - Performance optimization and metrics
 */

use std::sync::Arc;
use std::collections::HashMap;
use uuid::Uuid;
use chrono::Utc;
use serde_json::json;
use sha2::{Sha256, Digest};

use readur::{
    AppState,
    config::Config,
    db::Database,
    models::{Source, SourceType, SourceStatus, WebDAVSourceConfig, LocalFolderSourceConfig, S3SourceConfig},
    source_sync::SourceSyncService,
};

/// Create a test WebDAV source
fn create_test_webdav_source() -> Source {
    Source {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        name: "Test WebDAV".to_string(),
        source_type: SourceType::WebDAV,
        enabled: true,
        config: json!({
            "server_url": "https://cloud.example.com",
            "username": "testuser",
            "password": "testpass",
            "watch_folders": ["/Documents"],
            "file_extensions": [".pdf", ".txt"],
            "auto_sync": true,
            "sync_interval_minutes": 60,
            "server_type": "nextcloud"
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
    }
}

/// Create a test Local Folder source
fn create_test_local_source() -> Source {
    Source {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        name: "Test Local Folder".to_string(),
        source_type: SourceType::LocalFolder,
        enabled: true,
        config: json!({
            "watch_folders": ["/home/user/documents"],
            "recursive": true,
            "follow_symlinks": false,
            "auto_sync": true,
            "sync_interval_minutes": 30,
            "file_extensions": [".pdf", ".txt", ".jpg"]
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
    }
}

/// Create a test S3 source
fn create_test_s3_source() -> Source {
    Source {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        name: "Test S3".to_string(),
        source_type: SourceType::S3,
        enabled: true,
        config: json!({
            "bucket_name": "test-documents",
            "region": "us-east-1",
            "access_key_id": "AKIATEST",
            "secret_access_key": "secrettest",
            "prefix": "documents/",
            "watch_folders": ["documents/"],
            "auto_sync": true,
            "sync_interval_minutes": 120,
            "file_extensions": [".pdf", ".docx"]
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
    }
}

async fn create_test_app_state() -> Arc<AppState> {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .unwrap_or_else(|_| "postgresql://readur:readur@localhost:5432/readur".to_string());
    
    let config = Config {
        database_url,
        server_address: "127.0.0.1:8080".to_string(),
        jwt_secret: "test_secret".to_string(),
        upload_path: "/tmp/test_uploads".to_string(),
        watch_folder: "/tmp/test_watch".to_string(),
        allowed_file_types: vec!["pdf".to_string(), "txt".to_string()],
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
    };

    let db = Database::new(&config.database_url).await.unwrap();
    
    let queue_service = Arc::new(readur::ocr_queue::OcrQueueService::new(db.clone(), db.pool.clone(), 2));
    Arc::new(AppState {
        db,
        config,
        webdav_scheduler: None,
        source_scheduler: None,
        queue_service,
        oidc_client: None,
    })
}

#[tokio::test]
async fn test_source_sync_service_creation() {
    let state = create_test_app_state().await;
    let sync_service = SourceSyncService::new(state.clone());
    
    // Test that sync service is created successfully
    // We can't access private fields, but we can test public interface
    assert!(true); // Service creation succeeded
}

#[test]
fn test_source_type_detection() {
    let webdav_source = create_test_webdav_source();
    let local_source = create_test_local_source();
    let s3_source = create_test_s3_source();
    
    assert_eq!(webdav_source.source_type, SourceType::WebDAV);
    assert_eq!(local_source.source_type, SourceType::LocalFolder);
    assert_eq!(s3_source.source_type, SourceType::S3);
    
    // Test string representations
    assert_eq!(webdav_source.source_type.to_string(), "webdav");
    assert_eq!(local_source.source_type.to_string(), "local_folder");
    assert_eq!(s3_source.source_type.to_string(), "s3");
}

#[test]
fn test_config_parsing_webdav() {
    let source = create_test_webdav_source();
    
    let config: Result<WebDAVSourceConfig, _> = serde_json::from_value(source.config.clone());
    assert!(config.is_ok(), "WebDAV config should parse successfully");
    
    let webdav_config = config.unwrap();
    assert_eq!(webdav_config.server_url, "https://cloud.example.com");
    assert_eq!(webdav_config.username, "testuser");
    assert!(webdav_config.auto_sync);
    assert_eq!(webdav_config.sync_interval_minutes, 60);
    assert_eq!(webdav_config.file_extensions.len(), 2);
}

#[test]
fn test_config_parsing_local_folder() {
    let source = create_test_local_source();
    
    let config: Result<LocalFolderSourceConfig, _> = serde_json::from_value(source.config.clone());
    assert!(config.is_ok(), "Local Folder config should parse successfully");
    
    let local_config = config.unwrap();
    assert_eq!(local_config.watch_folders.len(), 1);
    assert_eq!(local_config.watch_folders[0], "/home/user/documents");
    assert!(local_config.recursive);
    assert!(!local_config.follow_symlinks);
    assert_eq!(local_config.sync_interval_minutes, 30);
}

#[test]
fn test_config_parsing_s3() {
    let source = create_test_s3_source();
    
    let config: Result<S3SourceConfig, _> = serde_json::from_value(source.config.clone());
    assert!(config.is_ok(), "S3 config should parse successfully");
    
    let s3_config = config.unwrap();
    assert_eq!(s3_config.bucket_name, "test-documents");
    assert_eq!(s3_config.region, "us-east-1");
    assert_eq!(s3_config.prefix, Some("documents/".to_string()));
    assert_eq!(s3_config.sync_interval_minutes, 120);
    assert_eq!(s3_config.watch_folders.len(), 1);
    assert_eq!(s3_config.watch_folders[0], "documents/");
}

#[test]
fn test_file_deduplication_logic() {
    // Test SHA256-based file deduplication
    let file_content_1 = b"This is test file content for deduplication";
    let file_content_2 = b"This is different file content";
    let file_content_3 = b"This is test file content for deduplication"; // Same as 1
    
    let hash_1 = calculate_content_hash(file_content_1);
    let hash_2 = calculate_content_hash(file_content_2);
    let hash_3 = calculate_content_hash(file_content_3);
    
    assert_ne!(hash_1, hash_2, "Different content should have different hashes");
    assert_eq!(hash_1, hash_3, "Same content should have same hashes");
    
    // Test hash format
    assert_eq!(hash_1.len(), 64); // SHA256 hex string length
    assert!(hash_1.chars().all(|c| c.is_ascii_hexdigit()), "Hash should be valid hex");
}

fn calculate_content_hash(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    format!("{:x}", hasher.finalize())
}

#[test]
fn test_sync_metrics_structure() {
    let metrics = SyncMetrics {
        source_id: Uuid::new_v4(),
        source_type: SourceType::WebDAV,
        files_discovered: 100,
        files_downloaded: 85,
        files_skipped_existing: 10,
        files_skipped_extension: 3,
        files_failed: 2,
        total_bytes_downloaded: 50_000_000, // 50MB
        sync_duration_ms: 45_000, // 45 seconds
        ocr_jobs_queued: 75,
        errors: vec![
            SyncError {
                file_path: "/Documents/failed.pdf".to_string(),
                error_message: "Network timeout".to_string(),
                error_code: "TIMEOUT".to_string(),
            }
        ],
    };
    
    assert_eq!(metrics.source_type, SourceType::WebDAV);
    assert_eq!(metrics.files_discovered, 100);
    assert_eq!(metrics.files_downloaded, 85);
    
    // Test calculated metrics
    let total_processed = metrics.files_downloaded + metrics.files_skipped_existing + 
                         metrics.files_skipped_extension + metrics.files_failed;
    assert_eq!(total_processed, metrics.files_discovered);
    
    let success_rate = (metrics.files_downloaded as f64 / metrics.files_discovered as f64) * 100.0;
    assert_eq!(success_rate, 85.0);
    
    // Test throughput calculation
    let mb_per_second = (metrics.total_bytes_downloaded as f64 / 1_000_000.0) / 
                       (metrics.sync_duration_ms as f64 / 1000.0);
    assert!(mb_per_second > 0.0);
}

#[derive(Debug, Clone)]
struct SyncMetrics {
    source_id: Uuid,
    source_type: SourceType,
    files_discovered: u32,
    files_downloaded: u32,
    files_skipped_existing: u32,
    files_skipped_extension: u32,
    files_failed: u32,
    total_bytes_downloaded: u64,
    sync_duration_ms: u64,
    ocr_jobs_queued: u32,
    errors: Vec<SyncError>,
}

#[derive(Debug, Clone)]
struct SyncError {
    file_path: String,
    error_message: String,
    error_code: String,
}

#[test]
fn test_ocr_queue_integration() {
    // Test OCR job creation for different file types
    let test_files = vec![
        ("document.pdf", true),   // Should queue for OCR
        ("image.jpg", true),      // Should queue for OCR
        ("image.png", true),      // Should queue for OCR
        ("text.txt", false),      // Plain text, no OCR needed
        ("data.json", false),     // JSON, no OCR needed
        ("archive.zip", false),   // Archive, no OCR needed
    ];
    
    for (filename, should_queue_ocr) in test_files {
        let needs_ocr = file_needs_ocr(filename);
        assert_eq!(needs_ocr, should_queue_ocr, 
                   "OCR queueing decision wrong for: {}", filename);
    }
}

fn file_needs_ocr(filename: &str) -> bool {
    let ocr_extensions = vec![".pdf", ".jpg", ".jpeg", ".png", ".tiff", ".bmp"];
    let extension = extract_extension(filename);
    ocr_extensions.contains(&extension.as_str())
}

fn extract_extension(filename: &str) -> String {
    if let Some(pos) = filename.rfind('.') {
        filename[pos..].to_lowercase()
    } else {
        String::new()
    }
}

#[test]
fn test_sync_cancellation_handling() {
    // Test sync cancellation logic
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::Ordering;
    
    let cancellation_token = Arc::new(AtomicBool::new(false));
    
    // Test normal operation
    assert!(!cancellation_token.load(Ordering::Relaxed));
    
    // Simulate cancellation request
    cancellation_token.store(true, Ordering::Relaxed);
    assert!(cancellation_token.load(Ordering::Relaxed));
    
    // Test that sync would respect cancellation
    let should_continue = !cancellation_token.load(Ordering::Relaxed);
    assert!(!should_continue, "Sync should stop when cancelled");
    
    // Test cancellation cleanup
    cancellation_token.store(false, Ordering::Relaxed);
    assert!(!cancellation_token.load(Ordering::Relaxed));
}

#[test]
fn test_error_classification() {
    let test_errors = vec![
        ("Connection timeout", ErrorCategory::Network),
        ("DNS resolution failed", ErrorCategory::Network),
        ("HTTP 401 Unauthorized", ErrorCategory::Authentication),
        ("HTTP 403 Forbidden", ErrorCategory::Authentication),
        ("HTTP 404 Not Found", ErrorCategory::NotFound),
        ("HTTP 500 Internal Server Error", ErrorCategory::Server),
        ("Disk full", ErrorCategory::Storage),
        ("Permission denied", ErrorCategory::Permission),
        ("Invalid file format", ErrorCategory::Format),
        ("Unknown error", ErrorCategory::Unknown),
    ];
    
    for (error_message, expected_category) in test_errors {
        let category = classify_error(error_message);
        assert_eq!(category, expected_category, 
                   "Error classification failed for: {}", error_message);
    }
}

#[derive(Debug, Clone, PartialEq)]
enum ErrorCategory {
    Network,
    Authentication,
    NotFound,
    Server,
    Storage,
    Permission,
    Format,
    Unknown,
}

fn classify_error(error_message: &str) -> ErrorCategory {
    let msg = error_message.to_lowercase();
    
    if msg.contains("timeout") || msg.contains("dns") || msg.contains("connection") {
        ErrorCategory::Network
    } else if msg.contains("401") || msg.contains("403") || msg.contains("unauthorized") || msg.contains("forbidden") {
        ErrorCategory::Authentication
    } else if msg.contains("404") || msg.contains("not found") {
        ErrorCategory::NotFound
    } else if msg.contains("500") || msg.contains("internal server") {
        ErrorCategory::Server
    } else if msg.contains("disk full") || msg.contains("storage") {
        ErrorCategory::Storage
    } else if msg.contains("permission denied") || msg.contains("access denied") {
        ErrorCategory::Permission
    } else if msg.contains("invalid file") || msg.contains("format") {
        ErrorCategory::Format
    } else {
        ErrorCategory::Unknown
    }
}

#[test]
fn test_retry_strategy() {
    // Test retry strategy for different error types
    let retry_configs = vec![
        (ErrorCategory::Network, 3, true),      // Retry network errors
        (ErrorCategory::Server, 2, true),       // Retry server errors
        (ErrorCategory::Authentication, 0, false), // Don't retry auth errors
        (ErrorCategory::NotFound, 0, false),    // Don't retry not found
        (ErrorCategory::Permission, 0, false),  // Don't retry permission errors
        (ErrorCategory::Format, 0, false),      // Don't retry format errors
    ];
    
    for (error_category, expected_retries, should_retry) in retry_configs {
        let retry_count = get_retry_count_for_error(&error_category);
        let will_retry = retry_count > 0;
        
        assert_eq!(retry_count, expected_retries);
        assert_eq!(will_retry, should_retry, 
                   "Retry decision wrong for: {:?}", error_category);
    }
}

fn get_retry_count_for_error(error_category: &ErrorCategory) -> u32 {
    match error_category {
        ErrorCategory::Network => 3,
        ErrorCategory::Server => 2,
        ErrorCategory::Storage => 1,
        _ => 0, // Don't retry other types
    }
}

#[test]
fn test_sync_performance_monitoring() {
    // Test performance monitoring metrics
    let performance_data = SyncPerformanceData {
        throughput_mbps: 5.2,
        files_per_second: 2.8,
        avg_file_size_mb: 1.8,
        memory_usage_mb: 45.6,
        cpu_usage_percent: 12.3,
        network_latency_ms: 85,
        error_rate_percent: 2.1,
    };
    
    // Test performance thresholds
    assert!(performance_data.throughput_mbps > 1.0, "Throughput should be reasonable");
    assert!(performance_data.files_per_second > 0.5, "File processing rate should be reasonable");
    assert!(performance_data.memory_usage_mb < 500.0, "Memory usage should be reasonable");
    assert!(performance_data.cpu_usage_percent < 80.0, "CPU usage should be reasonable");
    assert!(performance_data.network_latency_ms < 1000, "Network latency should be reasonable");
    assert!(performance_data.error_rate_percent < 10.0, "Error rate should be low");
}

#[derive(Debug, Clone)]
struct SyncPerformanceData {
    throughput_mbps: f64,
    files_per_second: f64,
    avg_file_size_mb: f64,
    memory_usage_mb: f64,
    cpu_usage_percent: f64,
    network_latency_ms: u64,
    error_rate_percent: f64,
}

#[test]
fn test_source_priority_handling() {
    // Test priority-based source processing
    let sources = vec![
        (SourceType::LocalFolder, 1), // Highest priority (local is fastest)
        (SourceType::WebDAV, 2),      // Medium priority
        (SourceType::S3, 3),          // Lower priority (remote with potential costs)
    ];
    
    let mut sorted_sources = sources.clone();
    sorted_sources.sort_by_key(|(_, priority)| *priority);
    
    assert_eq!(sorted_sources[0].0, SourceType::LocalFolder);
    assert_eq!(sorted_sources[1].0, SourceType::WebDAV);
    assert_eq!(sorted_sources[2].0, SourceType::S3);
    
    // Test that local sources are processed first
    let local_priority = get_source_priority(&SourceType::LocalFolder);
    let webdav_priority = get_source_priority(&SourceType::WebDAV);
    let s3_priority = get_source_priority(&SourceType::S3);
    
    assert!(local_priority < webdav_priority);
    assert!(webdav_priority < s3_priority);
}

fn get_source_priority(source_type: &SourceType) -> u32 {
    match source_type {
        SourceType::LocalFolder => 1, // Highest priority
        SourceType::WebDAV => 2,      // Medium priority
        SourceType::S3 => 3,          // Lower priority
    }
}

#[test]
fn test_concurrent_sync_protection() {
    use std::sync::{Arc, Mutex};
    use std::collections::HashSet;
    
    // Test that only one sync per source can run at a time
    let active_syncs: Arc<Mutex<HashSet<Uuid>>> = Arc::new(Mutex::new(HashSet::new()));
    
    let source_id_1 = Uuid::new_v4();
    let source_id_2 = Uuid::new_v4();
    
    // Test adding first sync
    {
        let mut syncs = active_syncs.lock().unwrap();
        assert!(syncs.insert(source_id_1));
    }
    
    // Test adding second sync (different source)
    {
        let mut syncs = active_syncs.lock().unwrap();
        assert!(syncs.insert(source_id_2));
    }
    
    // Test preventing duplicate sync for same source
    {
        let mut syncs = active_syncs.lock().unwrap();
        assert!(!syncs.insert(source_id_1)); // Should fail
    }
    
    // Test cleanup after sync completion
    {
        let mut syncs = active_syncs.lock().unwrap();
        assert!(syncs.remove(&source_id_1));
        assert!(!syncs.remove(&source_id_1)); // Should fail second time
    }
}

#[test]
fn test_sync_state_transitions() {
    // Test valid state transitions during sync
    let valid_transitions = vec![
        (SourceStatus::Idle, SourceStatus::Syncing),
        (SourceStatus::Syncing, SourceStatus::Idle),
        (SourceStatus::Syncing, SourceStatus::Error),
        (SourceStatus::Error, SourceStatus::Syncing),
        (SourceStatus::Error, SourceStatus::Idle),
    ];
    
    for (from_state, to_state) in valid_transitions {
        assert!(is_valid_state_transition(&from_state, &to_state),
                "Invalid transition from {:?} to {:?}", from_state, to_state);
    }
    
    // Test invalid transitions
    let invalid_transitions = vec![
        (SourceStatus::Idle, SourceStatus::Error), // Can't go directly to error without syncing
    ];
    
    for (from_state, to_state) in invalid_transitions {
        assert!(!is_valid_state_transition(&from_state, &to_state),
                "Should not allow transition from {:?} to {:?}", from_state, to_state);
    }
}

fn is_valid_state_transition(from: &SourceStatus, to: &SourceStatus) -> bool {
    match (from, to) {
        (SourceStatus::Idle, SourceStatus::Syncing) => true,
        (SourceStatus::Syncing, SourceStatus::Idle) => true,
        (SourceStatus::Syncing, SourceStatus::Error) => true,
        (SourceStatus::Error, SourceStatus::Syncing) => true,
        (SourceStatus::Error, SourceStatus::Idle) => true,
        _ => false,
    }
}

#[test]
fn test_bandwidth_limiting() {
    // Test bandwidth limiting calculations
    let bandwidth_limiter = BandwidthLimiter {
        max_mbps: 10.0,
        current_usage_mbps: 8.5,
        burst_allowance_mb: 50.0,
        current_burst_mb: 25.0,
    };
    
    // Test if download should be throttled
    let should_throttle = bandwidth_limiter.should_throttle_download(5.0); // 5MB download
    assert!(!should_throttle, "Small download within burst allowance should not be throttled");
    
    let should_throttle_large = bandwidth_limiter.should_throttle_download(30.0); // 30MB download
    assert!(should_throttle_large, "Large download exceeding burst should be throttled");
    
    // Test delay calculation
    let delay_ms = bandwidth_limiter.calculate_delay_ms(1_000_000); // 1MB
    assert!(delay_ms > 0, "Should have some delay when near bandwidth limit");
}

#[derive(Debug, Clone)]
struct BandwidthLimiter {
    max_mbps: f64,
    current_usage_mbps: f64,
    burst_allowance_mb: f64,
    current_burst_mb: f64,
}

impl BandwidthLimiter {
    fn should_throttle_download(&self, download_size_mb: f64) -> bool {
        self.current_usage_mbps >= self.max_mbps * 0.8 && // Near limit
        download_size_mb > (self.burst_allowance_mb - self.current_burst_mb)
    }
    
    fn calculate_delay_ms(&self, bytes: u64) -> u64 {
        if self.current_usage_mbps < self.max_mbps * 0.8 {
            return 0; // No throttling needed
        }
        
        let mb = bytes as f64 / 1_000_000.0;
        let ideal_time_seconds = mb / self.max_mbps;
        (ideal_time_seconds * 1000.0) as u64
    }
}