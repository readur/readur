use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use anyhow::{Result, anyhow};
use tracing::{debug, warn, error, info};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use tokio::sync::{Mutex, RwLock};
use chrono::{DateTime, Utc};
use tokio::time::timeout;

/// Configuration for loop detection behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopDetectionConfig {
    /// Enable loop detection (default: true)
    pub enabled: bool,
    
    /// Maximum number of times a directory can be accessed within the time window (default: 3)
    pub max_access_count: usize,
    
    /// Time window for tracking directory accesses in seconds (default: 300 = 5 minutes)
    pub time_window_secs: u64,
    
    /// Maximum time a directory scan can take before being flagged as stuck (default: 60 seconds)
    pub max_scan_duration_secs: u64,
    
    /// Minimum time between directory scans to avoid immediate re-scan loops (default: 5 seconds)
    pub min_scan_interval_secs: u64,
    
    /// Maximum depth for circular pattern detection (default: 10)
    pub max_pattern_depth: usize,
    
    /// Maximum number of directories to track simultaneously (default: 1000, reduced from 10000)
    pub max_tracked_directories: usize,
    
    /// Enable pattern analysis for A->B->A type cycles (default: true)
    pub enable_pattern_analysis: bool,
    
    /// Log level for loop detection events (default: "warn")
    pub log_level: String,
    
    /// Circuit breaker failure threshold before auto-disabling detection (default: 5)
    pub circuit_breaker_failure_threshold: u32,
    
    /// Circuit breaker timeout before re-enabling detection in seconds (default: 300)
    pub circuit_breaker_timeout_secs: u64,
    
    /// Enable graceful degradation when detection fails (default: true)
    pub enable_graceful_degradation: bool,
    
    /// Maximum time to wait for mutex acquisition in milliseconds (default: 100)
    pub mutex_timeout_ms: u64,
}

impl Default for LoopDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_access_count: 3,
            time_window_secs: 300,
            max_scan_duration_secs: 60,
            min_scan_interval_secs: 5,
            max_pattern_depth: 10,
            max_tracked_directories: 1000, // Reduced from 10000 for better memory management
            enable_pattern_analysis: true,
            log_level: "warn".to_string(),
            circuit_breaker_failure_threshold: 5,
            circuit_breaker_timeout_secs: 300,
            enable_graceful_degradation: true,
            mutex_timeout_ms: 100,
        }
    }
}

/// Represents a directory access event
#[derive(Debug, Clone)]
struct DirectoryAccess {
    /// Path of the directory being accessed
    path: String,
    /// When the access started
    started_at: DateTime<Utc>,
    /// When the access completed (None if still in progress)
    completed_at: Option<DateTime<Utc>>,
    /// Unique ID for this access
    access_id: Uuid,
    /// Operation type (scan, discovery, etc.)
    operation: String,
    /// Whether this access resulted in an error
    error: Option<String>,
    /// Number of files found during this access
    files_found: Option<usize>,
    /// Number of subdirectories found during this access
    subdirs_found: Option<usize>,
}

/// Loop detection findings
#[derive(Debug, Clone, Serialize)]
pub struct LoopDetectionResult {
    /// Whether a loop was detected
    pub loop_detected: bool,
    /// Type of loop detected
    pub loop_type: Option<LoopType>,
    /// Problematic directory path
    pub problem_path: Option<String>,
    /// Detailed description of the issue
    pub description: String,
    /// Access pattern that led to detection
    pub access_pattern: Vec<String>,
    /// Metrics about the detected issue
    pub metrics: LoopMetrics,
    /// Recommendations for resolving the issue
    pub recommendations: Vec<String>,
    /// Timestamp when the loop was detected
    pub detected_at: DateTime<Utc>,
    /// Suggested actions for the user
    pub suggested_actions: Vec<String>,
    /// Whether this is a critical issue that requires immediate attention
    pub is_critical: bool,
}


/// Types of loops that can be detected
#[derive(Debug, Clone, Serialize)]
pub enum LoopType {
    /// Same directory accessed too frequently
    FrequentReAccess,
    /// Directory scan is taking too long
    StuckScan,
    /// Immediate re-scan of the same directory
    ImmediateReScan,
    /// Circular pattern detected (A->B->A or A->B->C->A)
    CircularPattern,
    /// Too many concurrent accesses to the same directory
    ConcurrentAccess,
}

/// Metrics about loop detection
#[derive(Debug, Clone, Serialize)]
pub struct LoopMetrics {
    /// Number of accesses to the problematic path
    pub access_count: usize,
    /// Time span of the problematic accesses
    pub time_span_secs: f64,
    /// Average scan duration
    pub avg_scan_duration_secs: f64,
    /// Total files found across all accesses
    pub total_files_found: usize,
    /// Total subdirectories found across all accesses
    pub total_subdirs_found: usize,
    /// Number of failed accesses
    pub failed_accesses: usize,
}

/// Circuit breaker state for graceful degradation
#[derive(Debug, Clone)]
pub struct CircuitBreakerState {
    pub failures: u32,
    pub last_failure_time: Option<DateTime<Utc>>,
    pub is_open: bool,
}

/// Performance metrics for loop detection instrumentation
#[derive(Debug, Clone, Serialize)]
pub struct InstrumentationMetrics {
    pub total_operations: u64,
    pub avg_operation_duration_ms: f64,
    pub max_operation_duration_ms: f64,
    pub memory_usage_bytes: usize,
    pub cache_hit_rate: f64,
}

/// Internal state for tracking directory accesses
#[derive(Debug)]
struct LoopDetectionState {
    /// Active directory accesses (path -> access info)
    active_accesses: HashMap<String, DirectoryAccess>,
    /// Historical accesses within the time window
    access_history: VecDeque<DirectoryAccess>,
    /// Pattern tracking for circular detection - now with bounded size
    access_patterns: HashMap<String, VecDeque<String>>,
    /// Last access time for each directory
    last_access_times: HashMap<String, DateTime<Utc>>,
    /// Performance metrics
    total_accesses: u64,
    total_loops_detected: u64,
    /// Configuration reference
    config: LoopDetectionConfig,
    /// Circuit breaker for graceful degradation
    circuit_breaker: CircuitBreakerState,
    /// Instrumentation metrics
    instrumentation_metrics: InstrumentationMetrics,
    /// Last cleanup timestamp
    last_cleanup: DateTime<Utc>,
}

/// Main loop detection service
#[derive(Debug, Clone)]
pub struct LoopDetectionService {
    state: Arc<Mutex<LoopDetectionState>>,
    /// Read-write lock for configuration updates
    config: Arc<RwLock<LoopDetectionConfig>>,
}

impl LoopDetectionService {
    /// Create a new loop detection service with default configuration
    pub fn new() -> Self {
        Self::with_config(LoopDetectionConfig::default())
    }
    
    /// Create a new loop detection service with custom configuration
    pub fn with_config(config: LoopDetectionConfig) -> Self {
        let now = Utc::now();
        let state = LoopDetectionState {
            active_accesses: HashMap::new(),
            access_history: VecDeque::new(),
            access_patterns: HashMap::new(),
            last_access_times: HashMap::new(),
            total_accesses: 0,
            total_loops_detected: 0,
            config: config.clone(),
            circuit_breaker: CircuitBreakerState {
                failures: 0,
                last_failure_time: None,
                is_open: false,
            },
            instrumentation_metrics: InstrumentationMetrics {
                total_operations: 0,
                avg_operation_duration_ms: 0.0,
                max_operation_duration_ms: 0.0,
                memory_usage_bytes: 0,
                cache_hit_rate: 0.0,
            },
            last_cleanup: now,
        };
        
        Self {
            state: Arc::new(Mutex::new(state)),
            config: Arc::new(RwLock::new(config)),
        }
    }
    
    /// Start tracking a directory access
    pub async fn start_access(&self, path: &str, operation: &str) -> Result<Uuid> {
        let config = self.config.read().await;
        if !config.enabled {
            return Ok(Uuid::new_v4()); // Return dummy ID when disabled
        }
        drop(config);
        
        let operation_start = Utc::now();
        
        // Use timeout to prevent deadlocks
        let mut state = match timeout(
            Duration::from_millis(self.get_mutex_timeout().await),
            self.state.lock()
        ).await {
            Ok(state) => state,
            Err(_) => {
                warn!("Loop detection mutex timeout for path '{}' - enabling graceful degradation", path);
                if self.should_use_graceful_degradation().await {
                    return Ok(Uuid::new_v4()); // Return dummy ID and continue
                } else {
                    return Err(anyhow!("Loop detection service unavailable: mutex timeout"));
                }
            }
        };
        
        // Check circuit breaker
        if state.circuit_breaker.is_open {
            if let Some(last_failure) = state.circuit_breaker.last_failure_time {
                let config = self.config.read().await;
                let timeout_duration = chrono::Duration::seconds(config.circuit_breaker_timeout_secs as i64);
                if Utc::now().signed_duration_since(last_failure) > timeout_duration {
                    // Reset circuit breaker
                    state.circuit_breaker.is_open = false;
                    state.circuit_breaker.failures = 0;
                    info!("Loop detection circuit breaker reset for path '{}'"  , path);
                } else {
                    debug!("Loop detection circuit breaker is open for path '{}' - skipping detection", path);
                    return Ok(Uuid::new_v4()); // Return dummy ID when circuit breaker is open
                }
            }
        }
        
        let access_id = Uuid::new_v4();
        let now = Utc::now();
        
        // Update instrumentation metrics
        state.instrumentation_metrics.total_operations += 1;
        
        // Periodic cleanup to prevent memory leaks
        if now.signed_duration_since(state.last_cleanup).num_seconds() > 60 {
            self.cleanup_state_internal(&mut state, now).await;
            state.last_cleanup = now;
        }
        
        // Check for immediate re-scan
        if let Some(last_access) = state.last_access_times.get(path) {
            let time_since_last = now.signed_duration_since(*last_access);
            if time_since_last.num_seconds() < state.config.min_scan_interval_secs as i64 {
                let result = LoopDetectionResult {
                    loop_detected: true,
                    loop_type: Some(LoopType::ImmediateReScan),
                    problem_path: Some(path.to_string()),
                    description: format!(
                        "Directory '{}' re-accessed after only {:.2}s (minimum interval: {}s)",
                        path, time_since_last.num_seconds(), state.config.min_scan_interval_secs
                    ),
                    access_pattern: vec![path.to_string()],
                    metrics: self.calculate_metrics(&state, path),
                    recommendations: vec![
                        "Review your sync schedule - scanning this directory too frequently".to_string(),
                        "Check if multiple sync processes are running simultaneously".to_string(),
                        "Consider increasing the minimum scan interval in settings".to_string(),
                    ],
                    suggested_actions: vec![
                        "Wait at least 5 seconds before rescanning the same directory".to_string(),
                        "Check your WebDAV sync configuration for conflicts".to_string(),
                    ],
                    detected_at: now,
                    is_critical: false,
                };
                self.log_loop_detection(&result);
                state.total_loops_detected += 1;
                
                // Track operation duration
                let operation_duration = Utc::now().signed_duration_since(operation_start).num_milliseconds() as f64;
                self.update_instrumentation_metrics(&mut state, operation_duration);
                
                return Err(anyhow!("Loop detected: {}. {}", result.description, result.suggested_actions.join(". ")));
            }
        }
        
        // Check for concurrent access to the same directory
        if state.active_accesses.contains_key(path) {
            let result = LoopDetectionResult {
                loop_detected: true,
                loop_type: Some(LoopType::ConcurrentAccess),
                problem_path: Some(path.to_string()),
                description: format!("Multiple simultaneous scans detected for directory '{}'", path),
                access_pattern: vec![path.to_string()],
                metrics: self.calculate_metrics(&state, path),
                recommendations: vec![
                    "This indicates a synchronization issue in your application".to_string(),
                    "Multiple sync processes may be running simultaneously".to_string(),
                    "Check your scheduling configuration".to_string(),
                ],
                suggested_actions: vec![
                    "Stop any other running sync operations".to_string(),
                    "Review your sync schedule to prevent overlaps".to_string(),
                    "Contact support if this continues to occur".to_string(),
                ],
                detected_at: now,
                is_critical: true,
            };
            self.log_loop_detection(&result);
            state.total_loops_detected += 1;
            
            // Track operation duration
            let operation_duration = Utc::now().signed_duration_since(operation_start).num_milliseconds() as f64;
            self.update_instrumentation_metrics(&mut state, operation_duration);
            
            return Err(anyhow!("Critical sync issue detected: {}. {}", result.description, result.suggested_actions.join(". ")));
        }
        
        // Check access frequency
        if let Some(loop_result) = self.check_access_frequency(&state, path, now) {
            state.total_loops_detected += 1;
            self.log_loop_detection(&loop_result);
            
            // Track operation duration
            let operation_duration = Utc::now().signed_duration_since(operation_start).num_milliseconds() as f64;
            self.update_instrumentation_metrics(&mut state, operation_duration);
            
            return Err(anyhow!("Sync loop detected: {}. {}", loop_result.description, loop_result.suggested_actions.join(". ")));
        }
        
        // Check circular patterns
        if state.config.enable_pattern_analysis {
            if let Some(loop_result) = self.check_circular_patterns(&state, path, now) {
                state.total_loops_detected += 1;
                self.log_loop_detection(&loop_result);
                
                // Track operation duration
                let operation_duration = Utc::now().signed_duration_since(operation_start).num_milliseconds() as f64;
                self.update_instrumentation_metrics(&mut state, operation_duration);
                
                return Err(anyhow!("Circular sync pattern detected: {}. {}", loop_result.description, loop_result.suggested_actions.join(". ")));
            }
        }
        
        // Record the access
        let access = DirectoryAccess {
            path: path.to_string(),
            started_at: now,
            completed_at: None,
            access_id,
            operation: operation.to_string(),
            error: None,
            files_found: None,
            subdirs_found: None,
        };
        
        state.active_accesses.insert(path.to_string(), access);
        state.last_access_times.insert(path.to_string(), now);
        state.total_accesses += 1;
        
        // Clean up old history to prevent memory growth
        self.cleanup_old_history(&mut state, now);
        
        // Track operation duration
        let operation_duration = Utc::now().signed_duration_since(operation_start).num_milliseconds() as f64;
        self.update_instrumentation_metrics(&mut state, operation_duration);
        
        debug!("[{}] Started tracking access to '{}' with operation '{}'", access_id, path, operation);
        Ok(access_id)
    }
    
    /// Complete tracking a directory access
    pub async fn complete_access(
        &self, 
        access_id: Uuid, 
        files_found: Option<usize>, 
        subdirs_found: Option<usize>,
        error: Option<String>
    ) -> Result<()> {
        let config = self.config.read().await;
        if !config.enabled {
            return Ok(());
        }
        drop(config);
        
        let operation_start = Utc::now();
        
        // Use timeout to prevent deadlocks
        let mut state = match timeout(
            Duration::from_millis(self.get_mutex_timeout().await),
            self.state.lock()
        ).await {
            Ok(state) => state,
            Err(_) => {
                warn!("Loop detection mutex timeout for access_id '{}' - enabling graceful degradation", access_id);
                if self.should_use_graceful_degradation().await {
                    return Ok(()); // Silently continue when graceful degradation is enabled
                } else {
                    return Err(anyhow!("Loop detection service unavailable: mutex timeout"));
                }
            }
        };
        
        let now = Utc::now();
        
        // Handle circuit breaker failures
        let operation_result: Result<()> = (|| {
            // Find the access and collect information
            let mut access_info = None;
            let _max_scan_duration_secs = state.config.max_scan_duration_secs;
            let _enable_pattern_analysis = state.config.enable_pattern_analysis;
            let _max_pattern_depth = state.config.max_pattern_depth;
            
            // First pass: find and update the access
            for (path, access) in state.active_accesses.iter_mut() {
                if access.access_id == access_id {
                    access.completed_at = Some(now);
                    access.files_found = files_found;
                    access.subdirs_found = subdirs_found;
                    access.error = error.clone();
                    
                    let duration = now.signed_duration_since(access.started_at);
                    
                    // Collect info for later processing
                    access_info = Some((
                        path.clone(),
                        duration,
                        access.clone(),
                    ));
                    
                    debug!("[{}] Completed access to '{}' in {:.2}s, found {} files, {} subdirs", 
                           access_id, path, duration.num_milliseconds() as f64 / 1000.0, 
                           files_found.unwrap_or(0), subdirs_found.unwrap_or(0));
                    break;
                }
            }
            Ok(())
        })();
        
        // Handle circuit breaker on operation failure
        if operation_result.is_err() {
            state.circuit_breaker.failures += 1;
            state.circuit_breaker.last_failure_time = Some(now);
            
            if state.circuit_breaker.failures >= state.config.circuit_breaker_failure_threshold {
                state.circuit_breaker.is_open = true;
                warn!("Loop detection circuit breaker opened due to {} failures", state.circuit_breaker.failures);
            }
        } else {
            // Reset failure count on successful operation
            if state.circuit_breaker.failures > 0 {
                state.circuit_breaker.failures = 0;
            }
        }
        
        // Find the access and update it if found
        let mut access_info = None;
        let max_scan_duration_secs = state.config.max_scan_duration_secs;
        let enable_pattern_analysis = state.config.enable_pattern_analysis;
        let max_pattern_depth = state.config.max_pattern_depth;
        
        // Look for the access to complete
        for (path, access) in state.active_accesses.iter_mut() {
            if access.access_id == access_id {
                access.completed_at = Some(now);
                access.files_found = files_found;
                access.subdirs_found = subdirs_found;
                access.error = error.clone();
                
                let duration = now.signed_duration_since(access.started_at);
                access_info = Some((path.clone(), duration, access.clone()));
                
                debug!("[{}] Completed access to '{}' in {:.2}s, found {} files, {} subdirs", 
                       access_id, path, duration.num_milliseconds() as f64 / 1000.0, 
                       files_found.unwrap_or(0), subdirs_found.unwrap_or(0));
                break;
            }
        }
        
        // Process the completed access
        if let Some((path, duration, access)) = access_info {
            // Check if this access took too long
            if duration.num_seconds() > max_scan_duration_secs as i64 {
                let result = LoopDetectionResult {
                    loop_detected: true,
                    loop_type: Some(LoopType::StuckScan),
                    problem_path: Some(path.clone()),
                    description: format!(
                        "Directory scan is taking too long: '{}' has been scanning for {:.1}s (limit: {}s)",
                        path, duration.num_milliseconds() as f64 / 1000.0, max_scan_duration_secs
                    ),
                    access_pattern: vec![path.clone()],
                    metrics: self.calculate_metrics(&state, &path),
                    recommendations: vec![
                        "This directory may contain too many files or have connectivity issues".to_string(),
                        "Check your network connection to the WebDAV server".to_string(),
                        "Consider excluding large directories from sync if they're not needed".to_string(),
                    ],
                    suggested_actions: vec![
                        "Wait for the current scan to complete or cancel it".to_string(),
                        "Check if the directory contains an unusually large number of files".to_string(),
                        "Consider increasing timeout settings if this directory is expected to be large".to_string(),
                    ],
                    detected_at: now,
                    is_critical: false,
                };
                self.log_loop_detection(&result);
                state.total_loops_detected += 1;
            }
            
            // Move from active to history
            state.active_accesses.remove(&path);
            state.access_history.push_back(access.clone());
            
            // Update pattern tracking with better memory management
            if enable_pattern_analysis && state.access_patterns.len() < state.config.max_tracked_directories {
                let pattern = state.access_patterns.entry(path.clone())
                    .or_insert_with(VecDeque::new);
                pattern.push_back(path);
                if pattern.len() > max_pattern_depth {
                    pattern.pop_front();
                }
            }
        }
        
        // Track operation duration
        let operation_duration = Utc::now().signed_duration_since(operation_start).num_milliseconds() as f64;
        self.update_instrumentation_metrics(&mut state, operation_duration);
        
        operation_result
    }
    
    /// Get current loop detection metrics
    pub async fn get_metrics(&self) -> Result<serde_json::Value> {
        let config = self.config.read().await;
        if !config.enabled {
            return Ok(serde_json::json!({
                "enabled": false,
                "message": "Loop detection is disabled"
            }));
        }
        drop(config);
        
        // Use timeout to prevent deadlocks
        let state = match timeout(
            Duration::from_millis(self.get_mutex_timeout().await),
            self.state.lock()
        ).await {
            Ok(state) => state,
            Err(_) => {
                return Ok(serde_json::json!({
                    "enabled": true,
                    "error": "Service temporarily unavailable",
                    "message": "Metrics cannot be retrieved due to high load"
                }));
            }
        };
        
        Ok(serde_json::json!({
            "enabled": true,
            "total_accesses": state.total_accesses,
            "total_loops_detected": state.total_loops_detected,
            "active_accesses": state.active_accesses.len(),
            "history_size": state.access_history.len(),
            "tracked_patterns": state.access_patterns.len(),
            "circuit_breaker": {
                "is_open": state.circuit_breaker.is_open,
                "failures": state.circuit_breaker.failures,
                "last_failure": state.circuit_breaker.last_failure_time
            },
            "instrumentation": state.instrumentation_metrics,
            "memory_usage_estimated_bytes": self.estimate_memory_usage(&state)
        }))
    }
    
    /// Check if loop detection is enabled
    pub async fn is_enabled(&self) -> bool {
        let config = self.config.read().await;
        config.enabled
    }
    
    /// Update configuration
    pub async fn update_config(&self, new_config: LoopDetectionConfig) -> Result<()> {
        let mut config = self.config.write().await;
        *config = new_config.clone();
        
        // Also update the config in state for backward compatibility
        let state_result = timeout(
            Duration::from_millis(100), // Short timeout for config updates
            self.state.lock()
        ).await;
        
        if let Ok(mut state) = state_result {
            state.config = new_config;
            info!("Loop detection configuration updated successfully");
        } else {
            warn!("Could not update state config due to lock contention, but main config is updated");
        }
        
        Ok(())
    }
    
    /// Clear all tracking data (useful for testing)
    pub async fn clear_state(&self) -> Result<()> {
        let mut state = match timeout(
            Duration::from_millis(self.get_mutex_timeout().await),
            self.state.lock()
        ).await {
            Ok(state) => state,
            Err(_) => return Err(anyhow!("Could not clear state: service unavailable")),
        };
        
        state.active_accesses.clear();
        state.access_history.clear();
        state.access_patterns.clear();
        state.last_access_times.clear();
        
        // Reset circuit breaker
        state.circuit_breaker = CircuitBreakerState {
            failures: 0,
            last_failure_time: None,
            is_open: false,
        };
        
        // Reset instrumentation metrics
        state.instrumentation_metrics = InstrumentationMetrics {
            total_operations: 0,
            avg_operation_duration_ms: 0.0,
            max_operation_duration_ms: 0.0,
            memory_usage_bytes: 0,
            cache_hit_rate: 0.0,
        };
        
        debug!("Loop detection state cleared");
        Ok(())
    }
    
    // New helper methods for enhanced functionality
    
    /// Get mutex timeout from configuration
    async fn get_mutex_timeout(&self) -> u64 {
        let config = self.config.read().await;
        config.mutex_timeout_ms
    }
    
    /// Check if graceful degradation should be used
    async fn should_use_graceful_degradation(&self) -> bool {
        let config = self.config.read().await;
        config.enable_graceful_degradation
    }
    
    /// Update instrumentation metrics
    fn update_instrumentation_metrics(&self, state: &mut LoopDetectionState, operation_duration_ms: f64) {
        // Update instrumentation metrics
        let memory_usage = self.estimate_memory_usage(state);
        
        let metrics = &mut state.instrumentation_metrics;
        
        // Update average operation duration
        let total_ops = metrics.total_operations as f64;
        if total_ops > 0.0 {
            metrics.avg_operation_duration_ms = 
                (metrics.avg_operation_duration_ms * total_ops + operation_duration_ms) / (total_ops + 1.0);
        } else {
            metrics.avg_operation_duration_ms = operation_duration_ms;
        }
        
        // Update max operation duration
        if operation_duration_ms > metrics.max_operation_duration_ms {
            metrics.max_operation_duration_ms = operation_duration_ms;
        }
        
        // Update memory usage estimate
        metrics.memory_usage_bytes = memory_usage;
    }
    
    /// Estimate memory usage of the current state
    fn estimate_memory_usage(&self, state: &LoopDetectionState) -> usize {
        let mut size = std::mem::size_of::<LoopDetectionState>();
        
        // Estimate HashMap and VecDeque sizes
        size += state.active_accesses.len() * (std::mem::size_of::<String>() + std::mem::size_of::<DirectoryAccess>());
        size += state.access_history.len() * std::mem::size_of::<DirectoryAccess>();
        
        for (key, pattern) in &state.access_patterns {
            size += key.len();
            size += pattern.len() * std::mem::size_of::<String>();
            for path in pattern {
                size += path.len();
            }
        }
        
        size += state.last_access_times.len() * (std::mem::size_of::<String>() + std::mem::size_of::<DateTime<Utc>>());
        
        size
    }
    
    /// Enhanced cleanup that also manages memory bounds
    async fn cleanup_state_internal(&self, state: &mut LoopDetectionState, now: DateTime<Utc>) {
        // Clean up old history
        let time_window = chrono::Duration::seconds(state.config.time_window_secs as i64);
        let cutoff_time = now - time_window;
        
        // Remove old access history
        while let Some(access) = state.access_history.front() {
            if access.started_at < cutoff_time {
                state.access_history.pop_front();
            } else {
                break;
            }
        }
        
        // Aggressively clean up pattern tracking when approaching memory limits
        if state.access_patterns.len() > state.config.max_tracked_directories {
            let excess = state.access_patterns.len() - state.config.max_tracked_directories;
            
            // Remove patterns that haven't been accessed recently
            let mut patterns_to_remove = Vec::new();
            for (path, _pattern) in &state.access_patterns {
                if let Some(last_access) = state.last_access_times.get(path) {
                    if now.signed_duration_since(*last_access) > time_window {
                        patterns_to_remove.push(path.clone());
                        if patterns_to_remove.len() >= excess {
                            break;
                        }
                    }
                }
            }
            
            // If we still have too many patterns, remove the oldest ones
            if patterns_to_remove.len() < excess {
                let mut remaining_patterns: Vec<_> = state.access_patterns.keys().cloned().collect();
                remaining_patterns.sort_by(|a, b| {
                    let time_a = state.last_access_times.get(a).unwrap_or(&cutoff_time);
                    let time_b = state.last_access_times.get(b).unwrap_or(&cutoff_time);
                    time_a.cmp(time_b)
                });
                
                for path in remaining_patterns.into_iter().take(excess - patterns_to_remove.len()) {
                    patterns_to_remove.push(path);
                }
            }
            
            for path in patterns_to_remove {
                state.access_patterns.remove(&path);
            }
        }
        
        // Clean up last access times
        let paths_to_remove: Vec<String> = state.last_access_times
            .iter()
            .filter(|(_, &time)| time < cutoff_time)
            .map(|(path, _)| path.clone())
            .collect();
        
        for path in paths_to_remove {
            state.last_access_times.remove(&path);
        }
        
        debug!("Cleanup completed: {} active, {} history, {} patterns, {} last_access", 
               state.active_accesses.len(), 
               state.access_history.len(), 
               state.access_patterns.len(), 
               state.last_access_times.len());
    }
    
    // Private helper methods
    
    fn check_access_frequency(
        &self, 
        state: &LoopDetectionState, 
        path: &str, 
        now: DateTime<Utc>
    ) -> Option<LoopDetectionResult> {
        let time_window = chrono::Duration::seconds(state.config.time_window_secs as i64);
        let cutoff_time = now - time_window;
        
        let recent_accesses: Vec<_> = state.access_history
            .iter()
            .filter(|access| {
                access.path == path && 
                access.started_at >= cutoff_time &&
                access.completed_at.is_some()
            })
            .collect();
        
        if recent_accesses.len() >= state.config.max_access_count {
            let first_access_time = recent_accesses.first().unwrap().started_at;
            let time_span = now.signed_duration_since(first_access_time);
            
            return Some(LoopDetectionResult {
                loop_detected: true,
                loop_type: Some(LoopType::FrequentReAccess),
                problem_path: Some(path.to_string()),
                description: format!(
                    "Directory '{}' has been scanned {} times in the last {:.1} minutes (limit: {} scans per {} minutes)",
                    path, recent_accesses.len(), time_span.num_minutes() as f64,
                    state.config.max_access_count, state.config.time_window_secs / 60
                ),
                access_pattern: recent_accesses.iter().map(|a| a.path.clone()).collect(),
                metrics: self.calculate_metrics_from_accesses(&recent_accesses),
                recommendations: vec![
                    "This directory is being scanned too frequently".to_string(),
                    "Check if multiple sync processes are running".to_string(),
                    "Review your sync schedule settings".to_string(),
                ],
                suggested_actions: vec![
                    "Reduce sync frequency for this directory".to_string(),
                    "Check for duplicate sync configurations".to_string(),
                    "Consider excluding this directory if it changes infrequently".to_string(),
                ],
                detected_at: now,
                is_critical: false,
            });
        }
        
        None
    }
    
    fn check_circular_patterns(
        &self, 
        state: &LoopDetectionState, 
        path: &str, 
        now: DateTime<Utc>
    ) -> Option<LoopDetectionResult> {
        if let Some(pattern) = state.access_patterns.get(path) {
            // Look for simple A->A patterns
            if pattern.len() >= 2 && pattern.back() == Some(&path.to_string()) {
                if let Some(second_last) = pattern.get(pattern.len() - 2) {
                    if second_last == path {
                        return Some(LoopDetectionResult {
                            loop_detected: true,
                            loop_type: Some(LoopType::CircularPattern),
                            problem_path: Some(path.to_string()),
                            description: format!("Circular directory access pattern detected for '{}'", path),
                            access_pattern: pattern.iter().cloned().collect(),
                            metrics: self.calculate_metrics(state, path),
                            recommendations: vec![
                                "This indicates a potential infinite loop in directory scanning".to_string(),
                                "Check if the directory structure has circular references".to_string(),
                                "Verify that symbolic links are handled correctly".to_string(),
                            ],
                            suggested_actions: vec![
                                "Stop the current sync operation".to_string(),
                                "Check for symbolic links that might create loops".to_string(),
                                "Contact support if this directory should not have circular references".to_string(),
                            ],
                            detected_at: now,
                            is_critical: true,
                        });
                    }
                }
            }
            
            // Look for longer patterns like A->B->A or A->B->C->A
            if pattern.len() >= 3 {
                let pattern_vec: Vec<String> = pattern.iter().cloned().collect();
                for i in 0..pattern_vec.len().saturating_sub(2) {
                    if pattern_vec[i] == path {
                        for j in (i + 2)..pattern_vec.len() {
                            if pattern_vec[j] == path {
                                let cycle: Vec<String> = pattern_vec[i..=j].to_vec();
                                return Some(LoopDetectionResult {
                                    loop_detected: true,
                                    loop_type: Some(LoopType::CircularPattern),
                                    problem_path: Some(path.to_string()),
                                    description: format!(
                                        "Complex circular pattern detected: {} (involves {} directories)",
                                        cycle.join(" → "), cycle.len()
                                    ),
                                    access_pattern: cycle.clone(),
                                    metrics: self.calculate_metrics(state, path),
                                    recommendations: vec![
                                        "Multiple directories are creating a circular reference".to_string(),
                                        "This may indicate an issue with directory structure or symbolic links".to_string(),
                                        "Review the directory hierarchy for unexpected links".to_string(),
                                    ],
                                    suggested_actions: vec![
                                        "Stop the sync and examine the directory structure".to_string(),
                                        format!("Check these directories for circular links: {}", cycle.join(", ")),
                                        "Consider excluding problematic directories from sync".to_string(),
                                    ],
                                    detected_at: now,
                                    is_critical: true,
                                });
                            }
                        }
                    }
                }
            }
        }
        
        None
    }
    
    fn calculate_metrics(&self, state: &LoopDetectionState, path: &str) -> LoopMetrics {
        let accesses: Vec<_> = state.access_history
            .iter()
            .filter(|access| access.path == path)
            .collect();
        
        self.calculate_metrics_from_accesses(&accesses)
    }
    
    fn calculate_metrics_from_accesses(&self, accesses: &[&DirectoryAccess]) -> LoopMetrics {
        if accesses.is_empty() {
            return LoopMetrics {
                access_count: 0,
                time_span_secs: 0.0,
                avg_scan_duration_secs: 0.0,
                total_files_found: 0,
                total_subdirs_found: 0,
                failed_accesses: 0,
            };
        }
        
        let first_time = accesses.first().unwrap().started_at;
        let last_time = accesses.last().unwrap().started_at;
        let time_span = last_time.signed_duration_since(first_time);
        
        let total_duration_ms: i64 = accesses
            .iter()
            .filter_map(|access| {
                access.completed_at.map(|end| end.signed_duration_since(access.started_at).num_milliseconds())
            })
            .sum();
        
        let completed_count = accesses.iter().filter(|a| a.completed_at.is_some()).count();
        let avg_duration = if completed_count > 0 {
            total_duration_ms as f64 / 1000.0 / completed_count as f64
        } else {
            0.0
        };
        
        LoopMetrics {
            access_count: accesses.len(),
            time_span_secs: time_span.num_milliseconds() as f64 / 1000.0,
            avg_scan_duration_secs: avg_duration,
            total_files_found: accesses.iter().filter_map(|a| a.files_found).sum(),
            total_subdirs_found: accesses.iter().filter_map(|a| a.subdirs_found).sum(),
            failed_accesses: accesses.iter().filter(|a| a.error.is_some()).count(),
        }
    }
    
    fn cleanup_old_history(&self, state: &mut LoopDetectionState, now: DateTime<Utc>) {
        let time_window = chrono::Duration::seconds(state.config.time_window_secs as i64);
        let cutoff_time = now - time_window;
        
        // Remove old access history
        while let Some(access) = state.access_history.front() {
            if access.started_at < cutoff_time {
                state.access_history.pop_front();
            } else {
                break;
            }
        }
        
        // Clean up pattern tracking if we're tracking too many directories
        if state.access_patterns.len() > state.config.max_tracked_directories {
            let excess = state.access_patterns.len() - state.config.max_tracked_directories;
            let paths_to_remove: Vec<String> = state.access_patterns
                .keys()
                .take(excess)
                .cloned()
                .collect();
            
            for path in paths_to_remove {
                state.access_patterns.remove(&path);
            }
        }
        
        // Clean up last access times
        let paths_to_remove: Vec<String> = state.last_access_times
            .iter()
            .filter(|(_, &time)| time < cutoff_time)
            .map(|(path, _)| path.clone())
            .collect();
        
        for path in paths_to_remove {
            state.last_access_times.remove(&path);
        }
    }
    
    fn log_loop_detection(&self, result: &LoopDetectionResult) {
        let log_level = "warn"; // Default to warn level for production safety
        
        let severity_prefix = if result.is_critical { "🚨 CRITICAL" } else { "⚠️  WARNING" };
        
        let message = format!(
            "{} - Sync Loop Detected\n│ Type: {:?}\n│ Directory: '{}'\n│ Issue: {}\n│ Pattern: {}\n│ Action needed: {}",
            severity_prefix,
            result.loop_type.as_ref().unwrap_or(&LoopType::FrequentReAccess),
            result.problem_path.as_ref().unwrap_or(&"unknown".to_string()),
            result.description,
            result.access_pattern.join(" → "),
            result.suggested_actions.first().unwrap_or(&"Review sync configuration".to_string())
        );
        
        if result.is_critical {
            error!("{}", message);
        } else {
            match log_level {
                "error" => error!("{}", message),
                "warn" => warn!("{}", message),
                "info" => info!("{}", message),
                "debug" => debug!("{}", message),
                _ => warn!("{}", message),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    
    #[tokio::test]
    async fn test_immediate_rescan_detection() {
        let service = LoopDetectionService::new();
        
        // First access should succeed
        let access1 = service.start_access("/test", "scan").await.unwrap();
        service.complete_access(access1, Some(5), Some(2), None).await.unwrap();
        
        // Immediate second access should fail
        let result = service.start_access("/test", "scan").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("re-accessed after only"));
    }
    
    #[tokio::test]
    async fn test_concurrent_access_detection() {
        let service = LoopDetectionService::new();
        
        // Start first access
        let _access1 = service.start_access("/test", "scan").await.unwrap();
        
        // Second concurrent access should fail
        let result = service.start_access("/test", "scan").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("simultaneous scans detected"));
    }
    
    #[tokio::test]
    async fn test_frequency_detection() {
        let mut config = LoopDetectionConfig::default();
        config.max_access_count = 2;
        config.min_scan_interval_secs = 0; // Disable immediate re-scan check
        let service = LoopDetectionService::with_config(config);
        
        // Do multiple accesses that complete quickly
        for i in 0..3 {
            let access = service.start_access("/test", "scan").await.unwrap();
            service.complete_access(access, Some(i), Some(1), None).await.unwrap();
            tokio::time::sleep(Duration::from_millis(100)).await; // Small delay
        }
        
        // Next access should trigger frequency detection
        let result = service.start_access("/test", "scan").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("scanned 2 times"));
    }
    
    #[tokio::test]
    async fn test_metrics_collection() {
        let service = LoopDetectionService::new();
        
        let access = service.start_access("/test", "scan").await.unwrap();
        service.complete_access(access, Some(10), Some(3), None).await.unwrap();
        
        let metrics = service.get_metrics().await.unwrap();
        assert_eq!(metrics["total_accesses"], 1);
        assert_eq!(metrics["active_accesses"], 0);
        assert!(metrics["enabled"].as_bool().unwrap());
    }
    
    #[tokio::test]
    async fn test_disabled_service() {
        let mut config = LoopDetectionConfig::default();
        config.enabled = false;
        let service = LoopDetectionService::with_config(config);
        
        // Should not detect any loops when disabled
        let access1 = service.start_access("/test", "scan").await.unwrap();
        service.complete_access(access1, Some(5), Some(2), None).await.unwrap();
        
        let access2 = service.start_access("/test", "scan").await.unwrap();
        service.complete_access(access2, Some(5), Some(2), None).await.unwrap();
        
        let metrics = service.get_metrics().await.unwrap();
        assert!(!metrics["enabled"].as_bool().unwrap());
    }
    
    #[tokio::test]
    async fn test_circuit_breaker() {
        let mut config = LoopDetectionConfig::default();
        config.circuit_breaker_failure_threshold = 2;
        let service = LoopDetectionService::with_config(config);
        
        // Simulate circuit breaker by triggering concurrent access errors
        let _access1 = service.start_access("/test", "scan").await.unwrap();
        
        // Should fail with concurrent access
        let _result1 = service.start_access("/test", "scan").await;
        let _result2 = service.start_access("/test", "scan").await;
        
        let metrics = service.get_metrics().await.unwrap();
        assert!(metrics["circuit_breaker"]["failures"].as_u64().unwrap() > 0);
    }
    
    #[tokio::test]
    async fn test_graceful_degradation() {
        let mut config = LoopDetectionConfig::default();
        config.enable_graceful_degradation = true;
        config.mutex_timeout_ms = 1; // Very short timeout to trigger degradation
        let service = LoopDetectionService::with_config(config);
        
        // This should not panic even with very short timeout
        let result = service.start_access("/test", "scan").await;
        assert!(result.is_ok()); // Should succeed with graceful degradation
    }
}

/// Separate configuration module to decouple from WebDAV config
pub mod config {
    use super::LoopDetectionConfig;
    
    /// Create a loop detection config optimized for production use
    pub fn production_config() -> LoopDetectionConfig {
        LoopDetectionConfig {
            enabled: true,
            max_access_count: 3,
            time_window_secs: 300, // 5 minutes
            max_scan_duration_secs: 120, // 2 minutes for large directories
            min_scan_interval_secs: 10, // Longer interval for production
            max_pattern_depth: 5, // Reduced depth for better performance
            max_tracked_directories: 500, // Conservative limit
            enable_pattern_analysis: true,
            log_level: "warn".to_string(),
            circuit_breaker_failure_threshold: 3,
            circuit_breaker_timeout_secs: 300, // 5 minutes
            enable_graceful_degradation: true,
            mutex_timeout_ms: 200, // 200ms timeout
        }
    }
    
    /// Create a loop detection config optimized for development/testing
    pub fn development_config() -> LoopDetectionConfig {
        LoopDetectionConfig {
            enabled: true,
            max_access_count: 5, // More lenient for dev
            time_window_secs: 180, // 3 minutes
            max_scan_duration_secs: 60,
            min_scan_interval_secs: 2, // Shorter for faster development
            max_pattern_depth: 10,
            max_tracked_directories: 100,
            enable_pattern_analysis: true,
            log_level: "debug".to_string(),
            circuit_breaker_failure_threshold: 5,
            circuit_breaker_timeout_secs: 60,
            enable_graceful_degradation: true,
            mutex_timeout_ms: 500, // Longer timeout for debugging
        }
    }
    
    /// Create a minimal config that disables most detection for performance
    pub fn minimal_config() -> LoopDetectionConfig {
        LoopDetectionConfig {
            enabled: true,
            max_access_count: 10, // Very lenient
            time_window_secs: 600, // 10 minutes
            max_scan_duration_secs: 300, // 5 minutes
            min_scan_interval_secs: 1,
            max_pattern_depth: 3,
            max_tracked_directories: 50,
            enable_pattern_analysis: false, // Disabled for performance
            log_level: "error".to_string(), // Only log errors
            circuit_breaker_failure_threshold: 10,
            circuit_breaker_timeout_secs: 600,
            enable_graceful_degradation: true,
            mutex_timeout_ms: 50, // Very short timeout
        }
    }
}