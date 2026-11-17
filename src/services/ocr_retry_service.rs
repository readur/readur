use anyhow::Result;
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

use serde::{Deserialize, Serialize};
use sqlx::Row;
use ts_rs::TS;
use utoipa::ToSchema;

use crate::{ocr::queue::EnqueueOutcome, AppState};

// Which failed documents a filtered OCR retry targets. Part of the
// `POST /api/documents/ocr/retry/bulk` request and of retry recommendations.
// (A plain comment, not a doc comment: ts-rs and utoipa would publish a doc comment.)
#[derive(Debug, Default, Deserialize, Serialize, Clone, ToSchema, TS)]
#[ts(export, optional_fields)]
pub struct OcrRetryFilter {
    /// Filter by MIME types
    pub mime_types: Option<Vec<String>>,
    /// Filter by file extensions
    pub file_extensions: Option<Vec<String>>,
    /// Filter by OCR failure reasons
    pub failure_reasons: Option<Vec<String>>,
    /// Filter by minimum file size (bytes)
    pub min_file_size: Option<i64>,
    /// Filter by maximum file size (bytes)
    pub max_file_size: Option<i64>,
    /// Filter by date range - documents created after this date
    pub created_after: Option<chrono::DateTime<chrono::Utc>>,
    /// Filter by date range - documents created before this date
    pub created_before: Option<chrono::DateTime<chrono::Utc>>,
    /// Filter by tags
    pub tags: Option<Vec<String>>,
    /// Maximum number of documents to retry
    pub limit: Option<i64>,
}

/// Queue priority for a retried document: an explicit override (clamped to
/// 1-20), else the retry priority for its size (smaller files first).
pub fn retry_priority(file_size: i64, override_priority: Option<i32>) -> i32 {
    if let Some(priority) = override_priority {
        return priority.clamp(1, 20);
    }

    crate::ocr::priority::retry_priority_for_file_size(file_size)
}

#[derive(Clone)]
pub struct OcrRetryService {
    state: Arc<AppState>,
}

impl OcrRetryService {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    /// Retry OCR for all failed documents for a user
    pub async fn retry_all_failed(
        &self,
        user_id: Uuid,
        priority_override: Option<i32>,
    ) -> Result<RetryResult> {
        info!(
            "Starting bulk retry for all failed OCR documents for user {}",
            user_id
        );

        let documents = self.get_all_failed_documents(user_id).await?;
        let retry_result = self
            .process_documents_for_retry(documents, user_id, "bulk_retry_all", priority_override)
            .await?;

        info!(
            "Bulk retry completed: {} out of {} documents queued",
            retry_result.queued_count, retry_result.matched_count
        );

        Ok(retry_result)
    }

    /// Retry OCR for documents matching specific criteria
    pub async fn retry_by_criteria(
        &self,
        user_id: Uuid,
        filter: OcrRetryFilter,
        priority_override: Option<i32>,
    ) -> Result<RetryResult> {
        info!("Starting filtered retry for user {} with criteria: mime_types={:?}, failure_reasons={:?}", 
            user_id, filter.mime_types, filter.failure_reasons);

        let documents = self.get_filtered_documents(user_id, filter).await?;
        let retry_result = self
            .process_documents_for_retry(
                documents,
                user_id,
                "bulk_retry_filtered",
                priority_override,
            )
            .await?;

        info!(
            "Filtered retry completed: {} out of {} documents queued",
            retry_result.queued_count, retry_result.matched_count
        );

        Ok(retry_result)
    }

    /// Retry OCR for specific document IDs
    pub async fn retry_specific_documents(
        &self,
        user_id: Uuid,
        document_ids: Vec<Uuid>,
        priority_override: Option<i32>,
    ) -> Result<RetryResult> {
        info!(
            "Starting specific document retry for user {} with {} documents",
            user_id,
            document_ids.len()
        );

        let documents = self.get_specific_documents(user_id, document_ids).await?;
        let retry_result = self
            .process_documents_for_retry(
                documents,
                user_id,
                "bulk_retry_specific",
                priority_override,
            )
            .await?;

        info!(
            "Specific document retry completed: {} out of {} documents queued",
            retry_result.queued_count, retry_result.matched_count
        );

        Ok(retry_result)
    }

    /// Get retry recommendations based on failure patterns
    pub async fn get_retry_recommendations(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<RetryRecommendation>> {
        let mut recommendations = Vec::new();

        // Get failure statistics
        let failure_stats = self.get_failure_statistics(user_id).await?;

        // Recommend retrying recent font encoding errors (often transient)
        if let Some(font_errors) = failure_stats
            .iter()
            .find(|s| s.reason.contains("font_encoding"))
        {
            if font_errors.count > 0 && font_errors.recent_failures > 0 {
                recommendations.push(RetryRecommendation {
                    reason: "pdf_font_encoding".to_string(),
                    title: "Font Encoding Errors".to_string(),
                    description: "These PDF files failed due to font encoding issues. Recent OCR improvements may resolve these.".to_string(),
                    estimated_success_rate: 0.7,
                    document_count: font_errors.count,
                    filter: OcrRetryFilter {
                        failure_reasons: Some(vec!["pdf_font_encoding".to_string()]),
                        ..Default::default()
                    },
                });
            }
        }

        // Recommend retrying corrupted files with smaller size (might be fixed)
        if let Some(corruption_errors) = failure_stats
            .iter()
            .find(|s| s.reason.contains("corruption"))
        {
            if corruption_errors.count > 0 && corruption_errors.avg_file_size_mb < 10.0 {
                recommendations.push(RetryRecommendation {
                    reason: "pdf_corruption".to_string(),
                    title: "Small Corrupted Files".to_string(),
                    description: "These smaller PDF files failed due to corruption. They may succeed with updated parsing logic.".to_string(),
                    estimated_success_rate: 0.5,
                    document_count: corruption_errors.count,
                    filter: OcrRetryFilter {
                        failure_reasons: Some(vec!["pdf_corruption".to_string()]),
                        max_file_size: Some(10 * 1024 * 1024), // 10MB
                        ..Default::default()
                    },
                });
            }
        }

        // Recommend retrying timeout errors with higher priority
        if let Some(timeout_errors) = failure_stats.iter().find(|s| s.reason.contains("timeout")) {
            if timeout_errors.count > 0 {
                recommendations.push(RetryRecommendation {
                    reason: "ocr_timeout".to_string(),
                    title: "Timeout Errors".to_string(),
                    description: "These files timed out during processing. Retrying with higher priority may help.".to_string(),
                    estimated_success_rate: 0.8,
                    document_count: timeout_errors.count,
                    filter: OcrRetryFilter {
                        failure_reasons: Some(vec!["ocr_timeout".to_string()]),
                        ..Default::default()
                    },
                });
            }
        }

        Ok(recommendations)
    }

    // Helper methods

    async fn get_all_failed_documents(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::db::ocr_retry::EligibleDocument>> {
        let user_filter = if self.is_admin(user_id).await? {
            None
        } else {
            Some(user_id)
        };

        crate::db::ocr_retry::get_eligible_documents_for_retry(
            self.state.db.get_pool(),
            user_filter,
            None,    // No MIME type filter
            None,    // No failure reason filter
            Some(5), // Max 5 retries
            None,    // No limit
        )
        .await
    }

    async fn get_filtered_documents(
        &self,
        user_id: Uuid,
        filter: OcrRetryFilter,
    ) -> Result<Vec<crate::db::ocr_retry::EligibleDocument>> {
        let user_filter = if self.is_admin(user_id).await? {
            None
        } else {
            Some(user_id)
        };

        crate::db::ocr_retry::get_eligible_documents_for_retry(
            self.state.db.get_pool(),
            user_filter,
            filter.mime_types.as_deref(),
            filter.failure_reasons.as_deref(),
            Some(5), // Max 5 retries
            filter.limit,
        )
        .await
    }

    async fn get_specific_documents(
        &self,
        user_id: Uuid,
        document_ids: Vec<Uuid>,
    ) -> Result<Vec<crate::db::ocr_retry::EligibleDocument>> {
        let user_filter = if self.is_admin(user_id).await? {
            None
        } else {
            Some(user_id)
        };

        let documents = sqlx::query_as::<_, crate::db::ocr_retry::EligibleDocument>(
            r#"
            SELECT id, filename, file_size, mime_type, ocr_failure_reason, ocr_retry_count, created_at, updated_at
            FROM documents
            WHERE id = ANY($1)
              AND ocr_status = 'failed'
              AND ($2::uuid IS NULL OR user_id = $2)
            "#
        )
        .bind(&document_ids)
        .bind(user_filter)
        .fetch_all(self.state.db.get_pool())
        .await?;

        Ok(documents)
    }

    async fn process_documents_for_retry(
        &self,
        documents: Vec<crate::db::ocr_retry::EligibleDocument>,
        user_id: Uuid,
        retry_reason: &str,
        priority_override: Option<i32>,
    ) -> Result<RetryResult> {
        let mut queued_count = 0;
        let matched_count = documents.len();

        for doc in documents {
            let priority = retry_priority(doc.file_size, priority_override);

            // Reset OCR status
            if let Err(e) = self.reset_document_ocr_status(doc.id).await {
                warn!("Failed to reset OCR status for document {}: {}", doc.id, e);
                continue;
            }

            // Queue for OCR
            match self
                .state
                .queue_service
                .enqueue_document(doc.id, priority, doc.file_size)
                .await
            {
                Ok(EnqueueOutcome::AlreadyQueued) => {
                    // The pending or running job does the retry.
                    info!(
                        "Document {} is already queued for OCR; not queueing it again",
                        doc.id
                    );
                }
                Ok(EnqueueOutcome::Queued(queue_id)) => {
                    // Record retry history
                    if let Err(e) = crate::db::ocr_retry::record_ocr_retry(
                        self.state.db.get_pool(),
                        doc.id,
                        user_id,
                        retry_reason,
                        priority,
                        Some(queue_id),
                    )
                    .await
                    {
                        warn!(
                            "Failed to record retry history for document {}: {}",
                            doc.id, e
                        );
                    }

                    queued_count += 1;
                    info!(
                        "Queued document {} for OCR retry with priority {}",
                        doc.id, priority
                    );
                }
                Err(e) => {
                    error!("Failed to queue document {} for OCR retry: {}", doc.id, e);
                }
            }
        }

        Ok(RetryResult {
            queued_count,
            matched_count,
        })
    }

    async fn reset_document_ocr_status(&self, document_id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE documents
            SET ocr_status = 'pending',
                ocr_text = NULL,
                ocr_error = NULL,
                ocr_failure_reason = NULL,
                ocr_confidence = NULL,
                ocr_word_count = NULL,
                ocr_processing_time_ms = NULL,
                ocr_completed_at = NULL,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(document_id)
        .execute(self.state.db.get_pool())
        .await?;

        Ok(())
    }

    async fn is_admin(&self, user_id: Uuid) -> Result<bool> {
        let role: Option<String> = sqlx::query_scalar("SELECT role FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(self.state.db.get_pool())
            .await?;

        Ok(role.as_deref() == Some("admin"))
    }

    async fn get_failure_statistics(&self, user_id: Uuid) -> Result<Vec<FailureStatistic>> {
        let user_filter = if self.is_admin(user_id).await? {
            None
        } else {
            Some(user_id)
        };

        let stats = sqlx::query(
            r#"
            SELECT 
                COALESCE(ocr_failure_reason, 'unknown') as reason,
                COUNT(*) as count,
                AVG(file_size) as avg_file_size,
                COUNT(*) FILTER (WHERE updated_at > NOW() - INTERVAL '7 days') as recent_failures
            FROM documents
            WHERE ocr_status = 'failed'
              AND ($1::uuid IS NULL OR user_id = $1)
            GROUP BY ocr_failure_reason
            ORDER BY count DESC
            "#,
        )
        .bind(user_filter)
        .fetch_all(self.state.db.get_pool())
        .await?;

        let statistics: Vec<FailureStatistic> = stats
            .into_iter()
            .map(|row| FailureStatistic {
                reason: row.get::<String, _>("reason"),
                count: row.get::<i64, _>("count"),
                avg_file_size_mb: {
                    // Handle NUMERIC type from database by trying different types
                    if let Ok(val) = row.try_get::<f64, _>("avg_file_size") {
                        val / 1_048_576.0
                    } else if let Ok(val) = row.try_get::<i64, _>("avg_file_size") {
                        val as f64 / 1_048_576.0
                    } else {
                        0.0
                    }
                },
                recent_failures: row.get::<i64, _>("recent_failures"),
            })
            .collect();

        Ok(statistics)
    }
}

#[derive(Debug)]
pub struct RetryResult {
    pub queued_count: usize,
    pub matched_count: usize,
}

#[derive(Debug)]
pub struct RetryRecommendation {
    pub reason: String,
    pub title: String,
    pub description: String,
    pub estimated_success_rate: f64,
    pub document_count: i64,
    pub filter: OcrRetryFilter,
}

#[derive(Debug)]
struct FailureStatistic {
    reason: String,
    count: i64,
    avg_file_size_mb: f64,
    recent_failures: i64,
}
