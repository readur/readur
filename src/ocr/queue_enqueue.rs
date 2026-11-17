//! Adding documents to the OCR queue.
//!
//! The queue holds at most one pending or processing job per document,
//! enforced only by the partial unique index
//! `idx_ocr_queue_unique_pending_document` (migration 20261116000001). Every
//! insert here lets that index arbitrate with `ON CONFLICT ... DO NOTHING`,
//! so a document that is already queued is an outcome, not an error, and
//! concurrent enqueues of one document add exactly one job without any
//! check-then-insert race.

use anyhow::Result;
use sqlx::{Postgres, Transaction};
use tracing::info;
use uuid::Uuid;

use super::OcrQueueService;

/// The SQL predicate for a job a worker will claim: pending with attempts
/// left. The dequeue query, [`OcrQueueService::has_active_job`] and the
/// retirement of exhausted jobs all build on this one definition, so what
/// counts as "queued" can never drift from what the worker actually runs.
macro_rules! claimable_job {
    () => {
        "(status = 'pending' AND attempts < max_attempts)"
    };
}

/// A job that will run or is running: claimable, or claimed by a worker.
/// A 'processing' job counts whatever its attempts, since the claim itself
/// spends one; a crashed worker's job is reset by `recover_stale_items`.
macro_rules! active_job {
    () => {
        concat!("(status = 'processing' OR ", claimable_job!(), ")")
    };
}

/// Selects and locks the next job to claim, highest priority first.
pub(super) const NEXT_CLAIMABLE_JOB_SQL: &str = concat!(
    "SELECT id, document_id, priority, status, attempts, max_attempts, \
     created_at, started_at, completed_at, error_message, \
     worker_id, processing_time_ms, file_size \
     FROM ocr_queue WHERE ",
    claimable_job!(),
    " ORDER BY priority DESC, created_at ASC FOR UPDATE SKIP LOCKED LIMIT 1"
);

const HAS_ACTIVE_JOB_SQL: &str = concat!(
    "SELECT EXISTS(SELECT 1 FROM ocr_queue WHERE document_id = $1 AND ",
    active_job!(),
    ")"
);

/// Locks the document's active jobs so a cancel can remove them. Workers
/// lock a job row before writing its outcome, so holding these locks makes a
/// cancel and a finishing worker take turns: exactly one of them wins.
pub(super) const LOCK_ACTIVE_JOBS_SQL: &str = concat!(
    "SELECT id, status FROM ocr_queue WHERE document_id = $1 AND ",
    active_job!(),
    " FOR UPDATE"
);

/// Fails the document's pending jobs that no worker will ever claim. The
/// attempt columns are nullable, and a NULL makes the claimable predicate
/// NULL rather than false, so it is coalesced: such a job is never claimed
/// and must be retired too.
const RETIRE_UNCLAIMABLE_SQL: &str = concat!(
    "UPDATE ocr_queue SET status = 'failed' \
     WHERE document_id = $1 AND status = 'pending' AND NOT COALESCE(",
    claimable_job!(),
    ", FALSE)"
);

/// What an enqueue did.
#[must_use = "an already queued document gets no new job; decide whether that matters"]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnqueueOutcome {
    /// A new job was added, with this queue id.
    Queued(Uuid),
    /// The document already had a pending or processing job, so none was
    /// added. Its OCR will still run.
    AlreadyQueued,
}

impl EnqueueOutcome {
    /// The new job's id, if one was added.
    pub fn queue_id(self) -> Option<Uuid> {
        match self {
            Self::Queued(id) => Some(id),
            Self::AlreadyQueued => None,
        }
    }
}

impl OcrQueueService {
    /// Queues a document for OCR unless it already has a pending or
    /// processing job.
    ///
    /// When a job is added, a document left at `ocr_status = 'processing'`
    /// (a worker died mid-job and its job is gone) is reset to `'pending'`
    /// in the same transaction: no job was active, so 'processing' was
    /// stale, and the worker moves it back to 'processing' when it claims
    /// the new job.
    pub async fn enqueue_document(
        &self,
        document_id: Uuid,
        priority: i32,
        file_size: i64,
    ) -> Result<EnqueueOutcome> {
        let mut tx = self.pool.begin().await?;
        let outcome = insert_job(&mut tx, document_id, priority, file_size).await?;
        tx.commit().await?;

        match outcome {
            EnqueueOutcome::Queued(_) => info!(
                "Enqueued document {} with priority {} for OCR processing",
                document_id, priority
            ),
            EnqueueOutcome::AlreadyQueued => info!(
                "Document {} is already queued for OCR; no job added",
                document_id
            ),
        }
        Ok(outcome)
    }

    /// Queues several documents in one transaction, skipping any that are
    /// already queued. Returns the ids of the jobs added.
    pub async fn enqueue_documents_batch(
        &self,
        documents: Vec<(Uuid, i32, i64)>,
    ) -> Result<Vec<Uuid>> {
        let requested = documents.len();
        let mut ids = Vec::with_capacity(requested);
        let mut tx = self.pool.begin().await?;
        for (document_id, priority, file_size) in documents {
            let outcome = insert_job(&mut tx, document_id, priority, file_size).await?;
            ids.extend(outcome.queue_id());
        }
        tx.commit().await?;

        info!(
            "Batch enqueued {} documents for OCR processing ({} already queued)",
            ids.len(),
            requested - ids.len()
        );
        Ok(ids)
    }

    /// Whether the document's OCR will run without further action: it has
    /// a job a worker will claim, or one a worker is running. A pending job
    /// with no attempts left is not active: no worker claims it, so the
    /// document needs a retry (which retires that job).
    pub async fn has_active_job(&self, document_id: Uuid) -> Result<bool> {
        let active = sqlx::query_scalar::<_, bool>(HAS_ACTIVE_JOB_SQL)
            .bind(document_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(active)
    }
}

/// Adds one job inside `tx` unless the document already has an active one.
async fn insert_job(
    tx: &mut Transaction<'_, Postgres>,
    document_id: Uuid,
    priority: i32,
    file_size: i64,
) -> Result<EnqueueOutcome> {
    // A pending job no worker will claim (its attempts are used up) would
    // still hold the unique index and make the document unqueueable for
    // good. Retire it so it cannot block this enqueue.
    sqlx::query(RETIRE_UNCLAIMABLE_SQL)
        .bind(document_id)
        .execute(&mut **tx)
        .await?;

    let inserted = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO ocr_queue (document_id, priority, file_size) VALUES ($1, $2, $3) \
         ON CONFLICT (document_id) WHERE status IN ('pending', 'processing') DO NOTHING \
         RETURNING id",
    )
    .bind(document_id)
    .bind(priority)
    .bind(file_size)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(id) = inserted else {
        return Ok(EnqueueOutcome::AlreadyQueued);
    };

    sqlx::query(
        "UPDATE documents SET ocr_status = 'pending', updated_at = NOW() \
         WHERE id = $1 AND ocr_status = 'processing'",
    )
    .bind(document_id)
    .execute(&mut **tx)
    .await?;
    Ok(EnqueueOutcome::Queued(id))
}
