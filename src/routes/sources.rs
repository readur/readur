use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;
use uuid::Uuid;
use tracing::error;

use crate::{
    auth::AuthUser,
    models::{CreateSource, SourceResponse, SourceWithStats, UpdateSource},
    AppState,
};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_sources).post(create_source))
        .route("/{id}", get(get_source).put(update_source).delete(delete_source))
        .route("/{id}/sync", post(trigger_sync))
        .route("/{id}/test", post(test_connection))
        .route("/{id}/estimate", post(estimate_crawl))
        .route("/estimate", post(estimate_crawl_with_config))
        .route("/test-connection", post(test_connection_with_config))
}

#[utoipa::path(
    get,
    path = "/api/sources",
    tag = "sources",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "List of user sources", body = Vec<SourceResponse>),
        (status = 401, description = "Unauthorized")
    )
)]
async fn list_sources(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<SourceResponse>>, StatusCode> {
    let sources = state
        .db
        .get_sources(auth_user.user.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let responses: Vec<SourceResponse> = sources.into_iter().map(|s| s.into()).collect();
    Ok(Json(responses))
}

#[utoipa::path(
    post,
    path = "/api/sources",
    tag = "sources",
    security(
        ("bearer_auth" = [])
    ),
    request_body = CreateSource,
    responses(
        (status = 201, description = "Source created successfully", body = SourceResponse),
        (status = 400, description = "Bad request - invalid source data"),
        (status = 401, description = "Unauthorized")
    )
)]
async fn create_source(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(source_data): Json<CreateSource>,
) -> Result<Json<SourceResponse>, StatusCode> {
    // Validate source configuration based on type
    if let Err(_) = validate_source_config(&source_data) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let source = state
        .db
        .create_source(auth_user.user.id, &source_data)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Json(source.into()))
}

#[utoipa::path(
    get,
    path = "/api/sources/{id}",
    tag = "sources",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = Uuid, Path, description = "Source ID")
    ),
    responses(
        (status = 200, description = "Source details with stats", body = SourceWithStats),
        (status = 404, description = "Source not found"),
        (status = 401, description = "Unauthorized")
    )
)]
async fn get_source(
    auth_user: AuthUser,
    Path(source_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<SourceWithStats>, StatusCode> {
    let source = state
        .db
        .get_source(auth_user.user.id, source_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Get recent documents for this source
    let recent_documents = state
        .db
        .get_recent_documents_for_source(source_id, 10)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Calculate sync progress
    let sync_progress = if source.total_files_pending > 0 {
        Some(
            (source.total_files_synced as f32
                / (source.total_files_synced + source.total_files_pending) as f32)
                * 100.0,
        )
    } else {
        None
    };

    let response = SourceWithStats {
        source: source.into(),
        recent_documents: recent_documents.into_iter().map(|d| d.into()).collect(),
        sync_progress,
    };

    Ok(Json(response))
}

#[utoipa::path(
    put,
    path = "/api/sources/{id}",
    tag = "sources",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = Uuid, Path, description = "Source ID")
    ),
    request_body = UpdateSource,
    responses(
        (status = 200, description = "Source updated successfully", body = SourceResponse),
        (status = 404, description = "Source not found"),
        (status = 400, description = "Bad request - invalid update data"),
        (status = 401, description = "Unauthorized")
    )
)]
async fn update_source(
    auth_user: AuthUser,
    Path(source_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
    Json(update_data): Json<UpdateSource>,
) -> Result<Json<SourceResponse>, StatusCode> {
    // Check if source exists
    let existing = state
        .db
        .get_source(auth_user.user.id, source_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Validate config if provided
    if let Some(config) = &update_data.config {
        if let Err(_) = validate_config_for_type(&existing.source_type, config) {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let source = state
        .db
        .update_source(auth_user.user.id, source_id, &update_data)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Json(source.into()))
}

#[utoipa::path(
    delete,
    path = "/api/sources/{id}",
    tag = "sources",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = Uuid, Path, description = "Source ID")
    ),
    responses(
        (status = 204, description = "Source deleted successfully"),
        (status = 404, description = "Source not found"),
        (status = 401, description = "Unauthorized")
    )
)]
async fn delete_source(
    auth_user: AuthUser,
    Path(source_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, StatusCode> {
    let deleted = state
        .db
        .delete_source(auth_user.user.id, source_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

#[utoipa::path(
    post,
    path = "/api/sources/{id}/sync",
    tag = "sources",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = Uuid, Path, description = "Source ID")
    ),
    responses(
        (status = 200, description = "Sync triggered successfully"),
        (status = 404, description = "Source not found"),
        (status = 409, description = "Source is already syncing"),
        (status = 401, description = "Unauthorized")
    )
)]
async fn trigger_sync(
    auth_user: AuthUser,
    Path(source_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, StatusCode> {
    let source = state
        .db
        .get_source(auth_user.user.id, source_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Check if already syncing
    if matches!(source.status, crate::models::SourceStatus::Syncing) {
        return Err(StatusCode::CONFLICT);
    }

    // Update status to syncing
    state
        .db
        .update_source_status(source_id, crate::models::SourceStatus::Syncing, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Trigger sync using the universal source scheduler
    if let Some(scheduler) = &state.source_scheduler {
        if let Err(e) = scheduler.trigger_sync(source_id).await {
            error!("Failed to trigger sync for source {}: {}", source_id, e);
            state
                .db
                .update_source_status(
                    source_id,
                    crate::models::SourceStatus::Error,
                    Some(format!("Failed to trigger sync: {}", e)),
                )
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    } else {
        // Fallback to WebDAV scheduler for backward compatibility
        match source.source_type {
            crate::models::SourceType::WebDAV => {
                if let Some(webdav_scheduler) = &state.webdav_scheduler {
                    webdav_scheduler.trigger_sync(source_id).await;
                } else {
                    state
                        .db
                        .update_source_status(
                            source_id,
                            crate::models::SourceStatus::Error,
                            Some("No scheduler available".to_string()),
                        )
                        .await
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
            _ => {
                state
                    .db
                    .update_source_status(
                        source_id,
                        crate::models::SourceStatus::Error,
                        Some("Source type not supported".to_string()),
                    )
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                return Err(StatusCode::NOT_IMPLEMENTED);
            }
        }
    }

    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/api/sources/{id}/test",
    tag = "sources",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = Uuid, Path, description = "Source ID")
    ),
    responses(
        (status = 200, description = "Connection test result", body = serde_json::Value),
        (status = 404, description = "Source not found"),
        (status = 401, description = "Unauthorized")
    )
)]
async fn test_connection(
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
            // Test WebDAV connection
            let config: crate::models::WebDAVSourceConfig = serde_json::from_value(source.config)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            match crate::webdav_service::test_webdav_connection(
                &config.server_url,
                &config.username,
                &config.password,
            )
            .await
            {
                Ok(success) => Ok(Json(serde_json::json!({
                    "success": success,
                    "message": if success { "Connection successful" } else { "Connection failed" }
                }))),
                Err(e) => Ok(Json(serde_json::json!({
                    "success": false,
                    "message": format!("Connection failed: {}", e)
                }))),
            }
        }
        crate::models::SourceType::LocalFolder => {
            // Test Local Folder access
            let config: crate::models::LocalFolderSourceConfig = serde_json::from_value(source.config)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            match crate::local_folder_service::LocalFolderService::new(config) {
                Ok(service) => {
                    match service.test_connection().await {
                        Ok(message) => Ok(Json(serde_json::json!({
                            "success": true,
                            "message": message
                        }))),
                        Err(e) => Ok(Json(serde_json::json!({
                            "success": false,
                            "message": format!("Local folder test failed: {}", e)
                        }))),
                    }
                }
                Err(e) => Ok(Json(serde_json::json!({
                    "success": false,
                    "message": format!("Local folder configuration error: {}", e)
                }))),
            }
        }
        crate::models::SourceType::S3 => {
            // Test S3 connection
            let config: crate::models::S3SourceConfig = serde_json::from_value(source.config)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            match crate::s3_service::S3Service::new(config).await {
                Ok(service) => {
                    match service.test_connection().await {
                        Ok(message) => Ok(Json(serde_json::json!({
                            "success": true,
                            "message": message
                        }))),
                        Err(e) => Ok(Json(serde_json::json!({
                            "success": false,
                            "message": format!("S3 test failed: {}", e)
                        }))),
                    }
                }
                Err(e) => Ok(Json(serde_json::json!({
                    "success": false,
                    "message": format!("S3 configuration error: {}", e)
                }))),
            }
        }
    }
}

fn validate_source_config(source: &CreateSource) -> Result<(), &'static str> {
    validate_config_for_type(&source.source_type, &source.config)
}

fn validate_config_for_type(
    source_type: &crate::models::SourceType,
    config: &serde_json::Value,
) -> Result<(), &'static str> {
    match source_type {
        crate::models::SourceType::WebDAV => {
            let _: crate::models::WebDAVSourceConfig =
                serde_json::from_value(config.clone()).map_err(|_| "Invalid WebDAV configuration")?;
            Ok(())
        }
        crate::models::SourceType::LocalFolder => {
            let _: crate::models::LocalFolderSourceConfig =
                serde_json::from_value(config.clone()).map_err(|_| "Invalid Local Folder configuration")?;
            Ok(())
        }
        crate::models::SourceType::S3 => {
            let _: crate::models::S3SourceConfig =
                serde_json::from_value(config.clone()).map_err(|_| "Invalid S3 configuration")?;
            Ok(())
        }
    }
}

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
        (status = 404, description = "Source not found"),
        (status = 401, description = "Unauthorized")
    )
)]
async fn estimate_crawl(
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
        (status = 401, description = "Unauthorized")
    )
)]
async fn estimate_crawl_with_config(
    _auth_user: AuthUser,
    State(_state): State<Arc<AppState>>,
    Json(config_data): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Parse the WebDAV config from the request
    let config: crate::models::WebDAVSourceConfig = serde_json::from_value(config_data)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    estimate_webdav_crawl_internal(&config).await
}

async fn estimate_webdav_crawl_internal(
    config: &crate::models::WebDAVSourceConfig,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Create WebDAV service config
    let webdav_config = crate::webdav_service::WebDAVConfig {
        server_url: config.server_url.clone(),
        username: config.username.clone(),
        password: config.password.clone(),
        watch_folders: config.watch_folders.clone(),
        file_extensions: config.file_extensions.clone(),
        timeout_seconds: 300,
        server_type: config.server_type.clone(),
    };

    // Create WebDAV service and estimate crawl
    match crate::webdav_service::WebDAVService::new(webdav_config) {
        Ok(webdav_service) => {
            match webdav_service.estimate_crawl(&config.watch_folders).await {
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

#[derive(serde::Deserialize, utoipa::ToSchema)]
struct TestConnectionRequest {
    source_type: crate::models::SourceType,
    config: serde_json::Value,
}

#[utoipa::path(
    post,
    path = "/api/sources/test-connection",
    tag = "sources",
    security(
        ("bearer_auth" = [])
    ),
    request_body = TestConnectionRequest,
    responses(
        (status = 200, description = "Connection test result", body = serde_json::Value),
        (status = 400, description = "Bad request - invalid configuration"),
        (status = 401, description = "Unauthorized")
    )
)]
async fn test_connection_with_config(
    _auth_user: AuthUser,
    State(_state): State<Arc<AppState>>,
    Json(request): Json<TestConnectionRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match request.source_type {
        crate::models::SourceType::WebDAV => {
            // Test WebDAV connection
            let config: crate::models::WebDAVSourceConfig = serde_json::from_value(request.config)
                .map_err(|_| StatusCode::BAD_REQUEST)?;

            match crate::webdav_service::test_webdav_connection(
                &config.server_url,
                &config.username,
                &config.password,
            )
            .await
            {
                Ok(success) => Ok(Json(serde_json::json!({
                    "success": success,
                    "message": if success { "WebDAV connection successful" } else { "WebDAV connection failed" }
                }))),
                Err(e) => Ok(Json(serde_json::json!({
                    "success": false,
                    "message": format!("WebDAV connection failed: {}", e)
                }))),
            }
        }
        crate::models::SourceType::LocalFolder => {
            // Test Local Folder access
            let config: crate::models::LocalFolderSourceConfig = serde_json::from_value(request.config)
                .map_err(|_| StatusCode::BAD_REQUEST)?;

            match crate::local_folder_service::LocalFolderService::new(config) {
                Ok(service) => {
                    match service.test_connection().await {
                        Ok(message) => Ok(Json(serde_json::json!({
                            "success": true,
                            "message": message
                        }))),
                        Err(e) => Ok(Json(serde_json::json!({
                            "success": false,
                            "message": format!("Local folder test failed: {}", e)
                        }))),
                    }
                }
                Err(e) => Ok(Json(serde_json::json!({
                    "success": false,
                    "message": format!("Local folder configuration error: {}", e)
                }))),
            }
        }
        crate::models::SourceType::S3 => {
            // Test S3 connection
            let config: crate::models::S3SourceConfig = serde_json::from_value(request.config)
                .map_err(|_| StatusCode::BAD_REQUEST)?;

            match crate::s3_service::S3Service::new(config).await {
                Ok(service) => {
                    match service.test_connection().await {
                        Ok(message) => Ok(Json(serde_json::json!({
                            "success": true,
                            "message": message
                        }))),
                        Err(e) => Ok(Json(serde_json::json!({
                            "success": false,
                            "message": format!("S3 test failed: {}", e)
                        }))),
                    }
                }
                Err(e) => Ok(Json(serde_json::json!({
                    "success": false,
                    "message": format!("S3 configuration error: {}", e)
                }))),
            }
        }
    }
}