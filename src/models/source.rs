use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use utoipa::ToSchema;
use serde_json;

use super::responses::DocumentResponse;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
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
    // Validation status tracking
    #[sqlx(default)]
    pub validation_status: Option<String>,
    #[sqlx(default)]
    pub last_validation_at: Option<DateTime<Utc>>,
    #[sqlx(default)]
    pub validation_score: Option<i32>, // 0-100 health score
    #[sqlx(default)]
    pub validation_issues: Option<String>, // JSON array of validation issues
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
    /// Validation status and health score
    #[serde(default)]
    pub validation_status: Option<String>,
    #[serde(default)]
    pub last_validation_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub validation_score: Option<i32>,
    #[serde(default)]
    pub validation_issues: Option<String>,
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

// WebDAV-related structs
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

// WebDAV Scan Failure Tracking Models

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WebDAVScanFailureType {
    Timeout,
    PathTooLong,
    PermissionDenied,
    InvalidCharacters,
    NetworkError,
    ServerError,
    XmlParseError,
    TooManyItems,
    DepthLimit,
    SizeLimit,
    Unknown,
}

impl std::fmt::Display for WebDAVScanFailureType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => write!(f, "timeout"),
            Self::PathTooLong => write!(f, "path_too_long"),
            Self::PermissionDenied => write!(f, "permission_denied"),
            Self::InvalidCharacters => write!(f, "invalid_characters"),
            Self::NetworkError => write!(f, "network_error"),
            Self::ServerError => write!(f, "server_error"),
            Self::XmlParseError => write!(f, "xml_parse_error"),
            Self::TooManyItems => write!(f, "too_many_items"),
            Self::DepthLimit => write!(f, "depth_limit"),
            Self::SizeLimit => write!(f, "size_limit"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

impl TryFrom<String> for WebDAVScanFailureType {
    type Error = String;
    
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "timeout" => Ok(Self::Timeout),
            "path_too_long" => Ok(Self::PathTooLong),
            "permission_denied" => Ok(Self::PermissionDenied),
            "invalid_characters" => Ok(Self::InvalidCharacters),
            "network_error" => Ok(Self::NetworkError),
            "server_error" => Ok(Self::ServerError),
            "xml_parse_error" => Ok(Self::XmlParseError),
            "too_many_items" => Ok(Self::TooManyItems),
            "depth_limit" => Ok(Self::DepthLimit),
            "size_limit" => Ok(Self::SizeLimit),
            "unknown" => Ok(Self::Unknown),
            _ => Err(format!("Invalid WebDAV scan failure type: {}", value)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WebDAVScanFailureSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for WebDAVScanFailureSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

impl TryFrom<String> for WebDAVScanFailureSeverity {
    type Error = String;
    
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "critical" => Ok(Self::Critical),
            _ => Err(format!("Invalid WebDAV scan failure severity: {}", value)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct WebDAVScanFailure {
    pub id: Uuid,
    pub user_id: Uuid,
    pub directory_path: String,
    
    // Failure tracking
    #[sqlx(try_from = "String")]
    pub failure_type: WebDAVScanFailureType,
    #[sqlx(try_from = "String")]
    pub failure_severity: WebDAVScanFailureSeverity,
    pub failure_count: i32,
    pub consecutive_failures: i32,
    
    // Timestamps
    pub first_failure_at: DateTime<Utc>,
    pub last_failure_at: DateTime<Utc>,
    pub last_retry_at: Option<DateTime<Utc>>,
    pub next_retry_at: Option<DateTime<Utc>>,
    
    // Error details
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub http_status_code: Option<i32>,
    
    // Diagnostic information
    pub response_time_ms: Option<i32>,
    pub response_size_bytes: Option<i64>,
    pub path_length: Option<i32>,
    pub directory_depth: Option<i32>,
    pub estimated_item_count: Option<i32>,
    pub server_type: Option<String>,
    pub server_version: Option<String>,
    
    // Additional context
    pub diagnostic_data: Option<serde_json::Value>,
    
    // User actions
    pub user_excluded: bool,
    pub user_notes: Option<String>,
    
    // Retry strategy
    pub retry_strategy: Option<String>,
    pub max_retries: i32,
    pub retry_delay_seconds: i32,
    
    // Resolution tracking
    pub resolved: bool,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolution_method: Option<String>,
    
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateWebDAVScanFailure {
    pub user_id: Uuid,
    pub directory_path: String,
    pub failure_type: WebDAVScanFailureType,
    pub error_message: String,
    pub error_code: Option<String>,
    pub http_status_code: Option<i32>,
    pub response_time_ms: Option<i32>,
    pub response_size_bytes: Option<i64>,
    pub diagnostic_data: Option<serde_json::Value>,
    pub server_type: Option<String>,
    pub server_version: Option<String>,
    pub estimated_item_count: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WebDAVScanFailureResponse {
    pub id: Uuid,
    pub directory_path: String,
    pub failure_type: WebDAVScanFailureType,
    pub failure_severity: WebDAVScanFailureSeverity,
    pub failure_count: i32,
    pub consecutive_failures: i32,
    pub first_failure_at: DateTime<Utc>,
    pub last_failure_at: DateTime<Utc>,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub http_status_code: Option<i32>,
    pub user_excluded: bool,
    pub user_notes: Option<String>,
    pub resolved: bool,
    pub diagnostic_summary: WebDAVFailureDiagnostics,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WebDAVFailureDiagnostics {
    pub path_length: Option<i32>,
    pub directory_depth: Option<i32>,
    pub estimated_item_count: Option<i32>,
    pub response_time_ms: Option<i32>,
    pub response_size_mb: Option<f64>,
    pub server_type: Option<String>,
    pub recommended_action: String,
    pub can_retry: bool,
    pub user_action_required: bool,
}

// Notification-related structs
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
            // Validation fields
            validation_status: source.validation_status,
            last_validation_at: source.last_validation_at,
            validation_score: source.validation_score,
            validation_issues: source.validation_issues,
        }
    }
}