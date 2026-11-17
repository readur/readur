//! Document operations shared by the REST document handlers and
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
    errors::document::DocumentError,
    ingestion::document_ingestion::{
        DeduplicationPolicy, DocumentIngestionService, IngestionResult, USER_UPLOAD_SOURCE_TYPE,
    },
    models::{
        DocumentFilters, DocumentOcrResponse, DocumentPaginationInfo, DocumentResponse,
        DocumentSortField, DocumentUploadResponse, FileIngestionInfo, PaginatedDocumentsResponse,
        SortOrder, User,
    },
    ocr::{priority::user_priority_for_file_size, queue::EnqueueOutcome},
    services::ingestion_error::categorize_ingestion_error,
    AppState,
};

/// Upload warnings (`DocumentUploadResponse::warnings`). Public so the MCP
/// upload tool can recognise them and tell the model what to do next.
pub const OCR_NOT_QUEUED_WARNING: &str =
    "The document could not be queued for OCR; retry OCR for it.";
pub const LABELS_NOT_APPLIED_WARNING: &str = "The requested labels could not be applied.";

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
    // Sequential on purpose: running these concurrently would hold two
    // pool connections per list call, doubling pool pressure under load for
    // one round trip of latency.
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

    // Sequential for the same reason as `list_documents`.
    let labels = state.db.get_document_labels(document_id).await?;
    let owner = state.db.get_user_by_id(document.user_id).await?;
    let username = owner.map(|owner| owner.username);

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
        .delete_document_by_user(
            document_id,
            user.id,
            user.role,
            crate::db::ignored_files::REASON_DELETED_BY_USER,
        )
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
///
/// A file identical to one the caller already has is rejected with
/// `DocumentError::Conflict` (409), with no labels applied. This is the REST
/// contract the web UI relies on: it treats any success as a new document
/// (marks it new on the Board) and shows the conflict message, which names
/// the existing document, as the reason the file was not uploaded.
pub async fn upload_document(
    state: &AppState,
    user: &User,
    request: UploadRequest,
) -> Result<DocumentUploadResponse, DocumentError> {
    ingest_upload(state, user, request, DeduplicationPolicy::Skip).await
}

/// Like [`upload_document`], but a file identical to one the caller already
/// has returns that document (status `"duplicate"`) with the requested
/// labels applied to it, instead of an error. For clients that want the
/// upload to be idempotent, such as the MCP `upload_document` tool.
pub async fn upload_document_or_existing(
    state: &AppState,
    user: &User,
    request: UploadRequest,
) -> Result<DocumentUploadResponse, DocumentError> {
    ingest_upload(state, user, request, DeduplicationPolicy::ReturnExisting).await
}

/// The shared upload pipeline; `on_duplicate` decides what an upload of
/// content the caller already has does (`Skip` rejects it, `ReturnExisting`
/// returns the existing document).
async fn ingest_upload(
    state: &AppState,
    user: &User,
    request: UploadRequest,
    on_duplicate: DeduplicationPolicy,
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

    check_label_ownership(state, user, &label_ids).await?;

    info!("Uploading document: {} ({} bytes)", filename, data.len());

    let file_info = upload_file_info(user, &filename, &mime_type, &data).await;

    let ingestion_service =
        DocumentIngestionService::new(state.db.clone(), state.file_service.as_ref().clone());
    let ingestion_start = std::time::Instant::now();
    let result = ingestion_service
        .ingest_from_file_info(
            &file_info,
            data,
            user.id,
            on_duplicate,
            USER_UPLOAD_SOURCE_TYPE,
            None,
        )
        .await;

    match result {
        Ok(IngestionResult::Created(document)) => Ok(finish_new_upload(
            state,
            user,
            document,
            &label_ids,
            &ocr_languages,
            ocr_language.as_deref(),
        )
        .await),
        Ok(IngestionResult::ExistingDocument(existing)) => {
            Ok(existing_upload(state, user, existing, &label_ids).await)
        }
        Ok(IngestionResult::Skipped {
            existing_document_id,
            reason,
        }) => Err(rejected_upload(format!(
            "Document upload skipped - {}: {}",
            reason, existing_document_id
        ))),
        Ok(IngestionResult::TrackedAsDuplicate {
            existing_document_id,
        }) => Err(rejected_upload(format!(
            "Document tracked as duplicate: {}",
            existing_document_id
        ))),
        Ok(IngestionResult::Ignored { ignored_file_id }) => {
            Err(ignored_upload(&filename, ignored_file_id))
        }
        Err(e) => {
            warn!(
                "Upload of {} failed after {:?}",
                filename,
                ingestion_start.elapsed()
            );
            Err(categorize_ingestion_error(&e))
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

/// Post-ingestion steps for a newly stored upload: language preference,
/// OCR queueing and labels. Infallible by design (see the comment inside).
async fn finish_new_upload(
    state: &AppState,
    user: &User,
    document: crate::models::Document,
    label_ids: &[Uuid],
    ocr_languages: &[String],
    ocr_language: Option<&str>,
) -> DocumentUploadResponse {
    info!("Document uploaded successfully: {}", document.id);
    // The document is stored at this point, so nothing below may fail
    // the upload: a retry would only hit deduplication and never queue
    // OCR. Queue OCR first, then label; report failures as warnings.
    store_language_preference(state, user, ocr_languages, ocr_language).await;

    let mut warnings: Vec<String> = queue_upload_for_ocr(state, &document)
        .await
        .into_iter()
        .collect();
    warnings.extend(attach_labels(state, user, document.id, label_ids).await);

    DocumentUploadResponse {
        id: document.id,
        filename: document.filename,
        file_size: document.file_size,
        mime_type: document.mime_type,
        status: "success".to_string(),
        message: "Document uploaded successfully".to_string(),
        warnings,
    }
}

/// Describes an uploaded file for ingestion, including any metadata that
/// can be extracted from its content.
async fn upload_file_info(
    user: &User,
    filename: &str,
    mime_type: &str,
    data: &[u8],
) -> FileIngestionInfo {
    let now = Utc::now();
    let virtual_path = format!("upload/{}", filename);
    let mut file_info = FileIngestionInfo {
        relative_path: virtual_path.clone(),
        full_path: virtual_path.clone(),
        #[allow(deprecated)]
        path: virtual_path,
        name: filename.to_string(),
        size: data.len() as i64,
        mime_type: mime_type.to_string(),
        last_modified: Some(now),
        etag: format!("{}-{}", data.len(), now.timestamp()),
        is_directory: false,
        created_at: Some(now),
        permissions: None,
        owner: Some(user.username.clone()),
        group: None,
        metadata: None,
    };
    match crate::metadata_extraction::extract_content_metadata(data, mime_type, filename).await {
        Ok(metadata) => file_info.metadata = metadata,
        Err(e) => warn!("Failed to extract metadata from {}: {}", filename, e),
    }

    file_info
}

/// Fails an upload whose labels are not all the caller's own.
async fn check_label_ownership(
    state: &AppState,
    user: &User,
    label_ids: &[Uuid],
) -> Result<(), DocumentError> {
    let owns_labels = state
        .db
        .user_owns_labels(label_ids, user.id)
        .await
        .map_err(|e| {
            error!("Failed to verify upload labels: {}", e);
            DocumentError::InternalServerError("Failed to verify labels")
        })?;
    if owns_labels {
        Ok(())
    } else {
        Err(DocumentError::BadRequest(
            "One or more labels were not found".to_string(),
        ))
    }
}

/// An upload identical to one of the caller's documents: return that
/// document, still applying any requested labels to it.
async fn existing_upload(
    state: &AppState,
    user: &User,
    existing: crate::models::Document,
    label_ids: &[Uuid],
) -> DocumentUploadResponse {
    warn!("Duplicate document upload attempted: {}", existing.id);
    let warnings = attach_labels(state, user, existing.id, label_ids)
        .await
        .into_iter()
        .collect();
    DocumentUploadResponse {
        id: existing.id,
        filename: existing.filename,
        file_size: existing.file_size,
        mime_type: existing.mime_type,
        status: "duplicate".to_string(),
        message: "Document already exists".to_string(),
        warnings,
    }
}

/// The conflict reported for an ingestion that stored nothing.
fn rejected_upload(msg: String) -> DocumentError {
    info!("{}", msg);
    DocumentError::Conflict(msg)
}

/// Uploads are never checked against the ignored-files list
/// (`USER_UPLOAD_SOURCE_TYPE`), so an upload reported as ignored is an
/// ingestion bug.
fn ignored_upload(filename: &str, ignored_file_id: Uuid) -> DocumentError {
    error!(
        "Upload of {} was reported as ignored ({})",
        filename, ignored_file_id
    );
    DocumentError::InternalServerError("Failed to ingest document")
}

/// Queues a new upload for OCR, returning a warning if that failed.
async fn queue_upload_for_ocr(
    state: &AppState,
    document: &crate::models::Document,
) -> Option<String> {
    match state
        .queue_service
        .enqueue_document(
            document.id,
            user_priority_for_file_size(document.file_size),
            document.file_size,
        )
        .await
    {
        // A brand-new document cannot already have a job, but if it does
        // its OCR will run all the same.
        Ok(EnqueueOutcome::Queued(_) | EnqueueOutcome::AlreadyQueued) => {
            info!("Document {} enqueued for OCR processing", document.id);
            None
        }
        Err(e) => {
            error!("Failed to enqueue document {} for OCR: {}", document.id, e);
            Some(OCR_NOT_QUEUED_WARNING.to_string())
        }
    }
}

/// Attaches the requested labels, returning a warning instead of an error
/// on failure: by now the document is stored and must stay that way.
async fn attach_labels(
    state: &AppState,
    user: &User,
    document_id: Uuid,
    label_ids: &[Uuid],
) -> Option<String> {
    let result = state
        .db
        .add_labels_to_document(document_id, label_ids, user.id)
        .await;
    result.err().map(|e| {
        error!("Failed to label document {}: {}", document_id, e);
        LABELS_NOT_APPLIED_WARNING.to_string()
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
        store_preferred_languages(state, user, languages, primary).await;
    } else if let Some(lang) = language {
        match state.db.update_user_ocr_language(user.id, lang).await {
            Ok(_) => info!("Updated user {} OCR language to: {}", user.id, lang),
            Err(e) => warn!("Failed to update user OCR language to {}: {}", lang, e),
        }
    }
}

/// Stores a multi-language OCR preference with `primary` first, if the
/// combination is valid.
async fn store_preferred_languages(
    state: &AppState,
    user: &User,
    languages: &[String],
    primary: &str,
) {
    let health_checker = crate::ocr::health::OcrHealthChecker::new();
    if let Err(e) = health_checker.validate_preferred_languages(languages) {
        warn!(
            "Invalid language combination provided, not updating user settings: {}",
            e
        );
        return;
    }
    save_preferred_languages(state, user, languages, primary).await;
}

/// Saves an already validated multi-language OCR preference.
async fn save_preferred_languages(
    state: &AppState,
    user: &User,
    languages: &[String],
    primary: &str,
) {
    let update = crate::models::UpdateSettings::language_update(
        languages.to_vec(),
        primary.to_string(),
        primary.to_string(),
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
            DocumentError::InternalServerError("Failed to load document")
        })?
        .ok_or(DocumentError::NotFound)?;

    // No early return on the document's own `ocr_status`: 'processing' can
    // be stale (a worker died mid-job), and a retried 'failed' document stays
    // 'failed' until a worker claims it. Only the queue knows whether a job
    // is active, and the enqueue below asks it atomically.
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
        if let Some(primary) = languages.first() {
            save_preferred_languages(state, user, languages, primary).await;
        }
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

    let outcome = state
        .queue_service
        .enqueue_document(
            document.id,
            user_priority_for_file_size(document.file_size),
            document.file_size,
        )
        .await
        .map_err(|e| {
            error!("Failed to queue document {} for OCR: {}", document_id, e);
            DocumentError::InternalServerError("Failed to queue document for OCR")
        })?;
    if outcome == EnqueueOutcome::AlreadyQueued {
        return Ok(OcrRetryOutcome {
            queued: false,
            message: "The document is already queued for OCR".to_string(),
        });
    }
    info!("Document {} queued for OCR retry", document_id);
    Ok(OcrRetryOutcome {
        queued: true,
        message: "Document queued for OCR processing".to_string(),
    })
}
