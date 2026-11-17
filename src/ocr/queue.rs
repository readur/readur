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
    db::Database,
    db_guardrails_simple::DocumentTransactionManager,
    monitoring::request_throttler::RequestThrottler,
    ocr::{enhanced::EnhancedOcrService, failure_reason::classify_ocr_error},
};

#[path = "queue_enqueue.rs"]
mod enqueue;
#[path = "queue_maintenance.rs"]
mod maintenance;
#[path = "queue_processed_image.rs"]
mod processed_image;
#[path = "queue_stats.rs"]
mod stats;
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
                worker_id = $1,
                attempts = attempts + 1
            WHERE id = $2
              AND status = 'pending'  -- Extra safety check
            "#,
            )
            .bind(&self.worker_id)
            .bind(job_id)
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
                   worker_id, processing_time_ms, file_size
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

    /// Update OCR progress for a document (page X of Y)
    pub async fn update_progress(&self, document_id: Uuid, current: i32, total: i32) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE ocr_queue
            SET progress_current = $2,
                progress_total = $3
            WHERE document_id = $1
              AND status = 'processing'
            "#,
        )
        .bind(document_id)
        .bind(current)
        .bind(total)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Mark an item as completed
    async fn mark_completed(&self, item_id: Uuid, processing_time_ms: i32) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE ocr_queue
            SET status = 'completed',
                completed_at = NOW(),
                processing_time_ms = $2
            WHERE id = $1
            "#,
        )
        .bind(item_id)
        .bind(processing_time_ms)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark an item as failed
    async fn mark_failed(&self, item_id: Uuid, error: &str) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE ocr_queue
            SET status = CASE
                    WHEN attempts >= max_attempts THEN 'failed'
                    ELSE 'pending'
                END,
                error_message = $2,
                started_at = NULL,
                worker_id = NULL
            WHERE id = $1
            RETURNING status
            "#,
        )
        .bind(item_id)
        .bind(crate::db::sanitize::DbText(error))
        .fetch_one(&self.pool)
        .await?;

        let status: Option<String> = result.get("status");
        if status == Some("failed".to_string()) {
            error!(
                "OCR job {} permanently failed after max attempts: {}",
                item_id, error
            );
        }

        Ok(())
    }

    /// Process a single queue item
    pub async fn process_item(
        &self,
        item: OcrQueueItem,
        ocr_service: &EnhancedOcrService,
    ) -> Result<()> {
        let start_time = std::time::Instant::now();

        // Get document details including filename for validation
        let document = sqlx::query(
            r#"
            SELECT file_path, mime_type, user_id, filename, file_size
            FROM documents
            WHERE id = $1
            "#,
        )
        .bind(item.document_id)
        .fetch_optional(&self.pool)
        .await?;

        match document {
            Some(row) => {
                let file_path: String = row.get("file_path");
                let mime_type: String = row.get("mime_type");
                let user_id: Option<Uuid> = row.get("user_id");
                let filename: String = row.get("filename");
                let file_size: i64 = row.get("file_size");

                // Format file size for better readability
                let file_size_mb = file_size as f64 / (1024.0 * 1024.0);

                info!(
                    "Processing OCR job {} for document {} | File: '{}' | Type: {} | Size: {:.2} MB",
                    item.id, item.document_id, filename, mime_type, file_size_mb
                );
                // The owner's OCR settings (timeout, language, engine options),
                // or the defaults with the server's OCR timeout
                let stored = match user_id {
                    Some(user_id) => self.db.get_user_settings(user_id).await.ok().flatten(),
                    None => None,
                };
                let settings = crate::ocr::user_settings::settings_for_processing(
                    stored,
                    self.ocr_timeout_seconds,
                );

                // Create progress callback to report page-level OCR progress
                let progress_pool = self.pool.clone();
                let progress_doc_id = item.document_id;
                let progress_callback: Option<crate::ocr::enhanced::ProgressCallback> = Some(
                    Arc::new(move |current: i32, total: i32| {
                        let pool = progress_pool.clone();
                        tokio::spawn(async move {
                            let _ = sqlx::query(
                            "UPDATE ocr_queue SET progress_current = $2, progress_total = $3 WHERE document_id = $1 AND status = 'processing'"
                        )
                        .bind(progress_doc_id)
                        .bind(current)
                        .bind(total)
                        .execute(&pool)
                        .await;
                        });
                    }),
                );

                // Perform enhanced OCR
                match ocr_service
                    .extract_text_with_context(
                        &file_path,
                        &mime_type,
                        &filename,
                        file_size,
                        &settings,
                        progress_callback,
                    )
                    .await
                {
                    Ok(ocr_result) => {
                        // Validate OCR quality
                        if let Err(validation_error) =
                            ocr_service.validate_ocr_quality(&ocr_result, &settings)
                        {
                            let error_msg =
                                format!("OCR quality validation failed: {}", validation_error);
                            warn!("⚠️  OCR quality issues for '{}' | Job: {} | Document: {} | {:.1}% confidence | {} words",
                                  filename, item.id, item.document_id, ocr_result.confidence, ocr_result.word_count);

                            // Create failed document record using helper function
                            let _ = self
                                .create_failed_document_from_ocr_error(
                                    item.document_id,
                                    "low_ocr_confidence",
                                    &error_msg,
                                    item.attempts,
                                    Some(ocr_result.text.clone()),
                                    Some(ocr_result.confidence),
                                    Some(ocr_result.word_count as i32),
                                )
                                .await;

                            // Mark as failed for quality issues with proper failure reason
                            sqlx::query(
                                r#"
                                UPDATE documents
                                SET ocr_status = 'failed',
                                    ocr_failure_reason = 'low_ocr_confidence',
                                    ocr_error = $2,
                                    updated_at = NOW()
                                WHERE id = $1
                                "#,
                            )
                            .bind(item.document_id)
                            .bind(crate::db::sanitize::DbText(&error_msg))
                            .execute(&self.pool)
                            .await?;

                            self.mark_failed(item.id, &error_msg).await?;
                            return Ok(());
                        }

                        if !ocr_result.text.is_empty() {
                            // Use transaction-safe OCR update to prevent corruption
                            let processing_time_ms = start_time.elapsed().as_millis() as i64;

                            match self
                                .transaction_manager
                                .update_ocr_with_validation(
                                    item.document_id,
                                    &filename,
                                    &ocr_result.text,
                                    ocr_result.confidence as f64,
                                    ocr_result.word_count as i32,
                                    processing_time_ms,
                                )
                                .await
                            {
                                Ok(true) => {
                                    info!(
                                        "✅ Transaction-safe OCR update successful for document {}",
                                        item.document_id
                                    );
                                }
                                Ok(false) => {
                                    let error_msg = "OCR update failed validation (document may have been modified)";
                                    warn!("{} for document {}", error_msg, item.document_id);

                                    // Use classification function to determine proper failure reason
                                    let (failure_reason, _should_suppress) =
                                        classify_ocr_error(error_msg);

                                    // Create failed document record using helper function
                                    let _ = self
                                        .create_failed_document_from_ocr_error(
                                            item.document_id,
                                            failure_reason,
                                            error_msg,
                                            item.attempts,
                                            None,
                                            None,
                                            None,
                                        )
                                        .await;

                                    self.mark_failed(item.id, error_msg).await?;
                                    return Ok(());
                                }
                                Err(e) => {
                                    let error_msg =
                                        format!("Transaction-safe OCR update failed: {}", e);
                                    error!("{}", error_msg);

                                    // Use classification function to determine proper failure reason
                                    let (failure_reason, _should_suppress) =
                                        classify_ocr_error(&error_msg);

                                    // Create failed document record using helper function
                                    let _ = self
                                        .create_failed_document_from_ocr_error(
                                            item.document_id,
                                            failure_reason,
                                            &error_msg,
                                            item.attempts,
                                            None,
                                            None,
                                            None,
                                        )
                                        .await;

                                    self.mark_failed(item.id, &error_msg).await?;
                                    return Ok(());
                                }
                            }
                        } else {
                            // Handle empty text results - fail the document since no searchable content was extracted
                            let error_msg =
                                "No extractable text found in document (0 words)".to_string();
                            warn!("⚠️  No searchable content extracted for '{}' | Job: {} | Document: {} | 0 words",
                                  filename, item.id, item.document_id);

                            // Use classification function to determine proper failure reason
                            let (failure_reason, _should_suppress) = classify_ocr_error(&error_msg);

                            // Create failed document record using helper function
                            let _ = self
                                .create_failed_document_from_ocr_error(
                                    item.document_id,
                                    failure_reason,
                                    &error_msg,
                                    item.attempts,
                                    None,
                                    None,
                                    None,
                                )
                                .await;

                            // Mark document as failed for no extractable text
                            sqlx::query(
                                r#"
                                UPDATE documents
                                SET ocr_status = 'failed',
                                    ocr_failure_reason = 'no_extractable_text',
                                    ocr_error = $2,
                                    updated_at = NOW()
                                WHERE id = $1
                                "#,
                            )
                            .bind(item.document_id)
                            .bind(crate::db::sanitize::DbText(&error_msg))
                            .execute(&self.pool)
                            .await?;

                            self.mark_failed(item.id, &error_msg).await?;
                            return Ok(());
                        }

                        // Save processed image if setting is enabled and image was processed
                        if settings.save_processed_images {
                            if let Some(ref processed_image_path) = ocr_result.processed_image_path
                            {
                                match self
                                    .save_processed_image_for_review(
                                        item.document_id,
                                        user_id.unwrap_or_default(),
                                        &file_path,
                                        processed_image_path,
                                        &ocr_result.preprocessing_applied,
                                    )
                                    .await
                                {
                                    Ok(_) => {
                                        info!(
                                            "✅ Saved processed image for document {} for review",
                                            item.document_id
                                        );
                                    }
                                    Err(e) => {
                                        warn!(
                                            "Failed to save processed image for document {}: {}",
                                            item.document_id, e
                                        );
                                    }
                                }
                            }
                        }

                        // Clean up temporary processed image file if it exists
                        if let Some(ref temp_path) = ocr_result.processed_image_path {
                            let _ = tokio::fs::remove_file(temp_path).await;
                        }

                        let processing_time_ms = start_time.elapsed().as_millis() as i32;
                        self.mark_completed(item.id, processing_time_ms).await?;

                        info!(
                            "✅ OCR completed for '{}' | Job: {} | Document: {} | {:.1}% confidence | {} words | {}ms | Preprocessing: {:?}",
                            filename, item.id, item.document_id,
                            ocr_result.confidence, ocr_result.word_count, processing_time_ms, ocr_result.preprocessing_applied
                        );
                    }
                    Err(e) => {
                        let error_msg = format!("OCR extraction failed: {}", e);
                        let error_str = e.to_string();

                        // Classify error type and determine failure reason
                        let (failure_reason, should_suppress) = classify_ocr_error(&error_str);

                        // Use intelligent logging based on error type
                        if should_suppress {
                            // These are expected errors for certain PDF types - log at debug level
                            use tracing::debug;
                            debug!(
                                "Expected PDF processing issue for '{}' ({}): {}",
                                filename, failure_reason, e
                            );
                        } else {
                            // These are unexpected errors that may need attention
                            warn!("❌ OCR failed for '{}' | Job: {} | Document: {} | Reason: {} | Error: {}",
                                  filename, item.id, item.document_id, failure_reason, e);
                        }

                        // Create failed document record using helper function
                        let _ = self
                            .create_failed_document_from_ocr_error(
                                item.document_id,
                                failure_reason,
                                &error_msg,
                                item.attempts,
                                None,
                                None,
                                None,
                            )
                            .await;

                        // Always use 'failed' status with specific failure reason
                        sqlx::query(
                            r#"
                            UPDATE documents
                            SET ocr_status = 'failed',
                                ocr_error = $2,
                                ocr_failure_reason = $3,
                                updated_at = NOW()
                            WHERE id = $1
                            "#,
                        )
                        .bind(item.document_id)
                        .bind(crate::db::sanitize::DbText(&error_msg))
                        .bind(failure_reason)
                        .execute(&self.pool)
                        .await?;

                        self.mark_failed(item.id, &error_msg).await?;
                    }
                }
            }
            None => {
                let error_msg = "Document not found";
                self.mark_failed(item.id, error_msg).await?;
            }
        }

        Ok(())
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

                    let permit = semaphore.clone().acquire_owned().await?;
                    let self_clone = self.clone();
                    let ocr_service_clone = ocr_service.clone();

                    // Spawn task to process item with throttling
                    tokio::spawn(async move {
                        // Acquire throttling permit to prevent overwhelming the database
                        match self_clone.processing_throttler.acquire_permit().await {
                            Ok(_throttle_permit) => {
                                // Process the item with both semaphore and throttle permits held
                                if let Err(e) =
                                    self_clone.process_item(item, &ocr_service_clone).await
                                {
                                    error!("Error processing OCR item: {}", e);
                                }
                                // Permits are automatically released when dropped
                            }
                            Err(e) => {
                                error!(
                                    "Failed to acquire throttling permit for OCR processing: {}",
                                    e
                                );
                                // Mark the item as failed due to throttling
                                if let Err(mark_err) = self_clone
                                    .mark_failed(item.id, &format!("Throttling error: {}", e))
                                    .await
                                {
                                    error!(
                                        "Failed to mark item as failed after throttling error: {}",
                                        mark_err
                                    );
                                }
                            }
                        }
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

    /// Helper function to create failed document record from OCR failure
    #[allow(clippy::too_many_arguments)]
    async fn create_failed_document_from_ocr_error(
        &self,
        document_id: Uuid,
        failure_reason: &str,
        error_message: &str,
        retry_count: i32,
        ocr_text: Option<String>,
        ocr_confidence: Option<f32>,
        ocr_word_count: Option<i32>,
    ) -> Result<()> {
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
