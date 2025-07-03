use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;
use uuid::Uuid;
use tracing::{error, info};
use anyhow::Result;

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
        .route("/{id}/sync/stop", post(stop_sync))
        .route("/{id}/deep-scan", post(trigger_deep_scan))
        .route("/{id}/validate", post(validate_source))
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
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
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

    // Get source IDs for batch counting
    let source_ids: Vec<Uuid> = sources.iter().map(|s| s.id).collect();
    
    // Get document counts for all sources in one query
    let counts = state
        .db
        .count_documents_for_sources(&source_ids)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Create a map for quick lookup
    let count_map: std::collections::HashMap<Uuid, (i64, i64)> = counts
        .into_iter()
        .map(|(id, total, ocr)| (id, (total, ocr)))
        .collect();

    let responses: Vec<SourceResponse> = sources
        .into_iter()
        .map(|s| {
            let (total_docs, total_ocr) = count_map.get(&s.id).copied().unwrap_or((0, 0));
            let mut response: SourceResponse = s.into();
            response.total_documents = total_docs;
            response.total_documents_ocr = total_ocr;
            response
        })
        .collect();
    
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
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn create_source(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(source_data): Json<CreateSource>,
) -> Result<Json<SourceResponse>, StatusCode> {
    // Validate source configuration based on type
    if let Err(validation_error) = validate_source_config(&source_data) {
        error!("Source validation failed: {}", validation_error);
        error!("Invalid source data received: {:?}", source_data);
        return Err(StatusCode::BAD_REQUEST);
    }

    let source = state
        .db
        .create_source(auth_user.user.id, &source_data)
        .await
        .map_err(|e| {
            error!("Failed to create source in database: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let mut response: SourceResponse = source.into();
    // New sources have no documents yet
    response.total_documents = 0;
    response.total_documents_ocr = 0;

    Ok(Json(response))
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
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Source not found"),
        (status = 500, description = "Internal server error")
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

    // Get document counts
    let (total_documents, total_documents_ocr) = state
        .db
        .count_documents_for_source(source_id)
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

    let mut source_response: SourceResponse = source.into();
    source_response.total_documents = total_documents;
    source_response.total_documents_ocr = total_documents_ocr;

    let response = SourceWithStats {
        source: source_response,
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
        (status = 400, description = "Bad request - invalid update data"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Source not found"),
        (status = 500, description = "Internal server error")
    )
)]
async fn update_source(
    auth_user: AuthUser,
    Path(source_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
    Json(update_data): Json<UpdateSource>,
) -> Result<Json<SourceResponse>, StatusCode> {
    use tracing::info;
    info!("Updating source {} with data: {:?}", source_id, update_data);
    // Check if source exists
    let existing = state
        .db
        .get_source(auth_user.user.id, source_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Validate config if provided
    if let Some(config) = &update_data.config {
        if let Err(validation_error) = validate_config_for_type(&existing.source_type, config) {
            error!("Config validation failed for source {}: {}", source_id, validation_error);
            error!("Invalid config received: {:?}", config);
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let source = state
        .db
        .update_source(auth_user.user.id, source_id, &update_data)
        .await
        .map_err(|e| {
            error!("Failed to update source {} in database: {}", source_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get document counts
    let (total_documents, total_documents_ocr) = state
        .db
        .count_documents_for_source(source_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut response: SourceResponse = source.into();
    response.total_documents = total_documents;
    response.total_documents_ocr = total_documents_ocr;

    info!("Successfully updated source {}: {}", source_id, response.name);
    Ok(Json(response))
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
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Source not found"),
        (status = 500, description = "Internal server error")
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
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Source not found"),
        (status = 409, description = "Source is already syncing"),
        (status = 500, description = "Internal server error"),
        (status = 501, description = "Not implemented - Source type not supported")
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
    path = "/api/sources/{id}/deep-scan",
    tag = "sources",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = Uuid, Path, description = "Source ID")
    ),
    responses(
        (status = 200, description = "Deep scan started successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Source not found"),
        (status = 409, description = "Source is already syncing"),
        (status = 500, description = "Internal server error")
    )
)]
async fn trigger_deep_scan(
    auth_user: AuthUser,
    Path(source_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Starting deep scan for source {} by user {}", source_id, auth_user.user.username);
    
    let source = state
        .db
        .get_source(auth_user.user.id, source_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Check if source is already syncing
    if matches!(source.status, crate::models::SourceStatus::Syncing) {
        return Ok(Json(serde_json::json!({
            "success": false,
            "error": "source_already_syncing",
            "message": "Source is already syncing. Please wait for the current sync to complete before starting a deep scan."
        })));
    }

    match source.source_type {
        crate::models::SourceType::WebDAV => {
            // Handle WebDAV deep scan
            let config: crate::models::WebDAVSourceConfig = serde_json::from_value(source.config)
                .map_err(|e| {
                    error!("Failed to parse WebDAV config for source {}: {}", source_id, e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            // Create WebDAV service
            let webdav_config = crate::services::webdav_service::WebDAVConfig {
                server_url: config.server_url.clone(),
                username: config.username.clone(),
                password: config.password.clone(),
                watch_folders: config.watch_folders.clone(),
                file_extensions: config.file_extensions.clone(),
                timeout_seconds: 600, // 10 minutes for deep scan
                server_type: config.server_type.clone(),
            };

            let webdav_service = crate::services::webdav_service::WebDAVService::new(webdav_config.clone())
                .map_err(|e| {
                    error!("Failed to create WebDAV service for deep scan: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            // Update source status to syncing
            state
                .db
                .update_source_status(
                    source_id,
                    crate::models::SourceStatus::Syncing,
                    Some("Deep scan in progress".to_string()),
                )
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            // Start deep scan in background
            let state_clone = state.clone();
            let user_id = auth_user.user.id;
            let source_name = source.name.clone();
            let source_id_clone = source_id;
            let config_clone = config.clone();
            
            tokio::spawn(async move {
                let start_time = chrono::Utc::now();
                
                // Use guaranteed completeness deep scan method
                match webdav_service.deep_scan_with_guaranteed_completeness(user_id, &state_clone).await {
                    Ok(all_discovered_files) => {
                        info!("Deep scan with guaranteed completeness discovered {} files", all_discovered_files.len());
                        
                        if !all_discovered_files.is_empty() {
                            info!("Deep scan discovery completed for source {}: {} files found", source_id_clone, all_discovered_files.len());
                            
                            // Filter files by extensions and process them
                            let files_to_process: Vec<_> = all_discovered_files.into_iter()
                            .filter(|file_info| {
                                if file_info.is_directory {
                                    return false;
                                }
                                let file_extension = std::path::Path::new(&file_info.name)
                                    .extension()
                                    .and_then(|ext| ext.to_str())
                                    .unwrap_or("")
                                    .to_lowercase();
                                config_clone.file_extensions.contains(&file_extension)
                            })
                            .collect();
                        
                    info!("Deep scan will process {} files for source {}", files_to_process.len(), source_id_clone);
                        
                    // Process files using the existing sync mechanism
                    match crate::routes::webdav::webdav_sync::process_files_for_deep_scan(
                        state_clone.clone(),
                        user_id,
                        &webdav_service,
                        &files_to_process,
                        true, // enable background OCR
                        Some(source_id_clone)
                    ).await {
                        Ok(files_processed) => {
                                let duration = chrono::Utc::now() - start_time;
                                info!("Deep scan completed for source {}: {} files processed in {:?}", 
                                    source_id_clone, files_processed, duration);
                                
                                // Update source status to idle
                                if let Err(e) = state_clone.db.update_source_status(
                                    source_id_clone,
                                    crate::models::SourceStatus::Idle,
                                    Some(format!("Deep scan completed: {} files processed", files_processed)),
                                ).await {
                                    error!("Failed to update source status after deep scan: {}", e);
                                }
                                
                                // Send success notification
                                let notification = crate::models::CreateNotification {
                                    notification_type: "success".to_string(),
                                    title: "Deep Scan Completed".to_string(),
                                    message: format!(
                                        "Deep scan of {} completed successfully. {} files processed in {:.1} minutes.",
                                        source_name,
                                        files_processed,
                                        duration.num_seconds() as f64 / 60.0
                                    ),
                                    action_url: Some("/documents".to_string()),
                                    metadata: Some(serde_json::json!({
                                        "source_id": source_id_clone,
                                        "scan_type": "deep_scan",
                                        "files_processed": files_processed,
                                        "duration_seconds": duration.num_seconds()
                                    })),
                                };
                                
                            if let Err(e) = state_clone.db.create_notification(user_id, &notification).await {
                                error!("Failed to create deep scan success notification: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Deep scan file processing failed for source {}: {}", source_id_clone, e);
                            
                            // Update source status to error
                            if let Err(e2) = state_clone.db.update_source_status(
                                source_id_clone,
                                crate::models::SourceStatus::Error,
                                Some(format!("Deep scan failed: {}", e)),
                            ).await {
                                error!("Failed to update source status after deep scan error: {}", e2);
                            }
                            
                            // Send error notification
                            let notification = crate::models::CreateNotification {
                                notification_type: "error".to_string(),
                                title: "Deep Scan Failed".to_string(),
                                message: format!("Deep scan of {} failed: {}", source_name, e),
                                action_url: Some("/sources".to_string()),
                                metadata: Some(serde_json::json!({
                                    "source_id": source_id_clone,
                                    "scan_type": "deep_scan",
                                    "error": e.to_string()
                                })),
                            };
                            
                            if let Err(e) = state_clone.db.create_notification(user_id, &notification).await {
                                error!("Failed to create deep scan error notification: {}", e);
                            }
                        }
                    }

                        } else {
                            info!("Deep scan found no files for source {}", source_id_clone);
                            
                            // Update source status to idle even if no files found
                            if let Err(e) = state_clone.db.update_source_status(
                                source_id_clone,
                                crate::models::SourceStatus::Idle,
                                Some("Deep scan completed: no files found".to_string()),
                            ).await {
                                error!("Failed to update source status after empty deep scan: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Deep scan with guaranteed completeness failed for source {}: {}", source_id_clone, e);
                        
                        // Update source status to error
                        if let Err(e2) = state_clone.db.update_source_status(
                            source_id_clone,
                            crate::models::SourceStatus::Error,
                            Some(format!("Deep scan failed: {}", e)),
                        ).await {
                            error!("Failed to update source status after deep scan error: {}", e2);
                        }
                        
                        // Send error notification
                        let notification = crate::models::CreateNotification {
                            notification_type: "error".to_string(),
                            title: "Deep Scan Failed".to_string(),
                            message: format!("Deep scan of {} failed: {}", source_name, e),
                            action_url: Some("/sources".to_string()),
                            metadata: Some(serde_json::json!({
                                "source_id": source_id_clone,
                                "scan_type": "deep_scan",
                                "error": e.to_string()
                            })),
                        };
                        
                        if let Err(e) = state_clone.db.create_notification(user_id, &notification).await {
                            error!("Failed to create deep scan error notification: {}", e);
                        }
                    }
                }
            });

            Ok(Json(serde_json::json!({
                "success": true,
                "message": format!("Deep scan started for source '{}'. This will perform a complete rescan of all configured folders.", source.name)
            })))
        }
        _ => {
            error!("Deep scan not supported for source type: {:?}", source.source_type);
            Ok(Json(serde_json::json!({
                "success": false,
                "error": "unsupported_source_type",
                "message": "Deep scan is currently only supported for WebDAV sources"
            })))
        }
    }
}

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
async fn validate_source(
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

#[utoipa::path(
    post,
    path = "/api/sources/{id}/sync/stop",
    tag = "sources",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = Uuid, Path, description = "Source ID")
    ),
    responses(
        (status = 200, description = "Sync stopped successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Source not found"),
        (status = 409, description = "Source is not currently syncing"),
        (status = 500, description = "Internal server error")
    )
)]
async fn stop_sync(
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

    // Allow stopping sync regardless of current status to handle edge cases
    // where the database status might be out of sync with actual running tasks

    // Stop sync using the universal source scheduler
    if let Some(scheduler) = &state.source_scheduler {
        if let Err(e) = scheduler.stop_sync(source_id).await {
            let error_msg = e.to_string();
            // If no sync is running, treat it as success since the desired state is achieved
            if error_msg.contains("No running sync found") {
                info!("No sync was running for source {}, updating status to idle", source_id);
                // Update status to idle since no sync is running
                state
                    .db
                    .update_source_status(
                        source_id,
                        crate::models::SourceStatus::Idle,
                        None,
                    )
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            } else {
                error!("Failed to stop sync for source {}: {}", source_id, e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    } else {
        // Update status directly if no scheduler available (fallback)
        state
            .db
            .update_source_status(
                source_id,
                crate::models::SourceStatus::Idle,
                Some("Sync cancelled by user".to_string()),
            )
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Source not found"),
        (status = 500, description = "Internal server error")
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

            match crate::services::webdav_service::test_webdav_connection(
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
        crate::models::SourceType::S3 => {
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
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Source not found"),
        (status = 500, description = "Internal server error")
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
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
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
    let webdav_config = crate::services::webdav_service::WebDAVConfig {
        server_url: config.server_url.clone(),
        username: config.username.clone(),
        password: config.password.clone(),
        watch_folders: config.watch_folders.clone(),
        file_extensions: config.file_extensions.clone(),
        timeout_seconds: 300,
        server_type: config.server_type.clone(),
    };

    // Create WebDAV service and estimate crawl
    match crate::services::webdav_service::WebDAVService::new(webdav_config) {
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
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
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

            match crate::services::webdav_service::test_webdav_connection(
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
        crate::models::SourceType::S3 => {
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