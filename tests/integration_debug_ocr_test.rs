/*!
 * Debug OCR Test - Check what's actually happening with OCR text
 */

use reqwest::Client;
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::time::sleep;

use readur::models::{CreateUser, LoginRequest, LoginResponse};
use readur::routes::documents::types::DocumentUploadResponse;

fn get_base_url() -> String {
    std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8000".to_string())
}
const TIMEOUT: Duration = Duration::from_secs(60);

#[tokio::test]
async fn debug_ocr_content() {
    println!("🔍 Debugging OCR content to see what's actually stored");
    
    let client = Client::new();
    
    // Check server health
    let response = client
        .get(&format!("{}/api/health", get_base_url()))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .expect("Server should be running");
    
    if !response.status().is_success() {
        panic!("Server not healthy");
    }
    
    // Create test user
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let username = format!("debug_test_{}", timestamp);
    let email = format!("debug_{}@test.com", timestamp);
    
    // Register user
    let user_data = CreateUser {
        username: username.clone(),
        email: email.clone(),
        password: "testpass123".to_string(),
        role: Some(readur::models::UserRole::User),
    };
    
    let register_response = client
        .post(&format!("{}/api/auth/register", get_base_url()))
        .json(&user_data)
        .send()
        .await
        .expect("Registration should work");
    
    if !register_response.status().is_success() {
        panic!("Registration failed: {}", register_response.text().await.unwrap_or_default());
    }
    
    // Login
    let login_data = LoginRequest {
        username: username.clone(),
        password: "testpass123".to_string(),
    };
    
    let login_response = client
        .post(&format!("{}/api/auth/login", get_base_url()))
        .json(&login_data)
        .send()
        .await
        .expect("Login should work");
    
    if !login_response.status().is_success() {
        panic!("Login failed: {}", login_response.text().await.unwrap_or_default());
    }
    
    let login_result: LoginResponse = login_response.json().await.expect("Login should return JSON");
    let token = login_result.token;
    
    println!("✅ User logged in successfully");
    
    // Upload 2 test images that should trigger OCR processing
    let test_image1_path = "tests/test_images/test1.png";
    let test_image2_path = "tests/test_images/test2.jpg";
    
    let image1_data = std::fs::read(test_image1_path)
        .expect("Should be able to read test image 1");
    let image2_data = std::fs::read(test_image2_path)
        .expect("Should be able to read test image 2");
    
    let part1 = reqwest::multipart::Part::bytes(image1_data)
        .file_name("test1.png".to_string())
        .mime_str("image/png")
        .expect("Valid mime type");
    let form1 = reqwest::multipart::Form::new().part("file", part1);
    
    let part2 = reqwest::multipart::Part::bytes(image2_data)
        .file_name("test2.jpg".to_string())
        .mime_str("image/jpeg")
        .expect("Valid mime type");
    let form2 = reqwest::multipart::Form::new().part("file", part2);
    
    println!("📤 Uploading debug documents...");
    
    // Upload documents
    let doc1_response = client
        .post(&format!("{}/api/documents", get_base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(form1)
        .send()
        .await
        .expect("Upload should work");
    
    println!("📤 Document 1 upload response status: {}", doc1_response.status());
    if !doc1_response.status().is_success() {
        let status = doc1_response.status();
        let error_text = doc1_response.text().await.unwrap_or_else(|_| "No response body".to_string());
        panic!("Document 1 upload failed with status {}: {}", status, error_text);
    }
    
    let doc2_response = client
        .post(&format!("{}/api/documents", get_base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(form2)
        .send()
        .await
        .expect("Upload should work");
    
    println!("📤 Document 2 upload response status: {}", doc2_response.status());
    if !doc2_response.status().is_success() {
        let status = doc2_response.status();
        let error_text = doc2_response.text().await.unwrap_or_else(|_| "No response body".to_string());
        panic!("Document 2 upload failed with status {}: {}", status, error_text);
    }
    
    let doc1: DocumentUploadResponse = doc1_response.json().await.expect("Valid JSON for doc1");
    let doc2: DocumentUploadResponse = doc2_response.json().await.expect("Valid JSON for doc2");
    
    println!("📄 Document 1: {}", doc1.document_id);
    println!("📄 Document 2: {}", doc2.document_id);
    
    // Wait for OCR to complete
    let start = Instant::now();
    let mut doc1_completed = false;
    let mut doc2_completed = false;
    let mut last_status_print = Instant::now();
    
    while start.elapsed() < TIMEOUT && (!doc1_completed || !doc2_completed) {
        // Print progress every 10 seconds
        if last_status_print.elapsed() >= Duration::from_secs(10) {
            println!("⏳ OCR processing... elapsed: {:?}, Doc1: {}, Doc2: {}", 
                start.elapsed(), doc1_completed, doc2_completed);
            last_status_print = Instant::now();
        }
        // Check document 1
        if !doc1_completed {
            let response = client
                .get(&format!("{}/api/documents/{}/ocr", get_base_url(), doc1.document_id))
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await
                .expect("OCR endpoint should work");
            
            if response.status().is_success() {
                let ocr_data: Value = response.json().await.expect("Valid JSON");
                let current_status = ocr_data["ocr_status"].as_str().unwrap_or("unknown");
                println!("📊 Document 1 OCR status: {}", current_status);
                if current_status == "completed" {
                    doc1_completed = true;
                    println!("✅ Document 1 OCR completed");
                }
            } else {
                println!("❌ Document 1 OCR endpoint returned: {}", response.status());
            }
        }
        
        // Check document 2
        if !doc2_completed {
            let response = client
                .get(&format!("{}/api/documents/{}/ocr", get_base_url(), doc2.document_id))
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await
                .expect("OCR endpoint should work");
            
            if response.status().is_success() {
                let ocr_data: Value = response.json().await.expect("Valid JSON");
                let current_status = ocr_data["ocr_status"].as_str().unwrap_or("unknown");
                println!("📊 Document 2 OCR status: {}", current_status);
                if current_status == "completed" {
                    doc2_completed = true;
                    println!("✅ Document 2 OCR completed");
                }
            } else {
                println!("❌ Document 2 OCR endpoint returned: {}", response.status());
            }
        }
        
        sleep(Duration::from_millis(100)).await;
    }
    
    if !doc1_completed || !doc2_completed {
        println!("❌ OCR TIMEOUT DETAILS:");
        println!("  ⏱️  Total elapsed time: {:?}", start.elapsed());
        println!("  📄 Document 1 completed: {}", doc1_completed);
        println!("  📄 Document 2 completed: {}", doc2_completed);
        panic!("OCR did not complete within timeout");
    }
    
    // Now get the actual OCR content and analyze it
    let doc1_ocr_response = client
        .get(&format!("{}/api/documents/{}/ocr", get_base_url(), doc1.document_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("OCR endpoint should work");
    
    let doc2_ocr_response = client
        .get(&format!("{}/api/documents/{}/ocr", get_base_url(), doc2.document_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("OCR endpoint should work");
    
    let doc1_ocr: Value = doc1_ocr_response.json().await.expect("Valid JSON");
    let doc2_ocr: Value = doc2_ocr_response.json().await.expect("Valid JSON");
    
    println!("\n🔍 DETAILED OCR ANALYSIS:");
    println!("=====================================");
    
    println!("\n📋 Document 1 Analysis (test1.png):");
    println!("  - OCR status: {}", doc1_ocr["ocr_status"].as_str().unwrap_or("unknown"));
    println!("  - OCR text: {:?}", doc1_ocr["ocr_text"]);
    println!("  - OCR text length: {}", doc1_ocr["ocr_text"].as_str().unwrap_or("").len());
    println!("  - OCR confidence: {:?}", doc1_ocr["ocr_confidence"]);
    println!("  - OCR word count: {:?}", doc1_ocr["ocr_word_count"]);
    
    println!("\n📋 Document 2 Analysis (test2.jpg):");
    println!("  - OCR status: {}", doc2_ocr["ocr_status"].as_str().unwrap_or("unknown"));
    println!("  - OCR text: {:?}", doc2_ocr["ocr_text"]);
    println!("  - OCR text length: {}", doc2_ocr["ocr_text"].as_str().unwrap_or("").len());
    println!("  - OCR confidence: {:?}", doc2_ocr["ocr_confidence"]);
    println!("  - OCR word count: {:?}", doc2_ocr["ocr_word_count"]);
    
    // Check for basic OCR functionality
    let doc1_text = doc1_ocr["ocr_text"].as_str().unwrap_or("");
    let doc2_text = doc2_ocr["ocr_text"].as_str().unwrap_or("");
    
    println!("\n🔍 OCR ANALYSIS:");
    println!("  Document 1 has OCR text: {}", !doc1_text.is_empty());
    println!("  Document 2 has OCR text: {}", !doc2_text.is_empty());
    println!("  Documents have different content: {}", doc1_text != doc2_text);
    
    if doc1_text == doc2_text && !doc1_text.is_empty() {
        println!("❌ IDENTICAL OCR TEXT DETECTED - Documents have the same content!");
        println!("This suggests potential OCR corruption or cross-contamination.");
    }
    
    if doc1_text.is_empty() && doc2_text.is_empty() {
        println!("⚠️  EMPTY OCR TEXT - Both documents have no OCR content!");
        println!("This might be expected if the test images contain no readable text.");
    }
    
    if !doc1_text.is_empty() && !doc2_text.is_empty() && doc1_text != doc2_text {
        println!("✅ OCR PROCESSING SUCCESSFUL - Documents have different content!");
    }
}