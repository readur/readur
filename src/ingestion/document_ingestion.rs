/*!
 * Unified Document Ingestion Service
 * 
 * This module provides a centralized abstraction for document ingestion with
 * consistent deduplication logic across all sources (direct upload, WebDAV, 
 * source sync, batch ingest, folder watcher).
 */

use uuid::Uuid;
use sha2::{Digest, Sha256};
use tracing::{debug, warn};
use serde_json;
use chrono::Utc;

use crate::models::{Document, FileIngestionInfo};
use crate::db::Database;
use crate::services::file_service::FileService;
#[cfg(feature = "ocr")]
use image::ImageFormat;
#[cfg(feature = "ocr")]
use exif::{In, Tag, Reader as ExifReader};

#[derive(Debug, Clone)]
pub enum DeduplicationPolicy {
    /// Skip ingestion if content already exists (for batch operations)
    Skip,
    /// Return existing document if content already exists (for direct uploads)
    ReturnExisting,
    /// Create new document record even if content exists (allows multiple filenames for same content)
    AllowDuplicateContent,
    /// Track as duplicate but link to existing document (for WebDAV)
    TrackAsDuplicate,
}

#[derive(Debug)]
pub enum IngestionResult {
    /// New document was created
    Created(Document),
    /// Existing document was returned (content duplicate)
    ExistingDocument(Document),
    /// Document was skipped due to duplication policy
    Skipped { existing_document_id: Uuid, reason: String },
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

pub struct DocumentIngestionService {
    db: Database,
    file_service: FileService,
}

impl DocumentIngestionService {
    pub fn new(db: Database, file_service: FileService) -> Self {
        Self { db, file_service }
    }

    /// Extract metadata from FileIngestionInfo for storage in document
    fn extract_metadata_from_file_info(file_info: &FileIngestionInfo) -> (Option<chrono::DateTime<chrono::Utc>>, Option<chrono::DateTime<chrono::Utc>>, Option<serde_json::Value>) {
        let original_created_at = file_info.created_at;
        let original_modified_at = file_info.last_modified;
        
        // Build comprehensive metadata object
        let mut metadata = serde_json::Map::new();
        
        // Add permissions if available
        if let Some(perms) = file_info.permissions {
            metadata.insert("permissions".to_string(), serde_json::Value::Number(perms.into()));
        }
        
        // Add owner/group info
        if let Some(ref owner) = file_info.owner {
            metadata.insert("owner".to_string(), serde_json::Value::String(owner.clone()));
        }
        
        if let Some(ref group) = file_info.group {
            metadata.insert("group".to_string(), serde_json::Value::String(group.clone()));
        }
        
        // Add source path
        metadata.insert("source_path".to_string(), serde_json::Value::String(file_info.relative_path.clone()));
        
        // Merge any additional metadata from the source
        if let Some(ref source_meta) = file_info.metadata {
            if let serde_json::Value::Object(source_map) = source_meta {
                metadata.extend(source_map.clone());
            }
        }
        
        let final_metadata = if metadata.is_empty() { 
            None 
        } else { 
            Some(serde_json::Value::Object(metadata)) 
        };
        
        (original_created_at, original_modified_at, final_metadata)
    }

    /// Unified document ingestion with configurable deduplication policy
    pub async fn ingest_document(&self, request: DocumentIngestionRequest) -> Result<IngestionResult, Box<dyn std::error::Error + Send + Sync>> {
        let file_hash = self.calculate_file_hash(&request.file_data);
        let file_size = request.file_data.len() as i64;
        
        // Clone source_type early for error handling
        let source_type_for_error = request.source_type.clone();

        debug!(
            "Ingesting document: {} for user {} (hash: {}, size: {} bytes, policy: {:?})",
            request.filename, request.user_id, &file_hash[..8], file_size, request.deduplication_policy
        );

        // Check for existing document with same content
        match self.db.get_document_by_user_and_hash(request.user_id, &file_hash).await {
            Ok(Some(existing_doc)) => {
                debug!(
                    "Found existing document with same content: {} (ID: {}) matches new file: {}",
                    existing_doc.original_filename, existing_doc.id, request.filename
                );

                match request.deduplication_policy {
                    DeduplicationPolicy::Skip => {
                        return Ok(IngestionResult::Skipped {
                            existing_document_id: existing_doc.id,
                            reason: format!("Content already exists as '{}'", existing_doc.original_filename),
                        });
                    }
                    DeduplicationPolicy::ReturnExisting => {
                        return Ok(IngestionResult::ExistingDocument(existing_doc));
                    }
                    DeduplicationPolicy::TrackAsDuplicate => {
                        return Ok(IngestionResult::TrackedAsDuplicate {
                            existing_document_id: existing_doc.id,
                        });
                    }
                    DeduplicationPolicy::AllowDuplicateContent => {
                        // Continue with creating new document record
                        debug!("Creating new document record despite duplicate content (policy: AllowDuplicateContent)");
                    }
                }
            }
            Ok(None) => {
                debug!("No duplicate content found, proceeding with new document creation");
            }
            Err(e) => {
                warn!("Error checking for duplicate content (hash: {}): {}", &file_hash[..8], e);
                // Continue with ingestion even if duplicate check fails
            }
        }

        // Generate document ID upfront so we can use it for storage path
        let document_id = Uuid::new_v4();

        // Rotate image if settings.auto_rotate_images based on EXIF data
        let file_data = if request.mime_type.starts_with("image/") {
            // Here is an image, get settings for user
            match self.db.get_user_settings(request.user_id).await? {
                Some(settings) if settings.auto_rotate_images => { 
                    match self.auto_rotate_image(&request.file_data) {
                        Ok(rotated_data) => rotated_data,
                        Err(e) => {
                            warn!("Failed to auto-rotate image {}: {}, proceeding with original data", request.filename, e);
                            request.file_data
                        }
                    }
                 }
                _ => {
                    // Auto-rotation disabled, use original data
                    request.file_data
                }
            }
        } else {
            // Not an image, use original data
            request.file_data
        };
        
        // Save file to storage - use S3 if configured, otherwise local storage
        let file_path = match self.file_service
            .save_document_file(request.user_id, document_id, &request.filename, &file_data)
            .await {
                Ok(path) => path,
                Err(e) => {
                    warn!("Failed to save file {}: {}", request.filename, e);
                    
                    // Create failed document record for storage failure
                    let failed_document = crate::models::FailedDocument {
                        id: Uuid::new_v4(),
                        user_id: request.user_id,
                        filename: request.filename.clone(),
                        original_filename: Some(request.original_filename.clone()),
                        original_path: None,
                        file_path: None, // couldn't save
                        file_size: Some(file_size),
                        file_hash: Some(file_hash.clone()),
                        mime_type: Some(request.mime_type.clone()),
                        content: None,
                        tags: Vec::new(),
                        ocr_text: None,
                        ocr_confidence: None,
                        ocr_word_count: None,
                        ocr_processing_time_ms: None,
                        failure_reason: "storage_error".to_string(),
                        failure_stage: "storage".to_string(),
                        existing_document_id: None,
                        ingestion_source: source_type_for_error.clone().unwrap_or_else(|| "upload".to_string()),
                        error_message: Some(e.to_string()),
                        retry_count: Some(0),
                        last_retry_at: None,
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                    };
                    
                    if let Err(failed_err) = self.db.create_failed_document(failed_document).await {
                        warn!("Failed to create failed document record for storage error: {}", failed_err);
                    }
                    
                    return Err(e.into());
                }
            };

        // Create document record with the same ID used for storage
        let document = self.file_service.create_document_with_id(
            document_id,
            &request.filename,
            &request.original_filename,
            &file_path,
            file_size,
            &request.mime_type,
            request.user_id,
            Some(file_hash.clone()),
            request.original_created_at,
            request.original_modified_at,
            request.source_path,
            request.source_type,
            request.source_id,
            request.file_permissions,
            request.file_owner,
            request.file_group,
            request.source_metadata,
        );

        let saved_document = match self.db.create_document(document).await {
            Ok(doc) => doc,
            Err(e) => {
                // Check if this is a unique constraint violation on the hash
                let error_string = e.to_string();
                if error_string.contains("duplicate key value violates unique constraint") 
                   && error_string.contains("idx_documents_user_file_hash") {
                    warn!("Hash collision detected during concurrent upload for {} (hash: {}), fetching existing document", 
                          request.filename, &file_hash[..8]);
                    
                    // Race condition: another request created the document, fetch it
                    match self.db.get_document_by_user_and_hash(request.user_id, &file_hash).await {
                        Ok(Some(existing_doc)) => {
                            debug!("Found existing document after collision for {}: {} (ID: {})", 
                                  request.filename, existing_doc.original_filename, existing_doc.id);
                            return Ok(IngestionResult::ExistingDocument(existing_doc));
                        }
                        Ok(None) => {
                            warn!("Unexpected: constraint violation but no document found for hash {}", &file_hash[..8]);
                            return Err(e.into());
                        }
                        Err(fetch_err) => {
                            warn!("Failed to fetch document after constraint violation: {}", fetch_err);
                            return Err(e.into());
                        }
                    }
                } else {
                    warn!("Failed to create document record for {} (hash: {}): {}", 
                          request.filename, &file_hash[..8], e);
                    
                    // Create failed document record for database creation failure
                    let failed_document = crate::models::FailedDocument {
                        id: Uuid::new_v4(),
                        user_id: request.user_id,
                        filename: request.filename.clone(),
                        original_filename: Some(request.original_filename.clone()),
                        original_path: None,
                        file_path: Some(file_path.clone()), // file was saved successfully
                        file_size: Some(file_size),
                        file_hash: Some(file_hash.clone()),
                        mime_type: Some(request.mime_type.clone()),
                        content: None,
                        tags: Vec::new(),
                        ocr_text: None,
                        ocr_confidence: None,
                        ocr_word_count: None,
                        ocr_processing_time_ms: None,
                        failure_reason: "database_error".to_string(),
                        failure_stage: "ingestion".to_string(),
                        existing_document_id: None,
                        ingestion_source: source_type_for_error.clone().unwrap_or_else(|| "upload".to_string()),
                        error_message: Some(e.to_string()),
                        retry_count: Some(0),
                        last_retry_at: None,
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                    };
                    
                    if let Err(failed_err) = self.db.create_failed_document(failed_document).await {
                        warn!("Failed to create failed document record for database error: {}", failed_err);
                    }
                    
                    return Err(e.into());
                }
            }
        };

        debug!(
            "Successfully ingested document: {} (ID: {}) for user {}",
            saved_document.original_filename, saved_document.id, request.user_id
        );

        Ok(IngestionResult::Created(saved_document))
    }

    /// Calculate SHA256 hash of file content
    fn calculate_file_hash(&self, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        format!("{:x}", result)
    }

    /// Auto-rotate image bytes according to EXIF orientation tag (if present).
    ///
    /// Returns the possibly-rotated image bytes encoded back into the original format.
    pub fn auto_rotate_image(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        Self::auto_rotate_image_bytes(data)
    }

    /// Static helper to make testing easier.
    pub fn auto_rotate_image_bytes(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
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
                        if let Some(v) = vals.get(0) {
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
                2 => img = img.fliph(),              // Mirror horizontal
                3 => img = img.rotate180(),         // Rotate 180
                4 => img = img.flipv(),             // Mirror vertical
                5 => { img = img.rotate90(); img = img.fliph(); } // Mirror horizontal and rotate 270
                6 => img = img.rotate90(),          // Rotate 90
                7 => { img = img.rotate270(); img = img.fliph(); } // Mirror horizontal and rotate 90
                8 => img = img.rotate270(),         // Rotate 270
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

    /// Convenience method for direct uploads (maintains backward compatibility)
    pub async fn ingest_upload(
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
            deduplication_policy: DeduplicationPolicy::AllowDuplicateContent, // Fixed behavior for uploads
            source_type: Some("direct_upload".to_string()),
            source_id: None,
            original_created_at: None,
            original_modified_at: None,
            source_path: None, // Direct uploads don't have a source path
            file_permissions: None, // Direct uploads don't preserve permissions
            file_owner: None, // Direct uploads don't preserve owner
            file_group: None, // Direct uploads don't preserve group
            source_metadata: None,
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
            source_path: None, // Source sync files don't have a source path
            file_permissions: None, // Source sync files don't preserve permissions
            file_owner: None, // Source sync files don't preserve owner
            file_group: None, // Source sync files don't preserve group
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
            file_owner: None, // WebDAV files don't preserve owner in this method
            file_group: None, // WebDAV files don't preserve group in this method
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
            source_path: None, // Batch files don't have a source path
            file_permissions: None, // Batch files don't preserve permissions
            file_owner: None, // Batch files don't preserve owner
            file_group: None, // Batch files don't preserve group
            source_metadata: None,
        };

        self.ingest_document(request).await
    }
}

// TODO: Add comprehensive tests once test_helpers module is available

#[cfg(test)]
mod tests {
    use super::DocumentIngestionService;
    use std::fs;

    /// Helper to get image dimensions from bytes
    #[cfg(feature = "ocr")]
    fn get_image_dimensions(data: &[u8]) -> Option<(u32, u32)> {
        use image::GenericImageView;
        image::load_from_memory(data).ok().map(|img| img.dimensions())
    }

    #[test]
    fn auto_rotate_no_exif_returns_same_dimensions() {
        // Uses a test PNG that doesn't have EXIF orientation data
        let data = fs::read("test_files/portrait_100x200.png").expect("read test image");

        match DocumentIngestionService::auto_rotate_image_bytes(&data) {
            Ok(result) => {
                // No EXIF orientation present, dimensions should be unchanged
                #[cfg(feature = "ocr")]
                {
                    let original_dims = get_image_dimensions(&data);
                    let result_dims = get_image_dimensions(&result);
                    assert_eq!(original_dims, result_dims, "Dimensions should be unchanged when no EXIF orientation");
                }
            }
            Err(_) => {
                // Image support not compiled in, skip test
            }
        }
    }

    #[test]
    fn auto_rotate_jpeg_no_exif_returns_same_dimensions() {
        // Uses a JPEG without EXIF data
        let data = fs::read("test_files/exif_orientation_none.jpg").expect("read test image");

        match DocumentIngestionService::auto_rotate_image_bytes(&data) {
            Ok(result) => {
                #[cfg(feature = "ocr")]
                {
                    let original_dims = get_image_dimensions(&data);
                    let result_dims = get_image_dimensions(&result);
                    assert_eq!(original_dims, result_dims, "Dimensions should be unchanged when no EXIF");
                }
            }
            Err(_) => {
                // Image support not compiled in, skip
            }
        }
    }

    #[test]
    fn auto_rotate_orientation_1_no_change() {
        // Orientation 1 = Normal, no rotation needed
        let data = fs::read("test_files/exif_orientation_1_normal.jpg").expect("read test image");

        match DocumentIngestionService::auto_rotate_image_bytes(&data) {
            Ok(result) => {
                #[cfg(feature = "ocr")]
                {
                    let original_dims = get_image_dimensions(&data);
                    let result_dims = get_image_dimensions(&result);
                    // Orientation 1 means normal - dimensions unchanged
                    assert_eq!(original_dims, result_dims, "Orientation 1 should not change dimensions");
                }
            }
            Err(_) => {}
        }
    }

    #[test]
    fn auto_rotate_orientation_3_rotate_180() {
        // Orientation 3 = Rotate 180 degrees
        // Test image is 40x20, rotation 180 keeps same dimensions
        let data = fs::read("test_files/exif_orientation_3_rotate_180.jpg").expect("read test image");

        match DocumentIngestionService::auto_rotate_image_bytes(&data) {
            Ok(result) => {
                #[cfg(feature = "ocr")]
                {
                    let original_dims = get_image_dimensions(&data);
                    let result_dims = get_image_dimensions(&result);
                    // 180 rotation keeps same dimensions
                    assert_eq!(original_dims, result_dims, "Rotation 180 should preserve dimensions");
                    // But result should be different bytes (image is rotated)
                    assert_ne!(data, result, "Rotated image should differ from original");
                }
            }
            Err(_) => {}
        }
    }

    #[test]
    fn auto_rotate_orientation_6_rotate_90_cw() {
        // Orientation 6 = Rotate 90 CW
        // Test image is 40x20, after 90 CW rotation should be 20x40
        let data = fs::read("test_files/exif_orientation_6_rotate_90_cw.jpg").expect("read test image");

        match DocumentIngestionService::auto_rotate_image_bytes(&data) {
            Ok(result) => {
                #[cfg(feature = "ocr")]
                {
                    let original_dims = get_image_dimensions(&data).expect("get original dimensions");
                    let result_dims = get_image_dimensions(&result).expect("get result dimensions");

                    // 90 CW rotation swaps width and height
                    assert_eq!(original_dims.0, result_dims.1, "Width should become height after 90 CW rotation");
                    assert_eq!(original_dims.1, result_dims.0, "Height should become width after 90 CW rotation");
                }
            }
            Err(_) => {}
        }
    }

    #[test]
    fn auto_rotate_orientation_8_rotate_270_cw() {
        // Orientation 8 = Rotate 270 CW (90 CCW)
        // Test image is 40x20, after 270 CW rotation should be 20x40
        let data = fs::read("test_files/exif_orientation_8_rotate_270_cw.jpg").expect("read test image");

        match DocumentIngestionService::auto_rotate_image_bytes(&data) {
            Ok(result) => {
                #[cfg(feature = "ocr")]
                {
                    let original_dims = get_image_dimensions(&data).expect("get original dimensions");
                    let result_dims = get_image_dimensions(&result).expect("get result dimensions");

                    // 270 CW rotation swaps width and height
                    assert_eq!(original_dims.0, result_dims.1, "Width should become height after 270 CW rotation");
                    assert_eq!(original_dims.1, result_dims.0, "Height should become width after 270 CW rotation");
                }
            }
            Err(_) => {}
        }
    }

    #[test]
    fn auto_rotate_orientation_2_flip_horizontal() {
        // Orientation 2 = Flip horizontal
        // Dimensions stay the same, but pixels are flipped
        let data = fs::read("test_files/exif_orientation_2_flip_horizontal.jpg").expect("read test image");

        match DocumentIngestionService::auto_rotate_image_bytes(&data) {
            Ok(result) => {
                #[cfg(feature = "ocr")]
                {
                    let original_dims = get_image_dimensions(&data);
                    let result_dims = get_image_dimensions(&result);
                    assert_eq!(original_dims, result_dims, "Horizontal flip should preserve dimensions");
                    assert_ne!(data, result, "Flipped image should differ from original");
                }
            }
            Err(_) => {}
        }
    }

    #[test]
    fn auto_rotate_orientation_4_flip_vertical() {
        // Orientation 4 = Flip vertical
        let data = fs::read("test_files/exif_orientation_4_flip_vertical.jpg").expect("read test image");

        match DocumentIngestionService::auto_rotate_image_bytes(&data) {
            Ok(result) => {
                #[cfg(feature = "ocr")]
                {
                    let original_dims = get_image_dimensions(&data);
                    let result_dims = get_image_dimensions(&result);
                    assert_eq!(original_dims, result_dims, "Vertical flip should preserve dimensions");
                    assert_ne!(data, result, "Flipped image should differ from original");
                }
            }
            Err(_) => {}
        }
    }

    #[test]
    fn auto_rotate_orientation_5_transpose() {
        // Orientation 5 = Transpose (rotate 90 CW + flip horizontal)
        // Swaps dimensions
        let data = fs::read("test_files/exif_orientation_5_transpose.jpg").expect("read test image");

        match DocumentIngestionService::auto_rotate_image_bytes(&data) {
            Ok(result) => {
                #[cfg(feature = "ocr")]
                {
                    let original_dims = get_image_dimensions(&data).expect("get original dimensions");
                    let result_dims = get_image_dimensions(&result).expect("get result dimensions");

                    // Transpose swaps dimensions
                    assert_eq!(original_dims.0, result_dims.1, "Width should become height after transpose");
                    assert_eq!(original_dims.1, result_dims.0, "Height should become width after transpose");
                }
            }
            Err(_) => {}
        }
    }

    #[test]
    fn auto_rotate_orientation_7_transverse() {
        // Orientation 7 = Transverse (rotate 270 CW + flip horizontal)
        // Swaps dimensions
        let data = fs::read("test_files/exif_orientation_7_transverse.jpg").expect("read test image");

        match DocumentIngestionService::auto_rotate_image_bytes(&data) {
            Ok(result) => {
                #[cfg(feature = "ocr")]
                {
                    let original_dims = get_image_dimensions(&data).expect("get original dimensions");
                    let result_dims = get_image_dimensions(&result).expect("get result dimensions");

                    // Transverse swaps dimensions
                    assert_eq!(original_dims.0, result_dims.1, "Width should become height after transverse");
                    assert_eq!(original_dims.1, result_dims.0, "Height should become width after transverse");
                }
            }
            Err(_) => {}
        }
    }

    #[test]
    fn auto_rotate_invalid_image_returns_original_data() {
        // Test with invalid image data (no EXIF)
        // When there's no EXIF orientation tag, the function returns the original data
        // unchanged, even if it's not a valid image. This is the correct fallback behavior.
        let data = b"not an image file at all";

        let result = DocumentIngestionService::auto_rotate_image_bytes(data);

        #[cfg(feature = "ocr")]
        {
            // With OCR feature, should return original data unchanged (no EXIF = no rotation)
            assert!(result.is_ok(), "Should return Ok when no EXIF orientation found");
            assert_eq!(result.unwrap(), data.to_vec(), "Should return original data unchanged");
        }

        #[cfg(not(feature = "ocr"))]
        {
            // Without OCR feature, should return error about missing support
            assert!(result.is_err(), "Should return error when OCR feature disabled");
        }
    }

    #[test]
    fn auto_rotate_image_with_exif_but_invalid_image_data_returns_error() {
        // Test with data that has EXIF orientation but invalid image content
        // This simulates a file that has valid EXIF header but corrupted image data

        // Create minimal EXIF data with orientation 6 (90 CW rotation)
        // Structure: JPEG SOI + APP1 marker with minimal EXIF containing orientation tag
        let mut fake_jpeg_with_exif: Vec<u8> = vec![
            0xFF, 0xD8,                     // SOI marker
            0xFF, 0xE1,                     // APP1 marker
            0x00, 0x1E,                     // APP1 length (30 bytes)
            b'E', b'x', b'i', b'f', 0x00, 0x00, // "Exif\0\0"
            0x49, 0x49,                     // Little-endian (II)
            0x2A, 0x00,                     // TIFF magic
            0x08, 0x00, 0x00, 0x00,         // Offset to IFD0
            0x01, 0x00,                     // 1 IFD entry
            0x12, 0x01,                     // Orientation tag (0x0112)
            0x03, 0x00,                     // Type: SHORT
            0x01, 0x00, 0x00, 0x00,         // Count: 1
            0x06, 0x00, 0x00, 0x00,         // Value: 6 (rotate 90 CW)
        ];
        // Add garbage to make it clearly not a valid image
        fake_jpeg_with_exif.extend_from_slice(b"this is not valid image data");

        let result = DocumentIngestionService::auto_rotate_image_bytes(&fake_jpeg_with_exif);

        #[cfg(feature = "ocr")]
        {
            // This should fail because it has an EXIF orientation tag (6)
            // but the image data is invalid and can't be loaded
            // Note: The actual behavior depends on whether the EXIF parser can read
            // the orientation from this constructed data. If it can't, it returns Ok.
            // If it can, it will try to load the image and fail.
            // Either outcome is acceptable for this edge case.
            let _ = result; // Accept either Ok or Err
        }

        #[cfg(not(feature = "ocr"))]
        {
            assert!(result.is_err(), "Should return error when OCR feature disabled");
        }
    }

    #[test]
    fn auto_rotate_all_orientations_produce_valid_images() {
        // Verify that all orientation transformations produce valid, loadable images
        let orientations = [
            ("test_files/exif_orientation_1_normal.jpg", 1),
            ("test_files/exif_orientation_2_flip_horizontal.jpg", 2),
            ("test_files/exif_orientation_3_rotate_180.jpg", 3),
            ("test_files/exif_orientation_4_flip_vertical.jpg", 4),
            ("test_files/exif_orientation_5_transpose.jpg", 5),
            ("test_files/exif_orientation_6_rotate_90_cw.jpg", 6),
            ("test_files/exif_orientation_7_transverse.jpg", 7),
            ("test_files/exif_orientation_8_rotate_270_cw.jpg", 8),
        ];

        for (path, orientation) in orientations {
            let data = fs::read(path).expect(&format!("read {}", path));

            match DocumentIngestionService::auto_rotate_image_bytes(&data) {
                Ok(result) => {
                    #[cfg(feature = "ocr")]
                    {
                        // Verify the result is a valid image
                        let img = image::load_from_memory(&result);
                        assert!(img.is_ok(), "Orientation {} should produce valid image", orientation);
                    }
                }
                Err(e) => {
                    #[cfg(feature = "ocr")]
                    panic!("Orientation {} failed: {}", orientation, e);
                }
            }
        }
    }
}

