use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;
use uuid::Uuid;
use tracing::{error, info};
use serde::{Deserialize};
use utoipa::ToSchema;

use crate::{
    auth::AuthUser,
    models::SourceType,
    AppState,
};

#[derive(Deserialize, ToSchema)]
pub struct TestConnectionRequest {
    pub source_type: SourceType,
    pub config: serde_json::Value,
}

/// Test connection for an existing source
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
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Source not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn test_connection(
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
        SourceType::WebDAV => {
            // Test WebDAV connection
            let config: crate::models::WebDAVSourceConfig = serde_json::from_value(source.config)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            let test_config = crate::models::WebDAVTestConnection {
                server_url: config.server_url,
                username: config.username,
                password: config.password,
                server_type: config.server_type,
            };
            
            match crate::services::webdav::test_webdav_connection(&test_config).await {
                Ok(result) => Ok(Json(serde_json::json!({
                    "success": result.success,
                    "message": result.message
                }))),
                Err(e) => Ok(Json(serde_json::json!({
                    "success": false,
                    "message": format!("Connection failed: {}", e)
                }))),
            }
        }
        SourceType::LocalFolder => {
            // Test Local Folder access
            let config: crate::models::LocalFolderSourceConfig = serde_json::from_value(source.config)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            match crate::services::local_folder_service::LocalFolderService::new(config) {
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
        SourceType::S3 => {
            // Test S3 connection
            let config: crate::models::S3SourceConfig = serde_json::from_value(source.config)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            match crate::services::s3_service::S3Service::new(config).await {
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

/// Test connection with a configuration (before creating source)
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
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn test_connection_with_config(
    _auth_user: AuthUser,
    State(_state): State<Arc<AppState>>,
    Json(request): Json<TestConnectionRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match request.source_type {
        SourceType::WebDAV => {
            // Test WebDAV connection
            let config: crate::models::WebDAVSourceConfig = serde_json::from_value(request.config)
                .map_err(|_| StatusCode::BAD_REQUEST)?;

            let test_config = crate::models::WebDAVTestConnection {
                server_url: config.server_url,
                username: config.username,
                password: config.password,
                server_type: config.server_type,
            };
            
            match crate::services::webdav::test_webdav_connection(&test_config).await {
                Ok(result) => Ok(Json(serde_json::json!({
                    "success": result.success,
                    "message": result.message
                }))),
                Err(e) => Ok(Json(serde_json::json!({
                    "success": false,
                    "message": format!("WebDAV connection failed: {}", e)
                }))),
            }
        }
        SourceType::LocalFolder => {
            // Test Local Folder access
            let config: crate::models::LocalFolderSourceConfig = serde_json::from_value(request.config)
                .map_err(|_| StatusCode::BAD_REQUEST)?;

            match crate::services::local_folder_service::LocalFolderService::new(config) {
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
        SourceType::S3 => {
            // Test S3 connection
            let config: crate::models::S3SourceConfig = serde_json::from_value(request.config)
                .map_err(|_| StatusCode::BAD_REQUEST)?;

            match crate::services::s3_service::S3Service::new(config).await {
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

/// Validate source health and configuration
#[utoipa::path(
    post,
    path = "/api/sources/{id}/validate",
    tag = "sources",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = Uuid, Path, description = "Source ID")
    ),
    responses(
        (status = 200, description = "Validation started successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Source not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn validate_source(
    auth_user: AuthUser,
    Path(source_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Starting validation check for source {} by user {}", source_id, auth_user.user.username);
    
    let source = state
        .db
        .get_source(auth_user.user.id, source_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Start validation in background
    let state_clone = state.clone();
    let source_clone = source.clone();
    tokio::spawn(async move {
        if let Err(e) = crate::scheduling::source_scheduler::SourceScheduler::validate_source_health(&source_clone, &state_clone).await {
            error!("Manual validation check failed for source {}: {}", source_clone.name, e);
        }
    });

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Validation check started for source '{}'", source.name)
    })))
}