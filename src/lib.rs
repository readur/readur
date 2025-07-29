pub mod auth;
pub mod config;
pub mod db;
pub mod db_guardrails_simple;
pub mod errors;
pub mod ingestion;
pub mod metadata_extraction;
pub mod mime_detection;
pub mod models;
pub mod monitoring;
pub mod ocr;
pub mod oidc;
pub mod routes;
pub mod scheduling;
pub mod seed;
pub mod services;
pub mod swagger;
pub mod utils;
pub mod webdav_xml_parser;

#[cfg(test)]
mod tests;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

use axum::{http::StatusCode, Json};
use utoipa;
use config::Config;
use db::Database;
use oidc::OidcClient;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub config: Config,
    pub webdav_scheduler: Option<std::sync::Arc<scheduling::webdav_scheduler::WebDAVScheduler>>,
    pub source_scheduler: Option<std::sync::Arc<scheduling::source_scheduler::SourceScheduler>>,
    pub queue_service: std::sync::Arc<ocr::queue::OcrQueueService>,
    pub oidc_client: Option<std::sync::Arc<OidcClient>>,
    pub sync_progress_tracker: std::sync::Arc<services::sync_progress_tracker::SyncProgressTracker>,
}

/// Health check endpoint for monitoring
#[utoipa::path(
    get,
    path = "/api/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy", body = serde_json::Value),
    )
)]
pub async fn health_check() -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({"status": "ok"})))
}
