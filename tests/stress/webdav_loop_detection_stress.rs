/*!
 * WebDAV Loop Detection Stress Test
 * 
 * This stress test exercises the actual WebDAV sync functionality with loop detection enabled.
 * It creates scenarios that could cause loops and verifies that they are properly detected
 * and reported by the instrumented sync code.
 */

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::time::sleep;
use anyhow::{Result, Context};
use tracing::{info, warn, error, debug};
use uuid::Uuid;
use serde_json::{json, Value};

use readur::services::webdav::{
    WebDAVService, WebDAVConfig, SmartSyncService, 
    LoopDetectionConfig, LoopType
};
use readur::{AppState, config::Config};

/// Configuration for the stress test
#[derive(Debug, Clone)]
pub struct StressTestConfig {
    /// Duration to run the stress test
    pub duration_secs: u64,
    /// WebDAV server URL for testing
    pub webdav_url: String,
    /// WebDAV username
    pub username: String,
    /// WebDAV password
    pub password: String,
    /// Number of concurrent sync operations
    pub concurrent_syncs: usize,
    /// Directories to test
    pub test_directories: Vec<String>,
    /// Whether to intentionally trigger loops for testing
    pub trigger_test_loops: bool,
    /// Loop detection timeout
    pub loop_detection_timeout_secs: u64,
}

impl Default for StressTestConfig {
    fn default() -> Self {
        Self {
            duration_secs: std::env::var("STRESS_TEST_DURATION")
                .unwrap_or_else(|_| "300".to_string())
                .parse()
                .unwrap_or(300),
            webdav_url: std::env::var("WEBDAV_DUFS_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            username: std::env::var("WEBDAV_USERNAME")
                .unwrap_or_else(|_| "webdav_user".to_string()),
            password: std::env::var("WEBDAV_PASSWORD")
                .unwrap_or_else(|_| "webdav_pass".to_string()),
            concurrent_syncs: std::env::var("CONCURRENT_SYNCS")
                .unwrap_or_else(|_| "4".to_string())
                .parse()
                .unwrap_or(4),
            test_directories: vec![
                "/stress_test".to_string(),
                "/stress_test/nested".to_string(),
                "/stress_test/deep/structure".to_string(),
                "/stress_test/complex".to_string(),
            ],
            trigger_test_loops: std::env::var("TRIGGER_TEST_LOOPS")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            loop_detection_timeout_secs: std::env::var("LOOP_DETECTION_TIMEOUT")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .unwrap_or(60),
        }
    }
}

/// Metrics collected during stress testing
#[derive(Debug, Clone)]
pub struct StressTestMetrics {
    pub total_sync_operations: u64,
    pub successful_syncs: u64,
    pub failed_syncs: u64,
    pub loops_detected: u64,
    pub avg_sync_duration_ms: f64,
    pub max_sync_duration_ms: u64,
    pub min_sync_duration_ms: u64,
    pub files_discovered: u64,
    pub directories_discovered: u64,
    pub errors_by_type: HashMap<String, u64>,
    pub loop_types_detected: HashMap<String, u64>,
}

impl Default for StressTestMetrics {
    fn default() -> Self {
        Self {
            total_sync_operations: 0,
            successful_syncs: 0,
            failed_syncs: 0,
            loops_detected: 0,
            avg_sync_duration_ms: 0.0,
            max_sync_duration_ms: 0,
            min_sync_duration_ms: u64::MAX,
            files_discovered: 0,
            directories_discovered: 0,
            errors_by_type: HashMap::new(),
            loop_types_detected: HashMap::new(),
        }
    }
}

/// Main stress test runner
pub struct WebDAVLoopDetectionStressTest {
    config: StressTestConfig,
    metrics: Arc<tokio::sync::Mutex<StressTestMetrics>>,
}

impl WebDAVLoopDetectionStressTest {
    pub fn new(config: StressTestConfig) -> Self {
        Self {
            config,
            metrics: Arc::new(tokio::sync::Mutex::new(StressTestMetrics::default())),
        }
    }

    /// Create a WebDAV service with loop detection configured for stress testing
    fn create_webdav_service(&self) -> Result<WebDAVService> {
        let mut webdav_config = WebDAVConfig::new(
            self.config.webdav_url.clone(),
            self.config.username.clone(),
            self.config.password.clone(),
            self.config.test_directories.clone(),
            vec!["pdf".to_string(), "txt".to_string(), "doc".to_string(), "docx".to_string()],
        );

        // Configure loop detection for stress testing
        webdav_config.loop_detection = LoopDetectionConfig {
            enabled: true,
            max_access_count: 5, // Reasonable limit for stress testing
            time_window_secs: 60, // 1-minute window
            max_scan_duration_secs: self.config.loop_detection_timeout_secs,
            min_scan_interval_secs: 2, // 2-second minimum interval
            max_pattern_depth: 10,
            max_tracked_directories: 1000,
            enable_pattern_analysis: true,
            log_level: "warn".to_string(), // Reduce log noise during stress test
        };

        WebDAVService::new(webdav_config)
            .context("Failed to create WebDAV service for stress testing")
    }

    /// Run the main stress test
    pub async fn run(&self) -> Result<StressTestMetrics> {
        info!("🚀 Starting WebDAV Loop Detection Stress Test");
        info!("Configuration: {:?}", self.config);

        let start_time = Instant::now();
        let end_time = start_time + Duration::from_secs(self.config.duration_secs);

        // Create WebDAV services for concurrent testing
        let mut webdav_services = Vec::new();
        for i in 0..self.config.concurrent_syncs {
            match self.create_webdav_service() {
                Ok(service) => {
                    info!("✅ Created WebDAV service #{}", i);
                    webdav_services.push(service);
                }
                Err(e) => {
                    error!("❌ Failed to create WebDAV service #{}: {}", i, e);
                    return Err(e);
                }
            }
        }

        // Create app state for SmartSyncService
        let test_config = Config::test_default();
        let app_state = Arc::new(AppState::new_for_testing(test_config).await
            .context("Failed to create app state for testing")?);

        let smart_sync_service = SmartSyncService::new(app_state.clone());

        info!("🏁 Starting stress test operations...");

        // Launch concurrent sync operations
        let mut handles = Vec::new();
        
        for (service_id, webdav_service) in webdav_services.into_iter().enumerate() {
            let service = Arc::new(webdav_service);
            let smart_sync = smart_sync_service.clone();
            let config = self.config.clone();
            let metrics = Arc::clone(&self.metrics);
            
            let handle = tokio::spawn(async move {
                Self::run_sync_operations(
                    service_id,
                    service,
                    smart_sync,
                    config,
                    metrics,
                    end_time
                ).await
            });
            
            handles.push(handle);
        }

        // Wait for all operations to complete
        for (i, handle) in handles.into_iter().enumerate() {
            match handle.await {
                Ok(result) => {
                    if let Err(e) = result {
                        warn!("Sync operation #{} completed with error: {}", i, e);
                    } else {
                        info!("✅ Sync operation #{} completed successfully", i);
                    }
                }
                Err(e) => {
                    error!("❌ Failed to join sync operation #{}: {}", i, e);
                }
            }
        }

        let total_duration = start_time.elapsed();
        info!("🏁 Stress test completed in {:.2}s", total_duration.as_secs_f64());

        // Generate final metrics
        let final_metrics = self.generate_final_metrics().await;
        self.print_stress_test_report(&final_metrics, total_duration);

        Ok(final_metrics)
    }

    /// Run sync operations for a single WebDAV service
    async fn run_sync_operations(
        service_id: usize,
        webdav_service: Arc<WebDAVService>,
        smart_sync_service: SmartSyncService,
        config: StressTestConfig,
        metrics: Arc<tokio::sync::Mutex<StressTestMetrics>>,
        end_time: Instant,
    ) -> Result<()> {
        let user_id = Uuid::new_v4();
        let mut operation_count = 0;

        info!("🔄 Service #{} starting sync operations", service_id);

        while Instant::now() < end_time {
            operation_count += 1;
            let op_start = Instant::now();

            // Randomly select a directory to sync
            let dir_index = operation_count % config.test_directories.len();
            let target_directory = &config.test_directories[dir_index];

            debug!("Service #{} operation #{}: syncing {}", service_id, operation_count, target_directory);

            // Perform sync operation with loop detection
            let sync_result = Self::perform_monitored_sync(
                &*webdav_service,
                &smart_sync_service,
                user_id,
                target_directory,
                operation_count,
            ).await;

            let op_duration = op_start.elapsed();

            // Update metrics
            Self::update_metrics(
                &metrics,
                &sync_result,
                op_duration,
                &*webdav_service,
            ).await;

            // If we're testing loop triggers, occasionally create conditions that might cause loops
            if config.trigger_test_loops && operation_count % 10 == 0 {
                Self::trigger_test_loop_scenario(&*webdav_service, target_directory).await;
            }

            // Brief pause between operations to avoid overwhelming the server
            sleep(Duration::from_millis(100 + (service_id * 50) as u64)).await;
        }

        info!("📊 Service #{} completed {} operations", service_id, operation_count);
        Ok(())
    }

    /// Perform a single sync operation with comprehensive monitoring
    async fn perform_monitored_sync(
        webdav_service: &WebDAVService,
        smart_sync_service: &SmartSyncService,
        user_id: Uuid,
        directory: &str,
        operation_id: usize,
    ) -> Result<(usize, usize)> {
        // First evaluate if sync is needed
        match smart_sync_service.evaluate_sync_need(
            user_id,
            webdav_service,
            directory,
            None, // No progress tracking for stress test
        ).await {
            Ok(decision) => {
                match decision {
                    readur::services::webdav::SmartSyncDecision::SkipSync => {
                        debug!("Operation #{}: Sync skipped for {}", operation_id, directory);
                        Ok((0, 0))
                    }
                    readur::services::webdav::SmartSyncDecision::RequiresSync(strategy) => {
                        // Perform the actual sync
                        match smart_sync_service.perform_smart_sync(
                            user_id,
                            None, // No source ID for stress test
                            webdav_service,
                            directory,
                            strategy,
                            None, // No progress tracking
                        ).await {
                            Ok(result) => Ok((result.files.len(), result.directories.len())),
                            Err(e) => {
                                if e.to_string().contains("Loop detected") {
                                    debug!("Operation #{}: Loop detected for {} - {}", operation_id, directory, e);
                                    Err(e)
                                } else {
                                    warn!("Operation #{}: Sync failed for {} - {}", operation_id, directory, e);
                                    Err(e)
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Operation #{}: Sync evaluation failed for {} - {}", operation_id, directory, e);
                Err(e)
            }
        }
    }

    /// Trigger test scenarios that might cause loops (for testing purposes)
    async fn trigger_test_loop_scenario(webdav_service: &WebDAVService, directory: &str) {
        debug!("🧪 Triggering test loop scenario for {}", directory);
        
        // Rapid repeated access to the same directory
        for i in 0..3 {
            match webdav_service.discover_files_and_directories(directory, false).await {
                Ok(_) => debug!("Test loop trigger #{} succeeded for {}", i, directory),
                Err(e) => {
                    if e.to_string().contains("Loop detected") {
                        debug!("✅ Test loop scenario successfully triggered loop detection: {}", e);
                        return;
                    } else {
                        debug!("Test loop trigger #{} failed for {}: {}", i, directory, e);
                    }
                }
            }
            
            // Very short delay to trigger immediate re-scan detection
            sleep(Duration::from_millis(100)).await;
        }
    }

    /// Update metrics based on sync operation result
    async fn update_metrics(
        metrics: &Arc<tokio::sync::Mutex<StressTestMetrics>>,
        sync_result: &Result<(usize, usize)>,
        duration: Duration,
        webdav_service: &WebDAVService,
    ) {
        let mut m = metrics.lock().await;
        m.total_sync_operations += 1;

        let duration_ms = duration.as_millis() as u64;
        m.max_sync_duration_ms = m.max_sync_duration_ms.max(duration_ms);
        m.min_sync_duration_ms = m.min_sync_duration_ms.min(duration_ms);
        
        // Update average duration
        let total_duration = m.avg_sync_duration_ms * (m.total_sync_operations - 1) as f64;
        m.avg_sync_duration_ms = (total_duration + duration_ms as f64) / m.total_sync_operations as f64;

        match sync_result {
            Ok((files, dirs)) => {
                m.successful_syncs += 1;
                m.files_discovered += *files as u64;
                m.directories_discovered += *dirs as u64;
            }
            Err(e) => {
                m.failed_syncs += 1;
                
                let error_msg = e.to_string();
                if error_msg.contains("Loop detected") {
                    m.loops_detected += 1;
                    
                    // Classify loop types
                    if error_msg.contains("re-accessed after only") {
                        *m.loop_types_detected.entry("ImmediateReScan".to_string()).or_insert(0) += 1;
                    } else if error_msg.contains("Concurrent access detected") {
                        *m.loop_types_detected.entry("ConcurrentAccess".to_string()).or_insert(0) += 1;
                    } else if error_msg.contains("accessed") && error_msg.contains("times") {
                        *m.loop_types_detected.entry("FrequentReAccess".to_string()).or_insert(0) += 1;
                    } else if error_msg.contains("stuck") {
                        *m.loop_types_detected.entry("StuckScan".to_string()).or_insert(0) += 1;
                    } else if error_msg.contains("Circular pattern") {
                        *m.loop_types_detected.entry("CircularPattern".to_string()).or_insert(0) += 1;
                    } else {
                        *m.loop_types_detected.entry("Other".to_string()).or_insert(0) += 1;
                    }
                } else {
                    // Classify other error types
                    let error_type = if error_msg.contains("timeout") {
                        "Timeout"
                    } else if error_msg.contains("connection") {
                        "Connection"
                    } else if error_msg.contains("404") || error_msg.contains("Not Found") {
                        "NotFound"
                    } else if error_msg.contains("403") || error_msg.contains("Forbidden") {
                        "Forbidden"
                    } else if error_msg.contains("500") || error_msg.contains("Internal Server Error") {
                        "ServerError"
                    } else {
                        "Unknown"
                    };
                    
                    *m.errors_by_type.entry(error_type.to_string()).or_insert(0) += 1;
                }
            }
        }

        // Collect loop detection metrics from the WebDAV service
        if let Ok(ld_metrics) = webdav_service.get_loop_detection_metrics() {
            if let Some(total_loops) = ld_metrics.get("total_loops_detected") {
                if let Some(loops) = total_loops.as_u64() {
                    // Update our metrics with the actual count from loop detector
                    m.loops_detected = m.loops_detected.max(loops);
                }
            }
        }
    }

    /// Generate final comprehensive metrics
    async fn generate_final_metrics(&self) -> StressTestMetrics {
        self.metrics.lock().await.clone()
    }

    /// Print a comprehensive stress test report
    fn print_stress_test_report(&self, metrics: &StressTestMetrics, total_duration: Duration) {
        println!("\n" + "=".repeat(80).as_str());
        println!("📊 WEBDAV LOOP DETECTION STRESS TEST REPORT");
        println!("=".repeat(80));
        
        println!("\n🕒 Test Duration: {:.2}s", total_duration.as_secs_f64());
        println!("🔄 Total Sync Operations: {}", metrics.total_sync_operations);
        println!("✅ Successful Syncs: {} ({:.1}%)", 
                 metrics.successful_syncs, 
                 metrics.successful_syncs as f64 / metrics.total_sync_operations as f64 * 100.0);
        println!("❌ Failed Syncs: {} ({:.1}%)", 
                 metrics.failed_syncs, 
                 metrics.failed_syncs as f64 / metrics.total_sync_operations as f64 * 100.0);
        
        println!("\n🔄 Loop Detection Results:");
        println!("  🚨 Loops Detected: {} ({:.1}%)", 
                 metrics.loops_detected,
                 metrics.loops_detected as f64 / metrics.total_sync_operations as f64 * 100.0);
        
        if !metrics.loop_types_detected.is_empty() {
            println!("  📊 Loop Types Detected:");
            for (loop_type, count) in &metrics.loop_types_detected {
                println!("    - {}: {}", loop_type, count);
            }
        }

        println!("\n⚡ Performance Metrics:");
        println!("  📈 Average Sync Duration: {:.2}ms", metrics.avg_sync_duration_ms);
        println!("  🏃 Fastest Sync: {}ms", metrics.min_sync_duration_ms);
        println!("  🐌 Slowest Sync: {}ms", metrics.max_sync_duration_ms);
        println!("  🏁 Operations per Second: {:.2}", 
                 metrics.total_sync_operations as f64 / total_duration.as_secs_f64());

        println!("\n📁 Discovery Results:");
        println!("  📄 Files Discovered: {}", metrics.files_discovered);
        println!("  📂 Directories Discovered: {}", metrics.directories_discovered);
        
        if !metrics.errors_by_type.is_empty() {
            println!("\n❌ Error Breakdown:");
            for (error_type, count) in &metrics.errors_by_type {
                println!("  - {}: {} ({:.1}%)", 
                         error_type, count, 
                         *count as f64 / metrics.failed_syncs as f64 * 100.0);
            }
        }

        println!("\n" + "=".repeat(80).as_str());

        // Generate JSON report for CI/CD
        let report = json!({
            "test_type": "webdav_loop_detection_stress",
            "duration_secs": total_duration.as_secs_f64(),
            "total_operations": metrics.total_sync_operations,
            "successful_operations": metrics.successful_syncs,
            "failed_operations": metrics.failed_syncs,
            "success_rate": metrics.successful_syncs as f64 / metrics.total_sync_operations as f64 * 100.0,
            "loops_detected": metrics.loops_detected,
            "loop_detection_rate": metrics.loops_detected as f64 / metrics.total_sync_operations as f64 * 100.0,
            "avg_duration_ms": metrics.avg_sync_duration_ms,
            "min_duration_ms": metrics.min_sync_duration_ms,
            "max_duration_ms": metrics.max_sync_duration_ms,
            "ops_per_second": metrics.total_sync_operations as f64 / total_duration.as_secs_f64(),
            "files_discovered": metrics.files_discovered,
            "directories_discovered": metrics.directories_discovered,
            "loop_types": metrics.loop_types_detected,
            "error_types": metrics.errors_by_type,
        });

        // Write JSON report for CI/CD consumption
        if let Ok(report_dir) = std::env::var("STRESS_RESULTS_DIR") {
            let report_path = format!("{}/webdav_loop_detection_report.json", report_dir);
            if let Err(e) = std::fs::write(&report_path, serde_json::to_string_pretty(&report).unwrap()) {
                warn!("Failed to write JSON report to {}: {}", report_path, e);
            } else {
                info!("📋 JSON report written to {}", report_path);
            }
        }
    }
}

/// Main entry point for the stress test
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,webdav_loop_detection_stress=debug".to_string())
        )
        .init();

    let config = StressTestConfig::default();
    let stress_test = WebDAVLoopDetectionStressTest::new(config);
    
    let metrics = stress_test.run().await
        .context("Stress test failed")?;

    // Exit with error code if too many loops were detected (indicating a problem)
    let loop_rate = metrics.loops_detected as f64 / metrics.total_sync_operations as f64 * 100.0;
    if loop_rate > 50.0 {
        error!("🚨 CRITICAL: Loop detection rate ({:.1}%) exceeds threshold (50%)", loop_rate);
        std::process::exit(1);
    }

    // Exit with error code if success rate is too low
    let success_rate = metrics.successful_syncs as f64 / metrics.total_sync_operations as f64 * 100.0;
    if success_rate < 70.0 {
        error!("🚨 CRITICAL: Success rate ({:.1}%) below threshold (70%)", success_rate);
        std::process::exit(1);
    }

    info!("🎉 Stress test completed successfully!");
    Ok(())
}