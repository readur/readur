use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

use serde::Serialize;

/// Simplified WebDAV metrics using atomic counters
/// Replaces the complex database-backed metrics system with in-memory tracking
#[derive(Clone)]
pub struct WebDAVMetrics {
    // Primary counters - these map directly to Prometheus metrics
    sessions_total: Arc<AtomicU64>,
    sessions_successful: Arc<AtomicU64>,
    sessions_failed: Arc<AtomicU64>,
    
    files_processed: Arc<AtomicU64>,
    bytes_processed: Arc<AtomicU64>,
    http_requests_total: Arc<AtomicU64>,
    http_requests_successful: Arc<AtomicU64>,
    
    // Time-windowed data for calculating rates and recent activity
    recent_sessions: Arc<RwLock<CircularBuffer<SessionEvent>>>,
    recent_requests: Arc<RwLock<CircularBuffer<RequestEvent>>>,
    
    // Cached calculations to avoid recomputing on every metrics request
    cached_calculations: Arc<RwLock<CachedCalculations>>,
    cache_timestamp: Arc<AtomicU64>,
}

/// Minimal session event for time-window calculations
#[derive(Debug, Clone)]
struct SessionEvent {
    timestamp: u64,
    success: bool,
    duration_ms: u64,
    files_count: u32,
    bytes_count: u64,
}

/// Minimal request event for time-window calculations
#[derive(Debug, Clone)]
struct RequestEvent {
    timestamp: u64,
    success: bool,
    duration_ms: u64,
}


/// Cached calculated metrics to avoid recomputation
#[derive(Debug, Clone)]
struct CachedCalculations {
    success_rate: f64,
    avg_session_duration_sec: f64,
    avg_processing_rate: f64,
    request_success_rate: f64,
    avg_request_duration_ms: f64,
    sessions_last_hour: u64,
    error_rate_last_hour: f64,
}

/// Prometheus metrics structure matching the current API
#[derive(Debug, Serialize)]
pub struct PrometheusMetrics {
    pub total_sessions: u64,
    pub successful_sessions: u64,
    pub failed_sessions: u64,
    pub success_rate: f64,
    pub total_files_processed: u64,
    pub total_bytes_processed: u64,
    pub avg_session_duration_sec: f64,
    pub avg_processing_rate: f64,
    pub total_http_requests: u64,
    pub request_success_rate: f64,
    pub avg_request_duration_ms: f64,
    pub sessions_last_hour: u64,
    pub error_rate_last_hour: f64,
}
/// Circular buffer for efficient time-window tracking
#[derive(Debug)]
struct CircularBuffer<T> {
    data: Vec<Option<T>>,
    head: usize,
    size: usize,
    capacity: usize,
}

impl<T> CircularBuffer<T> {
    fn new(capacity: usize) -> Self {
        Self {
            data: (0..capacity).map(|_| None).collect(),
            head: 0,
            size: 0,
            capacity,
        }
    }

    fn push(&mut self, item: T) {
        self.data[self.head] = Some(item);
        self.head = (self.head + 1) % self.capacity;
        if self.size < self.capacity {
            self.size += 1;
        }
    }
}

// Implement specific iterator methods for each type
impl CircularBuffer<SessionEvent> {
    fn iter_recent(&self, cutoff_timestamp: u64) -> impl Iterator<Item = &SessionEvent> {
        self.data.iter()
            .filter_map(|opt| opt.as_ref())
            .filter(move |item| item.timestamp >= cutoff_timestamp)
    }
}

impl CircularBuffer<RequestEvent> {
    fn iter_recent(&self, cutoff_timestamp: u64) -> impl Iterator<Item = &RequestEvent> {
        self.data.iter()
            .filter_map(|opt| opt.as_ref())
            .filter(move |item| item.timestamp >= cutoff_timestamp)
    }
}

impl WebDAVMetrics {
    /// Create a new metrics instance
    pub fn new() -> Self {
        Self {
            sessions_total: Arc::new(AtomicU64::new(0)),
            sessions_successful: Arc::new(AtomicU64::new(0)),
            sessions_failed: Arc::new(AtomicU64::new(0)),
            files_processed: Arc::new(AtomicU64::new(0)),
            bytes_processed: Arc::new(AtomicU64::new(0)),
            http_requests_total: Arc::new(AtomicU64::new(0)),
            http_requests_successful: Arc::new(AtomicU64::new(0)),
            
            // Buffers sized for 1 hour of data at reasonable rates
            recent_sessions: Arc::new(RwLock::new(CircularBuffer::new(1000))),
            recent_requests: Arc::new(RwLock::new(CircularBuffer::new(10000))),
            
            cached_calculations: Arc::new(RwLock::new(CachedCalculations {
                success_rate: 0.0,
                avg_session_duration_sec: 0.0,
                avg_processing_rate: 0.0,
                request_success_rate: 0.0,
                avg_request_duration_ms: 0.0,
                sessions_last_hour: 0,
                error_rate_last_hour: 0.0,
            })),
            cache_timestamp: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Record a completed WebDAV sync session
    pub async fn record_session(&self, success: bool, duration_ms: u64, files_count: u32, bytes_count: u64) {
        let timestamp = current_timestamp();
        
        // Update atomic counters
        self.sessions_total.fetch_add(1, Ordering::Relaxed);
        if success {
            self.sessions_successful.fetch_add(1, Ordering::Relaxed);
        } else {
            self.sessions_failed.fetch_add(1, Ordering::Relaxed);
        }
        
        self.files_processed.fetch_add(files_count as u64, Ordering::Relaxed);
        self.bytes_processed.fetch_add(bytes_count, Ordering::Relaxed);
        
        // Add to time-windowed data
        let mut recent = self.recent_sessions.write().await;
        recent.push(SessionEvent {
            timestamp,
            success,
            duration_ms,
            files_count,
            bytes_count,
        });
        
        // Invalidate cache
        self.cache_timestamp.store(0, Ordering::Relaxed);
    }

    /// Record a WebDAV HTTP request
    pub async fn record_request(&self, success: bool, duration_ms: u64) {
        let timestamp = current_timestamp();
        
        // Update atomic counters
        self.http_requests_total.fetch_add(1, Ordering::Relaxed);
        if success {
            self.http_requests_successful.fetch_add(1, Ordering::Relaxed);
        }
        
        // Add to time-windowed data
        let mut recent = self.recent_requests.write().await;
        recent.push(RequestEvent {
            timestamp,
            success,
            duration_ms,
        });
        
        // Invalidate cache
        self.cache_timestamp.store(0, Ordering::Relaxed);
    }

    /// Get all metrics for Prometheus export
    pub async fn get_prometheus_metrics(&self) -> PrometheusMetrics {
        const CACHE_DURATION_SECS: u64 = 30; // Cache for 30 seconds
        
        let now = current_timestamp();
        let last_cache = self.cache_timestamp.load(Ordering::Relaxed);
        
        // Use cache if still valid
        if now - last_cache < CACHE_DURATION_SECS {
            let cached = self.cached_calculations.read().await;
            return self.build_prometheus_metrics(&cached).await;
        }
        
        // Recalculate metrics
        let calculations = self.calculate_derived_metrics(now).await;
        
        // Update cache
        {
            let mut cached = self.cached_calculations.write().await;
            *cached = calculations.clone();
        }
        self.cache_timestamp.store(now, Ordering::Relaxed);
        
        self.build_prometheus_metrics(&calculations).await
    }

    /// Calculate all derived metrics that require time-window analysis
    async fn calculate_derived_metrics(&self, now: u64) -> CachedCalculations {
        let one_hour_ago = now.saturating_sub(3600);
        
        // Get recent data
        let sessions = self.recent_sessions.read().await;
        let requests = self.recent_requests.read().await;
        
        // Calculate session metrics
        let recent_session_events: Vec<&SessionEvent> = sessions.iter_recent(one_hour_ago).collect();
        
        let total_sessions = self.sessions_total.load(Ordering::Relaxed);
        let successful_sessions = self.sessions_successful.load(Ordering::Relaxed);
        
        let success_rate = if total_sessions > 0 {
            (successful_sessions as f64 / total_sessions as f64) * 100.0
        } else {
            0.0
        };

        let (avg_session_duration_sec, avg_processing_rate) = if !recent_session_events.is_empty() {
            let total_duration: u64 = recent_session_events.iter().map(|e| e.duration_ms).sum();
            let total_files: u32 = recent_session_events.iter().map(|e| e.files_count).sum();
            
            let avg_duration = total_duration as f64 / recent_session_events.len() as f64 / 1000.0;
            let avg_rate = if total_duration > 0 {
                total_files as f64 / (total_duration as f64 / 1000.0)
            } else {
                0.0
            };
            
            (avg_duration, avg_rate)
        } else {
            (0.0, 0.0)
        };

        // Calculate request metrics
        let recent_request_events: Vec<&RequestEvent> = requests.iter_recent(one_hour_ago).collect();
        
        let total_requests = self.http_requests_total.load(Ordering::Relaxed);
        let successful_requests = self.http_requests_successful.load(Ordering::Relaxed);
        
        let request_success_rate = if total_requests > 0 {
            (successful_requests as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };

        let avg_request_duration_ms = if !recent_request_events.is_empty() {
            let total_duration: u64 = recent_request_events.iter().map(|e| e.duration_ms).sum();
            total_duration as f64 / recent_request_events.len() as f64
        } else {
            0.0
        };

        // Last hour metrics
        let sessions_last_hour = recent_session_events.len() as u64;
        let failed_sessions_last_hour = recent_session_events.iter()
            .filter(|e| !e.success)
            .count() as u64;
        
        let error_rate_last_hour = if sessions_last_hour > 0 {
            (failed_sessions_last_hour as f64 / sessions_last_hour as f64) * 100.0
        } else {
            0.0
        };

        CachedCalculations {
            success_rate,
            avg_session_duration_sec,
            avg_processing_rate,
            request_success_rate,
            avg_request_duration_ms,
            sessions_last_hour,
            error_rate_last_hour,
        }
    }

    /// Build the final Prometheus metrics structure
    async fn build_prometheus_metrics(&self, calculations: &CachedCalculations) -> PrometheusMetrics {
        PrometheusMetrics {
            total_sessions: self.sessions_total.load(Ordering::Relaxed),
            successful_sessions: self.sessions_successful.load(Ordering::Relaxed),
            failed_sessions: self.sessions_failed.load(Ordering::Relaxed),
            success_rate: calculations.success_rate,
            total_files_processed: self.files_processed.load(Ordering::Relaxed),
            total_bytes_processed: self.bytes_processed.load(Ordering::Relaxed),
            avg_session_duration_sec: calculations.avg_session_duration_sec,
            avg_processing_rate: calculations.avg_processing_rate,
            total_http_requests: self.http_requests_total.load(Ordering::Relaxed),
            request_success_rate: calculations.request_success_rate,
            avg_request_duration_ms: calculations.avg_request_duration_ms,
            sessions_last_hour: calculations.sessions_last_hour,
            error_rate_last_hour: calculations.error_rate_last_hour,
        }
    }

    /// Get simple session counter for basic tracking
    pub fn get_total_sessions(&self) -> u64 {
        self.sessions_total.load(Ordering::Relaxed)
    }
}


fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

impl Default for WebDAVMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_basic_session_recording() {
        let metrics = WebDAVMetrics::new();
        
        // Record successful session
        metrics.record_session(true, 5000, 10, 1024000).await;
        
        // Record failed session
        metrics.record_session(false, 2000, 0, 0).await;
        
        let prometheus_metrics = metrics.get_prometheus_metrics().await;
        
        assert_eq!(prometheus_metrics.total_sessions, 2);
        assert_eq!(prometheus_metrics.successful_sessions, 1);
        assert_eq!(prometheus_metrics.failed_sessions, 1);
        assert_eq!(prometheus_metrics.success_rate, 50.0);
        assert_eq!(prometheus_metrics.total_files_processed, 10);
        assert_eq!(prometheus_metrics.total_bytes_processed, 1024000);
    }

    #[tokio::test]
    async fn test_request_recording() {
        let metrics = WebDAVMetrics::new();
        
        // Record successful requests
        metrics.record_request(true, 100).await;
        metrics.record_request(true, 200).await;
        
        // Record failed request
        metrics.record_request(false, 5000).await;
        
        let prometheus_metrics = metrics.get_prometheus_metrics().await;
        
        assert_eq!(prometheus_metrics.total_http_requests, 3);
        assert!((prometheus_metrics.request_success_rate - 66.67).abs() < 0.1);
        assert!((prometheus_metrics.avg_request_duration_ms - 1766.67).abs() < 0.1);
    }

    #[tokio::test]
    async fn test_caching() {
        let metrics = WebDAVMetrics::new();
        
        metrics.record_session(true, 1000, 5, 512000).await;
        
        // First call should calculate
        let start = std::time::Instant::now();
        let metrics1 = metrics.get_prometheus_metrics().await;
        let first_duration = start.elapsed();
        
        // Second call should use cache
        let start = std::time::Instant::now();
        let metrics2 = metrics.get_prometheus_metrics().await;
        let second_duration = start.elapsed();
        
        // Results should be identical (from cache)
        assert_eq!(metrics1.total_sessions, metrics2.total_sessions);
        assert_eq!(metrics1.success_rate, metrics2.success_rate);
        
        // Second call should be faster (cached)
        assert!(second_duration < first_duration);
    }

    #[tokio::test]
    async fn test_circular_buffer() {
        let mut buffer = CircularBuffer::new(3);
        
        buffer.push(SessionEvent {
            timestamp: 100,
            success: true,
            duration_ms: 1000,
            files_count: 1,
            bytes_count: 100,
        });
        
        buffer.push(SessionEvent {
            timestamp: 200,
            success: false,
            duration_ms: 2000,
            files_count: 0,
            bytes_count: 0,
        });
        
        // Should have 2 items
        let recent: Vec<_> = buffer.iter_recent(50).collect();
        assert_eq!(recent.len(), 2);
        
        // Add more items than capacity
        buffer.push(SessionEvent {
            timestamp: 300,
            success: true,
            duration_ms: 3000,
            files_count: 2,
            bytes_count: 200,
        });
        
        buffer.push(SessionEvent {
            timestamp: 400,
            success: true,
            duration_ms: 4000,
            files_count: 3,
            bytes_count: 300,
        });
        
        // Should still have only 3 items (capacity limit)
        let recent: Vec<_> = buffer.iter_recent(50).collect();
        assert_eq!(recent.len(), 3);
        
        // Should not include the first item (timestamp 100) as it was overwritten
        assert!(recent.iter().all(|e| e.timestamp >= 200));
    }
}