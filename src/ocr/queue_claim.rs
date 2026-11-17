//! Who owns a running OCR job, and how the owner is stopped.
//!
//! Every dequeue writes a fresh `claim_id` onto the job row and hands it to
//! the worker in a [`JobClaim`]. Each write the worker makes about the job --
//! its OCR text, a failed attempt, progress, completion -- locks or matches
//! the row by `id`, `claim_id` and `status = 'processing'`. A job that was
//! cancelled (its row deleted) or reset as stale and claimed again by another
//! worker (a new `claim_id`) no longer matches, and the original worker
//! writes nothing. The database is the source of truth, so this holds across
//! processes.
//!
//! While a job runs, its worker bumps `heartbeat_at`: once when it starts
//! processing, on every page of progress, and every [`HEARTBEAT_INTERVAL`]
//! from a timer. Stale-job recovery keys on the heartbeat, so a slow but live
//! run is never reset under its worker.
//!
//! Stopping a running job is the fast path on top: [`OcrQueueService::run_claimed`]
//! registers a [`CancellationToken`] per claim; a cancel in this process fires
//! it, and a heartbeat that finds the claim gone fires it too (a cancel made
//! by another process, or a stale reset). The job's future is then dropped,
//! which stops extraction at its next await point -- external tools run with
//! `kill_on_drop`, and an in-process Tesseract page finishes in the
//! background with its result discarded -- and releases the worker's slot.
//! Only extraction runs under the token: the worker's own outcome commit
//! deletes or requeues the job, so a heartbeat after it would read the claim
//! as lost; the outcome and what follows it are written outside the token.

use anyhow::{anyhow, Result};
use sqlx::{PgPool, Postgres, Row, Transaction};
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use uuid::Uuid;

use super::{OcrQueueItem, OcrQueueService};

/// How often a running job's heartbeat is bumped by its timer. Far below the
/// stale window `recover_stale_items` is run with (minutes), so a live job is
/// never mistaken for a dead one.
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

/// A worker's claim on a job: the job, its document, and the token the
/// dequeue wrote. Every write about the job is guarded by it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobClaim {
    pub job_id: Uuid,
    pub claim_id: Uuid,
    pub document_id: Uuid,
}

impl OcrQueueItem {
    /// The claim this item was dequeued with. An item read some other way
    /// (it was never claimed) has none, and no worker write may be made for it.
    pub fn claim(&self) -> Result<JobClaim> {
        let claim_id = self
            .claim_id
            .ok_or_else(|| anyhow!("OCR job {} carries no claim; it was not dequeued", self.id))?;
        Ok(JobClaim {
            job_id: self.id,
            claim_id,
            document_id: self.document_id,
        })
    }
}

/// Why a worker no longer owns its job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimLoss {
    /// The job is gone: the user cancelled it.
    Cancelled,
    /// The job is still queued or running, but not under this claim: it was
    /// reset as stale and is (or will be) run by another claim.
    Reclaimed,
}

/// Locks the claimed job row inside `tx` while the claim still owns it.
/// `Err(loss)` when it does not -- the caller must then write nothing.
pub(crate) async fn lock_claimed_job(
    tx: &mut Transaction<'_, Postgres>,
    claim: &JobClaim,
) -> Result<std::result::Result<(), ClaimLoss>> {
    let row = sqlx::query("SELECT status, claim_id FROM ocr_queue WHERE id = $1 FOR UPDATE")
        .bind(claim.job_id)
        .fetch_optional(&mut **tx)
        .await?;
    Ok(classify(
        row.map(|r| (r.get("status"), r.get("claim_id"))),
        claim,
    ))
}

/// Whether the claim owns a job whose row reads `(status, claim_id)`, or is
/// missing (`None`).
fn classify(
    row: Option<(Option<String>, Option<Uuid>)>,
    claim: &JobClaim,
) -> std::result::Result<(), ClaimLoss> {
    match row {
        None => Err(ClaimLoss::Cancelled),
        Some((status, claim_id))
            if status.as_deref() == Some("processing") && claim_id == Some(claim.claim_id) =>
        {
            Ok(())
        }
        Some(_) => Err(ClaimLoss::Reclaimed),
    }
}

/// The outcome of running a claimed job's work.
#[derive(Debug)]
pub enum JobRun<T> {
    /// The work ran to completion.
    Finished(T),
    /// The claim was lost first; the work was dropped unfinished.
    Stopped(ClaimLoss),
}

/// Cancellation tokens of the jobs running in this process, by claim: the
/// same job can be run twice here (a stale reset, then a reclaim while the
/// first run is still winding down), and each run must unregister only
/// itself.
#[derive(Clone, Default)]
pub(super) struct RunningJobs(Arc<Mutex<HashMap<Uuid, (Uuid, CancellationToken)>>>);

impl RunningJobs {
    fn register(&self, claim: &JobClaim) -> RunningJob {
        let token = CancellationToken::new();
        self.lock()
            .insert(claim.claim_id, (claim.job_id, token.clone()));
        RunningJob {
            jobs: self.clone(),
            claim_id: claim.claim_id,
            token,
        }
    }

    /// Stops every run of the given jobs in this process.
    pub(super) fn cancel(&self, job_ids: &[Uuid]) {
        for (job_id, token) in self.lock().values() {
            if job_ids.contains(job_id) {
                token.cancel();
            }
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<Uuid, (Uuid, CancellationToken)>> {
        // The map holds no invariant a panicking holder could break.
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// A run registered as running; unregisters it (and only it) when dropped.
struct RunningJob {
    jobs: RunningJobs,
    claim_id: Uuid,
    token: CancellationToken,
}

impl Drop for RunningJob {
    fn drop(&mut self) {
        self.jobs.lock().remove(&self.claim_id);
    }
}

/// Aborts the heartbeat task when the job's run ends, however it ends.
struct AbortOnDrop(tokio::task::JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Bumps the job's heartbeat. False when the claim no longer owns the job.
async fn beat(pool: &PgPool, claim: &JobClaim) -> Result<bool> {
    let result = sqlx::query(
        "UPDATE ocr_queue SET heartbeat_at = NOW() \
         WHERE id = $1 AND claim_id = $2 AND status = 'processing'",
    )
    .bind(claim.job_id)
    .bind(claim.claim_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() == 1)
}

impl OcrQueueService {
    /// Bumps the claimed job's heartbeat; false when the claim was lost.
    pub async fn heartbeat(&self, claim: &JobClaim) -> Result<bool> {
        beat(&self.pool, claim).await
    }

    /// Records page progress of the claimed job, which is also a heartbeat.
    /// A job no longer owned by the claim is left untouched.
    pub async fn update_progress(&self, claim: &JobClaim, current: i32, total: i32) -> Result<()> {
        record_progress(&self.pool, claim, current, total).await
    }

    /// Why the claim no longer owns its job, read back from the database.
    async fn claim_loss(&self, claim: &JobClaim) -> Result<ClaimLoss> {
        let row: Option<(Option<String>, Option<Uuid>)> =
            sqlx::query_as("SELECT status, claim_id FROM ocr_queue WHERE id = $1")
                .bind(claim.job_id)
                .fetch_optional(&self.pool)
                .await?;
        // A row the claim still owns means only this process's token fired
        // ahead of the cancel committing elsewhere; it is being cancelled.
        Ok(classify(row, claim).err().unwrap_or(ClaimLoss::Cancelled))
    }

    /// Runs `work` for the claimed job: marks it started (a heartbeat), keeps
    /// heartbeating while it runs, and drops it as soon as the claim is lost
    /// -- a cancel in this process, or a heartbeat that finds the job
    /// cancelled or reclaimed elsewhere. A claim already lost when this is
    /// called never starts the work.
    pub async fn run_claimed<F: Future>(
        &self,
        claim: JobClaim,
        work: F,
    ) -> Result<JobRun<F::Output>> {
        let running = self.running_jobs.register(&claim);
        if !self.heartbeat(&claim).await? {
            return Ok(JobRun::Stopped(self.claim_loss(&claim).await?));
        }
        let _heartbeat = AbortOnDrop(tokio::spawn(heartbeat_until_lost(
            self.pool.clone(),
            claim,
            self.heartbeat_interval,
            running.token.clone(),
        )));

        // Decide first, so `work` is dropped -- and stops -- before the
        // loss is looked up.
        let finished = tokio::select! {
            biased;
            _ = running.token.cancelled() => None,
            output = work => Some(output),
        };
        match finished {
            Some(output) => Ok(JobRun::Finished(output)),
            None => {
                let loss = self.claim_loss(&claim).await?;
                info!(
                    "OCR job {} for document {} stopped: {:?}",
                    claim.job_id, claim.document_id, loss
                );
                Ok(JobRun::Stopped(loss))
            }
        }
    }

    /// A copy of this service whose running jobs heartbeat every `interval`
    /// instead of [`HEARTBEAT_INTERVAL`]. It shares the running-job registry,
    /// so a cancel through either reaches jobs run by both.
    pub fn with_heartbeat_interval(mut self, interval: Duration) -> Self {
        self.heartbeat_interval = interval;
        self
    }

    /// A copy of this service that pauses for `pause` right after each job's
    /// outcome commits, before the work that follows the commit: for
    /// exercising a heartbeat or cancel landing in that window.
    #[doc(hidden)]
    #[cfg(any(test, feature = "test-utils"))]
    pub fn with_post_commit_pause(mut self, pause: Duration) -> Self {
        self.post_commit_pause = pause;
        self
    }

    /// The [`Self::with_post_commit_pause`] test hook; a no-op by default.
    #[cfg(any(test, feature = "test-utils"))]
    pub(super) async fn pause_after_commit(&self) {
        if !self.post_commit_pause.is_zero() {
            tokio::time::sleep(self.post_commit_pause).await;
        }
    }
}

/// Progress write shared by [`OcrQueueService::update_progress`] and the
/// per-page callback.
pub(super) async fn record_progress(
    pool: &PgPool,
    claim: &JobClaim,
    current: i32,
    total: i32,
) -> Result<()> {
    sqlx::query(
        "UPDATE ocr_queue SET progress_current = $3, progress_total = $4, heartbeat_at = NOW() \
         WHERE id = $1 AND claim_id = $2 AND status = 'processing'",
    )
    .bind(claim.job_id)
    .bind(claim.claim_id)
    .bind(current)
    .bind(total)
    .execute(pool)
    .await?;
    Ok(())
}

/// Heartbeats every `interval` until the claim is lost, then fires `token`.
/// A failed heartbeat (database trouble) is retried on the next tick: the
/// stale window is many intervals long.
async fn heartbeat_until_lost(
    pool: PgPool,
    claim: JobClaim,
    interval: Duration,
    token: CancellationToken,
) {
    loop {
        tokio::time::sleep(interval).await;
        match beat(&pool, &claim).await {
            Ok(true) => {}
            Ok(false) => {
                token.cancel();
                return;
            }
            Err(e) => warn!("Heartbeat of OCR job {} failed: {}", claim.job_id, e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claim() -> JobClaim {
        JobClaim {
            job_id: Uuid::new_v4(),
            claim_id: Uuid::new_v4(),
            document_id: Uuid::new_v4(),
        }
    }

    #[test]
    fn only_a_processing_row_with_the_same_claim_is_owned() {
        let c = claim();
        let processing = Some("processing".to_string());
        assert_eq!(
            classify(Some((processing.clone(), Some(c.claim_id))), &c),
            Ok(())
        );
        assert_eq!(classify(None, &c), Err(ClaimLoss::Cancelled));
        assert_eq!(
            classify(Some((processing, Some(Uuid::new_v4()))), &c),
            Err(ClaimLoss::Reclaimed)
        );
        assert_eq!(
            classify(Some((Some("pending".into()), Some(c.claim_id))), &c),
            Err(ClaimLoss::Reclaimed)
        );
        assert_eq!(
            classify(Some((Some("processing".into()), None)), &c),
            Err(ClaimLoss::Reclaimed)
        );
    }

    #[test]
    fn cancel_fires_only_registered_tokens_and_drop_unregisters() {
        let jobs = RunningJobs::default();
        let (a, b) = (claim(), claim());
        let running = jobs.register(&a);
        jobs.cancel(&[b.job_id]);
        assert!(!running.token.is_cancelled());
        jobs.cancel(&[a.job_id]);
        assert!(running.token.is_cancelled());
        drop(running);
        assert!(jobs.lock().is_empty());
    }

    /// Two runs of one job: the first, stale run's claim was reset and the
    /// job claimed again in this process. Whichever ends first must leave the
    /// other registered and cancellable.
    #[test]
    fn a_rerun_of_the_same_job_keeps_its_own_registration() {
        for stale_ends_first in [true, false] {
            let jobs = RunningJobs::default();
            let stale = claim();
            let live = JobClaim {
                claim_id: Uuid::new_v4(),
                ..stale
            };
            let stale_run = jobs.register(&stale);
            let live_run = jobs.register(&live);
            let (ended, remaining) = if stale_ends_first {
                (stale_run, live_run)
            } else {
                (live_run, stale_run)
            };
            drop(ended);
            jobs.cancel(&[stale.job_id]);
            assert!(
                remaining.token.is_cancelled(),
                "stale_ends_first={stale_ends_first}: the remaining run was unregistered"
            );
            drop(remaining);
            assert!(jobs.lock().is_empty());
        }
    }
}
