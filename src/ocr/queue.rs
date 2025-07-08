use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, Row, Column};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Semaphore;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{db::Database, ocr::enhanced::EnhancedOcrService, db_guardrails_simple::DocumentTransactionManager, monitoring::request_throttler::RequestThrottler};

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
        crate::debug_log!("OCR_QUEUE",
            "document_id" => document_id,
            "priority" => priority,
            "file_size" => file_size,
            "message" => "Enqueueing document"
        );
        
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
        .await
        .map_err(|e| {
            crate::debug_error!("OCR_QUEUE", format!("Failed to insert document {} into queue: {}", document_id, e));
            e
        })?;
        
        let id: Uuid = row.get("id");

        crate::debug_log!("OCR_QUEUE",
            "document_id" => document_id,
            "queue_id" => id,
            "priority" => priority,
            "file_size" => file_size,
            "message" => "Successfully enqueued document"
        );
        
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
            },
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
            "#
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
        
        // Get document details including filename for validation
        let document = sqlx::query(
            r#"
            SELECT file_path, mime_type, user_id, filename, file_size
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
                let file_size: i64 = row.get("file_size");
                
                // Format file size for better readability
                let file_size_mb = file_size as f64 / (1024.0 * 1024.0);
                
                info!(
                    "Processing OCR job {} for document {} | File: '{}' | Type: {} | Size: {:.2} MB", 
                    item.id, item.document_id, filename, mime_type, file_size_mb
                );
                // Get user's OCR settings or use defaults
                let settings = if let Some(user_id) = user_id {
                    self.db.get_user_settings(user_id).await.ok().flatten()
                        .unwrap_or_else(|| crate::models::Settings::default())
                } else {
                    crate::models::Settings::default()
                };

                // Perform enhanced OCR
                match ocr_service.extract_text_with_context(&file_path, &mime_type, &filename, file_size, &settings).await {
                    Ok(ocr_result) => {
                        // Validate OCR quality
                        if !ocr_service.validate_ocr_quality(&ocr_result, &settings) {
                            let error_msg = format!("OCR quality below threshold: {:.1}% confidence, {} words", 
                                                   ocr_result.confidence, ocr_result.word_count);
                            warn!("⚠️  OCR quality issues for '{}' | Job: {} | Document: {} | {:.1}% confidence | {} words", 
                                  filename, item.id, item.document_id, ocr_result.confidence, ocr_result.word_count);
                            
                            // Create failed document record using helper function
                            let _ = self.create_failed_document_from_ocr_error(
                                item.document_id,
                                "low_ocr_confidence",
                                &error_msg,
                                item.attempts,
                            ).await;

                            // Mark as failed for quality issues with proper failure reason
                            sqlx::query(
                                r#"
                                UPDATE documents
                                SET ocr_status = 'failed',
                                    ocr_failure_reason = 'low_ocr_confidence',
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
                                    
                                    // Create failed document record using helper function
                                    let _ = self.create_failed_document_from_ocr_error(
                                        item.document_id,
                                        "processing",
                                        error_msg,
                                        item.attempts,
                                    ).await;
                                    
                                    self.mark_failed(item.id, error_msg).await?;
                                    return Ok(());
                                }
                                Err(e) => {
                                    let error_msg = format!("Transaction-safe OCR update failed: {}", e);
                                    error!("{}", error_msg);
                                    
                                    // Create failed document record using helper function
                                    let _ = self.create_failed_document_from_ocr_error(
                                        item.document_id,
                                        "processing",
                                        &error_msg,
                                        item.attempts,
                                    ).await;
                                    
                                    self.mark_failed(item.id, &error_msg).await?;
                                    return Ok(());
                                }
                            }
                        } else {
                            // Handle empty text results - fail the document since no searchable content was extracted
                            let error_msg = format!("No extractable text found in document (0 words)");
                            warn!("⚠️  No searchable content extracted for '{}' | Job: {} | Document: {} | 0 words", 
                                  filename, item.id, item.document_id);
                            
                            // Create failed document record using helper function
                            let _ = self.create_failed_document_from_ocr_error(
                                item.document_id,
                                "no_extractable_text",
                                &error_msg,
                                item.attempts,
                            ).await;

                            // Mark document as failed for no extractable text
                            sqlx::query(
                                r#"
                                UPDATE documents
                                SET ocr_status = 'failed',
                                    ocr_failure_reason = 'no_extractable_text',
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

                        // Save processed image if setting is enabled and image was processed
                        if settings.save_processed_images {
                            if let Some(ref processed_image_path) = ocr_result.processed_image_path {
                                match self.save_processed_image_for_review(
                                    item.document_id,
                                    user_id.unwrap_or_default(),
                                    &file_path,
                                    processed_image_path,
                                    &ocr_result.preprocessing_applied,
                                ).await {
                                    Ok(_) => {
                                        info!("✅ Saved processed image for document {} for review", item.document_id);
                                    }
                                    Err(e) => {
                                        warn!("Failed to save processed image for document {}: {}", item.document_id, e);
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
                        let (failure_reason, should_suppress) = Self::classify_ocr_error(&error_str);
                        
                        // Use intelligent logging based on error type
                        if should_suppress {
                            // These are expected errors for certain PDF types - log at debug level
                            use tracing::debug;
                            debug!("Expected PDF processing issue for '{}' ({}): {}", 
                                   filename, failure_reason, e);
                        } else {
                            // These are unexpected errors that may need attention
                            warn!("❌ OCR failed for '{}' | Job: {} | Document: {} | Reason: {} | Error: {}", 
                                  filename, item.id, item.document_id, failure_reason, e);
                        }
                        
                        // Create failed document record using helper function
                        let _ = self.create_failed_document_from_ocr_error(
                            item.document_id,
                            failure_reason,
                            &error_msg,
                            item.attempts,
                        ).await;
                        
                        // Always use 'failed' status with specific failure reason
                        sqlx::query(
                            r#"
                            UPDATE documents
                            SET ocr_status = 'failed',
                                ocr_error = $2,
                                ocr_failure_reason = $3,
                                updated_at = NOW()
                            WHERE id = $1
                            "#
                        )
                        .bind(item.document_id)
                        .bind(&error_msg)
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
        let ocr_service = Arc::new(EnhancedOcrService::new("/tmp".to_string()));
        
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

    /// Save processed image for review when the setting is enabled
    async fn save_processed_image_for_review(
        &self,
        document_id: Uuid,
        user_id: Uuid,
        original_image_path: &str,
        processed_image_path: &str,
        processing_steps: &[String],
    ) -> Result<()> {
        use std::path::Path;
        
        // Use the FileService to get the proper processed images directory
        use crate::services::file_service::FileService;
        let base_upload_dir = std::env::var("UPLOAD_PATH").unwrap_or_else(|_| "uploads".to_string());
        let file_service = FileService::new(base_upload_dir);
        let processed_images_dir = file_service.get_processed_images_path();
        
        // Ensure the directory exists with proper error handling
        if let Err(e) = tokio::fs::create_dir_all(&processed_images_dir).await {
            error!("Failed to create processed images directory {:?}: {}", processed_images_dir, e);
            return Err(anyhow::anyhow!("Failed to create processed images directory: {}", e));
        }
        
        info!("Ensured processed images directory exists: {:?}", processed_images_dir);
        
        // Generate a unique filename for the processed image
        let file_stem = Path::new(processed_image_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("processed");
        let extension = Path::new(processed_image_path)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("jpg");
        
        let permanent_filename = format!("{}_processed_{}.{}", document_id, chrono::Utc::now().timestamp(), extension);
        let permanent_path = processed_images_dir.join(&permanent_filename);
        
        // Verify source file exists before copying
        if !Path::new(processed_image_path).exists() {
            return Err(anyhow::anyhow!("Source processed image file does not exist: {}", processed_image_path));
        }
        
        // Copy the processed image to permanent location with error handling
        if let Err(e) = tokio::fs::copy(processed_image_path, &permanent_path).await {
            error!("Failed to copy processed image from {} to {:?}: {}", processed_image_path, permanent_path, e);
            return Err(anyhow::anyhow!("Failed to copy processed image: {}", e));
        }
        
        info!("Successfully copied processed image to: {:?}", permanent_path);
        
        // Get actual image dimensions and file size
        let image_metadata = tokio::fs::metadata(&permanent_path).await
            .map_err(|e| anyhow::anyhow!("Failed to get processed image metadata: {}", e))?;
        let file_size = image_metadata.len() as i64;
        
        // Get image dimensions using image crate
        let (image_width, image_height) = tokio::task::spawn_blocking({
            let path = permanent_path.clone();
            move || -> Result<(u32, u32), anyhow::Error> {
                let img = image::open(&path)
                    .map_err(|e| anyhow::anyhow!("Failed to open processed image for dimensions: {}", e))?;
                Ok((img.width(), img.height()))
            }
        }).await
        .map_err(|e| anyhow::anyhow!("Failed to get image dimensions: {}", e))??;
        
        // Save to database
        let processing_parameters = serde_json::json!({
            "steps": processing_steps,
            "timestamp": chrono::Utc::now(),
            "original_path": original_image_path,
        });
        
        // Save metadata to database with error handling
        if let Err(e) = sqlx::query(
            r#"
            INSERT INTO processed_images (document_id, user_id, original_image_path, processed_image_path, processing_parameters, processing_steps, image_width, image_height, file_size, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
            "#
        )
        .bind(document_id)
        .bind(user_id)
        .bind(original_image_path)
        .bind(permanent_path.to_string_lossy().as_ref())
        .bind(&processing_parameters)
        .bind(processing_steps)
        .bind(image_width as i32)
        .bind(image_height as i32)
        .bind(file_size)
        .execute(&self.pool)
        .await {
            error!("Failed to save processed image metadata to database for document {}: {}", document_id, e);
            
            // Clean up the copied file if database save fails
            if let Err(cleanup_err) = tokio::fs::remove_file(&permanent_path).await {
                warn!("Failed to clean up processed image file after database error: {}", cleanup_err);
            }
            
            return Err(anyhow::anyhow!("Failed to save processed image metadata: {}", e));
        }
        
        info!("Successfully saved processed image metadata for document {} to database", document_id);
        
        Ok(())
    }

    /// Get queue statistics
    pub async fn get_stats(&self) -> Result<QueueStats> {
        tracing::debug!("OCR Queue: Starting get_stats() call");
        
        // First, let's check the function signature/return type
        let function_info = sqlx::query(
            r#"
            SELECT 
                p.proname as function_name,
                pg_get_function_result(p.oid) as return_type,
                pg_get_function_arguments(p.oid) as arguments
            FROM pg_proc p
            JOIN pg_namespace n ON p.pronamespace = n.oid
            WHERE n.nspname = 'public' AND p.proname = 'get_ocr_queue_stats'
            "#
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get function info: {}", e);
            e
        })?;

        if let Some(info) = function_info {
            let function_name: String = info.get("function_name");
            let return_type: String = info.get("return_type");
            let arguments: String = info.get("arguments");
            tracing::debug!("Function info - name: {}, return_type: {}, arguments: {}", function_name, return_type, arguments);
        } else {
            tracing::error!("get_ocr_queue_stats function not found!");
            return Err(anyhow::anyhow!("get_ocr_queue_stats function not found"));
        }

        tracing::debug!("OCR Queue: Calling get_ocr_queue_stats() function");
        
        let stats = sqlx::query(
            r#"
            SELECT * FROM get_ocr_queue_stats()
            "#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get OCR queue stats: {}", e);
            tracing::debug!("This indicates a function structure mismatch error");
            e
        })?;

        tracing::debug!("OCR Queue: Successfully got result from function, analyzing structure...");
        
        // Debug the actual columns returned
        let columns = stats.columns();
        tracing::debug!("Function returned {} columns:", columns.len());
        for (i, column) in columns.iter().enumerate() {
            let column_name = column.name();
            let column_type = column.type_info();
            tracing::debug!("  Column {}: name='{}', type='{:?}'", i, column_name, column_type);
        }

        // Try to extract values with detailed logging
        tracing::debug!("Attempting to extract pending_count...");
        let pending_count = match stats.try_get::<i64, _>("pending_count") {
            Ok(val) => {
                tracing::debug!("Successfully got pending_count: {}", val);
                val
            }
            Err(e) => {
                tracing::error!("Failed to get pending_count: {}", e);
                tracing::debug!("Trying different type for pending_count...");
                stats.try_get::<Option<i64>, _>("pending_count")
                    .map_err(|e2| {
                        tracing::error!("Also failed with Option<i64>: {}", e2);
                        e
                    })?
                    .unwrap_or(0)
            }
        };

        tracing::debug!("Attempting to extract processing_count...");
        let processing_count = match stats.try_get::<i64, _>("processing_count") {
            Ok(val) => {
                tracing::debug!("Successfully got processing_count: {}", val);
                val
            }
            Err(e) => {
                tracing::error!("Failed to get processing_count: {}", e);
                stats.try_get::<Option<i64>, _>("processing_count")?.unwrap_or(0)
            }
        };

        tracing::debug!("Attempting to extract failed_count...");
        let failed_count = match stats.try_get::<i64, _>("failed_count") {
            Ok(val) => {
                tracing::debug!("Successfully got failed_count: {}", val);
                val
            }
            Err(e) => {
                tracing::error!("Failed to get failed_count: {}", e);
                stats.try_get::<Option<i64>, _>("failed_count")?.unwrap_or(0)
            }
        };

        tracing::debug!("Attempting to extract completed_today...");
        let completed_today = match stats.try_get::<i64, _>("completed_today") {
            Ok(val) => {
                tracing::debug!("Successfully got completed_today: {}", val);
                val
            }
            Err(e) => {
                tracing::error!("Failed to get completed_today: {}", e);
                stats.try_get::<Option<i64>, _>("completed_today")?.unwrap_or(0)
            }
        };

        tracing::debug!("Attempting to extract avg_wait_time_minutes...");
        let avg_wait_time_minutes = match stats.try_get::<Option<f64>, _>("avg_wait_time_minutes") {
            Ok(val) => {
                tracing::debug!("Successfully got avg_wait_time_minutes: {:?}", val);
                val
            }
            Err(e) => {
                tracing::error!("Failed to get avg_wait_time_minutes: {}", e);
                // Try as string and convert
                match stats.try_get::<Option<String>, _>("avg_wait_time_minutes") {
                    Ok(Some(str_val)) => {
                        let float_val = str_val.parse::<f64>().ok();
                        tracing::debug!("Converted string '{}' to f64: {:?}", str_val, float_val);
                        float_val
                    }
                    Ok(None) => None,
                    Err(e2) => {
                        tracing::error!("Also failed with String: {}", e2);
                        return Err(anyhow::anyhow!("Failed to get avg_wait_time_minutes: {}", e));
                    }
                }
            }
        };

        tracing::debug!("Attempting to extract oldest_pending_minutes...");
        let oldest_pending_minutes = match stats.try_get::<Option<f64>, _>("oldest_pending_minutes") {
            Ok(val) => {
                tracing::debug!("Successfully got oldest_pending_minutes: {:?}", val);
                val
            }
            Err(e) => {
                tracing::error!("Failed to get oldest_pending_minutes: {}", e);
                // Try as string and convert
                match stats.try_get::<Option<String>, _>("oldest_pending_minutes") {
                    Ok(Some(str_val)) => {
                        let float_val = str_val.parse::<f64>().ok();
                        tracing::debug!("Converted string '{}' to f64: {:?}", str_val, float_val);
                        float_val
                    }
                    Ok(None) => None,
                    Err(e2) => {
                        tracing::error!("Also failed with String: {}", e2);
                        return Err(anyhow::anyhow!("Failed to get oldest_pending_minutes: {}", e));
                    }
                }
            }
        };

        tracing::debug!("OCR Queue: Successfully extracted all values, creating QueueStats");

        Ok(QueueStats {
            pending_count,
            processing_count,
            failed_count,
            completed_today,
            avg_wait_time_minutes,
            oldest_pending_minutes,
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

    /// Helper function to create failed document record from OCR failure
    async fn create_failed_document_from_ocr_error(
        &self,
        document_id: Uuid,
        failure_reason: &str,
        error_message: &str,
        retry_count: i32,
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
            "#
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
                ocr_text: None,
                ocr_confidence: None,
                ocr_word_count: None,
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

    /// Helper function to map OCR error strings to standardized failure reasons
    fn classify_ocr_error(error_str: &str) -> (&'static str, bool) {
        if error_str.contains("font encoding") || error_str.contains("missing unicode map") {
            ("pdf_parsing_error", true)  // Font encoding issues are PDF parsing problems
        } else if error_str.contains("corrupted internal structure") || error_str.contains("corrupted") {
            ("file_corrupted", true)     // Corrupted files should use file_corrupted
        } else if error_str.contains("timeout") || error_str.contains("timed out") {
            ("ocr_timeout", false)
        } else if error_str.contains("memory") || error_str.contains("out of memory") {
            ("ocr_memory_limit", false)
        } else if error_str.contains("panic") {
            ("pdf_parsing_error", true)
        } else if error_str.contains("unsupported") {
            ("unsupported_format", false)
        } else if error_str.contains("too large") || error_str.contains("file size") {
            ("file_too_large", false)
        } else {
            ("other", false)
        }
    }
}