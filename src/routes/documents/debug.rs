use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::{
    auth::AuthUser,
    services::file_service::FileService,
    AppState,
};
use super::types::DocumentDebugInfo;

/// Get comprehensive debug information for a document
#[utoipa::path(
    get,
    path = "/api/documents/{id}/debug",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = uuid::Uuid, Path, description = "Document ID")
    ),
    responses(
        (status = 200, description = "Document debug information", body = DocumentDebugInfo),
        (status = 404, description = "Document not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_document_debug_info(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<uuid::Uuid>,
) -> Result<Json<DocumentDebugInfo>, StatusCode> {
    let document = state
        .db
        .get_document_by_id(document_id, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|e| {
            error!("Database error getting document {}: {}", document_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let file_service = FileService::new(state.config.upload_path.clone());
    
    // Check file existence and readability
    let file_exists = tokio::fs::metadata(&document.file_path).await.is_ok();
    let readable = if file_exists {
        file_service.read_file(&document.file_path).await.is_ok()
    } else {
        false
    };

    // Get file permissions (simplified)
    let permissions = if file_exists {
        Some("readable".to_string()) // This could be expanded with actual file permissions
    } else {
        None
    };

    // Construct processing steps based on document state
    let mut processing_steps = vec!["uploaded".to_string()];
    
    if document.content.is_some() {
        processing_steps.push("content_extracted".to_string());
    }
    
    match document.ocr_status.as_deref() {
        Some("pending") => processing_steps.push("ocr_queued".to_string()),
        Some("processing") => processing_steps.push("ocr_in_progress".to_string()),
        Some("completed") => processing_steps.push("ocr_completed".to_string()),
        Some("failed") => processing_steps.push("ocr_failed".to_string()),
        _ => {}
    }

    if document.ocr_text.is_some() {
        processing_steps.push("ocr_text_available".to_string());
    }

    let debug_info = DocumentDebugInfo {
        document_id: document.id,
        filename: document.original_filename,
        file_path: document.file_path,
        file_size: document.file_size,
        mime_type: document.mime_type,
        created_at: document.created_at,
        ocr_status: document.ocr_status,
        ocr_confidence: document.ocr_confidence,
        ocr_word_count: document.ocr_word_count,
        processing_steps,
        file_exists,
        readable,
        permissions,
    };

    debug!("Debug info generated for document: {}", document_id);
    Ok(Json(debug_info))
}

/// Get thumbnail for a document (if available)
#[utoipa::path(
    get,
    path = "/api/documents/{id}/thumbnail",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = uuid::Uuid, Path, description = "Document ID")
    ),
    responses(
        (status = 200, description = "Document thumbnail", content_type = "image/jpeg"),
        (status = 404, description = "Document or thumbnail not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_document_thumbnail(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<uuid::Uuid>,
) -> Result<axum::response::Response, StatusCode> {
    let document = state
        .db
        .get_document_by_id(document_id, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|e| {
            error!("Database error getting document {}: {}", document_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let file_service = FileService::new(state.config.upload_path.clone());
    
    // Try to read thumbnail from the thumbnails directory
    let thumbnail_path = format!("{}/thumbnails/{}.jpg", state.config.upload_path, document.id);
    match file_service.read_file(&thumbnail_path).await {
        Ok(data) => {
            let response = axum::response::Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "image/jpeg")
                .header("Content-Length", data.len().to_string())
                .header("Cache-Control", "public, max-age=3600") // Cache for 1 hour
                .body(axum::body::Body::from(data))
                .map_err(|e| {
                    error!("Failed to build thumbnail response: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            debug!("Thumbnail served for document: {}", document_id);
            Ok(response)
        }
        Err(_) => {
            // Return a default "no thumbnail" response or generate one on the fly
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// Get processed image for a document (if available)
#[utoipa::path(
    get,
    path = "/api/documents/{id}/processed-image",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = uuid::Uuid, Path, description = "Document ID")
    ),
    responses(
        (status = 200, description = "Processed image", content_type = "image/png"),
        (status = 404, description = "Document or processed image not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_processed_image(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<uuid::Uuid>,
) -> Result<axum::response::Response, StatusCode> {
    let document = state
        .db
        .get_document_by_id(document_id, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|e| {
            error!("Database error getting document {}: {}", document_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Check if this is an image document
    if !document.mime_type.starts_with("image/") {
        return Err(StatusCode::BAD_REQUEST);
    }

    let file_service = FileService::new(state.config.upload_path.clone());
    
    // Try to read processed image from the processed directory
    let processed_path = format!("{}/processed/{}.png", state.config.upload_path, document.id);
    match file_service.read_file(&processed_path).await {
        Ok(image_data) => {
            let response = axum::response::Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "image/png")
                .header("Content-Length", image_data.len().to_string())
                .header("Cache-Control", "public, max-age=3600") // Cache for 1 hour
                .body(axum::body::Body::from(image_data))
                .map_err(|e| {
                    error!("Failed to build processed image response: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            debug!("Processed image served for document: {}", document_id);
            Ok(response)
        }
        Err(_) => {
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// Get system-wide document statistics
pub async fn get_document_statistics(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Get OCR statistics
    let (total, pending, completed, failed) = state
        .db
        .count_documents_by_ocr_status(auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|e| {
            error!("Database error getting OCR stats: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get MIME type distribution
    let mime_type_facets = state
        .db
        .get_mime_type_facets(auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|e| {
            error!("Database error getting MIME type facets: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get recent upload activity (simplified)
    let recent_documents = state
        .db
        .get_documents_by_user_with_role(auth_user.user.id, auth_user.user.role, 10, 0)
        .await
        .map_err(|e| {
            error!("Database error getting recent documents: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let total_file_size: i64 = recent_documents.iter().map(|d| d.file_size).sum();

    Ok(Json(serde_json::json!({
        "document_counts": {
            "total": total,
            "pending_ocr": pending,
            "completed_ocr": completed,
            "failed_ocr": failed
        },
        "mime_types": mime_type_facets,
        "storage": {
            "recent_documents_size": total_file_size,
            "recent_documents_count": recent_documents.len()
        },
        "activity": {
            "recent_uploads": recent_documents.len(),
            "last_upload": recent_documents.first().map(|d| d.created_at)
        }
    })))
}

/// Validate document integrity
pub async fn validate_document_integrity(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let document = state
        .db
        .get_document_by_id(document_id, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|e| {
            error!("Database error getting document {}: {}", document_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let file_service = FileService::new(state.config.upload_path.clone());
    let mut issues = Vec::new();
    let mut checks = Vec::new();

    // Check file existence
    checks.push("file_existence".to_string());
    if tokio::fs::metadata(&document.file_path).await.is_err() {
        issues.push("File does not exist on disk".to_string());
    }

    // Check file readability
    checks.push("file_readability".to_string());
    match file_service.read_file(&document.file_path).await {
        Ok(data) => {
            // Verify file size matches
            if data.len() as i64 != document.file_size {
                issues.push(format!(
                    "File size mismatch: database={}, actual={}",
                    document.file_size,
                    data.len()
                ));
            }
        }
        Err(e) => {
            issues.push(format!("Cannot read file: {}", e));
        }
    }

    // Check OCR consistency
    checks.push("ocr_consistency".to_string());
    if document.ocr_text.is_some() && document.ocr_status.as_deref() != Some("completed") {
        issues.push("OCR text exists but status is not 'completed'".to_string());
    }

    if document.ocr_text.is_none() && document.ocr_status.as_deref() == Some("completed") {
        issues.push("OCR status is 'completed' but no OCR text available".to_string());
    }

    // Check confidence consistency
    checks.push("confidence_consistency".to_string());
    if let Some(confidence) = document.ocr_confidence {
        if confidence < 0.0 || confidence > 100.0 {
            issues.push(format!("Invalid OCR confidence value: {}", confidence));
        }
    }

    let is_valid = issues.is_empty();

    info!("Document {} integrity check: {} issues found", document_id, issues.len());

    Ok(Json(serde_json::json!({
        "document_id": document_id,
        "is_valid": is_valid,
        "checks_performed": checks,
        "issues": issues,
        "summary": if is_valid {
            "Document integrity is good".to_string()
        } else {
            format!("Found {} integrity issues", issues.len())
        }
    })))
}