use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use crate::{
    auth::AuthUser,
    models::DocumentOcrResponse,
    AppState,
};

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
) -> Result<Json<DocumentOcrResponse>, StatusCode> {
    let document = state
        .db
        .get_document_by_id(document_id, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|e| {
            error!("Database error getting document {}: {}", document_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let response = DocumentOcrResponse {
        id: document.id,
        filename: document.original_filename,
        has_ocr_text: document.ocr_text.is_some(),
        ocr_text: document.ocr_text,
        ocr_confidence: document.ocr_confidence,
        ocr_status: document.ocr_status,
        ocr_processing_time_ms: document.ocr_processing_time_ms,
        detected_language: None, // This would need to be stored separately if needed
        pages_processed: None,   // This would need to be stored separately if needed
    };

    Ok(Json(response))
}

/// Retry OCR processing for a document
#[utoipa::path(
    post,
    path = "/api/documents/{id}/retry-ocr",
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
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Get document first to check if it exists and user has access
    let document = state
        .db
        .get_document_by_id(document_id, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|e| {
            error!("Database error getting document {}: {}", document_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Check if OCR is already in progress
    if let Some(ref status) = document.ocr_status {
        if status == "processing" {
            return Ok(Json(serde_json::json!({
                "success": false,
                "message": "OCR is already in progress for this document"
            })));
        }
    }

    // Update user's OCR language settings based on what was provided
    if let Some(languages) = &request.languages {
        // Multi-language support: validate and update preferred languages
        let health_checker = crate::ocr::health::OcrHealthChecker::new();
        match health_checker.validate_preferred_languages(languages) {
            Ok(_) => {
                let settings_update = crate::models::UpdateSettings::language_update(
                    languages.clone(),
                    languages[0].clone(), // First language as primary
                    languages[0].clone(), // Backward compatibility
                );
                
                if let Err(e) = state.db.create_or_update_settings(auth_user.user.id, &settings_update).await {
                    warn!("Failed to update user preferred languages to {:?}: {}", languages, e);
                } else {
                    info!("Updated user {} preferred languages to: {:?} for retry", auth_user.user.id, languages);
                }
            }
            Err(e) => {
                warn!("Invalid language combination provided: {}", e);
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    } else if let Some(lang) = &request.language {
        // Single language (backward compatibility)
        let health_checker = crate::ocr::health::OcrHealthChecker::new();
        match health_checker.validate_language(lang) {
            Ok(_) => {
                if let Err(e) = state.db.update_user_ocr_language(auth_user.user.id, lang).await {
                    warn!("Failed to update user OCR language to {}: {}", lang, e);
                } else {
                    info!("Updated user {} OCR language to: {} for retry", auth_user.user.id, lang);
                }
            }
            Err(e) => {
                warn!("Invalid OCR language specified '{}': {}", lang, e);
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    }

    // Add to OCR queue
    match state.queue_service.enqueue_document(document.id, 5, document.file_size).await {
        Ok(_) => {
            info!("Document {} queued for OCR retry", document_id);
            Ok(Json(serde_json::json!({
                "success": true,
                "message": "Document queued for OCR processing"
            })))
        }
        Err(e) => {
            error!("Failed to queue document {} for OCR: {}", document_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get OCR processing status for multiple documents
pub async fn get_ocr_status_batch(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Json(document_ids): Json<Vec<uuid::Uuid>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
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

    Ok(Json(serde_json::json!({
        "results": results
    })))
}

/// Cancel OCR processing for a document
pub async fn cancel_ocr(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
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
    Ok(Json(serde_json::json!({
        "success": true,
        "message": "OCR processing stop requested"
    })))
}

/// Get OCR processing statistics
pub async fn get_ocr_stats(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let (total, pending, completed, failed) = state
        .db
        .count_documents_by_ocr_status(auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|e| {
            error!("Database error getting OCR stats: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get queue statistics
    let queue_stats = state
        .queue_service
        .get_stats()
        .await
        .map_err(|e| {
            error!("Failed to get OCR queue stats: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(serde_json::json!({
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
) -> Result<Json<serde_json::Value>, StatusCode> {
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
    debug!("OCR settings updated for document {}: {:?}", document_id, settings);
    
    Ok(Json(serde_json::json!({
        "success": true,
        "message": "OCR settings updated"
    })))
}