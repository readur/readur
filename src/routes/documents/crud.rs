use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use super::types::{DocumentUploadResponse, PaginatedDocumentsResponse, PaginationQuery};
use crate::{
    auth::AuthUser, models::DocumentResponse, services::documents as documents_service, AppState,
};

/// Custom error type for document operations
#[derive(Debug)]
pub enum DocumentError {
    BadRequest(String),
    /// A list or filter query parameter failed validation.
    InvalidQuery(String),
    NotFound,
    Conflict(String),
    PayloadTooLarge(String),
    InternalServerError(String),
    UploadTimeout(String),
    DatabaseConstraintViolation(String),
    OcrProcessingError(String),
    FileProcessingError(String),
    ConcurrentUploadError(String),
}

impl IntoResponse for DocumentError {
    fn into_response(self) -> Response {
        let (status, message, error_code) = match self {
            DocumentError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg, "UPLOAD_BAD_REQUEST"),
            DocumentError::InvalidQuery(msg) => (StatusCode::BAD_REQUEST, msg, "INVALID_QUERY"),
            DocumentError::NotFound => (
                StatusCode::NOT_FOUND,
                "Document not found".to_string(),
                "UPLOAD_NOT_FOUND",
            ),
            DocumentError::Conflict(msg) => (StatusCode::CONFLICT, msg, "UPLOAD_CONFLICT"),
            DocumentError::PayloadTooLarge(msg) => {
                (StatusCode::PAYLOAD_TOO_LARGE, msg, "UPLOAD_TOO_LARGE")
            }
            DocumentError::InternalServerError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                msg,
                "UPLOAD_INTERNAL_ERROR",
            ),
            DocumentError::UploadTimeout(msg) => {
                (StatusCode::REQUEST_TIMEOUT, msg, "UPLOAD_TIMEOUT")
            }
            DocumentError::DatabaseConstraintViolation(msg) => {
                (StatusCode::CONFLICT, msg, "UPLOAD_DB_CONSTRAINT")
            }
            DocumentError::OcrProcessingError(msg) => {
                (StatusCode::UNPROCESSABLE_ENTITY, msg, "UPLOAD_OCR_ERROR")
            }
            DocumentError::FileProcessingError(msg) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                msg,
                "UPLOAD_FILE_PROCESSING_ERROR",
            ),
            DocumentError::ConcurrentUploadError(msg) => (
                StatusCode::TOO_MANY_REQUESTS,
                msg,
                "UPLOAD_CONCURRENT_ERROR",
            ),
        };

        (
            status,
            Json(json!({
                "error": message,
                "status": status.as_u16(),
                "error_code": error_code,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "request_id": uuid::Uuid::new_v4()
            })),
        )
            .into_response()
    }
}

/// Upload a new document
#[utoipa::path(
    post,
    path = "/api/documents",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    request_body(content = String, description = "Document file", content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Document uploaded successfully", body = DocumentUploadResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 413, description = "File too large"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn upload_document(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<DocumentUploadResponse>, DocumentError> {
    let mut uploaded_file = None;
    let mut ocr_language: Option<String> = None;
    let mut ocr_languages: Vec<String> = Vec::new();
    let mut label_ids: Vec<uuid::Uuid> = Vec::new();

    // First pass: collect all multipart fields
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        let error_msg = format!("Failed to get multipart field: {}", e);
        error!("{}", error_msg);
        DocumentError::BadRequest(error_msg)
    })? {
        let name = field.name().unwrap_or("").to_string();

        if name == "ocr_language" {
            let language = field.text().await.map_err(|_| {
                DocumentError::BadRequest("Failed to read language field".to_string())
            })?;
            if !language.trim().is_empty() {
                // Validate that the language is available
                let health_checker = crate::ocr::health::OcrHealthChecker::new();
                match health_checker.validate_language(language.trim()) {
                    Ok(_) => {
                        ocr_language = Some(language.trim().to_string());
                        info!("OCR language specified and validated: {}", language);
                    }
                    Err(e) => {
                        let available_languages =
                            health_checker.get_available_languages().unwrap_or_default();
                        let error_msg = format!(
                            "Invalid OCR language '{}': {}. Available languages: {}",
                            language,
                            e,
                            available_languages.join(", ")
                        );
                        warn!("{}", error_msg);
                        return Err(DocumentError::BadRequest(error_msg));
                    }
                }
            }
        } else if name == "ocr_languages" || name.starts_with("ocr_languages[") {
            let language = field.text().await.map_err(|_| {
                DocumentError::BadRequest("Failed to read language field".to_string())
            })?;
            if !language.trim().is_empty() {
                // Validate that the language is available
                let health_checker = crate::ocr::health::OcrHealthChecker::new();
                debug!("Validating OCR language: '{}'", language.trim());
                match health_checker.validate_language(language.trim()) {
                    Ok(_) => {
                        ocr_languages.push(language.trim().to_string());
                        info!("OCR language added to list: {}", language);
                    }
                    Err(e) => {
                        let available_languages =
                            health_checker.get_available_languages().unwrap_or_default();
                        let error_msg = format!(
                            "Invalid OCR language '{}': {}. Available languages: {}",
                            language,
                            e,
                            available_languages.join(", ")
                        );
                        warn!("{}", error_msg);
                        return Err(DocumentError::BadRequest(error_msg));
                    }
                }
            }
        } else if name == "label_ids" || name.starts_with("label_ids[") {
            // Repeated fields or one comma-separated list of label UUIDs.
            let value = field.text().await.map_err(|_| {
                DocumentError::BadRequest("Failed to read label_ids field".to_string())
            })?;
            for id in value.split(',').map(str::trim).filter(|id| !id.is_empty()) {
                label_ids.push(id.parse().map_err(|_| {
                    DocumentError::BadRequest(format!("Invalid label id '{}'", id))
                })?);
            }
        } else if name == "file" {
            let filename = field
                .file_name()
                .ok_or_else(|| {
                    let error_msg = "No filename provided in upload".to_string();
                    error!("{}", error_msg);
                    DocumentError::BadRequest(error_msg)
                })?
                .to_string();

            let content_type = field
                .content_type()
                .unwrap_or("application/octet-stream")
                .to_string();

            let data = field.bytes().await.map_err(|e| {
                let error_msg = format!("Failed to read file data: {}", e);
                error!("{}", error_msg);
                DocumentError::BadRequest(error_msg)
            })?;

            uploaded_file = Some((filename, content_type, data.to_vec()));
        }
    }

    let (filename, content_type, data) = uploaded_file.ok_or_else(|| {
        let error_msg = "No file found in upload".to_string();
        error!("{}", error_msg);
        DocumentError::BadRequest(error_msg)
    })?;

    let response = documents_service::upload_document(
        &state,
        &auth_user.user,
        documents_service::UploadRequest {
            filename,
            mime_type: content_type,
            data,
            ocr_languages,
            ocr_language,
            label_ids,
        },
    )
    .await?;
    Ok(Json(response))
}

/// Get a specific document by ID
#[utoipa::path(
    get,
    path = "/api/documents/{id}",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = uuid::Uuid, Path, description = "Document ID")
    ),
    responses(
        (status = 200, description = "Document details", body = DocumentResponse),
        (status = 404, description = "Document not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_document_by_id(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<uuid::Uuid>,
) -> Result<Json<DocumentResponse>, StatusCode> {
    documents_service::get_document(&state, &auth_user.user, document_id)
        .await
        .map_err(|e| {
            error!("Database error getting document {}: {}", document_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// List documents with pagination and filtering
#[utoipa::path(
    get,
    path = "/api/documents",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    params(PaginationQuery),
    responses(
        (status = 200, description = "Paginated list of documents", body = PaginatedDocumentsResponse),
        (status = 400, description = "Invalid sort or filter parameter"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_documents(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<PaginatedDocumentsResponse>, DocumentError> {
    let filters = query.filters();
    filters.validate().map_err(DocumentError::InvalidQuery)?;

    let response = documents_service::list_documents(
        &state,
        &auth_user.user,
        &filters,
        query.sort_by,
        query.sort_order,
        query.limit.unwrap_or(25),
        query.offset.unwrap_or(0),
    )
    .await
    .map_err(|e| {
        error!("Database error listing documents: {}", e);
        DocumentError::InternalServerError("Failed to list documents".to_string())
    })?;
    Ok(Json(response))
}

/// Delete a specific document
#[utoipa::path(
    delete,
    path = "/api/documents/{id}",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = uuid::Uuid, Path, description = "Document ID")
    ),
    responses(
        (status = 204, description = "Document deleted successfully"),
        (status = 404, description = "Document not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_document(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<uuid::Uuid>,
) -> Result<StatusCode, StatusCode> {
    let deleted = documents_service::delete_document(&state, &auth_user.user, document_id)
        .await
        .map_err(|e| {
            error!("Database error deleting document {}: {}", document_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Download a document file
#[utoipa::path(
    get,
    path = "/api/documents/{id}/download",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = uuid::Uuid, Path, description = "Document ID")
    ),
    responses(
        (status = 200, description = "Document file", content_type = "application/octet-stream"),
        (status = 404, description = "Document not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn download_document(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<uuid::Uuid>,
) -> Result<Response<Body>, StatusCode> {
    let document = state
        .db
        .get_document_by_id(document_id, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|e| {
            error!("Database error getting document {}: {}", document_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let file_service = &state.file_service;
    let file_data = file_service
        .read_file(&document.file_path)
        .await
        .map_err(|e| {
            error!("Failed to read document file {}: {}", document_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let response = crate::http_security::user_content_response(
        &document.mime_type,
        &document.original_filename,
        false,
        file_data,
    )
    .map_err(|e| {
        error!("Failed to build response: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    debug!("Document downloaded: {}", document_id);
    Ok(response)
}

/// View a document in the browser
#[utoipa::path(
    get,
    path = "/api/documents/{id}/view",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = uuid::Uuid, Path, description = "Document ID")
    ),
    responses(
        (status = 200, description = "Document file for viewing", content_type = "application/octet-stream"),
        (status = 404, description = "Document not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn view_document(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<uuid::Uuid>,
) -> Result<Response<Body>, StatusCode> {
    let document = state
        .db
        .get_document_by_id(document_id, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|e| {
            error!("Database error getting document {}: {}", document_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let file_service = &state.file_service;
    let file_data = file_service
        .read_file(&document.file_path)
        .await
        .map_err(|e| {
            error!("Failed to read document file {}: {}", document_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let response = crate::http_security::user_content_response(
        &document.mime_type,
        &document.original_filename,
        true,
        file_data,
    )
    .map_err(|e| {
        error!("Failed to build response: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    debug!("Document viewed: {}", document_id);
    Ok(response)
}

/// Get user's duplicate documents
#[utoipa::path(
    get,
    path = "/api/documents/duplicates",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("limit" = Option<i64>, Query, description = "Number of duplicate groups to return per page"),
        ("offset" = Option<i64>, Query, description = "Number of duplicate groups to skip")
    ),
    responses(
        (status = 200, description = "User's duplicate documents grouped by hash", body = serde_json::Value),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_user_duplicates(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let limit = query.limit.unwrap_or(25);
    let offset = query.offset.unwrap_or(0);

    let duplicates = state
        .db
        .get_user_duplicates(auth_user.user.id, auth_user.user.role, limit, offset)
        .await
        .map_err(|e| {
            error!("Failed to get user duplicates: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let total_count = duplicates.len() as i64;

    let response = serde_json::json!({
        "duplicates": duplicates,
        "pagination": {
            "total": total_count,
            "limit": limit,
            "offset": offset,
            "has_more": offset + limit < total_count
        },
        "statistics": {
            "total_duplicate_groups": total_count
        }
    });

    Ok(Json(response))
}
