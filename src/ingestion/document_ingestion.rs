/*!
 * Unified Document Ingestion Service
 * 
 * This module provides a centralized abstraction for document ingestion with
 * consistent deduplication logic across all sources (direct upload, WebDAV, 
 * source sync, batch ingest, folder watcher).
 */

use uuid::Uuid;
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};
use chrono::Utc;
use serde_json;

use crate::models::{Document, FileInfo};
use crate::db::Database;
use crate::services::file_service::FileService;

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

    /// Extract metadata from FileInfo for storage in document
    fn extract_metadata_from_file_info(file_info: &FileInfo) -> (Option<chrono::DateTime<chrono::Utc>>, Option<chrono::DateTime<chrono::Utc>>, Option<serde_json::Value>) {
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
        metadata.insert("source_path".to_string(), serde_json::Value::String(file_info.path.clone()));
        
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

        debug!(
            "Ingesting document: {} for user {} (hash: {}, size: {} bytes, policy: {:?})",
            request.filename, request.user_id, &file_hash[..8], file_size, request.deduplication_policy
        );

        // Check for existing document with same content
        match self.db.get_document_by_user_and_hash(request.user_id, &file_hash).await {
            Ok(Some(existing_doc)) => {
                info!(
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
                        info!("Creating new document record despite duplicate content (policy: AllowDuplicateContent)");
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

        // Save file to storage
        let file_path = match self.file_service
            .save_file(&request.filename, &request.file_data)
            .await {
                Ok(path) => path,
                Err(e) => {
                    warn!("Failed to save file {}: {}", request.filename, e);
                    
                    // Create failed document record for storage failure
                    if let Err(failed_err) = self.db.create_failed_document(
                        request.user_id,
                        request.filename.clone(),
                        Some(request.original_filename.clone()),
                        None, // original_path
                        None, // file_path (couldn't save)
                        Some(file_size),
                        Some(file_hash.clone()),
                        Some(request.mime_type.clone()),
                        None, // content
                        Vec::new(), // tags
                        None, // ocr_text
                        None, // ocr_confidence
                        None, // ocr_word_count
                        None, // ocr_processing_time_ms
                        "storage_error".to_string(),
                        "storage".to_string(),
                        None, // existing_document_id
                        request.source_type.unwrap_or_else(|| "upload".to_string()),
                        Some(e.to_string()),
                        None, // retry_count
                    ).await {
                        warn!("Failed to create failed document record for storage error: {}", failed_err);
                    }
                    
                    return Err(e.into());
                }
            };

        // Create document record
        let document = self.file_service.create_document(
            &request.filename,
            &request.original_filename,
            &file_path,
            file_size,
            &request.mime_type,
            request.user_id,
            Some(file_hash.clone()),
            request.original_created_at,
            request.original_modified_at,
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
                            info!("Found existing document after collision for {}: {} (ID: {})", 
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
                    if let Err(failed_err) = self.db.create_failed_document(
                        request.user_id,
                        request.filename.clone(),
                        Some(request.original_filename.clone()),
                        None, // original_path
                        Some(file_path.clone()), // file was saved successfully
                        Some(file_size),
                        Some(file_hash.clone()),
                        Some(request.mime_type.clone()),
                        None, // content
                        Vec::new(), // tags
                        None, // ocr_text
                        None, // ocr_confidence
                        None, // ocr_word_count
                        None, // ocr_processing_time_ms
                        "database_error".to_string(),
                        "ingestion".to_string(),
                        None, // existing_document_id
                        request.source_type.unwrap_or_else(|| "upload".to_string()),
                        Some(e.to_string()),
                        None, // retry_count
                    ).await {
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

    /// Ingest document from source with FileInfo metadata
    pub async fn ingest_from_file_info(
        &self,
        file_info: &FileInfo,
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
            source_metadata: None,
        };

        self.ingest_document(request).await
    }
}

// TODO: Add comprehensive tests once test_helpers module is available