/*!
 * Sync Cancellation Behavior Unit Tests
 * 
 * Tests for sync cancellation functionality including:
 * - Graceful cancellation of ongoing downloads
 * - Allowing OCR to continue after sync cancellation
 * - Cleanup of partial downloads
 * - State management during cancellation
 * - Cancellation signal propagation
 * - Resource cleanup and memory management
 */

use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::time::{Duration, SystemTime, Instant};
use std::thread;
use uuid::Uuid;
use chrono::Utc;
use serde_json::json;
use tokio::time::{sleep, timeout};
use tokio::sync::mpsc;

use readur::{
    AppState,
    config::Config,
    db::Database,
    models::{Source, SourceType, SourceStatus, WebDAVSourceConfig},
    scheduling::source_scheduler::SourceScheduler,
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
        oidc_enabled: false,
        oidc_client_id: None,
        oidc_client_secret: None,
        oidc_issuer_url: None,
        oidc_redirect_uri: None,
    };

    let db = Database::new(&config.database_url).await.unwrap();
    
    let queue_service = Arc::new(readur::ocr::queue::OcrQueueService::new(db.clone(), db.pool.clone(), 2));
    let sync_progress_tracker = Arc::new(readur::services::sync_progress_tracker::SyncProgressTracker::new());
    Arc::new(AppState {
        db,
        config,
        webdav_scheduler: None,
        source_scheduler: None,
        queue_service,
        oidc_client: None,
        sync_progress_tracker,
    })
}

/// Create a test source for cancellation testing
fn create_test_source_for_cancellation(user_id: Uuid) -> Source {
    Source {
        id: Uuid::new_v4(),
        user_id,
        name: "Cancellable Test Source".to_string(),
        source_type: SourceType::WebDAV,
        enabled: true,
        config: json!({
            "server_url": "https://cloud.example.com",
            "username": "testuser",
            "password": "testpass",
            "watch_folders": ["/Documents"],
            "file_extensions": [".pdf", ".txt", ".jpg"],
            "auto_sync": true,
            "sync_interval_minutes": 60,
            "server_type": "nextcloud"
        }),
        status: SourceStatus::Syncing,
        last_sync_at: Some(Utc::now()),
        last_error: None,
        last_error_at: None,
        total_files_synced: 10,
        total_files_pending: 25, // Many files still to sync
        total_size_bytes: 100_000_000, // 100MB
        created_at: Utc::now() - chrono::Duration::hours(1),
        updated_at: Utc::now(),
        validation_status: None,
        last_validation_at: None,
        validation_score: None,
        validation_issues: None,
    }
}

#[tokio::test]
async fn test_cancellation_token_basic() {
    // Test basic cancellation token functionality
    let cancellation_token = Arc::new(AtomicBool::new(false));
    
    // Initially not cancelled
    assert!(!cancellation_token.load(Ordering::Relaxed));
    
    // Cancel the operation
    cancellation_token.store(true, Ordering::Relaxed);
    assert!(cancellation_token.load(Ordering::Relaxed));
    
    // Test that cancelled operations should stop
    let should_continue = !cancellation_token.load(Ordering::Relaxed);
    assert!(!should_continue, "Cancelled operation should not continue");
}

#[tokio::test]
async fn test_graceful_download_cancellation() {
    let cancellation_token = Arc::new(AtomicBool::new(false));
    let download_progress = Arc::new(Mutex::new(DownloadProgress::new()));
    
    // Simulate download process
    let token_clone = Arc::clone(&cancellation_token);
    let progress_clone = Arc::clone(&download_progress);
    
    let download_handle = tokio::spawn(async move {
        simulate_download_with_cancellation(token_clone, progress_clone).await
    });
    
    // Let download start
    sleep(Duration::from_millis(50)).await;
    
    // Cancel after some progress
    cancellation_token.store(true, Ordering::Relaxed);
    
    let result = download_handle.await.unwrap();
    
    // Check that download was cancelled gracefully
    assert!(result.was_cancelled, "Download should be marked as cancelled");
    assert!(result.bytes_downloaded > 0, "Some progress should have been made");
    assert!(result.bytes_downloaded < result.total_bytes, "Download should not be complete");
    
    let final_progress = download_progress.lock().unwrap();
    assert!(final_progress.files_downloaded < final_progress.total_files);
}

async fn simulate_download_with_cancellation(
    cancellation_token: Arc<AtomicBool>,
    progress: Arc<Mutex<DownloadProgress>>,
) -> DownloadResult {
    let total_files: u32 = 10;
    let file_size: u64 = 1024 * 1024; // 1MB per file
    let mut bytes_downloaded: u64 = 0;
    let mut files_downloaded: u32 = 0;
    
    for i in 0..total_files {
        // Check cancellation before each file
        if cancellation_token.load(Ordering::Relaxed) {
            return DownloadResult {
                was_cancelled: true,
                bytes_downloaded,
                total_bytes: total_files as u64 * file_size,
                files_downloaded,
                total_files,
            };
        }
        
        // Simulate file download
        sleep(Duration::from_millis(20)).await;
        bytes_downloaded += file_size;
        files_downloaded += 1;
        
        // Update progress
        {
            let mut prog = progress.lock().unwrap();
            prog.files_downloaded = files_downloaded;
            prog.bytes_downloaded = bytes_downloaded;
        }
    }
    
    DownloadResult {
        was_cancelled: false,
        bytes_downloaded,
        total_bytes: total_files as u64 * file_size,
        files_downloaded,
        total_files,
    }
}

#[derive(Debug, Clone)]
struct DownloadProgress {
    files_downloaded: u32,
    bytes_downloaded: u64,
    total_files: u32,
    total_bytes: u64,
}

impl DownloadProgress {
    fn new() -> Self {
        Self {
            files_downloaded: 0,
            bytes_downloaded: 0,
            total_files: 100,
            total_bytes: 100 * 1024 * 1024, // 100MB
        }
    }
}

#[derive(Debug)]
struct DownloadResult {
    was_cancelled: bool,
    bytes_downloaded: u64,
    total_bytes: u64,
    files_downloaded: u32,
    total_files: u32,
}

#[tokio::test]
async fn test_ocr_continues_after_sync_cancellation() {
    let sync_cancellation_token = Arc::new(AtomicBool::new(false));
    let ocr_queue = Arc::new(Mutex::new(OcrQueue::new()));
    
    // Add some files to OCR queue before cancellation
    {
        let mut queue = ocr_queue.lock().unwrap();
        queue.add_job(OcrJob { id: Uuid::new_v4(), file_path: "doc1.pdf".to_string() });
        queue.add_job(OcrJob { id: Uuid::new_v4(), file_path: "doc2.pdf".to_string() });
        queue.add_job(OcrJob { id: Uuid::new_v4(), file_path: "doc3.pdf".to_string() });
    }
    
    // Start OCR processing (should continue even after sync cancellation)
    let ocr_queue_clone = Arc::clone(&ocr_queue);
    let ocr_handle = tokio::spawn(async move {
        process_ocr_queue(ocr_queue_clone).await
    });
    
    // Cancel sync (should not affect OCR)
    sync_cancellation_token.store(true, Ordering::Relaxed);
    
    // Let OCR process for a bit
    sleep(Duration::from_millis(200)).await;
    
    // Check that OCR continued processing
    let queue_state = ocr_queue.lock().unwrap();
    assert!(queue_state.processed_jobs > 0, "OCR should have processed jobs despite sync cancellation");
    
    // Stop OCR processing
    queue_state.stop();
    drop(queue_state);
    
    let _ = ocr_handle.await;
}

#[derive(Debug, Clone)]
struct OcrJob {
    id: Uuid,
    file_path: String,
}

struct OcrQueue {
    pending_jobs: Vec<OcrJob>,
    processed_jobs: u32,
    is_stopped: Arc<AtomicBool>,
}

impl OcrQueue {
    fn new() -> Self {
        Self {
            pending_jobs: Vec::new(),
            processed_jobs: 0,
            is_stopped: Arc::new(AtomicBool::new(false)),
        }
    }
    
    fn add_job(&mut self, job: OcrJob) {
        self.pending_jobs.push(job);
    }
    
    fn stop(&self) {
        self.is_stopped.store(true, Ordering::Relaxed);
    }
    
    fn is_stopped(&self) -> bool {
        self.is_stopped.load(Ordering::Relaxed)
    }
}

async fn process_ocr_queue(ocr_queue: Arc<Mutex<OcrQueue>>) {
    loop {
        let should_stop = {
            let queue = ocr_queue.lock().unwrap();
            queue.is_stopped()
        };
        
        if should_stop {
            break;
        }
        
        // Process next job if available
        let job = {
            let mut queue = ocr_queue.lock().unwrap();
            queue.pending_jobs.pop()
        };
        
        if let Some(job) = job {
            // Simulate OCR processing
            sleep(Duration::from_millis(50)).await;
            
            let mut queue = ocr_queue.lock().unwrap();
            queue.processed_jobs += 1;
        } else {
            // No jobs available, wait a bit
            sleep(Duration::from_millis(10)).await;
        }
    }
}

#[tokio::test]
async fn test_partial_download_cleanup() {
    let temp_files = Arc::new(Mutex::new(Vec::new()));
    let cancellation_token = Arc::new(AtomicBool::new(false));
    
    // Simulate creating temporary files during download
    let temp_files_clone = Arc::clone(&temp_files);
    let token_clone = Arc::clone(&cancellation_token);
    
    let download_handle = tokio::spawn(async move {
        simulate_download_with_temp_files(token_clone, temp_files_clone).await
    });
    
    // Let some temp files be created
    sleep(Duration::from_millis(100)).await;
    
    // Cancel the download
    cancellation_token.store(true, Ordering::Relaxed);
    
    let result = download_handle.await.unwrap();
    assert!(result.was_cancelled);
    
    // Check that temporary files were cleaned up
    let temp_files = temp_files.lock().unwrap();
    for temp_file in temp_files.iter() {
        assert!(!temp_file.exists, "Temporary file should be cleaned up: {}", temp_file.path);
    }
}

async fn simulate_download_with_temp_files(
    cancellation_token: Arc<AtomicBool>,
    temp_files: Arc<Mutex<Vec<TempFile>>>,
) -> DownloadResult {
    let total_files: u32 = 5;
    let mut files_downloaded: u32 = 0;
    
    for i in 0..total_files {
        if cancellation_token.load(Ordering::Relaxed) {
            // Cleanup temp files on cancellation
            cleanup_temp_files(&temp_files).await;
            
            return DownloadResult {
                was_cancelled: true,
                bytes_downloaded: files_downloaded as u64 * 1024,
                total_bytes: total_files as u64 * 1024,
                files_downloaded,
                total_files,
            };
        }
        
        // Create temp file
        let temp_file = TempFile {
            path: format!("/tmp/download_{}.tmp", i),
            exists: true,
        };
        
        temp_files.lock().unwrap().push(temp_file);
        
        // Simulate download
        sleep(Duration::from_millis(50)).await;
        files_downloaded += 1;
    }
    
    DownloadResult {
        was_cancelled: false,
        bytes_downloaded: files_downloaded as u64 * 1024,
        total_bytes: total_files as u64 * 1024,
        files_downloaded,
        total_files,
    }
}

async fn cleanup_temp_files(temp_files: &Arc<Mutex<Vec<TempFile>>>) {
    let mut files = temp_files.lock().unwrap();
    for file in files.iter_mut() {
        file.exists = false; // Simulate file deletion
    }
}

#[derive(Debug, Clone)]
struct TempFile {
    path: String,
    exists: bool,
}

#[test]
fn test_cancellation_signal_propagation() {
    use std::sync::mpsc;
    
    // Test that cancellation signals propagate through the system
    let (cancel_sender, cancel_receiver) = mpsc::channel();
    let (progress_sender, progress_receiver) = mpsc::channel();
    
    // Simulate worker thread
    let worker_handle = thread::spawn(move || {
        let mut work_done = 0;
        
        loop {
            // Check for cancellation
            if let Ok(_) = cancel_receiver.try_recv() {
                progress_sender.send(WorkerMessage::Cancelled(work_done)).unwrap();
                break;
            }
            
            // Do some work
            work_done += 1;
            progress_sender.send(WorkerMessage::Progress(work_done)).unwrap();
            
            if work_done >= 10 {
                progress_sender.send(WorkerMessage::Completed(work_done)).unwrap();
                break;
            }
            
            thread::sleep(Duration::from_millis(50));
        }
    });
    
    // Let worker do some work
    thread::sleep(Duration::from_millis(150));
    
    // Send cancellation signal
    cancel_sender.send(()).unwrap();
    
    // Wait for worker to finish
    worker_handle.join().unwrap();
    
    // Check final message
    let mut messages = Vec::new();
    while let Ok(msg) = progress_receiver.try_recv() {
        messages.push(msg);
    }
    
    assert!(!messages.is_empty(), "Should receive progress messages");
    
    // Last message should be cancellation
    if let Some(last_msg) = messages.last() {
        match last_msg {
            WorkerMessage::Cancelled(work_done) => {
                assert!(*work_done > 0, "Some work should have been done before cancellation");
                assert!(*work_done < 10, "Work should not have completed");
            },
            _ => panic!("Expected cancellation message, got: {:?}", last_msg),
        }
    }
}

#[derive(Debug, Clone)]
enum WorkerMessage {
    Progress(u32),
    Completed(u32),
    Cancelled(u32),
}

#[tokio::test]
async fn test_cancellation_timeout() {
    let cancellation_token = Arc::new(AtomicBool::new(false));
    let slow_operation_started = Arc::new(AtomicBool::new(false));
    
    let token_clone = Arc::clone(&cancellation_token);
    let started_clone = Arc::clone(&slow_operation_started);
    
    // Start a slow operation that doesn't check cancellation frequently
    let slow_handle = tokio::spawn(async move {
        started_clone.store(true, Ordering::Relaxed);
        
        // Simulate slow operation that takes time to respond to cancellation
        for _ in 0..20 {
            sleep(Duration::from_millis(100)).await;
            
            // Only check cancellation every few iterations (slow response)
            if token_clone.load(Ordering::Relaxed) {
                return "cancelled".to_string();
            }
        }
        
        "completed".to_string()
    });
    
    // Wait for operation to start
    while !slow_operation_started.load(Ordering::Relaxed) {
        sleep(Duration::from_millis(10)).await;
    }
    
    // Cancel after a short time
    sleep(Duration::from_millis(150)).await;
    cancellation_token.store(true, Ordering::Relaxed);
    
    // Set a timeout for the cancellation to take effect
    let result = timeout(Duration::from_secs(3), slow_handle).await;
    
    match result {
        Ok(Ok(status)) => {
            assert_eq!(status, "cancelled", "Operation should have been cancelled");
        },
        Ok(Err(e)) => panic!("Operation failed: {:?}", e),
        Err(_) => panic!("Operation did not respond to cancellation within timeout"),
    }
}

#[tokio::test]
async fn test_resource_cleanup_on_cancellation() {
    let resources = Arc::new(Mutex::new(ResourceTracker::new()));
    let cancellation_token = Arc::new(AtomicBool::new(false));
    
    let resources_clone = Arc::clone(&resources);
    let token_clone = Arc::clone(&cancellation_token);
    
    let work_handle = tokio::spawn(async move {
        simulate_work_with_resources(token_clone, resources_clone).await
    });
    
    // Let work allocate some resources
    sleep(Duration::from_millis(100)).await;
    
    // Check that resources were allocated
    {
        let tracker = resources.lock().unwrap();
        assert!(tracker.allocated_resources > 0, "Resources should be allocated");
    }
    
    // Cancel the work
    cancellation_token.store(true, Ordering::Relaxed);
    
    let result = work_handle.await.unwrap();
    assert!(result.was_cancelled, "Work should be cancelled");
    
    // Check that resources were cleaned up
    {
        let tracker = resources.lock().unwrap();
        assert_eq!(tracker.allocated_resources, 0, "All resources should be cleaned up");
    }
}

async fn simulate_work_with_resources(
    cancellation_token: Arc<AtomicBool>,
    resources: Arc<Mutex<ResourceTracker>>,
) -> WorkResult {
    let mut allocated_count = 0;
    
    for i in 0..10 {
        if cancellation_token.load(Ordering::Relaxed) {
            // Cleanup resources on cancellation
            cleanup_resources(&resources, allocated_count).await;
            
            return WorkResult {
                was_cancelled: true,
                work_completed: i,
            };
        }
        
        // Allocate a resource
        {
            let mut tracker = resources.lock().unwrap();
            tracker.allocate_resource();
            allocated_count += 1;
        }
        
        sleep(Duration::from_millis(50)).await;
    }
    
    // Normal completion - cleanup resources
    cleanup_resources(&resources, allocated_count).await;
    
    WorkResult {
        was_cancelled: false,
        work_completed: 10,
    }
}

async fn cleanup_resources(resources: &Arc<Mutex<ResourceTracker>>, count: u32) {
    let mut tracker = resources.lock().unwrap();
    for _ in 0..count {
        tracker.deallocate_resource();
    }
}

#[derive(Debug)]
struct ResourceTracker {
    allocated_resources: u32,
}

impl ResourceTracker {
    fn new() -> Self {
        Self {
            allocated_resources: 0,
        }
    }
    
    fn allocate_resource(&mut self) {
        self.allocated_resources += 1;
    }
    
    fn deallocate_resource(&mut self) {
        if self.allocated_resources > 0 {
            self.allocated_resources -= 1;
        }
    }
}

#[derive(Debug)]
struct WorkResult {
    was_cancelled: bool,
    work_completed: u32,
}

#[test]
fn test_cancellation_state_transitions() {
    // Test valid state transitions during cancellation
    let test_cases = vec![
        (SourceStatus::Syncing, CancellationState::Requested, SourceStatus::Syncing),
        (SourceStatus::Syncing, CancellationState::InProgress, SourceStatus::Syncing),
        (SourceStatus::Syncing, CancellationState::Completed, SourceStatus::Idle),
    ];
    
    for (initial_status, cancellation_state, expected_final_status) in test_cases {
        let final_status = apply_cancellation_state_transition(initial_status, cancellation_state.clone());
        assert_eq!(final_status, expected_final_status,
                   "Wrong final status for cancellation state: {:?}", cancellation_state);
    }
}

#[derive(Debug, Clone)]
enum CancellationState {
    Requested,
    InProgress,
    Completed,
}

fn apply_cancellation_state_transition(
    current_status: SourceStatus,
    cancellation_state: CancellationState,
) -> SourceStatus {
    match (current_status, cancellation_state) {
        (SourceStatus::Syncing, CancellationState::Completed) => SourceStatus::Idle,
        (status, _) => status, // Other transitions don't change status
    }
}

#[tokio::test]
async fn test_concurrent_cancellation_requests() {
    use std::sync::atomic::AtomicU32;
    
    let cancellation_counter = Arc::new(AtomicU32::new(0));
    let work_in_progress = Arc::new(AtomicBool::new(true));
    
    let counter_clone = Arc::clone(&cancellation_counter);
    let work_clone = Arc::clone(&work_in_progress);
    
    // Start work that will receive cancellation
    let work_handle = tokio::spawn(async move {
        while work_clone.load(Ordering::Relaxed) {
            sleep(Duration::from_millis(10)).await;
        }
        counter_clone.load(Ordering::Relaxed)
    });
    
    // Send multiple concurrent cancellation requests
    let mut cancel_handles = Vec::new();
    
    for _ in 0..5 {
        let counter = Arc::clone(&cancellation_counter);
        let work = Arc::clone(&work_in_progress);
        
        let handle = tokio::spawn(async move {
            // Simulate cancellation request
            sleep(Duration::from_millis(50)).await;
            
            let count = counter.fetch_add(1, Ordering::Relaxed);
            if count == 0 {
                // First cancellation request should stop the work
                work.store(false, Ordering::Relaxed);
            }
        });
        
        cancel_handles.push(handle);
    }
    
    // Wait for all cancellation requests
    for handle in cancel_handles {
        handle.await.unwrap();
    }
    
    // Wait for work to complete
    let final_count = work_handle.await.unwrap();
    
    // Should have received exactly 5 cancellation requests
    assert_eq!(final_count, 5, "Should receive all cancellation requests");
    
    // Work should be stopped
    assert!(!work_in_progress.load(Ordering::Relaxed), "Work should be stopped");
}

#[test]
fn test_cancellation_reason_tracking() {
    // Test tracking different reasons for cancellation
    let cancellation_reasons = vec![
        CancellationReason::UserRequested,
        CancellationReason::ServerShutdown,
        CancellationReason::NetworkError,
        CancellationReason::Timeout,
        CancellationReason::ResourceExhaustion,
    ];
    
    for reason in cancellation_reasons {
        let should_retry = should_retry_after_cancellation(&reason);
        let cleanup_priority = get_cleanup_priority(&reason);
        
        match reason {
            CancellationReason::UserRequested => {
                assert!(!should_retry, "User-requested cancellation should not retry");
                assert_eq!(cleanup_priority, CleanupPriority::Low);
            },
            CancellationReason::ServerShutdown => {
                assert!(!should_retry, "Server shutdown should not retry");
                assert_eq!(cleanup_priority, CleanupPriority::High);
            },
            CancellationReason::NetworkError => {
                assert!(should_retry, "Network errors should retry");
                assert_eq!(cleanup_priority, CleanupPriority::Medium);
            },
            CancellationReason::Timeout => {
                assert!(should_retry, "Timeouts should retry");
                assert_eq!(cleanup_priority, CleanupPriority::Medium);
            },
            CancellationReason::ResourceExhaustion => {
                assert!(should_retry, "Resource exhaustion should retry later");
                assert_eq!(cleanup_priority, CleanupPriority::High);
            },
        }
    }
}

#[derive(Debug, Clone)]
enum CancellationReason {
    UserRequested,
    ServerShutdown,
    NetworkError,
    Timeout,
    ResourceExhaustion,
}

#[derive(Debug, Clone, PartialEq)]
enum CleanupPriority {
    Low,
    Medium,
    High,
}

fn should_retry_after_cancellation(reason: &CancellationReason) -> bool {
    match reason {
        CancellationReason::UserRequested => false,
        CancellationReason::ServerShutdown => false,
        CancellationReason::NetworkError => true,
        CancellationReason::Timeout => true,
        CancellationReason::ResourceExhaustion => true,
    }
}

fn get_cleanup_priority(reason: &CancellationReason) -> CleanupPriority {
    match reason {
        CancellationReason::UserRequested => CleanupPriority::Low,
        CancellationReason::ServerShutdown => CleanupPriority::High,
        CancellationReason::NetworkError => CleanupPriority::Medium,
        CancellationReason::Timeout => CleanupPriority::Medium,
        CancellationReason::ResourceExhaustion => CleanupPriority::High,
    }
}