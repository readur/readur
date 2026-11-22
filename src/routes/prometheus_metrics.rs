use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use std::fmt::{Display, Write};
use std::sync::Arc;
use std::time::Instant;

use crate::{
    auth::{authenticate_token, extract_token_from_headers},
    models::UserRole,
    utils::security::constant_time_eq,
    AppState,
};

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/", get(get_prometheus_metrics))
}

/// Returns metrics in Prometheus text format (text/plain; version=0.0.4)
#[utoipa::path(
    get,
    path = "/metrics",
    tag = "metrics",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Prometheus metrics in text format", content_type = "text/plain; version=0.0.4"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Admin access required"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_prometheus_metrics(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    if let Err(rejection) = authorize_scrape(&headers, &state).await {
        return *rejection;
    }
    match render_metrics(&state).await {
        Ok(body) => ([(header::CONTENT_TYPE, "text/plain; version=0.0.4")], body).into_response(),
        Err(status) => status.into_response(),
    }
}

/// Scrapers authenticate with `Authorization: Bearer <METRICS_TOKEN>` when a
/// token is configured; otherwise (or with any other credential) the caller
/// must be an admin.
async fn authorize_scrape(headers: &HeaderMap, state: &Arc<AppState>) -> Result<(), Box<Response>> {
    let token = extract_token_from_headers(headers).ok_or_else(|| {
        Box::new((StatusCode::UNAUTHORIZED, "Missing authorization header").into_response())
    })?;

    if let Some(expected) = &state.config.security.metrics_token {
        if constant_time_eq(token.as_bytes(), expected.as_bytes()) {
            return Ok(());
        }
    }

    let user = authenticate_token(&token, state).await.map_err(Box::new)?;
    if user.role != UserRole::Admin {
        return Err(Box::new(
            (StatusCode::FORBIDDEN, "Admin access required").into_response(),
        ));
    }
    Ok(())
}

/// The exposition text being written: one `# HELP`, `# TYPE` and sample per
/// metric, every sample stamped with the scrape time.
struct Exposition {
    out: String,
    timestamp: i64,
}

impl Exposition {
    fn new() -> Self {
        Self {
            out: String::new(),
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }

    fn header(&mut self, name: &str, kind: &str, help: &str) {
        let _ = writeln!(self.out, "# HELP {name} {help}");
        let _ = writeln!(self.out, "# TYPE {name} {kind}");
    }

    fn gauge(&mut self, name: &str, help: &str, value: impl Display) {
        self.header(name, "gauge", help);
        self.sample(name, "", value);
    }

    fn counter(&mut self, name: &str, help: &str, value: impl Display) {
        self.header(name, "counter", help);
        self.sample(name, "", value);
    }

    /// A gauge that is left out of the scrape while it has no value, so no
    /// stand-in number is ever reported.
    fn gauge_if(&mut self, name: &str, help: &str, value: Option<impl Display>) {
        if let Some(value) = value {
            self.gauge(name, help, value);
        }
    }

    fn sample(&mut self, name: &str, labels: &str, value: impl Display) {
        let _ = writeln!(self.out, "{name}{labels} {value} {}", self.timestamp);
    }
}

/// Every metric, each computed from the database, the queue or the process
/// at scrape time.
pub(crate) async fn render_metrics(state: &Arc<AppState>) -> Result<String, StatusCode> {
    let (documents, ocr, users, database, system, storage, webdav) = tokio::try_join!(
        collect_document_metrics(state),
        collect_ocr_metrics(state),
        collect_user_metrics(state),
        collect_database_metrics(state),
        collect_system_metrics(state),
        collect_storage_metrics(state),
        collect_webdav_metrics(state),
    )
    .map_err(|e| {
        tracing::error!("Prometheus: Failed to collect metrics: {:?}", e);
        e
    })?;

    let mut m = Exposition::new();
    write_document_metrics(&mut m, &documents, &storage);
    write_ocr_metrics(&mut m, &ocr);
    write_user_metrics(&mut m, &users);
    write_database_metrics(&mut m, &database);
    write_system_metrics(&mut m, &system);
    write_webdav_metrics(&mut m, &webdav);
    Ok(m.out)
}

fn write_document_metrics(m: &mut Exposition, d: &DocumentMetrics, s: &StorageMetrics) {
    m.gauge(
        "readur_documents_total",
        "Total number of documents",
        d.total_documents,
    );
    m.gauge(
        "readur_documents_uploaded_today",
        "Documents uploaded today",
        d.documents_uploaded_today,
    );
    m.gauge(
        "readur_storage_bytes",
        "Total storage used in bytes",
        d.total_storage_bytes,
    );
    m.gauge(
        "readur_documents_with_ocr",
        "Documents with OCR text",
        d.documents_with_ocr,
    );
    m.gauge(
        "readur_documents_without_ocr",
        "Documents without OCR text",
        d.documents_without_ocr,
    );
    m.gauge_if(
        "readur_storage_usage_percent",
        "Used space on the filesystem holding the upload directory (local storage only)",
        s.usage_percent,
    );
    m.gauge(
        "readur_avg_document_size_bytes",
        "Average document size in bytes",
        s.avg_document_size_bytes,
    );
    if !s.documents_by_type.is_empty() {
        m.header(
            "readur_documents_by_type",
            "gauge",
            "Documents count by file type",
        );
        for (doc_type, count) in &s.documents_by_type {
            m.sample(
                "readur_documents_by_type",
                &format!("{{type=\"{doc_type}\"}}"),
                count,
            );
        }
    }
}

fn write_ocr_metrics(m: &mut Exposition, o: &OcrMetrics) {
    m.gauge(
        "readur_ocr_queue_pending",
        "OCR jobs pending",
        o.pending_jobs,
    );
    m.gauge(
        "readur_ocr_queue_processing",
        "OCR jobs currently processing",
        o.processing_jobs,
    );
    m.gauge("readur_ocr_queue_failed", "OCR jobs failed", o.failed_jobs);
    m.gauge(
        "readur_ocr_completed_today",
        "OCR jobs completed today",
        o.completed_today,
    );
    m.gauge_if(
        "readur_ocr_avg_processing_minutes",
        "Average time OCR jobs waited in the queue, in minutes",
        o.avg_processing_time_minutes,
    );
    m.gauge_if(
        "readur_ocr_confidence_score",
        "Average OCR confidence score",
        o.avg_confidence,
    );
    m.gauge_if(
        "readur_ocr_queue_oldest_pending_minutes",
        "Age of oldest pending OCR job in minutes",
        o.oldest_pending_minutes,
    );
    m.gauge(
        "readur_ocr_stuck_jobs",
        "OCR jobs stuck in processing state",
        o.stuck_jobs,
    );
    m.gauge(
        "readur_ocr_queue_depth",
        "Total OCR queue depth (pending + processing)",
        o.queue_depth,
    );
}

fn write_user_metrics(m: &mut Exposition, u: &UserMetrics) {
    m.gauge("readur_users_total", "Total number of users", u.total_users);
    m.gauge(
        "readur_users_active_today",
        "Active users today",
        u.active_users_today,
    );
    m.gauge(
        "readur_users_registered_today",
        "New user registrations today",
        u.new_registrations_today,
    );
}

fn write_database_metrics(m: &mut Exposition, d: &DatabaseMetrics) {
    m.gauge(
        "readur_db_connections_active",
        "Active database connections",
        d.active_connections,
    );
    m.gauge(
        "readur_db_connections_idle",
        "Idle database connections",
        d.idle_connections,
    );
    m.gauge(
        "readur_db_connections_total",
        "Total database connections",
        d.total_connections,
    );
    m.gauge(
        "readur_db_utilization_percent",
        "Database connection pool utilization percentage",
        d.utilization_percent,
    );
    m.gauge(
        "readur_db_response_time_ms",
        "Database response time in milliseconds",
        d.response_time_ms,
    );
}

fn write_system_metrics(m: &mut Exposition, s: &SystemMetrics) {
    m.counter(
        "readur_uptime_seconds",
        "Seconds since the server process started",
        format!("{:.3}", s.uptime_seconds),
    );
    m.gauge_if(
        "readur_memory_usage_bytes",
        "Resident memory of the server process in bytes",
        s.memory_usage_bytes,
    );
    m.gauge(
        "readur_data_consistency_score",
        "Data integrity score (0-100)",
        s.data_consistency_score,
    );
}

fn write_webdav_metrics(m: &mut Exposition, w: &WebDavMetrics) {
    m.gauge(
        "readur_webdav_sources",
        "WebDAV sources configured",
        w.sources,
    );
    m.gauge(
        "readur_webdav_sources_enabled",
        "WebDAV sources that are enabled",
        w.enabled,
    );
    m.gauge(
        "readur_webdav_sources_syncing",
        "WebDAV sources syncing now",
        w.syncing,
    );
    m.gauge(
        "readur_webdav_sources_error",
        "WebDAV sources whose last sync failed",
        w.in_error,
    );
    m.gauge(
        "readur_webdav_files_synced",
        "Files synced from all WebDAV sources (as recorded on each source)",
        w.files_synced,
    );
    m.gauge(
        "readur_webdav_bytes_synced",
        "Bytes synced from all WebDAV sources (as recorded on each source)",
        w.bytes_synced,
    );
    m.gauge(
        "readur_webdav_files_pending",
        "Files WebDAV sources have found but not yet synced",
        w.files_pending,
    );
    m.gauge_if(
        "readur_webdav_oldest_sync_age_seconds",
        "Seconds since the least recently synced enabled WebDAV source last synced",
        w.oldest_sync_age_seconds,
    );
}

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
    uptime_seconds: f64,
    memory_usage_bytes: Option<u64>,
    data_consistency_score: f64,
}

struct StorageMetrics {
    usage_percent: Option<f64>,
    avg_document_size_bytes: f64,
    documents_by_type: std::collections::BTreeMap<String, i64>,
}

/// WebDAV sources as their rows in `sources` record them.
#[derive(sqlx::FromRow)]
struct WebDavMetrics {
    sources: i64,
    enabled: i64,
    syncing: i64,
    in_error: i64,
    files_synced: i64,
    bytes_synced: i64,
    files_pending: i64,
    oldest_sync_age_seconds: Option<f64>,
}

/// Maps a query failure to a 500, logging what failed.
fn db_error(what: &'static str) -> impl FnOnce(sqlx::Error) -> StatusCode {
    move |e| {
        tracing::error!("Prometheus: failed to get {}: {}", what, e);
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

async fn count(state: &AppState, sql: &'static str, what: &'static str) -> Result<i64, StatusCode> {
    sqlx::query_scalar::<_, i64>(sql)
        .fetch_one(&state.db.pool)
        .await
        .map_err(db_error(what))
}

async fn collect_document_metrics(state: &Arc<AppState>) -> Result<DocumentMetrics, StatusCode> {
    let total_documents = count(
        state,
        "SELECT COUNT(*) FROM documents",
        "total document count",
    )
    .await?;
    let documents_uploaded_today = count(
        state,
        "SELECT COUNT(*) FROM documents WHERE DATE(created_at) = CURRENT_DATE",
        "today's document count",
    )
    .await?;
    let total_storage_bytes = sqlx::query_scalar::<_, Option<f64>>(
        "SELECT CAST(COALESCE(SUM(file_size), 0) AS DOUBLE PRECISION) FROM documents",
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(db_error("total storage size"))?
    .unwrap_or(0.0) as i64;
    let documents_with_ocr = count(
        state,
        "SELECT COUNT(*) FROM documents WHERE ocr_text IS NOT NULL AND ocr_text != ''",
        "OCR document count",
    )
    .await?;

    Ok(DocumentMetrics {
        total_documents,
        documents_uploaded_today,
        total_storage_bytes,
        documents_with_ocr,
        documents_without_ocr: total_documents - documents_with_ocr,
    })
}

async fn collect_ocr_metrics(state: &Arc<AppState>) -> Result<OcrMetrics, StatusCode> {
    let stats = state.queue_service.get_stats().await.map_err(|e| {
        tracing::error!("Prometheus: Failed to get OCR stats: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let stuck_jobs = count(
        state,
        "SELECT COUNT(*) FROM documents WHERE ocr_status = 'processing' AND updated_at < NOW() - INTERVAL '30 minutes'",
        "stuck OCR jobs",
    )
    .await?;
    let avg_confidence = sqlx::query_scalar::<_, Option<f64>>(
        "SELECT AVG(ocr_confidence) FROM documents WHERE ocr_status = 'completed' AND ocr_completed_at > NOW() - INTERVAL '1 hour'",
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(db_error("average OCR confidence"))?;
    let oldest_pending = sqlx::query_scalar::<_, Option<f64>>(
        "SELECT CAST(EXTRACT(EPOCH FROM (NOW() - MIN(created_at)))/60 AS DOUBLE PRECISION) FROM documents WHERE ocr_status = 'pending'",
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(db_error("oldest pending OCR job"))?;

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
    Ok(UserMetrics {
        total_users: count(state, "SELECT COUNT(*) FROM users", "total user count").await?,
        // Users who added a document today
        active_users_today: count(
            state,
            "SELECT COUNT(DISTINCT user_id) FROM documents WHERE DATE(created_at) = CURRENT_DATE",
            "active user count",
        )
        .await?,
        new_registrations_today: count(
            state,
            "SELECT COUNT(*) FROM users WHERE DATE(created_at) = CURRENT_DATE",
            "new user count",
        )
        .await?,
    })
}

async fn collect_database_metrics(state: &Arc<AppState>) -> Result<DatabaseMetrics, StatusCode> {
    let start = Instant::now();
    sqlx::query("SELECT 1")
        .fetch_one(&state.db.pool)
        .await
        .map_err(db_error("database health check"))?;
    let response_time_ms = start.elapsed().as_millis() as u64;

    let total_connections = state.db.pool.size();
    let idle_connections = state.db.pool.num_idle() as u32;
    let active_connections = total_connections.saturating_sub(idle_connections);
    let utilization_percent = if total_connections > 0 {
        (active_connections as f64 / total_connections as f64 * 100.0) as u8
    } else {
        0
    };
    Ok(DatabaseMetrics {
        active_connections,
        idle_connections,
        total_connections,
        utilization_percent,
        response_time_ms,
    })
}

async fn collect_system_metrics(state: &Arc<AppState>) -> Result<SystemMetrics, StatusCode> {
    #[derive(sqlx::FromRow)]
    struct ConsistencyCheck {
        orphaned_queue: Option<i64>,
        inconsistent_states: Option<i64>,
    }
    let check = sqlx::query_as::<_, ConsistencyCheck>(
        r#"
        SELECT
            (SELECT COUNT(*) FROM ocr_queue q
             LEFT JOIN documents d ON q.document_id = d.id
             WHERE d.id IS NULL) as orphaned_queue,
            (SELECT COUNT(*) FROM documents d
             JOIN ocr_queue q ON d.id = q.document_id
             WHERE d.ocr_status = 'completed' AND q.status != 'completed') as inconsistent_states
        "#,
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(db_error("consistency metrics"))?;

    let total_issues = check.orphaned_queue.unwrap_or(0) + check.inconsistent_states.unwrap_or(0);
    let memory_usage_bytes =
        tokio::task::spawn_blocking(crate::monitoring::process::resident_memory_bytes)
            .await
            .ok()
            .flatten();

    Ok(SystemMetrics {
        uptime_seconds: crate::monitoring::process::uptime().as_secs_f64(),
        memory_usage_bytes,
        data_consistency_score: 100.0 - (total_issues as f64 * 10.0).min(100.0),
    })
}

async fn collect_storage_metrics(state: &Arc<AppState>) -> Result<StorageMetrics, StatusCode> {
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
        "#,
    )
    .fetch_all(&state.db.pool)
    .await
    .map_err(db_error("document types"))?;
    let documents_by_type = doc_types
        .into_iter()
        .map(|row| {
            (
                row.doc_type.unwrap_or_else(|| "unknown".to_string()),
                row.count.unwrap_or(0),
            )
        })
        .collect();

    let avg_document_size_bytes = sqlx::query_scalar::<_, f64>(
        "SELECT CAST(COALESCE(AVG(file_size), 0) AS DOUBLE PRECISION) FROM documents",
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(db_error("average document size"))?;

    // Only local storage fills a filesystem Readur can measure
    let usage_percent = if state.config.s3_enabled {
        None
    } else {
        let upload_path = std::path::PathBuf::from(&state.config.upload_path);
        tokio::task::spawn_blocking(move || {
            crate::monitoring::process::filesystem_usage_percent(&upload_path)
        })
        .await
        .ok()
        .flatten()
    };

    Ok(StorageMetrics {
        usage_percent,
        avg_document_size_bytes,
        documents_by_type,
    })
}

async fn collect_webdav_metrics(state: &Arc<AppState>) -> Result<WebDavMetrics, StatusCode> {
    sqlx::query_as::<_, WebDavMetrics>(
        r#"
        SELECT
            COUNT(*) AS sources,
            COUNT(*) FILTER (WHERE enabled) AS enabled,
            COUNT(*) FILTER (WHERE status = 'syncing') AS syncing,
            COUNT(*) FILTER (WHERE status = 'error') AS in_error,
            CAST(COALESCE(SUM(total_files_synced), 0) AS BIGINT) AS files_synced,
            CAST(COALESCE(SUM(total_size_bytes), 0) AS BIGINT) AS bytes_synced,
            CAST(COALESCE(SUM(total_files_pending), 0) AS BIGINT) AS files_pending,
            CAST(EXTRACT(EPOCH FROM (NOW() - MIN(last_sync_at) FILTER (WHERE enabled)))
                 AS DOUBLE PRECISION) AS oldest_sync_age_seconds
        FROM sources
        WHERE source_type = 'webdav'
        "#,
    )
    .fetch_one(&state.db.pool)
    .await
    .map_err(db_error("WebDAV source metrics"))
}
