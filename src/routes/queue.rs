use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use sqlx::Row;
use std::{sync::Arc, error::Error};

use crate::{auth::AuthUser, AppState, models::UserRole};

pub fn require_admin(auth_user: &AuthUser) -> Result<(), StatusCode> {
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
        .route("/enqueue-pending", post(enqueue_pending_documents))
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
        (status = 403, description = "Forbidden - Admin access required"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_queue_stats(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<serde_json::Value>, StatusCode> {
    require_admin(&auth_user)?;
    let queue_service = &*state.queue_service;
    
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
    path = "/api/queue/requeue/failed",
    tag = "queue",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Failed items requeued successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Admin access required"),
        (status = 500, description = "Internal server error")
    )
)]
async fn requeue_failed(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<serde_json::Value>, StatusCode> {
    require_admin(&auth_user)?;
    let queue_service = &*state.queue_service;
    
    let count = match queue_service.requeue_failed_items().await {
        Ok(count) => count,
        Err(e) => {
            let error_msg = format!("Failed to requeue failed items: {:?}", e);
            tracing::error!("{}", error_msg);
            
            // Print to stderr so we can see it in test output
            eprintln!("REQUEUE ERROR: {}", error_msg);
            
            // Try to get the source chain
            eprintln!("Error chain:");
            let mut source = e.source();
            let mut depth = 1;
            while let Some(err) = source {
                eprintln!("  {}: {}", depth, err);
                source = err.source();
                depth += 1;
            }
            
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    
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

#[utoipa::path(
    post,
    path = "/api/queue/enqueue-pending",
    tag = "queue",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Pending documents queued successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Admin access required"),
        (status = 500, description = "Internal server error")
    )
)]
async fn enqueue_pending_documents(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<serde_json::Value>, StatusCode> {
    require_admin(&auth_user)?;
    
    // Find all documents with pending OCR status that aren't already in the queue
    let pending_documents = sqlx::query(
        r#"
        SELECT d.id, d.file_size
        FROM documents d
        LEFT JOIN ocr_queue oq ON d.id = oq.document_id
        WHERE d.ocr_status = 'pending'
          AND oq.document_id IS NULL
          AND d.file_path IS NOT NULL
          AND (d.mime_type LIKE 'image/%' OR d.mime_type = 'application/pdf' OR d.mime_type = 'text/plain')
        ORDER BY d.created_at ASC
        "#
    )
    .fetch_all(state.db.get_pool())
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if pending_documents.is_empty() {
        return Ok(Json(serde_json::json!({
            "queued_count": 0,
            "message": "No pending documents found to queue"
        })));
    }
    
    // Prepare batch insert data
    let documents_to_queue: Vec<(uuid::Uuid, i32, i64)> = pending_documents
        .into_iter()
        .map(|row| {
            let document_id: uuid::Uuid = row.get("id");
            let file_size: i64 = row.get("file_size");
            
            // Calculate priority based on file size
            let priority = match file_size {
                0..=1048576 => 10,          // <= 1MB: highest priority
                ..=5242880 => 8,            // 1-5MB: high priority  
                ..=10485760 => 6,           // 5-10MB: medium priority
                ..=52428800 => 4,           // 10-50MB: low priority
                _ => 2,                     // > 50MB: lowest priority
            };
            
            (document_id, priority, file_size)
        })
        .collect();
    
    // Batch enqueue documents
    let queue_ids = state.queue_service
        .enqueue_documents_batch(documents_to_queue)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(serde_json::json!({
        "queued_count": queue_ids.len(),
        "message": format!("Successfully queued {} pending documents for OCR processing", queue_ids.len()),
        "queue_ids": queue_ids
    })))
}