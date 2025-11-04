/*!
 * Database Guardrails for Concurrent Processing Safety
 * 
 * This module provides database transaction patterns and validation
 * mechanisms to prevent race conditions and data corruption.
 */

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;
use anyhow::Result;
use tracing::{warn, error, info};

/// Transaction-safe document operations with validation
#[derive(Clone)]
pub struct DocumentTransactionManager {
    pool: PgPool,
}

impl DocumentTransactionManager {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Update OCR results with full transaction safety and validation
    /// Sanitize text for PostgreSQL storage
    /// Removes null bytes and ensures valid UTF-8 encoding
    fn sanitize_text_for_db(text: &str) -> String {
        // Remove null bytes which PostgreSQL cannot store in TEXT fields
        let cleaned: String = text
            .chars()
            .filter(|&c| c != '\0')
            .collect();
        
        // Additional safety: ensure the string is valid UTF-8
        // (should already be, but this is defensive)
        String::from_utf8_lossy(cleaned.as_bytes()).to_string()
    }

    pub async fn update_ocr_with_validation(
        &self,
        document_id: Uuid,
        expected_filename: &str,
        ocr_text: &str,
        confidence: f64,
        word_count: i32,
        processing_time_ms: i64,
    ) -> Result<bool> {
        let mut tx = self.pool.begin().await?;
        
        // 1. Lock the document row for update
        let document = sqlx::query(
            r#"
            SELECT id, filename, ocr_status, file_size, created_at
            FROM documents 
            WHERE id = $1 
            FOR UPDATE
            "#
        )
        .bind(document_id)
        .fetch_optional(&mut *tx)
        .await?;

        let document = match document {
            Some(doc) => doc,
            None => {
                tx.rollback().await?;
                warn!("Document {} not found during OCR update", document_id);
                return Ok(false);
            }
        };

        // 2. Validate document hasn't been modified unexpectedly
        let filename: String = document.get("filename");
        if filename != expected_filename {
            tx.rollback().await?;
            error!(
                "Document {} filename mismatch: expected '{}', got '{}'", 
                document_id, expected_filename, filename
            );
            return Ok(false);
        }

        // 3. Check if OCR is already completed (prevent double processing)
        let ocr_status: Option<String> = document.get("ocr_status");
        if ocr_status.as_deref() == Some("completed") {
            tx.rollback().await?;
            warn!("Document {} OCR already completed, skipping update", document_id);
            return Ok(false);
        }

        // 4. Validate OCR data quality
        if ocr_text.is_empty() && confidence > 50.0 {
            tx.rollback().await?;
            warn!("Document {} has high confidence ({}) but empty OCR text", document_id, confidence);
            return Ok(false);
        }

        // 5. Sanitize text before database insertion
        let sanitized_text = Self::sanitize_text_for_db(ocr_text);
        
        // Log if sanitization was needed
        if sanitized_text.len() != ocr_text.len() {
            warn!(
                "Text sanitization was required for document {}: original {} chars, sanitized {} chars",
                document_id, ocr_text.len(), sanitized_text.len()
            );
        }
        
        // 6. Perform the update with additional safety checks
        let updated_rows = sqlx::query!(
            r#"
            UPDATE documents
            SET ocr_text = $2,
                ocr_status = 'completed',
                ocr_completed_at = NOW(),
                ocr_confidence = $3,
                ocr_word_count = $4,
                ocr_processing_time_ms = $5,
                updated_at = NOW()
            WHERE id = $1 
              AND ocr_status != 'completed'  -- Extra safety check
            "#,
            document_id,
            sanitized_text.as_str(),
            confidence,
            word_count,
            processing_time_ms
        )
        .execute(&mut *tx)
        .await?;

        if updated_rows.rows_affected() != 1 {
            tx.rollback().await?;
            error!("Document {} OCR update affected {} rows (expected 1)", document_id, updated_rows.rows_affected());
            return Ok(false);
        }

        // 7. Remove from OCR queue atomically
        let queue_removed = sqlx::query!(
            r#"
            DELETE FROM ocr_queue 
            WHERE document_id = $1 
              AND status = 'processing'
            "#,
            document_id
        )
        .execute(&mut *tx)
        .await?;

        if queue_removed.rows_affected() == 0 {
            warn!("Document {} not found in OCR queue during completion", document_id);
        }

        // 8. Commit transaction
        tx.commit().await?;
        
        info!(
            "Document {} OCR updated successfully: {} chars, {:.1}% confidence, {} words", 
            document_id, sanitized_text.len(), confidence, word_count
        );
        
        Ok(true)
    }

    /// Safely claim a document from OCR queue with proper locking
    pub async fn claim_ocr_job(&self, worker_id: &str) -> Result<Option<OcrJob>> {
        let mut tx = self.pool.begin().await?;

        // 1. Find and lock next available job
        let job = sqlx::query_as::<_, OcrJob>(
            r#"
            UPDATE ocr_queue
            SET status = 'processing',
                started_at = NOW(),
                worker_id = $1,
                attempts = attempts + 1
            WHERE id = (
                SELECT id
                FROM ocr_queue
                WHERE status = 'pending'
                  AND attempts < max_attempts
                ORDER BY priority DESC, created_at ASC
                FOR UPDATE SKIP LOCKED
                LIMIT 1
            )
            RETURNING 
                id,
                document_id,
                priority,
                status,
                attempts,
                max_attempts,
                worker_id,
                created_at,
                started_at,
                completed_at,
                error_message
            "#
        )
        .bind(worker_id)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(job) = job {
            // 2. Validate document still exists and is processable
            let document_exists = sqlx::query!(
                r#"
                SELECT filename, file_path, ocr_status
                FROM documents 
                WHERE id = $1 
                  AND ocr_status IN ('pending', 'processing')
                "#,
                job.document_id
            )
            .fetch_optional(&mut *tx)
            .await?;

            if document_exists.is_none() {
                // Document was deleted or already processed
                sqlx::query!(
                    "DELETE FROM ocr_queue WHERE id = $1",
                    job.id
                )
                .execute(&mut *tx)
                .await?;
                
                tx.commit().await?;
                return Ok(None);
            }

            tx.commit().await?;
            Ok(Some(job))
        } else {
            tx.rollback().await?;
            Ok(None)
        }
    }

    /// Safely handle OCR job failure with retry logic
    pub async fn handle_ocr_failure(
        &self,
        job_id: Uuid,
        document_id: Uuid,
        error_message: &str,
    ) -> Result<bool> {
        let mut tx = self.pool.begin().await?;

        // 1. Check if job should be retried or marked as failed
        let job = sqlx::query!(
            r#"
            SELECT attempts, max_attempts 
            FROM ocr_queue 
            WHERE id = $1 
            FOR UPDATE
            "#,
            job_id
        )
        .fetch_optional(&mut *tx)
        .await?;

        let should_retry = if let Some(job) = job {
            job.attempts < job.max_attempts
        } else {
            false
        };

        if should_retry {
            // 2. Reset job for retry
            sqlx::query!(
                r#"
                UPDATE ocr_queue
                SET status = 'pending',
                    worker_id = NULL,
                    started_at = NULL,
                    error_message = $2
                WHERE id = $1
                "#,
                job_id,
                error_message
            )
            .execute(&mut *tx)
            .await?;

            info!("OCR job {} scheduled for retry", job_id);
        } else {
            // 3. Mark document as failed and remove from queue
            sqlx::query!(
                r#"
                UPDATE documents
                SET ocr_status = 'failed',
                    ocr_error = $2,
                    updated_at = NOW()
                WHERE id = $1
                "#,
                document_id,
                error_message
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query!(
                "DELETE FROM ocr_queue WHERE id = $1",
                job_id
            )
            .execute(&mut *tx)
            .await?;

            error!("OCR job {} failed permanently: {}", job_id, error_message);
        }

        tx.commit().await?;
        Ok(should_retry)
    }

    /// Validate database consistency and fix orphaned records
    pub async fn validate_consistency(&self) -> Result<ConsistencyReport> {
        let mut report = ConsistencyReport::default();

        // 1. Find documents with OCR status mismatch
        let orphaned_queue_items = sqlx::query!(
            r#"
            SELECT q.id, q.document_id, d.ocr_status
            FROM ocr_queue q
            LEFT JOIN documents d ON q.document_id = d.id
            WHERE d.id IS NULL 
               OR d.ocr_status = 'completed'
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        report.orphaned_queue_items = orphaned_queue_items.len();

        // 2. Find documents stuck in processing
        let stuck_processing = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM documents
            WHERE ocr_status = 'processing'
              AND updated_at < NOW() - INTERVAL '30 minutes'
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        report.stuck_processing_docs = stuck_processing.count.unwrap_or(0) as usize;

        // 3. Find queue items without corresponding documents
        let queue_without_docs = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM ocr_queue q
            LEFT JOIN documents d ON q.document_id = d.id
            WHERE d.id IS NULL
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        report.queue_without_docs = queue_without_docs.count.unwrap_or(0) as usize;

        Ok(report)
    }

    /// Clean up orphaned and inconsistent records
    pub async fn cleanup_orphaned_records(&self) -> Result<CleanupReport> {
        let mut report = CleanupReport::default();

        // 1. Remove queue items for completed documents
        let removed_completed = sqlx::query!(
            r#"
            DELETE FROM ocr_queue
            WHERE document_id IN (
                SELECT d.id FROM documents d 
                WHERE d.ocr_status = 'completed'
            )
            "#
        )
        .execute(&self.pool)
        .await?;

        report.removed_completed_queue_items = removed_completed.rows_affected() as usize;

        // 2. Remove queue items for non-existent documents  
        let removed_orphaned = sqlx::query!(
            r#"
            DELETE FROM ocr_queue
            WHERE document_id NOT IN (
                SELECT id FROM documents
            )
            "#
        )
        .execute(&self.pool)
        .await?;

        report.removed_orphaned_queue_items = removed_orphaned.rows_affected() as usize;

        // 3. Reset stuck processing documents
        let reset_stuck = sqlx::query!(
            r#"
            UPDATE documents
            SET ocr_status = 'pending'
            WHERE ocr_status = 'processing'
              AND updated_at < NOW() - INTERVAL '30 minutes'
            "#
        )
        .execute(&self.pool)
        .await?;

        report.reset_stuck_documents = reset_stuck.rows_affected() as usize;

        Ok(report)
    }
}

/// OCR job structure with all necessary fields
#[derive(Debug, Clone)]
pub struct OcrJob {
    pub id: Uuid,
    pub document_id: Uuid,
    pub priority: i32,
    pub status: String,
    pub attempts: i32,
    pub max_attempts: i32,
    pub worker_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub error_message: Option<String>,
}

/// Database consistency validation report
#[derive(Debug, Default)]
pub struct ConsistencyReport {
    pub orphaned_queue_items: usize,
    pub stuck_processing_docs: usize,
    pub queue_without_docs: usize,
    pub is_consistent: bool,
}

impl ConsistencyReport {
    pub fn is_consistent(&self) -> bool {
        self.orphaned_queue_items == 0 
            && self.stuck_processing_docs == 0 
            && self.queue_without_docs == 0
    }
}

/// Database cleanup operation report
#[derive(Debug, Default)]
pub struct CleanupReport {
    pub removed_completed_queue_items: usize,
    pub removed_orphaned_queue_items: usize,
    pub reset_stuck_documents: usize,
}

/// Database connection health checker
pub struct DatabaseHealthChecker {
    pool: PgPool,
}

impl DatabaseHealthChecker {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Check database connection pool health
    pub async fn check_pool_health(&self) -> Result<PoolHealthReport> {
        let start = std::time::Instant::now();
        
        // Test basic connectivity
        let test_query = sqlx::query!("SELECT 1 as test")
            .fetch_one(&self.pool)
            .await?;
        
        let response_time = start.elapsed();
        
        // Get pool statistics if available
        let pool_size = self.pool.size();
        let idle_connections = self.pool.num_idle();
        
        Ok(PoolHealthReport {
            is_healthy: test_query.test == Some(1),
            response_time_ms: response_time.as_millis() as u64,
            pool_size,
            idle_connections,
            utilization_percent: if pool_size > 0 {
                ((pool_size - idle_connections) as f64 / pool_size as f64 * 100.0) as u8
            } else {
                0
            },
        })
    }
}

#[derive(Debug)]
pub struct PoolHealthReport {
    pub is_healthy: bool,
    pub response_time_ms: u64,
    pub pool_size: u32,
    pub idle_connections: u32,
    pub utilization_percent: u8,
}

/// Distributed locking for critical sections
pub struct DistributedLock {
    pool: PgPool,
}

impl DistributedLock {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Acquire a named lock with timeout
    pub async fn acquire_lock(&self, lock_name: &str, timeout_secs: i32) -> Result<bool> {
        let lock_id = self.hash_lock_name(lock_name);
        
        let result = sqlx::query!(
            "SELECT pg_try_advisory_lock($1, $2) as acquired",
            lock_id,
            timeout_secs
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result.acquired.unwrap_or(false))
    }

    /// Release a named lock
    pub async fn release_lock(&self, lock_name: &str) -> Result<bool> {
        let lock_id = self.hash_lock_name(lock_name);
        
        let result = sqlx::query!(
            "SELECT pg_advisory_unlock($1, 0) as released",
            lock_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result.released.unwrap_or(false))
    }

    fn hash_lock_name(&self, name: &str) -> i64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        hasher.finish() as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sanitize_text_for_db() {
        // Test removing null bytes
        let text_with_nulls = "Hello\0World\0!";
        let sanitized = DocumentTransactionManager::sanitize_text_for_db(text_with_nulls);
        assert_eq!(sanitized, "HelloWorld!");

        // Test preserving normal text
        let normal_text = "This is a normal PDF text with special chars: €£¥";
        let sanitized = DocumentTransactionManager::sanitize_text_for_db(normal_text);
        assert_eq!(sanitized, normal_text);

        // Test handling empty string
        let empty = "";
        let sanitized = DocumentTransactionManager::sanitize_text_for_db(empty);
        assert_eq!(sanitized, "");

        // Test handling text with multiple null bytes
        let many_nulls = "\0\0Start\0Middle\0\0End\0\0";
        let sanitized = DocumentTransactionManager::sanitize_text_for_db(many_nulls);
        assert_eq!(sanitized, "StartMiddleEnd");
    }
}