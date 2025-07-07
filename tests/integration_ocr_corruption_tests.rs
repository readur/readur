/*!
 * OCR Corruption Integration Tests
 * 
 * Tests for diagnosing and reproducing the issue where FileA's OCR text
 * gets corrupted when FileB is processed simultaneously.
 */

use reqwest::Client;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use uuid::Uuid;

use readur::models::{CreateUser, LoginRequest, LoginResponse};
use readur::routes::documents::types::DocumentUploadResponse;

fn get_base_url() -> String {
    std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8000".to_string())
}
const TIMEOUT: Duration = Duration::from_secs(60);

/// Test client for OCR corruption scenarios
struct OcrTestClient {
    client: Client,
    token: Option<String>,
    user_id: Option<Uuid>,
}

impl OcrTestClient {
    fn new() -> Self {
        Self {
            client: Client::new(),
            token: None,
            user_id: None,
        }
    }
    
    async fn check_server_health(&self) -> Result<(), Box<dyn std::error::Error>> {
        let response = self.client
            .get(&format!("{}/api/health", get_base_url()))
            .timeout(Duration::from_secs(5))
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err("Server health check failed".into());
        }
        
        Ok(())
    }
    
    async fn register_and_login(&mut self, username: &str, email: &str, password: &str) -> Result<String, Box<dyn std::error::Error>> {
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
    
    /// Upload a document and return its ID and expected content
    async fn upload_document(&self, content: &str, filename: &str) -> Result<(Uuid, String), Box<dyn std::error::Error>> {
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
        Ok((document.document_id, content.to_string()))
    }
    
    /// Get document details including OCR status
    async fn get_document_details(&self, doc_id: Uuid) -> Result<Value, Box<dyn std::error::Error>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let response = self.client
            .get(&format!("{}/api/documents/{}/ocr", get_base_url(), doc_id))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Failed to get document details: {}", response.text().await?).into());
        }
        
        let doc_data: Value = response.json().await?;
        Ok(doc_data)
    }
    
    /// Wait for OCR to complete for a document
    async fn wait_for_ocr(&self, doc_id: Uuid) -> Result<Value, Box<dyn std::error::Error>> {
        let start = Instant::now();
        
        while start.elapsed() < TIMEOUT {
            let doc_data = self.get_document_details(doc_id).await?;
            
            match doc_data["ocr_status"].as_str() {
                Some("completed") => {
                    println!("✅ OCR completed for document {}", doc_id);
                    return Ok(doc_data);
                },
                Some("failed") => {
                    return Err(format!("OCR failed for document {}: {}", 
                                     doc_id, 
                                     doc_data["ocr_error"].as_str().unwrap_or("unknown error")).into());
                },
                Some("processing") => {
                    println!("⏳ OCR still processing for document {}", doc_id);
                },
                _ => {
                    println!("📋 Document {} queued for OCR", doc_id);
                }
            }
            
            sleep(Duration::from_millis(200)).await;
        }
        
        Err(format!("OCR did not complete within {} seconds for document {}", TIMEOUT.as_secs(), doc_id).into())
    }
    
    /// Upload multiple documents simultaneously and track their OCR results
    async fn upload_documents_simultaneously(&self, documents: Vec<(&str, &str)>) -> Result<Vec<(Uuid, String, Value)>, Box<dyn std::error::Error>> {
        use futures::future::join_all;
        
        let token = self.token.as_ref().ok_or("Not authenticated")?.clone();
        
        // Create upload futures
        let upload_futures: Vec<_> = documents.into_iter()
            .map(|(content, filename)| {
                let content_owned = content.to_string();
                let filename_owned = filename.to_string();
                let client = self.client.clone();
                let token = token.clone();
                let base_url = get_base_url();
                
                async move {
                    // Create multipart form
                    let part = reqwest::multipart::Part::text(content_owned.clone())
                        .file_name(filename_owned.clone())
                        .mime_str("text/plain")?;
                    let form = reqwest::multipart::Form::new()
                        .part("file", part);
                    
                    let response = client
                        .post(&format!("{}/api/documents", base_url))
                        .header("Authorization", format!("Bearer {}", token))
                        .multipart(form)
                        .send()
                        .await?;
                    
                    if !response.status().is_success() {
                        return Err(format!("Upload failed: {}", response.text().await?).into());
                    }
                    
                    let document: DocumentUploadResponse = response.json().await?;
                    Ok::<(Uuid, String), Box<dyn std::error::Error>>((document.document_id, content_owned))
                }
            })
            .collect();
        
        // Execute all uploads concurrently
        let upload_results = join_all(upload_futures).await;
        
        // Collect successfully uploaded documents
        let mut uploaded_docs = Vec::new();
        for result in upload_results {
            let (doc_id, expected_content) = result?;
            println!("📄 Uploaded document: {}", doc_id);
            uploaded_docs.push((doc_id, expected_content));
        }
        
        // Create OCR waiting futures
        let ocr_futures: Vec<_> = uploaded_docs.into_iter()
            .map(|(doc_id, expected_content)| {
                let client = self.client.clone();
                let token = token.clone();
                let base_url = get_base_url();
                
                async move {
                    // Wait for OCR with polling
                    let start = Instant::now();
                    
                    while start.elapsed() < TIMEOUT {
                        let response = client
                            .get(&format!("{}/api/documents/{}/ocr", base_url, doc_id))
                            .header("Authorization", format!("Bearer {}", token))
                            .send()
                            .await?;
                        
                        if !response.status().is_success() {
                            return Err(format!("Failed to get document details: {}", response.text().await?).into());
                        }
                        
                        let doc_data: Value = response.json().await?;
                        
                        match doc_data["ocr_status"].as_str() {
                            Some("completed") => {
                                println!("✅ OCR completed for document {}", doc_id);
                                return Ok::<(Uuid, String, Value), Box<dyn std::error::Error>>((doc_id, expected_content, doc_data));
                            },
                            Some("failed") => {
                                return Err(format!("OCR failed for document {}: {}", 
                                                 doc_id, 
                                                 doc_data["ocr_error"].as_str().unwrap_or("unknown error")).into());
                            },
                            Some("processing") => {
                                println!("⏳ OCR still processing for document {}", doc_id);
                            },
                            _ => {
                                println!("📋 Document {} queued for OCR", doc_id);
                            }
                        }
                        
                        sleep(Duration::from_millis(200)).await;
                    }
                    
                    Err(format!("OCR did not complete within {} seconds for document {}", TIMEOUT.as_secs(), doc_id).into())
                }
            })
            .collect();
        
        // Execute all OCR waiting concurrently
        let ocr_results = join_all(ocr_futures).await;
        
        // Collect results
        let mut results = Vec::new();
        for result in ocr_results {
            results.push(result?);
        }
        
        Ok(results)
    }
}

#[tokio::test]
async fn test_concurrent_ocr_corruption() {
    println!("🧪 Starting OCR corruption test with concurrent file processing");
    
    let mut client = OcrTestClient::new();
    
    // Check server health
    if let Err(e) = client.check_server_health().await {
        panic!("Server not running at {}: {}", get_base_url(), e);
    }
    
    // Create test user
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let username = format!("ocr_corruption_test_{}", timestamp);
    let email = format!("ocr_corruption_{}@test.com", timestamp);
    
    let _token = client.register_and_login(&username, &email, "testpass123").await
        .expect("Failed to register and login");
    
    println!("✅ User registered: {}", username);
    
    // Create test documents with distinctive content
    let file_a_content = r#"
=== DOCUMENT A - IMPORTANT CONTRACT ===
Contract Number: CONTRACT-A-001
Party 1: Alice Corporation
Party 2: Bob Industries
Date: 2024-01-15
Amount: $50,000
Terms: This is the content for Document A. It contains specific legal text 
that should remain associated with Document A only. Any corruption would 
be immediately visible.

DOCUMENT A SIGNATURE: Alice Smith, CEO
UNIQUE IDENTIFIER FOR A: ALPHA-BRAVO-CHARLIE-001
"#;
    
    let file_b_content = r#"
=== DOCUMENT B - TECHNICAL SPECIFICATION ===
Specification ID: SPEC-B-002
Product: Widget Manufacturing System
Version: 2.0
Author: Technical Team B
Date: 2024-01-16

This is Document B containing technical specifications. It has completely 
different content from Document A. If OCR corruption occurs, Document A 
might end up with this technical content instead of its contract text.

DOCUMENT B SIGNATURE: Bob Johnson, CTO
UNIQUE IDENTIFIER FOR B: DELTA-ECHO-FOXTROT-002
"#;
    
    // Test documents to upload simultaneously
    let documents = vec![
        (file_a_content, "contract_a.txt"),
        (file_b_content, "specification_b.txt"),
    ];
    
    println!("📤 Uploading documents simultaneously...");
    
    let results = client.upload_documents_simultaneously(documents).await
        .expect("Failed to upload documents simultaneously");
    
    println!("🔍 Analyzing OCR results for corruption...");
    
    let mut corruption_detected = false;
    
    for (doc_id, expected_content, ocr_result) in results {
        let actual_ocr_text = ocr_result["ocr_text"].as_str().unwrap_or("");
        let filename = ocr_result["filename"].as_str().unwrap_or("unknown");
        
        println!("\n📋 Document: {} ({})", doc_id, filename);
        println!("📄 Expected content length: {} chars", expected_content.len());
        println!("🔤 Actual OCR text length: {} chars", actual_ocr_text.len());
        
        // Check for content mismatch (corruption)
        if filename.contains("contract_a") {
            // Document A should contain contract-specific terms
            let has_contract_content = actual_ocr_text.contains("CONTRACT-A-001") 
                && actual_ocr_text.contains("Alice Corporation")
                && actual_ocr_text.contains("ALPHA-BRAVO-CHARLIE-001");
                
            let has_spec_content = actual_ocr_text.contains("SPEC-B-002")
                || actual_ocr_text.contains("Widget Manufacturing")
                || actual_ocr_text.contains("DELTA-ECHO-FOXTROT-002");
            
            if !has_contract_content {
                println!("❌ CORRUPTION DETECTED: Document A missing its original contract content!");
                corruption_detected = true;
            }
            
            if has_spec_content {
                println!("❌ CORRUPTION DETECTED: Document A contains Document B's specification content!");
                corruption_detected = true;
            }
            
            if has_contract_content && !has_spec_content {
                println!("✅ Document A has correct content");
            }
        } else if filename.contains("specification_b") {
            // Document B should contain specification-specific terms
            let has_spec_content = actual_ocr_text.contains("SPEC-B-002")
                && actual_ocr_text.contains("Widget Manufacturing")
                && actual_ocr_text.contains("DELTA-ECHO-FOXTROT-002");
                
            let has_contract_content = actual_ocr_text.contains("CONTRACT-A-001")
                || actual_ocr_text.contains("Alice Corporation")
                || actual_ocr_text.contains("ALPHA-BRAVO-CHARLIE-001");
            
            if !has_spec_content {
                println!("❌ CORRUPTION DETECTED: Document B missing its original specification content!");
                corruption_detected = true;
            }
            
            if has_contract_content {
                println!("❌ CORRUPTION DETECTED: Document B contains Document A's contract content!");
                corruption_detected = true;
            }
            
            if has_spec_content && !has_contract_content {
                println!("✅ Document B has correct content");
            }
        }
        
        // Additional integrity checks
        if let Some(confidence) = ocr_result["ocr_confidence"].as_f64() {
            println!("📊 OCR Confidence: {:.1}%", confidence);
            if confidence < 50.0 {
                println!("⚠️  Low OCR confidence may indicate processing issues");
            }
        }
        
        if let Some(word_count) = ocr_result["ocr_word_count"].as_i64() {
            println!("📝 OCR Word Count: {}", word_count);
        }
        
        if let Some(processing_time) = ocr_result["ocr_processing_time_ms"].as_i64() {
            println!("⏱️  OCR Processing Time: {}ms", processing_time);
        }
    }
    
    if corruption_detected {
        panic!("🚨 OCR CORRUPTION DETECTED! FileA's content was overwritten with FileB's data or vice versa.");
    } else {
        println!("\n🎉 No OCR corruption detected - all documents retained their correct content!");
    }
}

#[tokio::test]
async fn test_high_volume_concurrent_ocr() {
    println!("🧪 Starting high-volume concurrent OCR test");
    
    let mut client = OcrTestClient::new();
    
    if let Err(e) = client.check_server_health().await {
        panic!("Server not running at {}: {}", get_base_url(), e);
    }
    
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let username = format!("high_volume_test_{}", timestamp);
    let email = format!("high_volume_{}@test.com", timestamp);
    
    let _token = client.register_and_login(&username, &email, "testpass123").await
        .expect("Failed to register and login");
    
    // Create 5 documents with unique identifiable content
    let mut documents = Vec::new();
    for i in 1..=5 {
        let content = format!(r#"
=== DOCUMENT {} - UNIQUE CONTENT ===
Document Number: DOC-{:03}
Unique Signature: SIGNATURE-{}-{}-{}
Content: This is document number {} with completely unique content.
Every document should retain its own unique signature and number.
Any mixing of content between documents indicates corruption.
Random data: {}
End of Document {}
"#, i, i, i, timestamp, i*7, timestamp * i, i, i);
        
        documents.push((content, format!("doc_{}.txt", i)));
    }
    
    println!("📤 Uploading {} documents simultaneously...", documents.len());
    
    let documents_ref: Vec<(&str, &str)> = documents.iter()
        .map(|(content, filename)| (content.as_str(), filename.as_str()))
        .collect();
    
    let results = client.upload_documents_simultaneously(documents_ref).await
        .expect("Failed to upload documents simultaneously");
    
    println!("🔍 Analyzing results for content mixing...");
    
    let mut all_signatures = Vec::new();
    let mut corruption_found = false;
    
    // Extract all unique signatures
    for i in 1..=5 {
        all_signatures.push(format!("SIGNATURE-{}-{}-{}", i, timestamp, i*7));
    }
    
    // Check each document for corruption
    for (doc_id, expected_content, ocr_result) in results {
        let actual_ocr_text = ocr_result["ocr_text"].as_str().unwrap_or("");
        let filename = ocr_result["filename"].as_str().unwrap_or("unknown");
        
        println!("📝 OCR Text for {}: {}", filename, actual_ocr_text);
        
        // Determine which document this should be based on filename
        if let Some(doc_num_str) = filename.strip_prefix("doc_").and_then(|s| s.strip_suffix(".txt")) {
            if let Ok(doc_num) = doc_num_str.parse::<i32>() {
                let expected_signature = format!("SIGNATURE-{}-{}-{}", doc_num, timestamp, doc_num*7);
                
                println!("\n📋 Checking Document {} ({})", doc_num, doc_id);
                
                // Check if it has its own signature
                let has_own_signature = actual_ocr_text.contains(&expected_signature);
                
                // Check if it has any other document's signature
                let mut has_other_signatures = Vec::new();
                for (i, sig) in all_signatures.iter().enumerate() {
                    if i + 1 != doc_num as usize && actual_ocr_text.contains(sig) {
                        has_other_signatures.push(i + 1);
                    }
                }
                
                if !has_own_signature {
                    println!("❌ CORRUPTION: Document {} missing its own signature!", doc_num);
                    corruption_found = true;
                }
                
                if !has_other_signatures.is_empty() {
                    println!("❌ CORRUPTION: Document {} contains signatures from documents: {:?}", doc_num, has_other_signatures);
                    corruption_found = true;
                }
                
                if has_own_signature && has_other_signatures.is_empty() {
                    println!("✅ Document {} has correct content", doc_num);
                }
            }
        }
    }
    
    if corruption_found {
        panic!("🚨 CONTENT CORRUPTION DETECTED in high-volume test!");
    } else {
        println!("\n🎉 High-volume test passed - no corruption detected!");
    }
}

#[tokio::test]
async fn test_rapid_sequential_uploads() {
    println!("🧪 Testing rapid sequential uploads for race conditions");
    
    let mut client = OcrTestClient::new();
    
    if let Err(e) = client.check_server_health().await {
        panic!("Server not running at {}: {}", get_base_url(), e);
    }
    
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let username = format!("rapid_test_{}", timestamp);
    let email = format!("rapid_{}@test.com", timestamp);
    
    let _token = client.register_and_login(&username, &email, "testpass123").await
        .expect("Failed to register and login");
    
    println!("📤 Uploading documents in rapid sequence...");
    
    // Upload documents one after another with minimal delay
    let mut doc_ids = Vec::new();
    let mut expected_contents = Vec::new();
    
    for i in 1..=3 {
        let content = format!("RAPID-TEST-DOCUMENT-{}-{}-UNIQUE-CONTENT", i, timestamp);
        let filename = format!("rapid_{}.txt", i);
        
        let (doc_id, expected) = client.upload_document(&content, &filename).await
            .expect("Failed to upload document");
        
        doc_ids.push(doc_id);
        expected_contents.push(expected);
        
        println!("📄 Uploaded rapid document {}: {}", i, doc_id);
        
        // Very short delay to create timing pressure
        sleep(Duration::from_millis(50)).await;
    }
    
    println!("⏳ Waiting for all OCR to complete...");
    
    // Wait for all to complete and check for corruption
    for (i, doc_id) in doc_ids.iter().enumerate() {
        let ocr_result = client.wait_for_ocr(*doc_id).await
            .expect("Failed to wait for OCR");
        
        let actual_text = ocr_result["ocr_text"].as_str().unwrap_or("");
        let expected_marker = format!("RAPID-TEST-DOCUMENT-{}", i + 1);
        
        if !actual_text.contains(&expected_marker) {
            panic!("🚨 RAPID UPLOAD CORRUPTION: Document {} missing its unique marker '{}'", 
                   doc_id, expected_marker);
        }
        
        // Check it doesn't contain other documents' markers
        for j in 1..=3 {
            if j != (i + 1) {
                let other_marker = format!("RAPID-TEST-DOCUMENT-{}", j);
                if actual_text.contains(&other_marker) {
                    panic!("🚨 RAPID UPLOAD CORRUPTION: Document {} contains marker from document {}", 
                           doc_id, j);
                }
            }
        }
        
        println!("✅ Rapid document {} has correct content", i + 1);
    }
    
    println!("🎉 Rapid sequential upload test passed!");
}