use axum::http::StatusCode;
use thiserror::Error;

use super::{AppError, ErrorCategory, ErrorSeverity, impl_into_response};

#[derive(Error, Debug)]
pub enum CommentError {
    #[error("Comment not found")]
    NotFound,

    #[error("Document not found")]
    DocumentNotFound,

    #[error("Permission denied: {reason}")]
    PermissionDenied { reason: String },

    #[error("Comment content cannot be empty")]
    ContentEmpty,

    #[error("Comment content too long: {length} characters (max: {max_length})")]
    ContentTooLong { length: usize, max_length: usize },

    #[error("Parent comment not found")]
    ParentNotFound,

    #[error("Replies can only be one level deep")]
    NestingTooDeep,

    #[error("Internal error: {message}")]
    InternalError { message: String },
}

impl AppError for CommentError {
    fn status_code(&self) -> StatusCode {
        match self {
            CommentError::NotFound | CommentError::DocumentNotFound | CommentError::ParentNotFound => StatusCode::NOT_FOUND,
            CommentError::PermissionDenied { .. } => StatusCode::FORBIDDEN,
            CommentError::ContentEmpty | CommentError::ContentTooLong { .. } | CommentError::NestingTooDeep => StatusCode::BAD_REQUEST,
            CommentError::InternalError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn user_message(&self) -> String {
        match self {
            CommentError::NotFound => "Comment not found".to_string(),
            CommentError::DocumentNotFound => "Document not found".to_string(),
            CommentError::PermissionDenied { reason } => format!("Permission denied: {}", reason),
            CommentError::ContentEmpty => "Comment cannot be empty".to_string(),
            CommentError::ContentTooLong { max_length, .. } => format!("Comment is too long (max {} characters)", max_length),
            CommentError::ParentNotFound => "The comment you are replying to was not found".to_string(),
            CommentError::NestingTooDeep => "Replies can only be one level deep — reply to the original comment instead".to_string(),
            CommentError::InternalError { .. } => "An internal error occurred".to_string(),
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            CommentError::NotFound => "COMMENT_NOT_FOUND",
            CommentError::DocumentNotFound => "COMMENT_DOCUMENT_NOT_FOUND",
            CommentError::PermissionDenied { .. } => "COMMENT_PERMISSION_DENIED",
            CommentError::ContentEmpty => "COMMENT_CONTENT_EMPTY",
            CommentError::ContentTooLong { .. } => "COMMENT_CONTENT_TOO_LONG",
            CommentError::ParentNotFound => "COMMENT_PARENT_NOT_FOUND",
            CommentError::NestingTooDeep => "COMMENT_NESTING_TOO_DEEP",
            CommentError::InternalError { .. } => "COMMENT_INTERNAL_ERROR",
        }
    }

    fn error_category(&self) -> ErrorCategory {
        match self {
            CommentError::PermissionDenied { .. } => ErrorCategory::Auth,
            CommentError::InternalError { .. } => ErrorCategory::Database,
            _ => ErrorCategory::Network,
        }
    }

    fn error_severity(&self) -> ErrorSeverity {
        match self {
            CommentError::InternalError { .. } => ErrorSeverity::Critical,
            CommentError::PermissionDenied { .. } => ErrorSeverity::Important,
            CommentError::NotFound | CommentError::DocumentNotFound => ErrorSeverity::Expected,
            _ => ErrorSeverity::Minor,
        }
    }

    fn suggested_action(&self) -> Option<String> {
        match self {
            CommentError::ContentEmpty => Some("Enter some text for your comment".to_string()),
            CommentError::ContentTooLong { max_length, .. } => Some(format!("Shorten your comment to {} characters or less", max_length)),
            CommentError::NestingTooDeep => Some("Reply to the top-level comment instead".to_string()),
            _ => None,
        }
    }
}

impl_into_response!(CommentError);
