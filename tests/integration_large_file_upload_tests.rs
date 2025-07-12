use reqwest::Client;
use axum::http::StatusCode;
use std::time::Duration;
use uuid::Uuid;
use readur::models::{CreateUser, LoginRequest, LoginResponse, UserRole};

fn get_base_url() -> String {
    std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8000".to_string())
}

const TIMEOUT: Duration = Duration::from_secs(30);

/// Large file upload test client
struct LargeFileTestClient {
    client: Client,
    token: Option<String>,
}

impl LargeFileTestClient {
    fn new() -> Self {
        Self {
            client: Client::new(),
            token: None,
        }
    }
    
    /// Register a new user and login to get auth token
    async fn register_and_login(&mut self, role: UserRole) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let unique_id = Uuid::new_v4();
        let username = format!("large_file_test_{}", unique_id);
        let email = format!("large_file_test_{}@test.com", unique_id);
        
        // Register user
        let user_data = CreateUser {
            username: username.clone(),
            email: email.clone(),
            password: "testpass123".to_string(),
            role: Some(role),
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
            username: username.clone(),
            password: "testpass123".to_string(),
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
    
    /// Upload a file with specified content and filename
    async fn upload_file(&self, content: Vec<u8>, filename: &str, mime_type: &str) -> Result<reqwest::Response, Box<dyn std::error::Error + Send + Sync>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let part = reqwest::multipart::Part::bytes(content)
            .file_name(filename.to_string())
            .mime_str(mime_type)?;
        let form = reqwest::multipart::Form::new().part("file", part);
        
        let response = self.client
            .post(&format!("{}/api/documents", get_base_url()))
            .header("Authorization", format!("Bearer {}", token))
            .multipart(form)
            .send()
            .await?;
        
        Ok(response)
    }
}

/// Test uploading files of various sizes to verify body limit configuration
#[tokio::test]
async fn test_file_size_limits() {
    println!("🧪 Testing file size limits and body limit configuration...");
    
    let mut client = LargeFileTestClient::new();
    client.register_and_login(UserRole::User).await
        .expect("Failed to create test user and login");
    
    // Test 1: Small file (should succeed)
    println!("📄 Testing small file upload...");
    let small_content = "Small test file content.".repeat(100).into_bytes(); // ~2.5KB
    let small_response = client.upload_file(small_content, "small_test.txt", "text/plain")
        .await
        .expect("Small file upload should complete");
    
    println!("✅ Small file upload response: {}", small_response.status());
    assert!(small_response.status().is_success(), "Small file upload should succeed");
    
    // Test 2: Medium file (should succeed) - 3MB
    println!("📄 Testing medium file upload (3MB)...");
    let medium_content = "Medium test file content. ".repeat(125000).into_bytes(); // ~3MB
    let medium_response = client.upload_file(medium_content, "medium_test.txt", "text/plain")
        .await
        .expect("Medium file upload should complete");
    
    println!("✅ Medium file upload response: {}", medium_response.status());
    assert!(medium_response.status().is_success(), "Medium file upload should succeed");
    
    // Test 3: Large file (should succeed) - 15MB
    println!("📄 Testing large file upload (15MB)...");
    let large_content = "Large test file content. ".repeat(625000).into_bytes(); // ~15MB
    let large_response = client.upload_file(large_content, "large_test.txt", "text/plain")
        .await
        .expect("Large file upload should complete");
    
    println!("✅ Large file upload response: {}", large_response.status());
    assert!(large_response.status().is_success(), "Large file upload should succeed");
    
    // Test 4: Oversized file (should fail) - 60MB
    println!("📄 Testing oversized file upload (60MB) - should fail...");
    let oversized_content = vec![b'X'; 60 * 1024 * 1024]; // 60MB
    let oversized_response = client.upload_file(oversized_content, "oversized_test.bin", "application/octet-stream")
        .await
        .expect("Oversized file upload request should complete");
    
    println!("✅ Oversized file upload response: {}", oversized_response.status());
    // Accept either 413 (app-level rejection) or 400 (Axum body limit rejection)
    assert!(
        oversized_response.status() == StatusCode::PAYLOAD_TOO_LARGE || 
        oversized_response.status() == StatusCode::BAD_REQUEST,
        "Oversized file upload should fail with 413 Payload Too Large or 400 Bad Request, got: {}", 
        oversized_response.status()
    );
    
    println!("🎉 File size limit tests passed!");
}

/// Test specifically with the problematic PDF from the GitHub issue
#[tokio::test] 
async fn test_problematic_pdf_upload() {
    println!("🧪 Testing upload with the problematic PDF file...");
    
    let mut client = LargeFileTestClient::new();
    client.register_and_login(UserRole::User).await
        .expect("Failed to create test user and login");
    
    // Try to read the problematic PDF file
    let pdf_path = "test_files/porters-handbook_en.pdf";
    if !std::path::Path::new(pdf_path).exists() {
        println!("⚠️  Problematic PDF file not found at {}, skipping test", pdf_path);
        return;
    }
    
    let pdf_data = std::fs::read(pdf_path)
        .expect("Should be able to read PDF file");
    
    println!("📄 PDF file size: {} bytes ({:.2} MB)", 
             pdf_data.len(), pdf_data.len() as f64 / (1024.0 * 1024.0));
    
    let pdf_response = client.upload_file(pdf_data, "porters-handbook_en.pdf", "application/pdf")
        .await
        .expect("PDF upload request should complete");
    
    println!("✅ PDF upload response: {}", pdf_response.status());
    
    if pdf_response.status().is_success() {
        println!("🎉 Problematic PDF uploaded successfully!");
        
        // Verify the response contains expected data
        let response_body: serde_json::Value = pdf_response.json().await
            .expect("Should get JSON response");
        
        assert!(response_body.get("id").is_some(), "Response should contain document ID");
        assert_eq!(response_body.get("filename").and_then(|v| v.as_str()), 
                   Some("porters-handbook_en.pdf"), "Filename should match");
        
        println!("✅ Upload response data verified");
    } else {
        let status = pdf_response.status();
        let error_text = pdf_response.text().await.unwrap_or_default();
        panic!("PDF upload failed with status: {} - {}", status, error_text);
    }
}

/// Test that error messages are helpful for oversized files
#[tokio::test]
async fn test_oversized_file_error_handling() {
    println!("🧪 Testing error handling for oversized files...");
    
    let mut client = LargeFileTestClient::new();
    client.register_and_login(UserRole::User).await
        .expect("Failed to create test user and login");
    
    // Create a file that exceeds the 50MB limit
    let oversized_content = vec![b'X'; 60 * 1024 * 1024]; // 60MB
    let response = client.upload_file(oversized_content, "huge_file.bin", "application/octet-stream")
        .await
        .expect("Request should complete");
    
    println!("✅ Oversized file response status: {}", response.status());
    // Accept either 413 (app-level rejection) or 400 (Axum body limit rejection)
    assert!(
        response.status() == StatusCode::PAYLOAD_TOO_LARGE || 
        response.status() == StatusCode::BAD_REQUEST,
        "Should return 413 Payload Too Large or 400 Bad Request for oversized files, got: {}", 
        response.status()
    );
    
    println!("🎉 Error handling test passed!");
}