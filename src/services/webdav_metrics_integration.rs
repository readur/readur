use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;
use tracing::{debug, warn};
use crate::services::webdav_metrics_simple::WebDAVMetrics;

/// Integration layer for WebDAV metrics
/// This provides a clean interface for WebDAV services to record metrics
/// without coupling them tightly to the metrics implementation
pub struct WebDAVMetricsCollector {
    metrics: Arc<WebDAVMetrics>,
}

impl WebDAVMetricsCollector {
    pub fn new(metrics: Arc<WebDAVMetrics>) -> Self {
        Self { metrics }
    }

    /// Create a new session tracker
    pub fn start_session(&self, user_id: Uuid, source_id: Option<Uuid>) -> SessionTracker {
        SessionTracker {
            metrics: Arc::clone(&self.metrics),
            user_id,
            source_id,
            start_time: Instant::now(),
            files_processed: 0,
            bytes_processed: 0,
            requests_made: 0,
            successful_requests: 0,
        }
    }

    /// Record a standalone request (not part of a session)
    pub async fn record_standalone_request(&self, success: bool, duration_ms: u64) {
        self.metrics.record_request(success, duration_ms).await;
    }

    /// Get metrics for Prometheus export
    pub async fn get_prometheus_metrics(&self) -> crate::services::webdav_metrics_simple::PrometheusMetrics {
        self.metrics.get_prometheus_metrics().await
    }
}

/// Tracks metrics for a single WebDAV sync session
/// This replaces the complex database-backed session tracking
pub struct SessionTracker {
    metrics: Arc<WebDAVMetrics>,
    user_id: Uuid,
    source_id: Option<Uuid>,
    start_time: Instant,
    files_processed: u32,
    bytes_processed: u64,
    requests_made: u32,
    successful_requests: u32,
}

impl SessionTracker {
    /// Record that files were processed in this session
    pub fn record_files_processed(&mut self, count: u32, bytes: u64) {
        self.files_processed += count;
        self.bytes_processed += bytes;
        
        debug!("Session {}: processed {} files, {} bytes total", 
               self.user_id, self.files_processed, self.bytes_processed);
    }

    /// Record an HTTP request made during this session
    pub async fn record_request(&mut self, success: bool, duration_ms: u64) {
        self.requests_made += 1;
        if success {
            self.successful_requests += 1;
        }

        // Record in global metrics
        self.metrics.record_request(success, duration_ms).await;
        
        debug!("Session {}: request {} (success: {}, duration: {}ms)", 
               self.user_id, self.requests_made, success, duration_ms);
    }

    /// Complete the session successfully
    pub async fn complete_success(self) {
        let duration_ms = self.start_time.elapsed().as_millis() as u64;
        
        self.metrics.record_session(
            true,
            duration_ms,
            self.files_processed,
            self.bytes_processed,
        ).await;
        
        debug!("Session {} completed successfully: {}ms, {} files, {} bytes, {}/{} requests successful",
               self.user_id, duration_ms, self.files_processed, self.bytes_processed,
               self.successful_requests, self.requests_made);
    }

    /// Complete the session with failure
    pub async fn complete_failure(self, _error: &str) {
        let duration_ms = self.start_time.elapsed().as_millis() as u64;
        
        self.metrics.record_session(
            false,
            duration_ms,
            self.files_processed,
            self.bytes_processed,
        ).await;
        
        warn!("Session {} failed after {}ms: {} files, {} bytes, {}/{} requests successful",
               self.user_id, duration_ms, self.files_processed, self.bytes_processed,
               self.successful_requests, self.requests_made);
    }

    /// Get current session stats (for debugging/logging)
    pub fn current_stats(&self) -> SessionStats {
        SessionStats {
            user_id: self.user_id,
            source_id: self.source_id,
            duration_ms: self.start_time.elapsed().as_millis() as u64,
            files_processed: self.files_processed,
            bytes_processed: self.bytes_processed,
            requests_made: self.requests_made,
            successful_requests: self.successful_requests,
        }
    }
}

/// Simple session statistics for logging/debugging
#[derive(Debug, Clone)]
pub struct SessionStats {
    pub user_id: Uuid,
    pub source_id: Option<Uuid>,
    pub duration_ms: u64,
    pub files_processed: u32,
    pub bytes_processed: u64,
    pub requests_made: u32,
    pub successful_requests: u32,
}

/// Request timing helper for easy request measurement
pub struct RequestTimer {
    start_time: Instant,
}

impl RequestTimer {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }

    /// Complete and record the request
    pub async fn complete(self, session: &mut SessionTracker, success: bool) {
        let duration_ms = self.elapsed_ms();
        session.record_request(success, duration_ms).await;
    }
}

impl Default for RequestTimer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_session_tracking() {
        let metrics = Arc::new(WebDAVMetrics::new());
        let collector = WebDAVMetricsCollector::new(metrics);
        
        let user_id = Uuid::new_v4();
        let source_id = Some(Uuid::new_v4());
        
        // Start session
        let mut session = collector.start_session(user_id, source_id);
        
        // Simulate processing files
        session.record_files_processed(5, 1024000);
        session.record_files_processed(3, 512000);
        
        // Simulate HTTP requests
        session.record_request(true, 100).await;
        session.record_request(true, 150).await;
        session.record_request(false, 5000).await; // Failed request
        
        let stats = session.current_stats();
        assert_eq!(stats.files_processed, 8);
        assert_eq!(stats.bytes_processed, 1536000);
        assert_eq!(stats.requests_made, 3);
        assert_eq!(stats.successful_requests, 2);
        
        // Complete successfully
        session.complete_success().await;
        
        // Check metrics
        let prometheus_metrics = collector.get_prometheus_metrics().await;
        assert_eq!(prometheus_metrics.total_sessions, 1);
        assert_eq!(prometheus_metrics.successful_sessions, 1);
        assert_eq!(prometheus_metrics.total_files_processed, 8);
        assert_eq!(prometheus_metrics.total_bytes_processed, 1536000);
        assert_eq!(prometheus_metrics.total_http_requests, 3);
    }

    #[tokio::test]
    async fn test_request_timer() {
        let metrics = Arc::new(WebDAVMetrics::new());
        let collector = WebDAVMetricsCollector::new(metrics);
        
        let user_id = Uuid::new_v4();
        let mut session = collector.start_session(user_id, None);
        
        // Test request timing
        let timer = RequestTimer::new();
        sleep(Duration::from_millis(10)).await; // Simulate work
        let duration_before = timer.elapsed_ms();
        
        timer.complete(&mut session, true).await;
        
        // Should have recorded a request with reasonable duration
        assert!(duration_before >= 10);
        
        let stats = session.current_stats();
        assert_eq!(stats.requests_made, 1);
        assert_eq!(stats.successful_requests, 1);
    }

    #[tokio::test]
    async fn test_failed_session() {
        let metrics = Arc::new(WebDAVMetrics::new());
        let collector = WebDAVMetricsCollector::new(metrics);
        
        let user_id = Uuid::new_v4();
        let mut session = collector.start_session(user_id, None);
        
        // Process some data before failure
        session.record_files_processed(2, 100000);
        session.record_request(true, 100).await;
        session.record_request(false, 200).await;
        
        // Complete with failure
        session.complete_failure("Connection error").await;
        
        // Check metrics
        let prometheus_metrics = collector.get_prometheus_metrics().await;
        assert_eq!(prometheus_metrics.total_sessions, 1);
        assert_eq!(prometheus_metrics.successful_sessions, 0);
        assert_eq!(prometheus_metrics.failed_sessions, 1);
        assert_eq!(prometheus_metrics.total_files_processed, 2);
        assert_eq!(prometheus_metrics.total_http_requests, 2);
    }
}