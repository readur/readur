/*!
 * Integration Tests for Readur OCR System
 * 
 * Tests complete user workflows against a running server using Rust's reqwest client.
 * These tests import and use the same models/types as the main application.
 */

use reqwest::Client;
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::time::sleep;

use readur::models::{CreateUser, LoginRequest, LoginResponse, DocumentResponse};
use readur::routes::documents::types::DocumentUploadResponse;

fn get_base_url() -> String {
    std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8000".to_string())
}

const TIMEOUT: Duration = Duration::from_secs(30);

/// Integration test client that handles authentication and common operations
struct TestClient {
    client: Client,
    token: Option<String>,
}

impl TestClient {
    fn new() -> Self {
        Self {
            client: Client::new(),
            token: None,
        }
    }
    
    /// Check if server is running and healthy
    async fn check_server_health(&self) -> Result<(), Box<dyn std::error::Error>> {
        let response = self.client
            .get(&format!("{}/api/health", get_base_url()))
            .timeout(Duration::from_secs(5))
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err("Server health check failed".into());
        }
        
        let health: Value = response.json().await?;
        if health["status"] != "ok" {
            return Err("Server is not healthy".into());
        }
        
        Ok(())
    }
    
    /// Register a new user and login to get auth token
    async fn register_and_login(&mut self, username: &str, email: &str, password: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Register user
        let user_data = CreateUser {
            username: username.to_string(),
            email: email.to_string(),
            password: password.to_string(),
            role: Some(readur::models::UserRole::User),
        };
        
        let register_response = self.client
            .post(&format!("{}/api/auth/register", get_base_url()))
            .json(&user_data)
            .send()
            .await?;
        
        if !register_response.status().is_success() {
            return Err(format!("Registration failed: {}", register_response.text().await?).into());
        }
        
        // Login to get token
        let login_data = LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
        };
        
        let login_response = self.client
            .post(&format!("{}/api/auth/login", get_base_url()))
            .json(&login_data)
            .send()
            .await?;
        
        if !login_response.status().is_success() {
            return Err(format!("Login failed: {}", login_response.text().await?).into());
        }
        
        let login_result: LoginResponse = login_response.json().await?;
        self.token = Some(login_result.token.clone());
        
        Ok(login_result.token)
    }
    
    /// Upload a test document
    async fn upload_document(&self, content: &str, filename: &str) -> Result<DocumentUploadResponse, Box<dyn std::error::Error>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let part = reqwest::multipart::Part::text(content.to_string())
            .file_name(filename.to_string())
            .mime_str("text/plain")?;
        let form = reqwest::multipart::Form::new()
            .part("file", part);
        
        let response = self.client
            .post(&format!("{}/api/documents", get_base_url()))
            .header("Authorization", format!("Bearer {}", token))
            .multipart(form)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Upload failed: {}", response.text().await?).into());
        }
        
        let document: DocumentUploadResponse = response.json().await?;
        Ok(document)
    }
    
    /// Wait for OCR processing to complete
    async fn wait_for_ocr_completion(&self, document_id: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        let start = Instant::now();
        
        while start.elapsed() < TIMEOUT {
            let response = self.client
                .get(&format!("{}/api/documents", get_base_url()))
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await?;
            
            if response.status().is_success() {
                let response_json: serde_json::Value = response.json().await?;
                let documents = if let Some(docs_array) = response_json.get("documents").and_then(|d| d.as_array()) {
                    // Documents are in a "documents" key
                    docs_array
                } else if let Some(docs_array) = response_json.as_array() {
                    // Response is directly an array of documents
                    docs_array
                } else {
                    return Err("Invalid response format: missing documents array".into());
                };
                
                for doc_value in documents {
                    let doc: DocumentResponse = serde_json::from_value(doc_value.clone())?;
                    if doc.id.to_string() == document_id {
                        match doc.ocr_status.as_deref() {
                            Some("completed") => return Ok(true),
                            Some("failed") => return Err("OCR processing failed".into()),
                            _ => {
                                sleep(Duration::from_millis(500)).await;
                                continue;
                            }
                        }
                    }
                }
            }
            
            sleep(Duration::from_millis(500)).await;
        }
        
        Ok(false)
    }
    
    /// Get OCR text for a document
    async fn get_ocr_text(&self, document_id: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let response = self.client
            .get(&format!("{}/api/documents/{}/ocr", get_base_url(), document_id))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("OCR retrieval failed: {}", response.text().await?).into());
        }
        
        let ocr_data: Value = response.json().await?;
        Ok(ocr_data)
    }
}

#[tokio::test]
async fn test_complete_ocr_workflow() {
    let mut client = TestClient::new();
    
    // Check server health
    if let Err(e) = client.check_server_health().await {
        panic!("Server not running at {}: {}", get_base_url(), e);
    }
    
    // Create test user with unique timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let username = format!("rust_integration_test_{}", timestamp);
    let email = format!("rust_test_{}@example.com", timestamp);
    let password = "testpassword123";
    
    let token = client.register_and_login(&username, &email, password).await
        .expect("Failed to register and login");
    
    println!("✅ User registered and logged in, token: {}", &token[..20]);
    
    // Upload test document
    let test_content = r#"This is a test document for OCR processing.
It contains multiple lines of text.
The OCR service should extract this text accurately.

Document ID: RUST-INTEGRATION-TEST-001
Date: 2024-01-01
Technology: Rust + Axum + SQLx"#;
    
    let document = client.upload_document(test_content, "rust_test.txt").await
        .expect("Failed to upload document");
    
    println!("✅ Document uploaded: {}", document.document_id);
    
    // Validate document response structure using our types
    assert!(!document.filename.is_empty());
    assert!(document.file_size > 0);
    assert_eq!(document.mime_type, "text/plain");
    
    // Wait for OCR processing
    let ocr_completed = client.wait_for_ocr_completion(&document.document_id.to_string()).await
        .expect("Failed to wait for OCR completion");
    
    assert!(ocr_completed, "OCR processing did not complete within timeout");
    println!("✅ OCR processing completed");
    
    // Retrieve OCR text
    let ocr_data = client.get_ocr_text(&document.document_id.to_string()).await
        .expect("Failed to retrieve OCR text");
    
    // Validate OCR response structure
    assert_eq!(ocr_data["document_id"], document.document_id.to_string());
    assert_eq!(ocr_data["filename"], document.filename);
    assert!(ocr_data["has_ocr_text"].as_bool().unwrap_or(false));
    
    // Validate OCR content if available
    if let Some(ocr_text) = ocr_data["ocr_text"].as_str() {
        assert!(!ocr_text.is_empty(), "OCR text should not be empty");
        assert!(ocr_text.to_lowercase().contains("test document"), "OCR text should contain expected content");
        println!("✅ OCR text extracted: {} characters", ocr_text.len());
        
        // Validate optional fields using Rust type checking
        if let Some(confidence) = ocr_data["ocr_confidence"].as_f64() {
            assert!((0.0..=100.0).contains(&confidence), "OCR confidence should be 0-100");
            println!("✅ OCR confidence: {:.1}%", confidence);
        }
        
        if let Some(word_count) = ocr_data["ocr_word_count"].as_i64() {
            assert!(word_count > 0, "Word count should be positive");
            println!("✅ OCR word count: {}", word_count);
        }
        
        if let Some(processing_time) = ocr_data["ocr_processing_time_ms"].as_i64() {
            assert!(processing_time >= 0, "Processing time should be non-negative");
            println!("✅ OCR processing time: {}ms", processing_time);
        }
    }
    
    println!("🎉 Complete OCR workflow test passed!");
}

#[tokio::test]
async fn test_ocr_error_handling() {
    let mut client = TestClient::new();
    
    // Test unauthorized access
    let response = client.client
        .get(&format!("{}/api/documents/test-id/ocr", get_base_url()))
        .send()
        .await
        .expect("Failed to make request");
    
    assert_eq!(response.status(), 401, "Should return 401 for unauthorized access");
    
    // Test with valid auth but invalid document
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let token = client.register_and_login(
        &format!("rust_error_test_{}", timestamp), 
        &format!("rust_error_{}@test.com", timestamp), 
        "testpass123"
    ).await.expect("Failed to register and login");
    
    let response = client.client
        .get(&format!("{}/api/documents/00000000-0000-0000-0000-000000000000/ocr", get_base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to make request");
    
    assert_eq!(response.status(), 404, "Should return 404 for non-existent document");
    
    println!("✅ Error handling tests passed!");
}

#[tokio::test]
async fn test_health_endpoint() {
    let client = TestClient::new();
    
    client.check_server_health().await
        .expect("Health check should pass");
    
    println!("✅ Health endpoint test passed!");
}

#[tokio::test]
async fn test_document_list_structure() {
    let mut client = TestClient::new();
    
    // Register and login
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let _token = client.register_and_login(
        &format!("rust_list_test_{}", timestamp), 
        &format!("rust_list_{}@test.com", timestamp), 
        "testpass123"
    ).await.expect("Failed to register and login");
    
    // Upload a document
    let document = client.upload_document("Test content for list", "list_test.txt").await
        .expect("Failed to upload document");
    
    // Get document list
    let response = client.client
        .get(&format!("{}/api/documents", get_base_url()))
        .header("Authorization", format!("Bearer {}", client.token.as_ref().unwrap()))
        .send()
        .await
        .expect("Failed to get documents");
    
    assert!(response.status().is_success());
    
    // Parse as our DocumentResponse type to ensure structure compatibility
    let response_json: serde_json::Value = response.json().await
        .expect("Failed to parse response JSON");
    
    let documents_array = if let Some(docs_array) = response_json.get("documents").and_then(|d| d.as_array()) {
        // Documents are in a "documents" key
        docs_array
    } else if let Some(docs_array) = response_json.as_array() {
        // Response is directly an array of documents
        docs_array
    } else {
        panic!("Failed to find documents array in response");
    };
    
    let documents: Vec<DocumentResponse> = documents_array.iter()
        .map(|doc_value| serde_json::from_value(doc_value.clone()))
        .collect::<Result<Vec<_>, _>>()
        .expect("Failed to parse documents as DocumentResponse");
    
    // Find our uploaded document
    let found_doc = documents.iter().find(|d| d.id.to_string() == document.document_id.to_string())
        .expect("Uploaded document should be in list");
    
    // Validate structure matches our types
    assert_eq!(found_doc.filename, document.filename);
    assert_eq!(found_doc.file_size, document.file_size);
    assert_eq!(found_doc.mime_type, document.mime_type);
    assert!(found_doc.ocr_status.is_some());
    
    println!("✅ Document list structure test passed!");
}

/// Helper function to run all integration tests when server is not available
#[tokio::test]
async fn test_server_availability() {
    let client = TestClient::new();
    
    match client.check_server_health().await {
        Ok(_) => println!("✅ Server is running and healthy"),
        Err(e) => {
            println!("⚠️  Server not available: {}", e);
            println!("To run integration tests, start the server with: cargo run");
            // Don't fail the test, just skip
        }
    }
}