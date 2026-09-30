use super::*;
use crate::services::webdav::config::WebDAVConfig;

#[test]
fn test_user_agent_format() {
    // Test that the User-Agent header format matches the expected pattern
    let expected_version = env!("CARGO_PKG_VERSION");
    let expected_user_agent = build_user_agent();
    
    // Create a simple WebDAV config for testing
    let config = WebDAVConfig {
        server_url: "https://test.example.com".to_string(),
        username: "test_user".to_string(),
        password: "test_password".to_string(),
        watch_folders: vec!["/".to_string()],
        file_extensions: vec![],
        timeout_seconds: 30,
        server_type: Some("generic".to_string()),
    };
    
    let service = WebDAVService::new(config).expect("Failed to create WebDAV service");
    
    // Test that we can build the User-Agent string properly
    let user_agent = build_user_agent();
    assert_eq!(user_agent, expected_user_agent);
    assert!(user_agent.starts_with("Readur/"));
    assert!(user_agent.contains("(WebDAV-Sync; +https://github.com/readur)"));
    assert!(user_agent.contains(expected_version));
    
    println!("✅ User-Agent format test passed: {}", user_agent);
}

#[test]
fn test_retry_delay_with_jitter() {
    // Create a test WebDAV config
    let config = WebDAVConfig {
        server_url: "https://test.example.com".to_string(),
        username: "test_user".to_string(),
        password: "test_password".to_string(),
        watch_folders: vec!["/".to_string()],
        file_extensions: vec![],
        timeout_seconds: 30,
        server_type: Some("generic".to_string()),
    };
    
    let retry_config = RetryConfig {
        max_retries: 3,
        initial_delay_ms: 1000,
        max_delay_ms: 10000,
        backoff_multiplier: 2.0,
        timeout_seconds: 30,
        rate_limit_backoff_ms: 5000,
    };
    
    let service = WebDAVService::new_with_retry(config, retry_config).expect("Failed to create WebDAV service");
    
    // Test jitter calculation for different attempts
    for attempt in 0..5 {
        let delay = service.calculate_retry_delay_with_jitter(attempt, 1000);
        let expected_base = (1000.0 * 2.0_f64.powi(attempt as i32)) as u64;
        let capped_base = std::cmp::min(expected_base, 10000);
        let expected_min = (capped_base as f64 * 0.9) as u64;
        let expected_max = (capped_base as f64 * 1.1) as u64;
        
        println!("Attempt {}: delay = {}ms (expected range: {}-{}ms, base: {}ms)", 
                 attempt, delay, expected_min, expected_max, expected_base);
        
        // Verify the delay is within the expected jitter range
        assert!(delay >= expected_min, 
               "Delay {} for attempt {} is below minimum expected {}", 
               delay, attempt, expected_min);
        assert!(delay <= expected_max, 
               "Delay {} for attempt {} is above maximum expected {}", 
               delay, attempt, expected_max);
        
        // Ensure it doesn't exceed max_delay_ms
        assert!(delay <= 10000, 
               "Delay {} for attempt {} exceeds max_delay_ms {}", 
               delay, attempt, 10000);
    }
    
    // Test that jitter produces different values (run multiple times for same attempt)
    let mut delays = Vec::new();
    for _ in 0..10 {
        delays.push(service.calculate_retry_delay_with_jitter(2, 1000));
    }
    
    // Check that we get some variation (not all values are identical)
    let first_delay = delays[0];
    let has_variation = delays.iter().any(|&d| d != first_delay);
    assert!(has_variation, 
           "Jitter should produce some variation, but all delays were {}", first_delay);
    
    println!("✅ Retry delay with jitter test passed - variation observed");
    println!("   Sample delays for attempt 2: {:?}", delays);
}
