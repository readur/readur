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

#[derive(Deserialize, Serialize, ToSchema)]
pub struct BulkDeleteRequest {
    pub document_ids: Vec<uuid::Uuid>,
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
        .route("/failed-ocr", get(get_failed_ocr_documents))
        .route("/duplicates", get(get_user_duplicates))
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