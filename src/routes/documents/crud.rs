use axum::{
    extract::{Multipart, Path, Query, State},
    http::{StatusCode, header::CONTENT_TYPE},
    response::{Json, Response, IntoResponse},
    body::Body,
};
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use crate::{
    auth::AuthUser,
    ingestion::document_ingestion::{DocumentIngestionService, IngestionResult},
    models::DocumentResponse,
    AppState,
};
use super::types::{PaginationQuery, DocumentUploadResponse, PaginatedDocumentsResponse, DocumentPaginationInfo};

/// Custom error type for document operations
#[derive(Debug)]
pub enum DocumentError {
    BadRequest(String),
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
            DocumentError::NotFound => (StatusCode::NOT_FOUND, "Document not found".to_string(), "UPLOAD_NOT_FOUND"),
            DocumentError::Conflict(msg) => (StatusCode::CONFLICT, msg, "UPLOAD_CONFLICT"),
            DocumentError::PayloadTooLarge(msg) => (StatusCode::PAYLOAD_TOO_LARGE, msg, "UPLOAD_TOO_LARGE"),
            DocumentError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg, "UPLOAD_INTERNAL_ERROR"),
            DocumentError::UploadTimeout(msg) => (StatusCode::REQUEST_TIMEOUT, msg, "UPLOAD_TIMEOUT"),
            DocumentError::DatabaseConstraintViolation(msg) => (StatusCode::CONFLICT, msg, "UPLOAD_DB_CONSTRAINT"),
            DocumentError::OcrProcessingError(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg, "UPLOAD_OCR_ERROR"),
            DocumentError::FileProcessingError(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg, "UPLOAD_FILE_PROCESSING_ERROR"),
            DocumentError::ConcurrentUploadError(msg) => (StatusCode::TOO_MANY_REQUESTS, msg, "UPLOAD_CONCURRENT_ERROR"),
        };
        
        (status, Json(json!({
            "error": message,
            "status": status.as_u16(),
            "error_code": error_code,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "request_id": uuid::Uuid::new_v4()
        }))).into_response()
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
    
    // First pass: collect all multipart fields
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        let error_msg = format!("Failed to get multipart field: {}", e);
        error!("{}", error_msg);
        DocumentError::BadRequest(error_msg)
    })? {
        let name = field.name().unwrap_or("").to_string();
        
        if name == "ocr_language" {
            let language = field.text().await.map_err(|_| DocumentError::BadRequest("Failed to read language field".to_string()))?;
            if !language.trim().is_empty() {
                // Validate that the language is available
                let health_checker = crate::ocr::health::OcrHealthChecker::new();
                match health_checker.validate_language(language.trim()) {
                    Ok(_) => {
                        ocr_language = Some(language.trim().to_string());
                        info!("OCR language specified and validated: {}", language);
                    }
                    Err(e) => {
                        let available_languages = health_checker.get_available_languages().unwrap_or_default();
                        let error_msg = format!(
                            "Invalid OCR language '{}': {}. Available languages: {}",
                            language, e, available_languages.join(", ")
                        );
                        warn!("{}", error_msg);
                        return Err(DocumentError::BadRequest(error_msg));
                    }
                }
            }
        } else if name == "ocr_languages" || name.starts_with("ocr_languages[") {
            let language = field.text().await.map_err(|_| DocumentError::BadRequest("Failed to read language field".to_string()))?;
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
                        let available_languages = health_checker.get_available_languages().unwrap_or_default();
                        let error_msg = format!(
                            "Invalid OCR language '{}': {}. Available languages: {}",
                            language, e, available_languages.join(", ")
                        );
                        warn!("{}", error_msg);
                        return Err(DocumentError::BadRequest(error_msg));
                    }
                }
            }
        } else if name == "file" {
            let filename = field.file_name()
                .ok_or_else(|| {
                    let error_msg = "No filename provided in upload".to_string();
                    error!("{}", error_msg);
                    DocumentError::BadRequest(error_msg)
                })?
                .to_string();
            
            let content_type = field.content_type()
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
    
    // Validate file size against configured limit
    let max_file_size_bytes = state.config.max_file_size_mb as usize * 1024 * 1024;
    if data.len() > max_file_size_bytes {
        let error_msg = format!("File '{}' size ({} bytes) exceeds maximum allowed size ({} bytes / {}MB)", 
               filename, data.len(), max_file_size_bytes, state.config.max_file_size_mb);
        error!("{}", error_msg);
        return Err(DocumentError::PayloadTooLarge(error_msg));
    }
    
    info!("Uploading document: {} ({} bytes)", filename, data.len());
    
    // Create FileIngestionInfo from uploaded data
    use crate::models::FileIngestionInfo;
    use chrono::Utc;
    
    let mut file_info = FileIngestionInfo {
        relative_path: format!("upload/{}", filename), // Virtual path for web uploads
        full_path: format!("upload/{}", filename), // For web uploads, relative and full are the same
        #[allow(deprecated)]
        path: format!("upload/{}", filename), // Virtual path for web uploads
        name: filename.clone(),
        size: data.len() as i64,
        mime_type: content_type.clone(),
        last_modified: Some(Utc::now()), // Upload time as last modified
        etag: format!("{}-{}", data.len(), Utc::now().timestamp()),
        is_directory: false,
        created_at: Some(Utc::now()), // Upload time as creation time
        permissions: None, // Web uploads don't have filesystem permissions
        owner: Some(auth_user.user.username.clone()), // Uploader as owner
        group: None, // Web uploads don't have filesystem groups
        metadata: None, // Will be populated with extracted metadata below
    };
    
    // Extract content-based metadata from uploaded file
    if let Ok(Some(content_metadata)) = crate::metadata_extraction::extract_content_metadata(&data, &content_type, &filename).await {
        file_info.metadata = Some(content_metadata);
    }
    
    // Create ingestion service
    let file_service_clone = state.file_service.as_ref().clone();
    let ingestion_service = DocumentIngestionService::new(
        state.db.clone(),
        file_service_clone,
    );
    
    debug!("[UPLOAD_DEBUG] Calling ingestion service for file: {}", filename);
    let ingestion_start = std::time::Instant::now();
    
    match ingestion_service.ingest_from_file_info(
        &file_info, 
        data, 
        auth_user.user.id, 
        crate::ingestion::document_ingestion::DeduplicationPolicy::Skip, 
        "web_upload", 
        None
    ).await {
        Ok(IngestionResult::Created(document)) => {
            info!("Document uploaded successfully: {}", document.id);
            
            // Update user's OCR language settings based on what was provided
            if !ocr_languages.is_empty() {
                // Multi-language support: update preferred languages
                let health_checker = crate::ocr::health::OcrHealthChecker::new();
                match health_checker.validate_preferred_languages(&ocr_languages) {
                    Ok(_) => {
                        let settings_update = crate::models::UpdateSettings::language_update(
                            ocr_languages.clone(),
                            ocr_languages[0].clone(), // First language as primary
                            ocr_languages[0].clone(), // Backward compatibility
                        );
                        
                        if let Err(e) = state.db.create_or_update_settings(auth_user.user.id, &settings_update).await {
                            warn!("Failed to update user preferred languages to {:?}: {}", ocr_languages, e);
                        } else {
                            info!("Updated user {} preferred languages to: {:?}", auth_user.user.id, ocr_languages);
                        }
                    }
                    Err(e) => {
                        warn!("Invalid language combination provided, not updating user settings: {}", e);
                    }
                }
            } else if let Some(lang) = &ocr_language {
                // Single language (backward compatibility)
                if let Err(e) = state.db.update_user_ocr_language(auth_user.user.id, lang).await {
                    warn!("Failed to update user OCR language to {}: {}", lang, e);
                } else {
                    info!("Updated user {} OCR language to: {}", auth_user.user.id, lang);
                }
            }
            
            // Auto-enqueue all documents for processing (OCR queue handles both OCR and text extraction)
            let priority = 5; // Normal priority for direct uploads
            if let Err(e) = state.queue_service.enqueue_document(document.id, priority, document.file_size).await {
                error!("Failed to enqueue document {} for processing: {}", document.id, e);
                // Don't fail the upload if queueing fails, just log the error
            } else {
                if crate::utils::ocr::file_needs_ocr(&document.filename) {
                    info!("Document {} enqueued for OCR processing", document.id);
                } else {
                    info!("Document {} enqueued for text extraction", document.filename);
                }
            }
            
            Ok(Json(DocumentUploadResponse {
                id: document.id,
                filename: document.filename,
                file_size: document.file_size,
                mime_type: document.mime_type,
                status: "success".to_string(),
                message: "Document uploaded successfully".to_string(),
            }))
        }
        Ok(IngestionResult::ExistingDocument(existing_doc)) => {
            warn!("Duplicate document upload attempted: {}", existing_doc.id);
            Ok(Json(DocumentUploadResponse {
                id: existing_doc.id,
                filename: existing_doc.filename,
                file_size: existing_doc.file_size,
                mime_type: existing_doc.mime_type,
                status: "duplicate".to_string(),
                message: "Document already exists".to_string(),
            }))
        }
        Ok(IngestionResult::Skipped { existing_document_id, reason }) => {
            let error_msg = format!("Document upload skipped - {}: {}", reason, existing_document_id);
            info!("{}", error_msg);
            Err(DocumentError::Conflict(error_msg))
        }
        Ok(IngestionResult::TrackedAsDuplicate { existing_document_id }) => {
            let error_msg = format!("Document tracked as duplicate: {}", existing_document_id);
            info!("{}", error_msg);
            Err(DocumentError::Conflict(error_msg))
        }
        Err(e) => {
            let ingestion_duration = ingestion_start.elapsed();
            let error_msg = format!("Failed to ingest document: {} (failed after {:?})", e, ingestion_duration);
            error!("[UPLOAD_DEBUG] {}", error_msg);
            
            // Categorize the error for better client handling
            if e.to_string().contains("constraint") || e.to_string().contains("duplicate") {
                return Err(DocumentError::DatabaseConstraintViolation(format!("Database constraint violation during upload: {}", e)));
            } else if e.to_string().contains("timeout") {
                return Err(DocumentError::UploadTimeout(format!("Upload processing timed out: {}", e)));
            } else if e.to_string().contains("ocr") || e.to_string().contains("processing") {
                return Err(DocumentError::OcrProcessingError(format!("OCR processing error: {}", e)));
            } else if e.to_string().contains("file") || e.to_string().contains("read") {
                return Err(DocumentError::FileProcessingError(format!("File processing error: {}", e)));
            }
            
            Err(DocumentError::InternalServerError(error_msg))
        }
    }
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
    let document = state
        .db
        .get_document_by_id(document_id, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|e| {
            error!("Database error getting document {}: {}", document_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Get labels for this document
    let labels = state
        .db
        .get_document_labels(document_id)
        .await
        .map_err(|e| {
            error!("Failed to get labels for document {}: {}", document_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get username for the document owner
    let username = state
        .db
        .get_user_by_id(document.user_id)
        .await
        .map_err(|e| {
            error!("Failed to get user for document {}: {}", document_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map(|user| user.username);

    let mut response = DocumentResponse::from(document);
    response.labels = labels;
    response.username = username;

    Ok(Json(response))
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
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_documents(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<PaginatedDocumentsResponse>, StatusCode> {
    let limit = query.limit.unwrap_or(25);
    let offset = query.offset.unwrap_or(0);

    // Get total count for pagination
    let total_count = if let Some(ocr_status) = query.ocr_status.as_deref() {
        state
            .db
            .count_documents_by_user_with_role_and_filter(
                auth_user.user.id,
                auth_user.user.role,
                Some(ocr_status),
            )
            .await
    } else {
        state
            .db
            .count_documents_by_user_with_role(
                auth_user.user.id,
                auth_user.user.role,
            )
            .await
    }
    .map_err(|e| {
        error!("Database error counting documents: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let documents = if let Some(ocr_status) = query.ocr_status.as_deref() {
        state
            .db
            .get_documents_by_user_with_role_and_filter(
                auth_user.user.id,
                auth_user.user.role,
                Some(ocr_status),
                limit,
                offset,
            )
            .await
    } else {
        state
            .db
            .get_documents_by_user_with_role(
                auth_user.user.id,
                auth_user.user.role,
                limit,
                offset,
            )
            .await
    }
    .map_err(|e| {
        error!("Database error listing documents: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Get document IDs for batch label fetching
    let document_ids: Vec<uuid::Uuid> = documents.iter().map(|d| d.id).collect();
    
    // Get labels for all documents in batch
    let labels_map = if !document_ids.is_empty() {
        let labels = state
            .db
            .get_labels_for_documents(&document_ids)
            .await
            .map_err(|e| {
                error!("Failed to get labels for documents: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        
        labels.into_iter().collect::<std::collections::HashMap<_, _>>()
    } else {
        std::collections::HashMap::new()
    };

    // Convert to response format with labels
    let responses: Vec<DocumentResponse> = documents
        .into_iter()
        .map(|doc| {
            let mut response = DocumentResponse::from(doc.clone());
            if let Some(labels) = labels_map.get(&doc.id) {
                response.labels = labels.clone();
            }
            response
        })
        .collect();

    // Create pagination info
    let pagination = DocumentPaginationInfo {
        total: total_count,
        limit,
        offset,
        has_more: offset + limit < total_count,
    };

    Ok(Json(PaginatedDocumentsResponse {
        documents: responses,
        pagination,
    }))
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

    // Delete from database
    let deleted = state
        .db
        .delete_document(document_id, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|e| {
            error!("Database error deleting document {}: {}", document_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if !deleted {
        return Err(StatusCode::NOT_FOUND);
    }

    // Delete associated files
    let file_service = &state.file_service;
    if let Err(e) = file_service.delete_document_files(&document).await {
        warn!("Failed to delete files for document {}: {}", document_id, e);
        // Continue anyway - database deletion succeeded
    }

    info!("Document deleted successfully: {}", document_id);
    Ok(StatusCode::NO_CONTENT)
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

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, document.mime_type)
        .header("Content-Disposition", format!("attachment; filename=\"{}\"", document.original_filename))
        .header("Content-Length", file_data.len().to_string())
        .body(Body::from(file_data))
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

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, document.mime_type)
        .header("Content-Length", file_data.len().to_string())
        .body(Body::from(file_data))
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