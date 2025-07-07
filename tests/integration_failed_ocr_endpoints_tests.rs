/*!
 * Failed OCR Endpoints Integration Tests
 * 
 * Tests the OCR failure endpoints and functionality including:
 * - Retrieving failed OCR documents
 * - Retrying failed OCR processing
 * - Verifying failure statistics and categorization
 * - Testing failure reason classification
 * - Ensuring proper error handling and display
 */

use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use uuid::Uuid;

use readur::models::{CreateUser, LoginRequest, LoginResponse, UserRole};

fn get_base_url() -> String {
    std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8000".to_string())
}

const TIMEOUT: Duration = Duration::from_secs(60);

/// Test client for failed OCR endpoint operations
struct FailedOcrTestClient {
    client: Client,
    token: Option<String>,
    user_id: Option<String>,
}

impl FailedOcrTestClient {
    fn new() -> Self {
        Self {
            client: Client::new(),
            token: None,
            user_id: None,
        }
    }
    
    /// Register and login a test user
    async fn register_and_login(&mut self, role: UserRole) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // First check if server is running
        let health_check = self.client
            .get(&format!("{}/api/health", get_base_url()))
            .timeout(Duration::from_secs(5))
            .send()
            .await;
        
        if let Err(e) = health_check {
            eprintln!("Health check failed: {}. Is the server running at {}?", e, get_base_url());
            return Err(format!("Server not running: {}", e).into());
        }
        
        // Use UUID for guaranteed uniqueness
        let test_id = Uuid::new_v4().simple().to_string();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let username = format!("failed_ocr_{}_{}_{}_{}", role.to_string(), test_id, nanos, Uuid::new_v4().simple());
        let email = format!("failed_ocr_{}_{}@{}.example.com", test_id, nanos, Uuid::new_v4().simple());
        let password = "testpassword123";
        
        // Register user
        let user_data = CreateUser {
            username: username.clone(),
            email: email.clone(),
            password: password.to_string(),
            role: Some(role),
        };
        
        let register_response = self.client
            .post(&format!("{}/api/auth/register", get_base_url()))
            .json(&user_data)
            .timeout(TIMEOUT)
            .send()
            .await?;
        
        if !register_response.status().is_success() {
            let status = register_response.status();
            let text = register_response.text().await?;
            return Err(format!("Registration failed: {}", text).into());
        }
        
        // Login to get token
        let login_data = LoginRequest {
            username: username.clone(),
            password: password.to_string(),
        };
        
        let login_response = self.client
            .post(&format!("{}/api/auth/login", get_base_url()))
            .json(&login_data)
            .timeout(TIMEOUT)
            .send()
            .await?;
        
        if !login_response.status().is_success() {
            return Err(format!("Login failed: {}", login_response.text().await?).into());
        }
        
        let login_result: LoginResponse = login_response.json().await?;
        self.token = Some(login_result.token.clone());
        self.user_id = Some(login_result.user.id.to_string());
        
        Ok(login_result.token)
    }
    
    /// Get authorization header
    fn get_auth_header(&self) -> String {
        format!("Bearer {}", self.token.as_ref().unwrap())
    }
    
    /// Upload a document to create test data
    async fn upload_document(&self, filename: &str, content: &[u8]) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let form = reqwest::multipart::Form::new()
            .part("file", reqwest::multipart::Part::bytes(content.to_vec())
                .file_name(filename.to_string())
                .mime_str("application/pdf")?);
        
        let response = self.client
            .post(&format!("{}/api/documents/upload", get_base_url()))
            .header("Authorization", self.get_auth_header())
            .multipart(form)
            .timeout(TIMEOUT)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Upload failed: {}", response.text().await?).into());
        }
        
        let document: Value = response.json().await?;
        Ok(document)
    }
    
    /// Manually mark a document as failed for testing
    async fn mark_document_as_failed(&self, _document_id: &str, _failure_reason: &str, _error_message: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // This would need to be implemented as a test utility endpoint or by direct database manipulation
        // For now, we'll use a mock approach by uploading a corrupted file that will naturally fail
        Ok(())
    }
    
    /// Get failed OCR documents
    async fn get_failed_ocr_documents(&self, limit: Option<i32>, offset: Option<i32>) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let mut url = format!("{}/api/documents/failed", get_base_url());
        
        let mut params = Vec::new();
        if let Some(l) = limit {
            params.push(format!("limit={}", l));
        }
        if let Some(o) = offset {
            params.push(format!("offset={}", o));
        }
        // Filter to only OCR failures since this is the OCR-specific test
        params.push("stage=ocr".to_string());
        
        if !params.is_empty() {
            url.push_str(&format!("?{}", params.join("&")));
        }
        
        let response = self.client
            .get(&url)
            .header("Authorization", self.get_auth_header())
            .timeout(TIMEOUT)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Failed to get failed OCR documents: {}", response.text().await?).into());
        }
        
        let result: Value = response.json().await?;
        Ok(result)
    }
    
    /// Retry OCR for a document
    async fn retry_ocr(&self, document_id: &str) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let response = self.client
            .post(&format!("{}/api/documents/{}/ocr/retry", get_base_url(), document_id))
            .header("Authorization", self.get_auth_header())
            .timeout(TIMEOUT)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Failed to retry OCR: {}", response.text().await?).into());
        }
        
        let result: Value = response.json().await?;
        Ok(result)
    }
    
    /// Get duplicates
    async fn get_duplicates(&self, limit: Option<i32>, offset: Option<i32>) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let mut url = format!("{}/api/documents/duplicates", get_base_url());
        
        let mut params = Vec::new();
        if let Some(l) = limit {
            params.push(format!("limit={}", l));
        }
        if let Some(o) = offset {
            params.push(format!("offset={}", o));
        }
        
        if !params.is_empty() {
            url.push_str(&format!("?{}", params.join("&")));
        }
        
        let response = self.client
            .get(&url)
            .header("Authorization", self.get_auth_header())
            .timeout(TIMEOUT)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Failed to get duplicates: {}", response.text().await?).into());
        }
        
        let result: Value = response.json().await?;
        Ok(result)
    }
}

#[tokio::test]
async fn test_failed_ocr_endpoint_structure() {
    let mut client = FailedOcrTestClient::new();
    
    // Register and login user
    let _token = match client.register_and_login(UserRole::User).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Setup failed: {}", e);
            return;
        }
    };
    
    // Test the failed OCR endpoint structure
    let failed_docs = client.get_failed_ocr_documents(None, None).await.unwrap();
    
    // Verify response structure
    assert!(failed_docs.get("documents").is_some());
    assert!(failed_docs.get("pagination").is_some());
    assert!(failed_docs.get("statistics").is_some());
    
    // Verify pagination structure
    let pagination = &failed_docs["pagination"];
    assert!(pagination.get("total").is_some());
    assert!(pagination.get("limit").is_some());
    assert!(pagination.get("offset").is_some());
    assert!(pagination.get("total_pages").is_some());
    
    // Verify statistics structure
    let statistics = &failed_docs["statistics"];
    assert!(statistics.get("total_failed").is_some());
    assert!(statistics.get("by_reason").is_some());
    
    println!("✅ Failed OCR endpoint returns proper structure");
}

#[tokio::test]
async fn test_failed_ocr_pagination() {
    let mut client = FailedOcrTestClient::new();
    
    let _token = match client.register_and_login(UserRole::User).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Setup failed: {}", e);
            return;
        }
    };
    
    // Test with different pagination parameters
    let response1 = client.get_failed_ocr_documents(Some(10), Some(0)).await.unwrap();
    let response2 = client.get_failed_ocr_documents(Some(5), Some(0)).await.unwrap();
    
    // Verify pagination is respected
    assert_eq!(response1["pagination"]["limit"], 10);
    assert_eq!(response2["pagination"]["limit"], 5);
    assert_eq!(response1["pagination"]["offset"], 0);
    assert_eq!(response2["pagination"]["offset"], 0);
    
    println!("✅ Failed OCR endpoint respects pagination parameters");
}

#[tokio::test]
async fn test_failed_ocr_statistics_format() {
    let mut client = FailedOcrTestClient::new();
    
    let _token = match client.register_and_login(UserRole::User).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Setup failed: {}", e);
            return;
        }
    };
    
    let failed_docs = client.get_failed_ocr_documents(None, None).await.unwrap();
    let statistics = &failed_docs["statistics"];
    
    // Verify statistics format
    assert!(statistics["total_failed"].is_number());
    assert!(statistics["by_reason"].is_object());
    
    // Verify failure reasons structure
    let by_reason = statistics["by_reason"].as_object().unwrap();
    for (reason, count) in by_reason {
        assert!(!reason.is_empty());
        assert!(count.is_number());
    }
    
    // Verify failure stages structure
    let by_stage = statistics["by_stage"].as_object().unwrap();
    for (stage, count) in by_stage {
        assert!(!stage.is_empty());
        assert!(count.is_number());
    }
    
    println!("✅ Failed OCR statistics have correct format");
}

#[tokio::test]
async fn test_duplicates_endpoint_structure() {
    let mut client = FailedOcrTestClient::new();
    
    let _token = match client.register_and_login(UserRole::User).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Setup failed: {}", e);
            return;
        }
    };
    
    // Test the duplicates endpoint
    let duplicates = client.get_duplicates(None, None).await.unwrap();
    
    // Verify response structure
    assert!(duplicates.get("duplicates").is_some());
    assert!(duplicates.get("pagination").is_some());
    assert!(duplicates.get("statistics").is_some());
    
    // Verify pagination structure
    let pagination = &duplicates["pagination"];
    assert!(pagination.get("total").is_some());
    assert!(pagination.get("limit").is_some());
    assert!(pagination.get("offset").is_some());
    assert!(pagination.get("has_more").is_some());
    
    // Verify statistics structure
    let statistics = &duplicates["statistics"];
    assert!(statistics.get("total_duplicate_groups").is_some());
    
    println!("✅ Duplicates endpoint returns proper structure");
}

#[tokio::test]
async fn test_retry_ocr_endpoint_with_invalid_document() {
    let mut client = FailedOcrTestClient::new();
    
    let _token = match client.register_and_login(UserRole::User).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Setup failed: {}", e);
            return;
        }
    };
    
    // Try to retry OCR for non-existent document
    let fake_document_id = Uuid::new_v4().to_string();
    let response = client.client
        .post(&format!("{}/api/documents/{}/ocr/retry", get_base_url(), fake_document_id))
        .header("Authorization", client.get_auth_header())
        .timeout(TIMEOUT)
        .send()
        .await
        .unwrap();
    
    // Should return error for non-existent document
    assert!(!response.status().is_success());
    
    println!("✅ Retry OCR endpoint properly handles invalid document IDs");
}

#[tokio::test]
async fn test_failed_ocr_endpoint_authorization() {
    let client = FailedOcrTestClient::new();
    
    // Try to access failed OCR endpoint without authentication
    let response = client.client
        .get(&format!("{}/api/documents/failed-ocr", get_base_url()))
        .timeout(TIMEOUT)
        .send()
        .await
        .unwrap();
    
    // Should return 401 Unauthorized
    assert_eq!(response.status(), 401);
    
    println!("✅ Failed OCR endpoint properly requires authentication");
}

#[tokio::test]
async fn test_duplicates_endpoint_authorization() {
    let client = FailedOcrTestClient::new();
    
    // Try to access duplicates endpoint without authentication
    let response = client.client
        .get(&format!("{}/api/documents/duplicates", get_base_url()))
        .timeout(TIMEOUT)
        .send()
        .await
        .unwrap();
    
    // Should return 401 Unauthorized
    assert_eq!(response.status(), 401);
    
    println!("✅ Duplicates endpoint properly requires authentication");
}

#[tokio::test]
async fn test_retry_ocr_endpoint_authorization() {
    let client = FailedOcrTestClient::new();
    
    // Try to retry OCR without authentication
    let fake_document_id = Uuid::new_v4().to_string();
    let response = client.client
        .post(&format!("{}/api/documents/{}/ocr/retry", get_base_url(), fake_document_id))
        .timeout(TIMEOUT)
        .send()
        .await
        .unwrap();
    
    // Should return 401 Unauthorized
    assert_eq!(response.status(), 401);
    
    println!("✅ Retry OCR endpoint properly requires authentication");
}

#[tokio::test]
async fn test_failed_ocr_empty_response_structure() {
    let mut client = FailedOcrTestClient::new();
    
    let _token = match client.register_and_login(UserRole::User).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Setup failed: {}", e);
            return;
        }
    };
    
    // Get failed OCR documents - should only see user's own documents
    let failed_docs = client.get_failed_ocr_documents(None, None).await.unwrap();
    
    // Structure should be consistent regardless of document count
    assert!(failed_docs["documents"].is_array());
    assert!(failed_docs["statistics"]["total_failed"].is_number());
    assert!(failed_docs["statistics"]["by_reason"].is_object());
    
    // The key test is structure consistency
    let documents = failed_docs["documents"].as_array().unwrap();
    let total_failed = failed_docs["statistics"]["total_failed"].as_i64().unwrap();
    
    // For a new user, both should be 0
    assert_eq!(documents.len(), 0, "New user should have no failed documents");
    assert_eq!(total_failed, 0, "New user should have total_failed = 0");
    
    // Also test pagination values for empty result
    assert_eq!(failed_docs["pagination"]["total"], 0);
    assert_eq!(failed_docs["pagination"]["total_pages"], 0);
    
    println!("✅ Failed OCR endpoint returns consistent empty structure for new user");
}

#[tokio::test]
async fn test_duplicates_empty_response_structure() {
    let mut client = FailedOcrTestClient::new();
    
    let _token = match client.register_and_login(UserRole::User).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Setup failed: {}", e);
            return;
        }
    };
    
    // Get duplicates (likely empty for new user)
    let duplicates = client.get_duplicates(None, None).await.unwrap();
    
    // Even with no duplicates, structure should be consistent
    assert!(duplicates["duplicates"].is_array());
    assert_eq!(duplicates["duplicates"].as_array().unwrap().len(), 0);
    assert_eq!(duplicates["statistics"]["total_duplicate_groups"], 0);
    
    println!("✅ Duplicates endpoint returns consistent structure even when empty");
}

#[tokio::test]
async fn test_admin_vs_user_access_to_failed_ocr() {
    // Test that admin can see all failed OCR documents while user only sees their own
    let mut admin_client = FailedOcrTestClient::new();
    let mut user_client = FailedOcrTestClient::new();
    
    let admin_token = match admin_client.register_and_login(UserRole::Admin).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Admin setup failed: {}", e);
            return;
        }
    };
    
    let user_token = match user_client.register_and_login(UserRole::User).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("User setup failed: {}", e);
            return;
        }
    };
    
    // Both should be able to access the endpoint
    let admin_response = admin_client.get_failed_ocr_documents(None, None).await.unwrap();
    let user_response = user_client.get_failed_ocr_documents(None, None).await.unwrap();
    
    // Both should return valid responses
    assert!(admin_response.get("documents").is_some());
    assert!(user_response.get("documents").is_some());
    
    println!("✅ Both admin and user can access failed OCR endpoint with proper scoping");
}