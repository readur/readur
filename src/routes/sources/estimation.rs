use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    AppState,
};

/// Estimate crawl for an existing source
#[utoipa::path(
    post,
    path = "/api/sources/{id}/estimate",
    tag = "sources",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = Uuid, Path, description = "Source ID")
    ),
    responses(
        (status = 200, description = "Crawl estimate result", body = serde_json::Value),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Source not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn estimate_crawl(
    auth_user: AuthUser,
    Path(source_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let source = state
        .db
        .get_source(auth_user.user.id, source_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    match source.source_type {
        crate::models::SourceType::WebDAV => {
            let config: crate::models::WebDAVSourceConfig = serde_json::from_value(source.config)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            estimate_webdav_crawl_internal(&config).await
        }
        _ => Ok(Json(serde_json::json!({
            "error": "Source type not supported for estimation"
        }))),
    }
}

/// Estimate crawl with a configuration (before creating source)
#[utoipa::path(
    post,
    path = "/api/sources/estimate",
    tag = "sources",
    security(
        ("bearer_auth" = [])
    ),
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "Crawl estimate result", body = serde_json::Value),
        (status = 400, description = "Bad request - invalid configuration"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn estimate_crawl_with_config(
    _auth_user: AuthUser,
    State(_state): State<Arc<AppState>>,
    Json(config_data): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Parse the WebDAV config from the request
    let config: crate::models::WebDAVSourceConfig = serde_json::from_value(config_data)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    estimate_webdav_crawl_internal(&config).await
}

/// Internal helper function to estimate WebDAV crawl
async fn estimate_webdav_crawl_internal(
    config: &crate::models::WebDAVSourceConfig,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Create WebDAV service config
    let webdav_config = crate::services::webdav::WebDAVConfig {
        server_url: config.server_url.clone(),
        username: config.username.clone(),
        password: config.password.clone(),
        watch_folders: config.watch_folders.clone(),
        file_extensions: config.file_extensions.clone(),
        timeout_seconds: 300,
        server_type: config.server_type.clone(),
        loop_detection: crate::services::webdav::loop_detection::LoopDetectionConfig::default(),
    };

    // Create WebDAV service and estimate crawl
    match crate::services::webdav::WebDAVService::new(webdav_config) {
        Ok(webdav_service) => {
            match webdav_service.estimate_crawl().await {
                Ok(estimate) => Ok(Json(serde_json::to_value(estimate).unwrap())),
                Err(e) => Ok(Json(serde_json::json!({
                    "error": format!("Crawl estimation failed: {}", e),
                    "folders": [],
                    "total_files": 0,
                    "total_supported_files": 0,
                    "total_estimated_time_hours": 0.0,
                    "total_size_mb": 0.0,
                }))),
            }
        }
        Err(e) => Ok(Json(serde_json::json!({
            "error": format!("Failed to create WebDAV service: {}", e),
            "folders": [],
            "total_files": 0,
            "total_supported_files": 0,
            "total_estimated_time_hours": 0.0,
            "total_size_mb": 0.0,
        }))),
    }
}