use axum::http::StatusCode;
use thiserror::Error;

use super::{impl_into_response, AppError, ErrorCategory, ErrorSeverity};

#[derive(Error, Debug)]
pub enum ApiKeyError {
    #[error("API key not found")]
    NotFound,

    #[error("Invalid request: {reason}")]
    InvalidRequest { reason: String },

    #[error("Maximum number of API keys reached")]
    MaxKeysReached { limit: i64 },

    #[error("Permission denied: {reason}")]
    PermissionDenied { reason: String },

    #[error("Rate limit exceeded")]
    RateLimited { retry_after_secs: u64 },

    #[error("Internal error: {message}")]
    InternalError { message: String },
}

impl AppError for ApiKeyError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiKeyError::NotFound => StatusCode::NOT_FOUND,
            ApiKeyError::InvalidRequest { .. } => StatusCode::BAD_REQUEST,
            ApiKeyError::MaxKeysReached { .. } => StatusCode::CONFLICT,
            ApiKeyError::PermissionDenied { .. } => StatusCode::FORBIDDEN,
            ApiKeyError::RateLimited { .. } => StatusCode::TOO_MANY_REQUESTS,
            ApiKeyError::InternalError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn user_message(&self) -> String {
        match self {
            ApiKeyError::NotFound => "API key not found".to_string(),
            ApiKeyError::InvalidRequest { reason } => reason.clone(),
            ApiKeyError::MaxKeysReached { limit } => {
                format!("You already have {} active API keys. Revoke one before creating another.", limit)
            }
            ApiKeyError::PermissionDenied { reason } => format!("Permission denied: {}", reason),
            ApiKeyError::RateLimited { retry_after_secs } => {
                format!("Too many requests. Please try again in {} seconds.", retry_after_secs)
            }
            ApiKeyError::InternalError { .. } => "An internal error occurred".to_string(),
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            ApiKeyError::NotFound => "API_KEY_NOT_FOUND",
            ApiKeyError::InvalidRequest { .. } => "API_KEY_INVALID_REQUEST",
            ApiKeyError::MaxKeysReached { .. } => "API_KEY_MAX_REACHED",
            ApiKeyError::PermissionDenied { .. } => "API_KEY_PERMISSION_DENIED",
            ApiKeyError::RateLimited { .. } => "API_KEY_RATE_LIMITED",
            ApiKeyError::InternalError { .. } => "API_KEY_INTERNAL_ERROR",
        }
    }

    fn error_category(&self) -> ErrorCategory {
        match self {
            ApiKeyError::InternalError { .. } => ErrorCategory::Database,
            ApiKeyError::PermissionDenied { .. } | ApiKeyError::RateLimited { .. } => ErrorCategory::Auth,
            _ => ErrorCategory::Network,
        }
    }

    fn error_severity(&self) -> ErrorSeverity {
        match self {
            ApiKeyError::InternalError { .. } => ErrorSeverity::Critical,
            ApiKeyError::PermissionDenied { .. } | ApiKeyError::RateLimited { .. } => ErrorSeverity::Important,
            _ => ErrorSeverity::Expected,
        }
    }
}

impl_into_response!(ApiKeyError);
