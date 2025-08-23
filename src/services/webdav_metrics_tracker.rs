use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::{HashMap, VecDeque};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use reqwest::header::HeaderMap;

use crate::db::Database;
use crate::models::webdav_metrics::*;
use crate::services::webdav::build_user_agent;

/// Maximum number of response times to keep in memory to prevent unbounded growth
const MAX_RESPONSE_TIMES: usize = 1000;

/// Duration after which inactive sessions are considered stale and cleaned up
const SESSION_TIMEOUT_MINUTES: u64 = 60;

/// Duration after which inactive directory scans are considered stale and cleaned up
const DIRECTORY_TIMEOUT_MINUTES: u64 = 30;

/// WebDAV metrics collector that tracks performance and operations
/// 
/// This service collects detailed metrics about WebDAV sync operations including:
/// - Overall sync session metrics (files processed, time taken, etc.)
/// - Per-directory scan metrics (discovery time, file counts, errors)
/// - Individual HTTP request metrics (response times, success/failure rates)
/// 
/// The metrics are stored in the database for analysis and can be used to:
/// - Identify performance bottlenecks
/// - Track sync operation success rates
/// - Analyze network performance patterns
/// - Generate insights for optimization
#[derive(Clone)]
pub struct WebDAVMetricsTracker {
    db: Database,
    /// Active sessions being tracked
    active_sessions: Arc<RwLock<HashMap<Uuid, ActiveSession>>>,
    /// Active directory scans being tracked
    active_directories: Arc<RwLock<HashMap<Uuid, ActiveDirectoryScan>>>,
}

/// Represents an active sync session being tracked
struct ActiveSession {
    session_id: Uuid,
    user_id: Uuid,
    source_id: Option<Uuid>,
    started_at: Instant,
    last_activity: Instant,
    counters: SessionCounters,
}

/// Session-level counters that are updated during the sync
#[derive(Default)]
struct SessionCounters {
    directories_discovered: i32,
    directories_processed: i32,
    files_discovered: i32,
    files_processed: i32,
    total_bytes_discovered: i64,
    total_bytes_processed: i64,
    directories_skipped: i32,
    files_skipped: i32,
    skip_reasons: HashMap<String, i32>,
}

/// Represents an active directory scan being tracked
struct ActiveDirectoryScan {
    metric_id: Uuid,
    session_id: Uuid,
    directory_path: String,
    started_at: Instant,
    last_activity: Instant,
    counters: DirectoryCounters,
}

/// Directory-level counters
#[derive(Default)]
struct DirectoryCounters {
    files_found: i32,
    subdirectories_found: i32,
    total_size_bytes: i64,
    files_processed: i32,
    files_skipped: i32,
    files_failed: i32,
    http_requests_made: i32,
    propfind_requests: i32,
    get_requests: i32,
    errors_encountered: i32,
    error_types: Vec<String>,
    warnings_count: i32,
    response_times: VecDeque<i64>, // Use VecDeque for O(1) front removal
    etag_matches: i32,
    etag_mismatches: i32,
    cache_hits: i32,
    cache_misses: i32,
}

impl WebDAVMetricsTracker {
    /// Create a new WebDAV metrics tracker
    pub fn new(db: Database) -> Self {
        Self {
            db,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            active_directories: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start tracking a new WebDAV sync session
    pub async fn start_session(
        &self,
        user_id: Uuid,
        source_id: Option<Uuid>,
        sync_type: String,
        root_path: String,
        max_depth: Option<i32>,
    ) -> Result<Uuid> {
        let create_session = CreateWebDAVSyncSession {
            user_id,
            source_id,
            sync_type,
            root_path,
            max_depth,
        };

        let session_id = self.db.create_webdav_sync_session(&create_session).await?;

        let now = Instant::now();
        let active_session = ActiveSession {
            session_id,
            user_id,
            source_id,
            started_at: now,
            last_activity: now,
            counters: SessionCounters::default(),
        };

        self.active_sessions.write().await.insert(session_id, active_session);

        info!(
            "Started WebDAV metrics tracking for session {} (user: {}, source: {:?})",
            session_id, user_id, source_id
        );

        Ok(session_id)
    }

    /// Update session counters
    pub async fn update_session_counters(
        &self,
        session_id: Uuid,
        directories_discovered_delta: i32,
        directories_processed_delta: i32,
        files_discovered_delta: i32,
        files_processed_delta: i32,
        bytes_discovered_delta: i64,
        bytes_processed_delta: i64,
    ) -> Result<()> {
        let mut sessions = self.active_sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_id) {
            session.last_activity = Instant::now();
            session.counters.directories_discovered += directories_discovered_delta;
            session.counters.directories_processed += directories_processed_delta;
            session.counters.files_discovered += files_discovered_delta;
            session.counters.files_processed += files_processed_delta;
            session.counters.total_bytes_discovered += bytes_discovered_delta;
            session.counters.total_bytes_processed += bytes_processed_delta;

            debug!(
                "Updated session {} counters: +{} dirs, +{} files, +{} bytes",
                session_id, directories_processed_delta, files_processed_delta, bytes_processed_delta
            );
        }
        Ok(())
    }

    /// Record skipped items with reasons
    pub async fn record_skipped_items(
        &self,
        session_id: Uuid,
        directories_skipped: i32,
        files_skipped: i32,
        skip_reason: &str,
    ) -> Result<()> {
        let mut sessions = self.active_sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_id) {
            session.last_activity = Instant::now();
            session.counters.directories_skipped += directories_skipped;
            session.counters.files_skipped += files_skipped;
            *session.counters.skip_reasons.entry(skip_reason.to_string()).or_insert(0) += 
                directories_skipped + files_skipped;
        }
        Ok(())
    }

    /// Finish a sync session and calculate final metrics
    pub async fn finish_session(
        &self,
        session_id: Uuid,
        final_status: WebDAVSyncStatus,
        error_message: Option<String>,
    ) -> Result<()> {
        let session = {
            let mut sessions = self.active_sessions.write().await;
            sessions.remove(&session_id)
        };

        if let Some(session) = session {
            // Convert skip_reasons to JSON
            let skip_reasons_json = if session.counters.skip_reasons.is_empty() {
                None
            } else {
                Some(serde_json::to_value(&session.counters.skip_reasons)?)
            };

            let update = UpdateWebDAVSyncSession {
                directories_discovered: Some(session.counters.directories_discovered),
                directories_processed: Some(session.counters.directories_processed),
                files_discovered: Some(session.counters.files_discovered),
                files_processed: Some(session.counters.files_processed),
                total_bytes_discovered: Some(session.counters.total_bytes_discovered),
                total_bytes_processed: Some(session.counters.total_bytes_processed),
                directories_skipped: Some(session.counters.directories_skipped),
                files_skipped: Some(session.counters.files_skipped),
                skip_reasons: skip_reasons_json,
                status: Some(final_status),
                final_error_message: error_message,
            };

            self.db.update_webdav_sync_session(session_id, &update).await?;
            
            // Small delay to ensure all previous HTTP request inserts are committed
            // This addresses a transaction isolation issue where the finalize function
            // can't see the requests that were just inserted
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            
            self.db.finalize_webdav_sync_session(session_id).await?;

            info!(
                "Finished WebDAV session {} - processed {} files ({} bytes) in {} directories",
                session_id,
                session.counters.files_processed,
                session.counters.total_bytes_processed,
                session.counters.directories_processed
            );
        }

        Ok(())
    }

    /// Start tracking a directory scan
    pub async fn start_directory_scan(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        source_id: Option<Uuid>,
        directory_path: String,
        directory_depth: i32,
        parent_directory_path: Option<String>,
    ) -> Result<Uuid> {
        let create_metric = CreateWebDAVDirectoryMetric {
            session_id,
            user_id,
            source_id,
            directory_path: directory_path.clone(),
            directory_depth,
            parent_directory_path,
        };

        let metric_id = self.db.create_webdav_directory_metric(&create_metric).await?;

        let now = Instant::now();
        let active_scan = ActiveDirectoryScan {
            metric_id,
            session_id,
            directory_path: directory_path.clone(),
            started_at: now,
            last_activity: now,
            counters: DirectoryCounters::default(),
        };

        self.active_directories.write().await.insert(metric_id, active_scan);

        debug!(
            "Started directory scan tracking for '{}' (metric: {}, session: {})",
            directory_path, metric_id, session_id
        );

        Ok(metric_id)
    }

    /// Update directory scan counters
    pub async fn update_directory_counters(
        &self,
        metric_id: Uuid,
        files_found_delta: i32,
        subdirectories_found_delta: i32,
        size_bytes_delta: i64,
        files_processed_delta: i32,
        files_skipped_delta: i32,
        files_failed_delta: i32,
    ) -> Result<()> {
        let mut directories = self.active_directories.write().await;
        if let Some(scan) = directories.get_mut(&metric_id) {
            scan.last_activity = Instant::now();
            scan.counters.files_found += files_found_delta;
            scan.counters.subdirectories_found += subdirectories_found_delta;
            scan.counters.total_size_bytes += size_bytes_delta;
            scan.counters.files_processed += files_processed_delta;
            scan.counters.files_skipped += files_skipped_delta;
            scan.counters.files_failed += files_failed_delta;
        }
        Ok(())
    }

    /// Record directory scan error
    pub async fn record_directory_error(
        &self,
        metric_id: Uuid,
        error_type: &str,
        is_warning: bool,
    ) -> Result<()> {
        let mut directories = self.active_directories.write().await;
        if let Some(scan) = directories.get_mut(&metric_id) {
            scan.last_activity = Instant::now();
            if is_warning {
                scan.counters.warnings_count += 1;
            } else {
                scan.counters.errors_encountered += 1;
                scan.counters.error_types.push(error_type.to_string());
            }
        }
        Ok(())
    }

    /// Record ETag comparison result
    pub async fn record_etag_result(
        &self,
        metric_id: Uuid,
        etag_matched: bool,
        cache_hit: bool,
    ) -> Result<()> {
        let mut directories = self.active_directories.write().await;
        if let Some(scan) = directories.get_mut(&metric_id) {
            scan.last_activity = Instant::now();
            if etag_matched {
                scan.counters.etag_matches += 1;
            } else {
                scan.counters.etag_mismatches += 1;
            }

            if cache_hit {
                scan.counters.cache_hits += 1;
            } else {
                scan.counters.cache_misses += 1;
            }
        }
        Ok(())
    }

    /// Finish a directory scan
    pub async fn finish_directory_scan(
        &self,
        metric_id: Uuid,
        status: &str,
        skip_reason: Option<String>,
        error_message: Option<String>,
    ) -> Result<()> {
        let scan = {
            let mut directories = self.active_directories.write().await;
            directories.remove(&metric_id)
        };

        if let Some(scan) = scan {
            // Convert error types to JSON
            let error_types_json = if scan.counters.error_types.is_empty() {
                None
            } else {
                Some(serde_json::to_value(&scan.counters.error_types)?)
            };

            let update = UpdateWebDAVDirectoryMetric {
                files_found: Some(scan.counters.files_found),
                subdirectories_found: Some(scan.counters.subdirectories_found),
                total_size_bytes: Some(scan.counters.total_size_bytes),
                files_processed: Some(scan.counters.files_processed),
                files_skipped: Some(scan.counters.files_skipped),
                files_failed: Some(scan.counters.files_failed),
                http_requests_made: Some(scan.counters.http_requests_made),
                propfind_requests: Some(scan.counters.propfind_requests),
                get_requests: Some(scan.counters.get_requests),
                errors_encountered: Some(scan.counters.errors_encountered),
                error_types: error_types_json,
                warnings_count: Some(scan.counters.warnings_count),
                etag_matches: Some(scan.counters.etag_matches),
                etag_mismatches: Some(scan.counters.etag_mismatches),
                cache_hits: Some(scan.counters.cache_hits),
                cache_misses: Some(scan.counters.cache_misses),
                status: Some(status.to_string()),
                skip_reason,
                error_message,
            };

            self.db.update_webdav_directory_metric(metric_id, &update).await?;

            debug!(
                "Finished directory scan '{}' - found {} files, processed {} files, {} errors",
                scan.directory_path,
                scan.counters.files_found,
                scan.counters.files_processed,
                scan.counters.errors_encountered
            );
        }

        Ok(())
    }

    /// Record an HTTP request metric
    pub async fn record_http_request(
        &self,
        session_id: Option<Uuid>,
        directory_metric_id: Option<Uuid>,
        user_id: Uuid,
        source_id: Option<Uuid>,
        request_type: WebDAVRequestType,
        operation_type: WebDAVOperationType,
        target_path: String,
        duration: Duration,
        request_size_bytes: Option<i64>,
        response_size_bytes: Option<i64>,
        http_status_code: Option<i32>,
        success: bool,
        retry_attempt: i32,
        error_type: Option<String>,
        error_message: Option<String>,
        server_headers: Option<&HeaderMap>,
        remote_ip: Option<String>,
    ) -> Result<Uuid> {
        // Extract server information from headers
        let server_header = server_headers
            .and_then(|h| h.get("server"))
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let dav_header = server_headers
            .and_then(|h| h.get("dav"))
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let etag_value = server_headers
            .and_then(|h| h.get("etag"))
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let content_type = server_headers
            .and_then(|h| h.get("content-type"))
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let last_modified = server_headers
            .and_then(|h| h.get("last-modified"))
            .and_then(|v| v.to_str().ok())
            .and_then(|s| chrono::DateTime::parse_from_rfc2822(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));

        let metric = CreateWebDAVRequestMetric {
            session_id,
            directory_metric_id,
            user_id,
            source_id,
            request_type,
            operation_type,
            target_path: target_path.clone(),
            duration_ms: duration.as_millis() as i64,
            request_size_bytes,
            response_size_bytes,
            http_status_code,
            dns_lookup_ms: None,    // Could be enhanced with detailed timing
            tcp_connect_ms: None,   // Could be enhanced with detailed timing
            tls_handshake_ms: None, // Could be enhanced with detailed timing
            time_to_first_byte_ms: None, // Could be enhanced with detailed timing
            success,
            retry_attempt,
            error_type: error_type.clone(),
            error_message,
            server_header,
            dav_header,
            etag_value,
            last_modified,
            content_type,
            remote_ip,
            user_agent: Some(build_user_agent()),
        };

        tracing::debug!("Recording request with session_id: {:?}", session_id);
        let request_id = self.db.record_webdav_request_metric(&metric).await?;

        // Update active directory counters if applicable
        if let Some(dir_metric_id) = directory_metric_id {
            let mut directories = self.active_directories.write().await;
            if let Some(scan) = directories.get_mut(&dir_metric_id) {
                scan.last_activity = Instant::now();
                scan.counters.http_requests_made += 1;
                
                // Implement bounded circular buffer for response times using VecDeque for O(1) operations
                scan.counters.response_times.push_back(duration.as_millis() as i64);
                if scan.counters.response_times.len() > MAX_RESPONSE_TIMES {
                    scan.counters.response_times.pop_front(); // O(1) removal of oldest entry
                }

                match request_type {
                    WebDAVRequestType::PropFind => scan.counters.propfind_requests += 1,
                    WebDAVRequestType::Get => scan.counters.get_requests += 1,
                    _ => {}
                }

                if !success {
                    scan.counters.errors_encountered += 1;
                    if let Some(err_type) = &error_type {
                        scan.counters.error_types.push(err_type.clone());
                    }
                }
            }
        }

        debug!(
            "Recorded HTTP request: {} {} -> {} ({}ms, success: {})",
            request_type, target_path, 
            http_status_code.map(|c| c.to_string()).unwrap_or_else(|| "N/A".to_string()),
            duration.as_millis(), success
        );

        Ok(request_id)
    }

    /// Get metrics summary for a user or source
    pub async fn get_metrics_summary(
        &self,
        query: &WebDAVMetricsQuery,
    ) -> Result<Option<WebDAVMetricsSummary>> {
        self.db.get_webdav_metrics_summary(query).await
    }

    /// Get performance insights for a session
    pub async fn get_performance_insights(
        &self,
        session_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<WebDAVPerformanceInsights>> {
        self.db.get_webdav_performance_insights(session_id, user_id).await
    }

    /// List recent sessions for a user
    pub async fn list_sessions(
        &self,
        query: &WebDAVMetricsQuery,
    ) -> Result<Vec<WebDAVSyncSession>> {
        self.db.list_webdav_sync_sessions(query).await
    }

    /// Get detailed session information
    pub async fn get_session_details(
        &self,
        session_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<WebDAVSyncSession>> {
        self.db.get_webdav_sync_session(session_id, user_id).await
    }

    /// Get directory metrics for a session
    pub async fn get_directory_metrics(
        &self,
        session_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<WebDAVDirectoryMetric>> {
        self.db.get_webdav_directory_metrics(session_id, user_id).await
    }

    /// Get request metrics for analysis
    pub async fn get_request_metrics(
        &self,
        session_id: Option<Uuid>,
        directory_metric_id: Option<Uuid>,
        user_id: Uuid,
        limit: Option<i32>,
    ) -> Result<Vec<WebDAVRequestMetric>> {
        self.db.get_webdav_request_metrics(session_id, directory_metric_id, user_id, limit).await
    }

    /// Clean up old metrics (should be called periodically)
    pub async fn cleanup_old_metrics(&self, days_to_keep: i32) -> Result<u64> {
        self.db.cleanup_old_webdav_metrics(days_to_keep).await
    }

    /// Utility method to record a simple operation timing
    pub async fn time_operation<T, F, Fut>(
        &self,
        session_id: Option<Uuid>,
        directory_metric_id: Option<Uuid>,
        user_id: Uuid,
        source_id: Option<Uuid>,
        request_type: WebDAVRequestType,
        operation_type: WebDAVOperationType,
        target_path: String,
        operation: F,
    ) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let start_time = Instant::now();
        let result = operation().await;
        let duration = start_time.elapsed();

        let (success, error_type, error_message) = match &result {
            Ok(_) => (true, None, None),
            Err(e) => (false, Some("operation_error".to_string()), Some(e.to_string())),
        };

        // Record the request metric (ignore errors in metrics recording)
        let _ = self.record_http_request(
            session_id,
            directory_metric_id,
            user_id,
            source_id,
            request_type,
            operation_type,
            target_path,
            duration,
            None, // request_size_bytes
            None, // response_size_bytes
            None, // http_status_code
            success,
            0, // retry_attempt
            error_type,
            error_message,
            None, // server_headers
            None, // remote_ip
        ).await;

        result
    }

    /// Clean up stale sessions and directories to prevent memory leaks
    /// This should be called periodically (e.g., every 15-30 minutes)
    pub async fn cleanup_stale_sessions(&self) -> Result<(usize, usize)> {
        let now = Instant::now();
        let session_timeout = Duration::from_secs(SESSION_TIMEOUT_MINUTES * 60);
        let directory_timeout = Duration::from_secs(DIRECTORY_TIMEOUT_MINUTES * 60);

        let mut sessions_cleaned = 0;
        let mut directories_cleaned = 0;

        // Cleanup stale sessions
        {
            let mut sessions = self.active_sessions.write().await;
            let stale_sessions: Vec<Uuid> = sessions
                .iter()
                .filter(|(_, session)| {
                    now.duration_since(session.last_activity) > session_timeout
                })
                .map(|(session_id, _)| *session_id)
                .collect();

            for session_id in &stale_sessions {
                if let Some(session) = sessions.remove(session_id) {
                    sessions_cleaned += 1;
                    warn!(
                        "🧹 Cleaned up stale WebDAV session {} after {} minutes of inactivity",
                        session_id,
                        now.duration_since(session.last_activity).as_secs() / 60
                    );

                    // Try to finalize the session in the database
                    let _ = self.finish_session(
                        *session_id,
                        WebDAVSyncStatus::Failed,
                        Some("Session timed out due to inactivity".to_string()),
                    ).await;
                }
            }
        }

        // Cleanup stale directory scans
        {
            let mut directories = self.active_directories.write().await;
            let stale_directories: Vec<Uuid> = directories
                .iter()
                .filter(|(_, scan)| {
                    now.duration_since(scan.last_activity) > directory_timeout
                })
                .map(|(metric_id, _)| *metric_id)
                .collect();

            for metric_id in &stale_directories {
                if let Some(scan) = directories.remove(metric_id) {
                    directories_cleaned += 1;
                    warn!(
                        "🧹 Cleaned up stale directory scan {} for path '{}' after {} minutes of inactivity",
                        metric_id,
                        scan.directory_path,
                        now.duration_since(scan.last_activity).as_secs() / 60
                    );

                    // Try to finalize the directory scan in the database
                    let _ = self.finish_directory_scan(
                        *metric_id,
                        "timeout",
                        Some("Scan timed out due to inactivity".to_string()),
                        Some("Directory scan exceeded maximum time limit".to_string()),
                    ).await;
                }
            }
        }

        if sessions_cleaned > 0 || directories_cleaned > 0 {
            info!(
                "🧹 Cleanup completed: {} stale sessions and {} stale directory scans removed",
                sessions_cleaned, directories_cleaned
            );
        }

        Ok((sessions_cleaned, directories_cleaned))
    }

    /// Get the number of active sessions and directories currently being tracked
    pub async fn get_active_counts(&self) -> (usize, usize) {
        let sessions_count = self.active_sessions.read().await.len();
        let directories_count = self.active_directories.read().await.len();
        (sessions_count, directories_count)
    }

    /// Manually cleanup all active sessions and directories (useful for testing)
    pub async fn cleanup_all(&self) -> Result<(usize, usize)> {
        // Cleanup all sessions
        let sessions_cleaned = {
            let mut sessions = self.active_sessions.write().await;
            let count = sessions.len();
            for (session_id, _) in sessions.drain() {
                let _ = self.finish_session(
                    session_id,
                    WebDAVSyncStatus::Failed,
                    Some("Manually cleaned up".to_string()),
                ).await;
            }
            count
        };

        // Cleanup all directories
        let directories_cleaned = {
            let mut directories = self.active_directories.write().await;
            let count = directories.len();
            for (metric_id, _) in directories.drain() {
                let _ = self.finish_directory_scan(
                    metric_id,
                    "cleanup",
                    Some("Manually cleaned up".to_string()),
                    Some("Manual cleanup operation".to_string()),
                ).await;
            }
            count
        };

        info!(
            "🧹 Manual cleanup completed: {} sessions and {} directories removed",
            sessions_cleaned, directories_cleaned
        );

        Ok((sessions_cleaned, directories_cleaned))
    }
}

/// Extension trait to add metrics tracking to any operation
pub trait WebDAVMetricsExt {
    async fn with_metrics<T, F, Fut>(
        self,
        tracker: &WebDAVMetricsTracker,
        session_id: Option<Uuid>,
        directory_metric_id: Option<Uuid>,
        user_id: Uuid,
        source_id: Option<Uuid>,
        request_type: WebDAVRequestType,
        operation_type: WebDAVOperationType,
        target_path: String,
        operation: F,
    ) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>;
}

impl<S> WebDAVMetricsExt for S {
    async fn with_metrics<T, F, Fut>(
        self,
        tracker: &WebDAVMetricsTracker,
        session_id: Option<Uuid>,
        directory_metric_id: Option<Uuid>,
        user_id: Uuid,
        source_id: Option<Uuid>,
        request_type: WebDAVRequestType,
        operation_type: WebDAVOperationType,
        target_path: String,
        operation: F,
    ) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        tracker.time_operation(
            session_id,
            directory_metric_id,
            user_id,
            source_id,
            request_type,
            operation_type,
            target_path,
            operation,
        ).await
    }
}