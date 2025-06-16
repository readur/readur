use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, Row};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Semaphore;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{db::Database, enhanced_ocr::EnhancedOcrService, db_guardrails_simple::DocumentTransactionManager, request_throttler::RequestThrottler};

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
}

impl OcrQueueService {
    pub fn new(db: Database, pool: PgPool, max_concurrent_jobs: usize) -> Self {
        let worker_id = format!("worker-{}-{}", hostname::get().unwrap_or_default().to_string_lossy(), Uuid::new_v4());
        let transaction_manager = DocumentTransactionManager::new(pool.clone());
        
        // Create a processing throttler to limit concurrent OCR operations
        // This prevents overwhelming the database connection pool
        let processing_throttler = Arc::new(RequestThrottler::new(
            max_concurrent_jobs.min(15), // Don't exceed 15 concurrent OCR processes
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
        }
    }

    /// Add a document to the OCR queue
    pub async fn enqueue_document(&self, document_id: Uuid, priority: i32, file_size: i64) -> Result<Uuid> {
        let row = sqlx::query(
            r#"
            INSERT INTO ocr_queue (document_id, priority, file_size)
            VALUES ($1, $2, $3)
            RETURNING id
            "#
        )
        .bind(document_id)
        .bind(priority)
        .bind(file_size)
        .fetch_one(&self.pool)
        .await?;
        
        let id: Uuid = row.get("id");

        info!("Enqueued document {} with priority {} for OCR processing", document_id, priority);
        Ok(id)
    }

    /// Batch enqueue multiple documents
    pub async fn enqueue_documents_batch(&self, documents: Vec<(Uuid, i32, i64)>) -> Result<Vec<Uuid>> {
        let mut ids = Vec::new();
        
        // Use a transaction for batch insert
        let mut tx = self.pool.begin().await?;
        
        for (document_id, priority, file_size) in documents {
            let row = sqlx::query(
                r#"
                INSERT INTO ocr_queue (document_id, priority, file_size)
                VALUES ($1, $2, $3)
                RETURNING id
                "#
            )
            .bind(document_id)
            .bind(priority)
            .bind(file_size)
            .fetch_one(&mut *tx)
            .await?;
            
            let id: Uuid = row.get("id");
            ids.push(id);
        }
        
        tx.commit().await?;
        
        info!("Batch enqueued {} documents for OCR processing", ids.len());
        Ok(ids)
    }

    /// Get the next item from the queue with atomic job claiming and retry logic
    pub async fn dequeue(&self) -> Result<Option<OcrQueueItem>> {
        // Retry up to 3 times for race condition scenarios
        for attempt in 1..=3 {
            // Use a transaction to ensure atomic job claiming
            let mut tx = self.pool.begin().await?;
        
        // Step 1: Find and lock the next available job atomically
        let job_row = sqlx::query(
            r#"
            SELECT id, document_id, priority, status, attempts, max_attempts, 
                   created_at, started_at, completed_at, error_message, 
                   worker_id, processing_time_ms, file_size
            FROM ocr_queue
            WHERE status = 'pending'
              AND attempts < max_attempts
            ORDER BY priority DESC, created_at ASC
            FOR UPDATE SKIP LOCKED
            LIMIT 1
            "#
        )
        .fetch_optional(&mut *tx)
        .await?;

        let job_id = match job_row {
            Some(ref row) => row.get::<Uuid, _>("id"),
            None => {
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
            "#
        )
        .bind(&self.worker_id)
        .bind(job_id)
        .execute(&mut *tx)
        .await?;

        if updated_rows.rows_affected() != 1 {
            // Job was claimed by another worker between SELECT and UPDATE
            tx.rollback().await?;
            warn!("Job {} was claimed by another worker, retrying", job_id);
            return Ok(None);
        }

        // Step 3: Get the updated job details
        let row = sqlx::query(
            r#"
            SELECT id, document_id, priority, status, attempts, max_attempts, 
                   created_at, started_at, completed_at, error_message, 
                   worker_id, processing_time_ms, file_size
            FROM ocr_queue
            WHERE id = $1
            "#
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

        info!("✅ Worker {} successfully claimed job {} for document {}", 
              self.worker_id, item.id, item.document_id);
        
        return Ok(Some(item));
        }
        
        // If all retry attempts failed, return None
        Ok(None)
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
            "#
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
            "#
        )
        .bind(item_id)
        .bind(error)
        .fetch_one(&self.pool)
        .await?;

        let status: Option<String> = result.get("status");
        if status == Some("failed".to_string()) {
            error!("OCR job {} permanently failed after max attempts: {}", item_id, error);
        }

        Ok(())
    }

    /// Process a single queue item
    pub async fn process_item(&self, item: OcrQueueItem, ocr_service: &EnhancedOcrService) -> Result<()> {
        let start_time = std::time::Instant::now();
        
        info!("Processing OCR job {} for document {}", item.id, item.document_id);
        
        // Get document details including filename for validation
        let document = sqlx::query(
            r#"
            SELECT file_path, mime_type, user_id, filename
            FROM documents
            WHERE id = $1
            "#
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
                // Get user's OCR settings or use defaults
                let settings = if let Some(user_id) = user_id {
                    self.db.get_user_settings(user_id).await.ok().flatten()
                        .unwrap_or_else(|| crate::models::Settings::default())
                } else {
                    crate::models::Settings::default()
                };

                // Perform enhanced OCR
                match ocr_service.extract_text(&file_path, &mime_type, &settings).await {
                    Ok(ocr_result) => {
                        // Validate OCR quality
                        if !ocr_service.validate_ocr_quality(&ocr_result, &settings) {
                            let error_msg = format!("OCR quality below threshold: {:.1}% confidence, {} words", 
                                                   ocr_result.confidence, ocr_result.word_count);
                            warn!("{}", error_msg);
                            
                            // Mark as failed for quality issues
                            sqlx::query(
                                r#"
                                UPDATE documents
                                SET ocr_status = 'failed',
                                    ocr_error = $2,
                                    updated_at = NOW()
                                WHERE id = $1
                                "#
                            )
                            .bind(item.document_id)
                            .bind(&error_msg)
                            .execute(&self.pool)
                            .await?;
                            
                            self.mark_failed(item.id, &error_msg).await?;
                            return Ok(());
                        }
                        
                        if !ocr_result.text.is_empty() {
                            // Use transaction-safe OCR update to prevent corruption
                            let processing_time_ms = start_time.elapsed().as_millis() as i64;
                            
                            match self.transaction_manager.update_ocr_with_validation(
                                item.document_id,
                                &filename,
                                &ocr_result.text,
                                ocr_result.confidence as f64,
                                ocr_result.word_count as i32,
                                processing_time_ms,
                            ).await {
                                Ok(true) => {
                                    info!("✅ Transaction-safe OCR update successful for document {}", item.document_id);
                                }
                                Ok(false) => {
                                    let error_msg = "OCR update failed validation (document may have been modified)";
                                    warn!("{} for document {}", error_msg, item.document_id);
                                    self.mark_failed(item.id, error_msg).await?;
                                    return Ok(());
                                }
                                Err(e) => {
                                    let error_msg = format!("Transaction-safe OCR update failed: {}", e);
                                    error!("{}", error_msg);
                                    self.mark_failed(item.id, &error_msg).await?;
                                    return Ok(());
                                }
                            }
                        }

                        let processing_time_ms = start_time.elapsed().as_millis() as i32;
                        self.mark_completed(item.id, processing_time_ms).await?;
                        
                        info!(
                            "Successfully processed OCR job {} for document {} in {}ms - Enhanced OCR: {:.1}% confidence, {} words, Preprocessing: {:?}",
                            item.id, item.document_id, processing_time_ms, 
                            ocr_result.confidence, ocr_result.word_count, ocr_result.preprocessing_applied
                        );
                    }
                    Err(e) => {
                        let error_msg = format!("OCR extraction failed: {}", e);
                        warn!("{}", error_msg);
                        
                        // Update document status
                        sqlx::query(
                            r#"
                            UPDATE documents
                            SET ocr_status = 'failed',
                                ocr_error = $2,
                                updated_at = NOW()
                            WHERE id = $1
                            "#
                        )
                        .bind(item.document_id)
                        .bind(&error_msg)
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
        let ocr_service = Arc::new(EnhancedOcrService::new("/tmp".to_string()));
        
        info!(
            "Starting OCR worker {} with {} concurrent jobs",
            self.worker_id, self.max_concurrent_jobs
        );

        loop {
            // Check if processing is paused
            if self.is_paused() {
                info!("OCR processing is paused, waiting...");
                sleep(Duration::from_secs(5)).await;
                continue;
            }

            // Check for items to process
            match self.dequeue().await {
                Ok(Some(item)) => {
                    let permit = semaphore.clone().acquire_owned().await?;
                    let self_clone = self.clone();
                    let ocr_service_clone = ocr_service.clone();
                    
                    // Spawn task to process item with throttling
                    tokio::spawn(async move {
                        // Acquire throttling permit to prevent overwhelming the database
                        match self_clone.processing_throttler.acquire_permit().await {
                            Ok(_throttle_permit) => {
                                // Process the item with both semaphore and throttle permits held
                                if let Err(e) = self_clone.process_item(item, &ocr_service_clone).await {
                                    error!("Error processing OCR item: {}", e);
                                }
                                // Permits are automatically released when dropped
                            }
                            Err(e) => {
                                error!("Failed to acquire throttling permit for OCR processing: {}", e);
                                // Mark the item as failed due to throttling
                                if let Err(mark_err) = self_clone.mark_failed(item.id, &format!("Throttling error: {}", e)).await {
                                    error!("Failed to mark item as failed after throttling error: {}", mark_err);
                                }
                            }
                        }
                        drop(permit);
                    });
                }
                Ok(None) => {
                    // No items in queue or all jobs were claimed by other workers
                    // Use shorter sleep for high-concurrency scenarios
                    sleep(Duration::from_millis(100)).await;
                }
                Err(e) => {
                    error!("Error dequeuing item: {}", e);
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    /// Get queue statistics
    pub async fn get_stats(&self) -> Result<QueueStats> {
        let stats = sqlx::query(
            r#"
            SELECT * FROM get_ocr_queue_stats()
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(QueueStats {
            pending_count: stats.get::<Option<i64>, _>("pending_count").unwrap_or(0),
            processing_count: stats.get::<Option<i64>, _>("processing_count").unwrap_or(0),
            failed_count: stats.get::<Option<i64>, _>("failed_count").unwrap_or(0),
            completed_today: stats.get::<Option<i64>, _>("completed_today").unwrap_or(0),
            avg_wait_time_minutes: stats.get("avg_wait_time_minutes"),
            oldest_pending_minutes: stats.get("oldest_pending_minutes"),
        })
    }

    /// Requeue failed items
    pub async fn requeue_failed_items(&self) -> Result<i64> {
        let result = sqlx::query(
            r#"
            UPDATE ocr_queue
            SET status = 'pending',
                attempts = 0,
                error_message = NULL,
                started_at = NULL,
                worker_id = NULL
            WHERE status = 'failed'
              AND attempts < max_attempts
            "#
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }

    /// Clean up old completed items
    pub async fn cleanup_completed(&self, days_to_keep: i32) -> Result<i64> {
        let result = sqlx::query(
            r#"
            DELETE FROM ocr_queue
            WHERE status = 'completed'
              AND completed_at < NOW() - INTERVAL '1 day' * $1
            "#
        )
        .bind(days_to_keep)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }

    /// Handle stale processing items (worker crashed)
    pub async fn recover_stale_items(&self, stale_minutes: i32) -> Result<i64> {
        let result = sqlx::query(
            r#"
            UPDATE ocr_queue
            SET status = 'pending',
                started_at = NULL,
                worker_id = NULL
            WHERE status = 'processing'
              AND started_at < NOW() - INTERVAL '1 minute' * $1
            "#
        )
        .bind(stale_minutes)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() > 0 {
            warn!("Recovered {} stale OCR jobs", result.rows_affected());
        }

        Ok(result.rows_affected() as i64)
    }
}