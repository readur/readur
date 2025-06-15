/*!
 * OCR Queue Management Integration Tests
 * 
 * Tests OCR queue operations including:
 * - Queue statistics and monitoring
 * - Failed job recovery and requeuing
 * - Queue status tracking
 * - Performance monitoring
 * - Concurrent OCR processing
 * - Priority handling
 */

use reqwest::Client;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use uuid::Uuid;

use readur::models::{CreateUser, LoginRequest, LoginResponse, UserRole, DocumentResponse};

const BASE_URL: &str = "http://localhost:8000";
const TIMEOUT: Duration = Duration::from_secs(60);

/// Test client for OCR queue operations
struct OCRQueueTestClient {
    client: Client,
    token: Option<String>,
    user_id: Option<String>,
}

impl OCRQueueTestClient {
    fn new() -> Self {
        Self {
            client: Client::new(),
            token: None,
            user_id: None,
        }
    }
    
    /// Register and login a test user
    async fn register_and_login(&mut self, role: UserRole) -> Result<String, Box<dyn std::error::Error>> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let username = format!("ocr_queue_test_{}_{}", role.to_string(), timestamp);
        let email = format!("ocr_queue_test_{}@example.com", timestamp);
        let password = "testpassword123";
        
        // Register user
        let user_data = CreateUser {
            username: username.clone(),
            email: email.clone(),
            password: password.to_string(),
            role: Some(role),
        };
        
        let register_response = self.client
            .post(&format!("{}/api/auth/register", BASE_URL))
            .json(&user_data)
            .send()
            .await?;
        
        if !register_response.status().is_success() {
            return Err(format!("Registration failed: {}", register_response.text().await?).into());
        }
        
        // Login to get token
        let login_data = LoginRequest {
            username: username.clone(),
            password: password.to_string(),
        };
        
        let login_response = self.client
            .post(&format!("{}/api/auth/login", BASE_URL))
            .json(&login_data)
            .send()
            .await?;
        
        if !login_response.status().is_success() {
            return Err(format!("Login failed: {}", login_response.text().await?).into());
        }
        
        let login_result: LoginResponse = login_response.json().await?;
        self.token = Some(login_result.token.clone());
        
        // Get user info
        let me_response = self.client
            .get(&format!("{}/api/auth/me", BASE_URL))
            .header("Authorization", format!("Bearer {}", login_result.token))
            .send()
            .await?;
        
        if me_response.status().is_success() {
            let user_info: Value = me_response.json().await?;
            self.user_id = user_info["id"].as_str().map(|s| s.to_string());
        }
        
        Ok(login_result.token)
    }
    
    /// Get OCR queue statistics
    async fn get_queue_stats(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let response = self.client
            .get(&format!("{}/api/queue/stats", BASE_URL))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Get queue stats failed: {} - {}", response.status(), response.text().await?).into());
        }
        
        let stats: Value = response.json().await?;
        Ok(stats)
    }
    
    /// Requeue failed OCR jobs
    async fn requeue_failed_jobs(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let response = self.client
            .post(&format!("{}/api/queue/requeue-failed", BASE_URL))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Requeue failed jobs failed: {} - {}", response.status(), response.text().await?).into());
        }
        
        let result: Value = response.json().await?;
        Ok(result)
    }
    
    /// Upload a document for OCR processing
    async fn upload_document(&self, content: &str, filename: &str) -> Result<DocumentResponse, Box<dyn std::error::Error>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let part = reqwest::multipart::Part::text(content.to_string())
            .file_name(filename.to_string())
            .mime_str("text/plain")?;
        let form = reqwest::multipart::Form::new()
            .part("file", part);
        
        let response = self.client
            .post(&format!("{}/api/documents", BASE_URL))
            .header("Authorization", format!("Bearer {}", token))
            .multipart(form)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Upload failed: {}", response.text().await?).into());
        }
        
        let document: DocumentResponse = response.json().await?;
        Ok(document)
    }
    
    /// Upload multiple documents concurrently
    async fn upload_multiple_documents(&self, count: usize, base_content: &str) -> Result<Vec<DocumentResponse>, Box<dyn std::error::Error>> {
        let mut handles = Vec::new();
        
        for i in 0..count {
            let content = format!("{}\nDocument number: {}\nUnique ID: {}", base_content, i + 1, Uuid::new_v4());
            let filename = format!("test_doc_{}.txt", i + 1);
            let client_clone = self.clone();
            
            let handle = tokio::spawn(async move {
                client_clone.upload_document(&content, &filename).await
            });
            
            handles.push(handle);
        }
        
        let mut documents = Vec::new();
        for handle in handles {
            match handle.await? {
                Ok(doc) => documents.push(doc),
                Err(e) => return Err(e),
            }
        }
        
        Ok(documents)
    }
    
    /// Wait for OCR processing to complete for multiple documents
    async fn wait_for_multiple_ocr_completion(&self, document_ids: &[String]) -> Result<Vec<bool>, Box<dyn std::error::Error>> {
        let start = Instant::now();
        let mut completed_status = vec![false; document_ids.len()];
        
        while start.elapsed() < TIMEOUT && !completed_status.iter().all(|&x| x) {
            let token = self.token.as_ref().ok_or("Not authenticated")?;
            
            let response = self.client
                .get(&format!("{}/api/documents", BASE_URL))
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await?;
            
            if response.status().is_success() {
                let documents: Vec<DocumentResponse> = response.json().await?;
                
                for (i, doc_id) in document_ids.iter().enumerate() {
                    if !completed_status[i] {
                        if let Some(doc) = documents.iter().find(|d| d.id.to_string() == *doc_id) {
                            match doc.ocr_status.as_deref() {
                                Some("completed") => completed_status[i] = true,
                                Some("failed") => completed_status[i] = true, // Count failed as completed for this test
                                _ => continue,
                            }
                        }
                    }
                }
            }
            
            sleep(Duration::from_millis(1000)).await; // Check every second for multiple docs
        }
        
        Ok(completed_status)
    }
    
    /// Get all documents for the user
    async fn get_documents(&self) -> Result<Vec<DocumentResponse>, Box<dyn std::error::Error>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let response = self.client
            .get(&format!("{}/api/documents", BASE_URL))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Get documents failed: {}", response.text().await?).into());
        }
        
        let documents: Vec<DocumentResponse> = response.json().await?;
        Ok(documents)
    }
}

impl Clone for OCRQueueTestClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            token: self.token.clone(),
            user_id: self.user_id.clone(),
        }
    }
}

#[tokio::test]
async fn test_queue_stats_monitoring() {
    let mut client = OCRQueueTestClient::new();
    
    // Register and login
    client.register_and_login(UserRole::User).await
        .expect("Failed to register and login");
    
    println!("✅ User registered and logged in");
    
    // Get initial queue stats
    let initial_stats = client.get_queue_stats().await
        .expect("Failed to get initial queue stats");
    
    // Validate queue stats structure
    assert!(initial_stats.is_object());
    
    // Common queue stats fields to check for
    let expected_fields = ["pending", "processing", "completed", "failed", "total"];
    for field in &expected_fields {
        if initial_stats[field].is_number() {
            assert!(initial_stats[field].as_i64().unwrap() >= 0);
            println!("✅ Queue stat '{}': {}", field, initial_stats[field]);
        }
    }
    
    println!("✅ Initial queue stats retrieved and validated");
    
    // Upload a document to generate queue activity
    let document = client.upload_document("Test document for queue monitoring", "queue_test.txt").await
        .expect("Failed to upload document");
    
    println!("✅ Document uploaded: {}", document.id);
    
    // Wait a moment for queue to update
    sleep(Duration::from_secs(2)).await;
    
    // Get updated queue stats
    let updated_stats = client.get_queue_stats().await
        .expect("Failed to get updated queue stats");
    
    println!("✅ Updated queue stats retrieved");
    
    // The total should have increased (assuming the document entered the queue)
    if updated_stats["total"].is_number() && initial_stats["total"].is_number() {
        let initial_total = initial_stats["total"].as_i64().unwrap_or(0);
        let updated_total = updated_stats["total"].as_i64().unwrap_or(0);
        
        // Total should be equal or increased
        assert!(updated_total >= initial_total);
        println!("✅ Queue activity detected: total jobs {} -> {}", initial_total, updated_total);
    }
    
    println!("🎉 Queue stats monitoring test passed!");
}

#[tokio::test]
async fn test_failed_job_requeue_functionality() {
    let mut client = OCRQueueTestClient::new();
    
    client.register_and_login(UserRole::User).await
        .expect("Failed to register and login");
    
    println!("✅ User registered and logged in");
    
    // Get initial stats
    let initial_stats = client.get_queue_stats().await
        .expect("Failed to get initial stats");
    
    let initial_failed = initial_stats["failed"].as_i64().unwrap_or(0);
    println!("✅ Initial failed jobs: {}", initial_failed);
    
    // Try to requeue failed jobs
    let requeue_result = client.requeue_failed_jobs().await
        .expect("Failed to requeue failed jobs");
    
    // Validate requeue response structure
    assert!(requeue_result.is_object());
    
    // Common requeue result fields
    if requeue_result["requeued_count"].is_number() {
        let requeued_count = requeue_result["requeued_count"].as_i64().unwrap();
        assert!(requeued_count >= 0);
        println!("✅ Requeued {} failed jobs", requeued_count);
    }
    
    if requeue_result["message"].is_string() {
        println!("✅ Requeue message: {}", requeue_result["message"]);
    }
    
    // Wait a moment for the requeue to process
    sleep(Duration::from_secs(2)).await;
    
    // Get updated stats
    let updated_stats = client.get_queue_stats().await
        .expect("Failed to get updated stats after requeue");
    
    let updated_failed = updated_stats["failed"].as_i64().unwrap_or(0);
    
    // Failed count should be equal or decreased after requeue
    assert!(updated_failed <= initial_failed);
    println!("✅ Failed jobs after requeue: {}", updated_failed);
    
    println!("🎉 Failed job requeue functionality test passed!");
}

#[tokio::test]
async fn test_concurrent_ocr_processing() {
    let mut client = OCRQueueTestClient::new();
    
    client.register_and_login(UserRole::User).await
        .expect("Failed to register and login");
    
    println!("✅ User registered and logged in");
    
    // Get initial queue stats
    let initial_stats = client.get_queue_stats().await
        .expect("Failed to get initial stats");
    
    println!("✅ Initial queue stats captured");
    
    // Upload multiple documents concurrently
    let document_count = 5;
    let base_content = "This is a test document for concurrent OCR processing.\nIt contains multiple lines of text to ensure meaningful OCR work.\nThe system should handle multiple documents efficiently.";
    
    println!("📤 Starting concurrent upload of {} documents...", document_count);
    let start_time = Instant::now();
    
    let documents = client.upload_multiple_documents(document_count, base_content).await
        .expect("Failed to upload multiple documents");
    
    let upload_duration = start_time.elapsed();
    println!("✅ Uploaded {} documents in {:?}", documents.len(), upload_duration);
    
    // Collect document IDs
    let document_ids: Vec<String> = documents.iter()
        .map(|d| d.id.to_string())
        .collect();
    
    // Monitor queue stats during processing
    let processing_start = Instant::now();
    let mut stats_samples = Vec::new();
    
    // Take several queue stat samples during processing
    for i in 0..6 {
        let stats = client.get_queue_stats().await
            .expect("Failed to get queue stats during processing");
        
        stats_samples.push((processing_start.elapsed(), stats.clone()));
        
        if i < 5 {
            sleep(Duration::from_secs(3)).await;
        }
    }
    
    println!("✅ Collected {} queue stat samples during processing", stats_samples.len());
    
    // Print queue evolution
    for (elapsed, stats) in &stats_samples {
        println!("  {:?}: pending={}, processing={}, completed={}, failed={}", 
                 elapsed,
                 stats["pending"].as_i64().unwrap_or(0),
                 stats["processing"].as_i64().unwrap_or(0),
                 stats["completed"].as_i64().unwrap_or(0),
                 stats["failed"].as_i64().unwrap_or(0));
    }
    
    // Wait for all OCR processing to complete
    println!("⏳ Waiting for OCR processing to complete...");
    let completion_results = client.wait_for_multiple_ocr_completion(&document_ids).await
        .expect("Failed to wait for OCR completion");
    
    let completed_count = completion_results.iter().filter(|&&x| x).count();
    println!("✅ OCR completed for {}/{} documents", completed_count, document_count);
    
    // Get final queue stats
    let final_stats = client.get_queue_stats().await
        .expect("Failed to get final stats");
    
    println!("✅ Final queue stats: pending={}, processing={}, completed={}, failed={}",
             final_stats["pending"].as_i64().unwrap_or(0),
             final_stats["processing"].as_i64().unwrap_or(0),
             final_stats["completed"].as_i64().unwrap_or(0),
             final_stats["failed"].as_i64().unwrap_or(0));
    
    // Validate that the queue processed our documents
    let initial_total = initial_stats["total"].as_i64().unwrap_or(0);
    let final_total = final_stats["total"].as_i64().unwrap_or(0);
    
    assert!(final_total >= initial_total + document_count as i64);
    println!("✅ Queue total increased by at least {} jobs", document_count);
    
    println!("🎉 Concurrent OCR processing test passed!");
}

#[tokio::test]
async fn test_queue_performance_monitoring() {
    let mut client = OCRQueueTestClient::new();
    
    client.register_and_login(UserRole::User).await
        .expect("Failed to register and login");
    
    println!("✅ User registered and logged in");
    
    // Monitor queue performance over time
    let monitoring_duration = Duration::from_secs(30);
    let sample_interval = Duration::from_secs(5);
    let start_time = Instant::now();
    
    let mut performance_samples = Vec::new();
    
    // Upload a test document to create some queue activity
    let _document = client.upload_document("Performance monitoring test document", "perf_test.txt").await
        .expect("Failed to upload test document");
    
    println!("✅ Test document uploaded for performance monitoring");
    
    // Collect performance samples
    while start_time.elapsed() < monitoring_duration {
        let sample_time = Instant::now();
        
        let stats = client.get_queue_stats().await
            .expect("Failed to get queue stats for performance monitoring");
        
        let sample_duration = sample_time.elapsed();
        
        performance_samples.push((start_time.elapsed(), stats, sample_duration));
        
        println!("📊 Sample at {:?}: response_time={:?}, pending={}, processing={}",
                 start_time.elapsed(),
                 sample_duration,
                 stats["pending"].as_i64().unwrap_or(0),
                 stats["processing"].as_i64().unwrap_or(0));
        
        if start_time.elapsed() + sample_interval < monitoring_duration {
            sleep(sample_interval).await;
        }
    }
    
    println!("✅ Collected {} performance samples", performance_samples.len());
    
    // Analyze performance metrics
    let response_times: Vec<Duration> = performance_samples.iter()
        .map(|(_, _, duration)| *duration)
        .collect();
    
    let avg_response_time = response_times.iter().sum::<Duration>() / response_times.len() as u32;
    let max_response_time = *response_times.iter().max().unwrap();
    let min_response_time = *response_times.iter().min().unwrap();
    
    println!("📈 Performance Analysis:");
    println!("  Average response time: {:?}", avg_response_time);
    println!("  Max response time: {:?}", max_response_time);
    println!("  Min response time: {:?}", min_response_time);
    
    // Basic performance assertions
    assert!(avg_response_time < Duration::from_secs(5), "Average response time should be under 5 seconds");
    assert!(max_response_time < Duration::from_secs(10), "Max response time should be under 10 seconds");
    
    // Check for queue activity variations
    let queue_totals: Vec<i64> = performance_samples.iter()
        .map(|(_, stats, _)| stats["total"].as_i64().unwrap_or(0))
        .collect();
    
    let min_total = queue_totals.iter().min().unwrap();
    let max_total = queue_totals.iter().max().unwrap();
    
    println!("  Queue total range: {} - {}", min_total, max_total);
    
    println!("🎉 Queue performance monitoring test passed!");
}

#[tokio::test]
async fn test_queue_error_handling() {
    let mut client = OCRQueueTestClient::new();
    
    client.register_and_login(UserRole::User).await
        .expect("Failed to register and login");
    
    println!("✅ User registered and logged in");
    
    // Test unauthorized access to queue stats
    let unauth_client = Client::new();
    let unauth_response = unauth_client
        .get(&format!("{}/api/queue/stats", BASE_URL))
        .send()
        .await
        .expect("Request should complete");
    
    assert_eq!(unauth_response.status(), 401);
    println!("✅ Unauthorized queue stats access properly rejected");
    
    // Test unauthorized requeue attempt
    let unauth_requeue_response = unauth_client
        .post(&format!("{}/api/queue/requeue-failed", BASE_URL))
        .send()
        .await
        .expect("Request should complete");
    
    assert_eq!(unauth_requeue_response.status(), 401);
    println!("✅ Unauthorized requeue attempt properly rejected");
    
    // Test queue stats with valid authentication
    let stats_result = client.get_queue_stats().await;
    assert!(stats_result.is_ok());
    println!("✅ Authorized queue stats access successful");
    
    // Test requeue with valid authentication
    let requeue_result = client.requeue_failed_jobs().await;
    assert!(requeue_result.is_ok());
    println!("✅ Authorized requeue attempt successful");
    
    println!("🎉 Queue error handling test passed!");
}

#[tokio::test]
async fn test_queue_stats_consistency() {
    let mut client = OCRQueueTestClient::new();
    
    client.register_and_login(UserRole::User).await
        .expect("Failed to register and login");
    
    println!("✅ User registered and logged in");
    
    // Get multiple queue stat samples to check consistency
    let mut stat_samples = Vec::new();
    
    for i in 0..5 {
        let stats = client.get_queue_stats().await
            .expect("Failed to get queue stats");
        
        stat_samples.push(stats);
        
        if i < 4 {
            sleep(Duration::from_millis(500)).await;
        }
    }
    
    println!("✅ Collected {} queue stat samples", stat_samples.len());
    
    // Validate consistency across samples
    for (i, stats) in stat_samples.iter().enumerate() {
        // Check that all expected fields are numbers
        let numeric_fields = ["pending", "processing", "completed", "failed", "total"];
        
        for field in &numeric_fields {
            if let Some(value) = stats[field].as_i64() {
                assert!(value >= 0, "Field '{}' should be non-negative in sample {}", field, i);
            }
        }
        
        // Check logical consistency: total should equal sum of other states
        if let (Some(pending), Some(processing), Some(completed), Some(failed), Some(total)) = (
            stats["pending"].as_i64(),
            stats["processing"].as_i64(),
            stats["completed"].as_i64(),
            stats["failed"].as_i64(),
            stats["total"].as_i64()
        ) {
            let calculated_total = pending + processing + completed + failed;
            // Allow some tolerance for race conditions in a live system
            let tolerance = 5;
            assert!(
                (total - calculated_total).abs() <= tolerance,
                "Total ({}) should approximately equal sum of states ({}) in sample {}",
                total, calculated_total, i
            );
        }
        
        println!("✅ Sample {} consistency validated", i);
    }
    
    // Check for reasonable queue evolution (no massive jumps)
    for i in 1..stat_samples.len() {
        let prev_total = stat_samples[i-1]["total"].as_i64().unwrap_or(0);
        let curr_total = stat_samples[i]["total"].as_i64().unwrap_or(0);
        
        // Total should only increase or stay the same in a short time period
        assert!(curr_total >= prev_total - 1, "Total queue size should not decrease significantly between samples");
    }
    
    println!("🎉 Queue stats consistency test passed!");
}