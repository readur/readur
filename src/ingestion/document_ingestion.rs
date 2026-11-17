/*!
 * Unified Document Ingestion Service
 *
 * This module provides a centralized abstraction for document ingestion with
 * consistent deduplication logic across all sources (direct upload, WebDAV,
 * source sync, batch ingest, folder watcher).
 */

use chrono::Utc;
use serde_json;
use sha2::{Digest, Sha256};
use tracing::{debug, warn};
use uuid::Uuid;

use crate::db::unique_violation::{violated_unique_constraint_in, DOCUMENTS_USER_FILE_HASH_UNIQUE};
use crate::db::Database;
use crate::models::{Document, FileIngestionInfo};
use crate::services::file_service::FileService;
#[cfg(feature = "ocr")]
use exif::{In, Reader as ExifReader, Tag};
#[cfg(feature = "ocr")]
use image::ImageFormat;

/// What ingestion reports when the user already has a document with the same
/// content (same SHA-256 of the source bytes).
///
/// Every policy ends in exactly one stored document per user and content:
/// the per-user unique index `idx_documents_user_file_hash` makes a second
/// copy impossible, so there is deliberately no "store it again" policy (an
/// `AllowDuplicateContent` variant used to exist; it could only ever end in
/// the same outcome as `ReturnExisting`, after writing and deleting a file).
/// The policy only chooses how the duplicate is reported, and that choice is
/// the same whether the duplicate was found up front or by losing a race to a
/// concurrent ingestion of the same content (see [`duplicate_outcome`]).
#[derive(Debug, Clone)]
pub enum DeduplicationPolicy {
    /// Report `Skipped` (batch operations; REST uploads, which answer 409)
    Skip,
    /// Report `ExistingDocument` with the stored document
    ReturnExisting,
    /// Report `TrackedAsDuplicate` linking to the stored document (WebDAV)
    TrackAsDuplicate,
}

/// The result reported for content the user already has as `existing`.
/// Used by both the up-front duplicate check and the concurrent-insert race
/// path, so a caller sees the same outcome whichever one caught it.
pub fn duplicate_outcome(policy: &DeduplicationPolicy, existing: Document) -> IngestionResult {
    match policy {
        DeduplicationPolicy::Skip => IngestionResult::Skipped {
            existing_document_id: existing.id,
            reason: format!("Content already exists as '{}'", existing.original_filename),
        },
        DeduplicationPolicy::ReturnExisting => IngestionResult::ExistingDocument(existing),
        DeduplicationPolicy::TrackAsDuplicate => IngestionResult::TrackedAsDuplicate {
            existing_document_id: existing.id,
        },
    }
}

#[derive(Debug)]
pub enum IngestionResult {
    /// New document was created
    Created(Document),
    /// Existing document was returned (content duplicate)
    ExistingDocument(Document),
    /// Document was skipped due to duplication policy
    Skipped {
        existing_document_id: Uuid,
        reason: String,
    },
    /// Document was tracked as duplicate (for WebDAV)
    TrackedAsDuplicate { existing_document_id: Uuid },
}

#[derive(Debug)]
pub struct DocumentIngestionRequest {
    pub filename: String,
    pub original_filename: String,
    pub file_data: Vec<u8>,
    pub mime_type: String,
    pub user_id: Uuid,
    pub deduplication_policy: DeduplicationPolicy,
    /// Optional source identifier for tracking
    pub source_type: Option<String>,
    pub source_id: Option<Uuid>,
    /// Optional metadata from source file system
    pub original_created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub original_modified_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Original file path in source system
    pub source_path: Option<String>,
    /// File permissions from source system (Unix mode bits)
    pub file_permissions: Option<i32>,
    /// File owner from source system
    pub file_owner: Option<String>,
    /// File group from source system
    pub file_group: Option<String>,
    /// Additional metadata from source system (EXIF, PDF metadata, etc.)
    pub source_metadata: Option<serde_json::Value>,
}

/// What a failed-document record needs to know about an ingestion, taken
/// from the request before its fields are moved into the document row.
#[derive(Debug, Clone)]
pub(crate) struct IngestionFailureContext {
    user_id: Uuid,
    filename: String,
    original_filename: String,
    mime_type: String,
    file_hash: String,
    file_size: i64,
    /// The request's source type, or "upload" when it has none.
    ingestion_source: String,
}

impl IngestionFailureContext {
    pub(crate) fn new(request: &DocumentIngestionRequest, file_hash: &str, file_size: i64) -> Self {
        Self {
            user_id: request.user_id,
            filename: request.filename.clone(),
            original_filename: request.original_filename.clone(),
            mime_type: request.mime_type.clone(),
            file_hash: file_hash.to_string(),
            file_size,
            ingestion_source: request
                .source_type
                .clone()
                .unwrap_or_else(|| "upload".to_string()),
        }
    }

    /// The failed-document record for a failure at `stage` ("storage" when
    /// the file could not be saved, "ingestion" when its row could not be
    /// inserted). `file_path` is where the file was stored, if it was.
    pub(crate) fn failed_document(
        &self,
        stage: &str,
        file_path: Option<String>,
        error: &dyn std::fmt::Display,
    ) -> crate::models::FailedDocument {
        crate::models::FailedDocument {
            id: Uuid::new_v4(),
            user_id: self.user_id,
            filename: self.filename.clone(),
            original_filename: Some(self.original_filename.clone()),
            original_path: None,
            file_path,
            file_size: Some(self.file_size),
            file_hash: Some(self.file_hash.clone()),
            mime_type: Some(self.mime_type.clone()),
            content: None,
            tags: Vec::new(),
            ocr_text: None,
            ocr_confidence: None,
            ocr_word_count: None,
            ocr_processing_time_ms: None,
            failure_reason: crate::models::FailureReason::Other.to_string(),
            failure_stage: stage.to_string(),
            existing_document_id: None,
            ingestion_source: self.ingestion_source.clone(),
            error_message: Some(error.to_string()),
            retry_count: Some(0),
            last_retry_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

pub struct DocumentIngestionService {
    db: Database,
    file_service: FileService,
}

impl DocumentIngestionService {
    pub fn new(db: Database, file_service: FileService) -> Self {
        Self { db, file_service }
    }

    /// Extract metadata from FileIngestionInfo for storage in document
    fn extract_metadata_from_file_info(
        file_info: &FileIngestionInfo,
    ) -> (
        Option<chrono::DateTime<chrono::Utc>>,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<serde_json::Value>,
    ) {
        let original_created_at = file_info.created_at;
        let original_modified_at = file_info.last_modified;

        // Build comprehensive metadata object
        let mut metadata = serde_json::Map::new();

        // Add permissions if available
        if let Some(perms) = file_info.permissions {
            metadata.insert(
                "permissions".to_string(),
                serde_json::Value::Number(perms.into()),
            );
        }

        // Add owner/group info
        if let Some(ref owner) = file_info.owner {
            metadata.insert(
                "owner".to_string(),
                serde_json::Value::String(owner.clone()),
            );
        }

        if let Some(ref group) = file_info.group {
            metadata.insert(
                "group".to_string(),
                serde_json::Value::String(group.clone()),
            );
        }

        // Add source path
        metadata.insert(
            "source_path".to_string(),
            serde_json::Value::String(file_info.relative_path.clone()),
        );

        // Merge any additional metadata from the source
        if let Some(serde_json::Value::Object(source_map)) = &file_info.metadata {
            metadata.extend(source_map.clone());
        }

        let final_metadata = if metadata.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(metadata))
        };

        (original_created_at, original_modified_at, final_metadata)
    }

    /// Unified document ingestion with configurable deduplication policy.
    ///
    /// Identity semantics: `file_hash` and `file_size` both describe the
    /// bytes as received from the source. The hash lets a re-sync of the
    /// same remote file be recognised as a duplicate, and the size is what a
    /// source listing reports, so the unchanged-file check of source sync can
    /// compare against it. Both hold even when the stored bytes differ
    /// (images may be re-encoded by EXIF auto-rotation).
    ///
    /// Steps: report content the user already has; otherwise prepare the
    /// bytes (auto-rotation), store the file, and insert the row, resolving a
    /// lost race with a concurrent ingestion of the same content. Storage and
    /// insert failures are recorded as failed documents before returning.
    pub async fn ingest_document(
        &self,
        mut request: DocumentIngestionRequest,
    ) -> Result<IngestionResult, Box<dyn std::error::Error + Send + Sync>> {
        let file_hash = self.calculate_file_hash(&request.file_data);
        let file_size = request.file_data.len() as i64;

        debug!(
            "Ingesting document: {} for user {} (hash: {}, size: {} bytes, policy: {:?})",
            request.filename,
            request.user_id,
            &file_hash[..8],
            file_size,
            request.deduplication_policy
        );

        if let Some(existing_doc) = self
            .find_existing_content(request.user_id, &file_hash, &request.filename)
            .await
        {
            return Ok(duplicate_outcome(
                &request.deduplication_policy,
                existing_doc,
            ));
        }

        let failure = IngestionFailureContext::new(&request, &file_hash, file_size);

        // Generate document ID upfront so we can use it for storage path
        let document_id = Uuid::new_v4();
        let source_bytes = std::mem::take(&mut request.file_data);
        let file_data = self
            .prepare_file_data(
                request.user_id,
                &request.mime_type,
                &request.filename,
                source_bytes,
            )
            .await?;

        // Save file to storage - use S3 if configured, otherwise local storage
        let file_path = match self
            .file_service
            .save_document_file(request.user_id, document_id, &request.filename, &file_data)
            .await
        {
            Ok(path) => path,
            Err(e) => {
                warn!("Failed to save file {}: {}", request.filename, e);
                self.record_failure(
                    failure.failed_document("storage", None, &e),
                    "storage error",
                )
                .await;
                return Err(e.into());
            }
        };

        let policy = request.deduplication_policy.clone();
        let document = self.build_document(document_id, request, &file_path, file_size, &file_hash);
        self.insert_document(document, &policy, &failure, &file_path)
            .await
    }

    /// The user's stored document with this content, if any. A failed
    /// lookup is logged and treated as "none": ingestion goes on, and the
    /// per-user hash index still prevents a second copy at insert time.
    async fn find_existing_content(
        &self,
        user_id: Uuid,
        file_hash: &str,
        filename: &str,
    ) -> Option<Document> {
        match self
            .db
            .get_document_by_user_and_hash(user_id, file_hash)
            .await
        {
            Ok(Some(existing_doc)) => {
                debug!(
                    "Found existing document with same content: {} (ID: {}) matches new file: {}",
                    existing_doc.original_filename, existing_doc.id, filename
                );
                Some(existing_doc)
            }
            Ok(None) => {
                debug!("No duplicate content found, proceeding with new document creation");
                None
            }
            Err(e) => {
                warn!(
                    "Error checking for duplicate content (hash: {}): {}",
                    &file_hash[..8],
                    e
                );
                None
            }
        }
    }

    /// The bytes to store: images are rotated upright from their EXIF
    /// orientation when the user's `auto_rotate_images` setting is on; a
    /// rotation failure keeps the original bytes. Anything else is unchanged.
    async fn prepare_file_data(
        &self,
        user_id: Uuid,
        mime_type: &str,
        filename: &str,
        data: Vec<u8>,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        if !mime_type.starts_with("image/") {
            return Ok(data);
        }
        let auto_rotate = matches!(
            self.db.get_user_settings(user_id).await?,
            Some(settings) if settings.auto_rotate_images
        );
        if !auto_rotate {
            return Ok(data);
        }
        match self.auto_rotate_image(&data) {
            Ok(rotated_data) => Ok(rotated_data),
            Err(e) => {
                warn!(
                    "Failed to auto-rotate image {}: {}, proceeding with original data",
                    filename, e
                );
                Ok(data)
            }
        }
    }

    /// The document row for a stored file, with the same ID used for storage.
    fn build_document(
        &self,
        document_id: Uuid,
        request: DocumentIngestionRequest,
        file_path: &str,
        file_size: i64,
        file_hash: &str,
    ) -> Document {
        self.file_service.create_document_with_id(
            document_id,
            &request.filename,
            &request.original_filename,
            file_path,
            file_size,
            &request.mime_type,
            request.user_id,
            Some(file_hash.to_string()),
            request.original_created_at,
            request.original_modified_at,
            request.source_path,
            request.source_type,
            request.source_id,
            request.file_permissions,
            request.file_owner,
            request.file_group,
            request.source_metadata,
        )
    }

    /// Inserts the row for an already stored file. Losing the per-user hash
    /// index to a concurrent ingestion is resolved by `resolve_insert_race`;
    /// any other failure is recorded as a failed document and returned.
    async fn insert_document(
        &self,
        document: Document,
        policy: &DeduplicationPolicy,
        failure: &IngestionFailureContext,
        file_path: &str,
    ) -> Result<IngestionResult, Box<dyn std::error::Error + Send + Sync>> {
        let stored_document = document.clone();
        match self.db.create_document(document).await {
            Ok(saved_document) => {
                debug!(
                    "Successfully ingested document: {} (ID: {}) for user {}",
                    saved_document.original_filename, saved_document.id, saved_document.user_id
                );
                Ok(IngestionResult::Created(saved_document))
            }
            // Recognised by SQLSTATE and index name, not by the (localised) error text.
            Err(e)
                if violated_unique_constraint_in(&e) == Some(DOCUMENTS_USER_FILE_HASH_UNIQUE) =>
            {
                self.resolve_insert_race(&stored_document, failure, policy, e)
                    .await
            }
            Err(e) => {
                warn!(
                    "Failed to create document record for {} (hash: {}): {}",
                    failure.filename,
                    &failure.file_hash[..8],
                    e
                );
                let failed = failure.failed_document("ingestion", Some(file_path.to_string()), &e);
                self.record_failure(failed, "database error").await;
                Err(e.into())
            }
        }
    }

    /// A concurrent request stored the same content for this user between
    /// the duplicate check and our insert, so the per-user hash index
    /// rejected `uncommitted`. Its file has no owner and is removed; the
    /// winner is reported exactly as the up-front check would have. If the
    /// winner cannot be read back, the original insert error is returned.
    async fn resolve_insert_race(
        &self,
        uncommitted: &Document,
        failure: &IngestionFailureContext,
        policy: &DeduplicationPolicy,
        insert_error: anyhow::Error,
    ) -> Result<IngestionResult, Box<dyn std::error::Error + Send + Sync>> {
        warn!(
            "Hash collision detected during concurrent upload for {} (hash: {}), fetching existing document",
            failure.filename,
            &failure.file_hash[..8]
        );

        // The row was never created, so the file written above has no owner.
        self.remove_orphaned_file(uncommitted).await;

        match self.find_race_winner(failure).await {
            Some(existing_doc) => Ok(duplicate_outcome(policy, existing_doc)),
            None => Err(insert_error.into()),
        }
    }

    /// The document that won an insert race for this content, re-read after
    /// the unique violation. `None` (logged) if it cannot be read back.
    async fn find_race_winner(&self, failure: &IngestionFailureContext) -> Option<Document> {
        let short_hash = &failure.file_hash[..8];
        match self
            .db
            .get_document_by_user_and_hash(failure.user_id, &failure.file_hash)
            .await
        {
            Ok(Some(existing_doc)) => {
                debug!(
                    "Found existing document after collision for {}: {} (ID: {})",
                    failure.filename, existing_doc.original_filename, existing_doc.id
                );
                Some(existing_doc)
            }
            Ok(None) => {
                warn!(
                    "Unexpected: constraint violation but no document found for hash {}",
                    short_hash
                );
                None
            }
            Err(fetch_err) => {
                warn!(
                    "Failed to fetch document after constraint violation: {}",
                    fetch_err
                );
                None
            }
        }
    }

    /// Stores a failed-document record. A failure to record it is logged,
    /// never returned: the caller is already returning the original error.
    async fn record_failure(&self, failed_document: crate::models::FailedDocument, what: &str) {
        if let Err(failed_err) = self.db.create_failed_document(failed_document).await {
            warn!(
                "Failed to create failed document record for {}: {}",
                what, failed_err
            );
        }
    }

    /// Remove the stored file of a document whose database row was not
    /// created. A failure here leaves an orphan file but no data loss, so it
    /// is logged rather than propagated.
    async fn remove_orphaned_file(&self, document: &Document) {
        if let Err(e) = self.file_service.delete_document_files(document).await {
            warn!(
                "Failed to remove stored file {} for uncommitted document {}: {}",
                document.file_path, document.id, e
            );
        }
    }

    /// Calculate SHA256 hash of file content
    fn calculate_file_hash(&self, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        crate::utils::hash::to_hex(&result)
    }

    /// Auto-rotate image bytes according to EXIF orientation tag (if present).
    ///
    /// Returns the possibly-rotated image bytes encoded back into the original format.
    pub fn auto_rotate_image(
        &self,
        data: &[u8],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        Self::auto_rotate_image_bytes(data)
    }

    /// Static helper to make testing easier.
    pub fn auto_rotate_image_bytes(
        data: &[u8],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        // If compiled without image support, return an error so callers can fall back
        // to using the original bytes (ingest_document handles errors by falling back).
        #[cfg(not(feature = "ocr"))]
        {
            return Err("image support not compiled in".into());
        }

        #[cfg(feature = "ocr")]
        {
            // First, attempt to read EXIF orientation if available
            let mut orientation: Option<u16> = None;

            let mut cursor = std::io::Cursor::new(data);
            if let Ok(exif) = ExifReader::new().read_from_container(&mut cursor) {
                if let Some(field) = exif.get_field(Tag::Orientation, In::PRIMARY) {
                    if let exif::Value::Short(ref vals) = field.value {
                        if let Some(v) = vals.first() {
                            orientation = Some(*v);
                        }
                    }
                }
            }

            // If no orientation tag found, return original bytes unchanged
            if orientation.is_none() {
                return Ok(data.to_vec());
            }

            // Load the image, apply the transformation, and encode back into original format.
            let mut img = image::load_from_memory(data)?;

            match orientation.unwrap() {
                1 => { /* normal */ }
                2 => img = img.fliph(),     // Mirror horizontal
                3 => img = img.rotate180(), // Rotate 180
                4 => img = img.flipv(),     // Mirror vertical
                5 => {
                    img = img.rotate90();
                    img = img.fliph();
                } // Mirror horizontal and rotate 270
                6 => img = img.rotate90(),  // Rotate 90
                7 => {
                    img = img.rotate270();
                    img = img.fliph();
                } // Mirror horizontal and rotate 90
                8 => img = img.rotate270(), // Rotate 270
                _ => { /* Unknown orientation; do nothing */ }
            }

            // Re-encode using the original format guessed from bytes
            let fmt = image::guess_format(data).unwrap_or(ImageFormat::Png);
            let mut out = Vec::new();
            img.write_to(&mut std::io::Cursor::new(&mut out), fmt)?;

            Ok(out)
        }
    }

    /// Ingest document from source with FileIngestionInfo metadata
    pub async fn ingest_from_file_info(
        &self,
        file_info: &FileIngestionInfo,
        file_data: Vec<u8>,
        user_id: Uuid,
        deduplication_policy: DeduplicationPolicy,
        source_type: &str,
        source_id: Option<Uuid>,
    ) -> Result<IngestionResult, Box<dyn std::error::Error + Send + Sync>> {
        let (original_created_at, original_modified_at, source_metadata) =
            Self::extract_metadata_from_file_info(file_info);

        let request = DocumentIngestionRequest {
            filename: file_info.name.clone(),
            original_filename: file_info.name.clone(),
            file_data,
            mime_type: file_info.mime_type.clone(),
            user_id,
            deduplication_policy,
            source_type: Some(source_type.to_string()),
            source_id,
            original_created_at,
            original_modified_at,
            source_path: Some(file_info.relative_path.clone()),
            file_permissions: file_info.permissions.map(|p| p as i32),
            file_owner: file_info.owner.clone(),
            file_group: file_info.group.clone(),
            source_metadata,
        };

        self.ingest_document(request).await
    }

    /// Convenience method for source sync operations
    pub async fn ingest_from_source(
        &self,
        filename: &str,
        file_data: Vec<u8>,
        mime_type: &str,
        user_id: Uuid,
        source_id: Uuid,
        source_type: &str,
    ) -> Result<IngestionResult, Box<dyn std::error::Error + Send + Sync>> {
        let request = DocumentIngestionRequest {
            filename: filename.to_string(),
            original_filename: filename.to_string(),
            file_data,
            mime_type: mime_type.to_string(),
            user_id,
            deduplication_policy: DeduplicationPolicy::Skip, // Skip duplicates for source sync
            source_type: Some(source_type.to_string()),
            source_id: Some(source_id),
            original_created_at: None,
            original_modified_at: None,
            source_path: None,      // Source sync files don't have a source path
            file_permissions: None, // Source sync files don't preserve permissions
            file_owner: None,       // Source sync files don't preserve owner
            file_group: None,       // Source sync files don't preserve group
            source_metadata: None,
        };

        self.ingest_document(request).await
    }

    /// Convenience method for WebDAV operations
    pub async fn ingest_from_webdav(
        &self,
        filename: &str,
        file_data: Vec<u8>,
        mime_type: &str,
        user_id: Uuid,
        webdav_source_id: Uuid,
    ) -> Result<IngestionResult, Box<dyn std::error::Error + Send + Sync>> {
        let request = DocumentIngestionRequest {
            filename: filename.to_string(),
            original_filename: filename.to_string(),
            file_data,
            mime_type: mime_type.to_string(),
            user_id,
            deduplication_policy: DeduplicationPolicy::TrackAsDuplicate, // Track duplicates for WebDAV
            source_type: Some("webdav".to_string()),
            source_id: Some(webdav_source_id),
            original_created_at: None,
            original_modified_at: None,
            source_path: None, // WebDAV files don't have a source path in this method
            file_permissions: None, // WebDAV files don't preserve permissions in this method
            file_owner: None,  // WebDAV files don't preserve owner in this method
            file_group: None,  // WebDAV files don't preserve group in this method
            source_metadata: None,
        };

        self.ingest_document(request).await
    }

    /// Convenience method for batch ingestion
    pub async fn ingest_batch_file(
        &self,
        filename: &str,
        file_data: Vec<u8>,
        mime_type: &str,
        user_id: Uuid,
    ) -> Result<IngestionResult, Box<dyn std::error::Error + Send + Sync>> {
        let request = DocumentIngestionRequest {
            filename: filename.to_string(),
            original_filename: filename.to_string(),
            file_data,
            mime_type: mime_type.to_string(),
            user_id,
            deduplication_policy: DeduplicationPolicy::Skip, // Skip duplicates for batch operations
            source_type: Some("batch_ingest".to_string()),
            source_id: None,
            original_created_at: None,
            original_modified_at: None,
            source_path: None,      // Batch files don't have a source path
            file_permissions: None, // Batch files don't preserve permissions
            file_owner: None,       // Batch files don't preserve owner
            file_group: None,       // Batch files don't preserve group
            source_metadata: None,
        };

        self.ingest_document(request).await
    }
}

// TODO: Add comprehensive tests once test_helpers module is available

#[cfg(test)]
#[path = "document_ingestion_tests.rs"]
mod tests;
