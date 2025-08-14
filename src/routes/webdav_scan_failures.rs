use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{error, info, warn};
use uuid::Uuid;
use utoipa::ToSchema;

use crate::auth::AuthUser;
use crate::models::{WebDAVScanFailure, WebDAVScanFailureResponse};
use crate::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct RetryFailureRequest {
    /// Optional notes about why the retry is being attempted
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ExcludeFailureRequest {
    /// User notes about why the directory is being excluded
    pub notes: Option<String>,
    /// Whether to permanently exclude (true) or just temporarily (false)
    pub permanent: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ScanFailureStatsResponse {
    pub active_failures: i64,
    pub resolved_failures: i64,
    pub excluded_directories: i64,
    pub critical_failures: i64,
    pub high_failures: i64,
    pub medium_failures: i64,
    pub low_failures: i64,
    pub ready_for_retry: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ScanFailuresListResponse {
    pub failures: Vec<WebDAVScanFailureResponse>,
    pub stats: ScanFailureStatsResponse,
}

/// GET /api/webdav/scan-failures - List all scan failures for the authenticated user
#[utoipa::path(
    get,
    path = "/api/webdav/scan-failures",
    responses(
        (status = 200, description = "List of scan failures", body = ScanFailuresListResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "WebDAV"
)]
pub async fn list_scan_failures(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<ScanFailuresListResponse>, StatusCode> {
    info!(
        "📋 Listing WebDAV scan failures for user: {}",
        auth_user.user.id
    );
    
    // Get failures from database
    let failures = state.db.get_scan_failures(auth_user.user.id, false).await
        .map_err(|e| {
            error!("Failed to get scan failures: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // Get statistics
    let stats = state.db.get_scan_failure_stats(auth_user.user.id).await
        .map_err(|e| {
            error!("Failed to get scan failure stats: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // Convert failures to response format with diagnostics
    let mut failure_responses = Vec::new();
    for failure in failures {
        if let Ok(Some(response)) = state.db.get_scan_failure_with_diagnostics(auth_user.user.id, failure.id).await {
            failure_responses.push(response);
        }
    }
    
    // Convert stats to response format
    let stats_response = ScanFailureStatsResponse {
        active_failures: stats.get("active_failures")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        resolved_failures: stats.get("resolved_failures")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        excluded_directories: stats.get("excluded_directories")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        critical_failures: stats.get("critical_failures")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        high_failures: stats.get("high_failures")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        medium_failures: stats.get("medium_failures")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        low_failures: stats.get("low_failures")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        ready_for_retry: stats.get("ready_for_retry")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
    };
    
    info!(
        "Found {} active scan failures for user",
        failure_responses.len()
    );
    
    Ok(Json(ScanFailuresListResponse {
        failures: failure_responses,
        stats: stats_response,
    }))
}

/// GET /api/webdav/scan-failures/{id} - Get detailed information about a specific scan failure
#[utoipa::path(
    get,
    path = "/api/webdav/scan-failures/{id}",
    params(
        ("id" = Uuid, Path, description = "Scan failure ID")
    ),
    responses(
        (status = 200, description = "Scan failure details", body = WebDAVScanFailureResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Failure not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "WebDAV"
)]
pub async fn get_scan_failure(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(failure_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!(
        "🔍 Getting scan failure details for ID: {} (user: {})",
        failure_id, auth_user.user.id
    );
    
    match state.db.get_scan_failure_with_diagnostics(auth_user.user.id, failure_id).await {
        Ok(Some(failure)) => {
            info!("Found scan failure: {}", failure.directory_path);
            Ok(Json(serde_json::to_value(failure).unwrap()))
        }
        Ok(None) => {
            warn!("Scan failure not found: {}", failure_id);
            Err(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            error!("Failed to get scan failure: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// POST /api/webdav/scan-failures/{id}/retry - Reset and retry a failed scan
#[utoipa::path(
    post,
    path = "/api/webdav/scan-failures/{id}/retry",
    params(
        ("id" = Uuid, Path, description = "Scan failure ID")
    ),
    request_body = RetryFailureRequest,
    responses(
        (status = 200, description = "Failure reset for retry"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Failure not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "WebDAV"
)]
pub async fn retry_scan_failure(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(failure_id): Path<Uuid>,
    Json(request): Json<RetryFailureRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!(
        "🔄 Retrying scan failure {} for user: {}",
        failure_id, auth_user.user.id
    );
    
    // First get the failure to find the directory path
    let failure = match state.db.get_scan_failure_with_diagnostics(auth_user.user.id, failure_id).await {
        Ok(Some(f)) => f,
        Ok(None) => {
            warn!("Scan failure not found for retry: {}", failure_id);
            return Err(StatusCode::NOT_FOUND);
        }
        Err(e) => {
            error!("Failed to get scan failure for retry: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    
    // Reset the failure for retry
    match state.db.reset_scan_failure(auth_user.user.id, &failure.directory_path).await {
        Ok(success) => {
            if success {
                info!(
                    "✅ Reset scan failure for directory '{}' - ready for retry",
                    failure.directory_path
                );
                
                // TODO: Trigger an immediate scan of this directory
                // This would integrate with the WebDAV scheduler
                
                Ok(Json(json!({
                    "success": true,
                    "message": format!("Directory '{}' has been reset and will be retried", failure.directory_path),
                    "directory_path": failure.directory_path
                })))
            } else {
                warn!(
                    "Failed to reset scan failure for directory '{}'",
                    failure.directory_path
                );
                Err(StatusCode::BAD_REQUEST)
            }
        }
        Err(e) => {
            error!("Failed to reset scan failure: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// POST /api/webdav/scan-failures/{id}/exclude - Mark a directory as permanently excluded
#[utoipa::path(
    post,
    path = "/api/webdav/scan-failures/{id}/exclude",
    params(
        ("id" = Uuid, Path, description = "Scan failure ID")
    ),
    request_body = ExcludeFailureRequest,
    responses(
        (status = 200, description = "Directory excluded from scanning"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Failure not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "WebDAV"
)]
pub async fn exclude_scan_failure(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(failure_id): Path<Uuid>,
    Json(request): Json<ExcludeFailureRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!(
        "🚫 Excluding scan failure {} for user: {}",
        failure_id, auth_user.user.id
    );
    
    // First get the failure to find the directory path
    let failure = match state.db.get_scan_failure_with_diagnostics(auth_user.user.id, failure_id).await {
        Ok(Some(f)) => f,
        Ok(None) => {
            warn!("Scan failure not found for exclusion: {}", failure_id);
            return Err(StatusCode::NOT_FOUND);
        }
        Err(e) => {
            error!("Failed to get scan failure for exclusion: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    
    // Exclude the directory
    match state.db.exclude_directory_from_scan(
        auth_user.user.id,
        &failure.directory_path,
        request.notes.as_deref(),
    ).await {
        Ok(()) => {
            info!(
                "✅ Excluded directory '{}' from scanning",
                failure.directory_path
            );
            
            Ok(Json(json!({
                "success": true,
                "message": format!("Directory '{}' has been excluded from scanning", failure.directory_path),
                "directory_path": failure.directory_path,
                "permanent": request.permanent
            })))
        }
        Err(e) => {
            error!("Failed to exclude directory from scanning: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/webdav/scan-failures/retry-candidates - Get directories ready for retry
#[utoipa::path(
    get,
    path = "/api/webdav/scan-failures/retry-candidates",
    responses(
        (status = 200, description = "List of directories ready for retry", body = Vec<String>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "WebDAV"
)]
pub async fn get_retry_candidates(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!(
        "🔍 Getting retry candidates for user: {}",
        auth_user.user.id
    );
    
    match state.db.get_directories_ready_for_retry(auth_user.user.id).await {
        Ok(directories) => {
            info!(
                "Found {} directories ready for retry",
                directories.len()
            );
            Ok(Json(json!({
                "directories": directories,
                "count": directories.len()
            })))
        }
        Err(e) => {
            error!("Failed to get retry candidates: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}