/*!
 * Debug OCR Test - Check what's actually happening with OCR text
 */

use reqwest::Client;
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use uuid::Uuid;

use readur::models::{DocumentResponse, CreateUser, LoginRequest, LoginResponse};

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
    
    // Upload 2 documents with very distinctive content
    let doc1_content = "DOCUMENT-ONE-UNIQUE-SIGNATURE-12345-ALPHA";
    let doc2_content = "DOCUMENT-TWO-UNIQUE-SIGNATURE-67890-BETA";
    
    let part1 = reqwest::multipart::Part::text(doc1_content.to_string())
        .file_name("debug_doc1.txt".to_string())
        .mime_str("text/plain")
        .expect("Valid mime type");
    let form1 = reqwest::multipart::Form::new().part("file", part1);
    
    let part2 = reqwest::multipart::Part::text(doc2_content.to_string())
        .file_name("debug_doc2.txt".to_string())
        .mime_str("text/plain")
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
    
    let doc2_response = client
        .post(&format!("{}/api/documents", get_base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(form2)
        .send()
        .await
        .expect("Upload should work");
    
    let doc1: DocumentResponse = doc1_response.json().await.expect("Valid JSON");
    let doc2: DocumentResponse = doc2_response.json().await.expect("Valid JSON");
    
    println!("📄 Document 1: {}", doc1.id);
    println!("📄 Document 2: {}", doc2.id);
    
    // Wait for OCR to complete
    let start = Instant::now();
    let mut doc1_completed = false;
    let mut doc2_completed = false;
    
    while start.elapsed() < TIMEOUT && (!doc1_completed || !doc2_completed) {
        // Check document 1
        if !doc1_completed {
            let response = client
                .get(&format!("{}/api/documents/{}/ocr", get_base_url(), doc1.id))
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await
                .expect("OCR endpoint should work");
            
            if response.status().is_success() {
                let ocr_data: Value = response.json().await.expect("Valid JSON");
                if ocr_data["ocr_status"].as_str() == Some("completed") {
                    doc1_completed = true;
                    println!("✅ Document 1 OCR completed");
                }
            }
        }
        
        // Check document 2
        if !doc2_completed {
            let response = client
                .get(&format!("{}/api/documents/{}/ocr", get_base_url(), doc2.id))
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await
                .expect("OCR endpoint should work");
            
            if response.status().is_success() {
                let ocr_data: Value = response.json().await.expect("Valid JSON");
                if ocr_data["ocr_status"].as_str() == Some("completed") {
                    doc2_completed = true;
                    println!("✅ Document 2 OCR completed");
                }
            }
        }
        
        sleep(Duration::from_millis(100)).await;
    }
    
    if !doc1_completed || !doc2_completed {
        panic!("OCR did not complete within timeout");
    }
    
    // Now get the actual OCR content and analyze it
    let doc1_ocr_response = client
        .get(&format!("{}/api/documents/{}/ocr", get_base_url(), doc1.id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("OCR endpoint should work");
    
    let doc2_ocr_response = client
        .get(&format!("{}/api/documents/{}/ocr", get_base_url(), doc2.id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("OCR endpoint should work");
    
    let doc1_ocr: Value = doc1_ocr_response.json().await.expect("Valid JSON");
    let doc2_ocr: Value = doc2_ocr_response.json().await.expect("Valid JSON");
    
    println!("\n🔍 DETAILED OCR ANALYSIS:");
    println!("=====================================");
    
    println!("\n📋 Document 1 Analysis:");
    println!("  - Expected content: {}", doc1_content);
    println!("  - OCR status: {}", doc1_ocr["ocr_status"].as_str().unwrap_or("unknown"));
    println!("  - OCR text: {:?}", doc1_ocr["ocr_text"]);
    println!("  - OCR text length: {}", doc1_ocr["ocr_text"].as_str().unwrap_or("").len());
    println!("  - OCR confidence: {:?}", doc1_ocr["ocr_confidence"]);
    println!("  - OCR word count: {:?}", doc1_ocr["ocr_word_count"]);
    
    println!("\n📋 Document 2 Analysis:");
    println!("  - Expected content: {}", doc2_content);
    println!("  - OCR status: {}", doc2_ocr["ocr_status"].as_str().unwrap_or("unknown"));
    println!("  - OCR text: {:?}", doc2_ocr["ocr_text"]);
    println!("  - OCR text length: {}", doc2_ocr["ocr_text"].as_str().unwrap_or("").len());
    println!("  - OCR confidence: {:?}", doc2_ocr["ocr_confidence"]);
    println!("  - OCR word count: {:?}", doc2_ocr["ocr_word_count"]);
    
    // Check for corruption
    let doc1_text = doc1_ocr["ocr_text"].as_str().unwrap_or("");
    let doc2_text = doc2_ocr["ocr_text"].as_str().unwrap_or("");
    
    let doc1_has_own_signature = doc1_text.contains("DOCUMENT-ONE-UNIQUE-SIGNATURE-12345-ALPHA");
    let doc1_has_other_signature = doc1_text.contains("DOCUMENT-TWO-UNIQUE-SIGNATURE-67890-BETA");
    let doc2_has_own_signature = doc2_text.contains("DOCUMENT-TWO-UNIQUE-SIGNATURE-67890-BETA");
    let doc2_has_other_signature = doc2_text.contains("DOCUMENT-ONE-UNIQUE-SIGNATURE-12345-ALPHA");
    
    println!("\n🚨 CORRUPTION ANALYSIS:");
    println!("  Doc1 has own signature: {}", doc1_has_own_signature);
    println!("  Doc1 has Doc2's signature: {}", doc1_has_other_signature);
    println!("  Doc2 has own signature: {}", doc2_has_own_signature);
    println!("  Doc2 has Doc1's signature: {}", doc2_has_other_signature);
    
    if doc1_text == doc2_text && !doc1_text.is_empty() {
        println!("❌ IDENTICAL OCR TEXT DETECTED - Documents have the same content!");
    }
    
    if doc1_text.is_empty() && doc2_text.is_empty() {
        println!("❌ EMPTY OCR TEXT - Both documents have no OCR content!");
    }
    
    if !doc1_has_own_signature || !doc2_has_own_signature {
        println!("❌ MISSING SIGNATURES - Documents don't contain their expected content!");
    }
    
    if doc1_has_other_signature || doc2_has_other_signature {
        println!("❌ CROSS-CONTAMINATION - Documents contain each other's content!");
    }
    
    if doc1_has_own_signature && doc2_has_own_signature && !doc1_has_other_signature && !doc2_has_other_signature {
        println!("✅ NO CORRUPTION DETECTED - All documents have correct content!");
    }
}