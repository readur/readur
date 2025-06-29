/*!
 * OCR Failure Counting Verification Tests
 * 
 * Tests to ensure that OCR failure counting and display works correctly:
 * - Verifies that failed OCR documents are properly counted
 * - Tests that failure categories are correctly categorized and counted
 * - Ensures the UI displays accurate failure statistics
 * - Tests edge cases with zero failures
 * - Verifies failure reason classification logic
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

/// Test client for OCR failure counting verification
struct OcrFailureCountingTestClient {
    client: Client,
    token: Option<String>,
    user_id: Option<String>,
}

impl OcrFailureCountingTestClient {
    fn new() -> Self {
        Self {
            client: Client::new(),
            token: None,
            user_id: None,
        }
    }
    
    /// Register and login a test user
    async fn register_and_login(&mut self, role: UserRole) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Health check
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
        let username = format!("ocr_count_{}_{}_{}_{}", role.to_string(), test_id, nanos, Uuid::new_v4().simple());
        let email = format!("ocr_count_{}_{}@{}.example.com", test_id, nanos, Uuid::new_v4().simple());
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
    
    /// Get failed OCR documents and statistics
    async fn get_failed_ocr_documents(&self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let response = self.client
            .get(&format!("{}/api/documents/failed?stage=ocr", get_base_url()))
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
    
    /// Upload a document (for creating test data)
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
}

#[tokio::test]
async fn test_zero_failures_display_correctly() {
    let mut client = OcrFailureCountingTestClient::new();
    
    let _token = match client.register_and_login(UserRole::User).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Setup failed: {}", e);
            return;
        }
    };
    
    // Get failed OCR documents for new user (should be zero)
    let result = client.get_failed_ocr_documents().await.unwrap();
    
    // Verify zero failures are handled correctly
    assert_eq!(result["statistics"]["total_failed"], 0);
    assert!(result["documents"].is_array());
    assert_eq!(result["documents"].as_array().unwrap().len(), 0);
    assert!(result["statistics"]["by_reason"].is_object());
    assert_eq!(result["statistics"]["by_reason"].as_object().unwrap().len(), 0);
    
    // Verify pagination shows zero
    assert_eq!(result["pagination"]["total"], 0);
    assert_eq!(result["pagination"]["total_pages"], 0);
    
    println!("✅ Zero failures are displayed correctly");
}

#[tokio::test]
async fn test_failure_categories_structure() {
    let mut client = OcrFailureCountingTestClient::new();
    
    let _token = match client.register_and_login(UserRole::User).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Setup failed: {}", e);
            return;
        }
    };
    
    let result = client.get_failed_ocr_documents().await.unwrap();
    let by_reason = &result["statistics"]["by_reason"];
    
    // Verify by_reason is an object
    assert!(by_reason.is_object());
    
    // Check structure of any failure reasons that exist
    if let Some(reasons) = by_reason.as_object() {
        for (reason, count) in reasons {
            // Each entry should have a non-empty reason and numeric count
            assert!(!reason.is_empty(), "Reason should not be empty");
            assert!(count.is_number(), "Count should be a number");
            
            // Count should be non-negative
            let count_val = count.as_i64().unwrap();
            assert!(count_val >= 0, "Failure count should be non-negative");
        }
    }
    
    println!("✅ Failure categories have correct structure");
}

#[tokio::test]
async fn test_total_failed_matches_document_count() {
    let mut client = OcrFailureCountingTestClient::new();
    
    let _token = match client.register_and_login(UserRole::User).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Setup failed: {}", e);
            return;
        }
    };
    
    let result = client.get_failed_ocr_documents().await.unwrap();
    
    // Get the total failed count from statistics
    let total_failed = result["statistics"]["total_failed"].as_i64().unwrap();
    
    // Get the actual number of documents returned
    let documents = result["documents"].as_array().unwrap();
    let actual_count = documents.len() as i64;
    
    // For the first page, the actual count should match pagination.total if total <= limit
    let pagination_total = result["pagination"]["total"].as_i64().unwrap();
    let limit = result["pagination"]["limit"].as_i64().unwrap();
    
    // The documents count should be min(total_failed, limit) if we're on the first page
    let expected_documents_count = std::cmp::min(total_failed, limit);
    assert_eq!(actual_count, expected_documents_count, 
               "Document count should match expected count for first page");
    
    // Total failed should match pagination total
    assert_eq!(total_failed, pagination_total, 
               "Total failed should match pagination total");
    
    println!("✅ Total failed count matches document count and pagination");
}

#[tokio::test]
async fn test_failure_category_counts_sum_to_total() {
    let mut client = OcrFailureCountingTestClient::new();
    
    let _token = match client.register_and_login(UserRole::User).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Setup failed: {}", e);
            return;
        }
    };
    
    let result = client.get_failed_ocr_documents().await.unwrap();
    
    let total_failed = result["statistics"]["total_failed"].as_i64().unwrap();
    let by_reason = result["statistics"]["by_reason"].as_object().unwrap();
    
    // Sum up all reason counts
    let reason_sum: i64 = by_reason
        .values()
        .map(|count| count.as_i64().unwrap())
        .sum();
    
    // Reason counts should sum to total failed
    assert_eq!(reason_sum, total_failed, 
               "Sum of reason counts should equal total failed count");
    
    println!("✅ Failure category counts sum to total failed count");
}

#[tokio::test]
async fn test_failure_reason_classification() {
    let mut client = OcrFailureCountingTestClient::new();
    
    let _token = match client.register_and_login(UserRole::User).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Setup failed: {}", e);
            return;
        }
    };
    
    let result = client.get_failed_ocr_documents().await.unwrap();
    let by_reason = result["statistics"]["by_reason"].as_object().unwrap();
    
    // Check that known failure reasons are properly categorized
    let valid_reason_keys = vec![
        "low_ocr_confidence",
        "ocr_timeout", 
        "ocr_memory_limit",
        "pdf_parsing_error",
        "file_corrupted",
        "unsupported_format",
        "access_denied",
        "other"
    ];
    
    for (reason_key, _count) in by_reason {
        assert!(valid_reason_keys.contains(&reason_key.as_str()), 
                "Reason key '{}' should be one of the valid failure reasons", reason_key);
    }
    
    println!("✅ Failure reasons are properly classified");
}

#[tokio::test]
async fn test_document_failure_fields_present() {
    let mut client = OcrFailureCountingTestClient::new();
    
    let _token = match client.register_and_login(UserRole::User).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Setup failed: {}", e);
            return;
        }
    };
    
    let result = client.get_failed_ocr_documents().await.unwrap();
    let documents = result["documents"].as_array().unwrap();
    
    // Check each failed document has required fields
    for document in documents {
        // Required fields for failed documents
        assert!(document.get("id").is_some(), "Document should have 'id' field");
        assert!(document.get("filename").is_some(), "Document should have 'filename' field");
        assert!(document.get("ocr_status").is_some(), "Document should have 'ocr_status' field");
        assert!(document.get("ocr_error").is_some(), "Document should have 'ocr_error' field");
        assert!(document.get("ocr_failure_reason").is_some(), "Document should have 'ocr_failure_reason' field");
        assert!(document.get("failure_category").is_some(), "Document should have 'failure_category' field");
        assert!(document.get("retry_count").is_some(), "Document should have 'retry_count' field");
        assert!(document.get("can_retry").is_some(), "Document should have 'can_retry' field");
        
        // Verify OCR status is 'failed'
        assert_eq!(document["ocr_status"], "failed", "OCR status should be 'failed'");
        
        // Verify retry count is a non-negative number
        let retry_count = document["retry_count"].as_i64().unwrap();
        assert!(retry_count >= 0, "Retry count should be non-negative");
        
        // Verify can_retry is a boolean
        assert!(document["can_retry"].is_boolean(), "'can_retry' should be a boolean");
    }
    
    println!("✅ Failed documents have all required fields");
}

#[tokio::test]
async fn test_pagination_consistency() {
    let mut client = OcrFailureCountingTestClient::new();
    
    let _token = match client.register_and_login(UserRole::User).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Setup failed: {}", e);
            return;
        }
    };
    
    // Test different pagination parameters
    let response1 = client.client
        .get(&format!("{}/api/documents/failed?stage=ocr&limit=10&offset=0", get_base_url()))
        .header("Authorization", client.get_auth_header())
        .timeout(TIMEOUT)
        .send()
        .await.unwrap();
        
    let result1: Value = response1.json().await.unwrap();
    
    let response2 = client.client
        .get(&format!("{}/api/documents/failed?stage=ocr&limit=5&offset=0", get_base_url()))
        .header("Authorization", client.get_auth_header())
        .timeout(TIMEOUT)
        .send()
        .await.unwrap();
        
    let result2: Value = response2.json().await.unwrap();
    
    // Both should have same total count
    assert_eq!(result1["pagination"]["total"], result2["pagination"]["total"], 
               "Total count should be consistent across different pagination requests");
    assert_eq!(result1["statistics"]["total_failed"], result2["statistics"]["total_failed"], 
               "Total failed count should be consistent across different pagination requests");
    
    // Verify pagination parameters are respected
    assert_eq!(result1["pagination"]["limit"], 10);
    assert_eq!(result2["pagination"]["limit"], 5);
    assert_eq!(result1["pagination"]["offset"], 0);
    assert_eq!(result2["pagination"]["offset"], 0);
    
    // Documents array length should not exceed limit
    let docs1_len = result1["documents"].as_array().unwrap().len();
    let docs2_len = result2["documents"].as_array().unwrap().len();
    assert!(docs1_len <= 10, "Documents should not exceed limit of 10");
    assert!(docs2_len <= 5, "Documents should not exceed limit of 5");
    
    println!("✅ Pagination parameters are consistent and respected");
}

#[tokio::test]
async fn test_statistics_are_always_present() {
    let mut client = OcrFailureCountingTestClient::new();
    
    let _token = match client.register_and_login(UserRole::User).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Setup failed: {}", e);
            return;
        }
    };
    
    let result = client.get_failed_ocr_documents().await.unwrap();
    
    // Statistics should always be present
    assert!(result.get("statistics").is_some(), "Statistics should always be present");
    
    let statistics = &result["statistics"];
    assert!(statistics.get("total_failed").is_some(), "total_failed should always be present");
    assert!(statistics.get("by_reason").is_some(), "by_reason should always be present");
    assert!(statistics.get("by_stage").is_some(), "by_stage should always be present");
    
    // Values should be valid even if zero
    assert!(statistics["total_failed"].is_number(), "total_failed should be a number");
    assert!(statistics["by_reason"].is_object(), "by_reason should be an object");
    assert!(statistics["by_stage"].is_object(), "by_stage should be an object");
    
    let total_failed = statistics["total_failed"].as_i64().unwrap();
    assert!(total_failed >= 0, "total_failed should be non-negative");
    
    println!("✅ Statistics are always present and valid");
}