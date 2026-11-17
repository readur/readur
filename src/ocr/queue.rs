use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, Row};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{
    db::Database, db_guardrails_simple::DocumentTransactionManager,
    monitoring::request_throttler::RequestThrottler, ocr::enhanced::EnhancedOcrService,
};

#[path = "queue_cancel.rs"]
mod cancel;
#[path = "queue_claim.rs"]
mod claim;
#[path = "queue_enqueue.rs"]
mod enqueue;
#[path = "queue_maintenance.rs"]
mod maintenance;
#[path = "queue_process.rs"]
mod process;
#[path = "queue_processed_image.rs"]
mod processed_image;
#[path = "queue_stats.rs"]
mod stats;
pub use cancel::{
    CancelOutcome, CancelledDocument, CANCELLED_ERROR_MESSAGE, CANCELLED_FAILURE_REASON,
};
pub(crate) use claim::lock_claimed_job;
pub use claim::{ClaimLoss, JobClaim, JobRun, HEARTBEAT_INTERVAL};
pub use enqueue::EnqueueOutcome;

/// Upper bound on concurrent OCR jobs, to avoid exhausting the background DB pool.
const MAX_CONCURRENT_OCR_JOBS: usize = 15;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OcrQueueItem {
    pub id: Uuid,
    pub document_id: Uuid,
    pub status: String,
    pub priority: i32,
    pub attempts: i32,
    pub max_attempts: i32,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub worker_id: Option<String>,
    pub processing_time_ms: Option<i32>,
    pub file_size: Option<i64>,
    /// The token the dequeue that claimed this job wrote; every write the
    /// worker makes about the job is guarded by it. None on an item that was
    /// never claimed.
    #[sqlx(default)]
    #[serde(default)]
    pub claim_id: Option<Uuid>,
    /// When the claiming worker last showed it is alive.
    #[sqlx(default)]
    #[serde(default)]
    pub heartbeat_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    pub pending_count: i64,
    pub processing_count: i64,
    pub failed_count: i64,
    pub completed_today: i64,
    pub avg_wait_time_minutes: Option<f64>,
    pub oldest_pending_minutes: Option<f64>,
}

#[derive(Clone)]
pub struct OcrQueueService {
    db: Database,
    pool: PgPool,
    max_concurrent_jobs: usize,
    worker_id: String,
    transaction_manager: DocumentTransactionManager,
    processing_throttler: Arc<RequestThrottler>,
    is_paused: Arc<AtomicBool>,
    file_service: std::sync::Arc<crate::services::file_service::FileService>,
    max_pdf_size_mb: u64,
    max_office_document_size_mb: u64,
    ocr_timeout_seconds: u64,
    running_jobs: claim::RunningJobs,
    heartbeat_interval: Duration,
    /// Test hook: a pause right after a job's outcome commits, before the
    /// work that follows it (the failure record, the review copy).
    post_commit_pause: Duration,
}

impl OcrQueueService {
    pub fn new(
        db: Database,
        pool: PgPool,
        max_concurrent_jobs: usize,
        file_service: std::sync::Arc<crate::services::file_service::FileService>,
        max_pdf_size_mb: u64,
        max_office_document_size_mb: u64,
        ocr_timeout_seconds: u64,
    ) -> Self {
        // Clamp once so the worker semaphore, throttler and logs agree.
        let max_concurrent_jobs = max_concurrent_jobs.clamp(1, MAX_CONCURRENT_OCR_JOBS);
        let worker_id = format!(
            "worker-{}-{}",
            hostname::get().unwrap_or_default().to_string_lossy(),
            Uuid::new_v4()
        );
        let transaction_manager = DocumentTransactionManager::new(pool.clone());

        // Create a processing throttler to limit concurrent OCR operations
        // This prevents overwhelming the database connection pool
        let processing_throttler = Arc::new(RequestThrottler::new(
            max_concurrent_jobs,
            60, // 60 second max wait time for OCR processing
            format!("ocr-processing-{}", worker_id),
        ));

        Self {
            db,
            pool,
            max_concurrent_jobs,
            worker_id,
            transaction_manager,
            processing_throttler,
            is_paused: Arc::new(AtomicBool::new(false)),
            file_service,
            max_pdf_size_mb,
            max_office_document_size_mb,
            ocr_timeout_seconds,
            running_jobs: claim::RunningJobs::default(),
            heartbeat_interval: claim::HEARTBEAT_INTERVAL,
            post_commit_pause: Duration::ZERO,
        }
    }

    /// Get the next item from the queue with atomic job claiming and retry logic
    pub async fn dequeue(&self) -> Result<Option<OcrQueueItem>> {
        crate::debug_log!("OCR_QUEUE",
            "worker_id" => &self.worker_id,
            "message" => "Starting dequeue operation"
        );

        // Retry up to 3 times for race condition scenarios
        for attempt in 1..=3 {
            crate::debug_log!("OCR_QUEUE",
                "worker_id" => &self.worker_id,
                "attempt" => attempt,
                "message" => "Attempting to dequeue job"
            );
            // Use a transaction to ensure atomic job claiming
            let mut tx = self.pool.begin().await?;

            // Step 1: Find and lock the next available job atomically
            let job_row = sqlx::query(enqueue::NEXT_CLAIMABLE_JOB_SQL)
                .fetch_optional(&mut *tx)
                .await?;

            let job_id = match job_row {
                Some(ref row) => {
                    let job_id = row.get::<Uuid, _>("id");
                    let document_id = row.get::<Uuid, _>("document_id");
                    crate::debug_log!("OCR_QUEUE",
                        "worker_id" => &self.worker_id,
                        "job_id" => job_id,
                        "document_id" => document_id,
                        "attempt" => attempt,
                        "message" => "Found pending job in queue"
                    );
                    job_id
                }
                None => {
                    crate::debug_log!("OCR_QUEUE",
                        "worker_id" => &self.worker_id,
                        "attempt" => attempt,
                        "message" => "No pending jobs found in queue"
                    );
                    // No jobs available
                    tx.rollback().await?;
                    return Ok(None);
                }
            };

            // Step 2: Atomically update the job to processing state
            let updated_rows = sqlx::query(
                r#"
            UPDATE ocr_queue
            SET status = 'processing',
                started_at = NOW(),
                heartbeat_at = NOW(),
                claim_id = $3,
                worker_id = $1,
                attempts = attempts + 1
            WHERE id = $2
              AND status = 'pending'  -- Extra safety check
            "#,
            )
            .bind(&self.worker_id)
            .bind(job_id)
            .bind(Uuid::new_v4())
            .execute(&mut *tx)
            .await?;

            if updated_rows.rows_affected() != 1 {
                // Job was claimed by another worker between SELECT and UPDATE
                crate::debug_log!("OCR_QUEUE",
                    "worker_id" => &self.worker_id,
                    "job_id" => job_id,
                    "attempt" => attempt,
                    "rows_affected" => updated_rows.rows_affected(),
                    "message" => "Job was claimed by another worker, retrying"
                );
                tx.rollback().await?;
                warn!("Job {} was claimed by another worker, retrying", job_id);
                continue; // Continue to next attempt instead of returning
            }

            crate::debug_log!("OCR_QUEUE",
                "worker_id" => &self.worker_id,
                "job_id" => job_id,
                "attempt" => attempt,
                "message" => "Successfully claimed job, updating to processing state"
            );

            // Step 2b: Also update the document's ocr_status to 'processing'
            // so the frontend sees the transition (fixes #598)
            let document_id: Uuid = job_row.as_ref().unwrap().get("document_id");
            sqlx::query(
                r#"
            UPDATE documents
            SET ocr_status = 'processing',
                updated_at = NOW()
            WHERE id = $1
              AND ocr_status = 'pending'
            "#,
            )
            .bind(document_id)
            .execute(&mut *tx)
            .await?;

            // Step 3: Get the updated job details
            let row = sqlx::query(
                r#"
            SELECT id, document_id, priority, status, attempts, max_attempts,
                   created_at, started_at, completed_at, error_message,
                   worker_id, processing_time_ms, file_size, claim_id, heartbeat_at
            FROM ocr_queue
            WHERE id = $1
            "#,
            )
            .bind(job_id)
            .fetch_one(&mut *tx)
            .await?;

            tx.commit().await?;

            // Return the successfully claimed job
            let item = OcrQueueItem {
                id: row.get("id"),
                document_id: row.get("document_id"),
                status: row.get("status"),
                priority: row.get("priority"),
                attempts: row.get("attempts"),
                max_attempts: row.get("max_attempts"),
                created_at: row.get("created_at"),
                started_at: row.get("started_at"),
                completed_at: row.get("completed_at"),
                error_message: row.get("error_message"),
                worker_id: row.get("worker_id"),
                processing_time_ms: row.get("processing_time_ms"),
                file_size: row.get("file_size"),
                claim_id: row.get("claim_id"),
                heartbeat_at: row.get("heartbeat_at"),
            };

            info!(
                "✅ Worker {} successfully claimed job {} for document {}",
                self.worker_id, item.id, item.document_id
            );

            return Ok(Some(item));
        }

        // If all retry attempts failed, return None
        Ok(None)
    }

    /// Pause OCR processing
    pub fn pause(&self) {
        self.is_paused.store(true, Ordering::SeqCst);
        info!("OCR processing paused for worker {}", self.worker_id);
    }

    /// Resume OCR processing
    pub fn resume(&self) {
        self.is_paused.store(false, Ordering::SeqCst);
        info!("OCR processing resumed for worker {}", self.worker_id);
    }

    /// Check if OCR processing is paused
    pub fn is_paused(&self) -> bool {
        self.is_paused.load(Ordering::SeqCst)
    }

    /// Start the worker loop
    pub async fn start_worker(self: Arc<Self>) -> Result<()> {
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent_jobs));
        let ocr_service = Arc::new(EnhancedOcrService::new(
            crate::config::temp_dir_string(),
            (*self.file_service).clone(),
            self.max_pdf_size_mb,
            self.max_office_document_size_mb,
            self.ocr_timeout_seconds,
        ));

        info!(
            "Starting OCR worker {} with {} concurrent jobs",
            self.worker_id, self.max_concurrent_jobs
        );

        crate::debug_log!("OCR_WORKER",
            "worker_id" => &self.worker_id,
            "max_concurrent_jobs" => self.max_concurrent_jobs,
            "message" => "OCR worker loop starting"
        );

        loop {
            // Check if processing is paused
            if self.is_paused() {
                crate::debug_log!("OCR_WORKER",
                    "worker_id" => &self.worker_id,
                    "message" => "OCR processing is paused, waiting..."
                );
                info!("OCR processing is paused, waiting...");
                sleep(Duration::from_secs(5)).await;
                continue;
            }

            crate::debug_log!("OCR_WORKER",
                "worker_id" => &self.worker_id,
                "message" => "Worker loop iteration - checking for items to process"
            );

            // Take a worker slot before claiming a job, so a claimed job
            // never waits for a slot.
            let permit = semaphore.clone().acquire_owned().await?;

            // Check for items to process
            match self.dequeue().await {
                Ok(Some(item)) => {
                    crate::debug_log!("OCR_WORKER",
                        "worker_id" => &self.worker_id,
                        "job_id" => item.id,
                        "document_id" => item.document_id,
                        "priority" => item.priority,
                        "message" => "Dequeued job, spawning processing task"
                    );
                    let self_clone = self.clone();
                    let ocr_service_clone = ocr_service.clone();
                    tokio::spawn(async move {
                        self_clone.run_job(item, &ocr_service_clone).await;
                        // The slot is freed when the job ends -- including
                        // when a cancel stops it early.
                        drop(permit);
                    });
                }
                Ok(None) => {
                    crate::debug_log!("OCR_WORKER",
                        "worker_id" => &self.worker_id,
                        "message" => "No items in queue, sleeping for 5 seconds"
                    );
                    // No items in queue or all jobs were claimed by other workers
                    // Use exponential backoff to reduce database load when queue is empty
                    sleep(Duration::from_secs(5)).await;
                }
                Err(e) => {
                    error!("Error dequeuing item: {}", e);
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    /// Runs a dequeued job with the processing throttle held.
    async fn run_job(&self, item: OcrQueueItem, ocr_service: &EnhancedOcrService) {
        let result = match self.processing_throttler.acquire_permit().await {
            Ok(_throttle_permit) => self.process_item(item, ocr_service).await,
            Err(e) => self.fail_throttled(&item, e.to_string()).await,
        };
        if let Err(e) = result {
            error!("Error processing OCR item: {}", e);
        }
    }

    /// Records a job that could not get a processing permit as a failed attempt.
    async fn fail_throttled(&self, item: &OcrQueueItem, e: String) -> Result<()> {
        error!(
            "Failed to acquire throttling permit for OCR processing: {}",
            e
        );
        let message = format!("Throttling error: {}", e);
        let attempt = cancel::FailedAttempt {
            error_message: &message,
            ..Default::default()
        };
        let _ = self
            .record_failed_attempt(&item.claim()?, item.attempts, attempt)
            .await?;
        Ok(())
    }

    /// Helper function to create failed document record from OCR failure
    async fn create_failed_document_from_ocr_error(
        &self,
        document_id: Uuid,
        failure_reason: &str,
        error_message: &str,
        retry_count: i32,
        partial: Option<cancel::PartialOcr>,
    ) -> Result<()> {
        let (ocr_text, ocr_confidence, ocr_word_count) = match partial {
            Some(p) => (Some(p.text), Some(p.confidence), Some(p.word_count)),
            None => (None, None, None),
        };
        // Query document directly from database without user restrictions (OCR service context)
        let document_row = sqlx::query(
            r#"
            SELECT id, filename, original_filename, file_path, file_size, mime_type,
                   content, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms,
                   ocr_status, ocr_error, ocr_completed_at, tags, created_at, updated_at,
                   user_id, file_hash
            FROM documents
            WHERE id = $1
            "#,
        )
        .bind(document_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = document_row {
            // Extract document data
            let user_id: Uuid = row.get("user_id");
            let filename: String = row.get("filename");
            let original_filename: String = row.get("original_filename");
            let file_path: String = row.get("file_path");
            let file_size: i64 = row.get("file_size");
            let mime_type: String = row.get("mime_type");
            let file_hash: Option<String> = row.get("file_hash");

            // Create failed document record directly
            let failed_document = crate::models::FailedDocument {
                id: Uuid::new_v4(),
                user_id,
                filename,
                original_filename: Some(original_filename),
                original_path: None,
                file_path: Some(file_path),
                file_size: Some(file_size),
                file_hash,
                mime_type: Some(mime_type),
                content: None,
                tags: Vec::new(),
                ocr_text,
                ocr_confidence,
                ocr_word_count,
                ocr_processing_time_ms: None,
                failure_reason: failure_reason.to_string(),
                failure_stage: "ocr".to_string(),
                existing_document_id: None,
                ingestion_source: "ocr_queue".to_string(),
                error_message: Some(error_message.to_string()),
                retry_count: Some(retry_count),
                last_retry_at: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            if let Err(e) = self.db.create_failed_document(failed_document).await {
                error!("Failed to create failed document record: {}", e);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "queue_tests.rs"]
mod tests;
