use axum::{
    extract::{Multipart, Path, Query, State},
    http::{StatusCode, header::CONTENT_TYPE},
    response::{Json, Response},
    body::Body,
};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use crate::{
    auth::AuthUser,
    ingestion::document_ingestion::{DocumentIngestionService, IngestionResult},
    services::file_service::FileService,
    models::DocumentResponse,
    AppState,
};
use super::types::{PaginationQuery, DocumentUploadResponse, PaginatedDocumentsResponse, DocumentPaginationInfo};

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
) -> Result<Json<DocumentUploadResponse>, StatusCode> {
    let mut uploaded_file = None;
    let mut ocr_language: Option<String> = None;
    
    // First pass: collect all multipart fields
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        error!("Failed to get multipart field: {}", e);
        StatusCode::BAD_REQUEST
    })? {
        let name = field.name().unwrap_or("").to_string();
        
        if name == "ocr_language" {
            let language = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?;
            if !language.trim().is_empty() {
                // Validate that the language is available
                let health_checker = crate::ocr::health::OcrHealthChecker::new();
                match health_checker.validate_language(language.trim()) {
                    Ok(_) => {
                        ocr_language = Some(language.trim().to_string());
                        info!("OCR language specified and validated: {}", language);
                    }
                    Err(e) => {
                        warn!("Invalid OCR language specified '{}': {}", language, e);
                        // Return early with bad request for invalid language
                        return Err(StatusCode::BAD_REQUEST);
                    }
                }
            }
        } else if name == "file" {
            let filename = field.file_name()
                .ok_or_else(|| {
                    error!("No filename provided in upload");
                    StatusCode::BAD_REQUEST
                })?
                .to_string();
            
            let content_type = field.content_type()
                .unwrap_or("application/octet-stream")
                .to_string();
            
            let data = field.bytes().await.map_err(|e| {
                error!("Failed to read file data: {}", e);
                StatusCode::BAD_REQUEST
            })?;
            
            uploaded_file = Some((filename, content_type, data.to_vec()));
        }
    }
    
    let (filename, content_type, data) = uploaded_file.ok_or_else(|| {
        error!("No file found in upload");
        StatusCode::BAD_REQUEST
    })?;
    
    // Validate file size against configured limit
    let max_file_size_bytes = state.config.max_file_size_mb as usize * 1024 * 1024;
    if data.len() > max_file_size_bytes {
        error!("File '{}' size ({} bytes) exceeds maximum allowed size ({} bytes / {}MB)", 
               filename, data.len(), max_file_size_bytes, state.config.max_file_size_mb);
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    
    info!("Uploading document: {} ({} bytes)", filename, data.len());
    
    // Create FileIngestionInfo from uploaded data
    use crate::models::FileIngestionInfo;
    use chrono::Utc;
    
    let mut file_info = FileIngestionInfo {
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
    let file_service = FileService::new(state.config.upload_path.clone());
    let ingestion_service = DocumentIngestionService::new(
        state.db.clone(),
        file_service,
    );
    
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
            
            // If a language was specified, update the user's OCR language setting
            if let Some(lang) = &ocr_language {
                if let Err(e) = state.db.update_user_ocr_language(auth_user.user.id, lang).await {
                    warn!("Failed to update user OCR language to {}: {}", lang, e);
                } else {
                    info!("Updated user {} OCR language to: {}", auth_user.user.id, lang);
                }
            }
            
            // Auto-enqueue document for OCR processing
            let priority = 5; // Normal priority for direct uploads
            if let Err(e) = state.queue_service.enqueue_document(document.id, priority, document.file_size).await {
                error!("Failed to enqueue document {} for OCR: {}", document.id, e);
                // Don't fail the upload if OCR queueing fails, just log the error
            } else {
                info!("Document {} enqueued for OCR processing", document.id);
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
            info!("Document upload skipped - {}: {}", reason, existing_document_id);
            Err(StatusCode::CONFLICT)
        }
        Ok(IngestionResult::TrackedAsDuplicate { existing_document_id }) => {
            info!("Document tracked as duplicate: {}", existing_document_id);
            Err(StatusCode::CONFLICT)
        }
        Err(e) => {
            error!("Failed to ingest document: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
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
    let file_service = FileService::new(state.config.upload_path.clone());
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

    let file_service = FileService::new(state.config.upload_path.clone());
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

    let file_service = FileService::new(state.config.upload_path.clone());
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