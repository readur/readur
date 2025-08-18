use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;
use anyhow::Result;
use utoipa::ToSchema;

/// Generic source types that can be monitored for errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "source_type", rename_all = "lowercase")]
pub enum ErrorSourceType {
    #[sqlx(rename = "webdav")]
    WebDAV,
    #[sqlx(rename = "s3")]
    S3,
    #[sqlx(rename = "local")]
    Local,
    #[sqlx(rename = "dropbox")]
    Dropbox,
    #[sqlx(rename = "gdrive")]
    GDrive,
    #[sqlx(rename = "onedrive")]
    OneDrive,
}

impl fmt::Display for ErrorSourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorSourceType::WebDAV => write!(f, "webdav"),
            ErrorSourceType::S3 => write!(f, "s3"),
            ErrorSourceType::Local => write!(f, "local"),
            ErrorSourceType::Dropbox => write!(f, "dropbox"),
            ErrorSourceType::GDrive => write!(f, "gdrive"),
            ErrorSourceType::OneDrive => write!(f, "onedrive"),
        }
    }
}

/// Generic error types that can occur across all source types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "source_error_type", rename_all = "lowercase")]
pub enum SourceErrorType {
    #[sqlx(rename = "timeout")]
    Timeout,
    #[sqlx(rename = "permission_denied")]
    PermissionDenied,
    #[sqlx(rename = "network_error")]
    NetworkError,
    #[sqlx(rename = "server_error")]
    ServerError,
    #[sqlx(rename = "path_too_long")]
    PathTooLong,
    #[sqlx(rename = "invalid_characters")]
    InvalidCharacters,
    #[sqlx(rename = "too_many_items")]
    TooManyItems,
    #[sqlx(rename = "depth_limit")]
    DepthLimit,
    #[sqlx(rename = "size_limit")]
    SizeLimit,
    #[sqlx(rename = "xml_parse_error")]
    XmlParseError,
    #[sqlx(rename = "json_parse_error")]
    JsonParseError,
    #[sqlx(rename = "quota_exceeded")]
    QuotaExceeded,
    #[sqlx(rename = "rate_limited")]
    RateLimited,
    #[sqlx(rename = "not_found")]
    NotFound,
    #[sqlx(rename = "conflict")]
    Conflict,
    #[sqlx(rename = "unsupported_operation")]
    UnsupportedOperation,
    #[sqlx(rename = "unknown")]
    Unknown,
}

impl fmt::Display for SourceErrorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceErrorType::Timeout => write!(f, "timeout"),
            SourceErrorType::PermissionDenied => write!(f, "permission_denied"),
            SourceErrorType::NetworkError => write!(f, "network_error"),
            SourceErrorType::ServerError => write!(f, "server_error"),
            SourceErrorType::PathTooLong => write!(f, "path_too_long"),
            SourceErrorType::InvalidCharacters => write!(f, "invalid_characters"),
            SourceErrorType::TooManyItems => write!(f, "too_many_items"),
            SourceErrorType::DepthLimit => write!(f, "depth_limit"),
            SourceErrorType::SizeLimit => write!(f, "size_limit"),
            SourceErrorType::XmlParseError => write!(f, "xml_parse_error"),
            SourceErrorType::JsonParseError => write!(f, "json_parse_error"),
            SourceErrorType::QuotaExceeded => write!(f, "quota_exceeded"),
            SourceErrorType::RateLimited => write!(f, "rate_limited"),
            SourceErrorType::NotFound => write!(f, "not_found"),
            SourceErrorType::Conflict => write!(f, "conflict"),
            SourceErrorType::UnsupportedOperation => write!(f, "unsupported_operation"),
            SourceErrorType::Unknown => write!(f, "unknown"),
        }
    }
}

/// Error severity levels for determining retry strategy and user notification priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "source_error_severity", rename_all = "lowercase")]
pub enum SourceErrorSeverity {
    #[sqlx(rename = "low")]
    Low,
    #[sqlx(rename = "medium")]
    Medium,
    #[sqlx(rename = "high")]
    High,
    #[sqlx(rename = "critical")]
    Critical,
}

impl fmt::Display for SourceErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceErrorSeverity::Low => write!(f, "low"),
            SourceErrorSeverity::Medium => write!(f, "medium"),
            SourceErrorSeverity::High => write!(f, "high"),
            SourceErrorSeverity::Critical => write!(f, "critical"),
        }
    }
}

/// Retry strategies for handling failures
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetryStrategy {
    Exponential,
    Linear,
    Fixed,
}

impl fmt::Display for RetryStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RetryStrategy::Exponential => write!(f, "exponential"),
            RetryStrategy::Linear => write!(f, "linear"),
            RetryStrategy::Fixed => write!(f, "fixed"),
        }
    }
}

impl std::str::FromStr for RetryStrategy {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "exponential" => Ok(RetryStrategy::Exponential),
            "linear" => Ok(RetryStrategy::Linear),
            "fixed" => Ok(RetryStrategy::Fixed),
            _ => Err(anyhow::anyhow!("Invalid retry strategy: {}", s)),
        }
    }
}

/// Complete source scan failure record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SourceScanFailure {
    pub id: Uuid,
    pub user_id: Uuid,
    pub source_type: ErrorSourceType,
    pub source_id: Option<Uuid>,
    pub resource_path: String,
    
    // Failure classification
    pub error_type: SourceErrorType,
    pub error_severity: SourceErrorSeverity,
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
    
    // Performance metrics
    pub response_time_ms: Option<i32>,
    pub response_size_bytes: Option<i64>,
    
    // Resource characteristics
    pub resource_size_bytes: Option<i64>,
    pub resource_depth: Option<i32>,
    pub estimated_item_count: Option<i32>,
    
    // Source-specific diagnostic data
    pub diagnostic_data: serde_json::Value,
    
    // User actions
    pub user_excluded: bool,
    pub user_notes: Option<String>,
    
    // Retry configuration
    pub retry_strategy: String,
    pub max_retries: i32,
    pub retry_delay_seconds: i32,
    
    // Resolution tracking
    pub resolved: bool,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolution_method: Option<String>,
    pub resolution_notes: Option<String>,
    
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Model for creating new source scan failures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSourceScanFailure {
    pub user_id: Uuid,
    pub source_type: ErrorSourceType,
    pub source_id: Option<Uuid>,
    pub resource_path: String,
    pub error_type: SourceErrorType,
    pub error_message: String,
    pub error_code: Option<String>,
    pub http_status_code: Option<i32>,
    pub response_time_ms: Option<i32>,
    pub response_size_bytes: Option<i64>,
    pub resource_size_bytes: Option<i64>,
    pub diagnostic_data: Option<serde_json::Value>,
}

/// Response model for API endpoints with enhanced diagnostics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SourceScanFailureResponse {
    pub id: Uuid,
    pub source_type: ErrorSourceType,
    pub source_name: Option<String>, // From joined sources table
    pub resource_path: String,
    pub error_type: SourceErrorType,
    pub error_severity: SourceErrorSeverity,
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
    pub diagnostic_summary: SourceFailureDiagnostics,
}

/// Diagnostic information for helping users understand and resolve failures
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SourceFailureDiagnostics {
    pub resource_depth: Option<i32>,
    pub estimated_item_count: Option<i32>,
    pub response_time_ms: Option<i32>,
    pub response_size_mb: Option<f64>,
    pub resource_size_mb: Option<f64>,
    pub recommended_action: String,
    pub can_retry: bool,
    pub user_action_required: bool,
    pub source_specific_info: HashMap<String, serde_json::Value>,
}

/// Classification result for errors
#[derive(Debug, Clone)]
pub struct ErrorClassification {
    pub error_type: SourceErrorType,
    pub severity: SourceErrorSeverity,
    pub retry_strategy: RetryStrategy,
    pub retry_delay_seconds: u32,
    pub max_retries: u32,
    pub user_friendly_message: String,
    pub recommended_action: String,
    pub diagnostic_data: serde_json::Value,
}

/// Trait for source-specific error classification
pub trait SourceErrorClassifier: Send + Sync {
    /// Classify an error into the generic error tracking system
    fn classify_error(&self, error: &anyhow::Error, context: &ErrorContext) -> ErrorClassification;
    
    /// Get source-specific diagnostic information
    fn extract_diagnostics(&self, error: &anyhow::Error, context: &ErrorContext) -> serde_json::Value;
    
    /// Build user-friendly error message with source-specific guidance
    fn build_user_friendly_message(&self, failure: &SourceScanFailure) -> String;
    
    /// Determine if an error should be automatically retried
    fn should_retry(&self, failure: &SourceScanFailure) -> bool;
    
    /// Get the source type this classifier handles
    fn source_type(&self) -> ErrorSourceType;
}

/// Context information available during error classification
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub resource_path: String,
    pub source_id: Option<Uuid>,
    pub operation: String, // e.g., "list_directory", "read_file", "get_metadata"
    pub response_time: Option<std::time::Duration>,
    pub response_size: Option<usize>,
    pub server_type: Option<String>,
    pub server_version: Option<String>,
    pub additional_context: HashMap<String, serde_json::Value>,
}

impl ErrorContext {
    pub fn new(resource_path: String) -> Self {
        Self {
            resource_path,
            source_id: None,
            operation: "unknown".to_string(),
            response_time: None,
            response_size: None,
            server_type: None,
            server_version: None,
            additional_context: HashMap::new(),
        }
    }
    
    pub fn with_source_id(mut self, source_id: Uuid) -> Self {
        self.source_id = Some(source_id);
        self
    }
    
    pub fn with_operation(mut self, operation: String) -> Self {
        self.operation = operation;
        self
    }
    
    pub fn with_response_time(mut self, duration: std::time::Duration) -> Self {
        self.response_time = Some(duration);
        self
    }
    
    pub fn with_response_size(mut self, size: usize) -> Self {
        self.response_size = Some(size);
        self
    }
    
    pub fn with_server_info(mut self, server_type: Option<String>, server_version: Option<String>) -> Self {
        self.server_type = server_type;
        self.server_version = server_version;
        self
    }
    
    pub fn with_context(mut self, key: String, value: serde_json::Value) -> Self {
        self.additional_context.insert(key, value);
        self
    }
}

/// Statistics for source scan failures
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SourceScanFailureStats {
    pub active_failures: i64,
    pub resolved_failures: i64,
    pub excluded_resources: i64,
    pub critical_failures: i64,
    pub high_failures: i64,
    pub medium_failures: i64,
    pub low_failures: i64,
    pub ready_for_retry: i64,
    pub by_source_type: HashMap<String, i64>,
    pub by_error_type: HashMap<String, i64>,
}

/// Request model for retrying a failed resource
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RetryFailureRequest {
    pub reset_consecutive_count: Option<bool>,
    pub notes: Option<String>,
}

/// Request model for excluding a resource from scanning
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExcludeResourceRequest {
    pub reason: String,
    pub notes: Option<String>,
    pub permanent: Option<bool>,
}

/// Query parameters for listing failures
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListFailuresQuery {
    pub source_type: Option<ErrorSourceType>,
    pub source_id: Option<Uuid>,
    pub error_type: Option<SourceErrorType>,
    pub severity: Option<SourceErrorSeverity>,
    pub include_resolved: Option<bool>,
    pub include_excluded: Option<bool>,
    pub ready_for_retry: Option<bool>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl Default for ListFailuresQuery {
    fn default() -> Self {
        Self {
            source_type: None,
            source_id: None,
            error_type: None,
            severity: None,
            include_resolved: Some(false),
            include_excluded: Some(false),
            ready_for_retry: None,
            limit: Some(50),
            offset: Some(0),
        }
    }
}