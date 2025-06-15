/*!
 * Debug OCR Pipeline Test - Trace every step to find corruption source
 */

use reqwest::Client;
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use uuid::Uuid;
use futures;

use readur::models::{DocumentResponse, CreateUser, LoginRequest, LoginResponse};

const BASE_URL: &str = "http://localhost:8000";
const TIMEOUT: Duration = Duration::from_secs(120);

struct PipelineDebugger {
    client: Client,
    token: String,
}

impl PipelineDebugger {
    async fn new() -> Self {
        let client = Client::new();
        
        // Check server health
        let response = client
            .get(&format!("{}/api/health", BASE_URL))
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
        let username = format!("pipeline_debug_{}", timestamp);
        let email = format!("pipeline_debug_{}@test.com", timestamp);
        
        // Register user
        let user_data = CreateUser {
            username: username.clone(),
            email: email.clone(),
            password: "testpass123".to_string(),
            role: Some(readur::models::UserRole::User),
        };
        
        let register_response = client
            .post(&format!("{}/api/auth/register", BASE_URL))
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
            .post(&format!("{}/api/auth/login", BASE_URL))
            .json(&login_data)
            .send()
            .await
            .expect("Login should work");
        
        if !login_response.status().is_success() {
            panic!("Login failed: {}", login_response.text().await.unwrap_or_default());
        }
        
        let login_result: LoginResponse = login_response.json().await.expect("Login should return JSON");
        let token = login_result.token;
        
        println!("✅ Pipeline debugger initialized for user: {}", username);
        
        Self { client, token }
    }
    
    async fn upload_document_with_debug(&self, content: &str, filename: &str) -> DocumentResponse {
        println!("\n📤 UPLOAD PHASE - Starting upload for: {}", filename);
        println!("  Content: {}", content);
        println!("  Content Length: {} bytes", content.len());
        
        let part = reqwest::multipart::Part::text(content.to_string())
            .file_name(filename.to_string())
            .mime_str("text/plain")
            .expect("Valid mime type");
        let form = reqwest::multipart::Form::new().part("file", part);
        
        let upload_start = Instant::now();
        let response = self.client
            .post(&format!("{}/api/documents", BASE_URL))
            .header("Authorization", format!("Bearer {}", self.token))
            .multipart(form)
            .send()
            .await
            .expect("Upload should work");
        
        let upload_duration = upload_start.elapsed();
        
        if !response.status().is_success() {
            panic!("Upload failed: {}", response.text().await.unwrap_or_default());
        }
        
        let document: DocumentResponse = response.json().await.expect("Valid JSON");
        
        println!("  ✅ Upload completed in {:?}", upload_duration);
        println!("  📄 Document ID: {}", document.id);
        println!("  📂 Filename: {}", document.filename);
        println!("  📏 File Size: {} bytes", document.file_size);
        println!("  🏷️  MIME Type: {}", document.mime_type);
        println!("  🔄 Initial OCR Status: {:?}", document.ocr_status);
        
        document
    }
    
    async fn trace_ocr_processing(&self, document_id: Uuid, expected_content: &str) -> Value {
        println!("\n🔍 OCR PROCESSING PHASE - Tracing for document: {}", document_id);
        
        let start = Instant::now();
        let mut last_status = String::new();
        let mut status_changes = Vec::new();
        let mut poll_count = 0;
        
        while start.elapsed() < TIMEOUT {
            poll_count += 1;
            
            let response = self.client
                .get(&format!("{}/api/documents/{}/ocr", BASE_URL, document_id))
                .header("Authorization", format!("Bearer {}", self.token))
                .send()
                .await
                .expect("OCR endpoint should work");
            
            if !response.status().is_success() {
                println!("  ❌ OCR endpoint error: {}", response.status());
                sleep(Duration::from_millis(100)).await;
                continue;
            }
            
            let ocr_data: Value = response.json().await.expect("Valid JSON");
            let current_status = ocr_data["ocr_status"].as_str().unwrap_or("unknown").to_string();
            
            // Track status changes
            if current_status != last_status {
                let elapsed = start.elapsed();
                status_changes.push((elapsed, current_status.clone()));
                println!("  📋 Status Change #{}: {} -> {} (after {:?})", 
                        status_changes.len(), last_status, current_status, elapsed);
                last_status = current_status.clone();
            }
            
            // Detailed logging every 10 polls or on status change
            if poll_count % 10 == 0 || status_changes.len() > 0 {
                println!("  🔄 Poll #{}: Status={}, HasText={}, TextLen={}", 
                        poll_count, 
                        current_status,
                        ocr_data["has_ocr_text"].as_bool().unwrap_or(false),
                        ocr_data["ocr_text"].as_str().unwrap_or("").len()
                );
                
                if let Some(confidence) = ocr_data["ocr_confidence"].as_f64() {
                    println!("    📊 Confidence: {:.1}%", confidence);
                }
                if let Some(word_count) = ocr_data["ocr_word_count"].as_i64() {
                    println!("    📝 Word Count: {}", word_count);
                }
                if let Some(error) = ocr_data["ocr_error"].as_str() {
                    println!("    ❌ Error: {}", error);
                }
            }
            
            // Check if processing is complete
            match current_status.as_str() {
                "completed" => {
                    println!("  ✅ OCR Processing completed after {:?} and {} polls", start.elapsed(), poll_count);
                    
                    // Detailed final analysis
                    let ocr_text = ocr_data["ocr_text"].as_str().unwrap_or("");
                    println!("\n  🔬 FINAL CONTENT ANALYSIS:");
                    println!("    Expected: {}", expected_content);
                    println!("    Actual:   {}", ocr_text);
                    println!("    Match: {}", ocr_text == expected_content);
                    println!("    Expected Length: {} chars", expected_content.len());
                    println!("    Actual Length:   {} chars", ocr_text.len());
                    
                    if ocr_text != expected_content {
                        println!("    ⚠️  CONTENT MISMATCH DETECTED!");
                        
                        // Character-by-character comparison
                        let expected_chars: Vec<char> = expected_content.chars().collect();
                        let actual_chars: Vec<char> = ocr_text.chars().collect();
                        
                        for (i, (e, a)) in expected_chars.iter().zip(actual_chars.iter()).enumerate() {
                            if e != a {
                                println!("      Diff at position {}: expected '{}' got '{}'", i, e, a);
                                break;
                            }
                        }
                    }
                    
                    return ocr_data;
                }
                "failed" => {
                    println!("  ❌ OCR Processing failed after {:?} and {} polls", start.elapsed(), poll_count);
                    return ocr_data;
                }
                _ => {
                    // Continue polling
                }
            }
            
            sleep(Duration::from_millis(50)).await;
        }
        
        panic!("OCR processing did not complete within {:?}", TIMEOUT);
    }
    
    async fn get_all_documents(&self) -> Vec<Value> {
        let response = self.client
            .get(&format!("{}/api/documents", BASE_URL))
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await
            .expect("Documents endpoint should work");
        
        if !response.status().is_success() {
            panic!("Failed to get documents: {}", response.status());
        }
        
        let data: Value = response.json().await.expect("Valid JSON");
        
        // Handle both paginated and non-paginated response formats
        match data {
            Value::Object(obj) if obj.contains_key("documents") => {
                obj["documents"].as_array().unwrap_or(&vec![]).clone()
            }
            Value::Array(arr) => arr,
            _ => vec![]
        }
    }
}

#[tokio::test]
async fn debug_high_concurrency_pipeline() {
    println!("🚀 STARTING HIGH-CONCURRENCY PIPELINE DEBUG");
    println!("============================================");
    
    let debugger = PipelineDebugger::new().await;
    
    // Create 5 documents with unique, easily identifiable content
    let documents = vec![
        ("DOC-ALPHA-001-UNIQUE-SIGNATURE-ALPHA", "debug_alpha.txt"),
        ("DOC-BRAVO-002-UNIQUE-SIGNATURE-BRAVO", "debug_bravo.txt"),
        ("DOC-CHARLIE-003-UNIQUE-SIGNATURE-CHARLIE", "debug_charlie.txt"),
        ("DOC-DELTA-004-UNIQUE-SIGNATURE-DELTA", "debug_delta.txt"),
        ("DOC-ECHO-005-UNIQUE-SIGNATURE-ECHO", "debug_echo.txt"),
    ];
    
    println!("\n📝 TEST DOCUMENTS:");
    for (i, (content, filename)) in documents.iter().enumerate() {
        println!("  {}: {} -> {}", i+1, filename, content);
    }
    
    // Phase 1: Upload all documents simultaneously
    println!("\n🏁 PHASE 1: SIMULTANEOUS UPLOAD");
    println!("================================");
    
    let upload_start = Instant::now();
    
    // Execute all uploads concurrently
    let uploaded_docs = futures::future::join_all(
        documents.iter().map(|(content, filename)| {
            debugger.upload_document_with_debug(content, filename)
        }).collect::<Vec<_>>()
    ).await;
    let upload_duration = upload_start.elapsed();
    
    println!("\n✅ ALL UPLOADS COMPLETED in {:?}", upload_duration);
    
    // Phase 2: Trace OCR processing for each document
    println!("\n🔬 PHASE 2: OCR PROCESSING TRACE");
    println!("================================");
    
    let mut ocr_tasks = Vec::new();
    
    for (i, doc) in uploaded_docs.iter().enumerate() {
        let doc_id = doc.id;
        let expected_content = documents[i].0.to_string();
        let debugger_ref = &debugger;
        
        let task = async move {
            let result = debugger_ref.trace_ocr_processing(doc_id, &expected_content).await;
            (doc_id, expected_content, result)
        };
        
        ocr_tasks.push(task);
    }
    
    // Process all OCR traces concurrently  
    let ocr_results = futures::future::join_all(ocr_tasks).await;
    
    // Phase 3: Comprehensive analysis
    println!("\n📊 PHASE 3: COMPREHENSIVE ANALYSIS");
    println!("===================================");
    
    let mut corrupted_docs = Vec::new();
    let mut successful_docs = Vec::new();
    
    for (doc_id, expected_content, ocr_result) in ocr_results {
        let actual_text = ocr_result["ocr_text"].as_str().unwrap_or("");
        let status = ocr_result["ocr_status"].as_str().unwrap_or("unknown");
        
        println!("\n📄 Document Analysis: {}", doc_id);
        println!("  Status: {}", status);
        println!("  Expected: {}", expected_content);
        println!("  Actual:   {}", actual_text);
        
        if status == "completed" {
            if actual_text == expected_content {
                println!("  ✅ CONTENT CORRECT");
                successful_docs.push(doc_id);
            } else {
                println!("  ❌ CONTENT CORRUPTED");
                corrupted_docs.push((doc_id, expected_content.clone(), actual_text.to_string()));
                
                // Check if it contains any other document's content
                for (other_expected, _) in &documents {
                    if other_expected != &expected_content && actual_text.contains(other_expected) {
                        println!("    🔄 Contains content from: {}", other_expected);
                    }
                }
            }
        } else {
            println!("  ⚠️  NON-COMPLETED STATUS: {}", status);
        }
    }
    
    // Phase 4: System state analysis
    println!("\n🏗️  PHASE 4: SYSTEM STATE ANALYSIS");
    println!("===================================");
    
    let all_docs = debugger.get_all_documents().await;
    println!("📋 Total documents in system: {}", all_docs.len());
    
    for doc in &all_docs {
        if let (Some(id), Some(filename), Some(status)) = (
            doc["id"].as_str(),
            doc["filename"].as_str(), 
            doc["ocr_status"].as_str()
        ) {
            println!("  📄 {}: {} -> {}", id, filename, status);
        }
    }
    
    // Final verdict
    println!("\n🏆 FINAL VERDICT");
    println!("================");
    println!("✅ Successful: {}", successful_docs.len());
    println!("❌ Corrupted:  {}", corrupted_docs.len());
    
    if corrupted_docs.is_empty() {
        println!("🎉 NO CORRUPTION DETECTED!");
    } else {
        println!("🚨 CORRUPTION DETECTED IN {} DOCUMENTS:", corrupted_docs.len());
        for (doc_id, expected, actual) in &corrupted_docs {
            println!("  📄 {}: expected '{}' got '{}'", doc_id, expected, actual);
        }
        
        // Try to identify patterns
        if corrupted_docs.iter().all(|(_, _, actual)| actual.is_empty()) {
            println!("🔍 PATTERN: All corrupted documents have EMPTY content");
        } else if corrupted_docs.iter().all(|(_, _, actual)| actual == &corrupted_docs[0].2) {
            println!("🔍 PATTERN: All corrupted documents have IDENTICAL content: '{}'", corrupted_docs[0].2);
        } else {
            println!("🔍 PATTERN: Mixed corruption types detected");
        }
        
        panic!("CORRUPTION DETECTED - see analysis above");
    }
}

#[tokio::test]
async fn debug_extreme_high_concurrency_pipeline() {
    println!("🚀 STARTING EXTREME HIGH-CONCURRENCY PIPELINE STRESS TEST");
    println!("========================================================");
    
    let debugger = PipelineDebugger::new().await;
    
    // Create 50+ documents with unique, easily identifiable content
    let mut documents = Vec::new();
    for i in 1..=55 {
        let content = format!("STRESS-TEST-DOCUMENT-{:03}-UNIQUE-SIGNATURE-{:03}", i, i);
        let filename = format!("stress_test_{:03}.txt", i);
        documents.push((content, filename));
    }
    
    println!("\n📝 STRESS TEST SETUP:");
    println!("  📊 Total Documents: {}", documents.len());
    println!("  🔄 Concurrent Processing: All {} documents simultaneously", documents.len());
    println!("  🎯 Goal: Zero corruption across all documents");
    
    // Phase 1: Upload all documents simultaneously
    println!("\n🏁 PHASE 1: SIMULTANEOUS UPLOAD");
    println!("================================");
    
    let upload_start = Instant::now();
    
    // Execute all uploads concurrently
    let uploaded_docs = futures::future::join_all(
        documents.iter().map(|(content, filename)| {
            debugger.upload_document_with_debug(content, filename)
        }).collect::<Vec<_>>()
    ).await;
    let upload_duration = upload_start.elapsed();
    
    println!("\n✅ ALL UPLOADS COMPLETED in {:?}", upload_duration);
    
    // Phase 2: Trace OCR processing for each document
    println!("\n🔬 PHASE 2: OCR PROCESSING TRACE");
    println!("================================");
    
    let mut ocr_tasks = Vec::new();
    
    for (i, doc) in uploaded_docs.iter().enumerate() {
        let doc_id = doc.id;
        let expected_content = documents[i].0.to_string();
        let debugger_ref = &debugger;
        
        let task = async move {
            let result = debugger_ref.trace_ocr_processing(doc_id, &expected_content).await;
            (doc_id, expected_content, result)
        };
        
        ocr_tasks.push(task);
    }
    
    // Process all OCR traces concurrently  
    let ocr_results = futures::future::join_all(ocr_tasks).await;
    
    // Phase 3: Comprehensive analysis
    println!("\n📊 PHASE 3: COMPREHENSIVE ANALYSIS");
    println!("===================================");
    
    let mut corrupted_docs = Vec::new();
    let mut successful_docs = Vec::new();
    
    for (doc_id, expected_content, ocr_result) in ocr_results {
        let actual_text = ocr_result["ocr_text"].as_str().unwrap_or("");
        let status = ocr_result["ocr_status"].as_str().unwrap_or("unknown");
        
        println!("\n📄 Document Analysis: {}", doc_id);
        println!("  Status: {}", status);
        println!("  Expected: {}", expected_content);
        println!("  Actual:   {}", actual_text);
        
        if status == "completed" {
            if actual_text == expected_content {
                println!("  ✅ CONTENT CORRECT");
                successful_docs.push(doc_id);
            } else {
                println!("  ❌ CONTENT CORRUPTED");
                corrupted_docs.push((doc_id, expected_content.clone(), actual_text.to_string()));
                
                // Check if it contains any other document's content
                for (other_expected, _) in &documents {
                    if other_expected != &expected_content && actual_text.contains(other_expected) {
                        println!("    🔄 Contains content from: {}", other_expected);
                    }
                }
            }
        } else {
            println!("  ⚠️  NON-COMPLETED STATUS: {}", status);
        }
    }
    
    // Phase 4: System state analysis
    println!("\n🏗️  PHASE 4: SYSTEM STATE ANALYSIS");
    println!("===================================");
    
    let all_docs = debugger.get_all_documents().await;
    println!("📋 Total documents in system: {}", all_docs.len());
    
    for doc in &all_docs {
        if let (Some(id), Some(filename), Some(status)) = (
            doc["id"].as_str(),
            doc["filename"].as_str(), 
            doc["ocr_status"].as_str()
        ) {
            println!("  📄 {}: {} -> {}", id, filename, status);
        }
    }
    
    // Final verdict
    println!("\n🏆 FINAL VERDICT");
    println!("================");
    println!("✅ Successful: {}", successful_docs.len());
    println!("❌ Corrupted:  {}", corrupted_docs.len());
    
    if corrupted_docs.is_empty() {
        println!("🎉 NO CORRUPTION DETECTED!");
    } else {
        println!("🚨 CORRUPTION DETECTED IN {} DOCUMENTS:", corrupted_docs.len());
        for (doc_id, expected, actual) in &corrupted_docs {
            println!("  📄 {}: expected '{}' got '{}'", doc_id, expected, actual);
        }
        
        // Try to identify patterns
        if corrupted_docs.iter().all(|(_, _, actual)| actual.is_empty()) {
            println!("🔍 PATTERN: All corrupted documents have EMPTY content");
        } else if corrupted_docs.iter().all(|(_, _, actual)| actual == &corrupted_docs[0].2) {
            println!("🔍 PATTERN: All corrupted documents have IDENTICAL content: '{}'", corrupted_docs[0].2);
        } else {
            println!("🔍 PATTERN: Mixed corruption types detected");
        }
        
        panic!("CORRUPTION DETECTED - see analysis above");
    }
}

#[tokio::test] 
async fn debug_document_upload_race_conditions() {
    println!("🔍 DEBUGGING DOCUMENT UPLOAD PROCESS");
    println!("====================================");
    
    let debugger = PipelineDebugger::new().await;
    
    // Upload same content with different filenames to test upload race conditions
    let same_content = "IDENTICAL-CONTENT-FOR-RACE-CONDITION-TEST";
    let task1 = debugger.upload_document_with_debug(same_content, "race1.txt");
    let task2 = debugger.upload_document_with_debug(same_content, "race2.txt");
    let task3 = debugger.upload_document_with_debug(same_content, "race3.txt");
    
    let (doc1, doc2, doc3) = futures::future::join3(task1, task2, task3).await;
    let docs = vec![doc1, doc2, doc3];
    
    println!("\n📊 UPLOAD RACE CONDITION ANALYSIS:");
    for (i, doc) in docs.iter().enumerate() {
        println!("  Doc {}: ID={}, Filename={}, Size={}", 
                i+1, doc.id, doc.filename, doc.file_size);
    }
    
    // Check if all documents have unique IDs
    let mut ids: Vec<_> = docs.iter().map(|d| d.id).collect();
    ids.sort();
    ids.dedup();
    
    if ids.len() == docs.len() {
        println!("✅ All documents have unique IDs");
    } else {
        println!("❌ DUPLICATE DOCUMENT IDs DETECTED!");
        panic!("Document upload race condition detected");
    }
}