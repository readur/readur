use anyhow::Result;
use std::time::{Duration, Instant};
use uuid::Uuid;
use tracing::{debug, warn};

use crate::models::webdav_metrics::*;
use crate::services::webdav_metrics_tracker::WebDAVMetricsTracker;
use super::{WebDAVService, WebDAVDiscoveryResult};

/// Extension trait that adds metrics tracking to WebDAV operations
pub trait WebDAVServiceWithMetrics {
    /// Discover files and directories with metrics tracking
    async fn discover_with_metrics(
        &self,
        metrics_tracker: &WebDAVMetricsTracker,
        session_id: Uuid,
        directory_metric_id: Option<Uuid>,
        user_id: Uuid,
        source_id: Option<Uuid>,
        path: &str,
        depth: Option<i32>,
        file_extensions: &[String],
    ) -> Result<WebDAVDiscoveryResult>;

    /// Download file with metrics tracking
    async fn download_file_with_metrics(
        &self,
        metrics_tracker: &WebDAVMetricsTracker,
        session_id: Uuid,
        directory_metric_id: Option<Uuid>,
        user_id: Uuid,
        source_id: Option<Uuid>,
        file_url: &str,
        expected_size: Option<u64>,
    ) -> Result<super::WebDAVDownloadResult>;

    /// Test connection with metrics tracking
    async fn test_connection_with_metrics(
        &self,
        metrics_tracker: &WebDAVMetricsTracker,
        user_id: Uuid,
        source_id: Option<Uuid>,
    ) -> Result<super::HealthStatus>;
}

impl WebDAVServiceWithMetrics for WebDAVService {
    async fn discover_with_metrics(
        &self,
        metrics_tracker: &WebDAVMetricsTracker,
        session_id: Uuid,
        directory_metric_id: Option<Uuid>,
        user_id: Uuid,
        source_id: Option<Uuid>,
        path: &str,
        depth: Option<i32>,
        file_extensions: &[String],
    ) -> Result<WebDAVDiscoveryResult> {
        let start_time = Instant::now();
        
        // Start directory scan metrics if not provided
        let dir_metric_id = if let Some(id) = directory_metric_id {
            id
        } else {
            let path_depth = path.matches('/').count() as i32;
            let parent_path = if path == "/" {
                None
            } else {
                path.rfind('/').map(|pos| path[..pos].to_string())
            };

            metrics_tracker
                .start_directory_scan(
                    session_id,
                    user_id,
                    source_id,
                    path.to_string(),
                    path_depth,
                    parent_path,
                )
                .await?
        };

        // Record the discovery request
        let discovery_start = Instant::now();
        let discovery_result = self.discover_files_and_directories(path, depth.is_some()).await;
        let discovery_duration = discovery_start.elapsed();

        // Record HTTP request metric for the discovery operation
        let (success, error_type, error_message) = match &discovery_result {
            Ok(_) => (true, None, None),
            Err(e) => (false, Some("discovery_error".to_string()), Some(e.to_string())),
        };

        let _request_metric_id = metrics_tracker
            .record_http_request(
                Some(session_id),
                Some(dir_metric_id),
                user_id,
                source_id,
                WebDAVRequestType::PropFind,
                WebDAVOperationType::Discovery,
                path.to_string(),
                discovery_duration,
                None, // request_size_bytes
                None, // response_size_bytes (would need to track this in discover method)
                None, // http_status_code (would need to extract from discovery)
                success,
                0, // retry_attempt
                error_type,
                error_message,
                None, // server_headers (would need to pass through from discover)
                None, // remote_ip
            )
            .await
            .unwrap_or_else(|e| {
                warn!("Failed to record discovery request metric: {}", e);
                Uuid::new_v4() // Return dummy ID if metrics recording fails
            });

        match discovery_result {
            Ok(result) => {
                // Update directory metrics with discovery results
                let files_count = result.files.len() as i32;
                let dirs_count = result.directories.len() as i32;
                let total_size: u64 = result.files.iter()
                    .map(|f| f.size as u64)
                    .sum();

                metrics_tracker
                    .update_directory_counters(
                        dir_metric_id,
                        files_count,
                        dirs_count,
                        total_size as i64,
                        0, // files_processed (will be updated later)
                        0, // files_skipped
                        0, // files_failed
                    )
                    .await
                    .unwrap_or_else(|e| {
                        warn!("Failed to update directory counters: {}", e);
                    });

                // Update session counters
                metrics_tracker
                    .update_session_counters(
                        session_id,
                        dirs_count,
                        0, // directories_processed (will be updated later)
                        files_count,
                        0, // files_processed (will be updated later)
                        total_size as i64,
                        0, // bytes_processed (will be updated later)
                    )
                    .await
                    .unwrap_or_else(|e| {
                        warn!("Failed to update session counters: {}", e);
                    });

                debug!(
                    "Discovery completed for '{}': {} files, {} directories, {} bytes ({}ms)",
                    path, files_count, dirs_count, total_size, discovery_duration.as_millis()
                );

                Ok(result)
            }
            Err(e) => {
                // Record the error in directory metrics
                metrics_tracker
                    .record_directory_error(dir_metric_id, "discovery_failed", false)
                    .await
                    .unwrap_or_else(|err| {
                        warn!("Failed to record directory error: {}", err);
                    });

                // Finish the directory scan with error status
                metrics_tracker
                    .finish_directory_scan(
                        dir_metric_id,
                        "failed",
                        None,
                        Some(e.to_string()),
                    )
                    .await
                    .unwrap_or_else(|err| {
                        warn!("Failed to finish directory scan: {}", err);
                    });

                Err(e)
            }
        }
    }

    async fn download_file_with_metrics(
        &self,
        metrics_tracker: &WebDAVMetricsTracker,
        session_id: Uuid,
        directory_metric_id: Option<Uuid>,
        user_id: Uuid,
        source_id: Option<Uuid>,
        file_url: &str,
        expected_size: Option<u64>,
    ) -> Result<super::WebDAVDownloadResult> {
        let download_start = Instant::now();
        // Create a temporary FileIngestionInfo for download with mime detection
        let temp_file_info = crate::models::FileIngestionInfo {
            relative_path: file_url.to_string(),
            full_path: file_url.to_string(),
            path: file_url.to_string(),
            name: file_url.split('/').last().unwrap_or("unknown").to_string(),
            size: expected_size.unwrap_or(0) as i64,
            mime_type: "application/octet-stream".to_string(),
            last_modified: Some(chrono::Utc::now()),
            etag: "".to_string(),
            is_directory: false,
            created_at: None,
            permissions: None,
            owner: None,
            group: None,
            metadata: None,
        };
        let download_result = self.download_file_with_mime_detection(&temp_file_info).await;
        let download_duration = download_start.elapsed();

        let (success, error_type, error_message, response_size) = match &download_result {
            Ok(result) => (
                true,
                None,
                None,
                Some(result.content.len() as i64),
            ),
            Err(e) => (
                false,
                Some("download_error".to_string()),
                Some(e.to_string()),
                None,
            ),
        };

        // Record HTTP request metric for the download operation
        let _request_metric_id = metrics_tracker
            .record_http_request(
                Some(session_id),
                directory_metric_id,
                user_id,
                source_id,
                WebDAVRequestType::Get,
                WebDAVOperationType::Download,
                file_url.to_string(),
                download_duration,
                None, // request_size_bytes
                response_size,
                None, // http_status_code (would need to extract from download method)
                success,
                0, // retry_attempt
                error_type,
                error_message,
                None, // server_headers (would need to pass through)
                None, // remote_ip
            )
            .await
            .unwrap_or_else(|e| {
                warn!("Failed to record download request metric: {}", e);
                Uuid::new_v4()
            });

        match download_result {
            Ok(result) => {
                let file_size = result.content.len() as i64;

                // Update directory metrics if provided
                if let Some(dir_metric_id) = directory_metric_id {
                    metrics_tracker
                        .update_directory_counters(
                            dir_metric_id,
                            0, // files_found
                            0, // subdirectories_found
                            0, // size_bytes_delta (already counted in discovery)
                            1, // files_processed
                            0, // files_skipped
                            0, // files_failed
                        )
                        .await
                        .unwrap_or_else(|e| {
                            warn!("Failed to update directory counters for download: {}", e);
                        });
                }

                // Update session counters
                metrics_tracker
                    .update_session_counters(
                        session_id,
                        0, // directories_discovered
                        0, // directories_processed
                        0, // files_discovered
                        1, // files_processed
                        0, // bytes_discovered
                        file_size, // bytes_processed
                    )
                    .await
                    .unwrap_or_else(|e| {
                        warn!("Failed to update session counters for download: {}", e);
                    });

                debug!(
                    "Download completed for '{}': {} bytes ({}ms)",
                    file_url, file_size, download_duration.as_millis()
                );

                Ok(result)
            }
            Err(e) => {
                // Record failed download in directory metrics
                if let Some(dir_metric_id) = directory_metric_id {
                    metrics_tracker
                        .update_directory_counters(
                            dir_metric_id,
                            0, // files_found
                            0, // subdirectories_found
                            0, // size_bytes_delta
                            0, // files_processed
                            0, // files_skipped
                            1, // files_failed
                        )
                        .await
                        .unwrap_or_else(|err| {
                            warn!("Failed to update directory counters for failed download: {}", err);
                        });

                    metrics_tracker
                        .record_directory_error(dir_metric_id, "download_failed", false)
                        .await
                        .unwrap_or_else(|err| {
                            warn!("Failed to record directory error for download: {}", err);
                        });
                }

                Err(e)
            }
        }
    }

    async fn test_connection_with_metrics(
        &self,
        metrics_tracker: &WebDAVMetricsTracker,
        user_id: Uuid,
        source_id: Option<Uuid>,
    ) -> Result<super::HealthStatus> {
        let test_start = Instant::now();
        let test_result = self.test_connection().await;
        let test_duration = test_start.elapsed();

        let (success, error_type, error_message) = match &test_result {
            Ok(status) => (status.success, None, if status.success { None } else { Some(status.message.clone()) }),
            Err(e) => (false, Some("connection_test_error".to_string()), Some(e.to_string())),
        };

        // Record HTTP request metric for the connection test
        let _request_metric_id = metrics_tracker
            .record_http_request(
                None, // session_id (connection tests are not part of a sync session)
                None, // directory_metric_id
                user_id,
                source_id,
                WebDAVRequestType::Options,
                WebDAVOperationType::ConnectionTest,
                "/".to_string(), // Root path for connection test
                test_duration,
                None, // request_size_bytes
                None, // response_size_bytes
                None, // http_status_code
                success,
                0, // retry_attempt
                error_type,
                error_message,
                None, // server_headers
                None, // remote_ip
            )
            .await
            .unwrap_or_else(|e| {
                warn!("Failed to record connection test metric: {}", e);
                Uuid::new_v4()
            });

        debug!(
            "Connection test completed: success={}, duration={}ms",
            success, test_duration.as_millis()
        );

        // Convert WebDAVConnectionResult to HealthStatus
        match test_result {
            Ok(conn_result) => Ok(super::HealthStatus {
                healthy: conn_result.success,
                message: conn_result.message,
                response_time_ms: test_duration.as_millis() as u64,
                details: Some(serde_json::json!({
                    "server_version": conn_result.server_version,
                    "server_type": conn_result.server_type
                })),
            }),
            Err(e) => Ok(super::HealthStatus {
                healthy: false,
                message: e.to_string(),
                response_time_ms: test_duration.as_millis() as u64,
                details: None,
            }),
        }
    }
}

/// Helper struct for managing metrics during a complete sync operation
pub struct SyncWithMetrics<'a> {
    pub metrics_tracker: &'a WebDAVMetricsTracker,
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub source_id: Option<Uuid>,
    current_directory_metric: Option<Uuid>,
}

impl<'a> SyncWithMetrics<'a> {
    pub fn new(
        metrics_tracker: &'a WebDAVMetricsTracker,
        session_id: Uuid,
        user_id: Uuid,
        source_id: Option<Uuid>,
    ) -> Self {
        Self {
            metrics_tracker,
            session_id,
            user_id,
            source_id,
            current_directory_metric: None,
        }
    }

    /// Start tracking a new directory
    pub async fn start_directory(&mut self, directory_path: &str, depth: i32) -> Result<()> {
        let parent_path = if directory_path == "/" {
            None
        } else {
            directory_path.rfind('/').map(|pos| directory_path[..pos].to_string())
        };

        let metric_id = self.metrics_tracker
            .start_directory_scan(
                self.session_id,
                self.user_id,
                self.source_id,
                directory_path.to_string(),
                depth,
                parent_path,
            )
            .await?;

        self.current_directory_metric = Some(metric_id);
        Ok(())
    }

    /// Finish tracking the current directory
    pub async fn finish_directory(&mut self, status: &str, error_message: Option<String>) -> Result<()> {
        if let Some(metric_id) = self.current_directory_metric.take() {
            self.metrics_tracker
                .finish_directory_scan(metric_id, status, None, error_message)
                .await?;
        }
        Ok(())
    }

    /// Record a skipped item
    pub async fn record_skipped(&self, is_directory: bool, reason: &str) -> Result<()> {
        let (dirs_skipped, files_skipped) = if is_directory { (1, 0) } else { (0, 1) };
        
        self.metrics_tracker
            .record_skipped_items(self.session_id, dirs_skipped, files_skipped, reason)
            .await
    }

    /// Record an error
    pub async fn record_error(&self, error_type: &str, is_warning: bool) -> Result<()> {
        if let Some(metric_id) = self.current_directory_metric {
            self.metrics_tracker
                .record_directory_error(metric_id, error_type, is_warning)
                .await
        } else {
            Ok(())
        }
    }

    /// Record ETag comparison result
    pub async fn record_etag_result(&self, etag_matched: bool, cache_hit: bool) -> Result<()> {
        if let Some(metric_id) = self.current_directory_metric {
            self.metrics_tracker
                .record_etag_result(metric_id, etag_matched, cache_hit)
                .await
        } else {
            Ok(())
        }
    }

    /// Get the current directory metric ID
    pub fn current_directory_metric_id(&self) -> Option<Uuid> {
        self.current_directory_metric
    }
}