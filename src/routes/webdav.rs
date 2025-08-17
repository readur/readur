use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde_json::Value;
use tracing::{error, info, warn};

use crate::{
    auth::AuthUser,
    models::{
        WebDAVConnectionResult, WebDAVCrawlEstimate, WebDAVSyncStatus,
        WebDAVTestConnection,
    },
    AppState,
};
use crate::services::webdav::WebDAVConfig;
use crate::services::webdav::WebDAVService;

pub mod webdav_sync;
use webdav_sync::perform_webdav_sync_with_tracking;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/test-connection", post(test_webdav_connection))
        .route("/estimate-crawl", post(estimate_webdav_crawl))
        .route("/sync-status", get(get_webdav_sync_status))
        .route("/start-sync", post(start_webdav_sync))
        .route("/cancel-sync", post(cancel_webdav_sync))
        // Scan failure tracking endpoints
        .route("/scan-failures", get(crate::routes::webdav_scan_failures::list_scan_failures))
        .route("/scan-failures/retry-candidates", get(crate::routes::webdav_scan_failures::get_retry_candidates))
        .route("/scan-failures/{id}", get(crate::routes::webdav_scan_failures::get_scan_failure))
        .route("/scan-failures/{id}/retry", post(crate::routes::webdav_scan_failures::retry_scan_failure))
        .route("/scan-failures/{id}/exclude", post(crate::routes::webdav_scan_failures::exclude_scan_failure))
}

async fn get_user_webdav_config(state: &Arc<AppState>, user_id: uuid::Uuid) -> Result<WebDAVConfig, StatusCode> {
    let settings = state
        .db
        .get_user_settings(user_id)
        .await
        .map_err(|e| {
            error!("Failed to get user settings: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let settings = settings.unwrap_or_default();

    if !settings.webdav_enabled {
        error!("WebDAV is not enabled for user {}", user_id);
        return Err(StatusCode::BAD_REQUEST);
    }

    let server_url = settings.webdav_server_url.unwrap_or_default();
    let username = settings.webdav_username.unwrap_or_default();
    let password = settings.webdav_password.unwrap_or_default();

    if server_url.is_empty() || username.is_empty() {
        error!("WebDAV configuration incomplete for user {}", user_id);
        return Err(StatusCode::BAD_REQUEST);
    }

    Ok(WebDAVConfig {
        server_url,
        username,
        password,
        watch_folders: settings.webdav_watch_folders,
        file_extensions: settings.webdav_file_extensions,
        timeout_seconds: 300, // 5 minutes timeout for crawl estimation
        server_type: Some("nextcloud".to_string()), // Default to Nextcloud
    })
}

#[utoipa::path(
    post,
    path = "/api/webdav/test-connection",
    tag = "webdav",
    security(
        ("bearer_auth" = [])
    ),
    request_body = WebDAVTestConnection,
    responses(
        (status = 200, description = "Connection test result", body = WebDAVConnectionResult),
        (status = 400, description = "Invalid request data"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn test_webdav_connection(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Json(test_config): Json<WebDAVTestConnection>,
) -> Result<Json<WebDAVConnectionResult>, StatusCode> {
    info!("Testing WebDAV connection to: {} for user: {}", 
        test_config.server_url, auth_user.user.username);

    // Create WebDAV config from test data
    let webdav_config = WebDAVConfig {
        server_url: test_config.server_url.clone(),
        username: test_config.username.clone(),
        password: test_config.password.clone(),
        watch_folders: Vec::new(),
        file_extensions: Vec::new(),
        timeout_seconds: 300, // 5 minutes timeout for crawl estimation
        server_type: test_config.server_type.clone(),
    };

    // Create WebDAV service and test connection
    match WebDAVService::new(webdav_config) {
        Ok(webdav_service) => {
            match WebDAVService::test_connection_with_config(&test_config).await {
                Ok(result) => {
                    info!("WebDAV connection test completed: {}", result.message);
                    Ok(Json(result))
                }
                Err(e) => {
                    error!("WebDAV connection test failed: {}", e);
                    Ok(Json(WebDAVConnectionResult {
                        success: false,
                        message: format!("Connection test failed: {}", e),
                        server_version: None,
                        server_type: None,
                    }))
                }
            }
        }
        Err(e) => {
            error!("Failed to create WebDAV service: {}", e);
            Ok(Json(WebDAVConnectionResult {
                success: false,
                message: format!("Service creation failed: {}", e),
                server_version: None,
                server_type: None,
            }))
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/webdav/estimate-crawl",
    tag = "webdav",
    security(
        ("bearer_auth" = [])
    ),
    request_body = Value,
    responses(
        (status = 200, description = "Crawl estimate", body = WebDAVCrawlEstimate),
        (status = 400, description = "Invalid request data"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn estimate_webdav_crawl(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Json(request): Json<Value>,
) -> Result<Json<WebDAVCrawlEstimate>, StatusCode> {
    let folders = request
        .get("folders")
        .and_then(|f| f.as_array())
        .ok_or(StatusCode::BAD_REQUEST)?
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect::<Vec<String>>();

    info!("Estimating crawl for {} folders for user: {}", folders.len(), auth_user.user.username);

    // Get user's WebDAV configuration
    let webdav_config = match get_user_webdav_config(&state, auth_user.user.id).await {
        Ok(config) => config,
        Err(status_code) => {
            warn!("Could not get WebDAV config for user {}: {:?}", auth_user.user.id, status_code);
            return Ok(Json(WebDAVCrawlEstimate {
                folders: vec![],
                total_files: 0,
                total_supported_files: 0,
                total_estimated_time_hours: 0.0,
                total_size_mb: 0.0,
            }));
        }
    };

    // Create WebDAV service and estimate crawl
    match WebDAVService::new(webdav_config) {
        Ok(webdav_service) => {
            match webdav_service.estimate_crawl().await {
                Ok(estimate) => {
                    info!("Crawl estimation completed: {} total files, {} supported files", 
                        estimate.total_files, estimate.total_supported_files);
                    Ok(Json(estimate))
                }
                Err(e) => {
                    error!("Crawl estimation failed: {}", e);
                    Ok(Json(WebDAVCrawlEstimate {
                        folders: vec![],
                        total_files: 0,
                        total_supported_files: 0,
                        total_estimated_time_hours: 0.0,
                        total_size_mb: 0.0,
                    }))
                }
            }
        }
        Err(e) => {
            error!("Failed to create WebDAV service for crawl estimation: {}", e);
            Ok(Json(WebDAVCrawlEstimate {
                folders: vec![],
                total_files: 0,
                total_supported_files: 0,
                total_estimated_time_hours: 0.0,
                total_size_mb: 0.0,
            }))
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/webdav/sync-status",
    tag = "webdav",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Current sync status", body = WebDAVSyncStatus),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_webdav_sync_status(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<WebDAVSyncStatus>, StatusCode> {
    info!("Getting WebDAV sync status for user: {}", auth_user.user.username);

    // Check if WebDAV is configured
    let webdav_config = match get_user_webdav_config(&state, auth_user.user.id).await {
        Ok(config) => config,
        Err(_) => {
            return Ok(Json(WebDAVSyncStatus {
                is_running: false,
                last_sync: None,
                files_processed: 0,
                files_remaining: 0,
                current_folder: None,
                errors: vec!["WebDAV not configured".to_string()],
            }));
        }
    };

    // Get sync state from database
    match state.db.get_webdav_sync_state(auth_user.user.id).await {
        Ok(Some(sync_state)) => {
            Ok(Json(WebDAVSyncStatus {
                is_running: sync_state.is_running,
                last_sync: sync_state.last_sync_at,
                files_processed: sync_state.files_processed,
                files_remaining: sync_state.files_remaining,
                current_folder: sync_state.current_folder,
                errors: sync_state.errors,
            }))
        }
        Ok(None) => {
            // No sync state yet
            Ok(Json(WebDAVSyncStatus {
                is_running: false,
                last_sync: None,
                files_processed: 0,
                files_remaining: 0,
                current_folder: None,
                errors: Vec::new(),
            }))
        }
        Err(e) => {
            error!("Failed to get WebDAV sync state: {}", e);
            Ok(Json(WebDAVSyncStatus {
                is_running: false,
                last_sync: None,
                files_processed: 0,
                files_remaining: 0,
                current_folder: None,
                errors: vec![format!("Error retrieving sync state: {}", e)],
            }))
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/webdav/start-sync",
    tag = "webdav",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Sync started successfully"),
        (status = 400, description = "WebDAV not configured or already running"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn start_webdav_sync(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<Value>, StatusCode> {
    info!("Starting WebDAV sync for user: {}", auth_user.user.username);

    // Check if a sync is already running for this user
    match state.db.get_webdav_sync_state(auth_user.user.id).await {
        Ok(Some(sync_state)) if sync_state.is_running => {
            warn!("WebDAV sync already running for user {}", auth_user.user.id);
            return Ok(Json(serde_json::json!({
                "success": false,
                "error": "sync_already_running",
                "message": "A WebDAV sync is already in progress. Please wait for it to complete before starting a new sync."
            })));
        }
        Ok(_) => {
            // No sync running or no sync state exists yet - proceed
        }
        Err(e) => {
            error!("Failed to check sync state for user {}: {}", auth_user.user.id, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    // Get user's WebDAV configuration and settings
    let webdav_config = get_user_webdav_config(&state, auth_user.user.id).await?;
    
    let user_settings = state
        .db
        .get_user_settings(auth_user.user.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .unwrap_or_default();

    // Create WebDAV service
    let webdav_service = WebDAVService::new(webdav_config.clone())
        .map_err(|e| {
            error!("Failed to create WebDAV service: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Start sync in background task
    let state_clone = state.clone();
    let user_id = auth_user.user.id;
    let enable_background_ocr = user_settings.enable_background_ocr;
    
    tokio::spawn(async move {
        match perform_webdav_sync_with_tracking(state_clone.clone(), user_id, webdav_service, webdav_config, enable_background_ocr, None).await {
            Ok(files_processed) => {
                info!("WebDAV sync completed successfully for user {}: {} files processed", user_id, files_processed);
                
                // Send success notification
                let notification = crate::models::CreateNotification {
                    notification_type: "success".to_string(),
                    title: "Manual WebDAV Sync Completed".to_string(),
                    message: if files_processed > 0 {
                        format!("Successfully processed {} files from manual WebDAV sync", files_processed)
                    } else {
                        "Manual WebDAV sync completed - no new files found".to_string()
                    },
                    action_url: Some("/documents".to_string()),
                    metadata: Some(serde_json::json!({
                        "sync_type": "webdav_manual",
                        "files_processed": files_processed
                    })),
                };
                
                if let Err(e) = state_clone.db.create_notification(user_id, &notification).await {
                    error!("Failed to create success notification: {}", e);
                }
            }
            Err(e) => {
                error!("WebDAV sync failed for user {}: {}", user_id, e);
                
                // Send error notification
                let notification = crate::models::CreateNotification {
                    notification_type: "error".to_string(),
                    title: "Manual WebDAV Sync Failed".to_string(),
                    message: format!("Manual WebDAV sync encountered an error: {}", e),
                    action_url: Some("/settings".to_string()),
                    metadata: Some(serde_json::json!({
                        "sync_type": "webdav_manual",
                        "error": e.to_string()
                    })),
                };
                
                if let Err(e) = state_clone.db.create_notification(user_id, &notification).await {
                    error!("Failed to create error notification: {}", e);
                }
            }
        }
    });

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "WebDAV sync started successfully"
    })))
}

#[utoipa::path(
    post,
    path = "/api/webdav/cancel-sync",
    tag = "webdav",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Sync cancelled successfully"),
        (status = 400, description = "No sync running or WebDAV not configured"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn cancel_webdav_sync(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<Value>, StatusCode> {
    info!("Cancelling WebDAV sync for user: {}", auth_user.user.username);

    // Check if a sync is currently running
    match state.db.get_webdav_sync_state(auth_user.user.id).await {
        Ok(Some(sync_state)) if sync_state.is_running => {
            // Mark sync as cancelled
            let cancelled_state = crate::models::UpdateWebDAVSyncState {
                last_sync_at: Some(chrono::Utc::now()),
                sync_cursor: sync_state.sync_cursor,
                is_running: false,
                files_processed: sync_state.files_processed,
                files_remaining: 0,
                current_folder: None,
                errors: vec!["Sync cancelled by user".to_string()],
            };
            
            if let Err(e) = state.db.update_webdav_sync_state(auth_user.user.id, &cancelled_state).await {
                error!("Failed to update sync state for cancellation: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
            
            info!("WebDAV sync cancelled for user {}", auth_user.user.id);
            
            // Send cancellation notification
            let notification = crate::models::CreateNotification {
                notification_type: "info".to_string(),
                title: "WebDAV Sync Cancelled".to_string(),
                message: "WebDAV sync was cancelled by user request".to_string(),
                action_url: Some("/settings".to_string()),
                metadata: Some(serde_json::json!({
                    "sync_type": "webdav_manual",
                    "cancelled": true
                })),
            };
            
            if let Err(e) = state.db.create_notification(auth_user.user.id, &notification).await {
                error!("Failed to create cancellation notification: {}", e);
            }
            
            Ok(Json(serde_json::json!({
                "success": true,
                "message": "WebDAV sync cancelled successfully"
            })))
        }
        Ok(Some(_)) => {
            // No sync running
            warn!("Attempted to cancel WebDAV sync for user {} but no sync is running", auth_user.user.id);
            Ok(Json(serde_json::json!({
                "success": false,
                "error": "no_sync_running",
                "message": "No WebDAV sync is currently running"
            })))
        }
        Ok(None) => {
            // No sync state exists
            warn!("No WebDAV sync state found for user {}", auth_user.user.id);
            Ok(Json(serde_json::json!({
                "success": false,
                "error": "no_sync_state",
                "message": "No WebDAV sync state found"
            })))
        }
        Err(e) => {
            error!("Failed to get sync state for user {}: {}", auth_user.user.id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

