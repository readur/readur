use crate::ocr::enhanced_processing::EnhancedOcrService;
use crate::ocr::error::OcrError;
use crate::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, utoipa::ToSchema)]
pub struct OcrHealthResponse {
    pub status: String,
    pub tesseract_installed: bool,
    pub available_languages: Vec<String>,
    pub diagnostics: Option<String>,
    pub errors: Vec<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct OcrErrorResponse {
    pub error: String,
    pub error_code: String,
    pub details: Option<String>,
    pub is_recoverable: bool,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct OcrRequest {
    pub file_path: String,
    pub language: Option<String>,
    pub use_fallback: Option<bool>,
}

#[utoipa::path(
    get,
    path = "/api/ocr/health",
    tag = "ocr",
    responses(
        (status = 200, description = "OCR service health status", body = OcrHealthResponse),
        (status = 500, description = "OCR service is unhealthy", body = OcrErrorResponse)
    )
)]
pub async fn health_check(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<OcrHealthResponse>, (StatusCode, Json<OcrErrorResponse>)> {
    let service = EnhancedOcrService::new();
    let diagnostics = service.get_diagnostics().await;
    
    let health_checker = crate::ocr::health::OcrHealthChecker::new();
    
    match health_checker.perform_full_health_check() {
        Ok(diag) => {
            Ok(Json(OcrHealthResponse {
                status: "healthy".to_string(),
                tesseract_installed: true,
                available_languages: diag.available_languages,
                diagnostics: Some(diagnostics),
                errors: vec![],
            }))
        }
        Err(errors) => {
            let error_messages: Vec<String> = errors.iter()
                .map(|e| e.to_string())
                .collect();
            
            let _status_code = if errors.iter().any(|e| e.is_configuration_error()) {
                StatusCode::SERVICE_UNAVAILABLE
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            
            Ok(Json(OcrHealthResponse {
                status: "unhealthy".to_string(),
                tesseract_installed: errors.iter().all(|e| !matches!(e, OcrError::TesseractNotInstalled)),
                available_languages: vec![],
                diagnostics: Some(diagnostics),
                errors: error_messages,
            }))
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/ocr/perform",
    tag = "ocr",
    request_body = OcrRequest,
    responses(
        (status = 200, description = "OCR text extraction successful", body = serde_json::Value),
        (status = 400, description = "Bad request or invalid language", body = OcrErrorResponse),
        (status = 500, description = "OCR processing failed", body = OcrErrorResponse)
    )
)]
pub async fn perform_ocr(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<OcrRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<OcrErrorResponse>)> {
    let service = EnhancedOcrService::new();
    let lang = request.language.as_deref().unwrap_or("eng");
    let use_fallback = request.use_fallback.unwrap_or(true);
    
    let result = if use_fallback {
        service.extract_with_fallback(&request.file_path, lang).await
    } else {
        service.extract_text_with_validation(&request.file_path, lang).await
    };
    
    match result {
        Ok(text) => Ok(Json(serde_json::json!({
            "text": text,
            "status": "success"
        }))),
        Err(e) => {
            if let Some(ocr_error) = e.downcast_ref::<OcrError>() {
                let (status_code, details) = match ocr_error {
                    OcrError::TesseractNotInstalled => (StatusCode::SERVICE_UNAVAILABLE, "Please install Tesseract OCR"),
                    OcrError::LanguageDataNotFound { .. } => (StatusCode::BAD_REQUEST, "Language pack not installed"),
                    OcrError::InsufficientMemory { .. } => (StatusCode::INSUFFICIENT_STORAGE, "Not enough memory"),
                    OcrError::ImageTooLarge { .. } => (StatusCode::PAYLOAD_TOO_LARGE, "Image exceeds size limits"),
                    OcrError::OcrTimeout { .. } => (StatusCode::REQUEST_TIMEOUT, "OCR operation timed out"),
                    OcrError::PermissionDenied { .. } => (StatusCode::FORBIDDEN, "Cannot access file"),
                    OcrError::InvalidImageFormat { .. } => (StatusCode::UNPROCESSABLE_ENTITY, "Invalid image format"),
                    _ => (StatusCode::INTERNAL_SERVER_ERROR, "OCR processing failed"),
                };
                
                Err((
                    status_code,
                    Json(OcrErrorResponse {
                        error: ocr_error.to_string(),
                        error_code: ocr_error.error_code().to_string(),
                        details: Some(details.to_string()),
                        is_recoverable: ocr_error.is_recoverable(),
                    }),
                ))
            } else {
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(OcrErrorResponse {
                        error: e.to_string(),
                        error_code: "OCR_UNKNOWN_ERROR".to_string(),
                        details: None,
                        is_recoverable: false,
                    }),
                ))
            }
        }
    }
}