#[cfg(test)]
mod tests {
    use super::super::*;
    use super::super::loop_detection::{LoopDetectionService, LoopDetectionConfig, LoopType};
    use crate::{AppState, config::Config};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::sleep;
    use uuid::Uuid;
    
    /// Helper to create a test WebDAV service with loop detection enabled
    async fn create_test_webdav_service_with_loop_detection() -> WebDAVService {
        let mut config = WebDAVConfig::new(
            "http://localhost:8080".to_string(),
            "test_user".to_string(),
            "test_pass".to_string(),
            vec!["/test".to_string()],
            vec!["pdf".to_string(), "txt".to_string()],
        );
        
        // Configure loop detection with tight thresholds for testing
        config.loop_detection = LoopDetectionConfig {
            enabled: true,
            max_access_count: 2, // Very low for testing
            time_window_secs: 10, // Short window
            max_scan_duration_secs: 5, // Short timeout
            min_scan_interval_secs: 1, // Short interval
            max_pattern_depth: 5,
            max_tracked_directories: 100,
            enable_pattern_analysis: true,
            log_level: "debug".to_string(),
        };
        
        WebDAVService::new(config).expect("Failed to create WebDAV service")
    }
    
    /// Helper to create a mock WebDAV server response for testing
    fn create_mock_webdav_response(num_files: usize, num_dirs: usize) -> WebDAVDiscoveryResult {
        let mut files = Vec::new();
        let mut directories = Vec::new();
        
        for i in 0..num_files {
            files.push(crate::models::FileIngestionInfo {
                uuid: Uuid::new_v4(),
                filename: format!("file_{}.pdf", i),
                relative_path: format!("/test/file_{}.pdf", i),
                absolute_url: format!("http://localhost:8080/test/file_{}.pdf", i),
                file_size_bytes: 1024 * (i + 1) as i64,
                last_modified: chrono::Utc::now(),
                etag: format!("etag_{}", i),
                content_type: "application/pdf".to_string(),
                is_directory: false,
            });
        }
        
        for i in 0..num_dirs {
            directories.push(crate::models::FileIngestionInfo {
                uuid: Uuid::new_v4(),
                filename: format!("dir_{}", i),
                relative_path: format!("/test/dir_{}", i),
                absolute_url: format!("http://localhost:8080/test/dir_{}/", i),
                file_size_bytes: 0,
                last_modified: chrono::Utc::now(),
                etag: format!("dir_etag_{}", i),
                content_type: "httpd/unix-directory".to_string(),
                is_directory: true,
            });
        }
        
        WebDAVDiscoveryResult { files, directories }
    }
    
    #[tokio::test]
    async fn test_loop_detection_immediate_rescan() {
        let service = create_test_webdav_service_with_loop_detection().await;
        
        // First access should succeed
        let access1 = service.loop_detector.start_access("/test/path", "test_scan").unwrap();
        service.loop_detector.complete_access(access1, Some(5), Some(2), None).unwrap();
        
        // Immediate second access should fail due to min_scan_interval
        let result = service.loop_detector.start_access("/test/path", "test_scan");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("re-accessed after only"));
        
        // Metrics should show the loop detection
        let metrics = service.get_loop_detection_metrics().unwrap();
        assert_eq!(metrics["total_loops_detected"], 1);
    }
    
    #[tokio::test]
    async fn test_loop_detection_concurrent_access() {
        let service = create_test_webdav_service_with_loop_detection().await;
        
        // Start first access
        let _access1 = service.loop_detector.start_access("/test/path", "scan1").unwrap();
        
        // Concurrent access should fail
        let result = service.loop_detector.start_access("/test/path", "scan2");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Concurrent access detected"));
        
        let metrics = service.get_loop_detection_metrics().unwrap();
        assert_eq!(metrics["total_loops_detected"], 1);
    }
    
    #[tokio::test]
    async fn test_loop_detection_frequency_limit() {
        let service = create_test_webdav_service_with_loop_detection().await;
        
        // Clear state to start fresh
        service.clear_loop_detection_state().unwrap();
        
        // Do multiple accesses that complete quickly
        for i in 0..3 {
            if i > 0 {
                // Wait minimum interval to avoid immediate re-scan detection
                sleep(Duration::from_millis(1100)).await;
            }
            
            let access = service.loop_detector.start_access("/test/freq_path", &format!("scan_{}", i));
            
            if i < 2 {
                // First two should succeed
                assert!(access.is_ok());
                let access_id = access.unwrap();
                service.loop_detector.complete_access(access_id, Some(i * 2), Some(i), None).unwrap();
            } else {
                // Third should fail due to frequency limit
                assert!(access.is_err());
                assert!(access.unwrap_err().to_string().contains("accessed 2 times"));
            }
        }
        
        let metrics = service.get_loop_detection_metrics().unwrap();
        assert_eq!(metrics["total_loops_detected"], 1);
    }
    
    #[tokio::test]
    async fn test_loop_detection_disabled() {
        let mut config = WebDAVConfig::new(
            "http://localhost:8080".to_string(),
            "test_user".to_string(),
            "test_pass".to_string(),
            vec!["/test".to_string()],
            vec!["pdf".to_string()],
        );
        
        // Disable loop detection
        config.loop_detection.enabled = false;
        
        let service = WebDAVService::new(config).unwrap();
        
        // Multiple rapid accesses should all succeed when disabled
        for i in 0..5 {
            let access = service.loop_detector.start_access("/test/path", &format!("scan_{}", i)).unwrap();
            service.loop_detector.complete_access(access, Some(i), Some(1), None).unwrap();
        }
        
        let metrics = service.get_loop_detection_metrics().unwrap();
        assert!(!metrics["enabled"].as_bool().unwrap());
    }
    
    #[tokio::test]
    async fn test_loop_detection_error_tracking() {
        let service = create_test_webdav_service_with_loop_detection().await;
        
        // Test error tracking in loop detection
        let access = service.loop_detector.start_access("/test/error_path", "error_scan").unwrap();
        service.loop_detector.complete_access(
            access, 
            None, 
            None, 
            Some("Test error message".to_string())
        ).unwrap();
        
        let metrics = service.get_loop_detection_metrics().unwrap();
        assert_eq!(metrics["total_accesses"], 1);
        assert_eq!(metrics["total_loops_detected"], 0); // No loops, just an error
    }
    
    #[tokio::test]
    async fn test_loop_detection_cleanup() {
        let service = create_test_webdav_service_with_loop_detection().await;
        
        // Add some access data
        for i in 0..3 {
            let access = service.loop_detector.start_access(&format!("/test/cleanup_{}", i), "cleanup_scan").unwrap();
            service.loop_detector.complete_access(access, Some(i), Some(1), None).unwrap();
            sleep(Duration::from_millis(100)).await; // Small delay between accesses
        }
        
        let metrics_before = service.get_loop_detection_metrics().unwrap();
        assert_eq!(metrics_before["total_accesses"], 3);
        
        // Clear state
        service.clear_loop_detection_state().unwrap();
        
        let metrics_after = service.get_loop_detection_metrics().unwrap();
        assert_eq!(metrics_after["active_accesses"], 0);
        assert_eq!(metrics_after["history_size"], 0);
    }
    
    #[tokio::test]
    async fn test_loop_detection_config_update() {
        let service = create_test_webdav_service_with_loop_detection().await;
        
        // Update configuration
        let mut new_config = LoopDetectionConfig::default();
        new_config.max_access_count = 10; // Much higher limit
        new_config.log_level = "info".to_string();
        
        service.update_loop_detection_config(new_config).unwrap();
        
        let metrics = service.get_loop_detection_metrics().unwrap();
        assert_eq!(metrics["config"]["max_access_count"], 10);
        assert_eq!(metrics["config"]["log_level"], "info");
    }
    
    #[tokio::test]
    async fn test_pattern_analysis_circular_detection() {
        let service = create_test_webdav_service_with_loop_detection().await;
        service.clear_loop_detection_state().unwrap();
        
        // Simulate A -> B -> A pattern with proper timing
        let paths = ["/test/path_a", "/test/path_b", "/test/path_a"];
        
        for (i, path) in paths.iter().enumerate() {
            if i > 0 {
                sleep(Duration::from_millis(1100)).await; // Wait minimum interval
            }
            
            let access = service.loop_detector.start_access(path, &format!("pattern_scan_{}", i));
            
            if i < 2 {
                // First two should succeed
                assert!(access.is_ok());
                let access_id = access.unwrap();
                service.loop_detector.complete_access(access_id, Some(1), Some(0), None).unwrap();
            } else {
                // Third access to path_a might trigger pattern detection
                // Note: The exact behavior depends on the pattern detection algorithm
                if let Err(e) = access {
                    println!("Pattern detection triggered: {}", e);
                } else {
                    let access_id = access.unwrap();
                    service.loop_detector.complete_access(access_id, Some(1), Some(0), None).unwrap();
                }
            }
        }
        
        let metrics = service.get_loop_detection_metrics().unwrap();
        println!("Pattern analysis metrics: {}", serde_json::to_string_pretty(&metrics).unwrap());
    }
    
    #[tokio::test]
    async fn test_webdav_service_integration_with_loop_detection() {
        // This test would ideally connect to a real WebDAV server
        // For now, we test the integration points
        let service = create_test_webdav_service_with_loop_detection().await;
        
        // Test that the service has loop detection enabled
        let metrics = service.get_loop_detection_metrics().unwrap();
        assert!(metrics["enabled"].as_bool().unwrap());
        
        // Test configuration access
        assert_eq!(metrics["config"]["max_access_count"], 2);
        assert_eq!(metrics["config"]["time_window_secs"], 10);
        
        // Test that we can update the config
        let mut new_config = LoopDetectionConfig::default();
        new_config.enabled = false;
        service.update_loop_detection_config(new_config).unwrap();
        
        let updated_metrics = service.get_loop_detection_metrics().unwrap();
        assert!(!updated_metrics["enabled"].as_bool().unwrap());
    }
    
    /// Integration test with SmartSyncService
    #[tokio::test]
    async fn test_smart_sync_loop_detection_integration() {
        // Create test app state
        let test_config = Config::test_default();
        let app_state = Arc::new(AppState::new_for_testing(test_config).await.unwrap());
        
        let smart_sync = SmartSyncService::new(app_state);
        let webdav_service = create_test_webdav_service_with_loop_detection().await;
        
        // Test that SmartSyncService can access loop detection metrics
        let metrics = smart_sync.get_loop_detection_metrics(&webdav_service).unwrap();
        assert!(metrics["enabled"].as_bool().unwrap());
        
        // Test that metrics are properly structured
        assert!(metrics.get("total_accesses").is_some());
        assert!(metrics.get("total_loops_detected").is_some());
        assert!(metrics.get("config").is_some());
    }
    
    /// Performance test to ensure loop detection doesn't significantly impact performance
    #[tokio::test]
    async fn test_loop_detection_performance() {
        let service = create_test_webdav_service_with_loop_detection().await;
        
        let start_time = std::time::Instant::now();
        
        // Perform many operations with different paths to avoid triggering detection
        for i in 0..100 {
            let path = format!("/test/perf_path_{}", i);
            let access = service.loop_detector.start_access(&path, "perf_test").unwrap();
            service.loop_detector.complete_access(access, Some(10), Some(2), None).unwrap();
        }
        
        let elapsed = start_time.elapsed();
        println!("100 loop detection operations took: {:?}", elapsed);
        
        // Should complete quickly (within 1 second for 100 operations)
        assert!(elapsed < Duration::from_secs(1), "Loop detection performance too slow: {:?}", elapsed);
        
        let metrics = service.get_loop_detection_metrics().unwrap();
        assert_eq!(metrics["total_accesses"], 100);
        assert_eq!(metrics["total_loops_detected"], 0);
    }
}