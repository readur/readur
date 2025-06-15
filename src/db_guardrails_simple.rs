/*!
 * Critical Database Guardrails for OCR Corruption Prevention
 * 
 * Simplified transaction-safe operations to prevent the FileA/FileB
 * OCR corruption issue during concurrent processing.
 */

use sqlx::{PgPool, Row};
use uuid::Uuid;
use anyhow::Result;
use tracing::{warn, error, info};

/// Simplified transaction manager focused on preventing OCR corruption
#[derive(Clone)]
pub struct DocumentTransactionManager {
    pool: PgPool,
}

impl DocumentTransactionManager {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Update OCR results with full transaction safety and validation
    /// This is the critical function that prevents FileA/FileB corruption
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
        
        // 1. Lock the document row for update to prevent race conditions
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

        // 5. Perform the atomic update with additional safety checks
        let updated_rows = sqlx::query(
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
            "#
        )
        .bind(document_id)
        .bind(ocr_text)
        .bind(confidence)
        .bind(word_count)
        .bind(processing_time_ms)
        .execute(&mut *tx)
        .await?;

        if updated_rows.rows_affected() != 1 {
            tx.rollback().await?;
            error!("Document {} OCR update affected {} rows (expected 1)", document_id, updated_rows.rows_affected());
            return Ok(false);
        }

        // 6. Remove from OCR queue atomically
        let _queue_removed = sqlx::query(
            r#"
            DELETE FROM ocr_queue 
            WHERE document_id = $1 
              AND status = 'processing'
            "#
        )
        .bind(document_id)
        .execute(&mut *tx)
        .await?;

        // Note: We don't fail if queue removal fails - it might have been cleaned up already

        // 7. Commit transaction
        tx.commit().await?;
        
        info!(
            "✅ Document {} OCR updated successfully: {} chars, {:.1}% confidence, {} words", 
            document_id, ocr_text.len(), confidence, word_count
        );
        
        Ok(true)
    }

    /// Safely handle OCR job failure with proper transaction boundaries
    pub async fn mark_ocr_failed(
        &self,
        document_id: Uuid,
        error_message: &str,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

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
        .bind(document_id)
        .bind(error_message)
        .execute(&mut *tx)
        .await?;

        // Remove from queue
        sqlx::query(
            r#"
            DELETE FROM ocr_queue 
            WHERE document_id = $1
            "#
        )
        .bind(document_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        
        error!("Document {} OCR marked as failed: {}", document_id, error_message);
        Ok(())
    }

    /// Check database consistency for monitoring
    pub async fn check_consistency(&self) -> Result<ConsistencyReport> {
        let result = sqlx::query(
            r#"
            SELECT 
                -- Orphaned queue items
                (SELECT COUNT(*) FROM ocr_queue q 
                 LEFT JOIN documents d ON q.document_id = d.id 
                 WHERE d.id IS NULL) as orphaned_queue,
                
                -- Documents stuck in processing
                (SELECT COUNT(*) FROM documents
                 WHERE ocr_status = 'processing'
                   AND updated_at < NOW() - INTERVAL '30 minutes') as stuck_processing,
                
                -- Inconsistent states
                (SELECT COUNT(*) FROM documents d
                 JOIN ocr_queue q ON d.id = q.document_id
                 WHERE d.ocr_status = 'completed' AND q.status != 'completed') as inconsistent_states
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        let orphaned: i64 = result.get("orphaned_queue");
        let stuck: i64 = result.get("stuck_processing");
        let inconsistent: i64 = result.get("inconsistent_states");

        Ok(ConsistencyReport {
            orphaned_queue_items: orphaned as i32,
            stuck_processing_docs: stuck as i32,
            inconsistent_ocr_states: inconsistent as i32,
        })
    }

    /// Clean up stuck and orphaned records
    pub async fn cleanup_stuck_records(&self) -> Result<CleanupReport> {
        let mut tx = self.pool.begin().await?;

        // Reset stuck processing documents
        let reset_stuck = sqlx::query(
            r#"
            UPDATE documents
            SET ocr_status = 'pending'
            WHERE ocr_status = 'processing'
              AND updated_at < NOW() - INTERVAL '30 minutes'
            "#
        )
        .execute(&mut *tx)
        .await?;

        // Remove orphaned queue items
        let removed_orphaned = sqlx::query(
            r#"
            DELETE FROM ocr_queue
            WHERE document_id NOT IN (SELECT id FROM documents)
            "#
        )
        .execute(&mut *tx)
        .await?;

        // Remove completed queue items
        let removed_completed = sqlx::query(
            r#"
            DELETE FROM ocr_queue
            WHERE document_id IN (
                SELECT d.id FROM documents d 
                WHERE d.ocr_status = 'completed'
            )
            "#
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        let report = CleanupReport {
            reset_stuck_documents: reset_stuck.rows_affected() as usize,
            removed_orphaned_queue_items: removed_orphaned.rows_affected() as usize,
            removed_completed_queue_items: removed_completed.rows_affected() as usize,
        };

        info!("Database cleanup completed: {:?}", report);
        Ok(report)
    }
}

/// Database consistency validation report
#[derive(Debug)]
pub struct ConsistencyReport {
    pub orphaned_queue_items: i32,
    pub stuck_processing_docs: i32,
    pub inconsistent_ocr_states: i32,
}

impl ConsistencyReport {
    pub fn is_consistent(&self) -> bool {
        self.orphaned_queue_items == 0 
            && self.stuck_processing_docs == 0 
            && self.inconsistent_ocr_states == 0
    }
}

/// Database cleanup operation report
#[derive(Debug)]
pub struct CleanupReport {
    pub reset_stuck_documents: usize,
    pub removed_orphaned_queue_items: usize,
    pub removed_completed_queue_items: usize,
}