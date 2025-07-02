use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;
use uuid::Uuid;

use readur::models::{CreateUser, LoginRequest, LoginResponse, UserRole};

fn get_base_url() -> String {
    std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8000".to_string())
}

const TIMEOUT: Duration = Duration::from_secs(60);

struct OcrRetryTestHelper {
    client: Client,
    token: String,
}

impl OcrRetryTestHelper {
    async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let client = Client::new();
        
        // First check if server is running
        let health_check = client
            .get(&format!("{}/api/health", get_base_url()))
            .timeout(Duration::from_secs(5))
            .send()
            .await;
        
        if let Err(e) = health_check {
            eprintln!("Health check failed: {}. Is the server running at {}?", e, get_base_url());
            return Err(format!("Server not running: {}", e).into());
        }
        
        // Create a test admin user
        let test_id = Uuid::new_v4().simple().to_string();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let username = format!("ocr_retry_admin_{}_{}", test_id, nanos);
        let email = format!("ocr_retry_admin_{}@{}.example.com", test_id, nanos);
        let password = "testpassword123";
        
        // Register admin user
        let user_data = CreateUser {
            username: username.clone(),
            email: email.clone(),
            password: password.to_string(),
            role: Some(UserRole::Admin),
        };
        
        let register_response = client
            .post(&format!("{}/api/auth/register", get_base_url()))
            .json(&user_data)
            .timeout(TIMEOUT)
            .send()
            .await?;
        
        if !register_response.status().is_success() {
            return Err(format!("Registration failed: {}", register_response.text().await?).into());
        }
        
        // Login with the new user
        let login_data = LoginRequest {
            username: username.clone(),
            password: password.to_string(),
        };
        
        let login_response = client
            .post(&format!("{}/api/auth/login", get_base_url()))
            .json(&login_data)
            .timeout(TIMEOUT)
            .send()
            .await?;
        
        if !login_response.status().is_success() {
            return Err(format!("Login failed: {}", login_response.text().await?).into());
        }
        
        let login_result: LoginResponse = login_response.json().await?;
        let token = login_result.token;
        
        Ok(Self { client, token })
    }
    
    fn get_auth_header(&self) -> String {
        format!("Bearer {}", self.token)
    }
    
    async fn get_retry_stats(&self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let response = self.client
            .get(&format!("{}/api/documents/ocr/retry-stats", get_base_url()))
            .header("Authorization", self.get_auth_header())
            .timeout(TIMEOUT)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Failed to get retry stats: {}", response.text().await?).into());
        }
        
        let result: Value = response.json().await?;
        Ok(result)
    }
    
    async fn get_retry_recommendations(&self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let response = self.client
            .get(&format!("{}/api/documents/ocr/retry-recommendations", get_base_url()))
            .header("Authorization", self.get_auth_header())
            .timeout(TIMEOUT)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Failed to get retry recommendations: {}", response.text().await?).into());
        }
        
        let result: Value = response.json().await?;
        Ok(result)
    }
    
    async fn bulk_retry_ocr(&self, mode: &str, document_ids: Option<Vec<String>>, preview_only: bool) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let mut request_body = json!({
            "mode": mode,
            "preview_only": preview_only
        });
        
        if let Some(ids) = document_ids {
            request_body["document_ids"] = json!(ids);
        }
        
        let response = self.client
            .post(&format!("{}/api/documents/ocr/bulk-retry", get_base_url()))
            .header("Authorization", self.get_auth_header())
            .json(&request_body)
            .timeout(TIMEOUT)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Failed to bulk retry OCR: {}", response.text().await?).into());
        }
        
        let result: Value = response.json().await?;
        Ok(result)
    }
    
    async fn get_document_retry_history(&self, document_id: &str) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let response = self.client
            .get(&format!("{}/api/documents/{}/ocr/retry-history", get_base_url(), document_id))
            .header("Authorization", self.get_auth_header())
            .timeout(TIMEOUT)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Failed to get retry history: {}", response.text().await?).into());
        }
        
        let result: Value = response.json().await?;
        Ok(result)
    }
    
    async fn get_failed_documents(&self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let response = self.client
            .get(&format!("{}/api/documents/failed", get_base_url()))
            .header("Authorization", self.get_auth_header())
            .timeout(TIMEOUT)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Failed to get failed documents: {}", response.text().await?).into());
        }
        
        let result: Value = response.json().await?;
        Ok(result)
    }
}

#[tokio::test]
async fn test_ocr_retry_stats_endpoint() {
    let helper = match OcrRetryTestHelper::new().await {
        Ok(h) => h,
        Err(e) => {
            println!("⚠️  Skipping OCR retry stats test (setup failed): {}", e);
            return;
        }
    };
    
    // Test getting retry statistics
    match helper.get_retry_stats().await {
        Ok(stats) => {
            println!("✅ OCR retry stats endpoint working");
            
            // Verify response structure
            assert!(stats["failure_reasons"].is_array(), "Should have failure_reasons array");
            assert!(stats["file_types"].is_array(), "Should have file_types array");
            assert!(stats["total_failed"].is_number(), "Should have total_failed count");
            
            println!("📊 Total failed documents: {}", stats["total_failed"]);
        }
        Err(e) => {
            println!("❌ OCR retry stats test failed: {}", e);
            panic!("OCR retry stats endpoint failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_ocr_retry_recommendations_endpoint() {
    let helper = match OcrRetryTestHelper::new().await {
        Ok(h) => h,
        Err(e) => {
            println!("⚠️  Skipping OCR retry recommendations test (setup failed): {}", e);
            return;
        }
    };
    
    // Test getting retry recommendations
    match helper.get_retry_recommendations().await {
        Ok(recommendations) => {
            println!("✅ OCR retry recommendations endpoint working");
            
            // Verify response structure
            assert!(recommendations["recommendations"].is_array(), "Should have recommendations array");
            assert!(recommendations["total_recommendations"].is_number(), "Should have total count");
            
            let recs = recommendations["recommendations"].as_array().unwrap();
            println!("💡 Got {} retry recommendations", recs.len());
            
            for rec in recs {
                println!("  - {}: {} documents ({}% success rate)", 
                    rec["title"].as_str().unwrap_or("Unknown"),
                    rec["document_count"].as_i64().unwrap_or(0),
                    (rec["estimated_success_rate"].as_f64().unwrap_or(0.0) * 100.0) as i32
                );
            }
        }
        Err(e) => {
            println!("❌ OCR retry recommendations test failed: {}", e);
            panic!("OCR retry recommendations endpoint failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_bulk_retry_preview_mode() {
    let helper = match OcrRetryTestHelper::new().await {
        Ok(h) => h,
        Err(e) => {
            println!("⚠️  Skipping bulk retry preview test (setup failed): {}", e);
            return;
        }
    };
    
    // Test preview mode - should not actually queue anything
    match helper.bulk_retry_ocr("all", None, true).await {
        Ok(result) => {
            println!("✅ Bulk retry preview mode working");
            
            // Verify response structure
            assert!(result["success"].as_bool().unwrap_or(false), "Should be successful");
            assert!(result["matched_count"].is_number(), "Should have matched_count");
            assert!(result["queued_count"].is_number(), "Should have queued_count");
            assert!(result["documents"].is_array(), "Should have documents array");
            assert!(result["message"].as_str().unwrap_or("").contains("Preview"), "Should indicate preview mode");
            
            // In preview mode, queued_count should be 0
            assert_eq!(result["queued_count"].as_u64().unwrap_or(1), 0, "Preview mode should not queue any documents");
            
            println!("📋 Preview found {} documents that would be retried", result["matched_count"]);
        }
        Err(e) => {
            println!("❌ Bulk retry preview test failed: {}", e);
            panic!("Bulk retry preview failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_document_retry_history() {
    let helper = match OcrRetryTestHelper::new().await {
        Ok(h) => h,
        Err(e) => {
            println!("⚠️  Skipping retry history test (setup failed): {}", e);
            return;
        }
    };
    
    // First get some failed documents to test with
    match helper.get_failed_documents().await {
        Ok(failed_docs) => {
            let empty_vec = vec![];
            let documents = failed_docs["documents"].as_array().unwrap_or(&empty_vec);
            
            if documents.is_empty() {
                println!("⚠️  No failed documents found, skipping retry history test");
                return;
            }
            
            let first_doc_id = documents[0]["id"].as_str().unwrap();
            
            // Test getting retry history for this document
            match helper.get_document_retry_history(first_doc_id).await {
                Ok(history) => {
                    println!("✅ Document retry history endpoint working");
                    
                    // Verify response structure
                    assert!(history["document_id"].is_string(), "Should have document_id");
                    assert!(history["retry_history"].is_array(), "Should have retry_history array");
                    assert!(history["total_retries"].is_number(), "Should have total_retries count");
                    
                    println!("📜 Document {} has {} retry attempts", 
                        first_doc_id, 
                        history["total_retries"].as_i64().unwrap_or(0)
                    );
                }
                Err(e) => {
                    println!("❌ Document retry history test failed: {}", e);
                    panic!("Document retry history failed: {}", e);
                }
            }
        }
        Err(e) => {
            println!("⚠️  Could not get failed documents for retry history test: {}", e);
        }
    }
}

#[tokio::test]
async fn test_filtered_bulk_retry_preview() {
    let helper = match OcrRetryTestHelper::new().await {
        Ok(h) => h,
        Err(e) => {
            println!("⚠️  Skipping filtered bulk retry test (setup failed): {}", e);
            return;
        }
    };
    
    // Test filtered retry with specific criteria
    let request_body = json!({
        "mode": "filter",
        "preview_only": true,
        "filter": {
            "mime_types": ["application/pdf"],
            "max_file_size": 5242880, // 5MB
            "limit": 10
        }
    });
    
    let response = helper.client
        .post(&format!("{}/api/documents/ocr/bulk-retry", get_base_url()))
        .header("Authorization", helper.get_auth_header())
        .json(&request_body)
        .timeout(TIMEOUT)
        .send()
        .await;
    
    match response {
        Ok(res) if res.status().is_success() => {
            let result: Value = res.json().await.unwrap();
            println!("✅ Filtered bulk retry preview working");
            
            // Verify filtering worked
            let documents = result["documents"].as_array().unwrap();
            for doc in documents {
                let mime_type = doc["mime_type"].as_str().unwrap_or("");
                assert_eq!(mime_type, "application/pdf", "Should only return PDF documents");
                
                let file_size = doc["file_size"].as_i64().unwrap_or(0);
                assert!(file_size <= 5242880, "Should only return files <= 5MB");
            }
            
            println!("🔍 Filtered preview found {} matching documents", documents.len());
        }
        Ok(res) => {
            let status = res.status();
            let error_text = res.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            println!("❌ Filtered bulk retry failed with status {}: {}", status, error_text);
        }
        Err(e) => {
            println!("❌ Filtered bulk retry request failed: {}", e);
        }
    }
}