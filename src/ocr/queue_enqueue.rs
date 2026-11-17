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

    /// Whether the document has a pending or processing OCR job.
    pub async fn has_active_job(&self, document_id: Uuid) -> Result<bool> {
        let active = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM ocr_queue WHERE document_id = $1 AND status IN ('pending', 'processing'))",
        )
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
    // A pending job that has used up its attempts is never dequeued, yet it
    // would hold the unique index and make the document unqueueable for
    // good. Retire it so it cannot block this enqueue.
    sqlx::query(
        "UPDATE ocr_queue SET status = 'failed' \
         WHERE document_id = $1 AND status = 'pending' AND attempts >= max_attempts",
    )
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
