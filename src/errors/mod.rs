use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;

use crate::monitoring::error_management::{
    ErrorCategory, ErrorSeverity, ManagedError, get_error_manager,
};

/// Common trait for all custom error types in the application
pub trait AppError: std::error::Error + Send + Sync + 'static {
    /// Get the HTTP status code for this error
    fn status_code(&self) -> StatusCode;
    
    /// Get a user-friendly error message
    fn user_message(&self) -> String;
    
    /// Get the error code for frontend handling
    fn error_code(&self) -> &'static str;
    
    /// Get the error category for the error management system
    fn error_category(&self) -> ErrorCategory;
    
    /// Get the error severity for the error management system
    fn error_severity(&self) -> ErrorSeverity;
    
    /// Get an optional suppression key for repeated error handling
    fn suppression_key(&self) -> Option<String> {
        None
    }
    
    /// Get optional suggested action for the user
    fn suggested_action(&self) -> Option<String> {
        None
    }
    
    /// Convert to a ManagedError for the error management system
    fn to_managed_error(&self) -> ManagedError {
        ManagedError {
            category: self.error_category(),
            severity: self.error_severity(),
            code: self.error_code().to_string(),
            user_message: self.user_message(),
            technical_details: self.to_string(),
            suggested_action: self.suggested_action(),
            suppression_key: self.suppression_key(),
        }
    }
}

/// Macro to implement IntoResponse for all AppError types
/// This provides consistent HTTP response formatting
macro_rules! impl_into_response {
    ($error_type:ty) => {
        impl axum::response::IntoResponse for $error_type {
            fn into_response(self) -> axum::response::Response {
                use crate::errors::AppError;
                use crate::monitoring::error_management::get_error_manager;
                use axum::{http::StatusCode, response::Json};
                use serde_json::json;
                
                // Send error to management system
                let error_manager = get_error_manager();
                let managed_error = self.to_managed_error();
                tokio::spawn(async move {
                    error_manager.handle_error(managed_error).await;
                });
                
                // Create HTTP response
                let status = self.status_code();
                let body = Json(json!({
                    "error": self.user_message(),
                    "code": self.error_code(),
                    "status": status.as_u16()
                }));
                
                (status, body).into_response()
            }
        }
    };
}

// Re-export the macro for use in other modules
pub(crate) use impl_into_response;

/// Generic API error for cases where specific error types don't apply
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Bad request: {message}")]
    BadRequest { message: String },
    
    #[error("Resource not found")]
    NotFound,
    
    #[error("Conflict: {message}")]
    Conflict { message: String },
    
    #[error("Unauthorized access")]
    Unauthorized,
    
    #[error("Forbidden: {message}")]
    Forbidden { message: String },
    
    #[error("Payload too large: {message}")]
    PayloadTooLarge { message: String },
    
    #[error("Internal server error: {message}")]
    InternalServerError { message: String },
    
    #[error("Service unavailable: {message}")]
    ServiceUnavailable { message: String },
}

impl AppError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            ApiError::NotFound => StatusCode::NOT_FOUND,
            ApiError::Conflict { .. } => StatusCode::CONFLICT,
            ApiError::Unauthorized => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden { .. } => StatusCode::FORBIDDEN,
            ApiError::PayloadTooLarge { .. } => StatusCode::PAYLOAD_TOO_LARGE,
            ApiError::InternalServerError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::ServiceUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
    
    fn user_message(&self) -> String {
        match self {
            ApiError::BadRequest { message } => message.clone(),
            ApiError::NotFound => "Resource not found".to_string(),
            ApiError::Conflict { message } => message.clone(),
            ApiError::Unauthorized => "Authentication required".to_string(),
            ApiError::Forbidden { message } => message.clone(),
            ApiError::PayloadTooLarge { message } => message.clone(),
            ApiError::InternalServerError { .. } => "An internal error occurred".to_string(),
            ApiError::ServiceUnavailable { message } => message.clone(),
        }
    }
    
    fn error_code(&self) -> &'static str {
        match self {
            ApiError::BadRequest { .. } => "BAD_REQUEST",
            ApiError::NotFound => "NOT_FOUND",
            ApiError::Conflict { .. } => "CONFLICT",
            ApiError::Unauthorized => "UNAUTHORIZED",
            ApiError::Forbidden { .. } => "FORBIDDEN",
            ApiError::PayloadTooLarge { .. } => "PAYLOAD_TOO_LARGE",
            ApiError::InternalServerError { .. } => "INTERNAL_SERVER_ERROR",
            ApiError::ServiceUnavailable { .. } => "SERVICE_UNAVAILABLE",
        }
    }
    
    fn error_category(&self) -> ErrorCategory {
        ErrorCategory::Network // Default for generic API errors
    }
    
    fn error_severity(&self) -> ErrorSeverity {
        match self {
            ApiError::InternalServerError { .. } => ErrorSeverity::Critical,
            ApiError::ServiceUnavailable { .. } => ErrorSeverity::Important,
            ApiError::Unauthorized | ApiError::Forbidden { .. } => ErrorSeverity::Important,
            _ => ErrorSeverity::Minor,
        }
    }
}

impl_into_response!(ApiError);

/// Utility functions for common error creation patterns
impl ApiError {
    pub fn bad_request<S: Into<String>>(message: S) -> Self {
        Self::BadRequest { message: message.into() }
    }
    
    pub fn conflict<S: Into<String>>(message: S) -> Self {
        Self::Conflict { message: message.into() }
    }
    
    pub fn forbidden<S: Into<String>>(message: S) -> Self {
        Self::Forbidden { message: message.into() }
    }
    
    pub fn payload_too_large<S: Into<String>>(message: S) -> Self {
        Self::PayloadTooLarge { message: message.into() }
    }
    
    pub fn internal_server_error<S: Into<String>>(message: S) -> Self {
        Self::InternalServerError { message: message.into() }
    }
    
    pub fn service_unavailable<S: Into<String>>(message: S) -> Self {
        Self::ServiceUnavailable { message: message.into() }
    }
}

// Re-export commonly used types (already imported above)
// pub use crate::monitoring::error_management::{ErrorCategory, ErrorSeverity};

// Submodules for entity-specific errors
pub mod user;
pub mod source;
pub mod label;
pub mod settings;
pub mod search;