use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post, delete},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::{
    auth::AuthUser,
    models::{Notification, NotificationSummary},
    AppState,
};

#[derive(Deserialize, ToSchema)]
struct PaginationQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_notifications))
        .route("/summary", get(get_notification_summary))
        .route("/{id}/read", post(mark_notification_read))
        .route("/read-all", post(mark_all_notifications_read))
        .route("/{id}", delete(delete_notification))
}

#[utoipa::path(
    get,
    path = "/api/notifications",
    tag = "notifications",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("limit" = Option<i64>, Query, description = "Number of notifications to return (default: 25)"),
        ("offset" = Option<i64>, Query, description = "Number of notifications to skip (default: 0)")
    ),
    responses(
        (status = 200, description = "List of user notifications", body = Vec<Notification>),
        (status = 401, description = "Unauthorized")
    )
)]
async fn get_notifications(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<Vec<Notification>>, StatusCode> {
    let limit = pagination.limit.unwrap_or(25);
    let offset = pagination.offset.unwrap_or(0);
    
    let notifications = state
        .db
        .get_user_notifications(auth_user.user.id, limit, offset)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(notifications))
}

#[utoipa::path(
    get,
    path = "/api/notifications/summary",
    tag = "notifications",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Notification summary with unread count and recent notifications", body = NotificationSummary),
        (status = 401, description = "Unauthorized")
    )
)]
async fn get_notification_summary(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<NotificationSummary>, StatusCode> {
    let summary = state
        .db
        .get_notification_summary(auth_user.user.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(summary))
}

#[utoipa::path(
    post,
    path = "/api/notifications/{id}/read",
    tag = "notifications",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = uuid::Uuid, Path, description = "Notification ID")
    ),
    responses(
        (status = 200, description = "Notification marked as read"),
        (status = 404, description = "Notification not found"),
        (status = 401, description = "Unauthorized")
    )
)]
async fn mark_notification_read(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(notification_id): Path<uuid::Uuid>,
) -> Result<StatusCode, StatusCode> {
    state
        .db
        .mark_notification_read(auth_user.user.id, notification_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/api/notifications/read-all",
    tag = "notifications",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "All notifications marked as read"),
        (status = 401, description = "Unauthorized")
    )
)]
async fn mark_all_notifications_read(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<StatusCode, StatusCode> {
    state
        .db
        .mark_all_notifications_read(auth_user.user.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(StatusCode::OK)
}

#[utoipa::path(
    delete,
    path = "/api/notifications/{id}",
    tag = "notifications",
    security(
        ("bearer_auth" = [])
    ),
    params(
        ("id" = uuid::Uuid, Path, description = "Notification ID")
    ),
    responses(
        (status = 200, description = "Notification deleted"),
        (status = 404, description = "Notification not found"),
        (status = 401, description = "Unauthorized")
    )
)]
async fn delete_notification(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(notification_id): Path<uuid::Uuid>,
) -> Result<StatusCode, StatusCode> {
    state
        .db
        .delete_notification(auth_user.user.id, notification_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(StatusCode::OK)
}