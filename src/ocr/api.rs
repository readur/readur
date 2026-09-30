use crate::ocr::error::OcrError;
use crate::ocr::image_ocr::ImageOcrService;
use crate::AppState;
use axum::{extract::State, http::StatusCode, response::Json};
use serde::Serialize;
use std::sync::Arc;
use ts_rs::TS;

#[derive(Serialize, utoipa::ToSchema, TS)]
#[ts(export)]
pub struct OcrHealthResponse {
    pub status: String,
    pub tesseract_installed: bool,
    pub available_languages: Vec<String>,
    pub diagnostics: Option<String>,
    pub errors: Vec<String>,
}

#[derive(Serialize, utoipa::ToSchema, TS)]
#[ts(export)]
pub struct OcrErrorResponse {
    pub error: String,
    pub error_code: String,
    pub details: Option<String>,
    pub is_recoverable: bool,
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
    State(state): State<Arc<AppState>>,
) -> Result<Json<OcrHealthResponse>, (StatusCode, Json<OcrErrorResponse>)> {
    let service = ImageOcrService::new().with_timeout(state.config.ocr_timeout_seconds);
    let diagnostics = service.get_diagnostics().await;

    let health_checker = crate::ocr::health::OcrHealthChecker::new();

    match health_checker.perform_full_health_check() {
        Ok(diag) => Ok(Json(OcrHealthResponse {
            status: "healthy".to_string(),
            tesseract_installed: true,
            available_languages: diag.available_languages,
            diagnostics: Some(diagnostics),
            errors: vec![],
        })),
        Err(errors) => {
            let error_messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();

            let _status_code = if errors.iter().any(|e| e.is_configuration_error()) {
                StatusCode::SERVICE_UNAVAILABLE
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };

            Ok(Json(OcrHealthResponse {
                status: "unhealthy".to_string(),
                tesseract_installed: errors
                    .iter()
                    .all(|e| !matches!(e, OcrError::TesseractNotInstalled)),
                available_languages: vec![],
                diagnostics: Some(diagnostics),
                errors: error_messages,
            }))
        }
    }
}
