use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use std::sync::Arc;
use std::fmt::Write;
use std::time::Instant;

use crate::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_prometheus_metrics))
}

/// Returns metrics in Prometheus text format (text/plain; version=0.0.4)
#[utoipa::path(
    get,
    path = "/metrics",
    tag = "metrics",
    responses(
        (status = 200, description = "Prometheus metrics in text format", content_type = "text/plain; version=0.0.4"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_prometheus_metrics(
    State(state): State<Arc<AppState>>,
) -> Result<Response, StatusCode> {
    let mut output = String::new();
    
    // Get current timestamp
    let timestamp = chrono::Utc::now().timestamp_millis();
    
    // Collect all metrics
    let (document_metrics, ocr_metrics, user_metrics, database_metrics, system_metrics, storage_metrics, security_metrics) = tokio::try_join!(
        collect_document_metrics(&state),
        collect_ocr_metrics(&state),
        collect_user_metrics(&state),
        collect_database_metrics(&state),
        collect_system_metrics(&state),
        collect_storage_metrics(&state),
        collect_security_metrics(&state)
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
    
    // Database metrics
    writeln!(&mut output, "# HELP readur_db_connections_active Active database connections").unwrap();
    writeln!(&mut output, "# TYPE readur_db_connections_active gauge").unwrap();
    writeln!(&mut output, "readur_db_connections_active {} {}", database_metrics.active_connections, timestamp).unwrap();
    
    writeln!(&mut output, "# HELP readur_db_connections_idle Idle database connections").unwrap();
    writeln!(&mut output, "# TYPE readur_db_connections_idle gauge").unwrap();
    writeln!(&mut output, "readur_db_connections_idle {} {}", database_metrics.idle_connections, timestamp).unwrap();
    
    writeln!(&mut output, "# HELP readur_db_connections_total Total database connections").unwrap();
    writeln!(&mut output, "# TYPE readur_db_connections_total gauge").unwrap();
    writeln!(&mut output, "readur_db_connections_total {} {}", database_metrics.total_connections, timestamp).unwrap();
    
    writeln!(&mut output, "# HELP readur_db_utilization_percent Database connection pool utilization percentage").unwrap();
    writeln!(&mut output, "# TYPE readur_db_utilization_percent gauge").unwrap();
    writeln!(&mut output, "readur_db_utilization_percent {} {}", database_metrics.utilization_percent, timestamp).unwrap();
    
    writeln!(&mut output, "# HELP readur_db_response_time_ms Database response time in milliseconds").unwrap();
    writeln!(&mut output, "# TYPE readur_db_response_time_ms gauge").unwrap();
    writeln!(&mut output, "readur_db_response_time_ms {} {}", database_metrics.response_time_ms, timestamp).unwrap();
    
    // Enhanced OCR metrics
    if let Some(confidence) = ocr_metrics.avg_confidence {
        writeln!(&mut output, "# HELP readur_ocr_confidence_score Average OCR confidence score").unwrap();
        writeln!(&mut output, "# TYPE readur_ocr_confidence_score gauge").unwrap();
        writeln!(&mut output, "readur_ocr_confidence_score {} {}", confidence, timestamp).unwrap();
    }
    
    if let Some(oldest_pending) = ocr_metrics.oldest_pending_minutes {
        writeln!(&mut output, "# HELP readur_ocr_queue_oldest_pending_minutes Age of oldest pending OCR job in minutes").unwrap();
        writeln!(&mut output, "# TYPE readur_ocr_queue_oldest_pending_minutes gauge").unwrap();
        writeln!(&mut output, "readur_ocr_queue_oldest_pending_minutes {} {}", oldest_pending, timestamp).unwrap();
    }
    
    writeln!(&mut output, "# HELP readur_ocr_stuck_jobs OCR jobs stuck in processing state").unwrap();
    writeln!(&mut output, "# TYPE readur_ocr_stuck_jobs gauge").unwrap();
    writeln!(&mut output, "readur_ocr_stuck_jobs {} {}", ocr_metrics.stuck_jobs, timestamp).unwrap();
    
    writeln!(&mut output, "# HELP readur_ocr_queue_depth Total OCR queue depth (pending + processing)").unwrap();
    writeln!(&mut output, "# TYPE readur_ocr_queue_depth gauge").unwrap();
    writeln!(&mut output, "readur_ocr_queue_depth {} {}", ocr_metrics.queue_depth, timestamp).unwrap();
    
    // Storage metrics
    writeln!(&mut output, "# HELP readur_storage_usage_percent Storage utilization percentage").unwrap();
    writeln!(&mut output, "# TYPE readur_storage_usage_percent gauge").unwrap();
    writeln!(&mut output, "readur_storage_usage_percent {} {}", storage_metrics.usage_percent, timestamp).unwrap();
    
    writeln!(&mut output, "# HELP readur_avg_document_size_bytes Average document size in bytes").unwrap();
    writeln!(&mut output, "# TYPE readur_avg_document_size_bytes gauge").unwrap();
    writeln!(&mut output, "readur_avg_document_size_bytes {} {}", storage_metrics.avg_document_size_bytes, timestamp).unwrap();
    
    // Document type metrics
    for (doc_type, count) in &storage_metrics.documents_by_type {
        writeln!(&mut output, "# HELP readur_documents_by_type Documents count by file type").unwrap();
        writeln!(&mut output, "# TYPE readur_documents_by_type gauge").unwrap();
        writeln!(&mut output, "readur_documents_by_type{{type=\"{}\"}} {} {}", doc_type, count, timestamp).unwrap();
    }
    
    // System metrics
    writeln!(&mut output, "# HELP readur_uptime_seconds Application uptime in seconds").unwrap();
    writeln!(&mut output, "# TYPE readur_uptime_seconds counter").unwrap();
    writeln!(&mut output, "readur_uptime_seconds {} {}", system_metrics.uptime_seconds, timestamp).unwrap();
    
    writeln!(&mut output, "# HELP readur_memory_usage_bytes Memory usage in bytes").unwrap();
    writeln!(&mut output, "# TYPE readur_memory_usage_bytes gauge").unwrap();
    writeln!(&mut output, "readur_memory_usage_bytes {} {}", system_metrics.memory_usage_bytes, timestamp).unwrap();
    
    writeln!(&mut output, "# HELP readur_data_consistency_score Data integrity score (0-100)").unwrap();
    writeln!(&mut output, "# TYPE readur_data_consistency_score gauge").unwrap();
    writeln!(&mut output, "readur_data_consistency_score {} {}", system_metrics.data_consistency_score, timestamp).unwrap();
    
    // Security metrics
    writeln!(&mut output, "# HELP readur_failed_logins_today Failed login attempts today").unwrap();
    writeln!(&mut output, "# TYPE readur_failed_logins_today counter").unwrap();
    writeln!(&mut output, "readur_failed_logins_today {} {}", security_metrics.failed_logins_today, timestamp).unwrap();
    
    writeln!(&mut output, "# HELP readur_document_access_today Document access count today").unwrap();
    writeln!(&mut output, "# TYPE readur_document_access_today counter").unwrap();
    writeln!(&mut output, "readur_document_access_today {} {}", security_metrics.document_access_today, timestamp).unwrap();
    
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
    avg_confidence: Option<f64>,
    oldest_pending_minutes: Option<f64>,
    stuck_jobs: i64,
    queue_depth: i64,
}

struct UserMetrics {
    total_users: i64,
    active_users_today: i64,
    new_registrations_today: i64,
}

struct DatabaseMetrics {
    active_connections: u32,
    idle_connections: u32,
    total_connections: u32,
    utilization_percent: u8,
    response_time_ms: u64,
}

struct SystemMetrics {
    uptime_seconds: u64,
    memory_usage_bytes: u64,
    data_consistency_score: f64,
}

struct StorageMetrics {
    usage_percent: f64,
    avg_document_size_bytes: f64,
    documents_by_type: std::collections::HashMap<String, i64>,
}

struct SecurityMetrics {
    failed_logins_today: i64,
    document_access_today: i64,
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
    let total_size = sqlx::query_scalar::<_, Option<f64>>("SELECT CAST(COALESCE(SUM(file_size), 0) AS DOUBLE PRECISION) FROM documents")
        .fetch_one(&state.db.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get total storage size: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .unwrap_or(0.0) as i64;
    
    // Get documents with and without OCR
    let docs_with_ocr = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM documents WHERE ocr_text IS NOT NULL AND ocr_text != ''"
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
    use crate::ocr::queue::OcrQueueService;
    
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
    
    // Get additional OCR metrics
    let stuck_jobs = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM documents WHERE ocr_status = 'processing' AND updated_at < NOW() - INTERVAL '30 minutes'"
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get stuck OCR jobs: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let avg_confidence = sqlx::query_scalar::<_, Option<f64>>(
        "SELECT AVG(ocr_confidence) FROM documents WHERE ocr_status = 'completed' AND ocr_completed_at > NOW() - INTERVAL '1 hour'"
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get average OCR confidence: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let oldest_pending = sqlx::query_scalar::<_, Option<f64>>(
        "SELECT CAST(EXTRACT(EPOCH FROM (NOW() - MIN(created_at)))/60 AS DOUBLE PRECISION) FROM documents WHERE ocr_status = 'pending'"
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get oldest pending OCR job: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(OcrMetrics {
        pending_jobs: stats.pending_count,
        processing_jobs: stats.processing_count,
        failed_jobs: stats.failed_count,
        completed_today: stats.completed_today,
        avg_processing_time_minutes: stats.avg_wait_time_minutes,
        avg_confidence,
        oldest_pending_minutes: oldest_pending,
        stuck_jobs,
        queue_depth: stats.pending_count + stats.processing_count,
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

async fn collect_database_metrics(state: &Arc<AppState>) -> Result<DatabaseMetrics, StatusCode> {
    let start = Instant::now();
    
    // Test database responsiveness
    sqlx::query("SELECT 1")
        .fetch_one(&state.db.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database health check failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    let response_time = start.elapsed().as_millis() as u64;
    
    let total_connections = state.db.pool.size();
    let idle_connections = state.db.pool.num_idle() as u32;
    let active_connections = total_connections - idle_connections;
    let utilization = if total_connections > 0 {
        (active_connections as f64 / total_connections as f64 * 100.0) as u8
    } else {
        0
    };
    
    Ok(DatabaseMetrics {
        active_connections,
        idle_connections,
        total_connections,
        utilization_percent: utilization,
        response_time_ms: response_time,
    })
}

async fn collect_system_metrics(state: &Arc<AppState>) -> Result<SystemMetrics, StatusCode> {
    // Get application uptime (simplified - would need proper tracking in production)
    let uptime_seconds = 3600; // Placeholder
    
    // Get memory usage (simplified)
    let memory_usage_bytes = 0; // Would need proper memory tracking
    
    // Get data consistency score using similar logic from db_monitoring
    #[derive(sqlx::FromRow)]
    struct ConsistencyCheck {
        orphaned_queue: Option<i64>,
        inconsistent_states: Option<i64>,
    }
    
    let consistency_check = sqlx::query_as::<_, ConsistencyCheck>(
        r#"
        SELECT 
            (SELECT COUNT(*) FROM ocr_queue q 
             LEFT JOIN documents d ON q.document_id = d.id 
             WHERE d.id IS NULL) as orphaned_queue,
            (SELECT COUNT(*) FROM documents d
             JOIN ocr_queue q ON d.id = q.document_id
             WHERE d.ocr_status = 'completed' AND q.status != 'completed') as inconsistent_states
        "#
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get consistency metrics: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let orphaned = consistency_check.orphaned_queue.unwrap_or(0) as i32;
    let inconsistent = consistency_check.inconsistent_states.unwrap_or(0) as i32;
    let total_issues = orphaned + inconsistent;
    let consistency_score = if total_issues == 0 { 100.0 } else { 100.0 - (total_issues as f64 * 10.0).min(100.0) };
    
    Ok(SystemMetrics {
        uptime_seconds,
        memory_usage_bytes,
        data_consistency_score: consistency_score,
    })
}

async fn collect_storage_metrics(state: &Arc<AppState>) -> Result<StorageMetrics, StatusCode> {
    // Get document type distribution
    #[derive(sqlx::FromRow)]
    struct DocTypeCount {
        doc_type: Option<String>,
        count: Option<i64>,
    }
    
    let doc_types = sqlx::query_as::<_, DocTypeCount>(
        r#"
        SELECT 
            CASE 
                WHEN filename ILIKE '%.pdf' THEN 'pdf'
                WHEN filename ILIKE '%.jpg' OR filename ILIKE '%.jpeg' THEN 'jpeg'
                WHEN filename ILIKE '%.png' THEN 'png'
                WHEN filename ILIKE '%.gif' THEN 'gif'
                WHEN filename ILIKE '%.tiff' OR filename ILIKE '%.tif' THEN 'tiff'
                ELSE 'other'
            END as doc_type,
            COUNT(*) as count
        FROM documents 
        GROUP BY doc_type
        "#
    )
    .fetch_all(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get document types: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let mut documents_by_type = std::collections::HashMap::new();
    for row in doc_types {
        documents_by_type.insert(
            row.doc_type.unwrap_or("unknown".to_string()), 
            row.count.unwrap_or(0)
        );
    }
    
    // Get storage metrics
    #[derive(sqlx::FromRow)]
    struct StorageStats {
        total_docs: Option<i64>,
        total_size: Option<i64>,
        avg_size: Option<f64>,
    }
    
    let storage_stats = sqlx::query_as::<_, StorageStats>(
        "SELECT COUNT(*) as total_docs, CAST(COALESCE(SUM(file_size), 0) AS BIGINT) as total_size, CAST(COALESCE(AVG(file_size), 0) AS DOUBLE PRECISION) as avg_size FROM documents"
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get storage stats: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let total_size = storage_stats.total_size.unwrap_or(0) as f64;
    let avg_size = storage_stats.avg_size.unwrap_or(0.0);
    
    // Calculate usage percentage (simplified - would need actual disk space info)
    let usage_percent = 0.0; // Placeholder
    
    Ok(StorageMetrics {
        usage_percent,
        avg_document_size_bytes: avg_size,
        documents_by_type,
    })
}

async fn collect_security_metrics(state: &Arc<AppState>) -> Result<SecurityMetrics, StatusCode> {
    // Note: These metrics would need proper tracking in production
    // For now, we'll provide basic placeholders that could be implemented
    
    // Count document access today (simplified - would need proper audit logging)
    let document_access_today = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM documents WHERE DATE(created_at) = CURRENT_DATE"
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to get document access count: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    // Placeholder for failed logins (would need proper auth event tracking)
    let failed_logins_today = 0;
    
    Ok(SecurityMetrics {
        failed_logins_today,
        document_access_today,
    })
}