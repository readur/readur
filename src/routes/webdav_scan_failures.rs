use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{error, info, warn};
use uuid::Uuid;
use utoipa::ToSchema;

use crate::auth::AuthUser;
use crate::models::source::{WebDAVScanFailureResponse, WebDAVScanFailureType, WebDAVScanFailureSeverity, WebDAVFailureDiagnostics};
use crate::models::source_error::{SourceErrorType, SourceErrorSeverity};
use crate::AppState;

/// Map generic source error type to WebDAV-specific type
fn map_to_webdav_error_type(source_type: &SourceErrorType) -> WebDAVScanFailureType {
    match source_type {
        SourceErrorType::Timeout => WebDAVScanFailureType::Timeout,
        SourceErrorType::PathTooLong => WebDAVScanFailureType::PathTooLong,
        SourceErrorType::PermissionDenied => WebDAVScanFailureType::PermissionDenied,
        SourceErrorType::InvalidCharacters => WebDAVScanFailureType::InvalidCharacters,
        SourceErrorType::NetworkError => WebDAVScanFailureType::NetworkError,
        SourceErrorType::ServerError => WebDAVScanFailureType::ServerError,
        SourceErrorType::XmlParseError => WebDAVScanFailureType::XmlParseError,
        SourceErrorType::TooManyItems => WebDAVScanFailureType::TooManyItems,
        SourceErrorType::DepthLimit => WebDAVScanFailureType::DepthLimit,
        SourceErrorType::SizeLimit => WebDAVScanFailureType::SizeLimit,
        _ => WebDAVScanFailureType::Unknown,
    }
}

/// Map generic source error severity to WebDAV-specific severity
fn map_to_webdav_severity(source_severity: &SourceErrorSeverity) -> WebDAVScanFailureSeverity {
    match source_severity {
        SourceErrorSeverity::Low => WebDAVScanFailureSeverity::Low,
        SourceErrorSeverity::Medium => WebDAVScanFailureSeverity::Medium,
        SourceErrorSeverity::High => WebDAVScanFailureSeverity::High,
        SourceErrorSeverity::Critical => WebDAVScanFailureSeverity::Critical,
    }
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_scan_failures))
        .route("/{id}", get(get_scan_failure))
        .route("/{id}/retry", post(retry_scan_failure))
        .route("/{id}/exclude", post(exclude_scan_failure))
        .route("/retry-candidates", get(get_retry_candidates))
}

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
    
    // Get WebDAV failures from generic system using source type filter
    use crate::models::{ErrorSourceType, ListFailuresQuery};
    let query = ListFailuresQuery {
        source_type: Some(ErrorSourceType::WebDAV),
        include_resolved: Some(false),
        ..Default::default()
    };
    
    let error_tracker = crate::services::source_error_tracker::SourceErrorTracker::new(state.db.clone());
    let failures = error_tracker.list_failures(auth_user.user.id, query).await
        .map_err(|e| {
            error!("Failed to get scan failures: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // Get statistics for WebDAV
    let generic_stats = error_tracker.get_stats(auth_user.user.id, Some(ErrorSourceType::WebDAV)).await
        .map_err(|e| {
            error!("Failed to get scan failure stats: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // Convert generic failures to WebDAV legacy format
    let mut failure_responses = Vec::new();
    for failure in failures {
        // Convert SourceScanFailureResponse to WebDAVScanFailureResponse
        let webdav_response = WebDAVScanFailureResponse {
            id: failure.id,
            directory_path: failure.resource_path,
            failure_type: map_to_webdav_error_type(&failure.error_type),
            failure_severity: map_to_webdav_severity(&failure.error_severity),
            failure_count: failure.failure_count,
            consecutive_failures: failure.consecutive_failures,
            first_failure_at: failure.first_failure_at,
            last_failure_at: failure.last_failure_at,
            next_retry_at: failure.next_retry_at,
            error_message: failure.error_message,
            http_status_code: failure.http_status_code,
            user_excluded: failure.user_excluded,
            user_notes: failure.user_notes,
            resolved: failure.resolved,
            diagnostic_summary: WebDAVFailureDiagnostics {
                path_length: failure.diagnostic_summary.resource_depth,
                directory_depth: failure.diagnostic_summary.resource_depth,
                estimated_item_count: failure.diagnostic_summary.estimated_item_count,
                response_time_ms: failure.diagnostic_summary.response_time_ms,
                response_size_mb: failure.diagnostic_summary.response_size_mb,
                server_type: failure.diagnostic_summary.source_specific_info.get("webdav_server_type")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                recommended_action: failure.diagnostic_summary.recommended_action,
                can_retry: failure.diagnostic_summary.can_retry,
                user_action_required: failure.diagnostic_summary.user_action_required,
            },
        };
        failure_responses.push(webdav_response);
    }
    
    // Convert stats to response format
    let stats_response = ScanFailureStatsResponse {
        active_failures: generic_stats.active_failures,
        resolved_failures: generic_stats.resolved_failures,
        excluded_directories: generic_stats.excluded_resources,
        critical_failures: generic_stats.critical_failures,
        high_failures: generic_stats.high_failures,
        medium_failures: generic_stats.medium_failures,
        low_failures: generic_stats.low_failures,
        ready_for_retry: generic_stats.ready_for_retry,
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
    
    let error_tracker = crate::services::source_error_tracker::SourceErrorTracker::new(state.db.clone());
    
    match error_tracker.get_failure_details(auth_user.user.id, failure_id).await {
        Ok(Some(failure)) => {
            info!("Found scan failure: {}", failure.resource_path);
            // Convert to WebDAV legacy format for backward compatibility
            let webdav_response = WebDAVScanFailureResponse {
                id: failure.id,
                directory_path: failure.resource_path,
                failure_type: map_to_webdav_error_type(&failure.error_type),
                failure_severity: map_to_webdav_severity(&failure.error_severity),
                failure_count: failure.failure_count,
                consecutive_failures: failure.consecutive_failures,
                first_failure_at: failure.first_failure_at,
                last_failure_at: failure.last_failure_at,
                next_retry_at: failure.next_retry_at,
                error_message: failure.error_message,
                http_status_code: failure.http_status_code,
                user_excluded: failure.user_excluded,
                user_notes: failure.user_notes,
                resolved: failure.resolved,
                diagnostic_summary: WebDAVFailureDiagnostics {
                    path_length: failure.diagnostic_summary.resource_depth,
                    directory_depth: failure.diagnostic_summary.resource_depth,
                    estimated_item_count: failure.diagnostic_summary.estimated_item_count,
                    response_time_ms: failure.diagnostic_summary.response_time_ms,
                    response_size_mb: failure.diagnostic_summary.response_size_mb,
                    server_type: failure.diagnostic_summary.source_specific_info.get("webdav_server_type")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    recommended_action: failure.diagnostic_summary.recommended_action,
                    can_retry: failure.diagnostic_summary.can_retry,
                    user_action_required: failure.diagnostic_summary.user_action_required,
                },
            };
            Ok(Json(serde_json::to_value(webdav_response).unwrap()))
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
    
    let error_tracker = crate::services::source_error_tracker::SourceErrorTracker::new(state.db.clone());
    
    // Use the generic retry functionality
    let retry_request = crate::models::RetryFailureRequest {
        reset_consecutive_count: Some(true),
        notes: request.notes,
    };
    
    match error_tracker.retry_failure(auth_user.user.id, failure_id, retry_request).await {
        Ok(true) => {
            info!("✅ Reset scan failure {} - ready for retry", failure_id);
            
            // TODO: Trigger an immediate scan of this directory
            // This would integrate with the WebDAV scheduler
            
            Ok(Json(json!({
                "success": true,
                "message": "Failure has been reset and will be retried",
                "failure_id": failure_id
            })))
        }
        Ok(false) => {
            warn!("Failed to reset scan failure {}", failure_id);
            Err(StatusCode::NOT_FOUND)
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
    
    let error_tracker = crate::services::source_error_tracker::SourceErrorTracker::new(state.db.clone());
    
    // Use the generic exclude functionality
    let exclude_request = crate::models::ExcludeResourceRequest {
        reason: request.notes.unwrap_or_else(|| "User excluded via WebDAV interface".to_string()),
        notes: Some(format!("Permanent: {}", request.permanent)),
        permanent: Some(request.permanent),
    };
    
    match error_tracker.exclude_resource(auth_user.user.id, failure_id, exclude_request).await {
        Ok(true) => {
            info!("✅ Excluded failure {} from scanning", failure_id);
            
            Ok(Json(json!({
                "success": true,
                "message": "Resource has been excluded from scanning",
                "failure_id": failure_id,
                "permanent": request.permanent
            })))
        }
        Ok(false) => {
            warn!("Failed to exclude failure {} - not found", failure_id);
            Err(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            error!("Failed to exclude resource from scanning: {}", e);
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
    
    let error_tracker = crate::services::source_error_tracker::SourceErrorTracker::new(state.db.clone());
    
    match error_tracker.get_retry_candidates(auth_user.user.id, Some(crate::models::ErrorSourceType::WebDAV), Some(20)).await {
        Ok(candidates) => {
            let directories: Vec<String> = candidates.iter()
                .map(|failure| failure.resource_path.clone())
                .collect();
            
            info!(
                "Found {} WebDAV directories ready for retry",
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