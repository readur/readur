use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn};
use rand::Rng;

use super::xml_extractor::{OfficeExtractionResult, XmlOfficeExtractor};

/// Configuration for fallback strategy behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackConfig {
    /// Enable fallback mechanism
    pub enabled: bool,
    /// Maximum number of retry attempts for transient failures
    pub max_retries: u32,
    /// Initial retry delay in milliseconds
    pub initial_retry_delay_ms: u64,
    /// Maximum retry delay in milliseconds
    pub max_retry_delay_ms: u64,
    /// Circuit breaker configuration
    pub circuit_breaker: CircuitBreakerConfig,
    /// Learning mechanism configuration
    pub learning: LearningConfig,
    /// Timeout configuration for individual methods
    pub method_timeouts: MethodTimeouts,
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Enable circuit breaker
    pub enabled: bool,
    /// Number of consecutive failures before opening circuit
    pub failure_threshold: u32,
    /// Time to wait before attempting to close circuit
    pub recovery_timeout_seconds: u64,
    /// Percentage of successful requests needed to close circuit (0-100)
    pub success_threshold_percentage: u32,
}

/// Learning mechanism configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningConfig {
    /// Enable learning from successful extractions
    pub enabled: bool,
    /// Cache successful extraction methods per document type
    pub cache_successful_methods: bool,
    /// Time to keep method preferences in cache (in hours)
    pub cache_ttl_hours: u64,
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cache_successful_methods: true,
            cache_ttl_hours: 24,
        }
    }
}

/// Timeout configuration for different extraction methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodTimeouts {
    /// Timeout for library-based extraction in seconds
    pub library_timeout_seconds: u64,
    /// Timeout for XML-based extraction in seconds
    pub xml_timeout_seconds: u64,
    /// Timeout for OCR-based extraction in seconds
    pub ocr_timeout_seconds: u64,
}

impl Default for MethodTimeouts {
    fn default() -> Self {
        Self {
            library_timeout_seconds: 120,
            xml_timeout_seconds: 180,
            ocr_timeout_seconds: 300,
        }
    }
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_retries: 3,
            initial_retry_delay_ms: 1000,
            max_retry_delay_ms: 30000,
            circuit_breaker: CircuitBreakerConfig {
                enabled: true,
                failure_threshold: 5,
                recovery_timeout_seconds: 60,
                success_threshold_percentage: 50,
            },
            learning: LearningConfig {
                enabled: true,
                cache_successful_methods: true,
                cache_ttl_hours: 24,
            },
            method_timeouts: MethodTimeouts {
                library_timeout_seconds: 120,
                xml_timeout_seconds: 180,
                ocr_timeout_seconds: 300,
            },
        }
    }
}

/// Circuit breaker states
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,   // Normal operation
    Open,     // Failing fast
    HalfOpen, // Testing recovery
}

/// Circuit breaker for a specific extraction method
/// Thread-safe implementation using Arc<Mutex> for shared state
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    inner: Arc<std::sync::Mutex<CircuitBreakerInner>>,
}

#[derive(Debug)]
struct CircuitBreakerInner {
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure_time: Option<Instant>,
    config: CircuitBreakerConfig,
}

impl CircuitBreaker {
    fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            inner: Arc::new(Mutex::new(CircuitBreakerInner {
                state: CircuitState::Closed,
                failure_count: 0,
                success_count: 0,
                last_failure_time: None,
                config,
            })),
        }
    }

    /// Check if the circuit should allow a request
    fn should_allow_request(&self) -> bool {
        let mut inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("Circuit breaker mutex was poisoned, recovering");
                poisoned.into_inner()
            }
        };
        
        match inner.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if we should transition to half-open
                if let Some(last_failure) = inner.last_failure_time {
                    if last_failure.elapsed().as_secs() >= inner.config.recovery_timeout_seconds {
                        info!("Circuit breaker transitioning from Open to HalfOpen for recovery test");
                        inner.state = CircuitState::HalfOpen;
                        inner.success_count = 0;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Record a successful operation
    fn record_success(&self) {
        let mut inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("Circuit breaker mutex was poisoned during success recording, recovering");
                poisoned.into_inner()
            }
        };
        
        inner.success_count += 1;
        
        match inner.state {
            CircuitState::Closed => {
                // Reset failure count on success
                inner.failure_count = 0;
            }
            CircuitState::HalfOpen => {
                // Check if we should close the circuit
                let total_requests = inner.success_count + inner.failure_count;
                if total_requests >= 10 { // Minimum sample size
                    let success_percentage = (inner.success_count * 100) / total_requests;
                    if success_percentage >= inner.config.success_threshold_percentage {
                        info!("Circuit breaker closing after successful recovery ({}% success rate)", success_percentage);
                        inner.state = CircuitState::Closed;
                        inner.failure_count = 0;
                        inner.success_count = 0;
                    }
                }
            }
            CircuitState::Open => {
                // Should not happen, but reset if it does
                warn!("Unexpected success recorded while circuit is Open");
            }
        }
    }

    /// Record a failed operation
    fn record_failure(&self) {
        let mut inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("Circuit breaker mutex was poisoned during failure recording, recovering");
                poisoned.into_inner()
            }
        };
        
        inner.failure_count += 1;
        inner.last_failure_time = Some(Instant::now());
        
        match inner.state {
            CircuitState::Closed => {
                if inner.failure_count >= inner.config.failure_threshold {
                    warn!("Circuit breaker opening after {} consecutive failures", inner.failure_count);
                    inner.state = CircuitState::Open;
                }
            }
            CircuitState::HalfOpen => {
                warn!("Circuit breaker opening again after failure during recovery test");
                inner.state = CircuitState::Open;
                inner.success_count = 0;
            }
            CircuitState::Open => {
                // Already open, nothing to do
            }
        }
    }
}

/// Cached method preference for a specific document type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodPreference {
    pub method_name: String,
    pub success_count: u32,
    pub last_success_time: u64, // Unix timestamp
    pub average_processing_time_ms: u64,
    pub confidence_score: f32,
}

/// Learning cache for method preferences
#[derive(Debug, Clone)]
pub struct LearningCache {
    preferences: Arc<RwLock<HashMap<String, MethodPreference>>>,
    config: LearningConfig,
}

impl LearningCache {
    fn new(config: LearningConfig) -> Self {
        Self {
            preferences: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Get preferred method for a document type
    fn get_preferred_method(&self, document_type: &str) -> Option<String> {
        if !self.config.cache_successful_methods {
            return None;
        }

        let preferences = match self.preferences.read() {
            Ok(p) => p,
            Err(poisoned) => {
                warn!("Learning cache get_preferred_method: mutex was poisoned, attempting recovery");
                poisoned.into_inner()
            }
        };
        let preference = preferences.get(document_type)?;
        
        // Check if preference is still valid (not expired)
        let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_secs(),
            Err(_) => {
                warn!("Learning cache: failed to get current time, using cached preference anyway");
                return Some(preference.method_name.clone());
            }
        };
        let expire_time = preference.last_success_time + (self.config.cache_ttl_hours * 3600);
        
        if now <= expire_time {
            Some(preference.method_name.clone())
        } else {
            None
        }
    }

    /// Record successful method usage
    fn record_success(&self, document_type: &str, method_name: &str, processing_time_ms: u64, confidence: f32) {
        if !self.config.cache_successful_methods {
            return;
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut preferences = match self.preferences.write() {
            Ok(p) => p,
            Err(poisoned) => {
                warn!("Learning cache record_success: mutex was poisoned, attempting recovery");
                poisoned.into_inner()
            }
        };

        let preference = preferences.entry(document_type.to_string()).or_insert_with(|| MethodPreference {
            method_name: method_name.to_string(),
            success_count: 0,
            last_success_time: now,
            average_processing_time_ms: processing_time_ms,
            confidence_score: confidence,
        });

        // Update statistics
        preference.success_count += 1;
        preference.last_success_time = now;
        
        // Update rolling average for processing time
        let weight = 0.2; // Give recent results 20% weight
        preference.average_processing_time_ms = 
            ((1.0 - weight) * preference.average_processing_time_ms as f64 + 
             weight * processing_time_ms as f64) as u64;
        
        // Update rolling average for confidence
        preference.confidence_score = 
            (1.0 - weight as f32) * preference.confidence_score + 
            weight as f32 * confidence;
        
        // If this method is performing better, update the preference
        if method_name != preference.method_name {
            // Switch to new method if it's significantly better
            let time_improvement = preference.average_processing_time_ms as f64 / processing_time_ms as f64;
            let confidence_improvement = confidence / preference.confidence_score;
            
            if time_improvement > 1.2 || confidence_improvement > 1.1 {
                debug!("Switching preferred method for {} from {} to {} (time improvement: {:.2}x, confidence improvement: {:.2}x)",
                    document_type, preference.method_name, method_name, time_improvement, confidence_improvement);
                preference.method_name = method_name.to_string();
            }
        }
    }

    /// Clean up expired entries
    /// This method is thread-safe and handles poisoned mutexes gracefully
    fn cleanup_expired(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        
        match self.preferences.write() {
            Ok(mut preferences) => {
                let expire_threshold = now.saturating_sub(self.config.cache_ttl_hours * 3600);
                let initial_count = preferences.len();
                preferences.retain(|_, pref| pref.last_success_time > expire_threshold);
                let final_count = preferences.len();
                
                if initial_count != final_count {
                    debug!("Learning cache cleanup: removed {} expired entries ({}->{})", 
                        initial_count - final_count, initial_count, final_count);
                }
            }
            Err(poisoned) => {
                warn!("Learning cache cleanup: mutex was poisoned, attempting recovery");
                // In case of poisoned mutex, try to recover and clean up
                let mut preferences = poisoned.into_inner();
                let expire_threshold = now.saturating_sub(self.config.cache_ttl_hours * 3600);
                let initial_count = preferences.len();
                preferences.retain(|_, pref| pref.last_success_time > expire_threshold);
                let final_count = preferences.len();
                
                if initial_count != final_count {
                    debug!("Learning cache cleanup (recovered): removed {} expired entries ({}->{})", 
                        initial_count - final_count, initial_count, final_count);
                }
            }
        }
    }
}

/// Statistics for monitoring fallback performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackStats {
    pub total_extractions: u64,
    pub library_successes: u64,
    pub xml_successes: u64,
    pub fallback_used: u64,
    pub circuit_breaker_trips: u64,
    pub retry_attempts: u64,
    pub average_processing_time_ms: f64,
    pub success_rate_percentage: f64,
}

impl Default for FallbackStats {
    fn default() -> Self {
        Self {
            total_extractions: 0,
            library_successes: 0,
            xml_successes: 0,
            fallback_used: 0,
            circuit_breaker_trips: 0,
            retry_attempts: 0,
            average_processing_time_ms: 0.0,
            success_rate_percentage: 100.0,
        }
    }
}

/// Main fallback strategy implementation
pub struct FallbackStrategy {
    config: FallbackConfig,
    xml_extractor: XmlOfficeExtractor,
    circuit_breakers: Arc<RwLock<HashMap<String, CircuitBreaker>>>,
    learning_cache: LearningCache,
    stats: Arc<RwLock<FallbackStats>>,
}

impl FallbackStrategy {
    /// Create a new fallback strategy
    pub fn new(config: FallbackConfig, temp_dir: String) -> Self {
        Self {
            config: config.clone(),
            xml_extractor: XmlOfficeExtractor::new(temp_dir),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            learning_cache: LearningCache::new(config.learning),
            stats: Arc::new(RwLock::new(FallbackStats::default())),
        }
    }

    /// Execute extraction with intelligent fallback strategy
    pub async fn extract_with_fallback(
        &self,
        file_path: &str,
        mime_type: &str,
    ) -> Result<OfficeExtractionResult> {
        let start_time = Instant::now();
        let document_type = self.get_document_type(mime_type);
        
        info!("Starting extraction with fallback for {} (type: {})", file_path, document_type);
        
        // Update total extraction count
        match self.stats.write() {
            Ok(mut stats) => {
                stats.total_extractions += 1;
            }
            Err(_) => {
                warn!("Failed to acquire write lock on stats for extraction count update");
            }
        }

        // Use XML extraction as the primary method
        let result = self.execute_xml_extraction(file_path, mime_type).await;

        let processing_time = start_time.elapsed();
        
        // Update statistics  
        self.update_stats(&result, processing_time).await;
        
        // Clean up expired cache entries periodically (1% chance per extraction)
        // This is done asynchronously to avoid blocking the main extraction flow
        if rand::thread_rng().gen_range(0..100) == 0 {
            let cache_clone = self.learning_cache.clone();
            tokio::spawn(async move {
                cache_clone.cleanup_expired();
            });
        }

        result
    }

    /// Execute XML extraction directly 
    async fn execute_xml_extraction(
        &self,
        file_path: &str,
        mime_type: &str,
    ) -> Result<OfficeExtractionResult> {
        let result = self.xml_extractor.extract_text_from_office(file_path, mime_type).await?;
        
        // Update stats
        match self.stats.write() {
            Ok(mut stats) => {
                stats.xml_successes += 1;
            }
            Err(_) => {
                warn!("Failed to acquire write lock on stats for xml success update");
            }
        }
        
        Ok(result)
    }

    /// Record a failure for circuit breaker tracking
    async fn record_failure(&self, method_name: &str) {
        if !self.config.circuit_breaker.enabled {
            return;
        }

        match self.circuit_breakers.write() {
            Ok(mut breakers) => {
                let breaker = breakers.entry(method_name.to_string())
                    .or_insert_with(|| CircuitBreaker::new(self.config.circuit_breaker.clone()));
                breaker.record_failure();
                
                // Check if circuit is now open and update stats
                if let Ok(inner) = breaker.inner.lock() {
                    if inner.state == CircuitState::Open {
                        match self.stats.write() {
                            Ok(mut stats) => {
                                stats.circuit_breaker_trips += 1;
                            }
                            Err(_) => {
                                warn!("Failed to acquire write lock on stats for circuit breaker trip recording");
                            }
                        }
                    }
                } else {
                    warn!("Failed to check circuit breaker state after failure recording");
                }
            }
            Err(_) => {
                warn!("Failed to acquire write lock on circuit breakers for failure recording");
            }
        }
    }

    /// Get document type from MIME type
    fn get_document_type(&self, mime_type: &str) -> String {
        match mime_type {
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => "docx".to_string(),
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => "xlsx".to_string(),
            "application/vnd.openxmlformats-officedocument.presentationml.presentation" => "pptx".to_string(),
            "application/msword" => "doc".to_string(),
            "application/vnd.ms-excel" => "xls".to_string(),
            "application/vnd.ms-powerpoint" => "ppt".to_string(),
            "application/pdf" => "pdf".to_string(),
            _ => "unknown".to_string(),
        }
    }

    /// Update statistics after extraction
    async fn update_stats(&self, result: &Result<OfficeExtractionResult>, processing_time: Duration) {
        match self.stats.write() {
            Ok(mut stats) => {
                let processing_time_ms = processing_time.as_millis() as f64;
                
                // Update average processing time using exponential moving average
                let alpha = 0.1; // Smoothing factor
                stats.average_processing_time_ms = 
                    alpha * processing_time_ms + (1.0 - alpha) * stats.average_processing_time_ms;
                
                // Update success rate with proper division by zero protection
                let total_attempts = stats.total_extractions;
                let successful_attempts = stats.library_successes + stats.xml_successes;
                
                if total_attempts > 0 {
                    stats.success_rate_percentage = (successful_attempts as f64 / total_attempts as f64) * 100.0;
                } else {
                    // Keep existing success rate if no attempts yet, or set to 100% for first success
                    if result.is_ok() {
                        stats.success_rate_percentage = 100.0;
                    }
                }
            }
            Err(_) => {
                warn!("Failed to acquire write lock on stats for update");
            }
        }
    }

    /// Get current statistics
    pub async fn get_stats(&self) -> FallbackStats {
        match self.stats.read() {
            Ok(stats) => stats.clone(),
            Err(_) => {
                warn!("Failed to acquire read lock on stats, returning default");
                FallbackStats::default()
            }
        }
    }

    /// Reset statistics
    pub async fn reset_stats(&self) {
        match self.stats.write() {
            Ok(mut stats) => {
                *stats = FallbackStats::default();
            }
            Err(_) => {
                warn!("Failed to acquire write lock on stats for reset");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_strategy() -> (FallbackStrategy, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = FallbackConfig::default();
        let strategy = FallbackStrategy::new(config, temp_dir.path().to_string_lossy().to_string());
        (strategy, temp_dir)
    }

    #[test]
    fn test_circuit_breaker() {
        let config = CircuitBreakerConfig {
            enabled: true,
            failure_threshold: 3,
            recovery_timeout_seconds: 1,
            success_threshold_percentage: 50,
        };
        
        let breaker = CircuitBreaker::new(config);
        
        // Initially closed
        assert!(breaker.should_allow_request());
        
        // Record failures
        breaker.record_failure();
        breaker.record_failure();
        assert!(breaker.should_allow_request()); // Still closed after 2 failures
        
        breaker.record_failure(); // Should open circuit
        assert!(!breaker.should_allow_request()); // Now should be open
    }

    #[test]
    fn test_learning_cache() {
        let config = LearningConfig {
            enabled: true,
            cache_successful_methods: true,
            cache_ttl_hours: 1,
        };
        
        let cache = LearningCache::new(config);
        
        // Initially no preference
        assert!(cache.get_preferred_method("docx").is_none());
        
        // Record success
        cache.record_success("docx", "XML", 1000, 95.0);
        
        // Should have preference now
        assert_eq!(cache.get_preferred_method("docx"), Some("XML".to_string()));
    }

    #[tokio::test]
    async fn test_is_retryable_error() {
        let (strategy, _temp_dir) = create_test_strategy();
        
        // Test retryable errors
        let retryable_errors = [
            "Connection timeout occurred",
            "Network temporarily unavailable",
            "Resource busy, try again",
            "Service unavailable (503)",
            "Rate limit exceeded (429)",
            "Out of memory - allocation failed",
        ];
        
        for error_msg in retryable_errors {
            let error = anyhow!("{}", error_msg);
            assert!(strategy.is_retryable_error(&error), "Expected '{}' to be retryable", error_msg);
        }
        
        // Test non-retryable errors
        let non_retryable_errors = [
            "File is corrupted",
            "Invalid format detected",
            "Access denied - permission error",
            "File not found (404)",
            "Unauthorized access (403)",
            "Assertion failed in parser",
        ];
        
        for error_msg in non_retryable_errors {
            let error = anyhow!("{}", error_msg);
            assert!(!strategy.is_retryable_error(&error), "Expected '{}' to be non-retryable", error_msg);
        }
        
        // Test unknown errors (should be non-retryable by default)
        let unknown_error = anyhow!("Some unknown error occurred");
        assert!(!strategy.is_retryable_error(&unknown_error));
    }

    #[tokio::test] 
    async fn test_stats_tracking() {
        let (strategy, _temp_dir) = create_test_strategy();
        
        let initial_stats = strategy.get_stats().await;
        assert_eq!(initial_stats.total_extractions, 0);
        
        // Simulate some operations by updating stats directly
        match strategy.stats.write() {
            Ok(mut stats) => {
                stats.total_extractions = 10;
                stats.library_successes = 7;
                stats.xml_successes = 2;
            }
            Err(_) => {
                panic!("Failed to acquire write lock on stats in test");
            }
        }
        
        let updated_stats = strategy.get_stats().await;
        assert_eq!(updated_stats.total_extractions, 10);
        assert_eq!(updated_stats.success_rate_percentage, 90.0); // 9 successes out of 10
    }
}