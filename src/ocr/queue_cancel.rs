//! Cancelling a document's OCR, and the guarded writes that make a cancel
//! safe against a worker finishing the same job.
//!
//! A cancel deletes the document's active job. A worker that is running the
//! job keeps going (Tesseract cannot be interrupted mid-page), but every
//! write of its outcome -- the OCR text, a failed attempt -- first locks the
//! job row and requires it to still be `'processing'`. The cancel locks the
//! same rows before deleting them, and both sides lock the job row before
//! the document row, so a cancel and a finishing worker serialize on the job
//! row without deadlocking: whichever commits first wins, and the other sees
//! that and writes nothing.
//!
//! A cancelled document is left at `ocr_status = 'failed'` with
//! `ocr_failure_reason = 'cancelled'`: not 'pending' (no job would ever run
//! it), and 'failed' is the state a retry already starts from, so the user
//! can queue it again. A document whose OCR already completed keeps its text.

use anyhow::Result;
use sqlx::{Postgres, Row, Transaction};
use tracing::{error, info};
use uuid::Uuid;

use super::{enqueue::LOCK_ACTIVE_JOBS_SQL, OcrQueueService};

/// `ocr_failure_reason` of a document whose OCR the user cancelled.
pub const CANCELLED_FAILURE_REASON: &str = "cancelled";

/// `ocr_error` of a document whose OCR the user cancelled.
pub const CANCELLED_ERROR_MESSAGE: &str = "OCR cancelled by user";

/// What a cancel did.
#[must_use = "a document with nothing to cancel was left untouched; decide whether that matters"]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelOutcome {
    /// The document's active job was removed. `was_running` is true when a
    /// worker had already claimed it; that worker's result is discarded.
    Cancelled { was_running: bool },
    /// The document had no pending or running OCR job.
    NothingToCancel,
}

impl CancelOutcome {
    /// Whether a job was cancelled.
    pub fn cancelled(self) -> bool {
        matches!(self, Self::Cancelled { .. })
    }
}

impl OcrQueueService {
    /// Cancels the document's queued or running OCR.
    ///
    /// A pending job is removed before any worker claims it. A running job
    /// is removed too, and the worker running it discards its result when
    /// it finishes. Cancelling twice, or a document with no active job, is
    /// [`CancelOutcome::NothingToCancel`].
    pub async fn cancel_document(&self, document_id: Uuid) -> Result<CancelOutcome> {
        let mut tx = self.pool.begin().await?;
        let jobs = sqlx::query(LOCK_ACTIVE_JOBS_SQL)
            .bind(document_id)
            .fetch_all(&mut *tx)
            .await?;
        if jobs.is_empty() {
            tx.rollback().await?;
            return Ok(CancelOutcome::NothingToCancel);
        }
        let was_running = jobs
            .iter()
            .any(|job| job.get::<Option<String>, _>("status").as_deref() == Some("processing"));
        let ids: Vec<Uuid> = jobs.iter().map(|job| job.get("id")).collect();

        sqlx::query("DELETE FROM ocr_queue WHERE id = ANY($1)")
            .bind(&ids)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "UPDATE documents SET ocr_status = 'failed', ocr_failure_reason = $2, \
             ocr_error = $3, updated_at = NOW() \
             WHERE id = $1 AND ocr_status IS DISTINCT FROM 'completed'",
        )
        .bind(document_id)
        .bind(CANCELLED_FAILURE_REASON)
        .bind(CANCELLED_ERROR_MESSAGE)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        info!(
            "Cancelled OCR for document {} (job was {})",
            document_id,
            if was_running { "running" } else { "queued" }
        );
        Ok(CancelOutcome::Cancelled { was_running })
    }

    /// Records a failed attempt of a job a worker is running: optionally
    /// marks the document failed with `document_failure_reason`, and puts
    /// the job back to 'pending' (or 'failed' when it has no attempts left).
    ///
    /// Returns false, writing nothing, when the job is no longer running --
    /// it was cancelled -- so a cancelled document never gets the worker's
    /// failure written over it.
    pub(super) async fn fail_running_job(
        &self,
        job_id: Uuid,
        document_id: Uuid,
        error_message: &str,
        document_failure_reason: Option<&str>,
    ) -> Result<bool> {
        let mut tx = self.pool.begin().await?;
        if !lock_running_job(&mut tx, job_id).await? {
            tx.rollback().await?;
            info!(
                "OCR job {} for document {} was cancelled; discarding its failure",
                job_id, document_id
            );
            return Ok(false);
        }
        if let Some(reason) = document_failure_reason {
            sqlx::query(
                "UPDATE documents SET ocr_status = 'failed', ocr_failure_reason = $3, \
                 ocr_error = $2, updated_at = NOW() WHERE id = $1",
            )
            .bind(document_id)
            .bind(crate::db::sanitize::DbText(error_message))
            .bind(reason)
            .execute(&mut *tx)
            .await?;
        }
        let status: Option<String> = sqlx::query_scalar(
            "UPDATE ocr_queue SET status = CASE WHEN attempts >= max_attempts \
             THEN 'failed' ELSE 'pending' END, \
             error_message = $2, started_at = NULL, worker_id = NULL \
             WHERE id = $1 RETURNING status",
        )
        .bind(job_id)
        .bind(crate::db::sanitize::DbText(error_message))
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;

        if status.as_deref() == Some("failed") {
            error!(
                "OCR job {} permanently failed after max attempts: {}",
                job_id, error_message
            );
        }
        Ok(true)
    }
}

/// Locks the job row inside `tx` if the job is still running; false when it
/// is gone or no longer 'processing' (cancelled).
pub(crate) async fn lock_running_job(
    tx: &mut Transaction<'_, Postgres>,
    job_id: Uuid,
) -> Result<bool> {
    let locked: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM ocr_queue WHERE id = $1 AND status = 'processing' FOR UPDATE",
    )
    .bind(job_id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(locked.is_some())
}
