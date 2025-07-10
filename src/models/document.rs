use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use utoipa::ToSchema;
use serde_json;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Document {
    pub id: Uuid,
    pub filename: String,
    pub original_filename: String,
    pub file_path: String,
    pub file_size: i64,
    pub mime_type: String,
    pub content: Option<String>,
    pub ocr_text: Option<String>,
    pub ocr_confidence: Option<f32>,
    pub ocr_word_count: Option<i32>,
    pub ocr_processing_time_ms: Option<i32>,
    pub ocr_status: Option<String>,
    pub ocr_error: Option<String>,
    pub ocr_completed_at: Option<DateTime<Utc>>,
    pub ocr_retry_count: Option<i32>,
    pub ocr_failure_reason: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub user_id: Uuid,
    pub file_hash: Option<String>,
    /// Original file creation timestamp from source system
    pub original_created_at: Option<DateTime<Utc>>,
    /// Original file modification timestamp from source system
    pub original_modified_at: Option<DateTime<Utc>>,
    /// Original path where the file was located (from source system)
    pub source_path: Option<String>,
    /// Type of source where file was ingested from (e.g., "web_upload", "filesystem", "webdav")
    pub source_type: Option<String>,
    /// UUID of the source system/configuration
    pub source_id: Option<Uuid>,
    /// File permissions from source system (Unix mode bits)
    pub file_permissions: Option<i32>,
    /// File owner from source system (username or uid)
    pub file_owner: Option<String>,
    /// File group from source system (groupname or gid)
    pub file_group: Option<String>,
    /// Additional metadata from source system (EXIF data, PDF metadata, custom attributes, etc.)
    pub source_metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, ToSchema)]
pub enum FailureReason {
    #[serde(rename = "duplicate_content")]
    DuplicateContent,
    #[serde(rename = "duplicate_filename")]
    DuplicateFilename,
    #[serde(rename = "unsupported_format")]
    UnsupportedFormat,
    #[serde(rename = "file_too_large")]
    FileTooLarge,
    #[serde(rename = "file_corrupted")]
    FileCorrupted,
    #[serde(rename = "access_denied")]
    AccessDenied,
    #[serde(rename = "low_ocr_confidence")]
    LowOcrConfidence,
    #[serde(rename = "ocr_timeout")]
    OcrTimeout,
    #[serde(rename = "ocr_memory_limit")]
    OcrMemoryLimit,
    #[serde(rename = "pdf_parsing_error")]
    PdfParsingError,
    #[serde(rename = "storage_quota_exceeded")]
    StorageQuotaExceeded,
    #[serde(rename = "network_error")]
    NetworkError,
    #[serde(rename = "permission_denied")]
    PermissionDenied,
    #[serde(rename = "virus_detected")]
    VirusDetected,
    #[serde(rename = "invalid_structure")]
    InvalidStructure,
    #[serde(rename = "policy_violation")]
    PolicyViolation,
    #[serde(rename = "other")]
    Other,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, ToSchema)]
pub enum FailureStage {
    #[serde(rename = "ingestion")]
    Ingestion,
    #[serde(rename = "validation")]
    Validation,
    #[serde(rename = "ocr")]
    Ocr,
    #[serde(rename = "storage")]
    Storage,
    #[serde(rename = "processing")]
    Processing,
    #[serde(rename = "sync")]
    Sync,
}

impl std::fmt::Display for FailureReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FailureReason::DuplicateContent => write!(f, "duplicate_content"),
            FailureReason::DuplicateFilename => write!(f, "duplicate_filename"),
            FailureReason::UnsupportedFormat => write!(f, "unsupported_format"),
            FailureReason::FileTooLarge => write!(f, "file_too_large"),
            FailureReason::FileCorrupted => write!(f, "file_corrupted"),
            FailureReason::AccessDenied => write!(f, "access_denied"),
            FailureReason::LowOcrConfidence => write!(f, "low_ocr_confidence"),
            FailureReason::OcrTimeout => write!(f, "ocr_timeout"),
            FailureReason::OcrMemoryLimit => write!(f, "ocr_memory_limit"),
            FailureReason::PdfParsingError => write!(f, "pdf_parsing_error"),
            FailureReason::StorageQuotaExceeded => write!(f, "storage_quota_exceeded"),
            FailureReason::NetworkError => write!(f, "network_error"),
            FailureReason::PermissionDenied => write!(f, "permission_denied"),
            FailureReason::VirusDetected => write!(f, "virus_detected"),
            FailureReason::InvalidStructure => write!(f, "invalid_structure"),
            FailureReason::PolicyViolation => write!(f, "policy_violation"),
            FailureReason::Other => write!(f, "other"),
        }
    }
}

impl std::fmt::Display for FailureStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FailureStage::Ingestion => write!(f, "ingestion"),
            FailureStage::Validation => write!(f, "validation"),
            FailureStage::Ocr => write!(f, "ocr"),
            FailureStage::Storage => write!(f, "storage"),
            FailureStage::Processing => write!(f, "processing"),
            FailureStage::Sync => write!(f, "sync"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct FailedDocument {
    /// Unique identifier for the failed document record
    pub id: Uuid,
    /// User who attempted to ingest the document
    pub user_id: Uuid,
    /// Filename of the failed document
    pub filename: String,
    /// Original filename when uploaded
    pub original_filename: Option<String>,
    /// Original path where the file was located
    pub original_path: Option<String>,
    /// Stored file path (if file was saved before failure)
    pub file_path: Option<String>,
    /// Size of the file in bytes
    pub file_size: Option<i64>,
    /// SHA256 hash of the file content
    pub file_hash: Option<String>,
    /// MIME type of the file
    pub mime_type: Option<String>,
    /// Raw content if extracted before failure
    pub content: Option<String>,
    /// Tags that were assigned/detected
    pub tags: Vec<String>,
    /// Partial OCR text if extracted before failure
    pub ocr_text: Option<String>,
    /// OCR confidence if calculated
    pub ocr_confidence: Option<f32>,
    /// Word count if calculated
    pub ocr_word_count: Option<i32>,
    /// Processing time before failure in milliseconds
    pub ocr_processing_time_ms: Option<i32>,
    /// Reason why the document failed
    pub failure_reason: String,
    /// Stage at which the document failed
    pub failure_stage: String,
    /// Reference to existing document if failed due to duplicate
    pub existing_document_id: Option<Uuid>,
    /// Source of the ingestion attempt
    pub ingestion_source: String,
    /// Detailed error message
    pub error_message: Option<String>,
    /// Number of retry attempts
    pub retry_count: Option<i32>,
    /// Last retry timestamp
    pub last_retry_at: Option<DateTime<Utc>>,
    /// When the document failed
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ProcessedImage {
    pub id: Uuid,
    pub document_id: Uuid,
    pub user_id: Uuid,
    pub original_image_path: String,
    pub processed_image_path: String,
    pub processing_parameters: serde_json::Value,
    pub processing_steps: Vec<String>,
    pub image_width: i32,
    pub image_height: i32,
    pub file_size: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateProcessedImage {
    pub document_id: Uuid,
    pub user_id: Uuid,
    pub original_image_path: String,
    pub processed_image_path: String,
    pub processing_parameters: serde_json::Value,
    pub processing_steps: Vec<String>,
    pub image_width: i32,
    pub image_height: i32,
    pub file_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct IgnoredFile {
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
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateIgnoredFile {
    pub file_hash: String,
    pub filename: String,
    pub original_filename: String,
    pub file_path: String,
    pub file_size: i64,
    pub mime_type: String,
    pub source_type: Option<String>,
    pub source_path: Option<String>,
    pub source_identifier: Option<String>,
    pub ignored_by: Uuid,
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FileIngestionInfo {
    pub path: String,
    pub name: String,
    pub size: i64,
    pub mime_type: String,
    pub last_modified: Option<DateTime<Utc>>,
    pub etag: String,
    pub is_directory: bool,
    /// Original file creation time from source system
    pub created_at: Option<DateTime<Utc>>,
    /// File permissions (Unix mode bits or similar)
    pub permissions: Option<u32>,
    /// File owner (username or uid)
    pub owner: Option<String>,
    /// File group (groupname or gid)
    pub group: Option<String>,
    /// Additional metadata from source (EXIF, PDF metadata, custom attributes, etc.)
    pub metadata: Option<serde_json::Value>,
}