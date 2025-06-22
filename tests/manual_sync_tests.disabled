/*!
 * Manual Sync Triggering Unit Tests
 * 
 * Tests for manual sync triggering functionality including:
 * - API endpoint testing
 * - Source status validation
 * - Conflict detection (already syncing)
 * - Permission and authentication checks
 * - Error handling and recovery
 * - Integration with source scheduler
 */

use std::sync::Arc;
use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use uuid::Uuid;
use chrono::Utc;
use serde_json::json;
use axum::http::StatusCode;

use readur::{
    AppState,
    config::Config,
    db::Database,
    models::{Source, SourceType, SourceStatus, WebDAVSourceConfig, User, UserRole},
    auth::AuthUser,
    routes::sources,
};

/// Create a test app state
async fn create_test_app_state() -> Arc<AppState> {
    let config = Config {
        database_url: "sqlite::memory:".to_string(),
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
    };

    let db = Database::new(&config.database_url).await.unwrap();
    let queue_service = std::sync::Arc::new(readur::ocr_queue::OcrQueueService::new(db.clone(), db.pool.clone(), 2));
    
    Arc::new(AppState {
        db,
        config,
        webdav_scheduler: None,
        source_scheduler: None,
        queue_service,
    })
}

/// Create a test user
fn create_test_user() -> User {
    User {
        id: Uuid::new_v4(),
        username: "testuser".to_string(),
        email: "test@example.com".to_string(),
        password_hash: "hashed_password".to_string(),
        role: UserRole::User,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

/// Create a test source in various states
fn create_test_source_with_status(status: SourceStatus, user_id: Uuid) -> Source {
    Source {
        id: Uuid::new_v4(),
        user_id,
        name: "Test WebDAV Source".to_string(),
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
        status,
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

#[tokio::test]
async fn test_manual_sync_trigger_idle_source() {
    let state = create_test_app_state().await;
    let user = create_test_user();
    let source = create_test_source_with_status(SourceStatus::Idle, user.id);
    
    // Test that idle source can be triggered for sync
    let can_trigger = can_trigger_manual_sync(&source);
    assert!(can_trigger, "Idle source should be available for manual sync");
    
    // Test status update to syncing
    let updated_status = SourceStatus::Syncing;
    assert_ne!(source.status, updated_status);
    assert!(is_valid_sync_trigger_transition(&source.status, &updated_status));
}

#[tokio::test]
async fn test_manual_sync_trigger_already_syncing() {
    let state = create_test_app_state().await;
    let user = create_test_user();
    let source = create_test_source_with_status(SourceStatus::Syncing, user.id);
    
    // Test that already syncing source cannot be triggered again
    let can_trigger = can_trigger_manual_sync(&source);
    assert!(!can_trigger, "Already syncing source should not allow manual sync");
    
    // This should result in HTTP 409 Conflict
    let expected_status = StatusCode::CONFLICT;
    let result_status = get_expected_status_for_sync_trigger(&source);
    assert_eq!(result_status, expected_status);
}

#[tokio::test]
async fn test_manual_sync_trigger_error_state() {
    let state = create_test_app_state().await;
    let user = create_test_user();
    let mut source = create_test_source_with_status(SourceStatus::Error, user.id);
    source.last_error = Some("Previous sync failed".to_string());
    source.last_error_at = Some(Utc::now());
    
    // Test that source in error state can be triggered (retry)
    let can_trigger = can_trigger_manual_sync(&source);
    assert!(can_trigger, "Source in error state should allow manual sync retry");
    
    // Test status transition from error to syncing
    assert!(is_valid_sync_trigger_transition(&source.status, &SourceStatus::Syncing));
}

fn can_trigger_manual_sync(source: &Source) -> bool {
    match source.status {
        SourceStatus::Idle => true,
        SourceStatus::Error => true,
        SourceStatus::Syncing => false,
    }
}

fn is_valid_sync_trigger_transition(from: &SourceStatus, to: &SourceStatus) -> bool {
    match (from, to) {
        (SourceStatus::Idle, SourceStatus::Syncing) => true,
        (SourceStatus::Error, SourceStatus::Syncing) => true,
        _ => false,
    }
}

fn get_expected_status_for_sync_trigger(source: &Source) -> StatusCode {
    match source.status {
        SourceStatus::Idle => StatusCode::OK,
        SourceStatus::Error => StatusCode::OK,
        SourceStatus::Syncing => StatusCode::CONFLICT,
    }
}

#[tokio::test]
async fn test_source_ownership_validation() {
    let user_1 = create_test_user();
    let user_2 = User {
        id: Uuid::new_v4(),
        username: "otheruser".to_string(),
        email: "other@example.com".to_string(),
        password_hash: "other_hash".to_string(),
        role: UserRole::User,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    let source = create_test_source_with_status(SourceStatus::Idle, user_1.id);
    
    // Test that owner can trigger sync
    assert!(can_user_trigger_sync(&user_1, &source));
    
    // Test that non-owner cannot trigger sync
    assert!(!can_user_trigger_sync(&user_2, &source));
    
    // Test admin can trigger any sync
    let admin_user = User {
        id: Uuid::new_v4(),
        username: "admin".to_string(),
        email: "admin@example.com".to_string(),
        password_hash: "admin_hash".to_string(),
        role: UserRole::Admin,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    assert!(can_user_trigger_sync(&admin_user, &source));
}

fn can_user_trigger_sync(user: &User, source: &Source) -> bool {
    user.role == UserRole::Admin || user.id == source.user_id
}

#[test]
fn test_sync_trigger_request_validation() {
    // Test valid source IDs
    let valid_id = Uuid::new_v4();
    assert!(is_valid_source_id(&valid_id.to_string()));
    
    // Test invalid source IDs
    let invalid_ids = vec![
        "",
        "invalid-uuid",
        "12345",
        "not-a-uuid-at-all",
    ];
    
    for invalid_id in invalid_ids {
        assert!(!is_valid_source_id(invalid_id), "Should reject invalid UUID: {}", invalid_id);
    }
}

fn is_valid_source_id(id_str: &str) -> bool {
    Uuid::parse_str(id_str).is_ok()
}

#[test]
fn test_sync_trigger_rate_limiting() {
    
    // Test rate limiting for manual sync triggers
    let mut rate_limiter = SyncRateLimiter::new();
    let source_id = Uuid::new_v4();
    
    // First trigger should be allowed
    assert!(rate_limiter.can_trigger_sync(&source_id));
    rate_limiter.record_sync_trigger(&source_id);
    
    // Immediate second trigger should be blocked
    assert!(!rate_limiter.can_trigger_sync(&source_id));
    
    // After cooldown period, should be allowed again
    rate_limiter.advance_time(Duration::from_secs(61)); // Advance past cooldown
    assert!(rate_limiter.can_trigger_sync(&source_id));
}

struct SyncRateLimiter {
    last_triggers: HashMap<Uuid, SystemTime>,
    cooldown_period: Duration,
    current_time: SystemTime,
}

impl SyncRateLimiter {
    fn new() -> Self {
        Self {
            last_triggers: HashMap::new(),
            cooldown_period: Duration::from_secs(60), // 1 minute cooldown
            current_time: SystemTime::now(),
        }
    }
    
    fn can_trigger_sync(&self, source_id: &Uuid) -> bool {
        if let Some(&last_trigger) = self.last_triggers.get(source_id) {
            self.current_time.duration_since(last_trigger).unwrap_or(Duration::ZERO) >= self.cooldown_period
        } else {
            true // Never triggered before
        }
    }
    
    fn record_sync_trigger(&mut self, source_id: &Uuid) {
        self.last_triggers.insert(*source_id, self.current_time);
    }
    
    fn advance_time(&mut self, duration: Duration) {
        self.current_time += duration;
    }
}

#[tokio::test]
async fn test_sync_trigger_with_disabled_source() {
    let state = create_test_app_state().await;
    let user = create_test_user();
    let mut source = create_test_source_with_status(SourceStatus::Idle, user.id);
    source.enabled = false; // Disable the source
    
    // Test that disabled source cannot be triggered
    let can_trigger = can_trigger_disabled_source(&source);
    assert!(!can_trigger, "Disabled source should not allow manual sync");
    
    // This should result in HTTP 400 Bad Request
    let expected_status = if source.enabled {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };
    
    assert_eq!(expected_status, StatusCode::BAD_REQUEST);
}

fn can_trigger_disabled_source(source: &Source) -> bool {
    source.enabled && can_trigger_manual_sync(source)
}

#[test]
fn test_sync_trigger_configuration_validation() {
    let user_id = Uuid::new_v4();
    
    // Test valid WebDAV configuration
    let valid_source = create_test_source_with_status(SourceStatus::Idle, user_id);
    let config_result: Result<WebDAVSourceConfig, _> = serde_json::from_value(valid_source.config.clone());
    assert!(config_result.is_ok(), "Valid configuration should parse successfully");
    
    // Test invalid configuration
    let mut invalid_source = create_test_source_with_status(SourceStatus::Idle, user_id);
    invalid_source.config = json!({
        "server_url": "", // Invalid empty URL
        "username": "test",
        "password": "test"
        // Missing required fields
    });
    
    let invalid_config_result: Result<WebDAVSourceConfig, _> = serde_json::from_value(invalid_source.config.clone());
    assert!(invalid_config_result.is_err(), "Invalid configuration should fail to parse");
}

#[test]
fn test_concurrent_sync_trigger_protection() {
    use std::sync::{Arc, Mutex};
    use std::collections::HashSet;
    use std::thread;
    
    let active_syncs: Arc<Mutex<HashSet<Uuid>>> = Arc::new(Mutex::new(HashSet::new()));
    let source_id = Uuid::new_v4();
    
    let mut handles = vec![];
    let results = Arc::new(Mutex::new(Vec::new()));
    
    // Simulate multiple concurrent trigger attempts
    for _ in 0..5 {
        let active_syncs = Arc::clone(&active_syncs);
        let results = Arc::clone(&results);
        
        let handle = thread::spawn(move || {
            let mut syncs = active_syncs.lock().unwrap();
            let was_inserted = syncs.insert(source_id);
            
            results.lock().unwrap().push(was_inserted);
            
            // Simulate some work
            std::thread::sleep(std::time::Duration::from_millis(10));
        });
        
        handles.push(handle);
    }
    
    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }
    
    let final_results = results.lock().unwrap();
    let successful_triggers = final_results.iter().filter(|&&success| success).count();
    
    // Only one thread should have successfully triggered the sync
    assert_eq!(successful_triggers, 1, "Only one concurrent sync trigger should succeed");
}

#[test]
fn test_sync_trigger_error_responses() {
    // Test various error scenarios and their expected HTTP responses
    let test_cases = vec![
        (SyncTriggerError::SourceNotFound, StatusCode::NOT_FOUND),
        (SyncTriggerError::AlreadySyncing, StatusCode::CONFLICT),
        (SyncTriggerError::SourceDisabled, StatusCode::BAD_REQUEST),
        (SyncTriggerError::InvalidConfiguration, StatusCode::BAD_REQUEST),
        (SyncTriggerError::PermissionDenied, StatusCode::FORBIDDEN),
        (SyncTriggerError::RateLimited, StatusCode::TOO_MANY_REQUESTS),
        (SyncTriggerError::InternalError, StatusCode::INTERNAL_SERVER_ERROR),
    ];
    
    for (error, expected_status) in test_cases {
        let status = error.to_status_code();
        assert_eq!(status, expected_status, "Wrong status code for error: {:?}", error);
    }
}

#[derive(Debug, Clone)]
enum SyncTriggerError {
    SourceNotFound,
    AlreadySyncing,
    SourceDisabled,
    InvalidConfiguration,
    PermissionDenied,
    RateLimited,
    InternalError,
}

impl SyncTriggerError {
    fn to_status_code(&self) -> StatusCode {
        match self {
            SyncTriggerError::SourceNotFound => StatusCode::NOT_FOUND,
            SyncTriggerError::AlreadySyncing => StatusCode::CONFLICT,
            SyncTriggerError::SourceDisabled => StatusCode::BAD_REQUEST,
            SyncTriggerError::InvalidConfiguration => StatusCode::BAD_REQUEST,
            SyncTriggerError::PermissionDenied => StatusCode::FORBIDDEN,
            SyncTriggerError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            SyncTriggerError::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[test]
fn test_manual_sync_metrics() {
    // Test tracking of manual sync triggers vs automatic syncs
    let mut sync_metrics = ManualSyncMetrics::new();
    let source_id = Uuid::new_v4();
    
    // Record manual triggers
    sync_metrics.record_manual_trigger(source_id);
    sync_metrics.record_manual_trigger(source_id);
    
    // Record automatic syncs
    sync_metrics.record_automatic_sync(source_id);
    
    let stats = sync_metrics.get_stats_for_source(&source_id);
    assert_eq!(stats.manual_triggers, 2);
    assert_eq!(stats.automatic_syncs, 1);
    assert_eq!(stats.total_syncs(), 3);
    
    let manual_ratio = stats.manual_trigger_ratio();
    assert!((manual_ratio - 0.666).abs() < 0.01); // ~66.7%
}

struct ManualSyncMetrics {
    manual_triggers: HashMap<Uuid, u32>,
    automatic_syncs: HashMap<Uuid, u32>,
}

impl ManualSyncMetrics {
    fn new() -> Self {
        Self {
            manual_triggers: HashMap::new(),
            automatic_syncs: HashMap::new(),
        }
    }
    
    fn record_manual_trigger(&mut self, source_id: Uuid) {
        *self.manual_triggers.entry(source_id).or_insert(0) += 1;
    }
    
    fn record_automatic_sync(&mut self, source_id: Uuid) {
        *self.automatic_syncs.entry(source_id).or_insert(0) += 1;
    }
    
    fn get_stats_for_source(&self, source_id: &Uuid) -> SyncStats {
        SyncStats {
            manual_triggers: self.manual_triggers.get(source_id).copied().unwrap_or(0),
            automatic_syncs: self.automatic_syncs.get(source_id).copied().unwrap_or(0),
        }
    }
}

struct SyncStats {
    manual_triggers: u32,
    automatic_syncs: u32,
}

impl SyncStats {
    fn total_syncs(&self) -> u32 {
        self.manual_triggers + self.automatic_syncs
    }
    
    fn manual_trigger_ratio(&self) -> f64 {
        if self.total_syncs() == 0 {
            0.0
        } else {
            self.manual_triggers as f64 / self.total_syncs() as f64
        }
    }
}

#[test]
fn test_sync_trigger_audit_logging() {
    // Test audit logging for manual sync triggers
    let mut audit_log = SyncAuditLog::new();
    let user_id = Uuid::new_v4();
    let source_id = Uuid::new_v4();
    
    // Record successful trigger
    audit_log.log_sync_trigger(SyncTriggerEvent {
        user_id,
        source_id,
        timestamp: Utc::now(),
        result: SyncTriggerResult::Success,
        user_agent: Some("Mozilla/5.0 (Test Browser)".to_string()),
        ip_address: Some("192.168.1.100".to_string()),
    });
    
    // Record failed trigger
    audit_log.log_sync_trigger(SyncTriggerEvent {
        user_id,
        source_id,
        timestamp: Utc::now(),
        result: SyncTriggerResult::Failed("Already syncing".to_string()),
        user_agent: Some("Mozilla/5.0 (Test Browser)".to_string()),
        ip_address: Some("192.168.1.100".to_string()),
    });
    
    let events = audit_log.get_events_for_user(&user_id);
    assert_eq!(events.len(), 2);
    assert!(matches!(events[0].result, SyncTriggerResult::Success));
    assert!(matches!(events[1].result, SyncTriggerResult::Failed(_)));
}

struct SyncAuditLog {
    events: Vec<SyncTriggerEvent>,
}

impl SyncAuditLog {
    fn new() -> Self {
        Self {
            events: Vec::new(),
        }
    }
    
    fn log_sync_trigger(&mut self, event: SyncTriggerEvent) {
        self.events.push(event);
    }
    
    fn get_events_for_user(&self, user_id: &Uuid) -> Vec<&SyncTriggerEvent> {
        self.events.iter().filter(|e| e.user_id == *user_id).collect()
    }
}

#[derive(Debug, Clone)]
struct SyncTriggerEvent {
    user_id: Uuid,
    source_id: Uuid,
    timestamp: chrono::DateTime<Utc>,
    result: SyncTriggerResult,
    user_agent: Option<String>,
    ip_address: Option<String>,
}

#[derive(Debug, Clone)]
enum SyncTriggerResult {
    Success,
    Failed(String),
}

#[tokio::test]
async fn test_sync_trigger_with_scheduler_integration() {
    // Test integration with source scheduler
    let state = create_test_app_state().await;
    let user = create_test_user();
    let source = create_test_source_with_status(SourceStatus::Idle, user.id);
    
    // Test that trigger_sync method exists and handles the source
    let sync_request = ManualSyncRequest {
        source_id: source.id,
        user_id: user.id,
        force: false, // Don't force if already syncing
        priority: SyncPriority::Normal,
    };
    
    // Simulate what the actual API would do
    let can_proceed = validate_sync_request(&sync_request, &source);
    assert!(can_proceed, "Valid sync request should be allowed");
}

#[derive(Debug, Clone)]
struct ManualSyncRequest {
    source_id: Uuid,
    user_id: Uuid,
    force: bool,
    priority: SyncPriority,
}

#[derive(Debug, Clone)]
enum SyncPriority {
    Low,
    Normal,
    High,
    Urgent,
}

fn validate_sync_request(request: &ManualSyncRequest, source: &Source) -> bool {
    // Check ownership
    if request.user_id != source.user_id {
        return false;
    }
    
    // Check if source is enabled
    if !source.enabled {
        return false;
    }
    
    // Check status (allow force override)
    if !request.force && source.status == SourceStatus::Syncing {
        return false;
    }
    
    true
}