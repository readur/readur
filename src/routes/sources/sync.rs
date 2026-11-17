//! Starting and stopping a source's sync, and its progress as a one-off
//! request. The deep scan lives in `deep_scan`, the progress WebSocket in
//! `progress_ws`.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    models::SourceStatus,
    scheduling::source_scheduler::{StopSyncError, SyncStartError},
    AppState,
};

/// Trigger a sync for a source
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
        (status = 400, description = "Source configuration is invalid"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Source not found"),
        (status = 409, description = "Source is already syncing"),
        (status = 500, description = "Internal server error"),
        (status = 501, description = "Not implemented - Source type not supported")
    )
)]
pub async fn trigger_sync(
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

    // Trigger sync using the universal source scheduler
    // The scheduler will handle all status checks and atomic operations
    if let Some(scheduler) = &state.source_scheduler {
        match scheduler.trigger_sync(source_id).await {
            Ok(()) => {
                // Sync started successfully
            }
            Err(e) => {
                error!("Failed to trigger sync for source {}: {}", source_id, e);
                return Err(match e {
                    SyncStartError::AlreadyRunning
                    | SyncStartError::Stopping
                    | SyncStartError::NotClaimed => StatusCode::CONFLICT,
                    SyncStartError::NotFound => StatusCode::NOT_FOUND,
                    SyncStartError::Configuration(_) => StatusCode::BAD_REQUEST,
                    SyncStartError::Db(_) => StatusCode::INTERNAL_SERVER_ERROR,
                });
            }
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
                            SourceStatus::Error,
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
                        SourceStatus::Error,
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

/// Stop sync for a source
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
pub async fn stop_sync(
    auth_user: AuthUser,
    Path(source_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, StatusCode> {
    state
        .db
        .get_source(auth_user.user.id, source_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Allow stopping sync regardless of current status to handle edge cases
    // where the database status might be out of sync with actual running tasks

    // Stop sync using the universal source scheduler
    if let Some(scheduler) = &state.source_scheduler {
        match scheduler.stop_sync(source_id).await {
            Ok(()) => {}
            // Nothing running is a success: the desired state is achieved.
            Err(StopSyncError::NotRunning) => {
                info!(
                    "No sync was running for source {}, ensuring status is idle",
                    source_id
                );
                let _ = state
                    .db
                    .update_source_status_atomic(
                        source_id,
                        None, // Don't check current status
                        SourceStatus::Idle,
                        Some("No sync was running"),
                    )
                    .await;
            }
        }
    } else {
        // Update status directly if no scheduler available (fallback)
        state
            .db
            .update_source_status(
                source_id,
                SourceStatus::Idle,
                Some("Sync cancelled by user".to_string()),
            )
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(StatusCode::OK)
}

/// Get current sync progress (one-time API call)
#[utoipa::path(
    get,
    path = "/api/sources/{id}/sync/status",
    tag = "sources",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = Uuid, Path, description = "Source ID")
    ),
    responses(
        (status = 200, description = "Current sync progress", body = crate::services::sync_progress_tracker::SyncProgressInfo),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Source not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_sync_status(
    auth_user: AuthUser,
    Path(source_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Option<crate::services::sync_progress_tracker::SyncProgressInfo>>, StatusCode> {
    // Verify the source exists and the user has access
    let _source = state
        .db
        .get_source(auth_user.user.id, source_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Get current progress
    let progress_info = state.sync_progress_tracker.get_progress(source_id);

    Ok(Json(progress_info))
}
