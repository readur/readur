use reqwest::{Client, StatusCode};
use regex::Regex;
use std::collections::HashSet;
use serde_json::json;
use uuid;

use readur::models::{CreateUser, LoginRequest, LoginResponse};

fn get_base_url() -> String {
    std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8000".to_string())
}

/// Helper to create a test user and return the auth token
async fn create_test_user_with_token(client: &Client) -> Result<String, Box<dyn std::error::Error>> {
    let base_url = get_base_url();
    let username = format!("testuser_{}", uuid::Uuid::new_v4());
    let password = "test_password123";
    
    // Register user
    let register_data = CreateUser {
        username: username.clone(),
        password: password.to_string(),
        email: format!("{}@test.com", username),
        role: None,
    };
    
    client
        .post(&format!("{}/api/auth/register", base_url))
        .json(&register_data)
        .send()
        .await?;
    
    // Login to get token
    let login_data = LoginRequest {
        username,
        password: password.to_string(),
    };
    
    let login_response: LoginResponse = client
        .post(&format!("{}/api/auth/login", base_url))
        .json(&login_data)
        .send()
        .await?
        .json()
        .await?;
    
    Ok(login_response.token)
}

#[tokio::test]
async fn test_prometheus_metrics_endpoint_returns_success() {
    let client = Client::new();
    let base_url = get_base_url();
    
    let response = client
        .get(&format!("{}/metrics", base_url))
        .send()
        .await
        .expect("Failed to send request");
    
    assert_eq!(response.status(), StatusCode::OK);
    
    // Check content type
    let content_type = response.headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .expect("Missing content-type header");
    
    assert_eq!(content_type, "text/plain; version=0.0.4");
}

#[tokio::test]
async fn test_prometheus_metrics_format_is_valid() {
    let client = Client::new();
    let base_url = get_base_url();
    
    let response = client
        .get(&format!("{}/metrics", base_url))
        .send()
        .await
        .expect("Failed to send request");
    
    let body = response.text().await.expect("Failed to read response body");
    
    // Validate Prometheus format using regex
    // Format: metric_name{labels} value timestamp
    let metric_line_regex = Regex::new(r"^[a-zA-Z_:][a-zA-Z0-9_:]*(\{[^}]*\})?\s+[0-9.+-eE]+(\s+[0-9]+)?$").unwrap();
    let comment_regex = Regex::new(r"^#\s+(HELP|TYPE)\s+").unwrap();
    
    for line in body.lines() {
        if line.is_empty() {
            continue;
        }
        
        // Line should be either a comment or a metric
        assert!(
            comment_regex.is_match(line) || metric_line_regex.is_match(line),
            "Invalid Prometheus format in line: {}",
            line
        );
    }
}

#[tokio::test]
async fn test_all_expected_metrics_are_present() {
    let client = Client::new();
    let base_url = get_base_url();
    
    // Create some test data
    let token = create_test_user_with_token(&client).await.expect("Failed to create test user");
    
    // Upload a test document
    let file_content = b"Test document content";
    let form = reqwest::multipart::Form::new()
        .text("name", "test.txt")
        .part("file", reqwest::multipart::Part::bytes(file_content.to_vec())
            .file_name("test.txt")
            .mime_str("text/plain").unwrap());
    
    let _upload_response = client
        .post(&format!("{}/api/documents", base_url))
        .bearer_auth(&token)
        .multipart(form)
        .send()
        .await
        .expect("Failed to upload document");
    
    // Get metrics
    let response = client
        .get(&format!("{}/metrics", base_url))
        .send()
        .await
        .expect("Failed to send request");
    
    let body = response.text().await.expect("Failed to read response body");
    
    // Define expected metrics
    let expected_metrics = vec![
        // Document metrics
        "readur_documents_total",
        "readur_documents_uploaded_today",
        "readur_storage_bytes",
        "readur_documents_with_ocr",
        "readur_documents_without_ocr",
        
        // OCR metrics
        "readur_ocr_queue_pending",
        "readur_ocr_queue_processing",
        "readur_ocr_queue_failed",
        "readur_ocr_completed_today",
        "readur_ocr_stuck_jobs",
        "readur_ocr_queue_depth",
        
        // User metrics
        "readur_users_total",
        "readur_users_active_today",
        "readur_users_registered_today",
        
        // Database metrics
        "readur_db_connections_active",
        "readur_db_connections_idle",
        "readur_db_connections_total",
        "readur_db_utilization_percent",
        "readur_db_response_time_ms",
        
        // System metrics
        "readur_uptime_seconds",
        "readur_data_consistency_score",
        
        // Storage metrics
        "readur_avg_document_size_bytes",
        "readur_documents_by_type",
        
        // Security metrics
        "readur_failed_logins_today",
        "readur_document_access_today",
    ];
    
    // Check each metric is present
    for metric in expected_metrics {
        assert!(
            body.contains(metric),
            "Metric '{}' not found in response",
            metric
        );
    }
}

#[tokio::test]
async fn test_metrics_contain_valid_timestamps() {
    let client = Client::new();
    let base_url = get_base_url();
    
    let response = client
        .get(&format!("{}/metrics", base_url))
        .send()
        .await
        .expect("Failed to send request");
    
    let body = response.text().await.expect("Failed to read response body");
    
    // Check that metric lines contain timestamps
    let metric_with_timestamp_regex = Regex::new(r"^[a-zA-Z_:][a-zA-Z0-9_:]*(\{[^}]*\})?\s+[0-9.+-eE]+\s+([0-9]+)$").unwrap();
    
    let mut found_timestamps = false;
    for line in body.lines() {
        if let Some(captures) = metric_with_timestamp_regex.captures(line) {
            if let Some(timestamp_match) = captures.get(2) {
                let timestamp: i64 = timestamp_match.as_str().parse().unwrap();
                // Verify timestamp is reasonable (after year 2020 and not too far in future)
                assert!(timestamp > 1577836800000); // Jan 1, 2020 in milliseconds
                assert!(timestamp < 2000000000000); // Reasonable future date
                found_timestamps = true;
            }
        }
    }
    
    assert!(found_timestamps, "No timestamps found in metrics");
}

#[tokio::test]
async fn test_metrics_values_are_non_negative() {
    let client = Client::new();
    let base_url = get_base_url();
    
    let response = client
        .get(&format!("{}/metrics", base_url))
        .send()
        .await
        .expect("Failed to send request");
    
    let body = response.text().await.expect("Failed to read response body");
    
    // Parse metric values
    let metric_value_regex = Regex::new(r"^([a-zA-Z_:][a-zA-Z0-9_:]*)(\{[^}]*\})?\s+([-0-9.+eE]+)").unwrap();
    
    for line in body.lines() {
        if let Some(captures) = metric_value_regex.captures(line) {
            let metric_name = captures.get(1).unwrap().as_str();
            let value_str = captures.get(3).unwrap().as_str();
            
            if let Ok(value) = value_str.parse::<f64>() {
                // Most metrics should be non-negative except for special cases
                if !metric_name.contains("consistency_score") { // This could theoretically be negative
                    assert!(
                        value >= 0.0,
                        "Metric '{}' has negative value: {}",
                        metric_name,
                        value
                    );
                }
            }
        }
    }
}

#[tokio::test]
async fn test_document_type_metrics_have_labels() {
    let client = Client::new();
    let base_url = get_base_url();
    
    // Upload documents of different types
    let files = vec![
        ("test.pdf", "application/pdf"),
        ("test.jpg", "image/jpeg"),
        ("test.png", "image/png"),
    ];
    
    let token = create_test_user_with_token(&client).await.expect("Failed to create test user");
    
    for (filename, mime_type) in files {
        let form = reqwest::multipart::Form::new()
            .text("name", filename)
            .part("file", reqwest::multipart::Part::bytes(b"test content".to_vec())
                .file_name(filename)
                .mime_str(mime_type).unwrap());
        
        let _ = client
            .post(&format!("{}/api/documents", base_url))
            .bearer_auth(&token)
            .multipart(form)
            .send()
            .await;
    }
    
    let response = client
        .get(&format!("{}/metrics", base_url))
        .send()
        .await
        .expect("Failed to send request");
    
    let body = response.text().await.expect("Failed to read response body");
    
    // Check for labeled metrics - at least one document type should be present
    assert!(body.contains("readur_documents_by_type{type="));
    
    // Check that the uploaded files are categorized (may be pdf, jpeg, png, or other depending on upload success)
    let has_pdf = body.contains("readur_documents_by_type{type=\"pdf\"}");
    let has_jpeg = body.contains("readur_documents_by_type{type=\"jpeg\"}");
    let has_png = body.contains("readur_documents_by_type{type=\"png\"}");
    let has_other = body.contains("readur_documents_by_type{type=\"other\"}");
    
    // At least one document type should be present
    assert!(has_pdf || has_jpeg || has_png || has_other, 
           "No document type metrics found in response");
}

#[tokio::test]
async fn test_metrics_help_and_type_annotations() {
    let client = Client::new();
    let base_url = get_base_url();
    
    let response = client
        .get(&format!("{}/metrics", base_url))
        .send()
        .await
        .expect("Failed to send request");
    
    let body = response.text().await.expect("Failed to read response body");
    
    // Check that each metric has HELP and TYPE annotations
    let help_regex = Regex::new(r"^# HELP ([a-zA-Z_:][a-zA-Z0-9_:]*) (.+)$").unwrap();
    let type_regex = Regex::new(r"^# TYPE ([a-zA-Z_:][a-zA-Z0-9_:]*) (gauge|counter|histogram|summary)$").unwrap();
    
    let mut metrics_with_help = HashSet::new();
    let mut metrics_with_type = HashSet::new();
    
    for line in body.lines() {
        if let Some(captures) = help_regex.captures(line) {
            metrics_with_help.insert(captures.get(1).unwrap().as_str().to_string());
        }
        if let Some(captures) = type_regex.captures(line) {
            metrics_with_type.insert(captures.get(1).unwrap().as_str().to_string());
        }
    }
    
    // Verify key metrics have both HELP and TYPE
    let key_metrics = vec![
        "readur_documents_total",
        "readur_ocr_queue_pending",
        "readur_users_total",
        "readur_db_connections_active",
    ];
    
    for metric in key_metrics {
        assert!(
            metrics_with_help.contains(metric),
            "Metric '{}' missing HELP annotation",
            metric
        );
        assert!(
            metrics_with_type.contains(metric),
            "Metric '{}' missing TYPE annotation",
            metric
        );
    }
}

#[tokio::test]
async fn test_metrics_endpoint_performance() {
    let client = Client::new();
    let base_url = get_base_url();
    
    // Create some test data
    let token = create_test_user_with_token(&client).await.expect("Failed to create test user");
    
    // Upload multiple documents to create more data
    for i in 0..10 {
        let form = reqwest::multipart::Form::new()
            .text("name", format!("test{}.txt", i))
            .part("file", reqwest::multipart::Part::bytes(b"test content".to_vec())
                .file_name(format!("test{}.txt", i))
                .mime_str("text/plain").unwrap());
        
        let _ = client
            .post(&format!("{}/api/documents", base_url))
            .bearer_auth(&token)
            .multipart(form)
            .send()
            .await;
    }
    
    // Measure response time
    let start = std::time::Instant::now();
    
    let response = client
        .get(&format!("{}/metrics", base_url))
        .send()
        .await
        .expect("Failed to send request");
    
    let duration = start.elapsed();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    // Metrics endpoint should respond quickly (under 1 second)
    assert!(
        duration.as_millis() < 1000,
        "Metrics endpoint took too long: {}ms",
        duration.as_millis()
    );
}

#[tokio::test]
async fn test_metrics_concurrent_requests() {
    let base_url = get_base_url();
    
    // Send multiple concurrent requests
    let mut handles = vec![];
    
    for _ in 0..5 {
        let base_url_clone = base_url.clone();
        
        let handle = tokio::spawn(async move {
            let client = Client::new();
            let response = client
                .get(&format!("{}/metrics", base_url_clone))
                .send()
                .await
                .expect("Failed to send request");
            
            response.status()
        });
        
        handles.push(handle);
    }
    
    // Wait for all requests to complete
    for handle in handles {
        let status = handle.await.expect("Task panicked");
        assert_eq!(status, StatusCode::OK);
    }
}

#[tokio::test]
async fn test_metrics_endpoint_no_auth_required() {
    let client = Client::new();
    let base_url = get_base_url();
    
    // Test that metrics endpoint doesn't require authentication
    let response = client
        .get(&format!("{}/metrics", base_url))
        .send()
        .await
        .expect("Failed to send request");
    
    // Should succeed without authentication
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = response.text().await.expect("Failed to read response body");
    assert!(!body.is_empty());
    assert!(body.contains("readur_"));
}

// Helper to validate metric value ranges
fn assert_metric_in_range(body: &str, metric_name: &str, min: f64, max: f64) {
    let regex = Regex::new(&format!(r"^{}\s+([-0-9.+eE]+)", regex::escape(metric_name))).unwrap();
    
    for line in body.lines() {
        if let Some(captures) = regex.captures(line) {
            let value: f64 = captures.get(1).unwrap().as_str().parse().unwrap();
            assert!(
                value >= min && value <= max,
                "Metric '{}' value {} is out of range [{}, {}]",
                metric_name,
                value,
                min,
                max
            );
            return;
        }
    }
    
    panic!("Metric '{}' not found", metric_name);
}