use axum::http::StatusCode;
use thiserror::Error;
use uuid::Uuid;

use super::{AppError, ErrorCategory, ErrorSeverity, impl_into_response};

/// Errors related to file source operations (WebDAV, Local Folder, S3)
#[derive(Error, Debug)]
pub enum SourceError {
    #[error("Source not found")]
    NotFound,
    
    #[error("Source with ID {id} not found")]
    NotFoundById { id: Uuid },
    
    #[error("Source name '{name}' already exists")]
    DuplicateName { name: String },
    
    #[error("Invalid source path: {path}")]
    InvalidPath { path: String },
    
    #[error("Connection failed: {details}")]
    ConnectionFailed { details: String },
    
    #[error("Authentication failed for source '{name}': {reason}")]
    AuthenticationFailed { name: String, reason: String },
    
    #[error("Sync operation already in progress for source '{name}'")]
    SyncInProgress { name: String },
    
    #[error("Invalid source configuration: {details}")]
    ConfigurationInvalid { details: String },
    
    #[error("Access denied to path '{path}': {reason}")]
    AccessDenied { path: String, reason: String },
    
    #[error("Source '{name}' is disabled")]
    SourceDisabled { name: String },
    
    #[error("Invalid source type '{source_type}'. Valid types are: webdav, local_folder, s3")]
    InvalidSourceType { source_type: String },
    
    #[error("Network timeout connecting to '{url}' after {timeout_seconds} seconds")]
    NetworkTimeout { url: String, timeout_seconds: u64 },
    
    #[error("Source capacity exceeded: {details}")]
    CapacityExceeded { details: String },
    
    #[error("Server error from '{server}': {error_code} - {message}")]
    ServerError { server: String, error_code: u16, message: String },
    
    #[error("SSL/TLS certificate error for '{server}': {details}")]
    CertificateError { server: String, details: String },
    
    #[error("Unsupported server version '{version}' for source type '{source_type}'")]
    UnsupportedServerVersion { version: String, source_type: String },
    
    #[error("Rate limit exceeded for source '{name}': {retry_after_seconds} seconds until retry")]
    RateLimitExceeded { name: String, retry_after_seconds: u64 },
    
    #[error("File not found: {path}")]
    FileNotFound { path: String },
    
    #[error("Directory not found: {path}")]
    DirectoryNotFound { path: String },
    
    #[error("Validation failed: {issues}")]
    ValidationFailed { issues: String },
    
    #[error("Sync operation failed: {reason}")]
    SyncFailed { reason: String },
    
    #[error("Cannot delete source '{name}': {reason}")]
    DeleteRestricted { name: String, reason: String },
}

impl AppError for SourceError {
    fn status_code(&self) -> StatusCode {
        match self {
            SourceError::NotFound | SourceError::NotFoundById { .. } => StatusCode::NOT_FOUND,
            SourceError::DuplicateName { .. } => StatusCode::CONFLICT,
            SourceError::InvalidPath { .. } => StatusCode::BAD_REQUEST,
            SourceError::ConnectionFailed { .. } => StatusCode::BAD_GATEWAY,
            SourceError::AuthenticationFailed { .. } => StatusCode::UNAUTHORIZED,
            SourceError::SyncInProgress { .. } => StatusCode::CONFLICT,
            SourceError::ConfigurationInvalid { .. } => StatusCode::BAD_REQUEST,
            SourceError::AccessDenied { .. } => StatusCode::FORBIDDEN,
            SourceError::SourceDisabled { .. } => StatusCode::FORBIDDEN,
            SourceError::InvalidSourceType { .. } => StatusCode::BAD_REQUEST,
            SourceError::NetworkTimeout { .. } => StatusCode::GATEWAY_TIMEOUT,
            SourceError::CapacityExceeded { .. } => StatusCode::INSUFFICIENT_STORAGE,
            SourceError::ServerError { .. } => StatusCode::BAD_GATEWAY,
            SourceError::CertificateError { .. } => StatusCode::BAD_GATEWAY,
            SourceError::UnsupportedServerVersion { .. } => StatusCode::NOT_IMPLEMENTED,
            SourceError::RateLimitExceeded { .. } => StatusCode::TOO_MANY_REQUESTS,
            SourceError::FileNotFound { .. } => StatusCode::NOT_FOUND,
            SourceError::DirectoryNotFound { .. } => StatusCode::NOT_FOUND,
            SourceError::ValidationFailed { .. } => StatusCode::BAD_REQUEST,
            SourceError::SyncFailed { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            SourceError::DeleteRestricted { .. } => StatusCode::CONFLICT,
        }
    }
    
    fn user_message(&self) -> String {
        match self {
            SourceError::NotFound | SourceError::NotFoundById { .. } => "Source not found".to_string(),
            SourceError::DuplicateName { .. } => "A source with this name already exists".to_string(),
            SourceError::InvalidPath { .. } => "Invalid file path specified".to_string(),
            SourceError::ConnectionFailed { .. } => "Unable to connect to the source".to_string(),
            SourceError::AuthenticationFailed { .. } => "Authentication failed - please check credentials".to_string(),
            SourceError::SyncInProgress { .. } => "Sync operation is already running".to_string(),
            SourceError::ConfigurationInvalid { details } => format!("Invalid configuration: {}", details),
            SourceError::AccessDenied { .. } => "Access denied to the specified path".to_string(),
            SourceError::SourceDisabled { .. } => "Source is currently disabled".to_string(),
            SourceError::InvalidSourceType { .. } => "Invalid source type specified".to_string(),
            SourceError::NetworkTimeout { .. } => "Connection timed out".to_string(),
            SourceError::CapacityExceeded { details } => format!("Capacity exceeded: {}", details),
            SourceError::ServerError { .. } => "Server returned an error".to_string(),
            SourceError::CertificateError { .. } => "SSL certificate error".to_string(),
            SourceError::UnsupportedServerVersion { .. } => "Unsupported server version".to_string(),
            SourceError::RateLimitExceeded { retry_after_seconds, .. } => format!("Rate limit exceeded, try again in {} seconds", retry_after_seconds),
            SourceError::FileNotFound { .. } => "File not found".to_string(),
            SourceError::DirectoryNotFound { .. } => "Directory not found".to_string(),
            SourceError::ValidationFailed { issues } => format!("Validation failed: {}", issues),
            SourceError::SyncFailed { reason } => format!("Sync failed: {}", reason),
            SourceError::DeleteRestricted { reason, .. } => format!("Cannot delete source: {}", reason),
        }
    }
    
    fn error_code(&self) -> &'static str {
        match self {
            SourceError::NotFound => "SOURCE_NOT_FOUND",
            SourceError::NotFoundById { .. } => "SOURCE_NOT_FOUND_BY_ID",
            SourceError::DuplicateName { .. } => "SOURCE_DUPLICATE_NAME",
            SourceError::InvalidPath { .. } => "SOURCE_INVALID_PATH",
            SourceError::ConnectionFailed { .. } => "SOURCE_CONNECTION_FAILED",
            SourceError::AuthenticationFailed { .. } => "SOURCE_AUTH_FAILED",
            SourceError::SyncInProgress { .. } => "SOURCE_SYNC_IN_PROGRESS",
            SourceError::ConfigurationInvalid { .. } => "SOURCE_CONFIG_INVALID",
            SourceError::AccessDenied { .. } => "SOURCE_ACCESS_DENIED",
            SourceError::SourceDisabled { .. } => "SOURCE_DISABLED",
            SourceError::InvalidSourceType { .. } => "SOURCE_INVALID_TYPE",
            SourceError::NetworkTimeout { .. } => "SOURCE_NETWORK_TIMEOUT",
            SourceError::CapacityExceeded { .. } => "SOURCE_CAPACITY_EXCEEDED",
            SourceError::ServerError { .. } => "SOURCE_SERVER_ERROR",
            SourceError::CertificateError { .. } => "SOURCE_CERTIFICATE_ERROR",
            SourceError::UnsupportedServerVersion { .. } => "SOURCE_UNSUPPORTED_VERSION",
            SourceError::RateLimitExceeded { .. } => "SOURCE_RATE_LIMITED",
            SourceError::FileNotFound { .. } => "SOURCE_FILE_NOT_FOUND",
            SourceError::DirectoryNotFound { .. } => "SOURCE_DIRECTORY_NOT_FOUND",
            SourceError::ValidationFailed { .. } => "SOURCE_VALIDATION_FAILED",
            SourceError::SyncFailed { .. } => "SOURCE_SYNC_FAILED",
            SourceError::DeleteRestricted { .. } => "SOURCE_DELETE_RESTRICTED",
        }
    }
    
    fn error_category(&self) -> ErrorCategory {
        match self {
            SourceError::ConnectionFailed { .. } 
            | SourceError::NetworkTimeout { .. } 
            | SourceError::ServerError { .. } 
            | SourceError::CertificateError { .. } => ErrorCategory::Network,
            SourceError::ConfigurationInvalid { .. } 
            | SourceError::InvalidSourceType { .. } 
            | SourceError::UnsupportedServerVersion { .. } => ErrorCategory::Config,
            SourceError::AuthenticationFailed { .. } 
            | SourceError::AccessDenied { .. } => ErrorCategory::Auth,
            SourceError::InvalidPath { .. } 
            | SourceError::FileNotFound { .. } 
            | SourceError::DirectoryNotFound { .. } => ErrorCategory::FileSystem,
            _ => ErrorCategory::FileSystem, // Default for source-related operations
        }
    }
    
    fn error_severity(&self) -> ErrorSeverity {
        match self {
            SourceError::ConfigurationInvalid { .. } 
            | SourceError::AuthenticationFailed { .. } => ErrorSeverity::Critical,
            SourceError::ConnectionFailed { .. } 
            | SourceError::NetworkTimeout { .. } 
            | SourceError::ServerError { .. } 
            | SourceError::SyncFailed { .. } => ErrorSeverity::Important,
            SourceError::SyncInProgress { .. } 
            | SourceError::RateLimitExceeded { .. } => ErrorSeverity::Expected,
            _ => ErrorSeverity::Minor,
        }
    }
    
    fn suppression_key(&self) -> Option<String> {
        match self {
            SourceError::ConnectionFailed { .. } => Some("source_connection_failed".to_string()),
            SourceError::NetworkTimeout { .. } => Some("source_network_timeout".to_string()),
            SourceError::RateLimitExceeded { name, .. } => Some(format!("source_rate_limit_{}", name)),
            _ => None,
        }
    }
    
    fn suggested_action(&self) -> Option<String> {
        match self {
            SourceError::DuplicateName { .. } => Some("Please choose a different name for the source".to_string()),
            SourceError::ConnectionFailed { .. } => Some("Check network connectivity and server URL".to_string()),
            SourceError::AuthenticationFailed { .. } => Some("Verify username and password are correct".to_string()),
            SourceError::ConfigurationInvalid { .. } => Some("Review and correct the source configuration".to_string()),
            SourceError::NetworkTimeout { .. } => Some("Check network connection and try again".to_string()),
            SourceError::CertificateError { .. } => Some("Verify SSL certificate or contact server administrator".to_string()),
            SourceError::RateLimitExceeded { retry_after_seconds, .. } => Some(format!("Wait {} seconds before retrying", retry_after_seconds)),
            SourceError::SourceDisabled { .. } => Some("Enable the source to continue operations".to_string()),
            _ => None,
        }
    }
}

impl_into_response!(SourceError);

/// Convenience methods for creating common source errors
impl SourceError {
    pub fn not_found_by_id(id: Uuid) -> Self {
        Self::NotFoundById { id }
    }
    
    pub fn duplicate_name<S: Into<String>>(name: S) -> Self {
        Self::DuplicateName { name: name.into() }
    }
    
    pub fn invalid_path<S: Into<String>>(path: S) -> Self {
        Self::InvalidPath { path: path.into() }
    }
    
    pub fn connection_failed<S: Into<String>>(details: S) -> Self {
        Self::ConnectionFailed { details: details.into() }
    }
    
    pub fn authentication_failed<S: Into<String>>(name: S, reason: S) -> Self {
        Self::AuthenticationFailed { 
            name: name.into(), 
            reason: reason.into() 
        }
    }
    
    pub fn sync_in_progress<S: Into<String>>(name: S) -> Self {
        Self::SyncInProgress { name: name.into() }
    }
    
    pub fn configuration_invalid<S: Into<String>>(details: S) -> Self {
        Self::ConfigurationInvalid { details: details.into() }
    }
    
    pub fn access_denied<S: Into<String>>(path: S, reason: S) -> Self {
        Self::AccessDenied { 
            path: path.into(), 
            reason: reason.into() 
        }
    }
    
    pub fn source_disabled<S: Into<String>>(name: S) -> Self {
        Self::SourceDisabled { name: name.into() }
    }
    
    pub fn invalid_source_type<S: Into<String>>(source_type: S) -> Self {
        Self::InvalidSourceType { source_type: source_type.into() }
    }
    
    pub fn network_timeout<S: Into<String>>(url: S, timeout_seconds: u64) -> Self {
        Self::NetworkTimeout { 
            url: url.into(), 
            timeout_seconds 
        }
    }
    
    pub fn capacity_exceeded<S: Into<String>>(details: S) -> Self {
        Self::CapacityExceeded { details: details.into() }
    }
    
    pub fn server_error<S: Into<String>>(server: S, error_code: u16, message: S) -> Self {
        Self::ServerError { 
            server: server.into(), 
            error_code, 
            message: message.into() 
        }
    }
    
    pub fn certificate_error<S: Into<String>>(server: S, details: S) -> Self {
        Self::CertificateError { 
            server: server.into(), 
            details: details.into() 
        }
    }
    
    pub fn unsupported_server_version<S: Into<String>>(version: S, source_type: S) -> Self {
        Self::UnsupportedServerVersion { 
            version: version.into(), 
            source_type: source_type.into() 
        }
    }
    
    pub fn rate_limit_exceeded<S: Into<String>>(name: S, retry_after_seconds: u64) -> Self {
        Self::RateLimitExceeded { 
            name: name.into(), 
            retry_after_seconds 
        }
    }
    
    pub fn file_not_found<S: Into<String>>(path: S) -> Self {
        Self::FileNotFound { path: path.into() }
    }
    
    pub fn directory_not_found<S: Into<String>>(path: S) -> Self {
        Self::DirectoryNotFound { path: path.into() }
    }
    
    pub fn validation_failed<S: Into<String>>(issues: S) -> Self {
        Self::ValidationFailed { issues: issues.into() }
    }
    
    pub fn sync_failed<S: Into<String>>(reason: S) -> Self {
        Self::SyncFailed { reason: reason.into() }
    }
    
    pub fn delete_restricted<S: Into<String>>(name: S, reason: S) -> Self {
        Self::DeleteRestricted { 
            name: name.into(), 
            reason: reason.into() 
        }
    }
}