use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use std::sync::Arc;
use serde::Serialize;
use utoipa::ToSchema;

use crate::{auth::AuthUser, AppState, models::UserRole};

fn require_admin(auth_user: &AuthUser) -> Result<(), StatusCode> {
    if auth_user.user.role != UserRole::Admin {
        Err(StatusCode::FORBIDDEN)
    } else {
        Ok(())
    }
}

#[derive(Serialize, ToSchema)]
pub struct SystemMetrics {
    pub database: DatabaseMetrics,
    pub ocr: OcrMetrics,
    pub documents: DocumentMetrics,
    pub users: UserMetrics,
    pub system: GeneralSystemMetrics,
    pub timestamp: i64,
}

#[derive(Serialize, ToSchema)]
pub struct DatabaseMetrics {
    pub active_connections: i32,
    pub total_queries_today: i64,
    pub avg_query_time_ms: f64,
}

#[derive(Serialize, ToSchema)]
pub struct OcrMetrics {
    pub pending_jobs: i64,
    pub processing_jobs: i64,
    pub failed_jobs: i64,
    pub completed_today: i64,
    pub avg_processing_time_minutes: Option<f64>,
    pub queue_depth: i64,
    pub oldest_pending_minutes: Option<f64>,
}

#[derive(Serialize, ToSchema)]
pub struct DocumentMetrics {
    pub total_documents: i64,
    pub documents_uploaded_today: i64,
    pub total_storage_bytes: i64,
    pub avg_document_size_bytes: f64,
    pub documents_with_ocr: i64,
    pub documents_without_ocr: i64,
}

#[derive(Serialize, ToSchema)]
pub struct UserMetrics {
    pub total_users: i64,
    pub active_users_today: i64,
    pub new_registrations_today: i64,
}

#[derive(Serialize, ToSchema)]
pub struct GeneralSystemMetrics {
    pub uptime_seconds: u64,
    pub app_version: String,
    pub rust_version: String,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_system_metrics))
}

#[utoipa::path(
    get,
    path = "/api/metrics",
    tag = "metrics",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "System metrics and monitoring data", body = SystemMetrics),
        (status = 401, description = "Unauthorized - valid authentication required"),
        (status = 403, description = "Forbidden - Admin access required"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_system_metrics(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<SystemMetrics>, StatusCode> {
    require_admin(&auth_user)?;
    let timestamp = chrono::Utc::now().timestamp();
    
    // Collect all metrics concurrently for better performance
    let (database_metrics, ocr_metrics, document_metrics, user_metrics, system_metrics) = tokio::try_join!(
        collect_database_metrics(&state),
        collect_ocr_metrics(&state),
        collect_document_metrics(&state),
        collect_user_metrics(&state),
        collect_system_metrics()
    )?;
    
    let metrics = SystemMetrics {
        database: database_metrics,
        ocr: ocr_metrics,
        documents: document_metrics,
        users: user_metrics,
        system: system_metrics,
        timestamp,
    };
    
    Ok(Json(metrics))
}

async fn collect_database_metrics(state: &Arc<AppState>) -> Result<DatabaseMetrics, StatusCode> {
    // Get connection pool information
    let _pool_info = state.db.pool.options();
    let active_connections = state.db.pool.size() as i32;
    
    // For now, use placeholder values for queries
    // In production, you might want to implement query tracking
    Ok(DatabaseMetrics {
        active_connections,
        total_queries_today: 0, // Placeholder - would need query tracking
        avg_query_time_ms: 0.0, // Placeholder - would need query timing
    })
}

async fn collect_ocr_metrics(state: &Arc<AppState>) -> Result<OcrMetrics, StatusCode> {
    // Use existing OCR queue statistics
    
    
    let queue_service = &*state.queue_service;
    
    let stats = queue_service
        .get_stats()
        .await
        .map_err(|e| {
            tracing::error!("Failed to get OCR stats: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    Ok(OcrMetrics {
        pending_jobs: stats.pending_count,
        processing_jobs: stats.processing_count,
        failed_jobs: stats.failed_count,
        completed_today: stats.completed_today,
        avg_processing_time_minutes: stats.avg_wait_time_minutes,
        queue_depth: stats.pending_count + stats.processing_count,
        oldest_pending_minutes: stats.oldest_pending_minutes,
    })
}

async fn collect_document_metrics(state: &Arc<AppState>) -> Result<DocumentMetrics, StatusCode> {
    // Get total document count using retry mechanism
    let total_docs = state.db.with_retry(|| async {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM documents")
            .fetch_one(&state.db.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get total document count: {}", e))
    }).await.map_err(|e| {
        tracing::error!("Failed to get total document count: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    // Get documents uploaded today
    let docs_today = state.db.with_retry(|| async {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM documents WHERE DATE(created_at) = CURRENT_DATE"
        )
        .fetch_one(&state.db.pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get today's document count: {}", e))
    }).await.map_err(|e| {
        tracing::error!("Failed to get today's document count: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    // Get total storage size
    let total_size = state.db.with_retry(|| async {
        sqlx::query_scalar::<_, Option<f64>>("SELECT CAST(COALESCE(SUM(file_size), 0) AS DOUBLE PRECISION) FROM documents")
            .fetch_one(&state.db.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get total storage size: {}", e))
    }).await.map_err(|e| {
        tracing::error!("Failed to get total storage size: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?.unwrap_or(0.0) as i64;
    
    // Get documents with and without OCR
    let docs_with_ocr = state.db.with_retry(|| async {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM documents WHERE ocr_text IS NOT NULL AND ocr_text != ''"
        )
        .fetch_one(&state.db.pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get OCR document count: {}", e))
    }).await.map_err(|e| {
        tracing::error!("Failed to get OCR document count: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let docs_without_ocr = total_docs - docs_with_ocr;
    
    let avg_size = if total_docs > 0 {
        total_size as f64 / total_docs as f64
    } else {
        0.0
    };
    
    Ok(DocumentMetrics {
        total_documents: total_docs,
        documents_uploaded_today: docs_today,
        total_storage_bytes: total_size,
        avg_document_size_bytes: avg_size,
        documents_with_ocr: docs_with_ocr,
        documents_without_ocr: docs_without_ocr,
    })
}

async fn collect_user_metrics(state: &Arc<AppState>) -> Result<UserMetrics, StatusCode> {
    // Get total user count
    let total_users = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get total user count: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // Get new users today
    let new_users_today = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM users WHERE DATE(created_at) = CURRENT_DATE"
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get new user count: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    // For active users, count users who uploaded documents today
    let active_users_today = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(DISTINCT user_id) FROM documents WHERE DATE(created_at) = CURRENT_DATE"
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get active user count: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    Ok(UserMetrics {
        total_users,
        active_users_today,
        new_registrations_today: new_users_today,
    })
}

async fn collect_system_metrics() -> Result<GeneralSystemMetrics, StatusCode> {
    // Get application uptime (this is a simplified version)
    // In a real application, you'd track the start time
    let uptime_seconds = 3600; // Placeholder
    
    // Get version information
    let app_version = env!("CARGO_PKG_VERSION").to_string();
    let rust_version = std::env::var("RUST_VERSION").unwrap_or_else(|_| "unknown".to_string());
    
    Ok(GeneralSystemMetrics {
        uptime_seconds,
        app_version,
        rust_version,
    })
}