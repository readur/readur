use axum::{
    extract::{Multipart, Path, Query, State},
    http::{StatusCode, header::CONTENT_TYPE},
    response::{Json, Response},
    routing::{get, post, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use sqlx::Row;

use crate::{
    auth::AuthUser,
    ingestion::document_ingestion::{DocumentIngestionService, IngestionResult},
    services::file_service::FileService,
    models::DocumentResponse,
    AppState,
};
use tracing;

#[derive(Deserialize, ToSchema)]
struct PaginationQuery {
    limit: Option<i64>,
    offset: Option<i64>,
    ocr_status: Option<String>,
}

#[derive(Deserialize, ToSchema)]
struct FailedDocumentsQuery {
    limit: Option<i64>,
    offset: Option<i64>,
    stage: Option<String>,  // 'ocr', 'ingestion', 'validation', etc.
    reason: Option<String>, // 'duplicate_content', 'low_ocr_confidence', etc.
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct BulkDeleteRequest {
    pub document_ids: Vec<uuid::Uuid>,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct DeleteLowConfidenceRequest {
    pub max_confidence: f32,
    pub preview_only: Option<bool>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(upload_document))
        .route("/", get(list_documents))
        .route("/", delete(bulk_delete_documents))
        .route("/{id}", get(get_document_by_id))
        .route("/{id}", delete(delete_document))
        .route("/{id}/download", get(download_document))
        .route("/{id}/view", get(view_document))
        .route("/{id}/thumbnail", get(get_document_thumbnail))
        .route("/{id}/ocr", get(get_document_ocr))
        .route("/{id}/processed-image", get(get_processed_image))
        .route("/{id}/retry-ocr", post(retry_ocr))
        .route("/{id}/debug", get(get_document_debug_info))
        .route("/duplicates", get(get_user_duplicates))
        .route("/failed", get(get_failed_documents))
        .route("/failed/{id}/view", get(view_failed_document))
        .route("/delete-low-confidence", post(delete_low_confidence_documents))
        .route("/delete-failed-ocr", post(delete_failed_ocr_documents))
}

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
async fn get_document_by_id(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<uuid::Uuid>,
) -> Result<Json<DocumentResponse>, StatusCode> {
    // Get specific document with proper role-based access
    let document = state
        .db
        .get_document_by_id(document_id, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    
    // Get labels for this document
    let labels = state
        .db
        .get_document_labels(document_id)
        .await
        .unwrap_or_else(|_| Vec::new());
    
    // Convert to DocumentResponse
    let response = DocumentResponse {
        id: document.id,
        filename: document.filename,
        original_filename: document.original_filename,
        file_size: document.file_size,
        mime_type: document.mime_type,
        created_at: document.created_at,
        has_ocr_text: document.ocr_text.is_some(),
        tags: document.tags,
        labels,
        ocr_confidence: document.ocr_confidence,
        ocr_word_count: document.ocr_word_count,
        ocr_processing_time_ms: document.ocr_processing_time_ms,
        ocr_status: document.ocr_status,
        original_created_at: document.original_created_at,
        original_modified_at: document.original_modified_at,
        source_metadata: document.source_metadata,
    };
    
    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/api/documents",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    request_body(content = String, description = "Multipart form data with file. Supported formats: PDF, PNG, JPG, JPEG, TIFF, BMP, TXT. OCR will be automatically performed on image and PDF files.", content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Document uploaded successfully. OCR processing will begin automatically if enabled in user settings.", body = DocumentResponse),
        (status = 400, description = "Bad request - invalid file type or malformed data"),
        (status = 413, description = "Payload too large - file exceeds size limit"),
        (status = 401, description = "Unauthorized - valid authentication required")
    )
)]
async fn upload_document(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<DocumentResponse>, StatusCode> {
    let file_service = FileService::new(state.config.upload_path.clone());
    let ingestion_service = DocumentIngestionService::new(state.db.clone(), file_service.clone());
    
    // Get user settings for file upload restrictions
    let settings = state
        .db
        .get_user_settings(auth_user.user.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .unwrap_or_else(|| crate::models::Settings::default());
    
    let mut label_ids: Option<Vec<uuid::Uuid>> = None;
    
    // First pass: collect all multipart fields
    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        let name = field.name().unwrap_or("").to_string();
        
        tracing::info!("Processing multipart field: {}", name);
        
        if name == "label_ids" {
            let label_ids_text = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?;
            tracing::info!("Received label_ids field: {}", label_ids_text);
            
            match serde_json::from_str::<Vec<uuid::Uuid>>(&label_ids_text) {
                Ok(ids) => {
                    tracing::info!("Successfully parsed {} label IDs: {:?}", ids.len(), ids);
                    label_ids = Some(ids);
                },
                Err(e) => {
                    tracing::warn!("Failed to parse label_ids from upload: {} - Error: {}", label_ids_text, e);
                }
            }
        } else if name == "file" {
            let filename = field
                .file_name()
                .ok_or(StatusCode::BAD_REQUEST)?
                .to_string();
            
            let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
            let data_len = data.len();
            let file_size = data.len() as i64;
            tracing::info!("Received file: {}, size: {} bytes", filename, data_len);
            
            // Check file size limit
            let max_size_bytes = (settings.max_file_size_mb as i64) * 1024 * 1024;
            if file_size > max_size_bytes {
                return Err(StatusCode::PAYLOAD_TOO_LARGE);
            }
            
            let mime_type = mime_guess::from_path(&filename)
                .first_or_octet_stream()
                .to_string();
            
            // Use the unified ingestion service with AllowDuplicateContent policy
            // This will create separate documents for different filenames even with same content
            let result = ingestion_service
                .ingest_upload(&filename, data.to_vec(), &mime_type, auth_user.user.id)
                .await
                .map_err(|e| {
                    tracing::error!("Document ingestion failed for user {} filename {}: {}", 
                                   auth_user.user.id, filename, e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
            
            let (saved_document, should_queue_ocr) = match result {
                IngestionResult::Created(doc) => (doc, true), // New document - queue for OCR
                IngestionResult::ExistingDocument(doc) => (doc, false), // Existing document - don't re-queue OCR
                _ => return Err(StatusCode::INTERNAL_SERVER_ERROR),
            };
            
            let document_id = saved_document.id;
            let enable_background_ocr = settings.enable_background_ocr;
            
            if enable_background_ocr && should_queue_ocr {
                // Use the shared queue service from AppState instead of creating a new one
                // Calculate priority based on file size
                let priority = match saved_document.file_size {
                    0..=1048576 => 10,          // <= 1MB: highest priority
                    ..=5242880 => 8,            // 1-5MB: high priority
                    ..=10485760 => 6,           // 5-10MB: medium priority  
                    ..=52428800 => 4,           // 10-50MB: low priority
                    _ => 2,                     // > 50MB: lowest priority
                };
                
                state.queue_service.enqueue_document(document_id, priority, saved_document.file_size).await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            }
            
            return Ok(Json(saved_document.into()));
        }
    }
    
    // This should not be reached as file processing is handled above
    // If we get here, no file was provided
    
    Err(StatusCode::BAD_REQUEST)
}


#[utoipa::path(
    get,
    path = "/api/documents",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("limit" = Option<i64>, Query, description = "Number of documents to return (default: 50)"),
        ("offset" = Option<i64>, Query, description = "Number of documents to skip (default: 0)"),
        ("ocr_status" = Option<String>, Query, description = "Filter by OCR status (pending, processing, completed, failed)")
    ),
    responses(
        (status = 200, description = "Paginated list of user documents with metadata", body = String),
        (status = 401, description = "Unauthorized")
    )
)]
async fn list_documents(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let limit = pagination.limit.unwrap_or(50);
    let offset = pagination.offset.unwrap_or(0);
    
    let user_id = auth_user.user.id;
    let user_role = auth_user.user.role;
    let ocr_filter = pagination.ocr_status.as_deref();
    
    let (documents, total_count) = tokio::try_join!(
        state.db.get_documents_by_user_with_role_and_filter(
            user_id, 
            user_role.clone(), 
            limit, 
            offset, 
            ocr_filter
        ),
        state.db.get_documents_count_with_role_and_filter(
            user_id,
            user_role,
            ocr_filter
        )
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Get labels for all documents efficiently
    let document_ids: Vec<uuid::Uuid> = documents.iter().map(|doc| doc.id).collect();
    let labels_map = state
        .db
        .get_labels_for_documents(&document_ids)
        .await
        .unwrap_or_else(|_| std::collections::HashMap::new());
    
    let documents_response: Vec<DocumentResponse> = documents.into_iter().map(|doc| {
        let mut response: DocumentResponse = doc.into();
        response.labels = labels_map.get(&response.id).cloned().unwrap_or_else(Vec::new);
        response
    }).collect();
    
    let response = serde_json::json!({
        "documents": documents_response,
        "pagination": {
            "total": total_count,
            "limit": limit,
            "offset": offset,
            "has_more": offset + limit < total_count
        }
    });
    
    Ok(Json(response))
}

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
        (status = 200, description = "Document file content", content_type = "application/octet-stream"),
        (status = 404, description = "Document not found"),
        (status = 401, description = "Unauthorized")
    )
)]
async fn download_document(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<uuid::Uuid>,
) -> Result<Vec<u8>, StatusCode> {
    let document = state
        .db
        .get_document_by_id(document_id, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    
    let file_service = FileService::new(state.config.upload_path.clone());
    let file_data = file_service
        .read_file(&document.file_path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(file_data)
}

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
        (status = 200, description = "Document content for viewing in browser"),
        (status = 404, description = "Document not found"),
        (status = 401, description = "Unauthorized")
    )
)]
async fn view_document(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<uuid::Uuid>,
) -> Result<Response, StatusCode> {
    let document = state
        .db
        .get_document_by_id(document_id, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    
    let file_service = FileService::new(state.config.upload_path.clone());
    let file_data = file_service
        .read_file(&document.file_path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Determine content type from file extension
    let content_type = mime_guess::from_path(&document.filename)
        .first_or_octet_stream()
        .to_string();
    
    let response = Response::builder()
        .header(CONTENT_TYPE, content_type)
        .header("Content-Length", file_data.len())
        .body(file_data.into())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(response)
}

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
        (status = 200, description = "Document thumbnail image", content_type = "image/jpeg"),
        (status = 404, description = "Document not found or thumbnail not available"),
        (status = 401, description = "Unauthorized")
    )
)]
async fn get_document_thumbnail(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<uuid::Uuid>,
) -> Result<Response, StatusCode> {
    let document = state
        .db
        .get_document_by_id(document_id, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    
    let file_service = FileService::new(state.config.upload_path.clone());
    
    // Try to generate or get cached thumbnail
    match file_service.get_or_generate_thumbnail(&document.file_path, &document.filename).await {
        Ok(thumbnail_data) => {
            Ok(Response::builder()
                .header(CONTENT_TYPE, "image/jpeg")
                .header("Content-Length", thumbnail_data.len())
                .header("Cache-Control", "public, max-age=3600") // Cache for 1 hour
                .body(thumbnail_data.into())
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
        }
        Err(e) => {
            // Log the error for debugging
            tracing::error!("Failed to generate thumbnail for document {}: {}", document_id, e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

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
        (status = 200, description = "OCR extracted text and metadata", body = String),
        (status = 404, description = "Document not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_document_ocr(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let document = state
        .db
        .get_document_by_id(document_id, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    
    // Return OCR text and metadata
    Ok(Json(serde_json::json!({
        "document_id": document.id,
        "filename": document.filename,
        "has_ocr_text": document.ocr_text.is_some(),
        "ocr_text": document.ocr_text,
        "ocr_confidence": document.ocr_confidence,
        "ocr_word_count": document.ocr_word_count,
        "ocr_processing_time_ms": document.ocr_processing_time_ms,
        "ocr_status": document.ocr_status,
        "ocr_error": document.ocr_error,
        "ocr_completed_at": document.ocr_completed_at
    })))
}

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
        (status = 200, description = "Processed image file", content_type = "image/png"),
        (status = 404, description = "Document or processed image not found"),
        (status = 401, description = "Unauthorized")
    )
)]
async fn get_processed_image(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<uuid::Uuid>,
) -> Result<Response, StatusCode> {
    // Check if document exists and belongs to user
    let _document = state
        .db
        .get_document_by_id(document_id, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    
    // Get processed image record
    let processed_image = state
        .db
        .get_processed_image_by_document_id(document_id, auth_user.user.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    
    // Read processed image file
    let image_data = tokio::fs::read(&processed_image.processed_image_path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    
    // Return image as PNG
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "image/png")
        .header("Cache-Control", "public, max-age=86400") // Cache for 1 day
        .body(image_data.into())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(response)
}

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
    responses(
        (status = 200, description = "OCR retry queued successfully", body = String),
        (status = 404, description = "Document not found"),
        (status = 400, description = "Document is not eligible for OCR retry"),
        (status = 401, description = "Unauthorized")
    )
)]
async fn retry_ocr(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Check if document exists and belongs to user
    let document = state
        .db
        .get_document_by_id(document_id, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    
    // Check if document is eligible for OCR retry (failed or not processed)
    let eligible = document.ocr_status.as_ref().map_or(true, |status| {
        status == "failed" || status == "pending"
    });
    
    if !eligible {
        return Ok(Json(serde_json::json!({
            "success": false,
            "message": "Document is not eligible for OCR retry. Current status: {}",
            "current_status": document.ocr_status
        })));
    }
    
    // Reset document OCR fields
    let reset_result = sqlx::query(
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
        "#
    )
    .bind(document_id)
    .execute(state.db.get_pool())
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if reset_result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    
    // Calculate priority based on file size (higher priority for retries)
    let priority = match document.file_size {
        0..=1048576 => 15,          // <= 1MB: highest priority (boosted for retry)
        ..=5242880 => 12,           // 1-5MB: high priority
        ..=10485760 => 10,          // 5-10MB: medium priority  
        ..=52428800 => 8,           // 10-50MB: low priority
        _ => 6,                     // > 50MB: lowest priority
    };
    
    // Add to OCR queue with detailed logging
    match state.queue_service.enqueue_document(document_id, priority, document.file_size).await {
        Ok(queue_id) => {
            tracing::info!(
                "OCR retry queued for document {} ({}): queue_id={}, priority={}, size={}",
                document_id, document.filename, queue_id, priority, document.file_size
            );
            
            Ok(Json(serde_json::json!({
                "success": true,
                "message": "OCR retry queued successfully",
                "queue_id": queue_id,
                "document_id": document_id,
                "priority": priority,
                "estimated_wait_minutes": calculate_estimated_wait_time(priority).await
            })))
        }
        Err(e) => {
            tracing::error!("Failed to queue OCR retry for document {}: {}", document_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

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
        (status = 200, description = "Debug information for document processing pipeline", body = String),
        (status = 404, description = "Document not found"),
        (status = 401, description = "Unauthorized")
    )
)]
async fn get_document_debug_info(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    tracing::info!("Starting debug analysis for document {} by user {}", document_id, auth_user.user.id);
    
    // Get the document
    let document = match state
        .db
        .get_document_by_id(document_id, auth_user.user.id, auth_user.user.role)
        .await
    {
        Ok(Some(doc)) => {
            tracing::info!("Found document: {} ({})", doc.filename, doc.mime_type);
            doc
        }
        Ok(None) => {
            tracing::warn!("Document {} not found for user {}", document_id, auth_user.user.id);
            return Err(StatusCode::NOT_FOUND);
        }
        Err(e) => {
            tracing::error!("Database error fetching document {}: {}", document_id, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Get user settings
    tracing::info!("Fetching user settings for user {}", auth_user.user.id);
    let settings = match state
        .db
        .get_user_settings(auth_user.user.id)
        .await
    {
        Ok(Some(s)) => {
            tracing::info!("Found user settings: OCR enabled={}, min_confidence={}", s.enable_background_ocr, s.ocr_min_confidence);
            s
        }
        Ok(None) => {
            tracing::info!("No user settings found, using defaults");
            crate::models::Settings::default()
        }
        Err(e) => {
            tracing::error!("Error fetching user settings: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Get OCR queue history for this document
    tracing::info!("Fetching OCR queue history for document {}", document_id);
    let queue_history = match sqlx::query(
        r#"
        SELECT id, status, priority, created_at, started_at, completed_at, 
               error_message, attempts, worker_id
        FROM ocr_queue 
        WHERE document_id = $1 
        ORDER BY created_at DESC
        LIMIT 10
        "#
    )
    .bind(document_id)
    .fetch_all(state.db.get_pool())
    .await {
        Ok(history) => {
            tracing::info!("Queue history query successful, found {} entries", history.len());
            history
        },
        Err(e) => {
            tracing::error!("Queue history query error: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Get processed image info if it exists
    tracing::info!("Fetching processed image for document {}", document_id);
    let processed_image = match state
        .db
        .get_processed_image_by_document_id(document_id, auth_user.user.id)
        .await {
        Ok(Some(img)) => {
            tracing::info!("Found processed image for document {}", document_id);
            Some(img)
        },
        Ok(None) => {
            tracing::info!("No processed image found for document {}", document_id);
            None
        },
        Err(e) => {
            tracing::warn!("Error fetching processed image for document {}: {}", document_id, e);
            None
        }
    };

    // Get failed document record if it exists
    tracing::info!("Fetching failed document record for document {}", document_id);
    let failed_document = match sqlx::query(
        r#"
        SELECT failure_reason, failure_stage, error_message, retry_count, 
               last_retry_at, created_at, content, ocr_text, ocr_confidence,
               ocr_word_count, ocr_processing_time_ms
        FROM failed_documents 
        WHERE id = $1 OR existing_document_id = $1
        ORDER BY created_at DESC
        LIMIT 1
        "#
    )
    .bind(document_id)
    .fetch_optional(state.db.get_pool())
    .await {
        Ok(result) => {
            tracing::info!("Failed document query successful, found: {}", result.is_some());
            result
        },
        Err(e) => {
            tracing::error!("Failed document query error: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Get detailed OCR processing logs and attempts
    tracing::info!("Fetching detailed OCR processing logs for document {}", document_id);
    let ocr_processing_logs = match sqlx::query(
        r#"
        SELECT id, status, priority, created_at, started_at, completed_at,
               error_message, attempts, worker_id, processing_time_ms, file_size
        FROM ocr_queue 
        WHERE document_id = $1 
        ORDER BY created_at ASC
        "#
    )
    .bind(document_id)
    .fetch_all(state.db.get_pool())
    .await {
        Ok(logs) => {
            tracing::info!("OCR processing logs query successful, found {} entries", logs.len());
            logs
        },
        Err(e) => {
            tracing::error!("OCR processing logs query error: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // File service for file info
    let file_service = FileService::new(state.config.upload_path.clone());
    
    // Check if file exists
    let file_exists = tokio::fs::metadata(&document.file_path).await.is_ok();
    let file_metadata = if file_exists {
        tokio::fs::metadata(&document.file_path).await.ok()
    } else {
        None
    };

    // Try to analyze file content for additional diagnostic info
    tracing::info!("Analyzing file content for document {} (exists: {})", document_id, file_exists);
    let file_analysis = if file_exists {
        match analyze_file_content(&document.file_path, &document.mime_type).await {
            Ok(analysis) => {
                tracing::info!("File analysis successful for document {}", document_id);
                analysis
            },
            Err(e) => {
                tracing::warn!("Failed to analyze file content for {}: {}", document_id, e);
                FileAnalysis {
                    error_details: Some(format!("File analysis failed: {}", e)),
                    ..Default::default()
                }
            }
        }
    } else {
        tracing::warn!("File does not exist for document {}, skipping analysis", document_id);
        FileAnalysis::default()
    };

    // Pipeline steps analysis
    let mut pipeline_steps = Vec::new();

    // Step 1: File Upload & Ingestion
    pipeline_steps.push(serde_json::json!({
        "step": 1,
        "name": "File Upload & Ingestion",
        "status": "completed", // Document exists if we got this far
        "details": {
            "filename": document.filename,
            "original_filename": document.original_filename,
            "file_size": document.file_size,
            "mime_type": document.mime_type,
            "file_exists": file_exists,
            "file_path": document.file_path,
            "created_at": document.created_at,
            "file_metadata": file_metadata.as_ref().map(|m| serde_json::json!({
                "size": m.len(),
                "modified": m.modified().ok(),
                "is_file": m.is_file(),
                "is_dir": m.is_dir()
            })),
            "file_analysis": file_analysis
        },
        "success": true,
        "error": None::<String>
    }));

    // Step 2: OCR Queue Enrollment
    let queue_enrollment_status = if queue_history.is_empty() {
        if settings.enable_background_ocr {
            "not_queued"
        } else {
            "ocr_disabled"
        }
    } else {
        "queued"
    };

    pipeline_steps.push(serde_json::json!({
        "step": 2,
        "name": "OCR Queue Enrollment",
        "status": queue_enrollment_status,
        "details": {
            "user_ocr_enabled": settings.enable_background_ocr,
            "queue_entries_count": queue_history.len(),
            "queue_history": queue_history.iter().map(|row| serde_json::json!({
                "id": row.get::<uuid::Uuid, _>("id"),
                "status": row.get::<String, _>("status"),
                "priority": row.get::<i32, _>("priority"),
                "created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at"),
                "started_at": row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("started_at"),
                "completed_at": row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("completed_at"),
                "error_message": row.get::<Option<String>, _>("error_message"),
                "attempts": row.get::<i32, _>("attempts"),
                "worker_id": row.get::<Option<String>, _>("worker_id")
            })).collect::<Vec<_>>()
        },
        "success": !queue_history.is_empty() || !settings.enable_background_ocr,
        "error": if !settings.enable_background_ocr && queue_history.is_empty() {
            Some("OCR processing is disabled in user settings")
        } else { None }
    }));

    // Step 3: OCR Processing
    let ocr_status = document.ocr_status.as_deref().unwrap_or("not_started");
    let ocr_success = matches!(ocr_status, "completed");
    
    pipeline_steps.push(serde_json::json!({
        "step": 3,
        "name": "OCR Text Extraction",
        "status": ocr_status,
        "details": {
            "ocr_text_length": document.ocr_text.as_ref().map(|t| t.len()).unwrap_or(0),
            "ocr_confidence": document.ocr_confidence,
            "ocr_word_count": document.ocr_word_count,
            "ocr_processing_time_ms": document.ocr_processing_time_ms,
            "ocr_completed_at": document.ocr_completed_at,
            "ocr_error": document.ocr_error,
            "has_processed_image": processed_image.is_some(),
            "processed_image_info": processed_image.as_ref().map(|pi| serde_json::json!({
                "image_path": pi.processed_image_path,
                "image_width": pi.image_width,
                "image_height": pi.image_height,
                "file_size": pi.file_size,
                "processing_parameters": pi.processing_parameters,
                "processing_steps": pi.processing_steps,
                "created_at": pi.created_at
            }))
        },
        "success": ocr_success,
        "error": document.ocr_error.clone()
    }));

    // Step 4: Quality Validation
    let quality_passed = if let Some(confidence) = document.ocr_confidence {
        confidence >= settings.ocr_min_confidence && document.ocr_word_count.unwrap_or(0) > 0
    } else {
        false
    };

    pipeline_steps.push(serde_json::json!({
        "step": 4,
        "name": "OCR Quality Validation",
        "status": if ocr_success {
            if quality_passed { "passed" } else { "failed" }
        } else {
            "not_reached"
        },
        "details": {
            "quality_thresholds": {
                "min_confidence": settings.ocr_min_confidence,
                "brightness_threshold": settings.ocr_quality_threshold_brightness,
                "contrast_threshold": settings.ocr_quality_threshold_contrast,
                "noise_threshold": settings.ocr_quality_threshold_noise,
                "sharpness_threshold": settings.ocr_quality_threshold_sharpness
            },
            "actual_values": {
                "confidence": document.ocr_confidence,
                "word_count": document.ocr_word_count,
                "processed_image_available": processed_image.is_some(),
                "processing_parameters": processed_image.as_ref().map(|pi| &pi.processing_parameters)
            },
            "quality_checks": {
                "confidence_check": document.ocr_confidence.map(|c| c >= settings.ocr_min_confidence),
                "word_count_check": document.ocr_word_count.map(|w| w > 0),
                "processed_image_available": processed_image.is_some()
            }
        },
        "success": quality_passed,
        "error": if !quality_passed && ocr_success {
            Some(format!("Quality validation failed: confidence {:.1}% (required: {:.1}%), words: {}", 
                document.ocr_confidence.unwrap_or(0.0),
                settings.ocr_min_confidence,
                document.ocr_word_count.unwrap_or(0)
            ))
        } else { None }
    }));

    // Overall summary
    let overall_status = if quality_passed {
        "success"
    } else if matches!(ocr_status, "failed") {
        "failed"
    } else if matches!(ocr_status, "processing") {
        "processing"
    } else if matches!(ocr_status, "pending") {
        "pending"
    } else {
        "not_started"
    };

    Ok(Json(serde_json::json!({
        "document_id": document_id,
        "filename": document.filename,
        "overall_status": overall_status,
        "pipeline_steps": pipeline_steps,
        "failed_document_info": failed_document.as_ref().map(|row| serde_json::json!({
            "failure_reason": row.get::<String, _>("failure_reason"),
            "failure_stage": row.get::<String, _>("failure_stage"),
            "error_message": row.get::<Option<String>, _>("error_message"),
            "retry_count": row.get::<Option<i32>, _>("retry_count"),
            "last_retry_at": row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("last_retry_at"),
            "created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at"),
            "content_preview": row.get::<Option<String>, _>("content").map(|c| 
                c.chars().take(200).collect::<String>()
            ),
            "failed_ocr_text": row.get::<Option<String>, _>("ocr_text"),
            "failed_ocr_confidence": row.get::<Option<f32>, _>("ocr_confidence"),
            "failed_ocr_word_count": row.get::<Option<i32>, _>("ocr_word_count"),
            "failed_ocr_processing_time_ms": row.get::<Option<i32>, _>("ocr_processing_time_ms")
        })),
        "user_settings": {
            "enable_background_ocr": settings.enable_background_ocr,
            "ocr_min_confidence": settings.ocr_min_confidence,
            "max_file_size_mb": settings.max_file_size_mb,
            "quality_thresholds": {
                "brightness": settings.ocr_quality_threshold_brightness,
                "contrast": settings.ocr_quality_threshold_contrast,
                "noise": settings.ocr_quality_threshold_noise,
                "sharpness": settings.ocr_quality_threshold_sharpness
            }
        },
        "debug_timestamp": chrono::Utc::now(),
        "file_analysis": file_analysis,
        "detailed_processing_logs": ocr_processing_logs.iter().map(|row| serde_json::json!({
            "id": row.get::<uuid::Uuid, _>("id"),
            "status": row.get::<String, _>("status"),
            "priority": row.get::<i32, _>("priority"),
            "created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at"),
            "started_at": row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("started_at"),
            "completed_at": row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("completed_at"),
            "error_message": row.get::<Option<String>, _>("error_message"),
            "attempts": row.get::<i32, _>("attempts"),
            "worker_id": row.get::<Option<String>, _>("worker_id"),
            "processing_time_ms": row.get::<Option<i32>, _>("processing_time_ms"),
            "file_size": row.get::<Option<i64>, _>("file_size"),
            // Calculate processing duration if both timestamps are available
            "processing_duration_ms": if let (Some(started), Some(completed)) = (
                row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("started_at"),
                row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("completed_at")
            ) {
                Some((completed.timestamp_millis() - started.timestamp_millis()) as i32)
            } else {
                row.get::<Option<i32>, _>("processing_time_ms")
            },
            // Calculate queue wait time
            "queue_wait_time_ms": if let Some(started) = row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("started_at") {
                let created = row.get::<chrono::DateTime<chrono::Utc>, _>("created_at");
                Some((started.timestamp_millis() - created.timestamp_millis()) as i32)
            } else {
                None::<i32>
            }
        })).collect::<Vec<_>>()
    })))
}

#[derive(Debug, Default, serde::Serialize)]
struct FileAnalysis {
    file_type: String,
    file_size_bytes: u64,
    is_readable: bool,
    pdf_info: Option<PdfAnalysis>,
    text_preview: Option<String>,
    error_details: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct PdfAnalysis {
    is_valid_pdf: bool,
    page_count: Option<i32>,
    has_text_content: bool,
    has_images: bool,
    is_encrypted: bool,
    pdf_version: Option<String>,
    font_count: usize,
    text_extraction_error: Option<String>,
    estimated_text_length: usize,
}

async fn analyze_file_content(file_path: &str, mime_type: &str) -> Result<FileAnalysis, Box<dyn std::error::Error + Send + Sync>> {
    let mut analysis = FileAnalysis {
        file_type: mime_type.to_string(),
        ..Default::default()
    };

    // Try to read file size
    if let Ok(metadata) = tokio::fs::metadata(file_path).await {
        analysis.file_size_bytes = metadata.len();
    }

    // Try to read the file
    let file_content = match tokio::fs::read(file_path).await {
        Ok(content) => {
            analysis.is_readable = true;
            content
        }
        Err(e) => {
            analysis.error_details = Some(format!("Failed to read file: {}", e));
            return Ok(analysis);
        }
    };

    // Analyze based on file type
    if mime_type.contains("pdf") {
        analysis.pdf_info = Some(analyze_pdf_content(&file_content).await);
    } else if mime_type.starts_with("text/") {
        // For text files, show a preview
        match String::from_utf8(file_content.clone()) {
            Ok(text) => {
                analysis.text_preview = Some(text.chars().take(500).collect());
            }
            Err(e) => {
                analysis.error_details = Some(format!("Failed to decode text file: {}", e));
            }
        }
    }

    Ok(analysis)
}

async fn analyze_pdf_content(content: &[u8]) -> PdfAnalysis {
    use std::panic;

    let mut analysis = PdfAnalysis {
        is_valid_pdf: false,
        page_count: None,
        has_text_content: false,
        has_images: false,
        is_encrypted: false,
        pdf_version: None,
        font_count: 0,
        text_extraction_error: None,
        estimated_text_length: 0,
    };

    // Check PDF header
    if content.len() < 8 {
        analysis.text_extraction_error = Some("File too small to be a valid PDF".to_string());
        return analysis;
    }

    if !content.starts_with(b"%PDF-") {
        analysis.text_extraction_error = Some("File does not start with PDF header".to_string());
        return analysis;
    }

    analysis.is_valid_pdf = true;

    // Extract PDF version from header
    if content.len() >= 8 {
        if let Ok(header) = std::str::from_utf8(&content[0..8]) {
            if let Some(version) = header.strip_prefix("%PDF-") {
                analysis.pdf_version = Some(version.to_string());
            }
        }
    }

    // Try to extract text using pdf_extract (same as the main OCR pipeline)
    let text_result = panic::catch_unwind(|| {
        pdf_extract::extract_text_from_mem(content)
    });

    match text_result {
        Ok(Ok(text)) => {
            analysis.has_text_content = !text.trim().is_empty();
            analysis.estimated_text_length = text.len();
            
            // Count words for comparison with OCR results
            let word_count = text.split_whitespace().count();
            if word_count == 0 && text.len() > 0 {
                analysis.text_extraction_error = Some("PDF contains characters but no extractable words".to_string());
            }
        }
        Ok(Err(e)) => {
            analysis.text_extraction_error = Some(format!("PDF text extraction failed: {}", e));
        }
        Err(_) => {
            analysis.text_extraction_error = Some("PDF text extraction panicked (likely corrupted PDF)".to_string());
        }
    }

    // Basic PDF structure analysis
    let content_str = String::from_utf8_lossy(content);
    
    // Check for encryption
    analysis.is_encrypted = content_str.contains("/Encrypt");
    
    // Check for images
    analysis.has_images = content_str.contains("/Image") || content_str.contains("/XObject");
    
    // Estimate page count (rough)
    let page_matches = content_str.matches("/Type /Page").count();
    if page_matches > 0 {
        analysis.page_count = Some(page_matches as i32);
    }

    // Count fonts (rough)
    analysis.font_count = content_str.matches("/Type /Font").count();

    analysis
}

#[utoipa::path(
    get,
    path = "/api/documents/failed-ocr",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("limit" = Option<i64>, Query, description = "Number of documents to return (default: 50)"),
        ("offset" = Option<i64>, Query, description = "Number of documents to skip (default: 0)")
    ),
    responses(
        (status = 200, description = "List of documents with failed OCR", body = String),
        (status = 401, description = "Unauthorized")
    )
)]
async fn get_failed_ocr_documents(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let limit = pagination.limit.unwrap_or(50);
    let offset = pagination.offset.unwrap_or(0);
    
    // Get failed OCR documents with additional failure details
    let failed_docs = sqlx::query(
        r#"
        SELECT d.id, d.filename, d.original_filename, d.file_path, d.file_size, 
               d.mime_type, d.created_at, d.updated_at, d.user_id,
               d.ocr_status, d.ocr_error, d.ocr_failure_reason,
               d.ocr_completed_at, d.tags,
               -- Count retry attempts from OCR queue
               COALESCE(q.retry_count, 0) as retry_count,
               q.last_attempt_at
        FROM documents d
        LEFT JOIN (
            SELECT document_id, 
                   COUNT(*) as retry_count,
                   MAX(created_at) as last_attempt_at
            FROM ocr_queue 
            WHERE status IN ('failed', 'completed')
            GROUP BY document_id
        ) q ON d.id = q.document_id
        WHERE d.ocr_status = 'failed'
          AND ($1::uuid IS NULL OR d.user_id = $1)  -- Admin can see all, users see only their own
        ORDER BY d.updated_at DESC
        LIMIT $2 OFFSET $3
        "#
    )
    .bind(if auth_user.user.role == crate::models::UserRole::Admin { 
        None 
    } else { 
        Some(auth_user.user.id) 
    })
    .bind(limit)
    .bind(offset)
    .fetch_all(state.db.get_pool())
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Count total failed documents
    let total_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM documents 
        WHERE ocr_status = 'failed'
          AND ($1::uuid IS NULL OR user_id = $1)
        "#
    )
    .bind(if auth_user.user.role == crate::models::UserRole::Admin { 
        None 
    } else { 
        Some(auth_user.user.id) 
    })
    .fetch_one(state.db.get_pool())
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let failed_documents: Vec<serde_json::Value> = failed_docs
        .into_iter()
        .map(|row| {
            let tags: Vec<String> = row.get::<Option<Vec<String>>, _>("tags").unwrap_or_default();
            
            serde_json::json!({
                "id": row.get::<uuid::Uuid, _>("id"),
                "filename": row.get::<String, _>("filename"),
                "original_filename": row.get::<String, _>("original_filename"),
                "file_size": row.get::<i64, _>("file_size"),
                "mime_type": row.get::<String, _>("mime_type"),
                "created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at"),
                "updated_at": row.get::<chrono::DateTime<chrono::Utc>, _>("updated_at"),
                "tags": tags,
                "ocr_status": row.get::<Option<String>, _>("ocr_status"),
                "ocr_error": row.get::<Option<String>, _>("ocr_error"),
                "ocr_failure_reason": row.get::<Option<String>, _>("ocr_failure_reason"),
                "ocr_completed_at": row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("ocr_completed_at"),
                "retry_count": row.get::<Option<i64>, _>("retry_count").unwrap_or(0),
                "last_attempt_at": row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("last_attempt_at"),
                "can_retry": true,
                "failure_category": categorize_failure_reason(
                    row.get::<Option<String>, _>("ocr_failure_reason").as_deref(),
                    row.get::<Option<String>, _>("ocr_error").as_deref()
                )
            })
        })
        .collect();
    
    let response = serde_json::json!({
        "documents": failed_documents,
        "pagination": {
            "total": total_count,
            "limit": limit,
            "offset": offset,
            "has_more": offset + limit < total_count
        },
        "statistics": {
            "total_failed": total_count,
            "failure_categories": get_failure_statistics(&state, auth_user.user.id, auth_user.user.role.clone()).await?
        }
    });
    
    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/api/documents/failed",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("limit" = Option<i64>, Query, description = "Number of documents to return"),
        ("offset" = Option<i64>, Query, description = "Number of documents to skip"),
        ("stage" = Option<String>, Query, description = "Filter by failure stage (ocr, ingestion, validation, etc.)"),
        ("reason" = Option<String>, Query, description = "Filter by failure reason")
    ),
    responses(
        (status = 200, description = "List of failed documents", body = String),
        (status = 401, description = "Unauthorized")
    )
)]
async fn get_failed_documents(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Query(params): Query<FailedDocumentsQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let limit = params.limit.unwrap_or(25);
    let offset = params.offset.unwrap_or(0);
    
    // Query the unified failed_documents table
    let mut query_builder = sqlx::QueryBuilder::new(
        r#"
        SELECT id, filename, original_filename, file_path, file_size, mime_type,
               content, tags, ocr_text, ocr_confidence, ocr_word_count, ocr_processing_time_ms,
               failure_reason, failure_stage, error_message, existing_document_id,
               ingestion_source, retry_count, last_retry_at, created_at, updated_at
        FROM failed_documents
        WHERE ($1::uuid IS NULL OR user_id = $1)
        "#
    );
    
    let mut bind_count = 1;
    
    // Add stage filter if specified
    if let Some(stage) = &params.stage {
        bind_count += 1;
        query_builder.push(&format!(" AND failure_stage = ${}", bind_count));
    }
    
    // Add reason filter if specified  
    if let Some(reason) = &params.reason {
        bind_count += 1;
        query_builder.push(&format!(" AND failure_reason = ${}", bind_count));
    }
    
    query_builder.push(" ORDER BY created_at DESC");
    query_builder.push(&format!(" LIMIT ${} OFFSET ${}", bind_count + 1, bind_count + 2));
    
    let mut query = query_builder.build();
    
    // Bind parameters in order
    query = query.bind(if auth_user.user.role == crate::models::UserRole::Admin { 
        None 
    } else { 
        Some(auth_user.user.id) 
    });
    
    if let Some(stage) = &params.stage {
        query = query.bind(stage);
    }
    
    if let Some(reason) = &params.reason {
        query = query.bind(reason);
    }
    
    query = query.bind(limit).bind(offset);
    
    let failed_docs = query
        .fetch_all(state.db.get_pool())
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch failed documents: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // Count total for pagination
    let mut count_query_builder = sqlx::QueryBuilder::new(
        "SELECT COUNT(*) FROM failed_documents WHERE ($1::uuid IS NULL OR user_id = $1)"
    );
    
    let mut count_bind_count = 1;
    
    if let Some(stage) = &params.stage {
        count_bind_count += 1;
        count_query_builder.push(&format!(" AND failure_stage = ${}", count_bind_count));
    }
    
    if let Some(reason) = &params.reason {
        count_bind_count += 1;
        count_query_builder.push(&format!(" AND failure_reason = ${}", count_bind_count));
    }
    
    let mut count_query = count_query_builder.build_query_scalar::<i64>();
    
    count_query = count_query.bind(if auth_user.user.role == crate::models::UserRole::Admin { 
        None 
    } else { 
        Some(auth_user.user.id) 
    });
    
    if let Some(stage) = &params.stage {
        count_query = count_query.bind(stage);
    }
    
    if let Some(reason) = &params.reason {
        count_query = count_query.bind(reason);
    }
    
    let total_count = count_query
        .fetch_one(state.db.get_pool())
        .await
        .unwrap_or(0);
    
    // Convert to JSON response format
    let documents: Vec<serde_json::Value> = failed_docs.iter().map(|row| {
        serde_json::json!({
            "id": row.get::<uuid::Uuid, _>("id"),
            "filename": row.get::<String, _>("filename"),
            "original_filename": row.get::<Option<String>, _>("original_filename"),
            "file_path": row.get::<Option<String>, _>("file_path"),
            "file_size": row.get::<Option<i64>, _>("file_size"),
            "mime_type": row.get::<Option<String>, _>("mime_type"),
            "content": row.get::<Option<String>, _>("content"),
            "tags": row.get::<Vec<String>, _>("tags"),
            "ocr_text": row.get::<Option<String>, _>("ocr_text"),
            "ocr_confidence": row.get::<Option<f32>, _>("ocr_confidence"),
            "ocr_word_count": row.get::<Option<i32>, _>("ocr_word_count"),
            "ocr_processing_time_ms": row.get::<Option<i32>, _>("ocr_processing_time_ms"),
            "failure_reason": row.get::<String, _>("failure_reason"),
            "failure_stage": row.get::<String, _>("failure_stage"),
            "error_message": row.get::<Option<String>, _>("error_message"),
            "existing_document_id": row.get::<Option<uuid::Uuid>, _>("existing_document_id"),
            "ingestion_source": row.get::<String, _>("ingestion_source"),
            "retry_count": row.get::<Option<i32>, _>("retry_count"),
            "last_retry_at": row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("last_retry_at"),
            "created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at"),
            "updated_at": row.get::<chrono::DateTime<chrono::Utc>, _>("updated_at"),
            
            // Computed fields for backward compatibility
            "failure_category": categorize_failure_reason(
                Some(&row.get::<String, _>("failure_reason")),
                row.get::<Option<String>, _>("error_message").as_deref()
            ),
            "source": match row.get::<String, _>("failure_stage").as_str() {
                "ocr" => "OCR Processing",
                "ingestion" => "Document Ingestion", 
                "validation" => "Document Validation",
                "storage" => "File Storage",
                "processing" => "Document Processing",
                "sync" => "Source Synchronization",
                _ => "Unknown"
            }
        })
    }).collect();
    
    // Calculate statistics for the response
    let mut stage_stats = std::collections::HashMap::new();
    let mut reason_stats = std::collections::HashMap::new();
    
    for doc in &documents {
        let stage = doc["failure_stage"].as_str().unwrap_or("unknown");
        let reason = doc["failure_reason"].as_str().unwrap_or("unknown");
        
        *stage_stats.entry(stage).or_insert(0) += 1;
        *reason_stats.entry(reason).or_insert(0) += 1;
    }
    
    let response = serde_json::json!({
        "documents": documents,
        "pagination": {
            "limit": limit,
            "offset": offset,
            "total": total_count,
            "total_pages": (total_count as f64 / limit as f64).ceil() as i64
        },
        "statistics": {
            "total_failed": total_count,
            "by_stage": stage_stats,
            "by_reason": reason_stats
        },
        "filters": {
            "stage": params.stage,
            "reason": params.reason
        }
    });
    
    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/api/documents/failed/{id}/view",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = uuid::Uuid, Path, description = "Failed Document ID")
    ),
    responses(
        (status = 200, description = "Failed document content for viewing in browser"),
        (status = 404, description = "Failed document not found or file deleted"),
        (status = 401, description = "Unauthorized")
    )
)]
async fn view_failed_document(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(failed_document_id): Path<uuid::Uuid>,
) -> Result<Response, StatusCode> {
    // Get failed document from database
    let row = sqlx::query(
        r#"
        SELECT file_path, filename, mime_type, user_id
        FROM failed_documents 
        WHERE id = $1 AND ($2::uuid IS NULL OR user_id = $2)
        "#
    )
    .bind(failed_document_id)
    .bind(if auth_user.user.role == crate::models::UserRole::Admin { 
        None 
    } else { 
        Some(auth_user.user.id) 
    })
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;
    
    let file_path: Option<String> = row.get("file_path");
    let filename: String = row.get("filename");
    let mime_type: Option<String> = row.get("mime_type");
    
    // Check if file_path exists (some failed documents might not have been saved)
    let file_path = file_path.ok_or(StatusCode::NOT_FOUND)?;
    
    let file_service = FileService::new(state.config.upload_path.clone());
    let file_data = file_service
        .read_file(&file_path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?; // File was deleted or moved
    
    // Determine content type from mime_type or file extension
    let content_type = mime_type
        .unwrap_or_else(|| {
            mime_guess::from_path(&filename)
                .first_or_octet_stream()
                .to_string()
        });
    
    let response = Response::builder()
        .header(CONTENT_TYPE, content_type)
        .header("Content-Length", file_data.len())
        .header("Content-Disposition", format!("inline; filename=\"{}\"", filename))
        .body(file_data.into())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(response)
}

async fn calculate_estimated_wait_time(priority: i32) -> i64 {
    // Simple estimation based on priority - in a real implementation,
    // this would check actual queue depth and processing times
    match priority {
        15.. => 1,      // High priority retry: ~1 minute
        10..14 => 3,    // Medium priority: ~3 minutes  
        5..9 => 10,     // Low priority: ~10 minutes
        _ => 30,        // Very low priority: ~30 minutes
    }
}

fn categorize_failure_reason(failure_reason: Option<&str>, error_message: Option<&str>) -> &'static str {
    match failure_reason {
        Some("pdf_font_encoding") => "PDF Font Issues",
        Some("pdf_corruption") => "PDF Corruption", 
        Some("processing_timeout") => "Timeout",
        Some("memory_limit") => "Memory Limit",
        Some("pdf_parsing_panic") => "PDF Parsing Error",
        Some("low_ocr_confidence") => "Low OCR Confidence",
        Some("unknown") | None => {
            // Try to categorize based on error message
            if let Some(error) = error_message {
                let error_lower = error.to_lowercase();
                if error_lower.contains("timeout") {
                    "Timeout"
                } else if error_lower.contains("memory") {
                    "Memory Limit" 
                } else if error_lower.contains("font") || error_lower.contains("encoding") {
                    "PDF Font Issues"
                } else if error_lower.contains("corrupt") {
                    "PDF Corruption"
                } else if error_lower.contains("quality below threshold") || error_lower.contains("confidence") {
                    "Low OCR Confidence"
                } else {
                    "Unknown Error"
                }
            } else {
                "Unknown Error"
            }
        }
        _ => "Other"
    }
}

async fn get_failure_statistics(
    state: &Arc<AppState>, 
    user_id: uuid::Uuid, 
    user_role: crate::models::UserRole
) -> Result<serde_json::Value, StatusCode> {
    let stats = sqlx::query(
        r#"
        SELECT 
            ocr_failure_reason,
            COUNT(*) as count
        FROM documents 
        WHERE ocr_status = 'failed'
          AND ($1::uuid IS NULL OR user_id = $1)
        GROUP BY ocr_failure_reason
        ORDER BY count DESC
        "#
    )
    .bind(if user_role == crate::models::UserRole::Admin { 
        None 
    } else { 
        Some(user_id) 
    })
    .fetch_all(state.db.get_pool())
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let categories: Vec<serde_json::Value> = stats
        .into_iter()
        .map(|row| {
            let reason = row.get::<Option<String>, _>("ocr_failure_reason");
            let count = row.get::<i64, _>("count");
            
            serde_json::json!({
                "reason": reason.clone().unwrap_or_else(|| "unknown".to_string()),
                "display_name": categorize_failure_reason(reason.as_deref(), None),
                "count": count
            })
        })
        .collect();
    
    Ok(serde_json::json!(categories))
}

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
        (status = 200, description = "User's duplicate documents grouped by hash", body = String),
        (status = 401, description = "Unauthorized")
    )
)]
async fn get_user_duplicates(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let limit = query.limit.unwrap_or(25);
    let offset = query.offset.unwrap_or(0);

    let (duplicates, total_count) = state
        .db
        .get_user_duplicates(auth_user.user.id, auth_user.user.role, limit, offset)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
        (status = 200, description = "Document deleted successfully", body = String),
        (status = 404, description = "Document not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn delete_document(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(document_id): Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let deleted_document = state
        .db
        .delete_document(document_id, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Create ignored file record for future source sync prevention
    if let Err(e) = crate::db::ignored_files::create_ignored_file_from_document(
        state.db.get_pool(),
        document_id,
        auth_user.user.id,
        Some("deleted by user".to_string()),
        None, // source_type will be determined by sync processes
        None, // source_path will be determined by sync processes
        None, // source_identifier will be determined by sync processes
    ).await {
        tracing::warn!("Failed to create ignored file record for document {}: {}", document_id, e);
    }

    let file_service = FileService::new(state.config.upload_path.clone());
    
    if let Err(e) = file_service.delete_document_files(&deleted_document).await {
        tracing::warn!("Failed to delete some files for document {}: {}", document_id, e);
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Document deleted successfully",
        "document_id": document_id,
        "filename": deleted_document.filename
    })))
}

#[utoipa::path(
    delete,
    path = "/api/documents",
    tag = "documents",
    security(
        ("bearer_auth" = [])
    ),
    request_body(content = BulkDeleteRequest, description = "List of document IDs to delete"),
    responses(
        (status = 200, description = "Documents deleted successfully", body = String),
        (status = 400, description = "Bad request - no document IDs provided"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn bulk_delete_documents(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Json(request): Json<BulkDeleteRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if request.document_ids.is_empty() {
        return Ok(Json(serde_json::json!({
            "success": false,
            "message": "No document IDs provided",
            "deleted_count": 0
        })));
    }

    let deleted_documents = state
        .db
        .bulk_delete_documents(&request.document_ids, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create ignored file records for all successfully deleted documents
    let mut ignored_file_creation_failures = 0;
    for document in &deleted_documents {
        if let Err(e) = crate::db::ignored_files::create_ignored_file_from_document(
            state.db.get_pool(),
            document.id,
            auth_user.user.id,
            Some("bulk deleted by user".to_string()),
            None, // source_type will be determined by sync processes
            None, // source_path will be determined by sync processes
            None, // source_identifier will be determined by sync processes
        ).await {
            ignored_file_creation_failures += 1;
            tracing::warn!("Failed to create ignored file record for document {}: {}", document.id, e);
        }
    }

    let file_service = FileService::new(state.config.upload_path.clone());
    let mut successful_file_deletions = 0;
    let mut failed_file_deletions = 0;

    for document in &deleted_documents {
        match file_service.delete_document_files(document).await {
            Ok(_) => successful_file_deletions += 1,
            Err(e) => {
                failed_file_deletions += 1;
                tracing::warn!("Failed to delete files for document {}: {}", document.id, e);
            }
        }
    }

    let deleted_count = deleted_documents.len();
    let requested_count = request.document_ids.len();

    let message = if deleted_count == requested_count {
        format!("Successfully deleted {} documents", deleted_count)
    } else {
        format!("Deleted {} of {} requested documents (some may not exist or belong to other users)", deleted_count, requested_count)
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "message": message,
        "deleted_count": deleted_count,
        "requested_count": requested_count,
        "successful_file_deletions": successful_file_deletions,
        "failed_file_deletions": failed_file_deletions,
        "ignored_file_creation_failures": ignored_file_creation_failures,
        "deleted_document_ids": deleted_documents.iter().map(|d| d.id).collect::<Vec<_>>()
    })))
}

#[utoipa::path(
    post,
    path = "/api/documents/delete-low-confidence",
    request_body = DeleteLowConfidenceRequest,
    responses(
        (status = 200, description = "Low confidence documents operation result"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "documents"
)]
pub async fn delete_low_confidence_documents(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Json(request): Json<DeleteLowConfidenceRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if request.max_confidence < 0.0 || request.max_confidence > 100.0 {
        return Ok(Json(serde_json::json!({
            "success": false,
            "message": "max_confidence must be between 0.0 and 100.0",
            "matched_count": 0
        })));
    }

    let is_preview = request.preview_only.unwrap_or(false);
    
    // Find documents with confidence below threshold OR failed OCR
    let matched_documents = state
        .db
        .find_low_confidence_and_failed_documents(request.max_confidence, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let matched_count = matched_documents.len();

    if is_preview {
        // Convert documents to response format with key details
        let document_details: Vec<serde_json::Value> = matched_documents.iter().map(|d| {
            serde_json::json!({
                "id": d.id,
                "filename": d.filename,
                "original_filename": d.original_filename,
                "file_size": d.file_size,
                "ocr_confidence": d.ocr_confidence,
                "ocr_status": d.ocr_status,
                "created_at": d.created_at,
                "mime_type": d.mime_type
            })
        }).collect();

        return Ok(Json(serde_json::json!({
            "success": true,
            "message": format!("Found {} documents with OCR confidence below {}%", matched_count, request.max_confidence),
            "matched_count": matched_count,
            "preview": true,
            "document_ids": matched_documents.iter().map(|d| d.id).collect::<Vec<_>>(),
            "documents": document_details
        })));
    }

    if matched_documents.is_empty() {
        return Ok(Json(serde_json::json!({
            "success": true,
            "message": format!("No documents found with OCR confidence below {}%", request.max_confidence),
            "deleted_count": 0
        })));
    }

    // Extract document IDs for bulk deletion
    let document_ids: Vec<uuid::Uuid> = matched_documents.iter().map(|d| d.id).collect();

    // Use existing bulk delete logic
    let deleted_documents = state
        .db
        .bulk_delete_documents(&document_ids, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create ignored file records for all successfully deleted documents
    let mut ignored_file_creation_failures = 0;
    for document in &deleted_documents {
        if let Err(e) = crate::db::ignored_files::create_ignored_file_from_document(
            state.db.get_pool(),
            document.id,
            auth_user.user.id,
            Some(format!("deleted due to low OCR confidence ({}%)", 
                document.ocr_confidence.unwrap_or(0.0))),
            None,
            None,
            None,
        ).await {
            ignored_file_creation_failures += 1;
            tracing::warn!("Failed to create ignored file record for document {}: {}", document.id, e);
        }
    }

    let file_service = FileService::new(state.config.upload_path.clone());
    let mut successful_file_deletions = 0;
    let mut failed_file_deletions = 0;

    for document in &deleted_documents {
        match file_service.delete_document_files(document).await {
            Ok(_) => successful_file_deletions += 1,
            Err(e) => {
                failed_file_deletions += 1;
                tracing::warn!("Failed to delete files for document {}: {}", document.id, e);
            }
        }
    }

    let deleted_count = deleted_documents.len();

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Successfully deleted {} documents with OCR confidence below {}%", deleted_count, request.max_confidence),
        "deleted_count": deleted_count,
        "matched_count": matched_count,
        "successful_file_deletions": successful_file_deletions,
        "failed_file_deletions": failed_file_deletions,
        "ignored_file_creation_failures": ignored_file_creation_failures,
        "deleted_document_ids": deleted_documents.iter().map(|d| d.id).collect::<Vec<_>>()
    })))
}

/// Delete all documents with failed OCR processing
pub async fn delete_failed_ocr_documents(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let is_preview = request.get("preview_only").and_then(|v| v.as_bool()).unwrap_or(false);
    
    // Find documents with failed OCR
    let matched_documents = state
        .db
        .find_failed_ocr_documents(auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let matched_count = matched_documents.len();

    if is_preview {
        return Ok(Json(serde_json::json!({
            "success": true,
            "message": format!("Found {} documents with failed OCR processing", matched_count),
            "matched_count": matched_count,
            "preview": true,
            "document_ids": matched_documents.iter().map(|d| d.id).collect::<Vec<_>>()
        })));
    }

    if matched_documents.is_empty() {
        return Ok(Json(serde_json::json!({
            "success": true,
            "message": "No documents found with failed OCR processing",
            "deleted_count": 0
        })));
    }

    // Extract document IDs for bulk deletion
    let document_ids: Vec<uuid::Uuid> = matched_documents.iter().map(|d| d.id).collect();

    // Use existing bulk delete logic
    let deleted_documents = state
        .db
        .bulk_delete_documents(&document_ids, auth_user.user.id, auth_user.user.role)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create ignored file records for all successfully deleted documents
    let mut ignored_file_creation_failures = 0;
    for document in &deleted_documents {
        let reason = if let Some(ref error) = document.ocr_error {
            format!("deleted due to failed OCR processing: {}", error)
        } else {
            "deleted due to failed OCR processing".to_string()
        };
        
        if let Err(e) = crate::db::ignored_files::create_ignored_file_from_document(
            state.db.get_pool(),
            document.id,
            auth_user.user.id,
            Some(reason),
            None,
            None,
            None,
        ).await {
            ignored_file_creation_failures += 1;
            tracing::warn!("Failed to create ignored file record for document {}: {}", document.id, e);
        }
    }

    let file_service = FileService::new(state.config.upload_path.clone());
    let mut successful_file_deletions = 0;
    let mut failed_file_deletions = 0;

    for document in &deleted_documents {
        match file_service.delete_document_files(document).await {
            Ok(_) => successful_file_deletions += 1,
            Err(e) => {
                failed_file_deletions += 1;
                tracing::warn!("Failed to delete files for document {}: {}", document.id, e);
            }
        }
    }

    let deleted_count = deleted_documents.len();

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Successfully deleted {} documents with failed OCR processing", deleted_count),
        "deleted_count": deleted_count,
        "matched_count": matched_count,
        "successful_file_deletions": successful_file_deletions,
        "failed_file_deletions": failed_file_deletions,
        "ignored_file_creation_failures": ignored_file_creation_failures,
        "deleted_document_ids": deleted_documents.iter().map(|d| d.id).collect::<Vec<_>>()
    })))
}