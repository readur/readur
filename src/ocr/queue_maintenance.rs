//! OCR queue maintenance: requeueing failed jobs, pruning completed jobs
//! and recovering jobs left behind by crashed workers.

use anyhow::Result;
use tracing::warn;

use super::OcrQueueService;

impl OcrQueueService {
    /// Requeue failed items
    pub async fn requeue_failed_items(&self) -> Result<i64> {
        tracing::debug!("Attempting to requeue failed items");

        // First check if there are any failed items to requeue
        let failed_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM ocr_queue
            WHERE status = 'failed'
              AND attempts < max_attempts
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to count failed items: {:?}", e);
            e
        })?;

        tracing::debug!("Found {} failed items eligible for requeue", failed_count);

        if failed_count == 0 {
            return Ok(0);
        }

        // Check for potential constraint violations
        let conflict_check: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM ocr_queue q1
            WHERE q1.status = 'failed'
              AND q1.attempts < q1.max_attempts
              AND EXISTS (
                  SELECT 1 FROM ocr_queue q2
                  WHERE q2.document_id = q1.document_id
                    AND q2.id != q1.id
                    AND q2.status IN ('pending', 'processing')
              )
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check for conflicts: {:?}", e);
            e
        })?;

        if conflict_check > 0 {
            tracing::warn!(
                "Found {} documents with existing pending/processing entries",
                conflict_check
            );
            // Update only items that won't violate the unique constraint
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
                  AND NOT EXISTS (
                      SELECT 1 FROM ocr_queue q2
                      WHERE q2.document_id = ocr_queue.document_id
                        AND q2.id != ocr_queue.id
                        AND q2.status IN ('pending', 'processing')
                  )
                "#,
            )
            .execute(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Database error in requeue_failed_items (with conflict check): {:?}",
                    e
                );
                e
            })?;

            let rows_affected = result.rows_affected() as i64;
            tracing::debug!(
                "Requeued {} failed items (skipped {} due to conflicts)",
                rows_affected,
                conflict_check
            );
            return Ok(rows_affected);
        }

        // No conflicts, proceed with normal update
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
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error in requeue_failed_items: {:?}", e);
            e
        })?;

        let rows_affected = result.rows_affected() as i64;
        tracing::debug!("Requeued {} failed items", rows_affected);

        Ok(rows_affected)
    }

    /// Clean up old completed items
    pub async fn cleanup_completed(&self, days_to_keep: i32) -> Result<i64> {
        let result = sqlx::query(
            r#"
            DELETE FROM ocr_queue
            WHERE status = 'completed'
              AND completed_at < NOW() - INTERVAL '1 day' * $1
            "#,
        )
        .bind(days_to_keep)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }

    /// Handle stale processing items (worker crashed). A job with no
    /// attempts left fails instead: as 'pending' it would never be dequeued
    /// yet would block the document from being queued again.
    pub async fn recover_stale_items(&self, stale_minutes: i32) -> Result<i64> {
        let result = sqlx::query(
            r#"
            UPDATE ocr_queue
            SET status = CASE WHEN attempts >= max_attempts THEN 'failed' ELSE 'pending' END,
                started_at = NULL,
                worker_id = NULL
            WHERE status = 'processing'
              AND started_at < NOW() - INTERVAL '1 minute' * $1
            "#,
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
