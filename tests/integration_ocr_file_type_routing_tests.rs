/*!
 * OCR File Type Routing Integration Tests
 * 
 * Tests to verify that files are correctly routed between OCR and text extraction
 * based on their file types. This addresses the issue where Office documents
 * (DOC/DOCX) were incorrectly sent to the OCR pipeline instead of text extraction.
 * 
 * Tests include:
 * - DOCX files should NOT be queued for OCR
 * - DOC files should NOT be queued for OCR  
 * - Image files (PNG, JPEG) SHOULD be queued for OCR
 * - PDF files SHOULD be queued for OCR
 * - OCR queue validation prevents unsupported files from processing
 */

use reqwest::Client;
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use uuid::Uuid;

use readur::models::{CreateUser, LoginRequest, LoginResponse, UserRole, DocumentResponse};
use readur::routes::documents::types::{DocumentUploadResponse, PaginatedDocumentsResponse};

fn get_base_url() -> String {
    std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8000".to_string())
}
const TIMEOUT: Duration = Duration::from_secs(60);

/// Test client for file type routing tests
struct FileTypeRoutingTestClient {
    client: Client,
    token: Option<String>,
    user_id: Option<String>,
}

impl FileTypeRoutingTestClient {
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
            .timeout(TIMEOUT)
            .send()
            .await;
        
        if health_check.is_err() {
            return Err("Server is not running or not reachable".into());
        }

        let username = format!("test_user_{}", Uuid::new_v4());
        let password = "test123";
        let email = "test@example.com";

        // Register user
        let user_data = CreateUser {
            username: username.clone(),
            password: password.to_string(),
            email: email.to_string(),
            role,
        };

        let register_response = self.client
            .post(&format!("{}/api/auth/register", get_base_url()))
            .json(&user_data)
            .timeout(TIMEOUT)
            .send()
            .await?;

        if !register_response.status().is_success() {
            return Err(format!("Registration failed: {}", register_response.status()).into());
        }

        // Login
        let login_data = LoginRequest {
            username,
            password: password.to_string(),
        };

        let login_response = self.client
            .post(&format!("{}/api/auth/login", get_base_url()))
            .json(&login_data)
            .timeout(TIMEOUT)
            .send()
            .await?;

        let login_result: LoginResponse = login_response.json().await?;
        self.token = Some(login_result.token.clone());
        self.user_id = Some(login_result.user.id.to_string());
        
        Ok(login_result.token)
    }

    /// Get current OCR queue statistics
    async fn get_queue_stats(&self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let response = self.client
            .get(&format!("{}/api/queue/stats", get_base_url()))
            .header("Authorization", format!("Bearer {}", self.token.as_ref().unwrap()))
            .timeout(TIMEOUT)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Failed to get queue stats: {}", response.status()).into());
        }

        let stats: Value = response.json().await?;
        Ok(stats)
    }

    /// Upload a test file and return the document info
    async fn upload_test_file(&self, filename: &str, content: &[u8], mime_type: &str) -> Result<DocumentUploadResponse, Box<dyn std::error::Error + Send + Sync>> {
        let form = reqwest::multipart::Form::new()
            .part("file", reqwest::multipart::Part::bytes(content.to_vec())
                .file_name(filename)
                .mime_str(mime_type)?);

        let response = self.client
            .post(&format!("{}/api/documents", get_base_url()))
            .header("Authorization", format!("Bearer {}", self.token.as_ref().unwrap()))
            .multipart(form)
            .timeout(TIMEOUT)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("File upload failed: {} - {}", response.status(), error_text).into());
        }

        let document_info: DocumentUploadResponse = response.json().await?;
        Ok(document_info)
    }

    /// Wait for a specific duration and check if queue stats changed
    async fn wait_and_check_queue_change(&self, initial_pending: i64, initial_processing: i64, wait_seconds: u64) -> Result<(i64, i64), Box<dyn std::error::Error + Send + Sync>> {
        sleep(Duration::from_secs(wait_seconds)).await;
        
        let stats = self.get_queue_stats().await?;
        let current_pending = stats["pending_count"].as_i64().unwrap_or(0);
        let current_processing = stats["processing_count"].as_i64().unwrap_or(0);
        
        Ok((current_pending, current_processing))
    }
}

/// Create a minimal DOCX file content
fn create_test_docx_content() -> Vec<u8> {
    use std::io::Write;
    use zip::{ZipWriter, write::FileOptions};
    
    let mut buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
        let options = FileOptions::default();
        
        // Add minimal DOCX structure
        zip.start_file("[Content_Types].xml", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#).unwrap();
        
        zip.start_file("_rels/.rels", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#).unwrap();
        
        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>
<w:p><w:r><w:t>This is a test DOCX document with text content that should use text extraction, not OCR.</w:t></w:r></w:p>
</w:body>
</w:document>"#).unwrap();
        
        zip.finish().unwrap();
    }
    
    buffer
}

/// Create a simple PNG image content (1x1 red pixel)
fn create_test_png_content() -> Vec<u8> {
    // Minimal PNG file (1x1 red pixel)
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, // IHDR chunk length
        0x49, 0x48, 0x44, 0x52, // IHDR
        0x00, 0x00, 0x00, 0x01, // Width: 1
        0x00, 0x00, 0x00, 0x01, // Height: 1
        0x08, 0x02, 0x00, 0x00, 0x00, // Bit depth, color type, compression, filter, interlace
        0x90, 0x77, 0x53, 0xDE, // IHDR CRC
        0x00, 0x00, 0x00, 0x0C, // IDAT chunk length
        0x49, 0x44, 0x41, 0x54, // IDAT
        0x08, 0x99, 0x01, 0x01, 0x00, 0x03, 0x00, 0xFC, 0xFF, 0x00, 0x00, 0x00, // Compressed data (red pixel)
        0x02, 0x00, 0x01, 0xE2, // IDAT CRC  
        0x00, 0x00, 0x00, 0x00, // IEND chunk length
        0x49, 0x45, 0x4E, 0x44, // IEND
        0xAE, 0x42, 0x60, 0x82  // IEND CRC
    ]
}

#[tokio::test]
async fn test_docx_files_not_queued_for_ocr() {
    let mut client = FileTypeRoutingTestClient::new();
    
    // Register and login
    client.register_and_login(UserRole::User).await
        .expect("Failed to register and login user");

    // Get initial queue stats
    let initial_stats = client.get_queue_stats().await
        .expect("Failed to get initial queue stats");
    let initial_pending = initial_stats["pending_count"].as_i64().unwrap_or(0);
    let initial_processing = initial_stats["processing_count"].as_i64().unwrap_or(0);
    
    println!("Initial queue stats - Pending: {}, Processing: {}", initial_pending, initial_processing);

    // Upload a DOCX file
    let docx_content = create_test_docx_content();
    let document_info = client.upload_test_file(
        "test_document.docx", 
        &docx_content, 
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    ).await.expect("Failed to upload DOCX file");
    
    println!("Uploaded DOCX file: {}", document_info.filename);

    // Wait a bit and check if queue stats changed
    let (current_pending, current_processing) = client.wait_and_check_queue_change(initial_pending, initial_processing, 3).await
        .expect("Failed to check queue stats");
    
    println!("After DOCX upload - Pending: {}, Processing: {}", current_pending, current_processing);

    // DOCX files should NOT increase the OCR queue
    assert_eq!(current_pending, initial_pending, "DOCX file was incorrectly queued for OCR processing");
    assert_eq!(current_processing, initial_processing, "DOCX file was incorrectly sent to OCR processing");
    
    println!("✅ DOCX file was correctly NOT queued for OCR processing");
}

#[tokio::test]
async fn test_png_files_are_queued_for_ocr() {
    let mut client = FileTypeRoutingTestClient::new();
    
    // Register and login
    client.register_and_login(UserRole::User).await
        .expect("Failed to register and login user");

    // Get initial queue stats
    let initial_stats = client.get_queue_stats().await
        .expect("Failed to get initial queue stats");
    let initial_pending = initial_stats["pending_count"].as_i64().unwrap_or(0);
    let initial_processing = initial_stats["processing_count"].as_i64().unwrap_or(0);
    
    println!("Initial queue stats - Pending: {}, Processing: {}", initial_pending, initial_processing);

    // Upload a PNG file
    let png_content = create_test_png_content();
    let document_info = client.upload_test_file(
        "test_image.png", 
        &png_content, 
        "image/png"
    ).await.expect("Failed to upload PNG file");
    
    println!("Uploaded PNG file: {}", document_info.filename);

    // Wait a bit and check if queue stats changed
    let (current_pending, current_processing) = client.wait_and_check_queue_change(initial_pending, initial_processing, 3).await
        .expect("Failed to check queue stats");
    
    println!("After PNG upload - Pending: {}, Processing: {}", current_pending, current_processing);

    // PNG files SHOULD increase the OCR queue (either pending or processing)
    let total_initial = initial_pending + initial_processing;
    let total_current = current_pending + current_processing;
    
    assert!(total_current > total_initial, "PNG file was not queued for OCR processing (expected it to be queued)");
    
    println!("✅ PNG file was correctly queued for OCR processing");
}

#[tokio::test] 
async fn test_file_type_validation_utility_functions() {
    // Test the utility functions directly
    use readur::utils::ocr::{file_needs_ocr, file_needs_text_extraction};
    
    // Test files that should NOT go through OCR
    assert!(!file_needs_ocr("document.docx"), "DOCX should not need OCR");
    assert!(!file_needs_ocr("document.DOC"), "DOC should not need OCR"); 
    assert!(!file_needs_ocr("document.txt"), "TXT should not need OCR");
    assert!(!file_needs_ocr("document.html"), "HTML should not need OCR");
    
    // Test files that SHOULD go through OCR
    assert!(file_needs_ocr("document.pdf"), "PDF should need OCR");
    assert!(file_needs_ocr("image.png"), "PNG should need OCR");
    assert!(file_needs_ocr("image.JPG"), "JPG should need OCR");
    assert!(file_needs_ocr("scan.jpeg"), "JPEG should need OCR");
    
    // Test files that need text extraction
    assert!(file_needs_text_extraction("document.docx"), "DOCX should need text extraction");
    assert!(file_needs_text_extraction("document.doc"), "DOC should need text extraction");
    assert!(file_needs_text_extraction("document.txt"), "TXT should need text extraction");
    
    // Test files that don't need text extraction
    assert!(!file_needs_text_extraction("image.png"), "PNG should not need text extraction");
    assert!(!file_needs_text_extraction("document.pdf"), "PDF should not need text extraction");
    
    println!("✅ All utility function tests passed");
}

#[tokio::test]
async fn test_multiple_file_types_routing() {
    let mut client = FileTypeRoutingTestClient::new();
    
    // Register and login
    client.register_and_login(UserRole::User).await
        .expect("Failed to register and login user");

    // Get initial queue stats
    let initial_stats = client.get_queue_stats().await
        .expect("Failed to get initial queue stats");
    let initial_pending = initial_stats["pending_count"].as_i64().unwrap_or(0);
    let initial_processing = initial_stats["processing_count"].as_i64().unwrap_or(0);
    
    println!("Initial queue stats - Pending: {}, Processing: {}", initial_pending, initial_processing);

    // Upload multiple files of different types
    
    // 1. Upload DOCX (should NOT be queued)
    let docx_content = create_test_docx_content();
    client.upload_test_file(
        "test1.docx", 
        &docx_content, 
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    ).await.expect("Failed to upload DOCX file");
    
    // 2. Upload PNG (SHOULD be queued)  
    let png_content = create_test_png_content();
    client.upload_test_file(
        "test1.png", 
        &png_content, 
        "image/png"
    ).await.expect("Failed to upload PNG file");
    
    // 3. Upload another DOCX (should NOT be queued)
    client.upload_test_file(
        "test2.docx", 
        &docx_content, 
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    ).await.expect("Failed to upload second DOCX file");

    // Wait and check final queue stats
    let (final_pending, final_processing) = client.wait_and_check_queue_change(initial_pending, initial_processing, 5).await
        .expect("Failed to check final queue stats");
    
    println!("Final queue stats - Pending: {}, Processing: {}", final_pending, final_processing);

    // Only 1 file (the PNG) should have been queued for OCR
    let total_initial = initial_pending + initial_processing;
    let total_final = final_pending + final_processing;
    let queue_increase = total_final - total_initial;
    
    assert_eq!(queue_increase, 1, "Expected exactly 1 file to be queued for OCR (only the PNG), but {} files were queued", queue_increase);
    
    println!("✅ Multiple file types routed correctly - only PNG queued for OCR");
}