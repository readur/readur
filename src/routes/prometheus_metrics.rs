use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use std::sync::Arc;
use std::fmt::Write;

use crate::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_prometheus_metrics))
}

/// Returns metrics in Prometheus text format (text/plain; version=0.0.4)
pub async fn get_prometheus_metrics(
    State(state): State<Arc<AppState>>,
) -> Result<Response, StatusCode> {
    let mut output = String::new();
    
    // Get current timestamp
    let timestamp = chrono::Utc::now().timestamp_millis();
    
    // Collect all metrics
    let (document_metrics, ocr_metrics, user_metrics) = tokio::try_join!(
        collect_document_metrics(&state),
        collect_ocr_metrics(&state),
        collect_user_metrics(&state)
    )?;
    
    // Write Prometheus formatted metrics
    
    // Document metrics
    writeln!(&mut output, "# HELP readur_documents_total Total number of documents").unwrap();
    writeln!(&mut output, "# TYPE readur_documents_total gauge").unwrap();
    writeln!(&mut output, "readur_documents_total {} {}", document_metrics.total_documents, timestamp).unwrap();
    
    writeln!(&mut output, "# HELP readur_documents_uploaded_today Documents uploaded today").unwrap();
    writeln!(&mut output, "# TYPE readur_documents_uploaded_today gauge").unwrap();
    writeln!(&mut output, "readur_documents_uploaded_today {} {}", document_metrics.documents_uploaded_today, timestamp).unwrap();
    
    writeln!(&mut output, "# HELP readur_storage_bytes Total storage used in bytes").unwrap();
    writeln!(&mut output, "# TYPE readur_storage_bytes gauge").unwrap();
    writeln!(&mut output, "readur_storage_bytes {} {}", document_metrics.total_storage_bytes, timestamp).unwrap();
    
    writeln!(&mut output, "# HELP readur_documents_with_ocr Documents with OCR text").unwrap();
    writeln!(&mut output, "# TYPE readur_documents_with_ocr gauge").unwrap();
    writeln!(&mut output, "readur_documents_with_ocr {} {}", document_metrics.documents_with_ocr, timestamp).unwrap();
    
    writeln!(&mut output, "# HELP readur_documents_without_ocr Documents without OCR text").unwrap();
    writeln!(&mut output, "# TYPE readur_documents_without_ocr gauge").unwrap();
    writeln!(&mut output, "readur_documents_without_ocr {} {}", document_metrics.documents_without_ocr, timestamp).unwrap();
    
    // OCR metrics
    writeln!(&mut output, "# HELP readur_ocr_queue_pending OCR jobs pending").unwrap();
    writeln!(&mut output, "# TYPE readur_ocr_queue_pending gauge").unwrap();
    writeln!(&mut output, "readur_ocr_queue_pending {} {}", ocr_metrics.pending_jobs, timestamp).unwrap();
    
    writeln!(&mut output, "# HELP readur_ocr_queue_processing OCR jobs currently processing").unwrap();
    writeln!(&mut output, "# TYPE readur_ocr_queue_processing gauge").unwrap();
    writeln!(&mut output, "readur_ocr_queue_processing {} {}", ocr_metrics.processing_jobs, timestamp).unwrap();
    
    writeln!(&mut output, "# HELP readur_ocr_queue_failed OCR jobs failed").unwrap();
    writeln!(&mut output, "# TYPE readur_ocr_queue_failed gauge").unwrap();
    writeln!(&mut output, "readur_ocr_queue_failed {} {}", ocr_metrics.failed_jobs, timestamp).unwrap();
    
    writeln!(&mut output, "# HELP readur_ocr_completed_today OCR jobs completed today").unwrap();
    writeln!(&mut output, "# TYPE readur_ocr_completed_today gauge").unwrap();
    writeln!(&mut output, "readur_ocr_completed_today {} {}", ocr_metrics.completed_today, timestamp).unwrap();
    
    if let Some(avg_time) = ocr_metrics.avg_processing_time_minutes {
        writeln!(&mut output, "# HELP readur_ocr_avg_processing_minutes Average OCR processing time in minutes").unwrap();
        writeln!(&mut output, "# TYPE readur_ocr_avg_processing_minutes gauge").unwrap();
        writeln!(&mut output, "readur_ocr_avg_processing_minutes {} {}", avg_time, timestamp).unwrap();
    }
    
    // User metrics
    writeln!(&mut output, "# HELP readur_users_total Total number of users").unwrap();
    writeln!(&mut output, "# TYPE readur_users_total gauge").unwrap();
    writeln!(&mut output, "readur_users_total {} {}", user_metrics.total_users, timestamp).unwrap();
    
    writeln!(&mut output, "# HELP readur_users_active_today Active users today").unwrap();
    writeln!(&mut output, "# TYPE readur_users_active_today gauge").unwrap();
    writeln!(&mut output, "readur_users_active_today {} {}", user_metrics.active_users_today, timestamp).unwrap();
    
    writeln!(&mut output, "# HELP readur_users_registered_today New user registrations today").unwrap();
    writeln!(&mut output, "# TYPE readur_users_registered_today gauge").unwrap();
    writeln!(&mut output, "readur_users_registered_today {} {}", user_metrics.new_registrations_today, timestamp).unwrap();
    
    // Return the metrics with the correct content type
    Ok((
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        output,
    ).into_response())
}

// Reuse the same metric collection structs from the JSON endpoint
struct DocumentMetrics {
    total_documents: i64,
    documents_uploaded_today: i64,
    total_storage_bytes: i64,
    documents_with_ocr: i64,
    documents_without_ocr: i64,
}

struct OcrMetrics {
    pending_jobs: i64,
    processing_jobs: i64,
    failed_jobs: i64,
    completed_today: i64,
    avg_processing_time_minutes: Option<f64>,
}

struct UserMetrics {
    total_users: i64,
    active_users_today: i64,
    new_registrations_today: i64,
}

async fn collect_document_metrics(state: &Arc<AppState>) -> Result<DocumentMetrics, StatusCode> {
    // Get total document count
    let total_docs = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM documents")
        .fetch_one(&state.db.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get total document count: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // Get documents uploaded today
    let docs_today = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM documents WHERE DATE(created_at) = CURRENT_DATE"
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get today's document count: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    // Get total storage size
    let total_size = sqlx::query_scalar::<_, Option<i64>>("SELECT SUM(file_size) FROM documents")
        .fetch_one(&state.db.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get total storage size: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .unwrap_or(0);
    
    // Get documents with and without OCR
    let docs_with_ocr = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM documents WHERE has_ocr_text = true"
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get OCR document count: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let docs_without_ocr = total_docs - docs_with_ocr;
    
    Ok(DocumentMetrics {
        total_documents: total_docs,
        documents_uploaded_today: docs_today,
        total_storage_bytes: total_size,
        documents_with_ocr: docs_with_ocr,
        documents_without_ocr: docs_without_ocr,
    })
}

async fn collect_ocr_metrics(state: &Arc<AppState>) -> Result<OcrMetrics, StatusCode> {
    use crate::ocr_queue::OcrQueueService;
    
    let queue_service = OcrQueueService::new(
        state.db.clone(),
        state.db.pool.clone(),
        state.config.concurrent_ocr_jobs
    );
    
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