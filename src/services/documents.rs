//! Document operations shared by the REST handlers (`routes::documents`) and
//! the MCP tools (`mcp::tools::documents`), so both enforce the same access
//! rules and run the same upload pipeline.
//!
//! Lookups return `Ok(None)` when the caller cannot see the document, so a
//! foreign document is indistinguishable from a missing one.

use std::collections::HashMap;

use anyhow::Result;
use chrono::Utc;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{
    ingestion::document_ingestion::{
        DeduplicationPolicy, DocumentIngestionService, IngestionResult,
    },
    models::{
        DocumentFilters, DocumentOcrResponse, DocumentResponse, DocumentSortField,
        FileIngestionInfo, SortOrder, User,
    },
    routes::documents::{
        DocumentError, DocumentPaginationInfo, DocumentUploadResponse, PaginatedDocumentsResponse,
    },
    AppState,
};

/// Priority for documents queued by a user action (upload, manual retry).
const USER_OCR_PRIORITY: i32 = 5;

/// One page of the caller's documents, with labels and OCR progress.
/// Callers validate `filters` first.
pub async fn list_documents(
    state: &AppState,
    user: &User,
    filters: &DocumentFilters,
    sort_by: Option<DocumentSortField>,
    sort_order: Option<SortOrder>,
    limit: i64,
    offset: i64,
) -> Result<PaginatedDocumentsResponse> {
    let total = state
        .db
        .count_documents_filtered(user.id, user.role, filters)
        .await?;
    let documents = state
        .db
        .list_documents_filtered(
            user.id, user.role, filters, sort_by, sort_order, limit, offset,
        )
        .await?;

    let document_ids: Vec<Uuid> = documents.iter().map(|d| d.document.id).collect();
    let mut labels: HashMap<_, _> = state
        .db
        .get_labels_for_documents(&document_ids)
        .await?
        .into_iter()
        .collect();

    let documents = documents
        .into_iter()
        .map(|item| {
            let mut response = DocumentResponse::from(item.document);
            if let Some(labels) = labels.remove(&response.id) {
                response.labels = labels;
            }
            response.ocr_progress_current = item.ocr_progress_current;
            response.ocr_progress_total = item.ocr_progress_total;
            response
        })
        .collect();

    Ok(PaginatedDocumentsResponse {
        documents,
        pagination: DocumentPaginationInfo {
            total,
            limit,
            offset,
            has_more: offset + limit < total,
        },
    })
}

/// Full metadata for one document: labels, owner username and live OCR
/// progress while it is being processed.
pub async fn get_document(
    state: &AppState,
    user: &User,
    document_id: Uuid,
) -> Result<Option<DocumentResponse>> {
    let Some(document) = state
        .db
        .get_document_by_id(document_id, user.id, user.role)
        .await?
    else {
        return Ok(None);
    };

    let labels = state.db.get_document_labels(document_id).await?;
    let username = state
        .db
        .get_user_by_id(document.user_id)
        .await?
        .map(|owner| owner.username);

    let mut response = DocumentResponse::from(document);
    response.labels = labels;
    response.username = username;
    if response.ocr_status.as_deref() == Some("processing") {
        if let Some((current, total)) = ocr_progress(state, document_id).await {
            response.ocr_progress_current = current;
            response.ocr_progress_total = total;
        }
    }
    Ok(Some(response))
}

/// The document's OCR text and status.
pub async fn get_document_ocr(
    state: &AppState,
    user: &User,
    document_id: Uuid,
) -> Result<Option<DocumentOcrResponse>> {
    let Some(document) = state
        .db
        .get_document_by_id(document_id, user.id, user.role)
        .await?
    else {
        return Ok(None);
    };

    let pages_processed = if document.ocr_status.as_deref() == Some("processing") {
        ocr_progress(state, document_id)
            .await
            .and_then(|(current, _)| current)
    } else {
        None
    };

    Ok(Some(DocumentOcrResponse {
        id: document.id,
        filename: document.original_filename,
        has_ocr_text: document.ocr_text.is_some(),
        ocr_text: document.ocr_text,
        ocr_confidence: document.ocr_confidence,
        ocr_status: document.ocr_status,
        ocr_processing_time_ms: document.ocr_processing_time_ms,
        detected_language: None,
        pages_processed,
    }))
}

/// `(progress_current, progress_total)` of an in-flight OCR job. Best effort:
/// a failed lookup only hides the progress bar.
async fn ocr_progress(state: &AppState, document_id: Uuid) -> Option<(Option<i32>, Option<i32>)> {
    sqlx::query_as::<_, (Option<i32>, Option<i32>)>(
        "SELECT progress_current, progress_total FROM ocr_queue WHERE document_id = $1 AND status = 'processing' LIMIT 1",
    )
    .bind(document_id)
    .fetch_optional(&state.db.pool)
    .await
    .ok()
    .flatten()
}

/// Deletes a document and its stored files. Returns `false` when the caller
/// has no such document.
pub async fn delete_document(state: &AppState, user: &User, document_id: Uuid) -> Result<bool> {
    let Some(document) = state
        .db
        .get_document_by_id(document_id, user.id, user.role)
        .await?
    else {
        return Ok(false);
    };

    if !state
        .db
        .delete_document(document_id, user.id, user.role)
        .await?
    {
        return Ok(false);
    }

    if let Err(e) = state.file_service.delete_document_files(&document).await {
        // The database row is gone; orphaned files are only wasted space.
        warn!("Failed to delete files for document {}: {}", document_id, e);
    }
    info!("Document deleted successfully: {}", document_id);
    Ok(true)
}

/// A file to ingest on behalf of a user.
#[derive(Debug, Default)]
pub struct UploadRequest {
    pub filename: String,
    pub mime_type: String,
    pub data: Vec<u8>,
    /// Already-validated OCR languages to store as the user's preference.
    pub ocr_languages: Vec<String>,
    /// Legacy single-language preference, used when `ocr_languages` is empty.
    pub ocr_language: Option<String>,
    /// Labels (owned by the uploader) to attach to the document.
    pub label_ids: Vec<Uuid>,
}

/// Validates, stores and ingests an upload, attaches its labels and queues
/// it for OCR.
pub async fn upload_document(
    state: &AppState,
    user: &User,
    request: UploadRequest,
) -> Result<DocumentUploadResponse, DocumentError> {
    let UploadRequest {
        filename,
        mime_type,
        data,
        ocr_languages,
        ocr_language,
        label_ids,
    } = request;

    check_upload_size(state, &filename, data.len())?;

    let owns_labels = state
        .db
        .user_owns_labels(&label_ids, user.id)
        .await
        .map_err(|e| {
            error!("Failed to verify upload labels: {}", e);
            DocumentError::InternalServerError("Failed to verify labels".to_string())
        })?;
    if !owns_labels {
        return Err(DocumentError::BadRequest(
            "One or more labels were not found".to_string(),
        ));
    }

    info!("Uploading document: {} ({} bytes)", filename, data.len());

    let now = Utc::now();
    let virtual_path = format!("upload/{}", filename);
    let mut file_info = FileIngestionInfo {
        relative_path: virtual_path.clone(),
        full_path: virtual_path.clone(),
        #[allow(deprecated)]
        path: virtual_path,
        name: filename.clone(),
        size: data.len() as i64,
        mime_type: mime_type.clone(),
        last_modified: Some(now),
        etag: format!("{}-{}", data.len(), now.timestamp()),
        is_directory: false,
        created_at: Some(now),
        permissions: None,
        owner: Some(user.username.clone()),
        group: None,
        metadata: None,
    };
    if let Ok(Some(metadata)) =
        crate::metadata_extraction::extract_content_metadata(&data, &mime_type, &filename).await
    {
        file_info.metadata = Some(metadata);
    }

    let ingestion_service =
        DocumentIngestionService::new(state.db.clone(), state.file_service.as_ref().clone());
    let ingestion_start = std::time::Instant::now();
    let result = ingestion_service
        .ingest_from_file_info(
            &file_info,
            data,
            user.id,
            DeduplicationPolicy::Skip,
            "web_upload",
            None,
        )
        .await;

    match result {
        Ok(IngestionResult::Created(document)) => {
            info!("Document uploaded successfully: {}", document.id);
            attach_labels(state, user, document.id, &label_ids).await?;
            store_language_preference(state, user, &ocr_languages, ocr_language.as_deref()).await;

            match state
                .queue_service
                .enqueue_document(document.id, USER_OCR_PRIORITY, document.file_size)
                .await
            {
                // The upload itself succeeded; OCR can be retried later.
                Err(e) => error!("Failed to enqueue document {} for OCR: {}", document.id, e),
                Ok(_) => info!("Document {} enqueued for OCR processing", document.id),
            }

            Ok(DocumentUploadResponse {
                id: document.id,
                filename: document.filename,
                file_size: document.file_size,
                mime_type: document.mime_type,
                status: "success".to_string(),
                message: "Document uploaded successfully".to_string(),
            })
        }
        Ok(IngestionResult::ExistingDocument(existing)) => {
            warn!("Duplicate document upload attempted: {}", existing.id);
            attach_labels(state, user, existing.id, &label_ids).await?;
            Ok(DocumentUploadResponse {
                id: existing.id,
                filename: existing.filename,
                file_size: existing.file_size,
                mime_type: existing.mime_type,
                status: "duplicate".to_string(),
                message: "Document already exists".to_string(),
            })
        }
        Ok(IngestionResult::Skipped {
            existing_document_id,
            reason,
        }) => {
            let msg = format!(
                "Document upload skipped - {}: {}",
                reason, existing_document_id
            );
            info!("{}", msg);
            Err(DocumentError::Conflict(msg))
        }
        Ok(IngestionResult::TrackedAsDuplicate {
            existing_document_id,
        }) => {
            let msg = format!("Document tracked as duplicate: {}", existing_document_id);
            info!("{}", msg);
            Err(DocumentError::Conflict(msg))
        }
        Err(e) => {
            error!(
                "Failed to ingest document: {} (failed after {:?})",
                e,
                ingestion_start.elapsed()
            );
            Err(categorize_ingestion_error(&e.to_string()))
        }
    }
}

/// Rejects uploads larger than `MAX_FILE_SIZE_MB`.
pub fn check_upload_size(
    state: &AppState,
    filename: &str,
    size: usize,
) -> Result<(), DocumentError> {
    let max_bytes = state.config.max_file_size_mb as usize * 1024 * 1024;
    if size > max_bytes {
        let msg = format!(
            "File '{}' size ({} bytes) exceeds maximum allowed size ({} bytes / {}MB)",
            filename, size, max_bytes, state.config.max_file_size_mb
        );
        warn!("{}", msg);
        return Err(DocumentError::PayloadTooLarge(msg));
    }
    Ok(())
}

async fn attach_labels(
    state: &AppState,
    user: &User,
    document_id: Uuid,
    label_ids: &[Uuid],
) -> Result<(), DocumentError> {
    state
        .db
        .add_labels_to_document(document_id, label_ids, user.id)
        .await
        .map_err(|e| {
            error!("Failed to label document {}: {}", document_id, e);
            DocumentError::InternalServerError(
                "Document was uploaded but its labels could not be applied".to_string(),
            )
        })
}

/// Stores the OCR languages an upload or retry asked for as the user's
/// preference. Failures are logged, not returned: the document is already
/// stored and queued.
async fn store_language_preference(
    state: &AppState,
    user: &User,
    languages: &[String],
    language: Option<&str>,
) {
    if let Some(primary) = languages.first() {
        let health_checker = crate::ocr::health::OcrHealthChecker::new();
        if let Err(e) = health_checker.validate_preferred_languages(languages) {
            warn!(
                "Invalid language combination provided, not updating user settings: {}",
                e
            );
            return;
        }
        let update = crate::models::UpdateSettings::language_update(
            languages.to_vec(),
            primary.clone(),
            primary.clone(),
        );
        match state.db.create_or_update_settings(user.id, &update).await {
            Ok(_) => info!(
                "Updated user {} preferred languages to: {:?}",
                user.id, languages
            ),
            Err(e) => warn!(
                "Failed to update user preferred languages to {:?}: {}",
                languages, e
            ),
        }
    } else if let Some(lang) = language {
        match state.db.update_user_ocr_language(user.id, lang).await {
            Ok(_) => info!("Updated user {} OCR language to: {}", user.id, lang),
            Err(e) => warn!("Failed to update user OCR language to {}: {}", lang, e),
        }
    }
}

/// Maps an ingestion failure onto the closest client-facing error.
fn categorize_ingestion_error(message: &str) -> DocumentError {
    if message.contains("constraint") || message.contains("duplicate") {
        DocumentError::DatabaseConstraintViolation(format!(
            "Database constraint violation during upload: {}",
            message
        ))
    } else if message.contains("timeout") {
        DocumentError::UploadTimeout(format!("Upload processing timed out: {}", message))
    } else if message.contains("ocr") || message.contains("processing") {
        DocumentError::OcrProcessingError(format!("OCR processing error: {}", message))
    } else if message.contains("file") || message.contains("read") {
        DocumentError::FileProcessingError(format!("File processing error: {}", message))
    } else {
        DocumentError::InternalServerError(format!("Failed to ingest document: {}", message))
    }
}

/// Outcome of a manual OCR retry.
#[derive(Debug, serde::Serialize)]
pub struct OcrRetryOutcome {
    /// False when OCR was already running, so nothing was queued.
    pub queued: bool,
    pub message: String,
}

/// Queues a document for another OCR pass, optionally storing new OCR
/// languages as the user's preference first. Invalid languages are a
/// `BadRequest`; a missing or foreign document is `NotFound`.
pub async fn retry_document_ocr(
    state: &AppState,
    user: &User,
    document_id: Uuid,
    languages: Option<&[String]>,
    language: Option<&str>,
) -> Result<OcrRetryOutcome, DocumentError> {
    let document = state
        .db
        .get_document_by_id(document_id, user.id, user.role)
        .await
        .map_err(|e| {
            error!("Database error getting document {}: {}", document_id, e);
            DocumentError::InternalServerError("Failed to load document".to_string())
        })?
        .ok_or(DocumentError::NotFound)?;

    if document.ocr_status.as_deref() == Some("processing") {
        return Ok(OcrRetryOutcome {
            queued: false,
            message: "OCR is already in progress for this document".to_string(),
        });
    }

    let health_checker = crate::ocr::health::OcrHealthChecker::new();
    if let Some(languages) = languages {
        health_checker
            .validate_preferred_languages(languages)
            .map_err(|e| {
                warn!(
                    "Invalid OCR languages {:?} for document {}: {}",
                    languages, document_id, e
                );
                DocumentError::BadRequest(format!("Invalid OCR languages: {}", e))
            })?;
        store_language_preference(state, user, languages, None).await;
    } else if let Some(lang) = language {
        health_checker.validate_language(lang).map_err(|e| {
            warn!(
                "Invalid OCR language '{}' for document {}: {}",
                lang, document_id, e
            );
            DocumentError::BadRequest(format!("Invalid OCR language '{}': {}", lang, e))
        })?;
        store_language_preference(state, user, &[], Some(lang)).await;
    }

    state
        .queue_service
        .enqueue_document(document.id, USER_OCR_PRIORITY, document.file_size)
        .await
        .map_err(|e| {
            error!("Failed to queue document {} for OCR: {}", document_id, e);
            DocumentError::InternalServerError("Failed to queue document for OCR".to_string())
        })?;
    info!("Document {} queued for OCR retry", document_id);
    Ok(OcrRetryOutcome {
        queued: true,
        message: "Document queued for OCR processing".to_string(),
    })
}
