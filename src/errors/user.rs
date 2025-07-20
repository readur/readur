use axum::http::StatusCode;
use thiserror::Error;
use uuid::Uuid;

use super::{AppError, ErrorCategory, ErrorSeverity, impl_into_response};

/// Errors related to user management operations
#[derive(Error, Debug)]
pub enum UserError {
    #[error("User not found")]
    NotFound,
    
    #[error("User with ID {id} not found")]
    NotFoundById { id: Uuid },
    
    #[error("Username '{username}' already exists")]
    DuplicateUsername { username: String },
    
    #[error("Email '{email}' already exists")]
    DuplicateEmail { email: String },
    
    #[error("Invalid role '{role}'. Valid roles are: admin, user")]
    InvalidRole { role: String },
    
    #[error("Permission denied: {reason}")]
    PermissionDenied { reason: String },
    
    #[error("Invalid credentials")]
    InvalidCredentials,
    
    #[error("Account is disabled")]
    AccountDisabled,
    
    #[error("Password does not meet requirements: {requirements}")]
    InvalidPassword { requirements: String },
    
    #[error("Username '{username}' is invalid: {reason}")]
    InvalidUsername { username: String, reason: String },
    
    #[error("Email '{email}' is invalid")]
    InvalidEmail { email: String },
    
    #[error("Cannot delete user with ID {id}: {reason}")]
    DeleteRestricted { id: Uuid, reason: String },
    
    #[error("OIDC authentication failed: {details}")]
    OidcAuthenticationFailed { details: String },
    
    #[error("Authentication provider '{provider}' is not configured")]
    AuthProviderNotConfigured { provider: String },
    
    #[error("Token has expired")]
    TokenExpired,
    
    #[error("Invalid token format")]
    InvalidToken,
    
    #[error("User session has expired, please login again")]
    SessionExpired,
    
    #[error("Internal server error: {message}")]
    InternalServerError { message: String },
}

impl AppError for UserError {
    fn status_code(&self) -> StatusCode {
        match self {
            UserError::NotFound | UserError::NotFoundById { .. } => StatusCode::NOT_FOUND,
            UserError::DuplicateUsername { .. } | UserError::DuplicateEmail { .. } => StatusCode::CONFLICT,
            UserError::InvalidRole { .. } => StatusCode::BAD_REQUEST,
            UserError::PermissionDenied { .. } => StatusCode::FORBIDDEN,
            UserError::InvalidCredentials => StatusCode::UNAUTHORIZED,
            UserError::AccountDisabled => StatusCode::FORBIDDEN,
            UserError::InvalidPassword { .. } => StatusCode::BAD_REQUEST,
            UserError::InvalidUsername { .. } => StatusCode::BAD_REQUEST,
            UserError::InvalidEmail { .. } => StatusCode::BAD_REQUEST,
            UserError::DeleteRestricted { .. } => StatusCode::CONFLICT,
            UserError::OidcAuthenticationFailed { .. } => StatusCode::UNAUTHORIZED,
            UserError::AuthProviderNotConfigured { .. } => StatusCode::BAD_REQUEST,
            UserError::TokenExpired => StatusCode::UNAUTHORIZED,
            UserError::InvalidToken => StatusCode::UNAUTHORIZED,
            UserError::SessionExpired => StatusCode::UNAUTHORIZED,
            UserError::InternalServerError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
    
    fn user_message(&self) -> String {
        match self {
            UserError::NotFound | UserError::NotFoundById { .. } => "User not found".to_string(),
            UserError::DuplicateUsername { .. } => "Username already exists".to_string(),
            UserError::DuplicateEmail { .. } => "Email already exists".to_string(),
            UserError::InvalidRole { .. } => "Invalid user role specified".to_string(),
            UserError::PermissionDenied { reason } => format!("Permission denied: {}", reason),
            UserError::InvalidCredentials => "Invalid username or password".to_string(),
            UserError::AccountDisabled => "Account is disabled".to_string(),
            UserError::InvalidPassword { requirements } => format!("Password does not meet requirements: {}", requirements),
            UserError::InvalidUsername { reason, .. } => format!("Invalid username: {}", reason),
            UserError::InvalidEmail { .. } => "Invalid email address".to_string(),
            UserError::DeleteRestricted { reason, .. } => format!("Cannot delete user: {}", reason),
            UserError::OidcAuthenticationFailed { .. } => "OIDC authentication failed".to_string(),
            UserError::AuthProviderNotConfigured { .. } => "Authentication provider not configured".to_string(),
            UserError::TokenExpired => "Token has expired".to_string(),
            UserError::InvalidToken => "Invalid token".to_string(),
            UserError::SessionExpired => "Session has expired, please login again".to_string(),
            UserError::InternalServerError { .. } => "An internal error occurred".to_string(),
        }
    }
    
    fn error_code(&self) -> &'static str {
        match self {
            UserError::NotFound => "USER_NOT_FOUND",
            UserError::NotFoundById { .. } => "USER_NOT_FOUND_BY_ID",
            UserError::DuplicateUsername { .. } => "USER_DUPLICATE_USERNAME",
            UserError::DuplicateEmail { .. } => "USER_DUPLICATE_EMAIL",
            UserError::InvalidRole { .. } => "USER_INVALID_ROLE",
            UserError::PermissionDenied { .. } => "USER_PERMISSION_DENIED",
            UserError::InvalidCredentials => "USER_INVALID_CREDENTIALS",
            UserError::AccountDisabled => "USER_ACCOUNT_DISABLED",
            UserError::InvalidPassword { .. } => "USER_INVALID_PASSWORD",
            UserError::InvalidUsername { .. } => "USER_INVALID_USERNAME",
            UserError::InvalidEmail { .. } => "USER_INVALID_EMAIL",
            UserError::DeleteRestricted { .. } => "USER_DELETE_RESTRICTED",
            UserError::OidcAuthenticationFailed { .. } => "USER_OIDC_AUTH_FAILED",
            UserError::AuthProviderNotConfigured { .. } => "USER_AUTH_PROVIDER_NOT_CONFIGURED",
            UserError::TokenExpired => "USER_TOKEN_EXPIRED",
            UserError::InvalidToken => "USER_INVALID_TOKEN",
            UserError::SessionExpired => "USER_SESSION_EXPIRED",
            UserError::InternalServerError { .. } => "USER_INTERNAL_SERVER_ERROR",
        }
    }
    
    fn error_category(&self) -> ErrorCategory {
        ErrorCategory::Auth
    }
    
    fn error_severity(&self) -> ErrorSeverity {
        match self {
            UserError::PermissionDenied { .. } | UserError::DeleteRestricted { .. } => ErrorSeverity::Important,
            UserError::OidcAuthenticationFailed { .. } | UserError::AuthProviderNotConfigured { .. } => ErrorSeverity::Critical,
            UserError::InvalidCredentials | UserError::AccountDisabled => ErrorSeverity::Expected,
            UserError::InternalServerError { .. } => ErrorSeverity::Critical,
            _ => ErrorSeverity::Minor,
        }
    }
    
    fn suppression_key(&self) -> Option<String> {
        match self {
            UserError::InvalidCredentials => Some("user_invalid_credentials".to_string()),
            UserError::NotFound => Some("user_not_found".to_string()),
            _ => None,
        }
    }
    
    fn suggested_action(&self) -> Option<String> {
        match self {
            UserError::DuplicateUsername { .. } => Some("Please choose a different username".to_string()),
            UserError::DuplicateEmail { .. } => Some("Please use a different email address".to_string()),
            UserError::InvalidPassword { .. } => Some("Password must be at least 8 characters long and contain uppercase, lowercase, and numbers".to_string()),
            UserError::InvalidCredentials => Some("Please check your username and password".to_string()),
            UserError::SessionExpired | UserError::TokenExpired => Some("Please login again".to_string()),
            UserError::AccountDisabled => Some("Please contact an administrator".to_string()),
            _ => None,
        }
    }
}

impl_into_response!(UserError);

/// Convenience methods for creating common user errors
impl UserError {
    pub fn not_found_by_id(id: Uuid) -> Self {
        Self::NotFoundById { id }
    }
    
    pub fn duplicate_username<S: Into<String>>(username: S) -> Self {
        Self::DuplicateUsername { username: username.into() }
    }
    
    pub fn duplicate_email<S: Into<String>>(email: S) -> Self {
        Self::DuplicateEmail { email: email.into() }
    }
    
    pub fn invalid_role<S: Into<String>>(role: S) -> Self {
        Self::InvalidRole { role: role.into() }
    }
    
    pub fn permission_denied<S: Into<String>>(reason: S) -> Self {
        Self::PermissionDenied { reason: reason.into() }
    }
    
    pub fn invalid_password<S: Into<String>>(requirements: S) -> Self {
        Self::InvalidPassword { requirements: requirements.into() }
    }
    
    pub fn invalid_username<S: Into<String>>(username: S, reason: S) -> Self {
        Self::InvalidUsername { 
            username: username.into(), 
            reason: reason.into() 
        }
    }
    
    pub fn invalid_email<S: Into<String>>(email: S) -> Self {
        Self::InvalidEmail { email: email.into() }
    }
    
    pub fn delete_restricted<S: Into<String>>(id: Uuid, reason: S) -> Self {
        Self::DeleteRestricted { 
            id, 
            reason: reason.into() 
        }
    }
    
    pub fn oidc_authentication_failed<S: Into<String>>(details: S) -> Self {
        Self::OidcAuthenticationFailed { details: details.into() }
    }
    
    pub fn auth_provider_not_configured<S: Into<String>>(provider: S) -> Self {
        Self::AuthProviderNotConfigured { provider: provider.into() }
    }
    
    pub fn internal_server_error<S: Into<String>>(message: S) -> Self {
        Self::InternalServerError { message: message.into() }
    }
}