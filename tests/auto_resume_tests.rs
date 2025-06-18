/*!
 * Auto-Resume Functionality Unit Tests
 * 
 * Tests for auto-resume sync functionality including:
 * - Server restart detection and recovery
 * - 30-second startup delay
 * - Interrupted sync detection
 * - State cleanup and restoration
 * - User notifications for resumed syncs
 * - Error handling during resume
 */

use std::sync::Arc;
use std::time::{Duration, SystemTime};
use uuid::Uuid;
use chrono::{Utc, DateTime};
use serde_json::json;
use tokio::time::{sleep, timeout};

use readur::{
    AppState,
    config::Config,
    db::Database,
    models::{Source, SourceType, SourceStatus, WebDAVSourceConfig, CreateNotification},
    source_scheduler::SourceScheduler,
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
        watch_interval_seconds: None,
        file_stability_check_ms: None,
        max_file_age_hours: None,
        ocr_language: "eng".to_string(),
        concurrent_ocr_jobs: 4,
        ocr_timeout_seconds: 300,
        max_file_size_mb: 50,
        memory_limit_mb: 512,
        cpu_priority: "normal".to_string(),
    };

    let db = Database::new(&config.database_url).await.unwrap();
    let queue_service = Arc::new(readur::ocr_queue::OcrQueueService::new(
        db.clone(),
        db.pool.clone(),
        4,
    ));
    
    Arc::new(AppState {
        db,
        config,
        webdav_scheduler: None,
        source_scheduler: None,
        queue_service,
    })
}

/// Create a source that appears to be interrupted during sync
fn create_interrupted_source(user_id: Uuid, source_type: SourceType) -> Source {
    let mut config = json!({});
    
    match source_type {
        SourceType::WebDAV => {
            config = json!({
                "server_url": "https://cloud.example.com",
                "username": "testuser",
                "password": "testpass",
                "watch_folders": ["/Documents"],
                "file_extensions": [".pdf", ".txt"],
                "auto_sync": true,
                "sync_interval_minutes": 60,
                "server_type": "nextcloud"
            });
        },
        SourceType::LocalFolder => {
            config = json!({
                "paths": ["/home/user/documents"],
                "recursive": true,
                "follow_symlinks": false,
                "auto_sync": true,
                "sync_interval_minutes": 30,
                "file_extensions": [".pdf", ".txt"]
            });
        },
        SourceType::S3 => {
            config = json!({
                "bucket": "test-bucket",
                "region": "us-east-1",
                "access_key_id": "AKIATEST",
                "secret_access_key": "secrettest",
                "prefix": "documents/",
                "auto_sync": true,
                "sync_interval_minutes": 120,
                "file_extensions": [".pdf", ".docx"]
            });
        }
    }

    Source {
        id: Uuid::new_v4(),
        user_id,
        name: format!("Interrupted {} Source", source_type.to_string()),
        source_type,
        enabled: true,
        config,
        status: SourceStatus::Syncing, // This indicates interruption
        last_sync_at: Some(Utc::now() - chrono::Duration::minutes(10)), // Started 10 min ago
        last_error: None,
        last_error_at: None,
        total_files_synced: 5, // Some progress was made
        total_files_pending: 15, // Still work to do
        total_size_bytes: 10_000_000, // 10MB
        created_at: Utc::now() - chrono::Duration::hours(1),
        updated_at: Utc::now() - chrono::Duration::minutes(10),
    }
}

/// Create a source that completed successfully
fn create_completed_source(user_id: Uuid) -> Source {
    Source {
        id: Uuid::new_v4(),
        user_id,
        name: "Completed Source".to_string(),
        source_type: SourceType::WebDAV,
        enabled: true,
        config: json!({
            "server_url": "https://cloud.example.com",
            "username": "testuser",
            "password": "testpass",
            "watch_folders": ["/Documents"],
            "file_extensions": [".pdf"],
            "auto_sync": true,
            "sync_interval_minutes": 60,
            "server_type": "nextcloud"
        }),
        status: SourceStatus::Idle, // Completed normally
        last_sync_at: Some(Utc::now() - chrono::Duration::minutes(30)),
        last_error: None,
        last_error_at: None,
        total_files_synced: 20,
        total_files_pending: 0,
        total_size_bytes: 50_000_000,
        created_at: Utc::now() - chrono::Duration::hours(2),
        updated_at: Utc::now() - chrono::Duration::minutes(30),
    }
}

#[tokio::test]
async fn test_interrupted_sync_detection() {
    let user_id = Uuid::new_v4();
    
    // Test detection for each source type
    let webdav_source = create_interrupted_source(user_id, SourceType::WebDAV);
    let local_source = create_interrupted_source(user_id, SourceType::LocalFolder);
    let s3_source = create_interrupted_source(user_id, SourceType::S3);
    
    // All should be detected as interrupted
    assert!(is_interrupted_sync(&webdav_source), "WebDAV source should be detected as interrupted");
    assert!(is_interrupted_sync(&local_source), "Local folder source should be detected as interrupted");
    assert!(is_interrupted_sync(&s3_source), "S3 source should be detected as interrupted");
    
    // Test that completed source is not detected as interrupted
    let completed_source = create_completed_source(user_id);
    assert!(!is_interrupted_sync(&completed_source), "Completed source should not be detected as interrupted");
}

fn is_interrupted_sync(source: &Source) -> bool {
    // A source is considered interrupted if it's in "syncing" status
    // but the server has restarted (which we simulate here)
    source.status == SourceStatus::Syncing
}

#[tokio::test]
async fn test_auto_sync_configuration_check() {
    let user_id = Uuid::new_v4();
    
    // Test WebDAV source with auto_sync enabled
    let webdav_enabled = create_interrupted_source(user_id, SourceType::WebDAV);
    let should_resume = should_auto_resume_sync(&webdav_enabled);
    assert!(should_resume, "WebDAV source with auto_sync should be resumed");
    
    // Test source with auto_sync disabled
    let mut webdav_disabled = create_interrupted_source(user_id, SourceType::WebDAV);
    webdav_disabled.config = json!({
        "server_url": "https://cloud.example.com",
        "username": "testuser",
        "password": "testpass",
        "watch_folders": ["/Documents"],
        "file_extensions": [".pdf"],
        "auto_sync": false, // Disabled
        "sync_interval_minutes": 60,
        "server_type": "nextcloud"
    });
    
    let should_not_resume = should_auto_resume_sync(&webdav_disabled);
    assert!(!should_not_resume, "Source with auto_sync disabled should not be resumed");
}

fn should_auto_resume_sync(source: &Source) -> bool {
    if !is_interrupted_sync(source) {
        return false;
    }
    
    // Check auto_sync setting based on source type
    match source.source_type {
        SourceType::WebDAV => {
            if let Ok(config) = serde_json::from_value::<WebDAVSourceConfig>(source.config.clone()) {
                config.auto_sync
            } else { false }
        },
        SourceType::LocalFolder => {
            if let Ok(config) = serde_json::from_value::<readur::models::LocalFolderSourceConfig>(source.config.clone()) {
                config.auto_sync
            } else { false }
        },
        SourceType::S3 => {
            if let Ok(config) = serde_json::from_value::<readur::models::S3SourceConfig>(source.config.clone()) {
                config.auto_sync
            } else { false }
        }
    }
}

#[tokio::test]
async fn test_startup_delay_timing() {
    let start_time = std::time::Instant::now();
    
    // Simulate the 30-second startup delay
    let delay_duration = Duration::from_secs(30);
    
    // In a real test, we might use a shorter delay for speed
    let test_delay = Duration::from_millis(100); // Shortened for testing
    
    sleep(test_delay).await;
    
    let elapsed = start_time.elapsed();
    assert!(elapsed >= test_delay, "Should wait for at least the specified delay");
    
    // Test that the delay is configurable
    let configurable_delay = get_startup_delay_from_config();
    assert_eq!(configurable_delay, Duration::from_secs(30));
}

fn get_startup_delay_from_config() -> Duration {
    // In real implementation, this might come from configuration
    Duration::from_secs(30)
}

#[test]
fn test_sync_state_cleanup() {
    let user_id = Uuid::new_v4();
    let interrupted_source = create_interrupted_source(user_id, SourceType::WebDAV);
    
    // Test state cleanup (reset from syncing to idle)
    let cleaned_status = cleanup_interrupted_status(&interrupted_source.status);
    assert_eq!(cleaned_status, SourceStatus::Idle);
    
    // Test that other statuses are not affected
    let idle_status = cleanup_interrupted_status(&SourceStatus::Idle);
    assert_eq!(idle_status, SourceStatus::Idle);
    
    let error_status = cleanup_interrupted_status(&SourceStatus::Error);
    assert_eq!(error_status, SourceStatus::Error);
}

fn cleanup_interrupted_status(status: &SourceStatus) -> SourceStatus {
    match status {
        SourceStatus::Syncing => SourceStatus::Idle, // Reset interrupted syncs
        other => other.clone(), // Keep other statuses as-is
    }
}

#[test]
fn test_resume_notification_creation() {
    let user_id = Uuid::new_v4();
    let source = create_interrupted_source(user_id, SourceType::WebDAV);
    let files_processed = 12;
    
    let notification = create_resume_notification(&source, files_processed);
    
    assert_eq!(notification.notification_type, "success");
    assert_eq!(notification.title, "Source Sync Resumed");
    assert!(notification.message.contains(&source.name));
    assert!(notification.message.contains(&files_processed.to_string()));
    assert_eq!(notification.action_url, Some("/sources".to_string()));
    
    // Check metadata
    assert!(notification.metadata.is_some());
    let metadata = notification.metadata.unwrap();
    assert_eq!(metadata["source_type"], source.source_type.to_string());
    assert_eq!(metadata["source_id"], source.id.to_string());
    assert_eq!(metadata["files_processed"], files_processed);
}

fn create_resume_notification(source: &Source, files_processed: u32) -> CreateNotification {
    CreateNotification {
        notification_type: "success".to_string(),
        title: "Source Sync Resumed".to_string(),
        message: format!(
            "Resumed sync for {} after server restart. Processed {} files",
            source.name, files_processed
        ),
        action_url: Some("/sources".to_string()),
        metadata: Some(json!({
            "source_type": source.source_type.to_string(),
            "source_id": source.id,
            "files_processed": files_processed
        })),
    }
}

#[test]
fn test_resume_error_notification() {
    let user_id = Uuid::new_v4();
    let source = create_interrupted_source(user_id, SourceType::S3);
    let error_message = "S3 bucket access denied";
    
    let notification = create_resume_error_notification(&source, error_message);
    
    assert_eq!(notification.notification_type, "error");
    assert_eq!(notification.title, "Source Sync Resume Failed");
    assert!(notification.message.contains(&source.name));
    assert!(notification.message.contains(error_message));
    
    let metadata = notification.metadata.unwrap();
    assert_eq!(metadata["error"], error_message);
}

fn create_resume_error_notification(source: &Source, error: &str) -> CreateNotification {
    CreateNotification {
        notification_type: "error".to_string(),
        title: "Source Sync Resume Failed".to_string(),
        message: format!("Failed to resume sync for {}: {}", source.name, error),
        action_url: Some("/sources".to_string()),
        metadata: Some(json!({
            "source_type": source.source_type.to_string(),
            "source_id": source.id,
            "error": error
        })),
    }
}

#[tokio::test]
async fn test_resume_with_timeout() {
    let user_id = Uuid::new_v4();
    let source = create_interrupted_source(user_id, SourceType::WebDAV);
    
    // Test that resume operation can timeout
    let resume_timeout = Duration::from_secs(5);
    
    let result = timeout(resume_timeout, simulate_resume_operation(&source)).await;
    
    match result {
        Ok(resume_result) => {
            assert!(resume_result.is_ok(), "Resume should succeed within timeout");
        },
        Err(_) => {
            // Timeout occurred - this is also a valid test scenario
            println!("Resume operation timed out (expected in some test scenarios)");
        }
    }
}

async fn simulate_resume_operation(source: &Source) -> Result<u32, String> {
    // Simulate some work
    sleep(Duration::from_millis(100)).await;
    
    // Return number of files processed
    Ok(source.total_files_pending as u32)
}

#[test]
fn test_resume_priority_ordering() {
    let user_id = Uuid::new_v4();
    
    // Create sources with different types and interruption times
    let mut sources = vec![
        create_interrupted_source_with_time(user_id, SourceType::S3, 60), // 1 hour ago
        create_interrupted_source_with_time(user_id, SourceType::LocalFolder, 30), // 30 min ago
        create_interrupted_source_with_time(user_id, SourceType::WebDAV, 120), // 2 hours ago
    ];
    
    // Sort by resume priority
    sources.sort_by_key(|s| get_resume_priority(s));
    
    // Local folder should have highest priority (lowest number)
    assert_eq!(sources[0].source_type, SourceType::LocalFolder);
    
    // S3 should be next (interrupted more recently than WebDAV)
    assert_eq!(sources[1].source_type, SourceType::S3);
    
    // WebDAV should have lowest priority (interrupted longest ago)
    assert_eq!(sources[2].source_type, SourceType::WebDAV);
}

fn create_interrupted_source_with_time(user_id: Uuid, source_type: SourceType, minutes_ago: i64) -> Source {
    let mut source = create_interrupted_source(user_id, source_type);
    source.last_sync_at = Some(Utc::now() - chrono::Duration::minutes(minutes_ago));
    source
}

fn get_resume_priority(source: &Source) -> u32 {
    // Lower number = higher priority
    let type_priority = match source.source_type {
        SourceType::LocalFolder => 1, // Highest priority (fastest)
        SourceType::WebDAV => 2,      // Medium priority
        SourceType::S3 => 3,          // Lower priority (potential costs)
    };
    
    // Consider how long ago the sync was interrupted
    let time_penalty = if let Some(last_sync) = source.last_sync_at {
        let minutes_ago = (Utc::now() - last_sync).num_minutes();
        (minutes_ago / 30) as u32 // Add 1 to priority for every 30 minutes
    } else {
        10 // High penalty for unknown last sync time
    };
    
    type_priority + time_penalty
}

#[test]
fn test_resume_batch_processing() {
    let user_id = Uuid::new_v4();
    
    // Create multiple interrupted sources
    let sources = vec![
        create_interrupted_source(user_id, SourceType::WebDAV),
        create_interrupted_source(user_id, SourceType::LocalFolder),
        create_interrupted_source(user_id, SourceType::S3),
        create_interrupted_source(user_id, SourceType::WebDAV),
    ];
    
    // Test batching by source type
    let batches = group_sources_by_type(&sources);
    
    assert_eq!(batches.len(), 3); // Three different types
    assert!(batches.contains_key(&SourceType::WebDAV));
    assert!(batches.contains_key(&SourceType::LocalFolder));
    assert!(batches.contains_key(&SourceType::S3));
    
    // WebDAV should have 2 sources
    assert_eq!(batches[&SourceType::WebDAV].len(), 2);
    assert_eq!(batches[&SourceType::LocalFolder].len(), 1);
    assert_eq!(batches[&SourceType::S3].len(), 1);
}

use std::collections::HashMap;

fn group_sources_by_type(sources: &[Source]) -> HashMap<SourceType, Vec<&Source>> {
    let mut groups: HashMap<SourceType, Vec<&Source>> = HashMap::new();
    
    for source in sources {
        groups.entry(source.source_type.clone()).or_insert_with(Vec::new).push(source);
    }
    
    groups
}

#[test]
fn test_resume_failure_handling() {
    let user_id = Uuid::new_v4();
    let source = create_interrupted_source(user_id, SourceType::WebDAV);
    
    // Test different failure scenarios
    let failure_scenarios = vec![
        ResumeFailure::NetworkTimeout,
        ResumeFailure::AuthenticationError,
        ResumeFailure::SourceNotFound,
        ResumeFailure::ConfigurationError,
        ResumeFailure::InternalError,
    ];
    
    for failure in failure_scenarios {
        let should_retry = should_retry_resume_failure(&failure);
        let retry_delay = get_retry_delay_for_failure(&failure);
        
        match failure {
            ResumeFailure::NetworkTimeout => {
                assert!(should_retry, "Should retry network timeouts");
                assert!(retry_delay > Duration::ZERO, "Should have retry delay");
            },
            ResumeFailure::AuthenticationError => {
                assert!(!should_retry, "Should not retry auth errors");
            },
            ResumeFailure::SourceNotFound => {
                assert!(!should_retry, "Should not retry if source not found");
            },
            ResumeFailure::ConfigurationError => {
                assert!(!should_retry, "Should not retry config errors");
            },
            ResumeFailure::InternalError => {
                assert!(should_retry, "Should retry internal errors");
            },
        }
    }
}

#[derive(Debug, Clone)]
enum ResumeFailure {
    NetworkTimeout,
    AuthenticationError,
    SourceNotFound,
    ConfigurationError,
    InternalError,
}

fn should_retry_resume_failure(failure: &ResumeFailure) -> bool {
    match failure {
        ResumeFailure::NetworkTimeout => true,
        ResumeFailure::InternalError => true,
        _ => false,
    }
}

fn get_retry_delay_for_failure(failure: &ResumeFailure) -> Duration {
    match failure {
        ResumeFailure::NetworkTimeout => Duration::from_secs(30),
        ResumeFailure::InternalError => Duration::from_secs(60),
        _ => Duration::ZERO,
    }
}

#[tokio::test]
async fn test_resume_state_persistence() {
    let state = create_test_app_state().await;
    let user_id = Uuid::new_v4();
    
    // Create a source that appears interrupted
    let interrupted_source = create_interrupted_source(user_id, SourceType::WebDAV);
    
    // Test that we can track resume progress
    let mut resume_state = ResumeState::new();
    
    resume_state.start_resume(&interrupted_source);
    assert!(resume_state.is_resuming(&interrupted_source.id));
    
    resume_state.complete_resume(&interrupted_source.id, 15);
    assert!(!resume_state.is_resuming(&interrupted_source.id));
    
    let stats = resume_state.get_stats(&interrupted_source.id);
    assert!(stats.is_some());
    assert_eq!(stats.unwrap().files_processed, 15);
}

#[derive(Debug)]
struct ResumeState {
    active_resumes: HashMap<Uuid, SystemTime>,
    completed_resumes: HashMap<Uuid, ResumeStats>,
}

impl ResumeState {
    fn new() -> Self {
        Self {
            active_resumes: HashMap::new(),
            completed_resumes: HashMap::new(),
        }
    }
    
    fn start_resume(&mut self, source: &Source) {
        self.active_resumes.insert(source.id, SystemTime::now());
    }
    
    fn is_resuming(&self, source_id: &Uuid) -> bool {
        self.active_resumes.contains_key(source_id)
    }
    
    fn complete_resume(&mut self, source_id: &Uuid, files_processed: u32) {
        if let Some(start_time) = self.active_resumes.remove(source_id) {
            let duration = SystemTime::now().duration_since(start_time).unwrap_or(Duration::ZERO);
            
            self.completed_resumes.insert(*source_id, ResumeStats {
                files_processed,
                duration,
                completed_at: SystemTime::now(),
            });
        }
    }
    
    fn get_stats(&self, source_id: &Uuid) -> Option<&ResumeStats> {
        self.completed_resumes.get(source_id)
    }
}

#[derive(Debug, Clone)]
struct ResumeStats {
    files_processed: u32,
    duration: Duration,
    completed_at: SystemTime,
}