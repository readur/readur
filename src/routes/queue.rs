use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use crate::{auth::AuthUser, ocr_queue::OcrQueueService, AppState, models::UserRole};

fn require_admin(auth_user: &AuthUser) -> Result<(), StatusCode> {
    if auth_user.user.role != UserRole::Admin {
        Err(StatusCode::FORBIDDEN)
    } else {
        Ok(())
    }
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/stats", get(get_queue_stats))
        .route("/requeue-failed", post(requeue_failed))
        .route("/pause", post(pause_ocr_processing))
        .route("/resume", post(resume_ocr_processing))
        .route("/status", get(get_ocr_status))
}

#[utoipa::path(
    get,
    path = "/api/queue/stats",
    tag = "queue",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "OCR queue statistics including pending jobs, processing status, and performance metrics"),
        (status = 401, description = "Unauthorized - valid authentication required"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_queue_stats(
    State(state): State<Arc<AppState>>,
    _auth_user: AuthUser, // Require authentication
) -> Result<Json<serde_json::Value>, StatusCode> {
    let queue_service = OcrQueueService::new(state.db.clone(), state.db.get_pool().clone(), 1);
    
    let stats = queue_service
        .get_stats()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(serde_json::json!({
        "pending": stats.pending_count,
        "processing": stats.processing_count,
        "failed": stats.failed_count,
        "completed_today": stats.completed_today,
        "avg_wait_time_minutes": stats.avg_wait_time_minutes,
        "oldest_pending_minutes": stats.oldest_pending_minutes,
    })))
}

#[utoipa::path(
    post,
    path = "/api/queue/requeue-failed",
    tag = "queue",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Failed items requeued successfully"),
        (status = 401, description = "Unauthorized")
    )
)]
async fn requeue_failed(
    State(state): State<Arc<AppState>>,
    _auth_user: AuthUser, // Require authentication
) -> Result<Json<serde_json::Value>, StatusCode> {
    let queue_service = OcrQueueService::new(state.db.clone(), state.db.get_pool().clone(), 1);
    
    let count = queue_service
        .requeue_failed_items()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(serde_json::json!({
        "requeued_count": count,
    })))
}

#[utoipa::path(
    post,
    path = "/api/queue/pause",
    tag = "queue",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "OCR processing paused successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Admin access required")
    )
)]
async fn pause_ocr_processing(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<serde_json::Value>, StatusCode> {
    require_admin(&auth_user)?;
    
    state.queue_service.pause();
    
    Ok(Json(serde_json::json!({
        "status": "paused",
        "message": "OCR processing has been paused"
    })))
}

#[utoipa::path(
    post,
    path = "/api/queue/resume",
    tag = "queue",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "OCR processing resumed successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Admin access required")
    )
)]
async fn resume_ocr_processing(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<serde_json::Value>, StatusCode> {
    require_admin(&auth_user)?;
    
    state.queue_service.resume();
    
    Ok(Json(serde_json::json!({
        "status": "resumed",
        "message": "OCR processing has been resumed"
    })))
}

#[utoipa::path(
    get,
    path = "/api/queue/status",
    tag = "queue",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "OCR processing status"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Admin access required")
    )
)]
async fn get_ocr_status(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<serde_json::Value>, StatusCode> {
    require_admin(&auth_user)?;
    
    let is_paused = state.queue_service.is_paused();
    
    Ok(Json(serde_json::json!({
        "is_paused": is_paused,
        "status": if is_paused { "paused" } else { "running" }
    })))
}