use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use std::sync::Arc;
use uuid::Uuid;
use tracing::{error, info, warn};
use serde::{Deserialize};
use utoipa::ToSchema;

use crate::{
    auth::AuthUser,
    models::{merge_stored_secrets, SecretReuseRefused, SourceType, User},
    models::source::WebDAVTestConnection,
    utils::outbound::categorize_connection_error,
    AppState,
};

use super::crud::{authorize_source_config, ConfigRejection};

#[derive(Deserialize, ToSchema)]
pub struct TestConnectionRequest {
    pub source_type: SourceType,
    pub config: serde_json::Value,
    /// When editing an existing source, its ID: stored credentials fill in
    /// any secret the client leaves empty.
    #[serde(default)]
    pub source_id: Option<Uuid>,
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
        (status = 403, description = "Configuration not permitted"),
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

    run_connection_test(source.source_type, source.config, &auth_user.user, &state).await
}

/// Test connection with a configuration (before creating source)
#[utoipa::path(
    post,
    path = "/api/sources/test/connection",
    tag = "sources",
    security(
        ("bearer_auth" = [])
    ),
    request_body = TestConnectionRequest,
    responses(
        (status = 200, description = "Connection test result", body = serde_json::Value),
        (status = 400, description = "Bad request - invalid configuration, or a stored credential would be reused for a different server or account"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Configuration not permitted"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn test_connection_with_config(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(request): Json<TestConnectionRequest>,
) -> Result<Response, StatusCode> {
    let mut config = request.config;
    if let Some(source_id) = request.source_id {
        let source = state
            .db
            .get_source(auth_user.user.id, source_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;
        if source.source_type != request.source_type {
            return Err(StatusCode::BAD_REQUEST);
        }
        config = match merge_stored_secrets(source.source_type, &source.config, config) {
            Ok(config) => config,
            Err(refused) => return Ok(secret_reuse_refused(refused)),
        };
    }
    run_connection_test(request.source_type, config, &auth_user.user, &state)
        .await
        .map(IntoResponse::into_response)
}

pub(super) fn secret_reuse_refused(refused: SecretReuseRefused) -> Response {
    (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": refused.to_string() }))).into_response()
}

fn test_result(success: bool, message: impl Into<String>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "success": success, "message": message.into() }))
}

/// Connection failures are reported to the client as a coarse category; the
/// full error is only logged.
fn test_failure(kind: &str, detail: impl std::fmt::Display) -> Json<serde_json::Value> {
    let detail = detail.to_string();
    warn!("{} connection test failed: {}", kind, detail);
    test_result(false, categorize_connection_error(&detail))
}

async fn run_connection_test(
    source_type: SourceType,
    config: serde_json::Value,
    user: &User,
    state: &AppState,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match authorize_source_config(source_type, &config, user, state, true).await {
        Ok(()) => {}
        Err(ConfigRejection::Forbidden) => return Err(StatusCode::FORBIDDEN),
        Err(ConfigRejection::Invalid(reason)) => return Ok(test_result(false, reason)),
    }

    match source_type {
        SourceType::WebDAV => {
            let config: crate::models::WebDAVSourceConfig = serde_json::from_value(config)
                .map_err(|_| StatusCode::BAD_REQUEST)?;

            let test_config = WebDAVTestConnection {
                server_url: config.server_url,
                username: config.username,
                password: config.password,
                server_type: config.server_type,
            };

            match crate::services::webdav::test_webdav_connection(&test_config).await {
                Ok(result) if result.success => Ok(test_result(true, result.message)),
                Ok(result) => Ok(test_failure("WebDAV", result.message)),
                Err(e) => Ok(test_failure("WebDAV", e)),
            }
        }
        SourceType::LocalFolder => {
            let config: crate::models::LocalFolderSourceConfig = serde_json::from_value(config)
                .map_err(|_| StatusCode::BAD_REQUEST)?;

            match crate::services::local_folder_service::LocalFolderService::new(config) {
                Ok(service) => match service.test_connection().await {
                    Ok(message) => Ok(test_result(true, message)),
                    Err(e) => Ok(test_result(false, format!("Local folder test failed: {}", e))),
                },
                Err(e) => Ok(test_result(false, format!("Local folder configuration error: {}", e))),
            }
        }
        SourceType::S3 => {
            let config: crate::models::S3SourceConfig = serde_json::from_value(config)
                .map_err(|_| StatusCode::BAD_REQUEST)?;

            match crate::services::s3_service::S3Service::new(config).await {
                Ok(service) => match service.test_connection().await {
                    Ok(message) => Ok(test_result(true, message)),
                    Err(e) => Ok(test_failure("S3", e)),
                },
                Err(e) => Ok(test_failure("S3", e)),
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