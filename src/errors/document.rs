//! Errors from document operations (listing, upload, OCR retry). Produced by
//! `services::documents`; the REST document routes return them as-is through
//! the `IntoResponse` impl below, and the MCP document tools map them to tool
//! results.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde_json::json;

/// Custom error type for document operations
#[derive(Debug)]
pub enum DocumentError {
    BadRequest(String),
    /// A list or filter query parameter failed validation.
    InvalidQuery(String),
    NotFound,
    Conflict(String),
    PayloadTooLarge(String),
    InternalServerError(String),
    UploadTimeout(String),
    DatabaseConstraintViolation(String),
    OcrProcessingError(String),
    FileProcessingError(String),
    ConcurrentUploadError(String),
}

impl IntoResponse for DocumentError {
    fn into_response(self) -> Response {
        let (status, message, error_code) = match self {
            DocumentError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg, "UPLOAD_BAD_REQUEST"),
            DocumentError::InvalidQuery(msg) => (StatusCode::BAD_REQUEST, msg, "INVALID_QUERY"),
            DocumentError::NotFound => (
                StatusCode::NOT_FOUND,
                "Document not found".to_string(),
                "UPLOAD_NOT_FOUND",
            ),
            DocumentError::Conflict(msg) => (StatusCode::CONFLICT, msg, "UPLOAD_CONFLICT"),
            DocumentError::PayloadTooLarge(msg) => {
                (StatusCode::PAYLOAD_TOO_LARGE, msg, "UPLOAD_TOO_LARGE")
            }
            DocumentError::InternalServerError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                msg,
                "UPLOAD_INTERNAL_ERROR",
            ),
            DocumentError::UploadTimeout(msg) => {
                (StatusCode::REQUEST_TIMEOUT, msg, "UPLOAD_TIMEOUT")
            }
            DocumentError::DatabaseConstraintViolation(msg) => {
                (StatusCode::CONFLICT, msg, "UPLOAD_DB_CONSTRAINT")
            }
            DocumentError::OcrProcessingError(msg) => {
                (StatusCode::UNPROCESSABLE_ENTITY, msg, "UPLOAD_OCR_ERROR")
            }
            DocumentError::FileProcessingError(msg) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                msg,
                "UPLOAD_FILE_PROCESSING_ERROR",
            ),
            DocumentError::ConcurrentUploadError(msg) => (
                StatusCode::TOO_MANY_REQUESTS,
                msg,
                "UPLOAD_CONCURRENT_ERROR",
            ),
        };

        (
            status,
            Json(json!({
                "error": message,
                "status": status.as_u16(),
                "error_code": error_code,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "request_id": uuid::Uuid::new_v4()
            })),
        )
            .into_response()
    }
}
