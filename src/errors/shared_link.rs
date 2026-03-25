use axum::http::StatusCode;
use thiserror::Error;

use super::{AppError, ErrorCategory, ErrorSeverity, impl_into_response};

#[derive(Error, Debug)]
pub enum SharedLinkError {
    #[error("Shared link not found")]
    NotFound,

    #[error("Shared link has expired")]
    Expired,

    #[error("Shared link has been revoked")]
    Revoked,

    #[error("Password is required to access this link")]
    PasswordRequired,

    #[error("Invalid password")]
    InvalidPassword,

    #[error("Maximum views reached for this shared link")]
    MaxViewsReached,

    #[error("Permission denied: {reason}")]
    PermissionDenied { reason: String },

    #[error("Document not found")]
    DocumentNotFound,

    #[error("Rate limit exceeded")]
    RateLimited { retry_after_secs: u64 },

    #[error("Internal error: {message}")]
    InternalError { message: String },
}

impl AppError for SharedLinkError {
    fn status_code(&self) -> StatusCode {
        match self {
            SharedLinkError::NotFound | SharedLinkError::DocumentNotFound => StatusCode::NOT_FOUND,
            SharedLinkError::Expired | SharedLinkError::Revoked | SharedLinkError::MaxViewsReached => StatusCode::GONE,
            SharedLinkError::PasswordRequired => StatusCode::UNAUTHORIZED,
            SharedLinkError::InvalidPassword => StatusCode::FORBIDDEN,
            SharedLinkError::PermissionDenied { .. } => StatusCode::FORBIDDEN,
            SharedLinkError::RateLimited { .. } => StatusCode::TOO_MANY_REQUESTS,
            SharedLinkError::InternalError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn user_message(&self) -> String {
        match self {
            SharedLinkError::NotFound => "This shared link does not exist".to_string(),
            SharedLinkError::Expired => "This shared link has expired".to_string(),
            SharedLinkError::Revoked => "This shared link has been revoked".to_string(),
            SharedLinkError::PasswordRequired => "A password is required to access this document".to_string(),
            SharedLinkError::InvalidPassword => "The password you entered is incorrect".to_string(),
            SharedLinkError::MaxViewsReached => "This shared link has reached its maximum number of views".to_string(),
            SharedLinkError::PermissionDenied { reason } => format!("Permission denied: {}", reason),
            SharedLinkError::RateLimited { retry_after_secs } => format!("Too many requests. Please try again in {} seconds.", retry_after_secs),
            SharedLinkError::DocumentNotFound => "The shared document is no longer available".to_string(),
            SharedLinkError::InternalError { .. } => "An internal error occurred".to_string(),
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            SharedLinkError::NotFound => "SHARED_LINK_NOT_FOUND",
            SharedLinkError::Expired => "SHARED_LINK_EXPIRED",
            SharedLinkError::Revoked => "SHARED_LINK_REVOKED",
            SharedLinkError::PasswordRequired => "SHARED_LINK_PASSWORD_REQUIRED",
            SharedLinkError::InvalidPassword => "SHARED_LINK_INVALID_PASSWORD",
            SharedLinkError::MaxViewsReached => "SHARED_LINK_MAX_VIEWS",
            SharedLinkError::PermissionDenied { .. } => "SHARED_LINK_PERMISSION_DENIED",
            SharedLinkError::RateLimited { .. } => "SHARED_LINK_RATE_LIMITED",
            SharedLinkError::DocumentNotFound => "SHARED_LINK_DOCUMENT_NOT_FOUND",
            SharedLinkError::InternalError { .. } => "SHARED_LINK_INTERNAL_ERROR",
        }
    }

    fn error_category(&self) -> ErrorCategory {
        match self {
            SharedLinkError::PasswordRequired
            | SharedLinkError::InvalidPassword
            | SharedLinkError::PermissionDenied { .. }
            | SharedLinkError::RateLimited { .. } => ErrorCategory::Auth,
            SharedLinkError::InternalError { .. } => ErrorCategory::Database,
            _ => ErrorCategory::Network,
        }
    }

    fn error_severity(&self) -> ErrorSeverity {
        match self {
            SharedLinkError::InternalError { .. } => ErrorSeverity::Critical,
            SharedLinkError::PermissionDenied { .. } | SharedLinkError::RateLimited { .. } => ErrorSeverity::Important,
            _ => ErrorSeverity::Expected,
        }
    }

    fn suggested_action(&self) -> Option<String> {
        match self {
            SharedLinkError::Expired => Some("Request a new shared link from the document owner".to_string()),
            SharedLinkError::PasswordRequired => Some("Enter the password provided by the document owner".to_string()),
            SharedLinkError::InvalidPassword => Some("Check the password and try again".to_string()),
            SharedLinkError::MaxViewsReached => Some("Request a new shared link from the document owner".to_string()),
            SharedLinkError::RateLimited { retry_after_secs } => Some(format!("Wait {} seconds before trying again", retry_after_secs)),
            _ => None,
        }
    }
}

impl_into_response!(SharedLinkError);
