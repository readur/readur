use axum::{
    extract::{Multipart, Path, Query, State},
    http::{StatusCode, header::CONTENT_TYPE},
    response::{Json, Response},
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use std::sync::Arc;
use utoipa::ToSchema;
use sha2::{Sha256, Digest};

use crate::{
    auth::AuthUser,
    file_service::FileService,
    models::DocumentResponse,
    ocr_queue::OcrQueueService,
    AppState,
};

#[derive(Deserialize, ToSchema)]
struct PaginationQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(upload_document))
        .route("/", get(list_documents))
        .route("/{id}/download", get(download_document))
        .route("/{id}/view", get(view_document))
        .route("/{id}/thumbnail", get(get_document_thumbnail))
        .route("/{id}/ocr", get(get_document_ocr))
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
    
    // Get user settings for file upload restrictions
    let settings = state
        .db
        .get_user_settings(auth_user.user.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .unwrap_or_else(|| crate::models::Settings::default());
    
    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        let name = field.name().unwrap_or("").to_string();
        
        if name == "file" {
            let filename = field
                .file_name()
                .ok_or(StatusCode::BAD_REQUEST)?
                .to_string();
            
            if !file_service.is_allowed_file_type(&filename, &settings.allowed_file_types) {
                return Err(StatusCode::BAD_REQUEST);
            }
            
            let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
            let file_size = data.len() as i64;
            
            // Check file size limit
            let max_size_bytes = (settings.max_file_size_mb as i64) * 1024 * 1024;
            if file_size > max_size_bytes {
                return Err(StatusCode::PAYLOAD_TOO_LARGE);
            }
            
            // Calculate file hash for deduplication
            let file_hash = calculate_file_hash(&data);
            
            // Check if this exact file content already exists in the system
            // This prevents uploading and processing duplicate files
            if let Ok(existing_docs) = state.db.get_documents_by_user_with_role(auth_user.user.id, auth_user.user.role, 1000, 0).await {
                for existing_doc in existing_docs {
                    // Quick size check first (much faster than hash comparison)
                    if existing_doc.file_size == file_size {
                        // Read the existing file and compare hashes
                        if let Ok(existing_file_data) = tokio::fs::read(&existing_doc.file_path).await {
                            let existing_hash = calculate_file_hash(&existing_file_data);
                            if file_hash == existing_hash {
                                // Return the existing document instead of creating a duplicate
                                return Ok(Json(existing_doc.into()));
                            }
                        }
                    }
                }
            }
            
            let mime_type = mime_guess::from_path(&filename)
                .first_or_octet_stream()
                .to_string();
            
            let file_path = file_service
                .save_file(&filename, &data)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            
            let document = file_service.create_document(
                &filename,
                &filename,
                &file_path,
                file_size,
                &mime_type,
                auth_user.user.id,
            );
            
            let saved_document = state
                .db
                .create_document(document)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            
            let document_id = saved_document.id;
            let enable_background_ocr = settings.enable_background_ocr;
            
            if enable_background_ocr {
                let queue_service = OcrQueueService::new(state.db.clone(), state.db.pool.clone(), 1);
                
                // Calculate priority based on file size
                let priority = match file_size {
                    0..=1048576 => 10,          // <= 1MB: highest priority
                    ..=5242880 => 8,            // 1-5MB: high priority
                    ..=10485760 => 6,           // 5-10MB: medium priority  
                    ..=52428800 => 4,           // 10-50MB: low priority
                    _ => 2,                     // > 50MB: lowest priority
                };
                
                queue_service.enqueue_document(document_id, priority, file_size).await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            }
            
            return Ok(Json(saved_document.into()));
        }
    }
    
    Err(StatusCode::BAD_REQUEST)
}

fn calculate_file_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    format!("{:x}", result)
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
        ("offset" = Option<i64>, Query, description = "Number of documents to skip (default: 0)")
    ),
    responses(
        (status = 200, description = "List of user documents", body = Vec<DocumentResponse>),
        (status = 401, description = "Unauthorized")
    )
)]
async fn list_documents(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<Vec<DocumentResponse>>, StatusCode> {
    let limit = pagination.limit.unwrap_or(50);
    let offset = pagination.offset.unwrap_or(0);
    
    let documents = state
        .db
        .get_documents_by_user_with_role(auth_user.user.id, auth_user.user.role, limit, offset)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let response: Vec<DocumentResponse> = documents.into_iter().map(|doc| doc.into()).collect();
    
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
    let documents = state
        .db
        .get_documents_by_user_with_role(auth_user.user.id, auth_user.user.role, 1000, 0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let document = documents
        .into_iter()
        .find(|doc| doc.id == document_id)
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
    let documents = state
        .db
        .get_documents_by_user_with_role(auth_user.user.id, auth_user.user.role, 1000, 0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let document = documents
        .into_iter()
        .find(|doc| doc.id == document_id)
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
    let documents = state
        .db
        .get_documents_by_user_with_role(auth_user.user.id, auth_user.user.role, 1000, 0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let document = documents
        .into_iter()
        .find(|doc| doc.id == document_id)
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
        Err(_) => {
            // Return a placeholder thumbnail or 404
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
    let documents = state
        .db
        .get_documents_by_user_with_role(auth_user.user.id, auth_user.user.role, 1000, 0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let document = documents
        .into_iter()
        .find(|doc| doc.id == document_id)
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