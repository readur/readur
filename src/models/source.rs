use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use utoipa::ToSchema;
use serde_json;
use ts_rs::TS;

use super::responses::DocumentResponse;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema, TS)]
#[ts(export)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema, TS)]
#[ts(export)]
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

#[derive(Clone, Serialize, Deserialize, FromRow, ToSchema, TS)]
#[ts(export)]
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

#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
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

#[derive(Serialize, Deserialize, ToSchema, TS)]
#[ts(export, optional_fields)]
pub struct CreateSource {
    pub name: String,
    pub source_type: SourceType,
    pub enabled: Option<bool>,
    pub config: serde_json::Value,
}

#[derive(Serialize, Deserialize, ToSchema, TS)]
#[ts(export, optional_fields)]
pub struct UpdateSource {
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub config: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct SourceWithStats {
    pub source: SourceResponse,
    pub recent_documents: Vec<DocumentResponse>,
    pub sync_progress: Option<f32>,
}

#[derive(Clone, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct LocalFolderSourceConfig {
    pub watch_folders: Vec<String>,
    pub file_extensions: Vec<String>,
    pub auto_sync: bool,
    pub sync_interval_minutes: i32,
    pub recursive: bool,
    pub follow_symlinks: bool,
}

#[derive(Clone, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct S3SourceConfig {
    pub bucket_name: String,
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub endpoint_url: Option<String>, // For S3-compatible services
    /// S3 addressing style: Some(true) = force path-style (MinIO/RustFS),
    /// Some(false) = force virtual-hosted, None = auto-detect.
    #[serde(default)]
    pub force_path_style: Option<bool>,
    pub prefix: Option<String>,       // Optional path prefix
    pub watch_folders: Vec<String>,   // S3 prefixes to monitor
    pub file_extensions: Vec<String>,
    pub auto_sync: bool,
    pub sync_interval_minutes: i32,
}

// WebDAV-related structs
#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct WebDAVFolderInfo {
    pub path: String,
    pub total_files: i64,
    pub supported_files: i64,
    pub estimated_time_hours: f32,
    pub total_size_mb: f64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct WebDAVCrawlEstimate {
    pub folders: Vec<WebDAVFolderInfo>,
    pub total_files: i64,
    pub total_supported_files: i64,
    pub total_estimated_time_hours: f32,
    pub total_size_mb: f64,
}

#[derive(Serialize, Deserialize, ToSchema, TS)]
#[ts(export, optional_fields)]
pub struct WebDAVTestConnection {
    pub server_url: String,
    pub username: String,
    pub password: String,
    pub server_type: Option<String>, // "nextcloud", "owncloud", "generic"
}

#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct WebDAVConnectionResult {
    pub success: bool,
    pub message: String,
    pub server_version: Option<String>,
    pub server_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema, TS)]
#[ts(export)]
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

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema, TS)]
#[ts(export)]
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

#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
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

#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
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

#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
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
#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
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

#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct CreateNotification {
    pub notification_type: String,
    pub title: String,
    pub message: String,
    pub action_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
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
            config: redact_source_config(source.source_type, source.config),
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

/// Secret fields of a source configuration, paired with the boolean flag that
/// replaces each one in API responses.
fn secret_config_fields(source_type: SourceType) -> &'static [(&'static str, &'static str)] {
    match source_type {
        SourceType::WebDAV => &[("password", "has_password")],
        SourceType::S3 => &[("secret_access_key", "has_secret_access_key")],
        SourceType::LocalFolder => &[],
    }
}

/// Remove stored credentials from a source configuration before it leaves the
/// server, recording only whether each one is set.
pub fn redact_source_config(source_type: SourceType, mut config: serde_json::Value) -> serde_json::Value {
    if let Some(obj) = config.as_object_mut() {
        for (field, flag) in secret_config_fields(source_type) {
            let present = obj
                .remove(*field)
                .map(|v| v.as_str().map_or(!v.is_null(), |s| !s.is_empty()))
                .unwrap_or(false);
            obj.insert((*flag).to_string(), serde_json::Value::Bool(present));
        }
    }
    config
}

/// A stored credential may only be reused for the server and account it was
/// entered for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecretReuseRefused;

impl SecretReuseRefused {
    pub const MESSAGE: &'static str = "Re-enter the password when changing the server or account";
}

impl std::fmt::Display for SecretReuseRefused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(Self::MESSAGE)
    }
}

impl std::error::Error for SecretReuseRefused {}

/// Normalized form of a server URL for comparing configurations: protocol
/// defaulted to https, surrounding whitespace and trailing slashes removed,
/// scheme and host lower-cased.
pub fn normalize_endpoint_for_comparison(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let with_scheme = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{}", trimmed)
    };
    match url::Url::parse(&with_scheme) {
        Ok(url) => url.as_str().trim_end_matches('/').to_string(),
        Err(_) => with_scheme.trim_end_matches('/').to_string(),
    }
}

/// Fields that identify where a stored secret is used. A stored secret is
/// only carried over when all of them are unchanged.
fn secret_target(source_type: SourceType, config: &serde_json::Value) -> Vec<String> {
    let text = |field: &str| {
        config
            .get(field)
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
    };
    let endpoint = |field: &str| normalize_endpoint_for_comparison(&text(field));
    match source_type {
        SourceType::WebDAV => vec![endpoint("server_url"), text("username")],
        SourceType::S3 => vec![endpoint("endpoint_url"), text("access_key_id"), text("bucket_name")],
        SourceType::LocalFolder => Vec::new(),
    }
}

/// Prepare an incoming configuration for storage: carry over stored
/// credentials the client omitted or sent empty, and drop the response-only
/// flags. Clients never receive stored secrets, so an omitted secret means
/// "unchanged" — which is only accepted while the server and account the
/// secret belongs to are unchanged.
pub fn merge_stored_secrets(
    source_type: SourceType,
    stored: &serde_json::Value,
    mut incoming: serde_json::Value,
) -> Result<serde_json::Value, SecretReuseRefused> {
    let same_target = secret_target(source_type, stored) == secret_target(source_type, &incoming);
    if let Some(obj) = incoming.as_object_mut() {
        for (field, flag) in secret_config_fields(source_type) {
            obj.remove(*flag);
            let provided = obj.get(*field).and_then(|v| v.as_str()).is_some_and(|s| !s.is_empty());
            if provided {
                continue;
            }
            let stored_secret = stored
                .get(*field)
                .filter(|v| v.as_str().map_or(!v.is_null(), |s| !s.is_empty()));
            match stored_secret {
                Some(v) if same_target => {
                    obj.insert((*field).to_string(), v.clone());
                }
                Some(_) => return Err(SecretReuseRefused),
                // The typed configs require the field, so an unset secret
                // (anonymous WebDAV, empty S3 secret) stays an empty string.
                None => {
                    obj.insert((*field).to_string(), serde_json::Value::String(String::new()));
                }
            }
        }
    }
    Ok(incoming)
}

/// Copy of `config` with every known secret field masked, for Debug output.
fn masked_config(config: &serde_json::Value) -> serde_json::Value {
    let mut config = config.clone();
    if let Some(obj) = config.as_object_mut() {
        for source_type in [SourceType::WebDAV, SourceType::S3] {
            for (field, _) in secret_config_fields(source_type) {
                if obj.contains_key(*field) {
                    obj.insert((*field).to_string(), serde_json::Value::String("***".into()));
                }
            }
        }
    }
    config
}

impl std::fmt::Debug for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Source")
            .field("id", &self.id)
            .field("user_id", &self.user_id)
            .field("name", &self.name)
            .field("source_type", &self.source_type)
            .field("enabled", &self.enabled)
            .field("config", &masked_config(&self.config))
            .field("status", &self.status)
            .field("last_sync_at", &self.last_sync_at)
            .field("last_error", &self.last_error)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for CreateSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreateSource")
            .field("name", &self.name)
            .field("source_type", &self.source_type)
            .field("enabled", &self.enabled)
            .field("config", &masked_config(&self.config))
            .finish()
    }
}

impl std::fmt::Debug for UpdateSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UpdateSource")
            .field("name", &self.name)
            .field("enabled", &self.enabled)
            .field("config", &self.config.as_ref().map(masked_config))
            .finish()
    }
}

impl std::fmt::Debug for WebDAVSourceConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebDAVSourceConfig")
            .field("server_url", &self.server_url)
            .field("username", &self.username)
            .field("password", &"***")
            .field("watch_folders", &self.watch_folders)
            .field("file_extensions", &self.file_extensions)
            .field("auto_sync", &self.auto_sync)
            .field("sync_interval_minutes", &self.sync_interval_minutes)
            .field("server_type", &self.server_type)
            .finish()
    }
}

impl std::fmt::Debug for S3SourceConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("S3SourceConfig")
            .field("bucket_name", &self.bucket_name)
            .field("region", &self.region)
            .field("access_key_id", &self.access_key_id)
            .field("secret_access_key", &"***")
            .field("endpoint_url", &self.endpoint_url)
            .field("force_path_style", &self.force_path_style)
            .field("prefix", &self.prefix)
            .field("watch_folders", &self.watch_folders)
            .field("file_extensions", &self.file_extensions)
            .field("auto_sync", &self.auto_sync)
            .field("sync_interval_minutes", &self.sync_interval_minutes)
            .finish()
    }
}

impl std::fmt::Debug for WebDAVTestConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebDAVTestConnection")
            .field("server_url", &self.server_url)
            .field("username", &self.username)
            .field("password", &"***")
            .field("server_type", &self.server_type)
            .finish()
    }
}

#[cfg(test)]
mod secret_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn redacts_webdav_password_and_s3_secret() {
        let webdav = redact_source_config(SourceType::WebDAV, json!({"server_url": "https://x", "password": "p"}));
        assert!(webdav.get("password").is_none());
        assert_eq!(webdav["has_password"], json!(true));

        let s3 = redact_source_config(SourceType::S3, json!({"bucket_name": "b", "secret_access_key": ""}));
        assert!(s3.get("secret_access_key").is_none());
        assert_eq!(s3["has_secret_access_key"], json!(false));
    }

    #[test]
    fn merge_keeps_stored_secret_when_omitted_or_empty() {
        let stored = json!({"server_url": "https://dav.example", "username": "u", "password": "old"});
        let omitted = merge_stored_secrets(
            SourceType::WebDAV,
            &stored,
            json!({"server_url": "https://dav.example", "username": "u", "has_password": true}),
        )
        .unwrap();
        assert_eq!(omitted["password"], json!("old"));
        assert!(omitted.get("has_password").is_none());

        let empty = merge_stored_secrets(
            SourceType::WebDAV,
            &stored,
            json!({"server_url": "https://dav.example", "username": "u", "password": ""}),
        )
        .unwrap();
        assert_eq!(empty["password"], json!("old"));

        let replaced = merge_stored_secrets(SourceType::WebDAV, &stored, json!({"password": "new"})).unwrap();
        assert_eq!(replaced["password"], json!("new"));
    }

    #[test]
    fn merge_tolerates_equivalent_server_urls() {
        let stored = json!({"server_url": "https://dav.example/remote.php/dav/", "username": "u", "password": "old"});
        let merged = merge_stored_secrets(
            SourceType::WebDAV,
            &stored,
            json!({"server_url": " HTTPS://Dav.Example/remote.php/dav", "username": "u"}),
        )
        .unwrap();
        assert_eq!(merged["password"], json!("old"));

        let scheme_less = json!({"server_url": "dav.example", "username": "u", "password": "old"});
        assert!(merge_stored_secrets(
            SourceType::WebDAV,
            &scheme_less,
            json!({"server_url": "https://dav.example/", "username": "u"}),
        )
        .is_ok());
    }

    #[test]
    fn merge_refuses_stored_secret_for_a_different_target() {
        let stored = json!({"server_url": "https://dav.example", "username": "u", "password": "old"});
        for incoming in [
            json!({"server_url": "https://other.example", "username": "u"}),
            json!({"server_url": "https://dav.example", "username": "someone-else"}),
            json!({"server_url": "http://dav.example", "username": "u", "password": ""}),
        ] {
            assert_eq!(merge_stored_secrets(SourceType::WebDAV, &stored, incoming), Err(SecretReuseRefused));
        }
        // Supplying the secret again is always accepted.
        let changed = merge_stored_secrets(
            SourceType::WebDAV,
            &stored,
            json!({"server_url": "https://other.example", "username": "u", "password": "new"}),
        )
        .unwrap();
        assert_eq!(changed["password"], json!("new"));

        let s3 = json!({"endpoint_url": "http://minio:9000", "access_key_id": "AK", "bucket_name": "b", "secret_access_key": "sk"});
        let same = json!({"endpoint_url": "http://minio:9000/", "access_key_id": "AK", "bucket_name": "b"});
        assert_eq!(merge_stored_secrets(SourceType::S3, &s3, same).unwrap()["secret_access_key"], json!("sk"));
        for incoming in [
            json!({"endpoint_url": "http://other:9000", "access_key_id": "AK", "bucket_name": "b"}),
            json!({"endpoint_url": "http://minio:9000", "access_key_id": "AK2", "bucket_name": "b"}),
            json!({"endpoint_url": "http://minio:9000", "access_key_id": "AK", "bucket_name": "b2"}),
            json!({"access_key_id": "AK", "bucket_name": "b"}),
        ] {
            assert_eq!(merge_stored_secrets(SourceType::S3, &s3, incoming), Err(SecretReuseRefused));
        }
    }

    #[test]
    fn merge_without_stored_secret_is_unaffected_by_target_changes() {
        let stored = json!({"server_url": "https://dav.example", "username": "u"});
        let merged = merge_stored_secrets(
            SourceType::WebDAV,
            &stored,
            json!({"server_url": "https://other.example", "username": "v"}),
        )
        .unwrap();
        assert_eq!(merged["password"], json!(""));
    }

    #[test]
    fn merge_keeps_empty_secret_so_typed_config_still_parses() {
        let stored = json!({
            "server_url": "https://dav.example", "username": "", "password": "",
            "watch_folders": ["/"], "file_extensions": [], "auto_sync": false,
            "sync_interval_minutes": 60, "server_type": null
        });
        let merged = merge_stored_secrets(SourceType::WebDAV, &stored, stored.clone()).unwrap();
        assert_eq!(merged["password"], json!(""));
        serde_json::from_value::<WebDAVSourceConfig>(merged).expect("anonymous WebDAV config stays valid");

        let s3 = json!({
            "bucket_name": "b", "region": "us-east-1", "access_key_id": "", "secret_access_key": "",
            "endpoint_url": null, "prefix": null, "watch_folders": [], "file_extensions": [],
            "auto_sync": false, "sync_interval_minutes": 60
        });
        let mut incoming = s3.clone();
        incoming.as_object_mut().unwrap().remove("secret_access_key");
        let merged = merge_stored_secrets(SourceType::S3, &s3, incoming).unwrap();
        assert_eq!(merged["secret_access_key"], json!(""));
        serde_json::from_value::<S3SourceConfig>(merged).expect("S3 config with empty secret stays valid");
    }

    #[test]
    fn debug_output_hides_secrets() {
        let cfg = S3SourceConfig {
            bucket_name: "b".into(),
            region: "r".into(),
            access_key_id: "AKIA".into(),
            secret_access_key: "topsecret".into(),
            endpoint_url: None,
            force_path_style: None,
            prefix: None,
            watch_folders: vec![],
            file_extensions: vec![],
            auto_sync: false,
            sync_interval_minutes: 5,
        };
        assert!(!format!("{:?}", cfg).contains("topsecret"));

        let update = UpdateSource { name: None, enabled: None, config: Some(json!({"password": "hunter2"})) };
        assert!(!format!("{:?}", update).contains("hunter2"));
    }
}