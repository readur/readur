use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde_json::Value;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    models::{
        SourceScanFailureResponse, SourceScanFailureStats, MonitoredSourceType,
        ListFailuresQuery, RetryFailureRequest, ExcludeResourceRequest,
    },
    services::source_error_tracker::SourceErrorTracker,
    AppState,
};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_source_failures))
        .route("/stats", get(get_failure_stats))
        .route("/retry-candidates", get(get_retry_candidates))
        .route("/:failure_id", get(get_source_failure))
        .route("/:failure_id/retry", post(retry_source_failure))
        .route("/:failure_id/exclude", post(exclude_source_failure))
        .route("/:failure_id/resolve", post(resolve_source_failure))
        .route("/type/:source_type", get(list_source_type_failures))
        .route("/type/:source_type/stats", get(get_source_type_stats))
}

#[utoipa::path(
    get,
    path = "/api/source/errors",
    tag = "source-errors",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("source_type" = Option<String>, Query, description = "Filter by source type"),
        ("error_type" = Option<String>, Query, description = "Filter by error type"),
        ("severity" = Option<String>, Query, description = "Filter by severity"),
        ("include_resolved" = Option<bool>, Query, description = "Include resolved failures"),
        ("include_excluded" = Option<bool>, Query, description = "Include excluded resources"),
        ("ready_for_retry" = Option<bool>, Query, description = "Only show failures ready for retry"),
        ("limit" = Option<i32>, Query, description = "Maximum number of results"),
        ("offset" = Option<i32>, Query, description = "Number of results to skip")
    ),
    responses(
        (status = 200, description = "List of source scan failures", body = Vec<SourceScanFailureResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn list_source_failures(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Query(query): Query<ListFailuresQuery>,
) -> Result<Json<Vec<SourceScanFailureResponse>>, StatusCode> {
    info!("Listing source scan failures for user: {}", auth_user.user.username);

    // Create a basic error tracker (without classifiers for read-only operations)
    let error_tracker = SourceErrorTracker::new(state.db.clone());

    match error_tracker.list_failures(auth_user.user.id, query).await {
        Ok(failures) => {
            info!("Found {} source scan failures", failures.len());
            Ok(Json(failures))
        }
        Err(e) => {
            error!("Failed to list source scan failures: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/source/errors/stats",
    tag = "source-errors",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("source_type" = Option<String>, Query, description = "Filter stats by source type")
    ),
    responses(
        (status = 200, description = "Source scan failure statistics", body = SourceScanFailureStats),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_failure_stats(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Query(params): Query<serde_json::Value>,
) -> Result<Json<SourceScanFailureStats>, StatusCode> {
    info!("Getting source scan failure stats for user: {}", auth_user.user.username);

    // Parse source_type from query parameters
    let source_type = params.get("source_type")
        .and_then(|v| v.as_str())
        .and_then(|s| match s.to_lowercase().as_str() {
            "webdav" => Some(MonitoredSourceType::WebDAV),
            "s3" => Some(MonitoredSourceType::S3),
            "local" => Some(MonitoredSourceType::Local),
            _ => None,
        });

    let error_tracker = SourceErrorTracker::new(state.db.clone());

    match error_tracker.get_stats(auth_user.user.id, source_type).await {
        Ok(stats) => {
            info!("Retrieved source scan failure stats");
            Ok(Json(stats))
        }
        Err(e) => {
            error!("Failed to get source scan failure stats: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/source/errors/retry-candidates",
    tag = "source-errors",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("source_type" = Option<String>, Query, description = "Filter by source type"),
        ("limit" = Option<i32>, Query, description = "Maximum number of results")
    ),
    responses(
        (status = 200, description = "List of failures ready for retry", body = Vec<SourceScanFailureResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_retry_candidates(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Query(params): Query<serde_json::Value>,
) -> Result<Json<Vec<SourceScanFailureResponse>>, StatusCode> {
    info!("Getting retry candidates for user: {}", auth_user.user.username);

    let source_type = params.get("source_type")
        .and_then(|v| v.as_str())
        .and_then(|s| match s.to_lowercase().as_str() {
            "webdav" => Some(MonitoredSourceType::WebDAV),
            "s3" => Some(MonitoredSourceType::S3),
            "local" => Some(MonitoredSourceType::Local),
            _ => None,
        });

    let limit = params.get("limit")
        .and_then(|v| v.as_i64())
        .map(|l| l as i32);

    let error_tracker = SourceErrorTracker::new(state.db.clone());

    match error_tracker.get_retry_candidates(auth_user.user.id, source_type, limit).await {
        Ok(candidates) => {
            // Convert to response format
            let mut responses = Vec::new();
            for failure in candidates {
                if let Ok(Some(response)) = error_tracker.get_failure_details(auth_user.user.id, failure.id).await {
                    responses.push(response);
                }
            }
            
            info!("Found {} retry candidates", responses.len());
            Ok(Json(responses))
        }
        Err(e) => {
            error!("Failed to get retry candidates: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/source/errors/{failure_id}",
    tag = "source-errors",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("failure_id" = Uuid, Path, description = "Failure ID")
    ),
    responses(
        (status = 200, description = "Source scan failure details", body = SourceScanFailureResponse),
        (status = 404, description = "Failure not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_source_failure(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(failure_id): Path<Uuid>,
) -> Result<Json<SourceScanFailureResponse>, StatusCode> {
    info!("Getting source scan failure {} for user: {}", failure_id, auth_user.user.username);

    let error_tracker = SourceErrorTracker::new(state.db.clone());

    match error_tracker.get_failure_details(auth_user.user.id, failure_id).await {
        Ok(Some(failure)) => {
            info!("Found source scan failure: {}", failure.resource_path);
            Ok(Json(failure))
        }
        Ok(None) => {
            warn!("Source scan failure {} not found", failure_id);
            Err(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            error!("Failed to get source scan failure: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/source/errors/{failure_id}/retry",
    tag = "source-errors",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("failure_id" = Uuid, Path, description = "Failure ID")
    ),
    request_body = RetryFailureRequest,
    responses(
        (status = 200, description = "Failure retry scheduled"),
        (status = 404, description = "Failure not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn retry_source_failure(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(failure_id): Path<Uuid>,
    Json(request): Json<RetryFailureRequest>,
) -> Result<Json<Value>, StatusCode> {
    info!("Retrying source scan failure {} for user: {}", failure_id, auth_user.user.username);

    let error_tracker = SourceErrorTracker::new(state.db.clone());

    match error_tracker.retry_failure(auth_user.user.id, failure_id, request).await {
        Ok(true) => {
            info!("Successfully scheduled retry for failure {}", failure_id);
            Ok(Json(serde_json::json!({
                "success": true,
                "message": "Failure retry scheduled successfully"
            })))
        }
        Ok(false) => {
            warn!("Failed to schedule retry - failure {} not found", failure_id);
            Err(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            error!("Failed to retry source scan failure: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/source/errors/{failure_id}/exclude",
    tag = "source-errors",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("failure_id" = Uuid, Path, description = "Failure ID")
    ),
    request_body = ExcludeResourceRequest,
    responses(
        (status = 200, description = "Resource excluded from scanning"),
        (status = 404, description = "Failure not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn exclude_source_failure(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(failure_id): Path<Uuid>,
    Json(request): Json<ExcludeResourceRequest>,
) -> Result<Json<Value>, StatusCode> {
    info!("Excluding source scan failure {} for user: {}", failure_id, auth_user.user.username);

    let error_tracker = SourceErrorTracker::new(state.db.clone());

    match error_tracker.exclude_resource(auth_user.user.id, failure_id, request).await {
        Ok(true) => {
            info!("Successfully excluded resource for failure {}", failure_id);
            Ok(Json(serde_json::json!({
                "success": true,
                "message": "Resource excluded from scanning successfully"
            })))
        }
        Ok(false) => {
            warn!("Failed to exclude resource - failure {} not found", failure_id);
            Err(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            error!("Failed to exclude source scan failure: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/source/errors/{failure_id}/resolve",
    tag = "source-errors",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("failure_id" = Uuid, Path, description = "Failure ID")
    ),
    responses(
        (status = 200, description = "Failure resolved"),
        (status = 404, description = "Failure not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn resolve_source_failure(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(failure_id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    info!("Resolving source scan failure {} for user: {}", failure_id, auth_user.user.username);

    // Get the failure details first
    let error_tracker = SourceErrorTracker::new(state.db.clone());
    
    match error_tracker.get_failure_details(auth_user.user.id, failure_id).await {
        Ok(Some(failure)) => {
            // Resolve the failure by updating it directly
            match state.db.resolve_source_scan_failure(
                auth_user.user.id,
                failure.source_type,
                None, // source_id not available in response
                &failure.resource_path,
                "manual_resolution",
            ).await {
                Ok(true) => {
                    info!("Successfully resolved failure {}", failure_id);
                    Ok(Json(serde_json::json!({
                        "success": true,
                        "message": "Failure resolved successfully"
                    })))
                }
                Ok(false) => {
                    warn!("Failed to resolve failure {} - not found", failure_id);
                    Err(StatusCode::NOT_FOUND)
                }
                Err(e) => {
                    error!("Failed to resolve source scan failure: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        Ok(None) => {
            warn!("Source scan failure {} not found for resolution", failure_id);
            Err(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            error!("Failed to get source scan failure for resolution: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/source/errors/type/{source_type}",
    tag = "source-errors",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("source_type" = String, Path, description = "Source type (webdav, s3, local)"),
        ("error_type" = Option<String>, Query, description = "Filter by error type"),
        ("severity" = Option<String>, Query, description = "Filter by severity"),
        ("include_resolved" = Option<bool>, Query, description = "Include resolved failures"),
        ("include_excluded" = Option<bool>, Query, description = "Include excluded resources"),
        ("ready_for_retry" = Option<bool>, Query, description = "Only show failures ready for retry"),
        ("limit" = Option<i32>, Query, description = "Maximum number of results"),
        ("offset" = Option<i32>, Query, description = "Number of results to skip")
    ),
    responses(
        (status = 200, description = "List of source scan failures for specific type", body = Vec<SourceScanFailureResponse>),
        (status = 400, description = "Invalid source type"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn list_source_type_failures(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(source_type_str): Path<String>,
    Query(mut query): Query<ListFailuresQuery>,
) -> Result<Json<Vec<SourceScanFailureResponse>>, StatusCode> {
    info!("Listing {} scan failures for user: {}", source_type_str, auth_user.user.username);

    // Parse source type
    let source_type = match source_type_str.to_lowercase().as_str() {
        "webdav" => MonitoredSourceType::WebDAV,
        "s3" => MonitoredSourceType::S3,
        "local" => MonitoredSourceType::Local,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    // Set source type filter
    query.source_type = Some(source_type);

    let error_tracker = SourceErrorTracker::new(state.db.clone());

    match error_tracker.list_failures(auth_user.user.id, query).await {
        Ok(failures) => {
            info!("Found {} {} scan failures", failures.len(), source_type_str);
            Ok(Json(failures))
        }
        Err(e) => {
            error!("Failed to list {} scan failures: {}", source_type_str, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/source/errors/type/{source_type}/stats",
    tag = "source-errors",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("source_type" = String, Path, description = "Source type (webdav, s3, local)")
    ),
    responses(
        (status = 200, description = "Source scan failure statistics for specific type", body = SourceScanFailureStats),
        (status = 400, description = "Invalid source type"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_source_type_stats(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(source_type_str): Path<String>,
) -> Result<Json<SourceScanFailureStats>, StatusCode> {
    info!("Getting {} scan failure stats for user: {}", source_type_str, auth_user.user.username);

    // Parse source type
    let source_type = match source_type_str.to_lowercase().as_str() {
        "webdav" => MonitoredSourceType::WebDAV,
        "s3" => MonitoredSourceType::S3,
        "local" => MonitoredSourceType::Local,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    let error_tracker = SourceErrorTracker::new(state.db.clone());

    match error_tracker.get_stats(auth_user.user.id, Some(source_type)).await {
        Ok(stats) => {
            info!("Retrieved {} scan failure stats", source_type_str);
            Ok(Json(stats))
        }
        Err(e) => {
            error!("Failed to get {} scan failure stats: {}", source_type_str, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}