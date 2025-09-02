use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, timeout};
use tracing::{debug, error, info, warn};
use rand::Rng;

use super::extraction_comparator::{ExtractionConfig, ExtractionMode, SingleExtractionResult};
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
        extraction_config: &ExtractionConfig,
    ) -> Result<SingleExtractionResult> {
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

        let result = match extraction_config.mode {
            ExtractionMode::LibraryFirst => {
                self.execute_library_first_strategy(file_path, mime_type, &document_type, extraction_config).await
            }
            ExtractionMode::XmlFirst => {
                self.execute_xml_first_strategy(file_path, mime_type, &document_type, extraction_config).await
            }
            ExtractionMode::CompareAlways => {
                self.execute_compare_always_strategy(file_path, mime_type, &document_type, extraction_config).await
            }
            ExtractionMode::LibraryOnly => {
                self.execute_library_only_strategy(file_path, mime_type, &document_type).await
            }
            ExtractionMode::XmlOnly => {
                self.execute_xml_only_strategy(file_path, mime_type, &document_type).await
            }
        };

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

    /// Execute library-first strategy with XML fallback
    async fn execute_library_first_strategy(
        &self,
        file_path: &str,
        mime_type: &str,
        document_type: &str,
        extraction_config: &ExtractionConfig,
    ) -> Result<SingleExtractionResult> {
        // Check if we have a learned preference
        if let Some(preferred_method) = self.learning_cache.get_preferred_method(document_type) {
            debug!("Using learned preference: {} for document type: {}", preferred_method, document_type);
            
            if preferred_method.contains("XML") {
                // Try XML first based on learning
                match self.try_xml_extraction(file_path, mime_type).await {
                    Ok(result) => {
                        self.learning_cache.record_success(document_type, &result.method_name, result.processing_time.as_millis() as u64, result.confidence);
                        return Ok(result);
                    }
                    Err(e) => {
                        debug!("Learned preference failed, falling back to library: {}", e);
                    }
                }
            }
        }

        // Try library extraction first
        match self.try_library_extraction(file_path, mime_type).await {
            Ok(result) => {
                match self.stats.write() {
                    Ok(mut stats) => {
                        stats.library_successes += 1;
                    }
                    Err(_) => {
                        warn!("Failed to acquire write lock on stats for library success update");
                    }
                }
                self.learning_cache.record_success(document_type, &result.method_name, result.processing_time.as_millis() as u64, result.confidence);
                Ok(result)
            }
            Err(library_error) => {
                warn!("Library extraction failed, attempting XML fallback: {}", library_error);
                
                match self.stats.write() {
                    Ok(mut stats) => {
                        stats.fallback_used += 1;
                    }
                    Err(_) => {
                        warn!("Failed to acquire write lock on stats for fallback count update");
                    }
                }

                match self.try_xml_extraction(file_path, mime_type).await {
                    Ok(result) => {
                        match self.stats.write() {
                            Ok(mut stats) => {
                                stats.xml_successes += 1;
                            }
                            Err(_) => {
                                warn!("Failed to acquire write lock on stats for xml success update");
                            }
                        }
                        self.learning_cache.record_success(document_type, &result.method_name, result.processing_time.as_millis() as u64, result.confidence);
                        Ok(result)
                    }
                    Err(xml_error) => {
                        error!("Both library and XML extraction failed. Library error: {}. XML error: {}", library_error, xml_error);
                        Err(anyhow!(
                            "All extraction methods failed. Library extraction: {}. XML extraction: {}",
                            library_error, xml_error
                        ))
                    }
                }
            }
        }
    }

    /// Execute XML-first strategy with library fallback
    async fn execute_xml_first_strategy(
        &self,
        file_path: &str,
        mime_type: &str,
        document_type: &str,
        extraction_config: &ExtractionConfig,
    ) -> Result<SingleExtractionResult> {
        // Check if we have a learned preference
        if let Some(preferred_method) = self.learning_cache.get_preferred_method(document_type) {
            debug!("Using learned preference: {} for document type: {}", preferred_method, document_type);
            
            if preferred_method.contains("Library") {
                // Try library first based on learning
                match self.try_library_extraction(file_path, mime_type).await {
                    Ok(result) => {
                        self.learning_cache.record_success(document_type, &result.method_name, result.processing_time.as_millis() as u64, result.confidence);
                        return Ok(result);
                    }
                    Err(e) => {
                        debug!("Learned preference failed, falling back to XML: {}", e);
                    }
                }
            }
        }

        // Try XML extraction first
        match self.try_xml_extraction(file_path, mime_type).await {
            Ok(result) => {
                match self.stats.write() {
                    Ok(mut stats) => {
                        stats.xml_successes += 1;
                    }
                    Err(_) => {
                        warn!("Failed to acquire write lock on stats for xml success update");
                    }
                }
                self.learning_cache.record_success(document_type, &result.method_name, result.processing_time.as_millis() as u64, result.confidence);
                Ok(result)
            }
            Err(xml_error) => {
                warn!("XML extraction failed, attempting library fallback: {}", xml_error);
                
                match self.stats.write() {
                    Ok(mut stats) => {
                        stats.fallback_used += 1;
                    }
                    Err(_) => {
                        warn!("Failed to acquire write lock on stats for fallback count update");
                    }
                }

                match self.try_library_extraction(file_path, mime_type).await {
                    Ok(result) => {
                        match self.stats.write() {
                            Ok(mut stats) => {
                                stats.library_successes += 1;
                            }
                            Err(_) => {
                                warn!("Failed to acquire write lock on stats for library success update");
                            }
                        }
                        self.learning_cache.record_success(document_type, &result.method_name, result.processing_time.as_millis() as u64, result.confidence);
                        Ok(result)
                    }
                    Err(library_error) => {
                        error!("Both XML and library extraction failed. XML error: {}. Library error: {}", xml_error, library_error);
                        Err(anyhow!(
                            "All extraction methods failed. XML extraction: {}. Library extraction: {}",
                            xml_error, library_error
                        ))
                    }
                }
            }
        }
    }

    /// Execute compare-always strategy (runs both methods)
    async fn execute_compare_always_strategy(
        &self,
        file_path: &str,
        mime_type: &str,
        document_type: &str,
        extraction_config: &ExtractionConfig,
    ) -> Result<SingleExtractionResult> {
        let library_result = self.try_library_extraction(file_path, mime_type).await;
        let xml_result = self.try_xml_extraction(file_path, mime_type).await;

        match (library_result, xml_result) {
            (Ok(lib_result), Ok(xml_result)) => {
                // Both succeeded, choose the better one
                match self.stats.write() {
                    Ok(mut stats) => {
                        stats.library_successes += 1;
                        stats.xml_successes += 1;
                    }
                    Err(_) => {
                        warn!("Failed to acquire write lock on stats for dual success update");
                    }
                }

                let chosen_result = if lib_result.word_count >= xml_result.word_count && lib_result.processing_time <= xml_result.processing_time {
                    lib_result
                } else {
                    xml_result
                };

                self.learning_cache.record_success(document_type, &chosen_result.method_name, chosen_result.processing_time.as_millis() as u64, chosen_result.confidence);
                
                info!("Compare-always mode: both methods succeeded, chosen: {}", chosen_result.method_name);
                Ok(chosen_result)
            }
            (Ok(lib_result), Err(_)) => {
                match self.stats.write() {
                    Ok(mut stats) => {
                        stats.library_successes += 1;
                    }
                    Err(_) => {
                        warn!("Failed to acquire write lock on stats for library success update");
                    }
                }
                self.learning_cache.record_success(document_type, &lib_result.method_name, lib_result.processing_time.as_millis() as u64, lib_result.confidence);
                Ok(lib_result)
            }
            (Err(_), Ok(xml_result)) => {
                match self.stats.write() {
                    Ok(mut stats) => {
                        stats.xml_successes += 1;
                    }
                    Err(_) => {
                        warn!("Failed to acquire write lock on stats for xml success update");
                    }
                }
                self.learning_cache.record_success(document_type, &xml_result.method_name, xml_result.processing_time.as_millis() as u64, xml_result.confidence);
                Ok(xml_result)
            }
            (Err(lib_error), Err(xml_error)) => {
                error!("Both extraction methods failed in compare-always mode. Library: {}. XML: {}", lib_error, xml_error);
                Err(anyhow!(
                    "All extraction methods failed. Library: {}. XML: {}",
                    lib_error, xml_error
                ))
            }
        }
    }

    /// Execute library-only strategy
    async fn execute_library_only_strategy(
        &self,
        file_path: &str,
        mime_type: &str,
        document_type: &str,
    ) -> Result<SingleExtractionResult> {
        let result = self.try_library_extraction(file_path, mime_type).await?;
        match self.stats.write() {
            Ok(mut stats) => {
                stats.library_successes += 1;
            }
            Err(_) => {
                warn!("Failed to acquire write lock on stats for library success update");
            }
        }
        self.learning_cache.record_success(document_type, &result.method_name, result.processing_time.as_millis() as u64, result.confidence);
        Ok(result)
    }

    /// Execute XML-only strategy
    async fn execute_xml_only_strategy(
        &self,
        file_path: &str,
        mime_type: &str,
        document_type: &str,
    ) -> Result<SingleExtractionResult> {
        let result = self.try_xml_extraction(file_path, mime_type).await?;
        match self.stats.write() {
            Ok(mut stats) => {
                stats.xml_successes += 1;
            }
            Err(_) => {
                warn!("Failed to acquire write lock on stats for xml success update");
            }
        }
        self.learning_cache.record_success(document_type, &result.method_name, result.processing_time.as_millis() as u64, result.confidence);
        Ok(result)
    }

    /// Try library-based extraction with circuit breaker and retry logic
    async fn try_library_extraction(
        &self,
        file_path: &str,
        mime_type: &str,
    ) -> Result<SingleExtractionResult> {
        let method_name = "Library";
        
        // Check circuit breaker
        if !self.should_allow_request(method_name).await {
            return Err(anyhow!("Circuit breaker is open for library extraction"));
        }

        let result = self.execute_with_retry(
            || self.execute_library_extraction(file_path, mime_type),
            method_name
        ).await;

        // Update circuit breaker
        match &result {
            Ok(_) => self.record_success(method_name).await,
            Err(_) => self.record_failure(method_name).await,
        }

        result
    }

    /// Try XML-based extraction with circuit breaker and retry logic
    async fn try_xml_extraction(
        &self,
        file_path: &str,
        mime_type: &str,
    ) -> Result<SingleExtractionResult> {
        let method_name = "XML";
        
        // Check circuit breaker
        if !self.should_allow_request(method_name).await {
            return Err(anyhow!("Circuit breaker is open for XML extraction"));
        }

        let result = self.execute_with_retry(
            || self.execute_xml_extraction(file_path, mime_type),
            method_name
        ).await;

        // Update circuit breaker
        match &result {
            Ok(_) => self.record_success(method_name).await,
            Err(_) => self.record_failure(method_name).await,
        }

        result
    }

    /// Execute library extraction (placeholder - would integrate with actual library)
    async fn execute_library_extraction(
        &self,
        file_path: &str,
        mime_type: &str,
    ) -> Result<SingleExtractionResult> {
        let start_time = Instant::now();
        
        // Timeout wrapper
        let timeout_duration = Duration::from_secs(self.config.method_timeouts.library_timeout_seconds);
        
        timeout(timeout_duration, async {
            // This is a placeholder - in production this would call the actual library extraction
            // For now, simulate library extraction behavior
            tokio::time::sleep(Duration::from_millis(50)).await; // Simulate processing time
            
            // Simulate failure for certain conditions (for testing purposes)
            if file_path.contains("corrupt") || file_path.contains("unsupported") {
                return Err(anyhow!("Library extraction failed: unsupported document format"));
            }
            
            Ok(SingleExtractionResult {
                text: format!("Library-extracted text from {}", file_path),
                confidence: 85.0,
                processing_time: start_time.elapsed(),
                word_count: 150, // Simulated word count
                method_name: "Library-based extraction".to_string(),
                success: true,
                error_message: None,
            })
        }).await.map_err(|_| anyhow!("Library extraction timed out after {} seconds", self.config.method_timeouts.library_timeout_seconds))?
    }

    /// Execute XML extraction
    async fn execute_xml_extraction(
        &self,
        file_path: &str,
        mime_type: &str,
    ) -> Result<SingleExtractionResult> {
        let start_time = Instant::now();
        
        // Timeout wrapper
        let timeout_duration = Duration::from_secs(self.config.method_timeouts.xml_timeout_seconds);
        
        timeout(timeout_duration, async {
            let result = self.xml_extractor.extract_text_from_office_with_timeout(
                file_path, 
                mime_type, 
                self.config.method_timeouts.xml_timeout_seconds
            ).await?;
            
            Ok(SingleExtractionResult {
                text: result.text,
                confidence: result.confidence,
                processing_time: start_time.elapsed(),
                word_count: result.word_count,
                method_name: format!("XML-based extraction ({})", result.extraction_method),
                success: true,
                error_message: None,
            })
        }).await.map_err(|_| anyhow!("XML extraction timed out after {} seconds", self.config.method_timeouts.xml_timeout_seconds))?
    }

    /// Execute operation with retry logic and exponential backoff
    async fn execute_with_retry<F, Fut>(
        &self,
        operation: F,
        method_name: &str,
    ) -> Result<SingleExtractionResult>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<SingleExtractionResult>>,
    {
        let mut delay_ms = self.config.initial_retry_delay_ms;
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);
                    
                    if attempt < self.config.max_retries && self.is_retryable_error(&last_error.as_ref().unwrap()) {
                        warn!("Attempt {} failed for {}, retrying in {}ms: {}", 
                            attempt + 1, method_name, delay_ms, last_error.as_ref().unwrap());
                        
                        match self.stats.write() {
                            Ok(mut stats) => {
                                stats.retry_attempts += 1;
                            }
                            Err(_) => {
                                warn!("Failed to acquire write lock on stats for retry attempt update");
                            }
                        }
                        
                        sleep(Duration::from_millis(delay_ms)).await;
                        
                        // Exponential backoff with jitter
                        delay_ms = (delay_ms * 2).min(self.config.max_retry_delay_ms);
                        let jitter_range = delay_ms / 4;
                        if jitter_range > 0 {
                            delay_ms += rand::thread_rng().gen_range(0..jitter_range); // Add 0-25% jitter
                        }
                    } else {
                        break;
                    }
                }
            }
        }

        Err(last_error.unwrap())
    }

    /// Check if an error is retryable with improved classification
    /// This method categorizes errors into retryable and non-retryable based on their nature
    fn is_retryable_error(&self, error: &anyhow::Error) -> bool {
        let error_msg = error.to_string().to_lowercase();
        let error_chain = format!("{:?}", error).to_lowercase();
        
        // Definitely retryable errors (transient issues)
        let retryable_patterns = [
            // Network and I/O issues
            "timeout", "timed out", "connection", "network",
            "temporarily unavailable", "resource busy", "busy",
            "would block", "try again", "eagain", "ewouldblock",
            // File system temporary issues
            "no space left", "disk full", "quota exceeded",
            "file locked", "sharing violation",
            // Service temporary issues
            "service unavailable", "server unavailable", "503",
            "rate limit", "throttling", "429", "too many requests",
            // Memory pressure (might be temporary)
            "out of memory", "memory limit", "allocation failed",
        ];
        
        // Definitely non-retryable errors (permanent issues)
        let non_retryable_patterns = [
            // File format/content issues
            "corrupted", "invalid format", "unsupported format",
            "malformed", "parse error", "invalid structure",
            "not found", "404", "file not found", "no such file",
            // Permission issues
            "access denied", "permission denied", "unauthorized", "403",
            "forbidden", "authentication failed",
            // Logical errors in code
            "assertion failed", "panic", "index out of bounds",
            "null pointer", "segmentation fault",
        ];
        
        // Check for non-retryable patterns first (they take precedence)
        for pattern in &non_retryable_patterns {
            if error_msg.contains(pattern) || error_chain.contains(pattern) {
                debug!("Error classified as non-retryable due to pattern '{}': {}", pattern, error_msg);
                return false;
            }
        }
        
        // Check for retryable patterns
        for pattern in &retryable_patterns {
            if error_msg.contains(pattern) || error_chain.contains(pattern) {
                debug!("Error classified as retryable due to pattern '{}': {}", pattern, error_msg);
                return true;
            }
        }
        
        // Check error source chain for more context
        let mut source = error.source();
        while let Some(err) = source {
            let source_msg = err.to_string().to_lowercase();
            
            // Check source errors against patterns
            for pattern in &non_retryable_patterns {
                if source_msg.contains(pattern) {
                    debug!("Error classified as non-retryable due to source pattern '{}': {}", pattern, source_msg);
                    return false;
                }
            }
            
            for pattern in &retryable_patterns {
                if source_msg.contains(pattern) {
                    debug!("Error classified as retryable due to source pattern '{}': {}", pattern, source_msg);
                    return true;
                }
            }
            
            source = err.source();
        }
        
        // Default: unknown errors are not retryable to avoid infinite loops
        debug!("Error classified as non-retryable (default): {}", error_msg);
        false
    }

    /// Check if circuit breaker should allow request
    async fn should_allow_request(&self, method_name: &str) -> bool {
        if !self.config.circuit_breaker.enabled {
            return true;
        }

        match self.circuit_breakers.write() {
            Ok(mut breakers) => {
                let breaker = breakers.entry(method_name.to_string())
                    .or_insert_with(|| CircuitBreaker::new(self.config.circuit_breaker.clone()));
                breaker.should_allow_request()
            }
            Err(_) => {
                warn!("Failed to acquire write lock on circuit breakers, allowing request");
                true
            }
        }
    }

    /// Record successful operation for circuit breaker
    async fn record_success(&self, method_name: &str) {
        if !self.config.circuit_breaker.enabled {
            return;
        }

        match self.circuit_breakers.write() {
            Ok(mut breakers) => {
                let breaker = breakers.entry(method_name.to_string())
                    .or_insert_with(|| CircuitBreaker::new(self.config.circuit_breaker.clone()));
                breaker.record_success();
            }
            Err(_) => {
                warn!("Failed to acquire write lock on circuit breakers for success recording");
            }
        }
    }

    /// Record failed operation for circuit breaker
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
    async fn update_stats(&self, result: &Result<SingleExtractionResult>, processing_time: Duration) {
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