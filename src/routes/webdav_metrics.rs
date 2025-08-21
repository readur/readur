use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use utoipa::{ToSchema, IntoParams};
use uuid::Uuid;

use crate::{auth::AuthUser, AppState};
use crate::models::webdav_metrics::*;
use crate::services::webdav_metrics_tracker::WebDAVMetricsTracker;

/// Validate and normalize a limit parameter
fn validate_limit(limit: Option<i32>) -> Option<i32> {
    match limit {
        Some(l) if l < 1 => {
            tracing::warn!("Invalid limit parameter: {} (must be at least 1)", l);
            None
        }
        Some(l) if l > 1000 => {
            tracing::warn!("Limit parameter {} exceeds maximum, capping at 1000", l);
            Some(1000)
        }
        Some(l) => Some(l),
        None => None,
    }
}

/// Validate and normalize an offset parameter
fn validate_offset(offset: Option<i32>) -> Option<i32> {
    match offset {
        Some(o) if o < 0 => {
            tracing::warn!("Invalid offset parameter: {} (must be non-negative)", o);
            None
        }
        Some(o) => Some(o),
        None => None,
    }
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/sessions", get(list_webdav_sessions))
        .route("/sessions/:session_id", get(get_webdav_session))
        .route("/sessions/:session_id/insights", get(get_session_performance_insights))
        .route("/sessions/:session_id/directories", get(get_session_directory_metrics))
        .route("/sessions/:session_id/requests", get(get_session_request_metrics))
        .route("/summary", get(get_webdav_metrics_summary))
        .route("/performance", get(get_webdav_performance_overview))
}

/// Query parameters for listing WebDAV sessions
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct ListSessionsQuery {
    pub source_id: Option<Uuid>,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl ListSessionsQuery {
    /// Validate and normalize query parameters
    pub fn validate(&self) -> Result<Self, String> {
        // Validate limit
        let limit = match self.limit {
            Some(l) if l < 1 => return Err("limit must be at least 1".to_string()),
            Some(l) if l > 1000 => return Err("limit cannot exceed 1000".to_string()),
            Some(l) => Some(l),
            None => None,
        };

        // Validate offset
        let offset = match self.offset {
            Some(o) if o < 0 => return Err("offset must be non-negative".to_string()),
            Some(o) => Some(o),
            None => None,
        };

        Ok(Self {
            source_id: self.source_id,
            start_time: self.start_time,
            end_time: self.end_time,
            limit,
            offset,
        })
    }
}

/// Query parameters for metrics summary
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct MetricsSummaryQuery {
    pub source_id: Option<Uuid>,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
}

/// Performance overview response
#[derive(Debug, Serialize, ToSchema)]
pub struct WebDAVPerformanceOverview {
    pub recent_sessions: Vec<WebDAVSyncSession>,
    pub summary_stats: WebDAVMetricsSummary,
    pub top_slow_directories: Vec<SlowDirectoryInfo>,
    pub error_trends: ErrorTrendData,
    pub performance_recommendations: Vec<PerformanceRecommendation>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorTrendData {
    pub total_errors_last_24h: i32,
    pub error_rate_trend: f64, // Percentage change from previous period
    pub common_error_types: Vec<ErrorTypeCount>,
    pub most_problematic_sources: Vec<ProblematicSource>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorTypeCount {
    pub error_type: String,
    pub count: i32,
    pub percentage: f64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProblematicSource {
    pub source_id: Option<Uuid>,
    pub source_name: Option<String>,
    pub error_count: i32,
    pub last_error: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PerformanceRecommendation {
    pub category: String,
    pub title: String,
    pub description: String,
    pub priority: String, // "high", "medium", "low"
    pub potential_impact: String,
}

/// List WebDAV sync sessions for the current user
#[utoipa::path(
    get,
    path = "/api/webdav-metrics/sessions",
    tag = "webdav-metrics",
    security(("bearer_auth" = [])),
    params(ListSessionsQuery),
    responses(
        (status = 200, description = "List of WebDAV sync sessions", body = Vec<WebDAVSyncSession>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_webdav_sessions(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Query(query): Query<ListSessionsQuery>,
) -> Result<Json<Vec<WebDAVSyncSession>>, StatusCode> {
    // Validate query parameters
    let validated_query = query.validate().map_err(|e| {
        tracing::warn!("Invalid query parameters: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    let metrics_tracker = WebDAVMetricsTracker::new(state.db.clone());
    
    let metrics_query = WebDAVMetricsQuery {
        user_id: Some(auth_user.user.id),
        source_id: validated_query.source_id,
        start_time: validated_query.start_time,
        end_time: validated_query.end_time,
        limit: validated_query.limit,
        offset: validated_query.offset,
    };

    let sessions = metrics_tracker
        .list_sessions(&metrics_query)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list WebDAV sessions: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(sessions))
}

/// Get details for a specific WebDAV sync session
#[utoipa::path(
    get,
    path = "/api/webdav-metrics/sessions/{session_id}",
    tag = "webdav-metrics",
    security(("bearer_auth" = [])),
    params(
        ("session_id" = Uuid, Path, description = "Session ID")
    ),
    responses(
        (status = 200, description = "WebDAV sync session details", body = WebDAVSyncSession),
        (status = 404, description = "Session not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_webdav_session(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(session_id): Path<Uuid>,
) -> Result<Json<WebDAVSyncSession>, StatusCode> {
    let metrics_tracker = WebDAVMetricsTracker::new(state.db.clone());
    
    let session = metrics_tracker
        .get_session_details(session_id, auth_user.user.id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get WebDAV session {}: {}", session_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(session))
}

/// Get performance insights for a specific session
#[utoipa::path(
    get,
    path = "/api/webdav-metrics/sessions/{session_id}/insights",
    tag = "webdav-metrics",
    security(("bearer_auth" = [])),
    params(
        ("session_id" = Uuid, Path, description = "Session ID")
    ),
    responses(
        (status = 200, description = "Session performance insights", body = WebDAVPerformanceInsights),
        (status = 404, description = "Session not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_session_performance_insights(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(session_id): Path<Uuid>,
) -> Result<Json<WebDAVPerformanceInsights>, StatusCode> {
    let metrics_tracker = WebDAVMetricsTracker::new(state.db.clone());
    
    let insights = metrics_tracker
        .get_performance_insights(session_id, auth_user.user.id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get performance insights for session {}: {}", session_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(insights))
}

/// Get directory metrics for a specific session
#[utoipa::path(
    get,
    path = "/api/webdav-metrics/sessions/{session_id}/directories",
    tag = "webdav-metrics",
    security(("bearer_auth" = [])),
    params(
        ("session_id" = Uuid, Path, description = "Session ID")
    ),
    responses(
        (status = 200, description = "Directory metrics for the session", body = Vec<WebDAVDirectoryMetric>),
        (status = 404, description = "Session not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_session_directory_metrics(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(session_id): Path<Uuid>,
) -> Result<Json<Vec<WebDAVDirectoryMetric>>, StatusCode> {
    let metrics_tracker = WebDAVMetricsTracker::new(state.db.clone());
    
    let directory_metrics = metrics_tracker
        .get_directory_metrics(session_id, auth_user.user.id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get directory metrics for session {}: {}", session_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(directory_metrics))
}

/// Get request metrics for a specific session
#[utoipa::path(
    get,
    path = "/api/webdav-metrics/sessions/{session_id}/requests",
    tag = "webdav-metrics",
    security(("bearer_auth" = [])),
    params(
        ("session_id" = Uuid, Path, description = "Session ID"),
        ("limit" = Option<i32>, Query, description = "Maximum number of requests to return")
    ),
    responses(
        (status = 200, description = "HTTP request metrics for the session", body = Vec<WebDAVRequestMetric>),
        (status = 404, description = "Session not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_session_request_metrics(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(session_id): Path<Uuid>,
    Query(query): Query<serde_json::Value>,
) -> Result<Json<Vec<WebDAVRequestMetric>>, StatusCode> {
    let metrics_tracker = WebDAVMetricsTracker::new(state.db.clone());
    
    let limit = query.get("limit")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
        .and_then(|l| {
            if l < 1 {
                tracing::warn!("Invalid limit parameter: {} (must be at least 1)", l);
                None
            } else if l > 1000 {
                tracing::warn!("Invalid limit parameter: {} (cannot exceed 1000)", l);
                Some(1000) // Cap at maximum allowed
            } else {
                Some(l)
            }
        });
    
    let request_metrics = metrics_tracker
        .get_request_metrics(Some(session_id), None, auth_user.user.id, limit)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get request metrics for session {}: {}", session_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(request_metrics))
}

/// Get WebDAV metrics summary
#[utoipa::path(
    get,
    path = "/api/webdav-metrics/summary",
    tag = "webdav-metrics",
    security(("bearer_auth" = [])),
    params(MetricsSummaryQuery),
    responses(
        (status = 200, description = "WebDAV metrics summary", body = WebDAVMetricsSummary),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_webdav_metrics_summary(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Query(query): Query<MetricsSummaryQuery>,
) -> Result<Json<WebDAVMetricsSummary>, StatusCode> {
    let metrics_tracker = WebDAVMetricsTracker::new(state.db.clone());
    
    let metrics_query = WebDAVMetricsQuery {
        user_id: Some(auth_user.user.id),
        source_id: query.source_id,
        start_time: query.start_time,
        end_time: query.end_time,
        limit: None,
        offset: None,
    };

    let summary = metrics_tracker
        .get_metrics_summary(&metrics_query)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get WebDAV metrics summary: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .unwrap_or_else(|| {
            // Return empty summary if no data found
            WebDAVMetricsSummary {
                total_sessions: 0,
                successful_sessions: 0,
                failed_sessions: 0,
                total_files_processed: 0,
                total_bytes_processed: 0,
                avg_session_duration_sec: 0.0,
                avg_processing_rate: 0.0,
                total_http_requests: 0,
                request_success_rate: 0.0,
                avg_request_duration_ms: 0.0,
                common_error_types: serde_json::json!([]),
            }
        });

    Ok(Json(summary))
}

/// Get comprehensive WebDAV performance overview
#[utoipa::path(
    get,
    path = "/api/webdav-metrics/performance",
    tag = "webdav-metrics",
    security(("bearer_auth" = [])),
    params(MetricsSummaryQuery),
    responses(
        (status = 200, description = "WebDAV performance overview", body = WebDAVPerformanceOverview),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_webdav_performance_overview(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Query(query): Query<MetricsSummaryQuery>,
) -> Result<Json<WebDAVPerformanceOverview>, StatusCode> {
    let metrics_tracker = WebDAVMetricsTracker::new(state.db.clone());
    
    // Get recent sessions (last 10) - enforce reasonable limit
    let limited_sessions_limit = Some(10);
    let recent_sessions_query = WebDAVMetricsQuery {
        user_id: Some(auth_user.user.id),
        source_id: query.source_id,
        start_time: query.start_time,
        end_time: query.end_time,
        limit: limited_sessions_limit,
        offset: None,
    };

    let recent_sessions = metrics_tracker
        .list_sessions(&recent_sessions_query)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get recent WebDAV sessions: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get summary stats
    let summary_query = WebDAVMetricsQuery {
        user_id: Some(auth_user.user.id),
        source_id: query.source_id,
        start_time: query.start_time,
        end_time: query.end_time,
        limit: None,
        offset: None,
    };

    let summary_stats = metrics_tracker
        .get_metrics_summary(&summary_query)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get WebDAV metrics summary: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .unwrap_or_else(|| WebDAVMetricsSummary {
            total_sessions: 0,
            successful_sessions: 0,
            failed_sessions: 0,
            total_files_processed: 0,
            total_bytes_processed: 0,
            avg_session_duration_sec: 0.0,
            avg_processing_rate: 0.0,
            total_http_requests: 0,
            request_success_rate: 0.0,
            avg_request_duration_ms: 0.0,
            common_error_types: serde_json::json!([]),
        });

    // Analyze performance and generate recommendations
    let top_slow_directories = get_slow_directories_for_user(&recent_sessions, &metrics_tracker, auth_user.user.id).await;
    let error_trends = analyze_error_trends(&summary_stats);
    let performance_recommendations = generate_performance_recommendations(&summary_stats, &recent_sessions);

    let overview = WebDAVPerformanceOverview {
        recent_sessions,
        summary_stats,
        top_slow_directories,
        error_trends,
        performance_recommendations,
    };

    Ok(Json(overview))
}

/// Helper function to get slow directories across recent sessions
async fn get_slow_directories_for_user(
    sessions: &[WebDAVSyncSession],
    metrics_tracker: &WebDAVMetricsTracker,
    user_id: Uuid,
) -> Vec<SlowDirectoryInfo> {
    let mut all_slow_directories = Vec::new();

    for session in sessions.iter().take(5) { // Check last 5 sessions
        if let Ok(Some(insights)) = metrics_tracker
            .get_performance_insights(session.id, user_id)
            .await
        {
            all_slow_directories.extend(insights.slowest_directories);
        }
    }

    // Sort by scan duration and take top 10
    all_slow_directories.sort_by(|a, b| b.scan_duration_ms.cmp(&a.scan_duration_ms));
    all_slow_directories.into_iter().take(10).collect()
}

/// Analyze error trends from summary stats
fn analyze_error_trends(summary: &WebDAVMetricsSummary) -> ErrorTrendData {
    let total_requests = summary.total_http_requests as f64;
    let failed_requests = total_requests - (total_requests * summary.request_success_rate / 100.0);
    
    let common_error_types = if let Some(error_array) = summary.common_error_types.as_array() {
        error_array
            .iter()
            .filter_map(|v| {
                let obj = v.as_object()?;
                let error_type = obj.get("error_type")?.as_str()?.to_string();
                let count = obj.get("count")?.as_i64()? as i32;
                let percentage = if failed_requests > 0.0 {
                    (count as f64 / failed_requests) * 100.0
                } else {
                    0.0
                };
                Some(ErrorTypeCount {
                    error_type,
                    count,
                    percentage,
                })
            })
            .collect()
    } else {
        Vec::new()
    };

    ErrorTrendData {
        total_errors_last_24h: failed_requests as i32,
        error_rate_trend: 0.0, // Would calculate from historical data
        common_error_types,
        most_problematic_sources: Vec::new(), // Would analyze by source
    }
}

/// Generate performance recommendations based on metrics
fn generate_performance_recommendations(
    summary: &WebDAVMetricsSummary,
    sessions: &[WebDAVSyncSession],
) -> Vec<PerformanceRecommendation> {
    let mut recommendations = Vec::new();

    // Analyze success rate
    let success_rate = summary.request_success_rate;
    if success_rate < 90.0 {
        recommendations.push(PerformanceRecommendation {
            category: "reliability".to_string(),
            title: "Low Success Rate Detected".to_string(),
            description: format!(
                "Your WebDAV requests have a {:.1}% success rate. Consider checking network connectivity and server configuration.",
                success_rate
            ),
            priority: if success_rate < 70.0 { "high" } else { "medium" }.to_string(),
            potential_impact: "Improved sync reliability and reduced failures".to_string(),
        });
    }

    // Analyze response times
    let avg_response_time = summary.avg_request_duration_ms;
    if avg_response_time > 2000.0 {
        recommendations.push(PerformanceRecommendation {
            category: "performance".to_string(),
            title: "Slow Response Times".to_string(),
            description: format!(
                "Average request time is {:.0}ms. Consider checking network conditions or server performance.",
                avg_response_time
            ),
            priority: if avg_response_time > 5000.0 { "high" } else { "medium" }.to_string(),
            potential_impact: "Faster sync operations and improved user experience".to_string(),
        });
    }

    // Analyze session patterns
    let recent_failed_sessions = sessions.iter()
        .filter(|s| s.status == "failed")
        .count();
    
    if recent_failed_sessions > sessions.len() / 4 {
        recommendations.push(PerformanceRecommendation {
            category: "reliability".to_string(),
            title: "Frequent Sync Failures".to_string(),
            description: format!(
                "{} of your last {} sync sessions failed. Review error logs and server connectivity.",
                recent_failed_sessions, sessions.len()
            ),
            priority: "high".to_string(),
            potential_impact: "More reliable syncing and data consistency".to_string(),
        });
    }

    // Processing rate analysis
    let avg_processing_rate = summary.avg_processing_rate;
    if avg_processing_rate < 1.0 && summary.total_files_processed > 0 {
        recommendations.push(PerformanceRecommendation {
            category: "performance".to_string(),
            title: "Low Processing Rate".to_string(),
            description: format!(
                "Processing rate is {:.2} files/second. Consider optimizing file selection or increasing concurrency.",
                avg_processing_rate
            ),
            priority: "medium".to_string(),
            potential_impact: "Faster sync completion times".to_string(),
        });
    }

    // If no recommendations, add a positive note
    if recommendations.is_empty() {
        recommendations.push(PerformanceRecommendation {
            category: "general".to_string(),
            title: "Good Performance".to_string(),
            description: "Your WebDAV sync operations are performing well with good success rates and response times.".to_string(),
            priority: "low".to_string(),
            potential_impact: "Continue monitoring for optimal performance".to_string(),
        });
    }

    recommendations
}