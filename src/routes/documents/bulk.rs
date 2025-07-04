use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use crate::{
    auth::AuthUser,
    services::file_service::FileService,
    AppState,
};
use super::types::{BulkDeleteRequest, DeleteLowConfidenceRequest, BulkDeleteResponse};

/// Bulk delete multiple documents
#[utoipa::path(
    delete,
    path = "/api/documents",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    request_body = BulkDeleteRequest,
    responses(
        (status = 200, description = "Bulk delete results", body = BulkDeleteResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn bulk_delete_documents(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Json(request): Json<BulkDeleteRequest>,
) -> Result<Json<BulkDeleteResponse>, StatusCode> {
    if request.document_ids.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    if request.document_ids.len() > 1000 {
        return Err(StatusCode::BAD_REQUEST);
    }

    info!("Bulk deleting {} documents", request.document_ids.len());

    // Get documents first to check access and collect file paths
    let mut documents_to_delete = Vec::new();
    let mut accessible_ids = Vec::new();

    for document_id in &request.document_ids {
        match state
            .db
            .get_document_by_id(*document_id, auth_user.user.id, auth_user.user.role)
            .await
        {
            Ok(Some(document)) => {
                documents_to_delete.push(document);
                accessible_ids.push(*document_id);
            }
            Ok(None) => {
                debug!("Document {} not found or access denied", document_id);
            }
            Err(e) => {
                error!("Error checking document {}: {}", document_id, e);
            }
        }
    }

    // Perform bulk delete from database
    let (deleted_ids, failed_ids) = state
        .db
        .bulk_delete_documents(&accessible_ids, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|e| {
            error!("Database error during bulk delete: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Delete associated files
    let file_service = FileService::new(state.config.upload_path.clone());
    let mut files_deleted = 0;
    let mut files_failed = 0;

    for document in documents_to_delete {
        if deleted_ids.contains(&document.id) {
            match file_service.delete_document_files(&document).await {
                Ok(_) => files_deleted += 1,
                Err(e) => {
                    warn!("Failed to delete files for document {}: {}", document.id, e);
                    files_failed += 1;
                }
            }
        }
    }

    let response = BulkDeleteResponse {
        deleted_count: deleted_ids.len() as i64,
        failed_count: failed_ids.len() as i64,
        deleted_documents: deleted_ids,
        failed_documents: failed_ids,
        total_files_deleted: files_deleted,
        total_files_failed: files_failed,
    };

    info!("Bulk delete completed: {} deleted, {} failed", 
        response.deleted_count, response.failed_count);

    Ok(Json(response))
}

/// Delete documents with low OCR confidence
#[utoipa::path(
    post,
    path = "/api/documents/delete-low-confidence",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    request_body = DeleteLowConfidenceRequest,
    responses(
        (status = 200, description = "Low confidence delete results", body = BulkDeleteResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_low_confidence_documents(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Json(request): Json<DeleteLowConfidenceRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if request.max_confidence < 0.0 || request.max_confidence > 100.0 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let preview_only = request.preview_only.unwrap_or(false);

    info!("Finding documents with OCR confidence <= {}", request.max_confidence);

    // Find documents with low confidence
    let low_confidence_docs = state
        .db
        .find_documents_by_confidence_threshold(
            auth_user.user.id,
            auth_user.user.role,
            request.max_confidence,
            1000, // Limit to prevent excessive operations
            0,
        )
        .await
        .map_err(|e| {
            error!("Database error finding low confidence documents: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if preview_only {
        let preview_docs: Vec<_> = low_confidence_docs
            .iter()
            .take(10) // Show max 10 in preview
            .map(|doc| serde_json::json!({
                "id": doc.id,
                "filename": doc.original_filename,
                "ocr_confidence": doc.ocr_confidence,
                "created_at": doc.created_at
            }))
            .collect();

        return Ok(Json(serde_json::json!({
            "preview": true,
            "total_found": low_confidence_docs.len(),
            "documents": preview_docs,
            "message": format!("Found {} documents with OCR confidence <= {}", 
                low_confidence_docs.len(), request.max_confidence)
        })));
    }

    // Perform actual deletion
    let document_ids: Vec<uuid::Uuid> = low_confidence_docs.iter().map(|d| d.id).collect();

    if document_ids.is_empty() {
        return Ok(Json(serde_json::json!({
            "deleted_count": 0,
            "failed_count": 0,
            "message": "No documents found with the specified confidence threshold"
        })));
    }

    let (deleted_ids, failed_ids) = state
        .db
        .bulk_delete_documents(&document_ids, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|e| {
            error!("Database error during low confidence bulk delete: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Delete associated files
    let file_service = FileService::new(state.config.upload_path.clone());
    let mut files_deleted = 0;
    let mut files_failed = 0;

    for document in low_confidence_docs {
        if deleted_ids.contains(&document.id) {
            match file_service.delete_document_files(&document).await {
                Ok(_) => files_deleted += 1,
                Err(e) => {
                    warn!("Failed to delete files for document {}: {}", document.id, e);
                    files_failed += 1;
                }
            }
        }
    }

    info!("Low confidence delete completed: {} deleted, {} failed", 
        deleted_ids.len(), failed_ids.len());

    Ok(Json(serde_json::json!({
        "deleted_count": deleted_ids.len(),
        "failed_count": failed_ids.len(),
        "files_deleted": files_deleted,
        "files_failed": files_failed,
        "deleted_documents": deleted_ids,
        "failed_documents": failed_ids,
        "message": format!("Deleted {} documents with OCR confidence <= {}", 
            deleted_ids.len(), request.max_confidence)
    })))
}

/// Delete documents with failed OCR
#[utoipa::path(
    post,
    path = "/api/documents/delete-failed-ocr",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Failed OCR delete results"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_failed_ocr_documents(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Finding documents with failed OCR");

    // Find documents with failed OCR
    let failed_ocr_docs = state
        .db
        .find_failed_ocr_documents(
            auth_user.user.id,
            auth_user.user.role,
            1000, // Limit to prevent excessive operations
            0,
        )
        .await
        .map_err(|e| {
            error!("Database error finding failed OCR documents: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if failed_ocr_docs.is_empty() {
        return Ok(Json(serde_json::json!({
            "deleted_count": 0,
            "message": "No documents found with failed OCR status"
        })));
    }

    // Perform deletion
    let document_ids: Vec<uuid::Uuid> = failed_ocr_docs.iter().map(|d| d.id).collect();

    let (deleted_ids, failed_ids) = state
        .db
        .bulk_delete_documents(&document_ids, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|e| {
            error!("Database error during failed OCR bulk delete: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Delete associated files
    let file_service = FileService::new(state.config.upload_path.clone());
    let mut files_deleted = 0;
    let mut files_failed = 0;

    for document in failed_ocr_docs {
        if deleted_ids.contains(&document.id) {
            match file_service.delete_document_files(&document).await {
                Ok(_) => files_deleted += 1,
                Err(e) => {
                    warn!("Failed to delete files for document {}: {}", document.id, e);
                    files_failed += 1;
                }
            }
        }
    }

    info!("Failed OCR delete completed: {} deleted, {} failed", 
        deleted_ids.len(), failed_ids.len());

    Ok(Json(serde_json::json!({
        "deleted_count": deleted_ids.len(),
        "failed_count": failed_ids.len(),
        "files_deleted": files_deleted,
        "files_failed": files_failed,
        "deleted_documents": deleted_ids,
        "failed_documents": failed_ids,
        "message": format!("Deleted {} documents with failed OCR", deleted_ids.len())
    })))
}

/// Get documents marked for deletion (cleanup preview)
pub async fn get_cleanup_preview(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let max_confidence = params
        .get("max_confidence")
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(30.0);

    let include_failed = params
        .get("include_failed")
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(true);

    let mut cleanup_candidates = Vec::new();
    let mut total_size = 0i64;

    // Get low confidence documents
    let low_confidence_docs = state
        .db
        .find_documents_by_confidence_threshold(
            auth_user.user.id,
            auth_user.user.role,
            max_confidence,
            100,
            0,
        )
        .await
        .map_err(|e| {
            error!("Database error finding low confidence documents: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    for doc in low_confidence_docs {
        total_size += doc.file_size;
        cleanup_candidates.push(serde_json::json!({
            "id": doc.id,
            "filename": doc.original_filename,
            "file_size": doc.file_size,
            "ocr_confidence": doc.ocr_confidence,
            "reason": "low_confidence",
            "created_at": doc.created_at
        }));
    }

    // Get failed OCR documents if requested
    if include_failed {
        let failed_docs = state
            .db
            .find_failed_ocr_documents(
                auth_user.user.id,
                auth_user.user.role,
                100,
                0,
            )
            .await
            .map_err(|e| {
                error!("Database error finding failed OCR documents: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        for doc in failed_docs {
            total_size += doc.file_size;
            cleanup_candidates.push(serde_json::json!({
                "id": doc.id,
                "filename": doc.original_filename,
                "file_size": doc.file_size,
                "ocr_status": doc.ocr_status,
                "reason": "failed_ocr",
                "created_at": doc.created_at
            }));
        }
    }

    Ok(Json(serde_json::json!({
        "total_candidates": cleanup_candidates.len(),
        "total_size_bytes": total_size,
        "total_size_mb": (total_size as f64 / 1024.0 / 1024.0).round(),
        "candidates": cleanup_candidates,
        "criteria": {
            "max_confidence": max_confidence,
            "include_failed_ocr": include_failed
        }
    })))
}