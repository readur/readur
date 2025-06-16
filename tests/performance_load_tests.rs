/*!
 * Performance and Load Testing Integration Tests
 * 
 * Tests system performance under various load conditions including:
 * - High-volume document uploads
 * - Concurrent user operations
 * - Database query performance
 * - OCR processing throughput
 * - Search performance with large datasets
 * - Memory and resource usage patterns
 * - Response time consistency
 * - System scalability limits
 */

use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tokio::time::sleep;
use uuid::Uuid;

use readur::models::{CreateUser, LoginRequest, LoginResponse, UserRole, DocumentResponse};

const BASE_URL: &str = "http://localhost:8000";
const LOAD_TEST_TIMEOUT: Duration = Duration::from_secs(300); // 5 minutes for load tests

/// Performance metrics tracker
#[derive(Debug, Clone)]
struct PerformanceMetrics {
    total_requests: usize,
    successful_requests: usize,
    failed_requests: usize,
    total_duration: Duration,
    min_response_time: Duration,
    max_response_time: Duration,
    response_times: Vec<Duration>,
}

impl PerformanceMetrics {
    fn new() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            total_duration: Duration::ZERO,
            min_response_time: Duration::from_secs(u64::MAX),
            max_response_time: Duration::ZERO,
            response_times: Vec::new(),
        }
    }
    
    fn add_result(&mut self, success: bool, response_time: Duration) {
        self.total_requests += 1;
        if success {
            self.successful_requests += 1;
        } else {
            self.failed_requests += 1;
        }
        
        self.response_times.push(response_time);
        self.total_duration += response_time;
        
        if response_time < self.min_response_time {
            self.min_response_time = response_time;
        }
        if response_time > self.max_response_time {
            self.max_response_time = response_time;
        }
    }
    
    fn average_response_time(&self) -> Duration {
        if self.total_requests > 0 {
            self.total_duration / self.total_requests as u32
        } else {
            Duration::ZERO
        }
    }
    
    fn percentile(&self, p: f64) -> Duration {
        if self.response_times.is_empty() {
            return Duration::ZERO;
        }
        
        let mut sorted_times = self.response_times.clone();
        sorted_times.sort();
        
        let index = ((sorted_times.len() as f64 - 1.0) * p / 100.0).round() as usize;
        sorted_times[index.min(sorted_times.len() - 1)]
    }
    
    fn success_rate(&self) -> f64 {
        if self.total_requests > 0 {
            self.successful_requests as f64 / self.total_requests as f64
        } else {
            0.0
        }
    }
    
    fn requests_per_second(&self, total_elapsed: Duration) -> f64 {
        if total_elapsed.as_secs_f64() > 0.0 {
            self.total_requests as f64 / total_elapsed.as_secs_f64()
        } else {
            0.0
        }
    }
}

/// Load test client with performance tracking
struct LoadTestClient {
    client: Client,
    token: Option<String>,
    user_id: Option<String>,
}

impl LoadTestClient {
    fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .expect("Failed to create load test client"),
            token: None,
            user_id: None,
        }
    }
    
    /// Setup a test user for load testing
    async fn setup_user(&mut self, user_index: usize) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let username = format!("load_test_user_{}_{}", user_index, timestamp);
        let email = format!("load_test_{}@example.com", timestamp);
        let password = "loadtestpassword123";
        
        // Register user
        let user_data = CreateUser {
            username: username.clone(),
            email: email.clone(),
            password: password.to_string(),
            role: Some(UserRole::User),
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
    
    /// Perform a timed document upload
    async fn timed_upload(&self, content: &str, filename: &str) -> Result<(DocumentResponse, Duration), Box<dyn std::error::Error + Send + Sync>> {
        let start = Instant::now();
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
        
        let elapsed = start.elapsed();
        
        if !response.status().is_success() {
            return Err(format!("Upload failed: {}", response.text().await?).into());
        }
        
        let document: DocumentResponse = response.json().await?;
        Ok((document, elapsed))
    }
    
    /// Perform a timed document list request
    async fn timed_list_documents(&self) -> Result<(Vec<DocumentResponse>, Duration), Box<dyn std::error::Error + Send + Sync>> {
        let start = Instant::now();
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let response = self.client
            .get(&format!("{}/api/documents", BASE_URL))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        let elapsed = start.elapsed();
        
        if !response.status().is_success() {
            return Err(format!("List documents failed: {}", response.text().await?).into());
        }
        
        let documents: Vec<DocumentResponse> = response.json().await?;
        Ok((documents, elapsed))
    }
    
    /// Perform a timed search request
    async fn timed_search(&self, query: &str) -> Result<(Value, Duration), Box<dyn std::error::Error + Send + Sync>> {
        let start = Instant::now();
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let response = self.client
            .get(&format!("{}/api/search", BASE_URL))
            .header("Authorization", format!("Bearer {}", token))
            .query(&[("q", query)])
            .send()
            .await?;
        
        let elapsed = start.elapsed();
        
        if !response.status().is_success() {
            return Err(format!("Search failed: {}", response.text().await?).into());
        }
        
        let results: Value = response.json().await?;
        Ok((results, elapsed))
    }
}

impl Clone for LoadTestClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            token: self.token.clone(),
            user_id: self.user_id.clone(),
        }
    }
}

#[tokio::test]
async fn test_high_volume_document_uploads() {
    println!("📤 Testing high-volume document uploads...");
    
    let mut client = LoadTestClient::new();
    client.setup_user(0).await
        .expect("Failed to setup test user");
    
    let upload_count = 50;
    let concurrent_limit = 10;
    let semaphore = Arc::new(Semaphore::new(concurrent_limit));
    
    let mut metrics = PerformanceMetrics::new();
    let overall_start = Instant::now();
    
    println!("🚀 Starting {} concurrent uploads with limit of {}", upload_count, concurrent_limit);
    
    let mut handles = Vec::new();
    
    for i in 0..upload_count {
        let client_clone = client.clone();
        let semaphore_clone = semaphore.clone();
        
        let handle = tokio::spawn(async move {
            let _permit = semaphore_clone.acquire().await.expect("Failed to acquire semaphore");
            
            let content = format!(
                "Load test document content for upload {}.\n\
                This document contains multiple lines of text to provide meaningful content for OCR processing.\n\
                Generated at: {}\n\
                Document ID: LOAD-TEST-{}\n\
                Content length should be sufficient for testing purposes.",
                i,
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
                Uuid::new_v4()
            );
            let filename = format!("load_test_{}.txt", i);
            
            let result = client_clone.timed_upload(&content, &filename).await;
            
            match result {
                Ok((document, duration)) => (i, true, duration, Some(document.id.to_string())),
                Err(_) => (i, false, Duration::ZERO, None),
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all uploads to complete
    let mut upload_results = Vec::new();
    for handle in handles {
        let result = handle.await.expect("Upload task should complete");
        upload_results.push(result);
    }
    
    let overall_elapsed = overall_start.elapsed();
    
    // Collect metrics
    for (_, success, duration, _) in &upload_results {
        metrics.add_result(*success, *duration);
    }
    
    // Print performance results
    println!("📊 High-Volume Upload Performance Results:");
    println!("  Total uploads: {}", metrics.total_requests);
    println!("  Successful: {}", metrics.successful_requests);
    println!("  Failed: {}", metrics.failed_requests);
    println!("  Success rate: {:.2}%", metrics.success_rate() * 100.0);
    println!("  Total time: {:?}", overall_elapsed);
    println!("  Throughput: {:.2} uploads/sec", metrics.requests_per_second(overall_elapsed));
    println!("  Average response time: {:?}", metrics.average_response_time());
    println!("  Min response time: {:?}", metrics.min_response_time);
    println!("  Max response time: {:?}", metrics.max_response_time);
    println!("  95th percentile: {:?}", metrics.percentile(95.0));
    println!("  99th percentile: {:?}", metrics.percentile(99.0));
    
    // Performance assertions
    assert!(metrics.success_rate() >= 0.9, "Success rate should be at least 90%");
    assert!(metrics.average_response_time() < Duration::from_secs(10), "Average response time should be under 10 seconds");
    assert!(metrics.percentile(95.0) < Duration::from_secs(20), "95th percentile should be under 20 seconds");
    
    println!("🎉 High-volume document uploads test passed!");
}

#[tokio::test]
async fn test_concurrent_user_operations() {
    println!("👥 Testing concurrent user operations...");
    
    let user_count = 10;
    let operations_per_user = 5;
    
    // Setup multiple users
    let mut clients = Vec::new();
    for i in 0..user_count {
        let mut client = LoadTestClient::new();
        client.setup_user(i).await
            .expect(&format!("Failed to setup user {}", i));
        clients.push(client);
    }
    
    println!("✅ Setup {} concurrent users", user_count);
    
    let overall_start = Instant::now();
    let mut all_handles = Vec::new();
    
    // Each user performs multiple operations concurrently
    for (user_index, client) in clients.into_iter().enumerate() {
        let handle = tokio::spawn(async move {
            let mut user_metrics = PerformanceMetrics::new();
            let mut operation_handles = Vec::new();
            
            // Upload documents
            for op_index in 0..operations_per_user {
                let client_clone = client.clone();
                let upload_handle = tokio::spawn(async move {
                    let content = format!("User {} operation {} content", user_index, op_index);
                    let filename = format!("user_{}_op_{}.txt", user_index, op_index);
                    
                    client_clone.timed_upload(&content, &filename).await
                });
                operation_handles.push(upload_handle);
            }
            
            // Wait for all operations for this user
            let mut successful_ops = 0;
            let mut total_ops = 0;
            let mut total_time = Duration::ZERO;
            
            for handle in operation_handles {
                total_ops += 1;
                match handle.await.expect("Operation should complete") {
                    Ok((_, duration)) => {
                        successful_ops += 1;
                        total_time += duration;
                        user_metrics.add_result(true, duration);
                    }
                    Err(_) => {
                        user_metrics.add_result(false, Duration::ZERO);
                    }
                }
            }
            
            (user_index, successful_ops, total_ops, user_metrics)
        });
        
        all_handles.push(handle);
    }
    
    // Wait for all users to complete their operations
    let mut all_user_results = Vec::new();
    for handle in all_handles {
        let result = handle.await.expect("User operations should complete");
        all_user_results.push(result);
    }
    
    let overall_elapsed = overall_start.elapsed();
    
    // Aggregate metrics across all users
    let mut global_metrics = PerformanceMetrics::new();
    for (user_index, successful_ops, total_ops, user_metrics) in &all_user_results {
        println!("  User {}: {}/{} operations successful", user_index, successful_ops, total_ops);
        
        // Merge user metrics into global metrics
        for &response_time in &user_metrics.response_times {
            global_metrics.add_result(true, response_time);
        }
        global_metrics.failed_requests += user_metrics.failed_requests;
    }
    
    println!("📊 Concurrent User Operations Performance Results:");
    println!("  Total users: {}", user_count);
    println!("  Operations per user: {}", operations_per_user);
    println!("  Total operations: {}", global_metrics.total_requests + global_metrics.failed_requests);
    println!("  Successful operations: {}", global_metrics.successful_requests);
    println!("  Failed operations: {}", global_metrics.failed_requests);
    println!("  Overall success rate: {:.2}%", global_metrics.success_rate() * 100.0);
    println!("  Total time: {:?}", overall_elapsed);
    println!("  Throughput: {:.2} operations/sec", global_metrics.requests_per_second(overall_elapsed));
    println!("  Average response time: {:?}", global_metrics.average_response_time());
    println!("  95th percentile: {:?}", global_metrics.percentile(95.0));
    
    // Performance assertions
    assert!(global_metrics.success_rate() >= 0.8, "Success rate should be at least 80% under load");
    assert!(global_metrics.average_response_time() < Duration::from_secs(15), "Average response time should be reasonable under load");
    
    println!("🎉 Concurrent user operations test passed!");
}

#[tokio::test]
async fn test_search_performance_with_load() {
    println!("🔍 Testing search performance under load...");
    
    let mut client = LoadTestClient::new();
    client.setup_user(0).await
        .expect("Failed to setup test user");
    
    // First, upload several documents to create a searchable dataset
    let document_count = 20;
    println!("📤 Creating dataset with {} documents...", document_count);
    
    let mut document_ids = Vec::new();
    for i in 0..document_count {
        let content = format!(
            "Document {} for search performance testing.\n\
            This document contains searchable keywords like: performance, test, document, search, load.\n\
            Additional content: technology, system, user, data, processing.\n\
            Unique identifier: SEARCH-PERF-{}\n\
            Number: {}",
            i, Uuid::new_v4(), i
        );
        let filename = format!("search_perf_doc_{}.txt", i);
        
        match client.timed_upload(&content, &filename).await {
            Ok((document, _)) => {
                document_ids.push(document.id.to_string());
            }
            Err(e) => {
                println!("⚠️  Failed to upload document {}: {}", i, e);
            }
        }
    }
    
    println!("✅ Created dataset with {} documents", document_ids.len());
    
    // Wait a moment for documents to be indexed
    sleep(Duration::from_secs(5)).await;
    
    // Perform multiple search queries concurrently
    let search_queries = vec![
        "performance",
        "test document",
        "search load",
        "technology system",
        "user data",
        "processing",
        "unique identifier",
        "SEARCH-PERF",
    ];
    
    let searches_per_query = 5;
    let mut search_metrics = PerformanceMetrics::new();
    let search_start = Instant::now();
    
    let mut search_handles = Vec::new();
    
    for (query_index, query) in search_queries.iter().enumerate() {
        for search_index in 0..searches_per_query {
            let client_clone = client.clone();
            let query_clone = query.to_string();
            
            let handle = tokio::spawn(async move {
                let result = client_clone.timed_search(&query_clone).await;
                
                match result {
                    Ok((results, duration)) => {
                        let result_count = results["documents"].as_array()
                            .map(|arr| arr.len())
                            .unwrap_or(0);
                        (query_index, search_index, true, duration, result_count)
                    }
                    Err(_) => (query_index, search_index, false, Duration::ZERO, 0),
                }
            });
            
            search_handles.push(handle);
        }
    }
    
    // Wait for all search operations to complete
    let mut search_results = Vec::new();
    for handle in search_handles {
        let result = handle.await.expect("Search task should complete");
        search_results.push(result);
    }
    
    let search_elapsed = search_start.elapsed();
    
    // Collect search metrics
    for (_, _, success, duration, result_count) in &search_results {
        search_metrics.add_result(*success, *duration);
        if *success {
            println!("  Search returned {} results in {:?}", result_count, duration);
        }
    }
    
    println!("📊 Search Performance Results:");
    println!("  Total searches: {}", search_metrics.total_requests);
    println!("  Successful searches: {}", search_metrics.successful_requests);
    println!("  Failed searches: {}", search_metrics.failed_requests);
    println!("  Success rate: {:.2}%", search_metrics.success_rate() * 100.0);
    println!("  Total time: {:?}", search_elapsed);
    println!("  Search throughput: {:.2} searches/sec", search_metrics.requests_per_second(search_elapsed));
    println!("  Average search time: {:?}", search_metrics.average_response_time());
    println!("  Min search time: {:?}", search_metrics.min_response_time);
    println!("  Max search time: {:?}", search_metrics.max_response_time);
    println!("  95th percentile: {:?}", search_metrics.percentile(95.0));
    
    // Performance assertions for search
    assert!(search_metrics.success_rate() >= 0.9, "Search success rate should be at least 90%");
    assert!(search_metrics.average_response_time() < Duration::from_secs(5), "Average search time should be under 5 seconds");
    assert!(search_metrics.percentile(95.0) < Duration::from_secs(10), "95th percentile search time should be under 10 seconds");
    
    println!("🎉 Search performance under load test passed!");
}

#[tokio::test]
async fn test_database_query_performance() {
    println!("🗄️ Testing database query performance...");
    
    let mut client = LoadTestClient::new();
    client.setup_user(0).await
        .expect("Failed to setup test user");
    
    // Test repeated document list queries to stress database
    let query_count = 100;
    let concurrent_queries = 20;
    let semaphore = Arc::new(Semaphore::new(concurrent_queries));
    
    let mut query_metrics = PerformanceMetrics::new();
    let query_start = Instant::now();
    
    println!("🚀 Starting {} database queries with concurrency {}", query_count, concurrent_queries);
    
    let mut query_handles = Vec::new();
    
    for i in 0..query_count {
        let client_clone = client.clone();
        let semaphore_clone = semaphore.clone();
        
        let handle = tokio::spawn(async move {
            let _permit = semaphore_clone.acquire().await.expect("Failed to acquire semaphore");
            
            let result = client_clone.timed_list_documents().await;
            
            match result {
                Ok((documents, duration)) => (i, true, duration, documents.len()),
                Err(_) => (i, false, Duration::ZERO, 0),
            }
        });
        
        query_handles.push(handle);
    }
    
    // Wait for all queries to complete
    let mut query_results = Vec::new();
    for handle in query_handles {
        let result = handle.await.expect("Query task should complete");
        query_results.push(result);
    }
    
    let query_elapsed = query_start.elapsed();
    
    // Collect query metrics
    for (_, success, duration, doc_count) in &query_results {
        query_metrics.add_result(*success, *duration);
        if *success && doc_count > &0 {
            println!("  Query returned {} documents in {:?}", doc_count, duration);
        }
    }
    
    println!("📊 Database Query Performance Results:");
    println!("  Total queries: {}", query_metrics.total_requests);
    println!("  Successful queries: {}", query_metrics.successful_requests);
    println!("  Failed queries: {}", query_metrics.failed_requests);
    println!("  Success rate: {:.2}%", query_metrics.success_rate() * 100.0);
    println!("  Total time: {:?}", query_elapsed);
    println!("  Query throughput: {:.2} queries/sec", query_metrics.requests_per_second(query_elapsed));
    println!("  Average query time: {:?}", query_metrics.average_response_time());
    println!("  Min query time: {:?}", query_metrics.min_response_time);
    println!("  Max query time: {:?}", query_metrics.max_response_time);
    println!("  95th percentile: {:?}", query_metrics.percentile(95.0));
    println!("  99th percentile: {:?}", query_metrics.percentile(99.0));
    
    // Performance assertions for database queries
    assert!(query_metrics.success_rate() >= 0.95, "Database query success rate should be at least 95%");
    assert!(query_metrics.average_response_time() < Duration::from_secs(2), "Average query time should be under 2 seconds");
    assert!(query_metrics.percentile(95.0) < Duration::from_secs(5), "95th percentile query time should be under 5 seconds");
    
    println!("🎉 Database query performance test passed!");
}

#[tokio::test]
async fn test_system_stability_under_sustained_load() {
    println!("🔄 Testing system stability under sustained load...");
    
    let mut client = LoadTestClient::new();
    client.setup_user(0).await
        .expect("Failed to setup test user");
    
    let test_duration = Duration::from_secs(60); // 1 minute sustained load
    let operation_interval = Duration::from_millis(500); // Operation every 500ms
    
    let mut stability_metrics = PerformanceMetrics::new();
    let stability_start = Instant::now();
    
    println!("⏳ Running sustained load for {:?} with operations every {:?}", test_duration, operation_interval);
    
    let mut operation_counter = 0;
    let mut response_time_samples = Vec::new();
    
    while stability_start.elapsed() < test_duration {
        let operation_start = Instant::now();
        
        // Alternate between different operation types
        let operation_result = match operation_counter % 3 {
            0 => {
                // Document list operation
                client.timed_list_documents().await
                    .map(|(docs, duration)| (format!("list({} docs)", docs.len()), duration))
            }
            1 => {
                // Document upload operation
                let content = format!("Stability test document {}", operation_counter);
                let filename = format!("stability_{}.txt", operation_counter);
                client.timed_upload(&content, &filename).await
                    .map(|(doc, duration)| (format!("upload({})", doc.id), duration))
            }
            _ => {
                // Search operation
                let queries = ["test", "document", "stability"];
                let query = queries[operation_counter % queries.len()];
                client.timed_search(query).await
                    .map(|(results, duration)| {
                        let count = results["documents"].as_array().map(|a| a.len()).unwrap_or(0);
                        (format!("search({} results)", count), duration)
                    })
            }
        };
        
        let operation_elapsed = operation_start.elapsed();
        
        match operation_result {
            Ok((operation_desc, response_time)) => {
                stability_metrics.add_result(true, response_time);
                response_time_samples.push((stability_start.elapsed(), response_time));
                println!("  {:?}: {} completed in {:?}", 
                         stability_start.elapsed(), operation_desc, response_time);
            }
            Err(e) => {
                stability_metrics.add_result(false, operation_elapsed);
                println!("  {:?}: Operation failed: {}", stability_start.elapsed(), e);
            }
        }
        
        operation_counter += 1;
        
        // Sleep to maintain operation interval
        if operation_elapsed < operation_interval {
            sleep(operation_interval - operation_elapsed).await;
        }
    }
    
    let total_elapsed = stability_start.elapsed();
    
    // Analyze stability over time
    let sample_windows = 6; // Divide test into 6 windows
    let window_duration = test_duration / sample_windows as u32;
    
    println!("📊 System Stability Results:");
    println!("  Test duration: {:?}", total_elapsed);
    println!("  Total operations: {}", stability_metrics.total_requests);
    println!("  Successful operations: {}", stability_metrics.successful_requests);
    println!("  Failed operations: {}", stability_metrics.failed_requests);
    println!("  Overall success rate: {:.2}%", stability_metrics.success_rate() * 100.0);
    println!("  Average throughput: {:.2} ops/sec", stability_metrics.requests_per_second(total_elapsed));
    println!("  Average response time: {:?}", stability_metrics.average_response_time());
    
    // Analyze response time stability across windows
    for window in 0..sample_windows {
        let window_start = window_duration * window as u32;
        let window_end = window_duration * (window + 1) as u32;
        
        let window_samples: Vec<_> = response_time_samples.iter()
            .filter(|(elapsed, _)| *elapsed >= window_start && *elapsed < window_end)
            .map(|(_, duration)| *duration)
            .collect();
        
        if !window_samples.is_empty() {
            let window_avg = window_samples.iter().sum::<Duration>() / window_samples.len() as u32;
            println!("  Window {} ({:?}-{:?}): {} ops, avg {:?}", 
                     window + 1, window_start, window_end, window_samples.len(), window_avg);
        }
    }
    
    // Stability assertions
    assert!(stability_metrics.success_rate() >= 0.8, "Success rate should remain above 80% under sustained load");
    assert!(operation_counter >= 100, "Should complete at least 100 operations during stability test");
    
    println!("🎉 System stability under sustained load test passed!");
}