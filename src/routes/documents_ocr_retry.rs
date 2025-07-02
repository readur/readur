use std::sync::Arc;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;
use tracing::{info, error, warn};
use utoipa::ToSchema;

use crate::{
    auth::AuthUser,
    AppState,
    models::UserRole,
};

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct BulkOcrRetryRequest {
    /// Selection mode: "all", "specific", "filter"
    pub mode: SelectionMode,
    /// Specific document IDs (when mode = "specific")
    pub document_ids: Option<Vec<Uuid>>,
    /// Filter criteria (when mode = "filter")
    pub filter: Option<OcrRetryFilter>,
    /// Priority override (1-20, higher = more urgent)
    pub priority_override: Option<i32>,
    /// Preview mode - just return what would be processed
    pub preview_only: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SelectionMode {
    All,      // All failed OCR documents
    Specific, // Specific document IDs
    Filter,   // Filter by criteria
}

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
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

#[derive(Debug, Serialize, ToSchema)]
pub struct BulkOcrRetryResponse {
    pub success: bool,
    pub message: String,
    pub queued_count: usize,
    pub matched_count: usize,
    pub documents: Vec<OcrRetryDocumentInfo>,
    pub estimated_total_time_minutes: f64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OcrRetryDocumentInfo {
    pub id: Uuid,
    pub filename: String,
    pub file_size: i64,
    pub mime_type: String,
    pub ocr_failure_reason: Option<String>,
    pub priority: i32,
    pub queue_id: Option<Uuid>,
}

/// Bulk retry OCR for multiple documents based on selection criteria
#[utoipa::path(
    post,
    path = "/api/documents/ocr/bulk-retry",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    request_body = BulkOcrRetryRequest,
    responses(
        (status = 200, description = "Bulk OCR retry result", body = BulkOcrRetryResponse),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn bulk_retry_ocr(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Json(request): Json<BulkOcrRetryRequest>,
) -> Result<Json<BulkOcrRetryResponse>, StatusCode> {
    crate::debug_log!("BULK_OCR_RETRY",
        "user_id" => auth_user.user.id,
        "mode" => format!("{:?}", request.mode),
        "preview_only" => request.preview_only.unwrap_or(false),
        "priority_override" => request.priority_override.unwrap_or(-1),
        "message" => "Starting bulk OCR retry request"
    );
    
    info!("Bulk OCR retry requested by user {} with mode: {:?}", auth_user.user.id, request.mode);
    
    let preview_only = request.preview_only.unwrap_or(false);
    
    // Build query based on selection mode
    crate::debug_log!("BULK_OCR_RETRY", "Building document query based on selection mode");
    
    let documents = match request.mode {
        SelectionMode::All => {
            crate::debug_log!("BULK_OCR_RETRY", "Fetching all failed OCR documents");
            get_all_failed_ocr_documents(&state, &auth_user).await?
        }
        SelectionMode::Specific => {
            if let Some(ids) = &request.document_ids {
                crate::debug_log!("BULK_OCR_RETRY",
                    "document_count" => ids.len(),
                    "message" => "Fetching specific documents"
                );
                get_specific_documents(&state, &auth_user, ids.clone()).await?
            } else {
                crate::debug_error!("BULK_OCR_RETRY", "Specific mode requested but no document IDs provided");
                return Err(StatusCode::BAD_REQUEST);
            }
        }
        SelectionMode::Filter => {
            if let Some(filter) = &request.filter {
                crate::debug_log!("BULK_OCR_RETRY",
                    "filter_mime_types" => filter.mime_types.as_ref().map(|v| v.len()).unwrap_or(0),
                    "filter_failure_reasons" => filter.failure_reasons.as_ref().map(|v| v.len()).unwrap_or(0),
                    "message" => "Fetching filtered documents"
                );
                get_filtered_documents(&state, &auth_user, filter.clone()).await?
            } else {
                crate::debug_error!("BULK_OCR_RETRY", "Filter mode requested but no filter provided");
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    };
    
    let matched_count = documents.len();
    crate::debug_log!("BULK_OCR_RETRY",
        "matched_count" => matched_count,
        "message" => "Document query completed"
    );
    let mut retry_documents = Vec::new();
    let mut queued_count = 0;
    let mut total_estimated_time = 0.0;
    
    for (index, doc) in documents.iter().enumerate() {
        let priority = calculate_priority(doc.file_size, request.priority_override);
        
        crate::debug_log!("BULK_OCR_RETRY",
            "index" => index + 1,
            "total" => matched_count,
            "document_id" => doc.id,
            "filename" => &doc.filename,
            "file_size" => doc.file_size,
            "priority" => priority,
            "failure_reason" => doc.ocr_failure_reason.as_deref().unwrap_or("none"),
            "message" => "Processing document"
        );
        
        let mut doc_info = OcrRetryDocumentInfo {
            id: doc.id,
            filename: doc.filename.clone(),
            file_size: doc.file_size,
            mime_type: doc.mime_type.clone(),
            ocr_failure_reason: doc.ocr_failure_reason.clone(),
            priority,
            queue_id: None,
        };
        
        if !preview_only {
            // Reset OCR fields
            crate::debug_log!("BULK_OCR_RETRY",
                "document_id" => doc.id,
                "message" => "Resetting OCR status for document"
            );
            
            if let Err(e) = reset_document_ocr_status(&state, doc.id).await {
                crate::debug_error!("BULK_OCR_RETRY", format!("Failed to reset OCR status for document {}: {}", doc.id, e));
                warn!("Failed to reset OCR status for document {}: {}", doc.id, e);
                continue;
            }
            
            // Queue for OCR
            crate::debug_log!("BULK_OCR_RETRY",
                "document_id" => doc.id,
                "priority" => priority,
                "file_size" => doc.file_size,
                "message" => "Enqueueing document for OCR"
            );
            
            match state.queue_service.enqueue_document(doc.id, priority, doc.file_size).await {
                Ok(queue_id) => {
                    doc_info.queue_id = Some(queue_id);
                    queued_count += 1;
                    
                    crate::debug_log!("BULK_OCR_RETRY",
                        "document_id" => doc.id,
                        "queue_id" => queue_id,
                        "priority" => priority,
                        "queued_count" => queued_count,
                        "message" => "Successfully enqueued document"
                    );
                    
                    // Record retry history
                    let retry_reason = match &request.mode {
                        SelectionMode::All => "bulk_retry_all",
                        SelectionMode::Specific => "bulk_retry_specific",
                        SelectionMode::Filter => "bulk_retry_filtered",
                    };
                    
                    crate::debug_log!("BULK_OCR_RETRY",
                        "document_id" => doc.id,
                        "retry_reason" => retry_reason,
                        "queue_id" => queue_id,
                        "message" => "Recording retry history"
                    );
                    
                    if let Err(e) = crate::db::ocr_retry::record_ocr_retry(
                        state.db.get_pool(),
                        doc.id,
                        auth_user.user.id,
                        retry_reason,
                        priority,
                        Some(queue_id),
                    ).await {
                        crate::debug_error!("BULK_OCR_RETRY", format!("Failed to record retry history for document {}: {}", doc.id, e));
                        warn!("Failed to record retry history for document {}: {}", doc.id, e);
                    } else {
                        crate::debug_log!("BULK_OCR_RETRY", 
                            "document_id" => doc.id,
                            "queue_id" => queue_id,
                            "message" => "Successfully recorded retry history"
                        );
                    }
                    
                    info!("Queued document {} for OCR retry with priority {}", doc.id, priority);
                }
                Err(e) => {
                    crate::debug_error!("BULK_OCR_RETRY", format!("Failed to enqueue document {}: {}", doc.id, e));
                    error!("Failed to queue document {} for OCR retry: {}", doc.id, e);
                }
            }
        }
        
        // Estimate processing time (2 seconds per MB as rough estimate)
        total_estimated_time += (doc.file_size as f64 / 1_048_576.0) * 2.0;
        retry_documents.push(doc_info);
    }
    
    crate::debug_log!("BULK_OCR_RETRY", 
        "matched_count" => matched_count,
        "queued_count" => queued_count,
        "preview_only" => preview_only,
        "estimated_time_minutes" => (total_estimated_time / 60.0) as i32,
        "user_id" => auth_user.user.id,
        "message" => "Bulk retry operation completed"
    );
    
    let response = BulkOcrRetryResponse {
        success: true,
        message: if preview_only {
            format!("Preview: {} documents would be queued for OCR retry", matched_count)
        } else {
            format!("Successfully queued {} out of {} documents for OCR retry", queued_count, matched_count)
        },
        queued_count,
        matched_count,
        documents: retry_documents,
        estimated_total_time_minutes: total_estimated_time / 60.0,
    };
    
    Ok(Json(response))
}

/// Get retry history for a specific document
#[utoipa::path(
    get,
    path = "/api/documents/{id}/ocr/retry-history",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = Uuid, Path, description = "Document ID")
    ),
    responses(
        (status = 200, description = "OCR retry history", body = String),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Document not found")
    )
)]
pub async fn get_document_retry_history(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Check if document exists and belongs to user
    let doc_exists = sqlx::query(
        r#"
        SELECT 1 FROM documents 
        WHERE id = $1 
          AND ($2::uuid IS NULL OR user_id = $2)
        "#
    )
    .bind(document_id)
    .bind(if auth_user.user.role == UserRole::Admin { None } else { Some(auth_user.user.id) })
    .fetch_optional(state.db.get_pool())
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if doc_exists.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }
    
    let history = crate::db::ocr_retry::get_document_retry_history(state.db.get_pool(), document_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let history_items: Vec<serde_json::Value> = history.into_iter()
        .map(|h| {
            serde_json::json!({
                "id": h.id,
                "retry_reason": h.retry_reason,
                "previous_status": h.previous_status,
                "previous_failure_reason": h.previous_failure_reason,
                "previous_error": h.previous_error,
                "priority": h.priority,
                "queue_id": h.queue_id,
                "created_at": h.created_at,
            })
        })
        .collect();
    
    Ok(Json(serde_json::json!({
        "document_id": document_id,
        "retry_history": history_items,
        "total_retries": history_items.len(),
    })))
}

/// Get OCR retry statistics
#[utoipa::path(
    get,
    path = "/api/documents/ocr/retry-stats",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "OCR retry statistics", body = String),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_ocr_retry_stats(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user_filter = if auth_user.user.role == UserRole::Admin {
        None
    } else {
        Some(auth_user.user.id)
    };
    
    // Get statistics by failure reason
    let failure_stats = sqlx::query(
        r#"
        SELECT 
            ocr_failure_reason,
            COUNT(*) as count,
            AVG(file_size) as avg_file_size,
            MIN(created_at) as first_occurrence,
            MAX(updated_at) as last_occurrence
        FROM documents
        WHERE ocr_status = 'failed'
          AND ($1::uuid IS NULL OR user_id = $1)
        GROUP BY ocr_failure_reason
        ORDER BY count DESC
        "#
    )
    .bind(user_filter)
    .fetch_all(state.db.get_pool())
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Get statistics by file type
    let type_stats = sqlx::query(
        r#"
        SELECT 
            mime_type,
            COUNT(*) as count,
            AVG(file_size) as avg_file_size
        FROM documents
        WHERE ocr_status = 'failed'
          AND ($1::uuid IS NULL OR user_id = $1)
        GROUP BY mime_type
        ORDER BY count DESC
        "#
    )
    .bind(user_filter)
    .fetch_all(state.db.get_pool())
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let failure_reasons: Vec<serde_json::Value> = failure_stats.into_iter()
        .map(|row| {
            // Handle NUMERIC type from database by trying different types
            let avg_file_size_mb = if let Ok(val) = row.try_get::<f64, _>("avg_file_size") {
                val / 1_048_576.0
            } else if let Ok(val) = row.try_get::<i64, _>("avg_file_size") {
                val as f64 / 1_048_576.0
            } else {
                0.0
            };
            
            serde_json::json!({
                "reason": row.get::<Option<String>, _>("ocr_failure_reason").unwrap_or_else(|| "unknown".to_string()),
                "count": row.get::<i64, _>("count"),
                "avg_file_size_mb": avg_file_size_mb,
                "first_occurrence": row.get::<chrono::DateTime<chrono::Utc>, _>("first_occurrence"),
                "last_occurrence": row.get::<chrono::DateTime<chrono::Utc>, _>("last_occurrence"),
            })
        })
        .collect();
    
    let file_types: Vec<serde_json::Value> = type_stats.into_iter()
        .map(|row| {
            // Handle NUMERIC type from database by trying different types
            let avg_file_size_mb = if let Ok(val) = row.try_get::<f64, _>("avg_file_size") {
                val / 1_048_576.0
            } else if let Ok(val) = row.try_get::<i64, _>("avg_file_size") {
                val as f64 / 1_048_576.0
            } else {
                0.0
            };
            
            serde_json::json!({
                "mime_type": row.get::<String, _>("mime_type"),
                "count": row.get::<i64, _>("count"),
                "avg_file_size_mb": avg_file_size_mb,
            })
        })
        .collect();
    
    Ok(Json(serde_json::json!({
        "failure_reasons": failure_reasons,
        "file_types": file_types,
        "total_failed": failure_reasons.iter().map(|r| r["count"].as_i64().unwrap_or(0)).sum::<i64>(),
    })))
}

/// Get intelligent retry recommendations based on failure patterns
#[utoipa::path(
    get,
    path = "/api/documents/ocr/retry-recommendations",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "OCR retry recommendations", body = String),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_retry_recommendations(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let retry_service = crate::services::ocr_retry_service::OcrRetryService::new(state);
    
    let recommendations = retry_service.get_retry_recommendations(auth_user.user.id)
        .await
        .map_err(|e| {
            error!("Failed to get retry recommendations: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    let recommendations_json: Vec<serde_json::Value> = recommendations.into_iter()
        .map(|rec| {
            serde_json::json!({
                "reason": rec.reason,
                "title": rec.title,
                "description": rec.description,
                "estimated_success_rate": rec.estimated_success_rate,
                "document_count": rec.document_count,
                "filter": rec.filter,
            })
        })
        .collect();
    
    Ok(Json(serde_json::json!({
        "recommendations": recommendations_json,
        "total_recommendations": recommendations_json.len(),
    })))
}

// Helper functions

async fn get_all_failed_ocr_documents(
    state: &Arc<AppState>, 
    auth_user: &AuthUser
) -> Result<Vec<DocumentInfo>, StatusCode> {
    let user_filter = if auth_user.user.role == UserRole::Admin {
        None
    } else {
        Some(auth_user.user.id)
    };
    
    let documents = sqlx::query_as::<_, DocumentInfo>(
        r#"
        SELECT id, filename, file_size, mime_type, ocr_failure_reason
        FROM documents
        WHERE ocr_status = 'failed'
          AND ($1::uuid IS NULL OR user_id = $1)
        ORDER BY created_at DESC
        "#
    )
    .bind(user_filter)
    .fetch_all(state.db.get_pool())
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(documents)
}

async fn get_specific_documents(
    state: &Arc<AppState>,
    auth_user: &AuthUser,
    document_ids: Vec<Uuid>
) -> Result<Vec<DocumentInfo>, StatusCode> {
    let user_filter = if auth_user.user.role == UserRole::Admin {
        None
    } else {
        Some(auth_user.user.id)
    };
    
    let documents = sqlx::query_as::<_, DocumentInfo>(
        r#"
        SELECT id, filename, file_size, mime_type, ocr_failure_reason
        FROM documents
        WHERE id = ANY($1)
          AND ocr_status = 'failed'
          AND ($2::uuid IS NULL OR user_id = $2)
        "#
    )
    .bind(&document_ids)
    .bind(user_filter)
    .fetch_all(state.db.get_pool())
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(documents)
}

async fn get_filtered_documents(
    state: &Arc<AppState>,
    auth_user: &AuthUser,
    filter: OcrRetryFilter
) -> Result<Vec<DocumentInfo>, StatusCode> {
    let mut query = sqlx::QueryBuilder::new(
        "SELECT id, filename, file_size, mime_type, ocr_failure_reason FROM documents WHERE ocr_status = 'failed'"
    );
    
    // User filter
    if auth_user.user.role != UserRole::Admin {
        query.push(" AND user_id = ");
        query.push_bind(auth_user.user.id);
    }
    
    // MIME type filter
    if let Some(mime_types) = &filter.mime_types {
        if !mime_types.is_empty() {
            query.push(" AND mime_type = ANY(");
            query.push_bind(mime_types);
            query.push(")");
        }
    }
    
    // File extension filter
    if let Some(extensions) = &filter.file_extensions {
        if !extensions.is_empty() {
            query.push(" AND (");
            for (i, ext) in extensions.iter().enumerate() {
                if i > 0 {
                    query.push(" OR ");
                }
                query.push("filename ILIKE ");
                query.push_bind(format!("%.{}", ext));
            }
            query.push(")");
        }
    }
    
    // Failure reason filter
    if let Some(reasons) = &filter.failure_reasons {
        if !reasons.is_empty() {
            query.push(" AND ocr_failure_reason = ANY(");
            query.push_bind(reasons);
            query.push(")");
        }
    }
    
    // File size filters
    if let Some(min_size) = filter.min_file_size {
        query.push(" AND file_size >= ");
        query.push_bind(min_size);
    }
    
    if let Some(max_size) = filter.max_file_size {
        query.push(" AND file_size <= ");
        query.push_bind(max_size);
    }
    
    // Date filters
    if let Some(created_after) = filter.created_after {
        query.push(" AND created_at >= ");
        query.push_bind(created_after);
    }
    
    if let Some(created_before) = filter.created_before {
        query.push(" AND created_at <= ");
        query.push_bind(created_before);
    }
    
    // Tag filter
    if let Some(tags) = &filter.tags {
        if !tags.is_empty() {
            query.push(" AND tags && ");
            query.push_bind(tags);
        }
    }
    
    // Order and limit
    query.push(" ORDER BY created_at DESC");
    
    if let Some(limit) = filter.limit {
        query.push(" LIMIT ");
        query.push_bind(limit);
    }
    
    let documents = query.build_query_as::<DocumentInfo>()
        .fetch_all(state.db.get_pool())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(documents)
}

async fn reset_document_ocr_status(state: &Arc<AppState>, document_id: Uuid) -> Result<(), anyhow::Error> {
    sqlx::query(
        r#"
        UPDATE documents
        SET ocr_status = 'pending',
            ocr_text = NULL,
            ocr_error = NULL,
            ocr_failure_reason = NULL,
            ocr_retry_count = NULL,
            ocr_confidence = NULL,
            ocr_word_count = NULL,
            ocr_processing_time_ms = NULL,
            ocr_completed_at = NULL,
            updated_at = NOW()
        WHERE id = $1
        "#
    )
    .bind(document_id)
    .execute(state.db.get_pool())
    .await?;
    
    Ok(())
}

fn calculate_priority(file_size: i64, override_priority: Option<i32>) -> i32 {
    if let Some(priority) = override_priority {
        return priority.clamp(1, 20);
    }
    
    match file_size {
        0..=1048576 => 15,      // <= 1MB: highest priority
        ..=5242880 => 12,       // 1-5MB: high priority
        ..=10485760 => 10,      // 5-10MB: medium priority  
        ..=52428800 => 8,       // 10-50MB: low priority
        _ => 6,                 // > 50MB: lowest priority
    }
}

#[derive(Debug, sqlx::FromRow)]
struct DocumentInfo {
    id: Uuid,
    filename: String,
    file_size: i64,
    mime_type: String,
    ocr_failure_reason: Option<String>,
}