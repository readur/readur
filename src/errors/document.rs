//! Errors from document operations (listing, upload, OCR retry). Produced by
//! `services::documents`; the REST document routes return them as-is through
//! the `IntoResponse` impl below, and the MCP document tools map them to tool
//! results.
//!
//! Every message is shown to the client. The variants for server-side
//! failures therefore take a `&'static str`: a fixed, generic message, so a
//! raw database or storage error (constraint names, SQL, file paths) cannot
//! be formatted into one. Log the underlying error where it is caught.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

/// Custom error type for document operations
#[derive(Debug)]
pub enum DocumentError {
    BadRequest(String),
    /// A list or filter query parameter failed validation.
    InvalidQuery(String),
    NotFound,
    Conflict(String),
    PayloadTooLarge(String),
    InternalServerError(&'static str),
    UploadTimeout(&'static str),
    DatabaseConstraintViolation(&'static str),
    OcrProcessingError(&'static str),
    FileProcessingError(&'static str),
    ConcurrentUploadError(String),
}

impl DocumentError {
    /// The message for the client.
    pub fn message(&self) -> &str {
        match self {
            Self::BadRequest(msg)
            | Self::InvalidQuery(msg)
            | Self::Conflict(msg)
            | Self::PayloadTooLarge(msg)
            | Self::ConcurrentUploadError(msg) => msg,
            Self::NotFound => "Document not found",
            Self::InternalServerError(msg)
            | Self::UploadTimeout(msg)
            | Self::DatabaseConstraintViolation(msg)
            | Self::OcrProcessingError(msg)
            | Self::FileProcessingError(msg) => msg,
        }
    }

    /// The HTTP status and the stable `code` clients can match on.
    pub fn status_and_code(&self) -> (StatusCode, &'static str) {
        match self {
            Self::BadRequest(_) => (StatusCode::BAD_REQUEST, "UPLOAD_BAD_REQUEST"),
            Self::InvalidQuery(_) => (StatusCode::BAD_REQUEST, "INVALID_QUERY"),
            Self::NotFound => (StatusCode::NOT_FOUND, "UPLOAD_NOT_FOUND"),
            Self::Conflict(_) => (StatusCode::CONFLICT, "UPLOAD_CONFLICT"),
            Self::PayloadTooLarge(_) => (StatusCode::PAYLOAD_TOO_LARGE, "UPLOAD_TOO_LARGE"),
            Self::InternalServerError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "UPLOAD_INTERNAL_ERROR")
            }
            Self::UploadTimeout(_) => (StatusCode::REQUEST_TIMEOUT, "UPLOAD_TIMEOUT"),
            Self::DatabaseConstraintViolation(_) => (StatusCode::CONFLICT, "UPLOAD_DB_CONSTRAINT"),
            Self::OcrProcessingError(_) => (StatusCode::UNPROCESSABLE_ENTITY, "UPLOAD_OCR_ERROR"),
            Self::FileProcessingError(_) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "UPLOAD_FILE_PROCESSING_ERROR",
            ),
            Self::ConcurrentUploadError(_) => {
                (StatusCode::TOO_MANY_REQUESTS, "UPLOAD_CONCURRENT_ERROR")
            }
        }
    }
}

impl IntoResponse for DocumentError {
    fn into_response(self) -> Response {
        let (status, code) = self.status_and_code();
        super::error_response(status, code, self.message())
    }
}
