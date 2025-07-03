use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use utoipa::{ToSchema, IntoParams};
use serde_json;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, ToSchema)]
pub enum UserRole {
    #[serde(rename = "admin")]
    Admin,
    #[serde(rename = "user")]
    User,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, ToSchema)]
pub enum AuthProvider {
    #[serde(rename = "local")]
    Local,
    #[serde(rename = "oidc")]
    Oidc,
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserRole::Admin => write!(f, "admin"),
            UserRole::User => write!(f, "user"),
        }
    }
}

impl TryFrom<String> for UserRole {
    type Error = String;
    
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "admin" => Ok(UserRole::Admin),
            "user" => Ok(UserRole::User),
            _ => Err(format!("Invalid user role: {}", value)),
        }
    }
}

impl std::fmt::Display for AuthProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthProvider::Local => write!(f, "local"),
            AuthProvider::Oidc => write!(f, "oidc"),
        }
    }
}

impl TryFrom<String> for AuthProvider {
    type Error = String;
    
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "local" => Ok(AuthProvider::Local),
            "oidc" => Ok(AuthProvider::Oidc),
            _ => Err(format!("Invalid auth provider: {}", value)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: Option<String>,
    #[sqlx(try_from = "String")]
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub oidc_subject: Option<String>,
    pub oidc_issuer: Option<String>,
    pub oidc_email: Option<String>,
    #[sqlx(try_from = "String")]
    pub auth_provider: AuthProvider,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateUser {
    pub username: String,
    pub email: String,
    pub password: String,
    #[serde(default = "default_user_role")]
    pub role: Option<UserRole>,
}

fn default_user_role() -> Option<UserRole> {
    Some(UserRole::User)
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserResponse,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub role: UserRole,
}

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
    /// Additional metadata from source system (permissions, attributes, EXIF data, etc.)
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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DocumentResponse {
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
    /// Labels associated with the document
    #[serde(default)]
    pub labels: Vec<crate::routes::labels::Label>,
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
    /// Original file creation timestamp from source system
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub original_created_at: Option<DateTime<Utc>>,
    /// Original file modification timestamp from source system
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub original_modified_at: Option<DateTime<Utc>>,
    /// Additional metadata from source system (permissions, attributes, etc.)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source_metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct SearchRequest {
    /// Search query text (searches both document content and OCR-extracted text)
    pub query: String,
    /// Filter by specific tags
    pub tags: Option<Vec<String>>,
    /// Filter by MIME types (e.g., "application/pdf", "image/png")
    pub mime_types: Option<Vec<String>>,
    /// Maximum number of results to return (default: 25)
    pub limit: Option<i64>,
    /// Number of results to skip for pagination (default: 0)
    pub offset: Option<i64>,
    /// Whether to include text snippets with search matches (default: true)
    pub include_snippets: Option<bool>,
    /// Length of text snippets in characters (default: 200)
    pub snippet_length: Option<i32>,
    /// Search algorithm to use (default: simple)
    pub search_mode: Option<SearchMode>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub enum SearchMode {
    /// Simple text search with basic word matching
    #[serde(rename = "simple")]
    Simple,
    /// Exact phrase matching
    #[serde(rename = "phrase")]
    Phrase,
    /// Fuzzy search using similarity matching (good for typos and partial matches)
    #[serde(rename = "fuzzy")]
    Fuzzy,
    /// Boolean search with AND, OR, NOT operators
    #[serde(rename = "boolean")]
    Boolean,
}

impl Default for SearchMode {
    fn default() -> Self {
        SearchMode::Simple
    }
}

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
pub struct SearchResponse {
    /// List of matching documents with enhanced metadata and snippets
    pub documents: Vec<EnhancedDocumentResponse>,
    /// Total number of documents matching the search criteria
    pub total: i64,
    /// Time taken to execute the search in milliseconds
    pub query_time_ms: u64,
    /// Search suggestions for query improvement
    pub suggestions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FacetItem {
    /// The facet value (e.g., mime type or tag)
    pub value: String,
    /// Number of documents with this value
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SearchFacetsResponse {
    /// MIME type facets with counts
    pub mime_types: Vec<FacetItem>,
    /// Tag facets with counts
    pub tags: Vec<FacetItem>,
}

impl From<Document> for DocumentResponse {
    fn from(doc: Document) -> Self {
        Self {
            id: doc.id,
            filename: doc.filename,
            original_filename: doc.original_filename,
            file_size: doc.file_size,
            mime_type: doc.mime_type,
            tags: doc.tags,
            labels: Vec::new(), // Labels will be populated separately where needed
            created_at: doc.created_at,
            has_ocr_text: doc.ocr_text.is_some(),
            ocr_confidence: doc.ocr_confidence,
            ocr_word_count: doc.ocr_word_count,
            ocr_processing_time_ms: doc.ocr_processing_time_ms,
            ocr_status: doc.ocr_status,
            original_created_at: doc.original_created_at,
            original_modified_at: doc.original_modified_at,
            source_metadata: doc.source_metadata,
        }
    }
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            role: user.role,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateUser {
    pub username: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Settings {
    pub id: Uuid,
    pub user_id: Uuid,
    pub ocr_language: String,
    pub concurrent_ocr_jobs: i32,
    pub ocr_timeout_seconds: i32,
    pub max_file_size_mb: i32,
    pub allowed_file_types: Vec<String>,
    pub auto_rotate_images: bool,
    pub enable_image_preprocessing: bool,
    pub search_results_per_page: i32,
    pub search_snippet_length: i32,
    pub fuzzy_search_threshold: f32,
    pub retention_days: Option<i32>,
    pub enable_auto_cleanup: bool,
    pub enable_compression: bool,
    pub memory_limit_mb: i32,
    pub cpu_priority: String,
    pub enable_background_ocr: bool,
    pub ocr_page_segmentation_mode: i32,
    pub ocr_engine_mode: i32,
    pub ocr_min_confidence: f32,
    pub ocr_dpi: i32,
    pub ocr_enhance_contrast: bool,
    pub ocr_remove_noise: bool,
    pub ocr_detect_orientation: bool,
    pub ocr_whitelist_chars: Option<String>,
    pub ocr_blacklist_chars: Option<String>,
    pub ocr_brightness_boost: f32,
    pub ocr_contrast_multiplier: f32,
    pub ocr_noise_reduction_level: i32,
    pub ocr_sharpening_strength: f32,
    pub ocr_morphological_operations: bool,
    pub ocr_adaptive_threshold_window_size: i32,
    pub ocr_histogram_equalization: bool,
    pub ocr_upscale_factor: f32,
    pub ocr_max_image_width: i32,
    pub ocr_max_image_height: i32,
    pub save_processed_images: bool,
    pub ocr_quality_threshold_brightness: f32,
    pub ocr_quality_threshold_contrast: f32,
    pub ocr_quality_threshold_noise: f32,
    pub ocr_quality_threshold_sharpness: f32,
    pub ocr_skip_enhancement: bool,
    pub webdav_enabled: bool,
    pub webdav_server_url: Option<String>,
    pub webdav_username: Option<String>,
    pub webdav_password: Option<String>,
    pub webdav_watch_folders: Vec<String>,
    pub webdav_file_extensions: Vec<String>,
    pub webdav_auto_sync: bool,
    pub webdav_sync_interval_minutes: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SettingsResponse {
    pub ocr_language: String,
    pub concurrent_ocr_jobs: i32,
    pub ocr_timeout_seconds: i32,
    pub max_file_size_mb: i32,
    pub allowed_file_types: Vec<String>,
    pub auto_rotate_images: bool,
    pub enable_image_preprocessing: bool,
    pub search_results_per_page: i32,
    pub search_snippet_length: i32,
    pub fuzzy_search_threshold: f32,
    pub retention_days: Option<i32>,
    pub enable_auto_cleanup: bool,
    pub enable_compression: bool,
    pub memory_limit_mb: i32,
    pub cpu_priority: String,
    pub enable_background_ocr: bool,
    pub ocr_page_segmentation_mode: i32,
    pub ocr_engine_mode: i32,
    pub ocr_min_confidence: f32,
    pub ocr_dpi: i32,
    pub ocr_enhance_contrast: bool,
    pub ocr_remove_noise: bool,
    pub ocr_detect_orientation: bool,
    pub ocr_whitelist_chars: Option<String>,
    pub ocr_blacklist_chars: Option<String>,
    pub ocr_brightness_boost: f32,
    pub ocr_contrast_multiplier: f32,
    pub ocr_noise_reduction_level: i32,
    pub ocr_sharpening_strength: f32,
    pub ocr_morphological_operations: bool,
    pub ocr_adaptive_threshold_window_size: i32,
    pub ocr_histogram_equalization: bool,
    pub ocr_upscale_factor: f32,
    pub ocr_max_image_width: i32,
    pub ocr_max_image_height: i32,
    pub save_processed_images: bool,
    pub ocr_quality_threshold_brightness: f32,
    pub ocr_quality_threshold_contrast: f32,
    pub ocr_quality_threshold_noise: f32,
    pub ocr_quality_threshold_sharpness: f32,
    pub ocr_skip_enhancement: bool,
    pub webdav_enabled: bool,
    pub webdav_server_url: Option<String>,
    pub webdav_username: Option<String>,
    pub webdav_password: Option<String>,
    pub webdav_watch_folders: Vec<String>,
    pub webdav_file_extensions: Vec<String>,
    pub webdav_auto_sync: bool,
    pub webdav_sync_interval_minutes: i32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateSettings {
    pub ocr_language: Option<String>,
    pub concurrent_ocr_jobs: Option<i32>,
    pub ocr_timeout_seconds: Option<i32>,
    pub max_file_size_mb: Option<i32>,
    pub allowed_file_types: Option<Vec<String>>,
    pub auto_rotate_images: Option<bool>,
    pub enable_image_preprocessing: Option<bool>,
    pub search_results_per_page: Option<i32>,
    pub search_snippet_length: Option<i32>,
    pub fuzzy_search_threshold: Option<f32>,
    pub retention_days: Option<Option<i32>>,
    pub enable_auto_cleanup: Option<bool>,
    pub enable_compression: Option<bool>,
    pub memory_limit_mb: Option<i32>,
    pub cpu_priority: Option<String>,
    pub enable_background_ocr: Option<bool>,
    pub ocr_page_segmentation_mode: Option<i32>,
    pub ocr_engine_mode: Option<i32>,
    pub ocr_min_confidence: Option<f32>,
    pub ocr_dpi: Option<i32>,
    pub ocr_enhance_contrast: Option<bool>,
    pub ocr_remove_noise: Option<bool>,
    pub ocr_detect_orientation: Option<bool>,
    pub ocr_whitelist_chars: Option<Option<String>>,
    pub ocr_blacklist_chars: Option<Option<String>>,
    pub ocr_brightness_boost: Option<f32>,
    pub ocr_contrast_multiplier: Option<f32>,
    pub ocr_noise_reduction_level: Option<i32>,
    pub ocr_sharpening_strength: Option<f32>,
    pub ocr_morphological_operations: Option<bool>,
    pub ocr_adaptive_threshold_window_size: Option<i32>,
    pub ocr_histogram_equalization: Option<bool>,
    pub ocr_upscale_factor: Option<f32>,
    pub ocr_max_image_width: Option<i32>,
    pub ocr_max_image_height: Option<i32>,
    pub save_processed_images: Option<bool>,
    pub ocr_quality_threshold_brightness: Option<f32>,
    pub ocr_quality_threshold_contrast: Option<f32>,
    pub ocr_quality_threshold_noise: Option<f32>,
    pub ocr_quality_threshold_sharpness: Option<f32>,
    pub ocr_skip_enhancement: Option<bool>,
    pub webdav_enabled: Option<bool>,
    pub webdav_server_url: Option<Option<String>>,
    pub webdav_username: Option<Option<String>>,
    pub webdav_password: Option<Option<String>>,
    pub webdav_watch_folders: Option<Vec<String>>,
    pub webdav_file_extensions: Option<Vec<String>>,
    pub webdav_auto_sync: Option<bool>,
    pub webdav_sync_interval_minutes: Option<i32>,
}

impl From<Settings> for SettingsResponse {
    fn from(settings: Settings) -> Self {
        Self {
            ocr_language: settings.ocr_language,
            concurrent_ocr_jobs: settings.concurrent_ocr_jobs,
            ocr_timeout_seconds: settings.ocr_timeout_seconds,
            max_file_size_mb: settings.max_file_size_mb,
            allowed_file_types: settings.allowed_file_types,
            auto_rotate_images: settings.auto_rotate_images,
            enable_image_preprocessing: settings.enable_image_preprocessing,
            search_results_per_page: settings.search_results_per_page,
            search_snippet_length: settings.search_snippet_length,
            fuzzy_search_threshold: settings.fuzzy_search_threshold,
            retention_days: settings.retention_days,
            enable_auto_cleanup: settings.enable_auto_cleanup,
            enable_compression: settings.enable_compression,
            memory_limit_mb: settings.memory_limit_mb,
            cpu_priority: settings.cpu_priority,
            enable_background_ocr: settings.enable_background_ocr,
            ocr_page_segmentation_mode: settings.ocr_page_segmentation_mode,
            ocr_engine_mode: settings.ocr_engine_mode,
            ocr_min_confidence: settings.ocr_min_confidence,
            ocr_dpi: settings.ocr_dpi,
            ocr_enhance_contrast: settings.ocr_enhance_contrast,
            ocr_remove_noise: settings.ocr_remove_noise,
            ocr_detect_orientation: settings.ocr_detect_orientation,
            ocr_whitelist_chars: settings.ocr_whitelist_chars,
            ocr_blacklist_chars: settings.ocr_blacklist_chars,
            ocr_brightness_boost: settings.ocr_brightness_boost,
            ocr_contrast_multiplier: settings.ocr_contrast_multiplier,
            ocr_noise_reduction_level: settings.ocr_noise_reduction_level,
            ocr_sharpening_strength: settings.ocr_sharpening_strength,
            ocr_morphological_operations: settings.ocr_morphological_operations,
            ocr_adaptive_threshold_window_size: settings.ocr_adaptive_threshold_window_size,
            ocr_histogram_equalization: settings.ocr_histogram_equalization,
            ocr_upscale_factor: settings.ocr_upscale_factor,
            ocr_max_image_width: settings.ocr_max_image_width,
            ocr_max_image_height: settings.ocr_max_image_height,
            save_processed_images: settings.save_processed_images,
            ocr_quality_threshold_brightness: settings.ocr_quality_threshold_brightness,
            ocr_quality_threshold_contrast: settings.ocr_quality_threshold_contrast,
            ocr_quality_threshold_noise: settings.ocr_quality_threshold_noise,
            ocr_quality_threshold_sharpness: settings.ocr_quality_threshold_sharpness,
            ocr_skip_enhancement: settings.ocr_skip_enhancement,
            webdav_enabled: settings.webdav_enabled,
            webdav_server_url: settings.webdav_server_url,
            webdav_username: settings.webdav_username,
            webdav_password: settings.webdav_password,
            webdav_watch_folders: settings.webdav_watch_folders,
            webdav_file_extensions: settings.webdav_file_extensions,
            webdav_auto_sync: settings.webdav_auto_sync,
            webdav_sync_interval_minutes: settings.webdav_sync_interval_minutes,
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id: Uuid::nil(),
            ocr_language: "eng".to_string(),
            concurrent_ocr_jobs: 4,
            ocr_timeout_seconds: 300,
            max_file_size_mb: 50,
            allowed_file_types: vec![
                "pdf".to_string(),
                "png".to_string(),
                "jpg".to_string(),
                "jpeg".to_string(),
                "tiff".to_string(),
                "bmp".to_string(),
                "txt".to_string(),
            ],
            auto_rotate_images: true,
            enable_image_preprocessing: false,
            search_results_per_page: 25,
            search_snippet_length: 200,
            fuzzy_search_threshold: 0.8,
            retention_days: None,
            enable_auto_cleanup: false,
            enable_compression: false,
            memory_limit_mb: 512,
            cpu_priority: "normal".to_string(),
            enable_background_ocr: true,
            ocr_page_segmentation_mode: 3, // PSM_AUTO_OSD - Fully automatic page segmentation, but no OSD
            ocr_engine_mode: 3, // OEM_DEFAULT - Default, based on what is available
            ocr_min_confidence: 30.0, // Minimum confidence threshold (0-100)
            ocr_dpi: 300, // Optimal DPI for OCR
            ocr_enhance_contrast: true, // Enable contrast enhancement
            ocr_remove_noise: true, // Enable noise removal
            ocr_detect_orientation: true, // Enable orientation detection
            ocr_whitelist_chars: None, // No character whitelist by default
            ocr_blacklist_chars: None, // No character blacklist by default
            ocr_brightness_boost: 1.0, // Conservative brightness boost
            ocr_contrast_multiplier: 1.2, // Conservative contrast enhancement
            ocr_noise_reduction_level: 1, // Light noise reduction
            ocr_sharpening_strength: 0.5, // Light sharpening
            ocr_morphological_operations: false, // Conservative - no morphological ops by default
            ocr_adaptive_threshold_window_size: 15, // Small window for adaptive threshold
            ocr_histogram_equalization: false, // Conservative - no histogram equalization by default
            ocr_upscale_factor: 1.0, // No upscaling by default
            ocr_max_image_width: 3000, // Reasonable max width
            ocr_max_image_height: 3000, // Reasonable max height
            save_processed_images: false, // Conservative - don't save by default
            ocr_quality_threshold_brightness: 0.3, // Conservative threshold
            ocr_quality_threshold_contrast: 0.2, // Conservative threshold
            ocr_quality_threshold_noise: 0.7, // Conservative threshold
            ocr_quality_threshold_sharpness: 0.3, // Conservative threshold
            ocr_skip_enhancement: false, // Allow enhancement by default
            webdav_enabled: false,
            webdav_server_url: None,
            webdav_username: None,
            webdav_password: None,
            webdav_watch_folders: vec!["/Documents".to_string()],
            webdav_file_extensions: vec![
                "pdf".to_string(),
                "png".to_string(),
                "jpg".to_string(),
                "jpeg".to_string(),
                "tiff".to_string(),
                "bmp".to_string(),
                "txt".to_string(),
            ],
            webdav_auto_sync: false,
            webdav_sync_interval_minutes: 60,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WebDAVFolderInfo {
    pub path: String,
    pub total_files: i64,
    pub supported_files: i64,
    pub estimated_time_hours: f32,
    pub total_size_mb: f64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WebDAVCrawlEstimate {
    pub folders: Vec<WebDAVFolderInfo>,
    pub total_files: i64,
    pub total_supported_files: i64,
    pub total_estimated_time_hours: f32,
    pub total_size_mb: f64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WebDAVTestConnection {
    pub server_url: String,
    pub username: String,
    pub password: String,
    pub server_type: Option<String>, // "nextcloud", "owncloud", "generic"
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WebDAVConnectionResult {
    pub success: bool,
    pub message: String,
    pub server_version: Option<String>,
    pub server_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WebDAVSyncStatus {
    pub is_running: bool,
    pub last_sync: Option<DateTime<Utc>>,
    pub files_processed: i64,
    pub files_remaining: i64,
    pub current_folder: Option<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Notification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub notification_type: String,
    pub title: String,
    pub message: String,
    pub read: bool,
    pub action_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateNotification {
    pub notification_type: String,
    pub title: String,
    pub message: String,
    pub action_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NotificationSummary {
    pub unread_count: i64,
    pub recent_notifications: Vec<Notification>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebDAVSyncState {
    pub id: Uuid,
    pub user_id: Uuid,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub sync_cursor: Option<String>,
    pub is_running: bool,
    pub files_processed: i64,
    pub files_remaining: i64,
    pub current_folder: Option<String>,
    pub errors: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateWebDAVSyncState {
    pub last_sync_at: Option<DateTime<Utc>>,
    pub sync_cursor: Option<String>,
    pub is_running: bool,
    pub files_processed: i64,
    pub files_remaining: i64,
    pub current_folder: Option<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebDAVFile {
    pub id: Uuid,
    pub user_id: Uuid,
    pub webdav_path: String,
    pub etag: String,
    pub last_modified: Option<DateTime<Utc>>,
    pub file_size: i64,
    pub mime_type: String,
    pub document_id: Option<Uuid>,
    pub sync_status: String,
    pub sync_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateWebDAVFile {
    pub user_id: Uuid,
    pub webdav_path: String,
    pub etag: String,
    pub last_modified: Option<DateTime<Utc>>,
    pub file_size: i64,
    pub mime_type: String,
    pub document_id: Option<Uuid>,
    pub sync_status: String,
    pub sync_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FileInfo {
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

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct WebDAVDirectory {
    pub id: Uuid,
    pub user_id: Uuid,
    pub directory_path: String,
    pub directory_etag: String,
    pub last_scanned_at: DateTime<Utc>,
    pub file_count: i64,
    pub total_size_bytes: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateWebDAVDirectory {
    pub user_id: Uuid,
    pub directory_path: String,
    pub directory_etag: String,
    pub file_count: i64,
    pub total_size_bytes: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateWebDAVDirectory {
    pub directory_etag: String,
    pub last_scanned_at: DateTime<Utc>,
    pub file_count: i64,
    pub total_size_bytes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub enum SourceType {
    #[serde(rename = "webdav")]
    WebDAV,
    #[serde(rename = "local_folder")]
    LocalFolder,
    #[serde(rename = "s3")]
    S3,
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceType::WebDAV => write!(f, "webdav"),
            SourceType::LocalFolder => write!(f, "local_folder"),
            SourceType::S3 => write!(f, "s3"),
        }
    }
}

impl TryFrom<String> for SourceType {
    type Error = String;
    
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "webdav" => Ok(SourceType::WebDAV),
            "local_folder" => Ok(SourceType::LocalFolder),
            "s3" => Ok(SourceType::S3),
            _ => Err(format!("Invalid source type: {}", value)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub enum SourceStatus {
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "syncing")]
    Syncing,
    #[serde(rename = "error")]
    Error,
}

impl std::fmt::Display for SourceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceStatus::Idle => write!(f, "idle"),
            SourceStatus::Syncing => write!(f, "syncing"),
            SourceStatus::Error => write!(f, "error"),
        }
    }
}

impl TryFrom<String> for SourceStatus {
    type Error = String;
    
    fn try_from(value: String) -> Result<Self, <SourceStatus as TryFrom<String>>::Error> {
        match value.as_str() {
            "idle" => Ok(SourceStatus::Idle),
            "syncing" => Ok(SourceStatus::Syncing),
            "error" => Ok(SourceStatus::Error),
            _ => Err(format!("Invalid source status: {}", value)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Source {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    #[sqlx(try_from = "String")]
    pub source_type: SourceType,
    pub enabled: bool,
    pub config: serde_json::Value,
    #[sqlx(try_from = "String")]
    pub status: SourceStatus,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub last_error_at: Option<DateTime<Utc>>,
    pub total_files_synced: i64,
    pub total_files_pending: i64,
    pub total_size_bytes: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SourceResponse {
    pub id: Uuid,
    pub name: String,
    pub source_type: SourceType,
    pub enabled: bool,
    pub config: serde_json::Value,
    pub status: SourceStatus,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub last_error_at: Option<DateTime<Utc>>,
    pub total_files_synced: i64,
    pub total_files_pending: i64,
    pub total_size_bytes: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Total number of documents/files currently stored from this source
    #[serde(default)]
    pub total_documents: i64,
    /// Total number of documents that have been OCR'd from this source
    #[serde(default)]
    pub total_documents_ocr: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateSource {
    pub name: String,
    pub source_type: SourceType,
    pub enabled: Option<bool>,
    pub config: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateSource {
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub config: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SourceWithStats {
    pub source: SourceResponse,
    pub recent_documents: Vec<DocumentResponse>,
    pub sync_progress: Option<f32>,
}

impl From<Source> for SourceResponse {
    fn from(source: Source) -> Self {
        Self {
            id: source.id,
            name: source.name,
            source_type: source.source_type,
            enabled: source.enabled,
            config: source.config,
            status: source.status,
            last_sync_at: source.last_sync_at,
            last_error: source.last_error,
            last_error_at: source.last_error_at,
            total_files_synced: source.total_files_synced,
            total_files_pending: source.total_files_pending,
            total_size_bytes: source.total_size_bytes,
            created_at: source.created_at,
            updated_at: source.updated_at,
            // These will be populated separately when needed
            total_documents: 0,
            total_documents_ocr: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebDAVSourceConfig {
    pub server_url: String,
    pub username: String,
    pub password: String,
    pub watch_folders: Vec<String>,
    pub file_extensions: Vec<String>,
    pub auto_sync: bool,
    pub sync_interval_minutes: i32,
    pub server_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LocalFolderSourceConfig {
    pub watch_folders: Vec<String>,
    pub file_extensions: Vec<String>,
    pub auto_sync: bool,
    pub sync_interval_minutes: i32,
    pub recursive: bool,
    pub follow_symlinks: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct S3SourceConfig {
    pub bucket_name: String,
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub endpoint_url: Option<String>, // For S3-compatible services
    pub prefix: Option<String>,       // Optional path prefix
    pub watch_folders: Vec<String>,   // S3 prefixes to monitor
    pub file_extensions: Vec<String>,
    pub auto_sync: bool,
    pub sync_interval_minutes: i32,
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

impl From<IgnoredFile> for IgnoredFileResponse {
    fn from(ignored_file: IgnoredFile) -> Self {
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

// Additional response schemas for better API documentation

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
    pub document_id: Uuid,
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