use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::{ToSchema, IntoParams};
use serde_json;

use super::document::Document;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SearchSnippet {
    /// The snippet text content
    pub text: String,
    /// Starting character position in the original document
    pub start_offset: i32,
    /// Ending character position in the original document
    pub end_offset: i32,
    /// Ranges within the snippet that should be highlighted
    pub highlight_ranges: Vec<HighlightRange>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HighlightRange {
    /// Start position of highlight within the snippet
    pub start: i32,
    /// End position of highlight within the snippet
    pub end: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DocumentResponse {
    /// Unique identifier for the document
    pub id: Uuid,
    /// Current filename in the system
    pub filename: String,
    /// Original filename when uploaded
    pub original_filename: String,
    /// File path where the document is stored
    pub file_path: String,
    /// File size in bytes
    pub file_size: i64,
    /// MIME type of the file
    pub mime_type: String,
    /// Tags associated with the document
    pub tags: Vec<String>,
    /// Labels associated with the document
    #[serde(default)]
    pub labels: Vec<crate::routes::labels::Label>,
    /// When the document was created
    pub created_at: DateTime<Utc>,
    /// When the document was last updated
    pub updated_at: DateTime<Utc>,
    /// User who uploaded/owns the document
    pub user_id: Uuid,
    /// Username of the user who uploaded/owns the document
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub username: Option<String>,
    /// SHA256 hash of the file content
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub file_hash: Option<String>,
    /// Whether OCR text has been extracted
    pub has_ocr_text: bool,
    /// OCR confidence score (0-100, higher is better)
    pub ocr_confidence: Option<f32>,
    /// Number of words detected by OCR
    pub ocr_word_count: Option<i32>,
    /// Time taken for OCR processing in milliseconds
    pub ocr_processing_time_ms: Option<i32>,
    /// Current status of OCR processing (pending, processing, completed, failed)
    pub ocr_status: Option<String>,
    /// Original file creation timestamp from source system
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub original_created_at: Option<DateTime<Utc>>,
    /// Original file modification timestamp from source system
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub original_modified_at: Option<DateTime<Utc>>,
    /// Original path where the file was located (from source system)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source_path: Option<String>,
    /// Type of source where file was ingested from
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source_type: Option<String>,
    /// UUID of the source system/configuration
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source_id: Option<Uuid>,
    /// File permissions from source system (Unix mode bits)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub file_permissions: Option<i32>,
    /// File owner from source system
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub file_owner: Option<String>,
    /// File group from source system
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub file_group: Option<String>,
    /// Additional metadata from source system (EXIF data, PDF metadata, custom attributes, etc.)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source_metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EnhancedDocumentResponse {
    /// Unique identifier for the document
    pub id: Uuid,
    /// Current filename in the system
    pub filename: String,
    /// Original filename when uploaded
    pub original_filename: String,
    /// File size in bytes
    pub file_size: i64,
    /// MIME type of the file
    pub mime_type: String,
    /// Tags associated with the document
    pub tags: Vec<String>,
    /// When the document was created
    pub created_at: DateTime<Utc>,
    /// Whether OCR text has been extracted
    pub has_ocr_text: bool,
    /// OCR confidence score (0-100, higher is better)
    pub ocr_confidence: Option<f32>,
    /// Number of words detected by OCR
    pub ocr_word_count: Option<i32>,
    /// Time taken for OCR processing in milliseconds
    pub ocr_processing_time_ms: Option<i32>,
    /// Current status of OCR processing (pending, processing, completed, failed)
    pub ocr_status: Option<String>,
    /// Search relevance score (0-1, higher is more relevant)
    pub search_rank: Option<f32>,
    /// Text snippets showing search matches with highlights
    pub snippets: Vec<SearchSnippet>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct IgnoredFileResponse {
    pub id: Uuid,
    pub file_hash: String,
    pub filename: String,
    pub original_filename: String,
    pub file_path: String,
    pub file_size: i64,
    pub mime_type: String,
    pub source_type: Option<String>,
    pub source_path: Option<String>,
    pub source_identifier: Option<String>,
    pub ignored_at: DateTime<Utc>,
    pub ignored_by: Uuid,
    pub ignored_by_username: Option<String>,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DocumentListResponse {
    /// List of documents
    pub documents: Vec<DocumentResponse>,
    /// Total number of documents (without pagination)
    pub total: i64,
    /// Number of documents returned in this response
    pub count: i64,
    /// Pagination offset used
    pub offset: i64,
    /// Pagination limit used
    pub limit: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DocumentOcrResponse {
    /// Document ID
    #[serde(rename = "id", with = "uuid_as_string")]
    pub id: Uuid,
    /// Original filename
    pub filename: String,
    /// Whether the document has OCR text available
    pub has_ocr_text: bool,
    /// OCR text content (if available)
    pub ocr_text: Option<String>,
    /// OCR processing confidence score (0-100)
    pub ocr_confidence: Option<f32>,
    /// Current OCR processing status
    pub ocr_status: Option<String>,
    /// Time taken for OCR processing in milliseconds
    pub ocr_processing_time_ms: Option<i32>,
    /// Language detected in the document
    pub detected_language: Option<String>,
    /// Number of pages processed (for multi-page documents)
    pub pages_processed: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DocumentOperationResponse {
    /// Whether the operation was successful
    pub success: bool,
    /// Human-readable message describing the result
    pub message: String,
    /// Document ID(s) affected by the operation
    pub document_ids: Vec<Uuid>,
    /// Number of documents processed
    pub count: i64,
    /// Any warnings or additional information
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BulkDeleteResponse {
    /// Whether the operation was successful
    pub success: bool,
    /// Number of documents successfully deleted
    pub deleted_count: i64,
    /// Number of documents that failed to delete
    pub failed_count: i64,
    /// List of document IDs that were successfully deleted
    pub deleted_documents: Vec<Uuid>,
    /// List of document IDs that failed to delete
    pub failed_documents: Vec<Uuid>,
    /// Number of files successfully deleted from storage
    pub files_deleted: i64,
    /// Number of files that failed to delete from storage
    pub files_failed: i64,
    /// Any warnings or additional information
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginationInfo {
    /// Total number of items available
    pub total: i64,
    /// Number of items returned in current response
    pub count: i64,
    /// Current offset
    pub offset: i64,
    /// Current limit
    pub limit: i64,
    /// Whether there are more items available
    pub has_more: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DocumentDuplicatesResponse {
    /// List of document groups that are duplicates of each other
    pub duplicate_groups: Vec<Vec<DocumentResponse>>,
    /// Total number of duplicate documents found
    pub total_duplicates: i64,
    /// Number of duplicate groups
    pub group_count: i64,
    /// Pagination information
    pub pagination: PaginationInfo,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct IgnoredFilesQuery {
    /// Maximum number of results to return (default: 25)
    pub limit: Option<i64>,
    /// Number of results to skip for pagination (default: 0)
    pub offset: Option<i64>,
    /// Filter by source type
    pub source_type: Option<String>,
    /// Filter by source identifier (specific source)
    pub source_identifier: Option<String>,
    /// Filter by user who ignored the files
    pub ignored_by: Option<Uuid>,
    /// Search by filename
    pub filename: Option<String>,
}

impl From<Document> for DocumentResponse {
    fn from(doc: Document) -> Self {
        Self {
            id: doc.id,
            filename: doc.filename,
            original_filename: doc.original_filename,
            file_path: doc.file_path,
            file_size: doc.file_size,
            mime_type: doc.mime_type,
            tags: doc.tags,
            labels: Vec::new(), // Labels will be populated separately where needed
            created_at: doc.created_at,
            updated_at: doc.updated_at,
            user_id: doc.user_id,
            username: None, // Username will be populated separately where needed
            file_hash: doc.file_hash,
            has_ocr_text: doc.ocr_text.is_some(),
            ocr_confidence: doc.ocr_confidence,
            ocr_word_count: doc.ocr_word_count,
            ocr_processing_time_ms: doc.ocr_processing_time_ms,
            ocr_status: doc.ocr_status,
            original_created_at: doc.original_created_at,
            original_modified_at: doc.original_modified_at,
            source_path: doc.source_path,
            source_type: doc.source_type,
            source_id: doc.source_id,
            file_permissions: doc.file_permissions,
            file_owner: doc.file_owner,
            file_group: doc.file_group,
            source_metadata: doc.source_metadata,
        }
    }
}

impl From<crate::models::document::IgnoredFile> for IgnoredFileResponse {
    fn from(ignored_file: crate::models::document::IgnoredFile) -> Self {
        Self {
            id: ignored_file.id,
            file_hash: ignored_file.file_hash,
            filename: ignored_file.filename,
            original_filename: ignored_file.original_filename,
            file_path: ignored_file.file_path,
            file_size: ignored_file.file_size,
            mime_type: ignored_file.mime_type,
            source_type: ignored_file.source_type,
            source_path: ignored_file.source_path,
            source_identifier: ignored_file.source_identifier,
            ignored_at: ignored_file.ignored_at,
            ignored_by: ignored_file.ignored_by,
            ignored_by_username: None, // Will be populated separately where needed
            reason: ignored_file.reason,
            created_at: ignored_file.created_at,
        }
    }
}

mod uuid_as_string {
    use serde::{Deserialize, Deserializer, Serializer};
    use uuid::Uuid;

    pub fn serialize<S>(uuid: &Uuid, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&uuid.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Uuid, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Uuid::parse_str(&s).map_err(serde::de::Error::custom)
    }
}