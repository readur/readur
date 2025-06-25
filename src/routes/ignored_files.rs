use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get},
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;
use utoipa::{OpenApi, ToSchema};

use crate::{
    auth::AuthUser,
    db::ignored_files,
    models::{IgnoredFilesQuery, IgnoredFileResponse},
    AppState,
};

#[derive(OpenApi)]
#[openapi(
    paths(
        list_ignored_files,
        get_ignored_file,
        delete_ignored_file,
        bulk_delete_ignored_files,
        get_ignored_files_stats,
    ),
    components(schemas(
        IgnoredFileResponse,
        IgnoredFilesQuery,
        BulkDeleteIgnoredFilesRequest,
        IgnoredFilesStats,
    )),
    tags(
        (name = "ignored_files", description = "Ignored files management endpoints")
    )
)]
pub struct IgnoredFilesApi;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BulkDeleteIgnoredFilesRequest {
    /// List of ignored file IDs to delete
    pub ignored_file_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct IgnoredFilesStats {
    /// Total number of ignored files for the user
    pub total_ignored_files: i64,
    /// Number of ignored files by source type
    pub by_source_type: Vec<SourceTypeCount>,
    /// Total size of ignored files in bytes
    pub total_size_bytes: i64,
    /// Most recent ignored file timestamp
    pub most_recent_ignored_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SourceTypeCount {
    pub source_type: Option<String>,
    pub count: i64,
    pub total_size_bytes: i64,
}

pub fn ignored_files_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_ignored_files))
        .route("/stats", get(get_ignored_files_stats))
        .route("/{id}", get(get_ignored_file))
        .route("/{id}", delete(delete_ignored_file))
        .route("/bulk-delete", delete(bulk_delete_ignored_files))
}

#[utoipa::path(
    get,
    path = "/api/ignored-files",
    tag = "ignored_files",
    security(
        ("bearer_auth" = [])
    ),
    params(IgnoredFilesQuery),
    responses(
        (status = 200, description = "List of ignored files", body = Vec<IgnoredFileResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_ignored_files(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Query(query): Query<IgnoredFilesQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let ignored_files = ignored_files::list_ignored_files(
        state.db.get_pool(),
        auth_user.user.id,
        &query,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to list ignored files: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let total_count = ignored_files::count_ignored_files(
        state.db.get_pool(),
        auth_user.user.id,
        &query,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to count ignored files: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(serde_json::json!({
        "ignored_files": ignored_files,
        "total": total_count,
        "limit": query.limit.unwrap_or(25),
        "offset": query.offset.unwrap_or(0)
    })))
}

#[utoipa::path(
    get,
    path = "/api/ignored-files/{id}",
    tag = "ignored_files",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = uuid::Uuid, Path, description = "Ignored file ID")
    ),
    responses(
        (status = 200, description = "Ignored file details", body = IgnoredFileResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Ignored file not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_ignored_file(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<IgnoredFileResponse>, StatusCode> {
    let ignored_file = ignored_files::get_ignored_file_by_id(
        state.db.get_pool(),
        id,
        auth_user.user.id,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to get ignored file: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(ignored_file))
}

#[utoipa::path(
    delete,
    path = "/api/ignored-files/{id}",
    tag = "ignored_files",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = uuid::Uuid, Path, description = "Ignored file ID")
    ),
    responses(
        (status = 200, description = "Ignored file deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Ignored file not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_ignored_file(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let deleted = ignored_files::delete_ignored_file(
        state.db.get_pool(),
        id,
        auth_user.user.id,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to delete ignored file: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if deleted {
        Ok(Json(serde_json::json!({
            "success": true,
            "message": "Ignored file deleted successfully",
            "id": id
        })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

#[utoipa::path(
    delete,
    path = "/api/ignored-files/bulk-delete",
    tag = "ignored_files",
    security(
        ("bearer_auth" = [])
    ),
    request_body(content = BulkDeleteIgnoredFilesRequest, description = "List of ignored file IDs to delete"),
    responses(
        (status = 200, description = "Ignored files deleted successfully"),
        (status = 400, description = "Bad request - no ignored file IDs provided"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn bulk_delete_ignored_files(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Json(request): Json<BulkDeleteIgnoredFilesRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if request.ignored_file_ids.is_empty() {
        return Ok(Json(serde_json::json!({
            "success": false,
            "message": "No ignored file IDs provided",
            "deleted_count": 0
        })));
    }

    let deleted_count = ignored_files::bulk_delete_ignored_files(
        state.db.get_pool(),
        request.ignored_file_ids.clone(),
        auth_user.user.id,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to bulk delete ignored files: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let requested_count = request.ignored_file_ids.len();
    let message = if deleted_count as usize == requested_count {
        format!("Successfully deleted {} ignored files", deleted_count)
    } else {
        format!("Deleted {} of {} requested ignored files", deleted_count, requested_count)
    };

    Ok(Json(serde_json::json!({
        "success": true,
        "message": message,
        "deleted_count": deleted_count,
        "requested_count": requested_count
    })))
}

#[utoipa::path(
    get,
    path = "/api/ignored-files/stats",
    tag = "ignored_files",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Ignored files statistics", body = IgnoredFilesStats),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_ignored_files_stats(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<IgnoredFilesStats>, StatusCode> {
    let stats_result = sqlx::query(
        r#"
        SELECT 
            COUNT(*) as total_ignored_files,
            COALESCE(SUM(file_size), 0) as total_size_bytes,
            MAX(ignored_at) as most_recent_ignored_at
        FROM ignored_files 
        WHERE ignored_by = $1
        "#
    )
    .bind(auth_user.user.id)
    .fetch_one(state.db.get_pool())
    .await
    .map_err(|e| {
        tracing::error!("Failed to get ignored files stats: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let by_source_type_result = sqlx::query(
        r#"
        SELECT 
            source_type,
            COUNT(*) as count,
            COALESCE(SUM(file_size), 0) as total_size_bytes
        FROM ignored_files 
        WHERE ignored_by = $1
        GROUP BY source_type
        ORDER BY count DESC
        "#
    )
    .bind(auth_user.user.id)
    .fetch_all(state.db.get_pool())
    .await
    .map_err(|e| {
        tracing::error!("Failed to get ignored files by source type: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let by_source_type = by_source_type_result
        .into_iter()
        .map(|row| SourceTypeCount {
            source_type: row.get("source_type"),
            count: row.get::<Option<i64>, _>("count").unwrap_or(0),
            total_size_bytes: row.get::<Option<i64>, _>("total_size_bytes").unwrap_or(0),
        })
        .collect();

    let stats = IgnoredFilesStats {
        total_ignored_files: stats_result.get::<Option<i64>, _>("total_ignored_files").unwrap_or(0),
        by_source_type,
        total_size_bytes: stats_result.get::<Option<i64>, _>("total_size_bytes").unwrap_or(0),
        most_recent_ignored_at: stats_result.get("most_recent_ignored_at"),
    };

    Ok(Json(stats))
}