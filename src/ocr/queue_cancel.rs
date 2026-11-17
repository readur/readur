//! Cancelling a document's OCR, and the claim-guarded record of a failed
//! attempt.
//!
//! A cancel deletes the document's active job, then signals the worker
//! running it (if it runs in this process) to stop at once; a worker in
//! another process notices on its next heartbeat. Whatever the worker is
//! doing, every write of its outcome is guarded by its claim (see
//! `queue_claim.rs`): the cancel locks the job rows before deleting them, and
//! a finishing worker locks its job row -- job row before document row, on
//! both sides -- so the two serialize without deadlocking: whichever commits
//! first wins, and the other sees that and writes nothing.
//!
//! What the document is left as depends on what it was: a document without
//! OCR text is left at `ocr_status = 'failed'` with `ocr_failure_reason =
//! 'cancelled'` -- not 'pending' (no job would ever run it), and 'failed' is
//! the state a retry already starts from. A document whose OCR had already
//! completed (a re-OCR was cancelled) stays 'completed' with its text.

use anyhow::Result;
use sqlx::{Postgres, Row, Transaction};
use tracing::{error, info};
use uuid::Uuid;

use super::{
    claim::{lock_claimed_job, ClaimLoss, JobClaim},
    enqueue::LOCK_ACTIVE_JOBS_SQL,
    OcrQueueService,
};

/// `ocr_failure_reason` of a document whose OCR the user cancelled.
pub const CANCELLED_FAILURE_REASON: &str = "cancelled";

/// `ocr_error` of a document whose OCR the user cancelled.
pub const CANCELLED_ERROR_MESSAGE: &str = "OCR cancelled by user";

/// The state a cancel leaves the document's OCR in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelledDocument {
    /// The document had no OCR text: it is now `'failed'` with reason
    /// [`CANCELLED_FAILURE_REASON`], ready to be retried.
    Failed,
    /// An earlier OCR had completed (this was a re-OCR): the document is
    /// still `'completed'` and keeps its text.
    KeptCompleted,
}

impl CancelledDocument {
    /// The document's `ocr_status` after the cancel.
    pub fn ocr_status(self) -> &'static str {
        match self {
            Self::Failed => "failed",
            Self::KeptCompleted => "completed",
        }
    }
}

/// What a cancel did.
#[must_use = "a document with nothing to cancel was left untouched; decide whether that matters"]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelOutcome {
    /// The document's active job was removed. `was_running` is true when a
    /// worker had already claimed it; that worker is stopped and writes
    /// nothing. `document` is the state the document was left in.
    Cancelled {
        was_running: bool,
        document: CancelledDocument,
    },
    /// The document had no pending or running OCR job.
    NothingToCancel,
}

impl CancelOutcome {
    /// Whether a job was cancelled.
    pub fn cancelled(self) -> bool {
        matches!(self, Self::Cancelled { .. })
    }
}

/// OCR output of a failed attempt worth keeping in its failure record.
#[derive(Debug, Clone)]
pub(super) struct PartialOcr {
    pub text: String,
    pub confidence: f32,
    pub word_count: i32,
}

/// A failed attempt of a claimed job, as [`OcrQueueService::record_failed_attempt`]
/// records it.
#[derive(Debug, Clone, Default)]
pub(super) struct FailedAttempt<'a> {
    pub error_message: &'a str,
    /// When set, the document is marked `'failed'` with this
    /// `ocr_failure_reason`; otherwise it is left as it is.
    pub document_failure_reason: Option<&'a str>,
    /// When set, a `failed_documents` record with this reason is added.
    pub record_reason: Option<&'a str>,
    pub partial: Option<PartialOcr>,
}

impl OcrQueueService {
    /// Cancels the document's queued or running OCR.
    ///
    /// A pending job is removed before any worker claims it. A running job
    /// is removed too, and its worker is stopped: at once when it runs in
    /// this process, by its next heartbeat otherwise -- and either way its
    /// claim is gone, so it writes nothing. Cancelling twice, or a document
    /// with no active job, is [`CancelOutcome::NothingToCancel`].
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
        let status: Option<String> =
            sqlx::query_scalar("SELECT ocr_status FROM documents WHERE id = $1")
                .bind(document_id)
                .fetch_optional(&mut *tx)
                .await?
                .flatten();
        tx.commit().await?;
        // Only after the commit: a stopped worker finds its claim gone.
        self.running_jobs.cancel(&ids);

        let document = if status.as_deref() == Some("completed") {
            CancelledDocument::KeptCompleted
        } else {
            CancelledDocument::Failed
        };
        info!(
            "Cancelled OCR for document {} (job was {}; document is {})",
            document_id,
            if was_running { "running" } else { "queued" },
            document.ocr_status()
        );
        Ok(CancelOutcome::Cancelled {
            was_running,
            document,
        })
    }

    /// Records a failed attempt of a claimed job: optionally marks the
    /// document failed, puts the job back to 'pending' (or 'failed' when it
    /// has no attempts left), and optionally adds a `failed_documents`
    /// record.
    ///
    /// When the claim no longer owns the job -- it was cancelled, or reset
    /// and claimed again -- nothing is written and the loss is returned, so
    /// neither a cancelled document nor another worker's job is touched.
    pub(super) async fn record_failed_attempt(
        &self,
        claim: &JobClaim,
        attempt_number: i32,
        attempt: FailedAttempt<'_>,
    ) -> Result<Result<(), ClaimLoss>> {
        let mut tx = self.pool.begin().await?;
        if let Err(loss) = lock_claimed_job(&mut tx, claim).await? {
            tx.rollback().await?;
            info!(
                "OCR job {} for document {} no longer owned ({:?}); discarding its failure",
                claim.job_id, claim.document_id, loss
            );
            return Ok(Err(loss));
        }
        let status = write_failed_attempt(&mut tx, claim, &attempt).await?;
        tx.commit().await?;

        if status.as_deref() == Some("failed") {
            error!(
                "OCR job {} permanently failed after max attempts: {}",
                claim.job_id, attempt.error_message
            );
        }
        self.record_failure(claim, attempt_number, attempt).await;
        Ok(Ok(()))
    }
}

impl OcrQueueService {
    /// Adds the attempt's `failed_documents` record, if it asks for one.
    /// Best effort: the attempt itself is already recorded on the job.
    async fn record_failure(
        &self,
        claim: &JobClaim,
        attempt_number: i32,
        attempt: FailedAttempt<'_>,
    ) {
        let Some(reason) = attempt.record_reason else {
            return;
        };
        let recorded = self
            .create_failed_document_from_ocr_error(
                claim.document_id,
                reason,
                attempt.error_message,
                attempt_number,
                attempt.partial,
            )
            .await;
        if let Err(e) = recorded {
            error!("Failed to record failed OCR attempt: {}", e);
        }
    }
}

/// Inside `tx`, which holds the claimed job's lock: marks the document
/// failed if the attempt says so, and puts the job back in the queue (or
/// fails it when it has no attempts left). Returns the job's new status.
async fn write_failed_attempt(
    tx: &mut Transaction<'_, Postgres>,
    claim: &JobClaim,
    attempt: &FailedAttempt<'_>,
) -> Result<Option<String>> {
    let error_message = crate::db::sanitize::DbText(attempt.error_message);
    if let Some(reason) = attempt.document_failure_reason {
        sqlx::query(
            "UPDATE documents SET ocr_status = 'failed', ocr_failure_reason = $3, \
             ocr_error = $2, updated_at = NOW() WHERE id = $1",
        )
        .bind(claim.document_id)
        .bind(error_message)
        .bind(reason)
        .execute(&mut **tx)
        .await?;
    }
    let status = sqlx::query_scalar(
        "UPDATE ocr_queue SET status = CASE WHEN attempts >= max_attempts \
         THEN 'failed' ELSE 'pending' END, \
         error_message = $2, started_at = NULL, worker_id = NULL, \
         claim_id = NULL, heartbeat_at = NULL \
         WHERE id = $1 RETURNING status",
    )
    .bind(claim.job_id)
    .bind(error_message)
    .fetch_one(&mut **tx)
    .await?;
    Ok(status)
}
