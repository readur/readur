//! Running one claimed OCR job: extraction under the job's claim, then
//! writing its outcome.
//!
//! Every outcome write is claim-guarded (see `queue_claim.rs`), every failed
//! attempt goes through [`OcrQueueService::record_failed_attempt`], and the
//! preprocessed image extraction may leave behind is owned by a
//! [`TempPath`] that removes it on every way out.

use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::{
    cancel::{FailedAttempt, PartialOcr},
    claim::{record_progress, ClaimLoss, JobClaim, JobRun},
    OcrQueueItem, OcrQueueService,
};
use crate::{
    db_guardrails_simple::{OcrWrite, OcrWriteOutcome},
    models::Settings,
    ocr::{
        enhanced::{EnhancedOcrService, OcrResult, ProgressCallback},
        failure_reason::classify_ocr_error,
    },
    utils::temp_files::TempPath,
};

/// The temporary preprocessed image of an OCR result; removed when dropped.
fn temp_image(result: &OcrResult) -> Option<TempPath> {
    result.processed_image_path.clone().map(TempPath::new)
}

/// A claimed job with its document loaded.
struct LoadedJob {
    claim: JobClaim,
    attempts: i32,
    file_path: String,
    mime_type: String,
    user_id: Option<Uuid>,
    filename: String,
    file_size: i64,
    settings: Settings,
    started: std::time::Instant,
}

impl OcrQueueService {
    /// Processes a claimed queue item: extracts its text and writes the
    /// outcome, all under the item's claim. A claim lost before or during
    /// the run (a cancel, or a stale reset and reclaim) stops it and writes
    /// nothing.
    pub async fn process_item(
        &self,
        item: OcrQueueItem,
        ocr_service: &EnhancedOcrService,
    ) -> Result<()> {
        let claim = item.claim()?;
        let job = match self.load_job(&item).await? {
            Some(job) => job,
            None => {
                let missing = FailedAttempt {
                    error_message: "Document not found",
                    ..Default::default()
                };
                let _ = self
                    .record_failed_attempt(&claim, item.attempts, missing)
                    .await?;
                return Ok(());
            }
        };
        info!(
            "Processing OCR job {} for document {} | File: '{}' | Type: {} | Size: {:.2} MB",
            item.id,
            item.document_id,
            job.filename,
            job.mime_type,
            job.file_size as f64 / (1024.0 * 1024.0)
        );

        let work = async {
            let extracted = ocr_service
                .extract_text_with_context(
                    &job.file_path,
                    &job.mime_type,
                    &job.filename,
                    job.file_size,
                    &job.settings,
                    Some(self.progress_callback(claim)),
                )
                .await;
            self.finish(&job, ocr_service, extracted).await
        };
        match self.run_claimed(claim, work).await? {
            JobRun::Finished(result) => result,
            JobRun::Stopped(loss) => {
                info!(
                    "OCR job {} for document {} stopped before finishing ({:?}); nothing written",
                    item.id, item.document_id, loss
                );
                Ok(())
            }
        }
    }

    /// Writes the outcome of an extraction that already ran for a claimed
    /// item, exactly as [`Self::process_item`] does after extracting: for
    /// driving each outcome (and its cleanup) directly.
    #[doc(hidden)]
    pub async fn finish_extraction(
        &self,
        item: &OcrQueueItem,
        ocr_service: &EnhancedOcrService,
        extracted: Result<OcrResult>,
    ) -> Result<()> {
        let _temp = extracted.as_ref().ok().and_then(temp_image);
        match self.load_job(item).await? {
            Some(job) => self.finish(&job, ocr_service, extracted).await,
            None => Ok(()),
        }
    }

    async fn load_job(&self, item: &OcrQueueItem) -> Result<Option<LoadedJob>> {
        let row: Option<(String, String, Option<Uuid>, String, i64)> = sqlx::query_as(
            "SELECT file_path, mime_type, user_id, filename, file_size FROM documents WHERE id = $1",
        )
        .bind(item.document_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some((file_path, mime_type, user_id, filename, file_size)) = row else {
            return Ok(None);
        };
        // The owner's OCR settings (timeout, language, engine options), or
        // the defaults with the server's OCR timeout.
        let stored = match user_id {
            Some(user_id) => self.db.get_user_settings(user_id).await.ok().flatten(),
            None => None,
        };
        let settings =
            crate::ocr::user_settings::settings_for_processing(stored, self.ocr_timeout_seconds);
        Ok(Some(LoadedJob {
            claim: item.claim()?,
            attempts: item.attempts,
            file_path,
            mime_type,
            user_id,
            filename,
            file_size,
            settings,
            started: std::time::Instant::now(),
        }))
    }

    /// Page progress, written under the claim (and a heartbeat).
    fn progress_callback(&self, claim: JobClaim) -> ProgressCallback {
        let pool = self.pool.clone();
        Arc::new(move |current: i32, total: i32| {
            let pool = pool.clone();
            tokio::spawn(async move {
                if let Err(e) = record_progress(&pool, &claim, current, total).await {
                    debug!("OCR progress of job {} not recorded: {}", claim.job_id, e);
                }
            });
        })
    }

    async fn finish(
        &self,
        job: &LoadedJob,
        ocr_service: &EnhancedOcrService,
        extracted: Result<OcrResult>,
    ) -> Result<()> {
        let result = match extracted {
            Ok(result) => result,
            Err(e) => return self.extraction_failed(job, &e).await,
        };
        // Removes the preprocessed image however this returns.
        let _temp = temp_image(&result);

        if let Err(validation_error) = ocr_service.validate_ocr_quality(&result, &job.settings) {
            let error_msg = format!("OCR quality validation failed: {}", validation_error);
            warn!(
                "OCR quality issues for '{}' | Job: {} | Document: {} | {:.1}% confidence | {} words",
                job.filename, job.claim.job_id, job.claim.document_id, result.confidence, result.word_count
            );
            let attempt = FailedAttempt {
                error_message: &error_msg,
                document_failure_reason: Some("low_ocr_confidence"),
                record_reason: Some("low_ocr_confidence"),
                partial: Some(PartialOcr {
                    text: result.text.clone(),
                    confidence: result.confidence,
                    word_count: result.word_count as i32,
                }),
            };
            return self.fail(job, attempt).await;
        }
        if result.text.is_empty() {
            let error_msg = "No extractable text found in document (0 words)";
            warn!(
                "No searchable content extracted for '{}' | Job: {} | Document: {} | 0 words",
                job.filename, job.claim.job_id, job.claim.document_id
            );
            let attempt = FailedAttempt {
                error_message: error_msg,
                document_failure_reason: Some("no_extractable_text"),
                record_reason: Some(classify_ocr_error(error_msg).0),
                partial: None,
            };
            return self.fail(job, attempt).await;
        }
        self.store(job, &result).await
    }

    /// Writes a successful result under the claim.
    async fn store(&self, job: &LoadedJob, result: &OcrResult) -> Result<()> {
        let write = OcrWrite {
            document_id: job.claim.document_id,
            expected_filename: &job.filename,
            ocr_text: &result.text,
            confidence: result.confidence as f64,
            word_count: result.word_count as i32,
            processing_time_ms: job.started.elapsed().as_millis() as i64,
        };
        let outcome = self
            .transaction_manager
            .update_ocr_for_job(&job.claim, &write)
            .await;
        match write_failure(outcome) {
            WriteFailure::None => {}
            WriteFailure::Discarded(lost) => {
                info!(
                    "OCR result for job {} discarded: {:?}",
                    job.claim.job_id, lost
                );
                return Ok(());
            }
            WriteFailure::Failed(error_msg) => {
                warn!("{} for document {}", error_msg, job.claim.document_id);
                let attempt = FailedAttempt {
                    error_message: &error_msg,
                    document_failure_reason: None,
                    record_reason: Some(classify_ocr_error(&error_msg).0),
                    partial: None,
                };
                return self.fail(job, attempt).await;
            }
        }
        if job.settings.save_processed_images {
            self.save_for_review(job, result).await;
        }
        info!(
            "OCR completed for '{}' | Job: {} | Document: {} | {:.1}% confidence | {} words | {}ms | Preprocessing: {:?}",
            job.filename, job.claim.job_id, job.claim.document_id, result.confidence,
            result.word_count, job.started.elapsed().as_millis(), result.preprocessing_applied
        );
        Ok(())
    }

    /// Keeps the preprocessed image for review, when there is one.
    async fn save_for_review(&self, job: &LoadedJob, result: &OcrResult) {
        let Some(ref processed) = result.processed_image_path else {
            return;
        };
        let saved = self
            .save_processed_image_for_review(
                job.claim.document_id,
                job.user_id.unwrap_or_default(),
                &job.file_path,
                processed,
                &result.preprocessing_applied,
            )
            .await;
        if let Err(e) = saved {
            warn!(
                "Failed to save processed image for document {}: {}",
                job.claim.document_id, e
            );
        }
    }

    async fn extraction_failed(&self, job: &LoadedJob, e: &anyhow::Error) -> Result<()> {
        let error_msg = format!("OCR extraction failed: {}", e);
        let (failure_reason, should_suppress) = classify_ocr_error(&e.to_string());
        if should_suppress {
            debug!(
                "Expected PDF processing issue for '{}' ({}): {}",
                job.filename, failure_reason, e
            );
        } else {
            warn!(
                "OCR failed for '{}' | Job: {} | Document: {} | Reason: {} | Error: {}",
                job.filename, job.claim.job_id, job.claim.document_id, failure_reason, e
            );
        }
        let attempt = FailedAttempt {
            error_message: &error_msg,
            document_failure_reason: Some(failure_reason),
            record_reason: Some(failure_reason),
            partial: None,
        };
        self.fail(job, attempt).await
    }

    async fn fail(&self, job: &LoadedJob, attempt: FailedAttempt<'_>) -> Result<()> {
        if let Err(loss) = self
            .record_failed_attempt(&job.claim, job.attempts, attempt)
            .await?
        {
            log_discarded(job, loss);
        }
        Ok(())
    }
}

fn log_discarded(job: &LoadedJob, loss: ClaimLoss) {
    match loss {
        ClaimLoss::Cancelled => info!(
            "OCR job {} for document {} was cancelled; its failure is discarded",
            job.claim.job_id, job.claim.document_id
        ),
        ClaimLoss::Reclaimed => warn!(
            "OCR job {} for document {} was reclaimed by another worker; its failure is discarded",
            job.claim.job_id, job.claim.document_id
        ),
    }
}

/// What a result write that did not store the result means for the job.
#[derive(Debug, PartialEq, Eq)]
enum WriteFailure {
    /// It was stored.
    None,
    /// The claim was lost: nothing may be written, not even a failure.
    Discarded(OcrWriteOutcome),
    /// The write failed; the attempt fails with this message.
    Failed(String),
}

fn write_failure(outcome: Result<OcrWriteOutcome>) -> WriteFailure {
    match outcome {
        Ok(OcrWriteOutcome::Written) => WriteFailure::None,
        Ok(lost @ (OcrWriteOutcome::JobCancelled | OcrWriteOutcome::ClaimLost)) => {
            WriteFailure::Discarded(lost)
        }
        Ok(OcrWriteOutcome::Rejected) => WriteFailure::Failed(
            "OCR update failed validation (document may have been modified)".to_string(),
        ),
        Err(e) => WriteFailure::Failed(format!("Transaction-safe OCR update failed: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lost_claim_discards_and_only_real_failures_fail_the_attempt() {
        assert_eq!(
            write_failure(Ok(OcrWriteOutcome::Written)),
            WriteFailure::None
        );
        for lost in [OcrWriteOutcome::JobCancelled, OcrWriteOutcome::ClaimLost] {
            assert_eq!(write_failure(Ok(lost)), WriteFailure::Discarded(lost));
        }
        assert!(matches!(
            write_failure(Ok(OcrWriteOutcome::Rejected)),
            WriteFailure::Failed(_)
        ));
        assert!(matches!(
            write_failure(Err(anyhow::anyhow!("db down"))),
            WriteFailure::Failed(m) if m.contains("db down")
        ));
    }

    #[test]
    fn a_results_preprocessed_image_is_removed_when_dropped() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("processed.png");
        std::fs::write(&path, b"x").unwrap();
        let result = OcrResult {
            text: String::new(),
            confidence: 0.0,
            processing_time_ms: 0,
            word_count: 0,
            preprocessing_applied: Vec::new(),
            processed_image_path: Some(path.to_string_lossy().into_owned()),
        };
        drop(temp_image(&result));
        assert!(!path.exists());
        // Already gone is fine.
        drop(temp_image(&result));
    }
}
