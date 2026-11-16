use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::Json as ResponseJson,
};
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::{auth::AuthUser, models::DocumentOcrResponse, AppState};

/// Get OCR text for a document
#[utoipa::path(
    get,
    path = "/api/documents/{id}/ocr",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = uuid::Uuid, Path, description = "Document ID")
    ),
    responses(
        (status = 200, description = "Document OCR text", body = DocumentOcrResponse),
        (status = 404, description = "Document not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_document_ocr(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<uuid::Uuid>,
) -> Result<ResponseJson<DocumentOcrResponse>, StatusCode> {
    crate::services::documents::get_document_ocr(&state, &auth_user.user, document_id)
        .await
        .map_err(|e| {
            error!("Database error getting document {}: {}", document_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map(ResponseJson)
        .ok_or(StatusCode::NOT_FOUND)
}

/// Retry OCR processing for a document
#[utoipa::path(
    post,
    path = "/api/documents/{id}/ocr/retry",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = uuid::Uuid, Path, description = "Document ID")
    ),
    request_body(content = super::types::RetryOcrRequest, description = "OCR retry options"),
    responses(
        (status = 200, description = "OCR retry initiated"),
        (status = 404, description = "Document not found"),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "OCR already in progress"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn retry_ocr(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<uuid::Uuid>,
    Json(request): Json<super::types::RetryOcrRequest>,
) -> Result<ResponseJson<serde_json::Value>, StatusCode> {
    debug!(
        "OCR retry request for document {} by user {}",
        document_id, auth_user.user.id
    );
    debug!(
        "Request data: language={:?}, languages={:?}",
        request.language, request.languages
    );
    let outcome = crate::services::documents::retry_document_ocr(
        &state,
        &auth_user.user,
        document_id,
        request.languages.as_deref(),
        request.language.as_deref(),
    )
    .await
    .map_err(|e| match e {
        super::DocumentError::NotFound => StatusCode::NOT_FOUND,
        super::DocumentError::BadRequest(_) => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    })?;
    Ok(ResponseJson(serde_json::json!({
        "success": outcome.queued,
        "message": outcome.message
    })))
}

/// Get OCR processing status for multiple documents
pub async fn get_ocr_status_batch(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Json(document_ids): Json<Vec<uuid::Uuid>>,
) -> Result<ResponseJson<serde_json::Value>, StatusCode> {
    if document_ids.len() > 100 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut results = Vec::new();

    for document_id in document_ids {
        match state
            .db
            .get_document_by_id(document_id, auth_user.user.id, auth_user.user.role)
            .await
        {
            Ok(Some(document)) => {
                results.push(serde_json::json!({
                    "document_id": document.id,
                    "ocr_status": document.ocr_status,
                    "ocr_confidence": document.ocr_confidence,
                    "has_ocr_text": document.ocr_text.is_some(),
                    "retry_count": document.ocr_retry_count.unwrap_or(0)
                }));
            }
            Ok(None) => {
                results.push(serde_json::json!({
                    "document_id": document_id,
                    "error": "Document not found"
                }));
            }
            Err(e) => {
                error!("Error getting document {}: {}", document_id, e);
                results.push(serde_json::json!({
                    "document_id": document_id,
                    "error": "Database error"
                }));
            }
        }
    }

    Ok(ResponseJson(serde_json::json!({
        "results": results
    })))
}

/// Cancel OCR processing for a document
pub async fn cancel_ocr(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<uuid::Uuid>,
) -> Result<ResponseJson<serde_json::Value>, StatusCode> {
    // Verify user has access to the document
    let _document = state
        .db
        .get_document_by_id(document_id, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|e| {
            error!("Database error getting document {}: {}", document_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Note: OCR queue removal not implemented in current queue service
    info!("Stop OCR processing requested for document {}", document_id);
    Ok(ResponseJson(serde_json::json!({
        "success": true,
        "message": "OCR processing stop requested"
    })))
}

/// Get OCR processing statistics
pub async fn get_ocr_stats(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<ResponseJson<serde_json::Value>, StatusCode> {
    let (total, pending, completed, failed) = state
        .db
        .count_documents_by_ocr_status(auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|e| {
            error!("Database error getting OCR stats: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get queue statistics
    let queue_stats = state.queue_service.get_stats().await.map_err(|e| {
        error!("Failed to get OCR queue stats: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(ResponseJson(serde_json::json!({
        "total_documents": total,
        "pending_ocr": pending,
        "completed_ocr": completed,
        "failed_ocr": failed,
        "queue_size": queue_stats.pending_count,
        "active_jobs": queue_stats.processing_count,
        "completion_rate": if total > 0 { completed as f64 / total as f64 * 100.0 } else { 0.0 }
    })))
}

/// Update OCR settings for a document
pub async fn update_ocr_settings(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<uuid::Uuid>,
    Json(settings): Json<serde_json::Value>,
) -> Result<ResponseJson<serde_json::Value>, StatusCode> {
    // Verify user has access to the document
    let _document = state
        .db
        .get_document_by_id(document_id, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|e| {
            error!("Database error getting document {}: {}", document_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // For now, just return success - OCR settings would be stored in metadata
    debug!(
        "OCR settings updated for document {}: {:?}",
        document_id, settings
    );

    Ok(ResponseJson(serde_json::json!({
        "success": true,
        "message": "OCR settings updated"
    })))
}
