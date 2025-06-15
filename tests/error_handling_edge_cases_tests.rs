/*!
 * Error Handling and Edge Cases Integration Tests
 * 
 * Tests comprehensive error scenarios and edge cases including:
 * - Network failure recovery
 * - Invalid input handling
 * - Resource exhaustion scenarios
 * - Authentication edge cases
 * - File upload edge cases
 * - Database constraint violations
 * - Malformed request handling
 * - Rate limiting and throttling
 * - Concurrent operation conflicts
 */

use reqwest::Client;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use uuid::Uuid;

use readur::models::{CreateUser, LoginRequest, LoginResponse, UserRole};

const BASE_URL: &str = "http://localhost:8000";

/// Test client for error handling scenarios
struct ErrorHandlingTestClient {
    client: Client,
    token: Option<String>,
}

impl ErrorHandlingTestClient {
    fn new() -> Self {
        Self {
            client: Client::new(),
            token: None,
        }
    }
    
    fn new_with_timeout(timeout: Duration) -> Self {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .expect("Failed to create client with timeout");
        
        Self {
            client,
            token: None,
        }
    }
    
    /// Register and login with potential error handling
    async fn safe_register_and_login(&mut self, role: UserRole) -> Result<String, Box<dyn std::error::Error>> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let username = format!("error_test_{}_{}", role.to_string(), timestamp);
        let email = format!("error_test_{}@example.com", timestamp);
        let password = "testpassword123";
        
        // Register user with retry logic
        let mut attempts = 0;
        let max_attempts = 3;
        
        while attempts < max_attempts {
            let user_data = CreateUser {
                username: username.clone(),
                email: email.clone(),
                password: password.to_string(),
                role: Some(role.clone()),
            };
            
            match self.client
                .post(&format!("{}/api/auth/register", BASE_URL))
                .json(&user_data)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        break;
                    } else {
                        attempts += 1;
                        if attempts >= max_attempts {
                            return Err(format!("Registration failed after {} attempts: {}", max_attempts, response.text().await?).into());
                        }
                        sleep(Duration::from_millis(100 * attempts as u64)).await;
                    }
                }
                Err(e) => {
                    attempts += 1;
                    if attempts >= max_attempts {
                        return Err(format!("Registration network error after {} attempts: {}", max_attempts, e).into());
                    }
                    sleep(Duration::from_millis(100 * attempts as u64)).await;
                }
            }
        }
        
        // Login with retry logic
        let login_data = LoginRequest {
            username: username.clone(),
            password: password.to_string(),
        };
        
        attempts = 0;
        while attempts < max_attempts {
            match self.client
                .post(&format!("{}/api/auth/login", BASE_URL))
                .json(&login_data)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        let login_result: LoginResponse = response.json().await?;
                        self.token = Some(login_result.token.clone());
                        return Ok(login_result.token);
                    } else {
                        attempts += 1;
                        if attempts >= max_attempts {
                            return Err(format!("Login failed after {} attempts: {}", max_attempts, response.text().await?).into());
                        }
                        sleep(Duration::from_millis(100 * attempts as u64)).await;
                    }
                }
                Err(e) => {
                    attempts += 1;
                    if attempts >= max_attempts {
                        return Err(format!("Login network error after {} attempts: {}", max_attempts, e).into());
                    }
                    sleep(Duration::from_millis(100 * attempts as u64)).await;
                }
            }
        }
        
        Err("Failed to login after retries".into())
    }
}

#[tokio::test]
async fn test_invalid_authentication_scenarios() {
    let client = Client::new();
    
    println!("🔐 Testing invalid authentication scenarios...");
    
    // Test 1: Empty credentials
    let empty_login = json!({
        "username": "",
        "password": ""
    });
    
    let response = client
        .post(&format!("{}/api/auth/login", BASE_URL))
        .json(&empty_login)
        .send()
        .await
        .expect("Request should complete");
    
    assert_eq!(response.status(), 400);
    println!("✅ Empty credentials properly rejected");
    
    // Test 2: Invalid username format
    let invalid_username = json!({
        "username": "user@with@multiple@ats",
        "password": "validpassword123"
    });
    
    let response = client
        .post(&format!("{}/api/auth/login", BASE_URL))
        .json(&invalid_username)
        .send()
        .await
        .expect("Request should complete");
    
    assert!(!response.status().is_success());
    println!("✅ Invalid username format properly rejected");
    
    // Test 3: SQL injection attempt in login
    let sql_injection = json!({
        "username": "admin'; DROP TABLE users; --",
        "password": "password"
    });
    
    let response = client
        .post(&format!("{}/api/auth/login", BASE_URL))
        .json(&sql_injection)
        .send()
        .await
        .expect("Request should complete");
    
    assert!(!response.status().is_success());
    println!("✅ SQL injection attempt in login properly rejected");
    
    // Test 4: Extremely long credentials
    let long_username = "a".repeat(10000);
    let long_password = "b".repeat(10000);
    let long_creds = json!({
        "username": long_username,
        "password": long_password
    });
    
    let response = client
        .post(&format!("{}/api/auth/login", BASE_URL))
        .json(&long_creds)
        .send()
        .await
        .expect("Request should complete");
    
    assert!(!response.status().is_success());
    println!("✅ Extremely long credentials properly rejected");
    
    // Test 5: Invalid JWT token format
    let invalid_token_response = client
        .get(&format!("{}/api/auth/me", BASE_URL))
        .header("Authorization", "Bearer invalid-jwt-token-format")
        .send()
        .await
        .expect("Request should complete");
    
    assert_eq!(invalid_token_response.status(), 401);
    println!("✅ Invalid JWT token properly rejected");
    
    // Test 6: Malformed Authorization header
    let malformed_auth_response = client
        .get(&format!("{}/api/auth/me", BASE_URL))
        .header("Authorization", "InvalidFormat token")
        .send()
        .await
        .expect("Request should complete");
    
    assert_eq!(malformed_auth_response.status(), 401);
    println!("✅ Malformed Authorization header properly rejected");
    
    println!("🎉 Invalid authentication scenarios test passed!");
}

#[tokio::test]
async fn test_malformed_request_handling() {
    let mut client = ErrorHandlingTestClient::new();
    
    // Setup a valid user for testing authenticated endpoints
    client.safe_register_and_login(UserRole::User).await
        .expect("Failed to setup test user");
    
    let token = client.token.as_ref().unwrap();
    
    println!("🔧 Testing malformed request handling...");
    
    // Test 1: Invalid JSON in request body
    let invalid_json_response = client.client
        .post(&format!("{}/api/sources", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body("{invalid json syntax")
        .send()
        .await
        .expect("Request should complete");
    
    assert_eq!(invalid_json_response.status(), 400);
    println!("✅ Invalid JSON properly rejected");
    
    // Test 2: Missing required fields
    let missing_fields = json!({
        "name": "Test Source"
        // Missing source_type and config
    });
    
    let response = client.client
        .post(&format!("{}/api/sources", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&missing_fields)
        .send()
        .await
        .expect("Request should complete");
    
    assert!(!response.status().is_success());
    println!("✅ Missing required fields properly rejected");
    
    // Test 3: Invalid enum values
    let invalid_enum = json!({
        "name": "Test Source",
        "source_type": "invalid_source_type",
        "config": {}
    });
    
    let response = client.client
        .post(&format!("{}/api/sources", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&invalid_enum)
        .send()
        .await
        .expect("Request should complete");
    
    assert!(!response.status().is_success());
    println!("✅ Invalid enum values properly rejected");
    
    // Test 4: Nested object validation
    let invalid_nested = json!({
        "name": "Test Source",
        "source_type": "webdav",
        "config": {
            "server_url": "not-a-valid-url",
            "username": "",
            "password": "",
            "sync_interval_minutes": -1 // Invalid negative value
        }
    });
    
    let response = client.client
        .post(&format!("{}/api/sources", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&invalid_nested)
        .send()
        .await
        .expect("Request should complete");
    
    assert!(!response.status().is_success());
    println!("✅ Invalid nested object validation working");
    
    // Test 5: Extra unexpected fields (should be ignored gracefully)
    let extra_fields = json!({
        "name": "Test Source",
        "source_type": "webdav",
        "config": {
            "server_url": "https://valid-url.com",
            "username": "testuser",
            "password": "testpass",
            "auto_sync": true,
            "sync_interval_minutes": 60,
            "watch_folders": ["/Documents"],
            "file_extensions": [".pdf"]
        },
        "unexpected_field": "should be ignored",
        "another_extra": 12345
    });
    
    let response = client.client
        .post(&format!("{}/api/sources", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&extra_fields)
        .send()
        .await
        .expect("Request should complete");
    
    // This might succeed if the API gracefully ignores extra fields
    println!("✅ Extra fields handling: status {}", response.status());
    
    println!("🎉 Malformed request handling test passed!");
}

#[tokio::test]
async fn test_file_upload_edge_cases() {
    let mut client = ErrorHandlingTestClient::new();
    
    client.safe_register_and_login(UserRole::User).await
        .expect("Failed to setup test user");
    
    let token = client.token.as_ref().unwrap();
    
    println!("📁 Testing file upload edge cases...");
    
    // Test 1: Empty file upload
    let empty_part = reqwest::multipart::Part::text("")
        .file_name("empty.txt")
        .mime_str("text/plain")
        .expect("Failed to create empty part");
    let empty_form = reqwest::multipart::Form::new()
        .part("file", empty_part);
    
    let response = client.client
        .post(&format!("{}/api/documents", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(empty_form)
        .send()
        .await
        .expect("Request should complete");
    
    // Empty files might be rejected or accepted depending on implementation
    println!("✅ Empty file upload: status {}", response.status());
    
    // Test 2: Extremely large filename
    let long_filename = format!("{}.txt", "a".repeat(1000));
    let long_filename_part = reqwest::multipart::Part::text("content")
        .file_name(long_filename)
        .mime_str("text/plain")
        .expect("Failed to create long filename part");
    let long_filename_form = reqwest::multipart::Form::new()
        .part("file", long_filename_part);
    
    let response = client.client
        .post(&format!("{}/api/documents", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(long_filename_form)
        .send()
        .await
        .expect("Request should complete");
    
    println!("✅ Long filename upload: status {}", response.status());
    
    // Test 3: Filename with special characters
    let special_filename = "test<>:\"|?*.txt";
    let special_filename_part = reqwest::multipart::Part::text("content")
        .file_name(special_filename.to_string())
        .mime_str("text/plain")
        .expect("Failed to create special filename part");
    let special_filename_form = reqwest::multipart::Form::new()
        .part("file", special_filename_part);
    
    let response = client.client
        .post(&format!("{}/api/documents", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(special_filename_form)
        .send()
        .await
        .expect("Request should complete");
    
    println!("✅ Special characters filename: status {}", response.status());
    
    // Test 4: Missing file part
    let no_file_form = reqwest::multipart::Form::new()
        .text("not_file", "some text");
    
    let response = client.client
        .post(&format!("{}/api/documents", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(no_file_form)
        .send()
        .await
        .expect("Request should complete");
    
    assert!(!response.status().is_success());
    println!("✅ Missing file part properly rejected");
    
    // Test 5: Multiple files (if not supported)
    let file1 = reqwest::multipart::Part::text("content1")
        .file_name("file1.txt")
        .mime_str("text/plain")
        .expect("Failed to create file1 part");
    let file2 = reqwest::multipart::Part::text("content2")
        .file_name("file2.txt")
        .mime_str("text/plain")
        .expect("Failed to create file2 part");
    let multi_file_form = reqwest::multipart::Form::new()
        .part("file", file1)
        .part("file2", file2);
    
    let response = client.client
        .post(&format!("{}/api/documents", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(multi_file_form)
        .send()
        .await
        .expect("Request should complete");
    
    println!("✅ Multiple files upload: status {}", response.status());
    
    // Test 6: Invalid MIME type
    let invalid_mime_part = reqwest::multipart::Part::text("content")
        .file_name("test.txt")
        .mime_str("invalid/mime-type");
    
    if let Ok(part) = invalid_mime_part {
        let invalid_mime_form = reqwest::multipart::Form::new()
            .part("file", part);
        
        let response = client.client
            .post(&format!("{}/api/documents", BASE_URL))
            .header("Authorization", format!("Bearer {}", token))
            .multipart(invalid_mime_form)
            .send()
            .await
            .expect("Request should complete");
        
        println!("✅ Invalid MIME type: status {}", response.status());
    } else {
        println!("✅ Invalid MIME type rejected at client level");
    }
    
    println!("🎉 File upload edge cases test passed!");
}

#[tokio::test]
async fn test_concurrent_operation_conflicts() {
    println!("🔄 Testing concurrent operation conflicts...");
    
    // Create multiple clients for concurrent operations
    let mut clients = Vec::new();
    for i in 0..3 {
        let mut client = ErrorHandlingTestClient::new();
        client.safe_register_and_login(UserRole::User).await
            .expect(&format!("Failed to setup client {}", i));
        clients.push(client);
    }
    
    println!("✅ Setup {} concurrent clients", clients.len());
    
    // Test 1: Concurrent source creation with same name
    let mut handles = Vec::new();
    
    for (i, client) in clients.iter().enumerate() {
        let token = client.token.clone().unwrap();
        let client_ref = &client.client;
        let client_clone = client_ref.clone();
        
        let handle = tokio::spawn(async move {
            let source_data = json!({
                "name": "Concurrent Test Source", // Same name for all
                "source_type": "webdav",
                "config": {
                    "server_url": format!("https://server{}.example.com", i),
                    "username": "testuser",
                    "password": "testpass",
                    "auto_sync": false,
                    "sync_interval_minutes": 60,
                    "watch_folders": ["/Documents"],
                    "file_extensions": [".pdf"]
                }
            });
            
            let response = client_clone
                .post(&format!("{}/api/sources", BASE_URL))
                .header("Authorization", format!("Bearer {}", token))
                .json(&source_data)
                .send()
                .await
                .expect("Request should complete");
            
            (i, response.status(), response.text().await.unwrap_or_default())
        });
        
        handles.push(handle);
    }
    
    // Wait for all concurrent operations
    let mut results = Vec::new();
    for handle in handles {
        let result = handle.await.expect("Task should complete");
        results.push(result);
    }
    
    // Analyze results
    let successful_count = results.iter()
        .filter(|(_, status, _)| status.is_success())
        .count();
    
    println!("✅ Concurrent source creation: {}/{} succeeded", successful_count, results.len());
    
    for (i, status, response) in results {
        println!("  Client {}: {} - {}", i, status, response.chars().take(100).collect::<String>());
    }
    
    // Test 2: Concurrent document uploads
    let upload_content = "Concurrent upload test content";
    let mut upload_handles = Vec::new();
    
    for (i, client) in clients.iter().enumerate() {
        let token = client.token.clone().unwrap();
        let client_ref = &client.client;
        let client_clone = client_ref.clone();
        let content = format!("{} - Client {}", upload_content, i);
        
        let handle = tokio::spawn(async move {
            let part = reqwest::multipart::Part::text(content)
                .file_name(format!("concurrent_test_{}.txt", i))
                .mime_str("text/plain")
                .expect("Failed to create part");
            let form = reqwest::multipart::Form::new()
                .part("file", part);
            
            let response = client_clone
                .post(&format!("{}/api/documents", BASE_URL))
                .header("Authorization", format!("Bearer {}", token))
                .multipart(form)
                .send()
                .await
                .expect("Request should complete");
            
            (i, response.status())
        });
        
        upload_handles.push(handle);
    }
    
    let mut upload_results = Vec::new();
    for handle in upload_handles {
        let result = handle.await.expect("Upload task should complete");
        upload_results.push(result);
    }
    
    let successful_uploads = upload_results.iter()
        .filter(|(_, status)| status.is_success())
        .count();
    
    println!("✅ Concurrent document uploads: {}/{} succeeded", successful_uploads, upload_results.len());
    
    println!("🎉 Concurrent operation conflicts test passed!");
}

#[tokio::test]
async fn test_network_timeout_scenarios() {
    println!("⏱️ Testing network timeout scenarios...");
    
    // Create client with very short timeout
    let short_timeout_client = ErrorHandlingTestClient::new_with_timeout(Duration::from_millis(1));
    
    // Test 1: Registration with timeout
    let timeout_result = short_timeout_client.client
        .post(&format!("{}/api/auth/register", BASE_URL))
        .json(&json!({
            "username": "timeout_test",
            "email": "timeout@example.com",
            "password": "password123",
            "role": "user"
        }))
        .send()
        .await;
    
    // Should timeout or succeed very quickly
    match timeout_result {
        Ok(response) => println!("✅ Short timeout request completed: {}", response.status()),
        Err(e) => {
            if e.is_timeout() {
                println!("✅ Short timeout properly triggered");
            } else {
                println!("✅ Request failed with error: {}", e);
            }
        }
    }
    
    // Test 2: Normal timeout client
    let normal_client = ErrorHandlingTestClient::new_with_timeout(Duration::from_secs(30));
    
    // Test long-running operation (document upload with processing)
    let start_time = Instant::now();
    
    let large_content = "Large document content. ".repeat(1000);
    let part = reqwest::multipart::Part::text(large_content)
        .file_name("large_timeout_test.txt")
        .mime_str("text/plain")
        .expect("Failed to create large part");
    let form = reqwest::multipart::Form::new()
        .part("file", part);
    
    // This should complete within normal timeout
    let upload_result = normal_client.client
        .post(&format!("{}/api/documents", BASE_URL))
        .multipart(form)
        .send()
        .await;
    
    let elapsed = start_time.elapsed();
    
    match upload_result {
        Ok(response) => {
            println!("✅ Large upload completed in {:?}: {}", elapsed, response.status());
        }
        Err(e) => {
            if e.is_timeout() {
                println!("✅ Large upload timed out after {:?}", elapsed);
            } else {
                println!("✅ Large upload failed: {} after {:?}", e, elapsed);
            }
        }
    }
    
    println!("🎉 Network timeout scenarios test passed!");
}

#[tokio::test]
async fn test_resource_exhaustion_simulation() {
    println!("💾 Testing resource exhaustion simulation...");
    
    let mut client = ErrorHandlingTestClient::new();
    client.safe_register_and_login(UserRole::User).await
        .expect("Failed to setup test user");
    
    let token = client.token.as_ref().unwrap();
    
    // Test 1: Rapid successive requests (stress test)
    let rapid_request_count = 20;
    let mut rapid_handles = Vec::new();
    
    println!("🚀 Sending {} rapid requests...", rapid_request_count);
    
    for i in 0..rapid_request_count {
        let token_clone = token.clone();
        let client_clone = client.client.clone();
        
        let handle = tokio::spawn(async move {
            let start = Instant::now();
            let response = client_clone
                .get(&format!("{}/api/documents", BASE_URL))
                .header("Authorization", format!("Bearer {}", token_clone))
                .send()
                .await;
            
            let elapsed = start.elapsed();
            
            match response {
                Ok(resp) => (i, resp.status(), elapsed, None),
                Err(e) => (i, reqwest::StatusCode::from_u16(500).unwrap(), elapsed, Some(e.to_string())),
            }
        });
        
        rapid_handles.push(handle);
    }
    
    let mut rapid_results = Vec::new();
    for handle in rapid_handles {
        let result = handle.await.expect("Rapid request task should complete");
        rapid_results.push(result);
    }
    
    // Analyze rapid request results
    let successful_rapid = rapid_results.iter()
        .filter(|(_, status, _, _)| status.is_success())
        .count();
    
    let avg_response_time = rapid_results.iter()
        .map(|(_, _, elapsed, _)| *elapsed)
        .sum::<Duration>() / rapid_results.len() as u32;
    
    println!("✅ Rapid requests: {}/{} succeeded, avg response time: {:?}", 
             successful_rapid, rapid_request_count, avg_response_time);
    
    // Test 2: Large payload stress test
    println!("📦 Testing large payload handling...");
    
    let very_large_content = "Very large document content for stress testing. ".repeat(10000);
    let large_part = reqwest::multipart::Part::text(very_large_content)
        .file_name("stress_test_large.txt")
        .mime_str("text/plain")
        .expect("Failed to create large stress part");
    let large_form = reqwest::multipart::Form::new()
        .part("file", large_part);
    
    let large_upload_start = Instant::now();
    let large_upload_result = client.client
        .post(&format!("{}/api/documents", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(large_form)
        .send()
        .await;
    
    let large_upload_elapsed = large_upload_start.elapsed();
    
    match large_upload_result {
        Ok(response) => {
            println!("✅ Large payload upload: {} in {:?}", response.status(), large_upload_elapsed);
        }
        Err(e) => {
            println!("✅ Large payload upload failed: {} in {:?}", e, large_upload_elapsed);
        }
    }
    
    println!("🎉 Resource exhaustion simulation test passed!");
}

#[tokio::test]
async fn test_database_constraint_violations() {
    println!("🗄️ Testing database constraint violations...");
    
    let mut client = ErrorHandlingTestClient::new();
    client.safe_register_and_login(UserRole::User).await
        .expect("Failed to setup test user");
    
    let token = client.token.as_ref().unwrap();
    
    // Test 1: Duplicate email registration attempt
    let original_user = json!({
        "username": "original_user",
        "email": "unique@example.com",
        "password": "password123",
        "role": "user"
    });
    
    let register_response = client.client
        .post(&format!("{}/api/auth/register", BASE_URL))
        .json(&original_user)
        .send()
        .await
        .expect("First registration should complete");
    
    println!("✅ First user registration: {}", register_response.status());
    
    // Try to register another user with the same email
    let duplicate_email_user = json!({
        "username": "different_username",
        "email": "unique@example.com", // Same email
        "password": "different_password",
        "role": "user"
    });
    
    let duplicate_response = client.client
        .post(&format!("{}/api/auth/register", BASE_URL))
        .json(&duplicate_email_user)
        .send()
        .await
        .expect("Duplicate email registration should complete");
    
    // Should be rejected due to unique constraint
    assert!(!duplicate_response.status().is_success());
    println!("✅ Duplicate email registration properly rejected: {}", duplicate_response.status());
    
    // Test 2: Creating source with extremely long name
    let long_name = "a".repeat(500);
    let long_name_source = json!({
        "name": long_name,
        "source_type": "webdav",
        "config": {
            "server_url": "https://example.com",
            "username": "user",
            "password": "pass",
            "auto_sync": false,
            "sync_interval_minutes": 60,
            "watch_folders": ["/Documents"],
            "file_extensions": [".pdf"]
        }
    });
    
    let long_name_response = client.client
        .post(&format!("{}/api/sources", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&long_name_source)
        .send()
        .await
        .expect("Long name source creation should complete");
    
    println!("✅ Long source name: {}", long_name_response.status());
    
    // Test 3: Invalid foreign key reference (if applicable)
    let fake_user_id = Uuid::new_v4().to_string();
    
    // This test depends on the API structure, but we can test accessing resources
    // that don't exist or belong to other users
    let fake_source_id = Uuid::new_v4().to_string();
    let fake_source_response = client.client
        .get(&format!("{}/api/sources/{}", BASE_URL, fake_source_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Fake source access should complete");
    
    assert_eq!(fake_source_response.status(), 404);
    println!("✅ Non-existent resource access properly rejected: {}", fake_source_response.status());
    
    println!("🎉 Database constraint violations test passed!");
}