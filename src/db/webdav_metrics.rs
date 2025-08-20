use anyhow::Result;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::db::Database;
use crate::models::webdav_metrics::*;

impl Database {
    /// Create a new WebDAV sync session
    pub async fn create_webdav_sync_session(&self, session: &CreateWebDAVSyncSession) -> Result<Uuid> {
        self.with_retry(|| async {
            let row = sqlx::query!(
                r#"
                INSERT INTO webdav_sync_sessions (
                    user_id, source_id, sync_type, root_path, max_depth
                ) VALUES ($1, $2, $3, $4, $5)
                RETURNING id
                "#,
                session.user_id,
                session.source_id,
                session.sync_type,
                session.root_path,
                session.max_depth
            )
            .fetch_one(&self.pool)
            .await?;
            
            Ok(row.id)
        }).await
    }

    /// Update a WebDAV sync session with new metrics
    pub async fn update_webdav_sync_session(
        &self, 
        session_id: Uuid, 
        update: &UpdateWebDAVSyncSession
    ) -> Result<bool> {
        self.with_retry(|| async {
            let rows_affected = sqlx::query!(
                r#"
                UPDATE webdav_sync_sessions SET
                    directories_discovered = COALESCE($2, directories_discovered),
                    directories_processed = COALESCE($3, directories_processed),
                    files_discovered = COALESCE($4, files_discovered),
                    files_processed = COALESCE($5, files_processed),
                    total_bytes_discovered = COALESCE($6, total_bytes_discovered),
                    total_bytes_processed = COALESCE($7, total_bytes_processed),
                    directories_skipped = COALESCE($8, directories_skipped),
                    files_skipped = COALESCE($9, files_skipped),
                    skip_reasons = COALESCE($10, skip_reasons),
                    status = COALESCE($11, status),
                    final_error_message = COALESCE($12, final_error_message),
                    updated_at = NOW()
                WHERE id = $1
                "#,
                session_id,
                update.directories_discovered,
                update.directories_processed,
                update.files_discovered,
                update.files_processed,
                update.total_bytes_discovered,
                update.total_bytes_processed,
                update.directories_skipped,
                update.files_skipped,
                update.skip_reasons,
                update.status.as_ref().map(|s| s.to_string()),
                update.final_error_message
            )
            .execute(&self.pool)
            .await?;
            
            Ok(rows_affected.rows_affected() > 0)
        }).await
    }

    /// Finalize a WebDAV sync session (calculate final metrics)
    pub async fn finalize_webdav_sync_session(&self, session_id: Uuid) -> Result<()> {
        self.with_retry(|| async {
            sqlx::query!(
                "SELECT finalize_webdav_session_metrics($1)",
                session_id
            )
            .execute(&self.pool)
            .await?;
            
            Ok(())
        }).await
    }

    /// Get a WebDAV sync session by ID
    pub async fn get_webdav_sync_session(
        &self, 
        session_id: Uuid, 
        user_id: Uuid
    ) -> Result<Option<WebDAVSyncSession>> {
        self.with_retry(|| async {
            let session = sqlx::query_as!(
                WebDAVSyncSession,
                "SELECT * FROM webdav_sync_sessions WHERE id = $1 AND user_id = $2",
                session_id,
                user_id
            )
            .fetch_optional(&self.pool)
            .await?;
            
            Ok(session)
        }).await
    }

    /// List WebDAV sync sessions with optional filtering
    pub async fn list_webdav_sync_sessions(
        &self,
        query: &WebDAVMetricsQuery
    ) -> Result<Vec<WebDAVSyncSession>> {
        self.with_retry(|| async {
            let start_time = query.start_time.unwrap_or_else(|| Utc::now() - chrono::Duration::days(7));
            let end_time = query.end_time.unwrap_or_else(|| Utc::now());
            let limit = query.limit.unwrap_or(100).min(1000); // Cap at 1000
            let offset = query.offset.unwrap_or(0);

            let sessions = sqlx::query_as!(
                WebDAVSyncSession,
                r#"
                SELECT * FROM webdav_sync_sessions 
                WHERE started_at BETWEEN $1 AND $2
                AND ($3::UUID IS NULL OR user_id = $3)
                AND ($4::UUID IS NULL OR source_id = $4)
                ORDER BY started_at DESC
                LIMIT $5 OFFSET $6
                "#,
                start_time,
                end_time,
                query.user_id,
                query.source_id,
                limit as i64,
                offset as i64
            )
            .fetch_all(&self.pool)
            .await?;
            
            Ok(sessions)
        }).await
    }

    /// Create a new WebDAV directory metric
    pub async fn create_webdav_directory_metric(
        &self, 
        metric: &CreateWebDAVDirectoryMetric
    ) -> Result<Uuid> {
        self.with_retry(|| async {
            let row = sqlx::query!(
                r#"
                INSERT INTO webdav_directory_metrics (
                    session_id, user_id, source_id, directory_path, 
                    directory_depth, parent_directory_path
                ) VALUES ($1, $2, $3, $4, $5, $6)
                RETURNING id
                "#,
                metric.session_id,
                metric.user_id,
                metric.source_id,
                metric.directory_path,
                metric.directory_depth,
                metric.parent_directory_path
            )
            .fetch_one(&self.pool)
            .await?;
            
            Ok(row.id)
        }).await
    }

    /// Update a WebDAV directory metric
    pub async fn update_webdav_directory_metric(
        &self,
        metric_id: Uuid,
        update: &UpdateWebDAVDirectoryMetric
    ) -> Result<bool> {
        self.with_retry(|| async {
            let rows_affected = sqlx::query!(
                r#"
                UPDATE webdav_directory_metrics SET
                    completed_at = CASE 
                        WHEN completed_at IS NULL THEN NOW() 
                        ELSE completed_at 
                    END,
                    scan_duration_ms = CASE 
                        WHEN completed_at IS NULL THEN EXTRACT(EPOCH FROM (NOW() - started_at)) * 1000 
                        ELSE scan_duration_ms 
                    END,
                    files_found = COALESCE($2, files_found),
                    subdirectories_found = COALESCE($3, subdirectories_found),
                    total_size_bytes = COALESCE($4, total_size_bytes),
                    files_processed = COALESCE($5, files_processed),
                    files_skipped = COALESCE($6, files_skipped),
                    files_failed = COALESCE($7, files_failed),
                    http_requests_made = COALESCE($8, http_requests_made),
                    propfind_requests = COALESCE($9, propfind_requests),
                    get_requests = COALESCE($10, get_requests),
                    errors_encountered = COALESCE($11, errors_encountered),
                    error_types = COALESCE($12, error_types),
                    warnings_count = COALESCE($13, warnings_count),
                    etag_matches = COALESCE($14, etag_matches),
                    etag_mismatches = COALESCE($15, etag_mismatches),
                    cache_hits = COALESCE($16, cache_hits),
                    cache_misses = COALESCE($17, cache_misses),
                    status = COALESCE($18, status),
                    skip_reason = COALESCE($19, skip_reason),
                    error_message = COALESCE($20, error_message)
                WHERE id = $1
                "#,
                metric_id,
                update.files_found,
                update.subdirectories_found,
                update.total_size_bytes,
                update.files_processed,
                update.files_skipped,
                update.files_failed,
                update.http_requests_made,
                update.propfind_requests,
                update.get_requests,
                update.errors_encountered,
                update.error_types,
                update.warnings_count,
                update.etag_matches,
                update.etag_mismatches,
                update.cache_hits,
                update.cache_misses,
                update.status,
                update.skip_reason,
                update.error_message
            )
            .execute(&self.pool)
            .await?;
            
            Ok(rows_affected.rows_affected() > 0)
        }).await
    }

    /// Get directory metrics for a session
    pub async fn get_webdav_directory_metrics(
        &self,
        session_id: Uuid,
        user_id: Uuid
    ) -> Result<Vec<WebDAVDirectoryMetric>> {
        self.with_retry(|| async {
            let metrics = sqlx::query_as!(
                WebDAVDirectoryMetric,
                r#"
                SELECT * FROM webdav_directory_metrics 
                WHERE session_id = $1 AND user_id = $2
                ORDER BY started_at ASC
                "#,
                session_id,
                user_id
            )
            .fetch_all(&self.pool)
            .await?;
            
            Ok(metrics)
        }).await
    }

    /// Record a WebDAV HTTP request metric
    pub async fn record_webdav_request_metric(
        &self, 
        metric: &CreateWebDAVRequestMetric
    ) -> Result<Uuid> {
        self.with_retry(|| async {
            let row = sqlx::query!(
                r#"
                INSERT INTO webdav_request_metrics (
                    session_id, directory_metric_id, user_id, source_id,
                    request_type, operation_type, target_path, duration_ms,
                    request_size_bytes, response_size_bytes, http_status_code,
                    dns_lookup_ms, tcp_connect_ms, tls_handshake_ms, time_to_first_byte_ms,
                    success, retry_attempt, error_type, error_message,
                    server_header, dav_header, etag_value, last_modified,
                    content_type, remote_ip, user_agent,
                    completed_at
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15,
                    $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, NOW()
                )
                RETURNING id
                "#,
                metric.session_id,
                metric.directory_metric_id,
                metric.user_id,
                metric.source_id,
                metric.request_type.to_string(),
                metric.operation_type.to_string(),
                metric.target_path,
                metric.duration_ms,
                metric.request_size_bytes,
                metric.response_size_bytes,
                metric.http_status_code,
                metric.dns_lookup_ms,
                metric.tcp_connect_ms,
                metric.tls_handshake_ms,
                metric.time_to_first_byte_ms,
                metric.success,
                metric.retry_attempt,
                metric.error_type,
                metric.error_message,
                metric.server_header,
                metric.dav_header,
                metric.etag_value,
                metric.last_modified,
                metric.content_type,
                metric.remote_ip,
                metric.user_agent
            )
            .fetch_one(&self.pool)
            .await?;
            
            Ok(row.id)
        }).await
    }

    /// Get request metrics for a session or directory
    pub async fn get_webdav_request_metrics(
        &self,
        session_id: Option<Uuid>,
        directory_metric_id: Option<Uuid>,
        user_id: Uuid,
        limit: Option<i32>
    ) -> Result<Vec<WebDAVRequestMetric>> {
        self.with_retry(|| async {
            let limit = limit.unwrap_or(1000).min(10000); // Cap at 10k

            let metrics = sqlx::query_as!(
                WebDAVRequestMetric,
                r#"
                SELECT * FROM webdav_request_metrics 
                WHERE user_id = $1
                AND ($2::UUID IS NULL OR session_id = $2)
                AND ($3::UUID IS NULL OR directory_metric_id = $3)
                ORDER BY started_at DESC
                LIMIT $4
                "#,
                user_id,
                session_id,
                directory_metric_id,
                limit as i64
            )
            .fetch_all(&self.pool)
            .await?;
            
            Ok(metrics)
        }).await
    }

    /// Get WebDAV metrics summary for a time period
    pub async fn get_webdav_metrics_summary(
        &self,
        query: &WebDAVMetricsQuery
    ) -> Result<Option<WebDAVMetricsSummary>> {
        self.with_retry(|| async {
            let start_time = query.start_time.unwrap_or_else(|| Utc::now() - chrono::Duration::days(1));
            let end_time = query.end_time.unwrap_or_else(|| Utc::now());

            let summary = sqlx::query_as!(
                WebDAVMetricsSummary,
                r#"
                SELECT * FROM get_webdav_metrics_summary($1, $2, $3, $4)
                "#,
                query.user_id,
                query.source_id,
                start_time,
                end_time
            )
            .fetch_optional(&self.pool)
            .await?;
            
            Ok(summary)
        }).await
    }

    /// Get performance insights for a specific session
    pub async fn get_webdav_performance_insights(
        &self,
        session_id: Uuid,
        user_id: Uuid
    ) -> Result<Option<WebDAVPerformanceInsights>> {
        self.with_retry(|| async {
            // Get session info
            let session = self.get_webdav_sync_session(session_id, user_id).await?;
            if session.is_none() {
                return Ok(None);
            }

            // Get directory metrics
            let directory_metrics = self.get_webdav_directory_metrics(session_id, user_id).await?;

            // Calculate average directory scan time
            let avg_directory_scan_time_ms = if !directory_metrics.is_empty() {
                directory_metrics.iter()
                    .filter_map(|d| d.scan_duration_ms)
                    .sum::<i64>() as f64 / directory_metrics.len() as f64
            } else {
                0.0
            };

            // Find slowest directories
            let mut slowest_directories: Vec<SlowDirectoryInfo> = directory_metrics.iter()
                .filter_map(|d| {
                    d.scan_duration_ms.map(|duration| SlowDirectoryInfo {
                        path: d.directory_path.clone(),
                        scan_duration_ms: duration,
                        files_count: d.files_found,
                        size_bytes: d.total_size_bytes,
                        error_count: d.errors_encountered,
                    })
                })
                .collect();
            slowest_directories.sort_by(|a, b| b.scan_duration_ms.cmp(&a.scan_duration_ms));
            slowest_directories.truncate(10); // Top 10

            // Get request metrics for analysis
            let request_metrics = self.get_webdav_request_metrics(
                Some(session_id), 
                None, 
                user_id, 
                Some(10000)
            ).await?;

            // Calculate request type distribution
            let propfind_requests: Vec<_> = request_metrics.iter()
                .filter(|r| r.request_type == "PROPFIND")
                .collect();
            let get_requests: Vec<_> = request_metrics.iter()
                .filter(|r| r.request_type == "GET")
                .collect();

            let request_distribution = RequestTypeDistribution {
                propfind_count: propfind_requests.len() as i32,
                get_count: get_requests.len() as i32,
                head_count: request_metrics.iter().filter(|r| r.request_type == "HEAD").count() as i32,
                options_count: request_metrics.iter().filter(|r| r.request_type == "OPTIONS").count() as i32,
                total_count: request_metrics.len() as i32,
                avg_propfind_duration_ms: if !propfind_requests.is_empty() {
                    propfind_requests.iter().map(|r| r.duration_ms).sum::<i64>() as f64 / propfind_requests.len() as f64
                } else { 0.0 },
                avg_get_duration_ms: if !get_requests.is_empty() {
                    get_requests.iter().map(|r| r.duration_ms).sum::<i64>() as f64 / get_requests.len() as f64
                } else { 0.0 },
            };

            // Analyze errors
            let total_errors = request_metrics.iter().filter(|r| !r.success).count() as i32;
            let network_errors = request_metrics.iter()
                .filter(|r| !r.success && r.error_type.as_ref().map_or(false, |e| e.contains("network") || e.contains("timeout")))
                .count() as i32;
            let auth_errors = request_metrics.iter()
                .filter(|r| !r.success && r.http_status_code.map_or(false, |s| s == 401 || s == 403))
                .count() as i32;
            let timeout_errors = request_metrics.iter()
                .filter(|r| !r.success && r.error_type.as_ref().map_or(false, |e| e.contains("timeout")))
                .count() as i32;
            let server_errors = request_metrics.iter()
                .filter(|r| !r.success && r.http_status_code.map_or(false, |s| s >= 500))
                .count() as i32;

            // Find most problematic paths
            let mut path_errors: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
            for metric in &request_metrics {
                if !metric.success {
                    *path_errors.entry(metric.target_path.clone()).or_insert(0) += 1;
                }
            }
            let mut most_problematic_paths: Vec<_> = path_errors.into_iter().collect();
            most_problematic_paths.sort_by(|a, b| b.1.cmp(&a.1));
            let most_problematic_paths: Vec<String> = most_problematic_paths.into_iter()
                .take(5)
                .map(|(path, _)| path)
                .collect();

            let error_analysis = ErrorAnalysis {
                total_errors,
                network_errors,
                auth_errors,
                timeout_errors,
                server_errors,
                most_problematic_paths,
            };

            // Create simple performance trends (would be more sophisticated in production)
            let performance_trends = PerformanceTrends {
                requests_per_minute: vec![], // Would calculate from time-series data
                avg_response_time_trend: vec![],
                error_rate_trend: vec![],
                throughput_mbps_trend: vec![],
            };

            Ok(Some(WebDAVPerformanceInsights {
                session_id,
                avg_directory_scan_time_ms,
                slowest_directories,
                request_distribution,
                error_analysis,
                performance_trends,
            }))
        }).await
    }

    /// Clean up old WebDAV metrics (for maintenance)
    pub async fn cleanup_old_webdav_metrics(&self, days_to_keep: i32) -> Result<u64> {
        self.with_retry(|| async {
            let cutoff_date = Utc::now() - chrono::Duration::days(days_to_keep as i64);
            
            let result = sqlx::query!(
                r#"
                DELETE FROM webdav_sync_sessions 
                WHERE created_at < $1 
                AND status IN ('completed', 'failed', 'cancelled')
                "#,
                cutoff_date
            )
            .execute(&self.pool)
            .await?;
            
            Ok(result.rows_affected())
        }).await
    }
}