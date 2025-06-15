/*!
 * Simple High-Concurrency Stress Test - Focus on Results Only
 */

use reqwest::Client;
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use uuid::Uuid;
use futures;

use readur::models::{DocumentResponse, CreateUser, LoginRequest, LoginResponse};

const BASE_URL: &str = "http://localhost:8000";
const TIMEOUT: Duration = Duration::from_secs(180);

struct SimpleStressTester {
    client: Client,
    token: String,
}

impl SimpleStressTester {
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
        let username = format!("stress_test_{}", timestamp);
        let email = format!("stress_test_{}@test.com", timestamp);
        
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
        
        println!("✅ Stress tester initialized for user: {}", username);
        
        Self { client, token }
    }
    
    async fn upload_document(&self, content: &str, filename: &str) -> DocumentResponse {
        let part = reqwest::multipart::Part::text(content.to_string())
            .file_name(filename.to_string())
            .mime_str("text/plain")
            .expect("Valid mime type");
        let form = reqwest::multipart::Form::new().part("file", part);
        
        let response = self.client
            .post(&format!("{}/api/documents", BASE_URL))
            .header("Authorization", format!("Bearer {}", self.token))
            .multipart(form)
            .send()
            .await
            .expect("Upload should work");
        
        if !response.status().is_success() {
            panic!("Upload failed: {}", response.text().await.unwrap_or_default());
        }
        
        response.json().await.expect("Valid JSON")
    }
    
    async fn wait_for_completion(&self, document_ids: &[Uuid]) -> Vec<Value> {
        let start = Instant::now();
        let mut last_completed = 0;
        
        while start.elapsed() < TIMEOUT {
            let all_docs = self.get_all_documents().await;
            let completed = all_docs.iter()
                .filter(|doc| {
                    let doc_id_str = doc["id"].as_str().unwrap_or("");
                    let status = doc["ocr_status"].as_str().unwrap_or("");
                    document_ids.iter().any(|id| id.to_string() == doc_id_str) && status == "completed"
                })
                .count();
            
            if completed != last_completed {
                last_completed = completed;
                let progress = (completed as f64 / document_ids.len() as f64) * 100.0;
                println!("  📊 Progress: {}/{} documents completed ({:.1}%)", 
                        completed, document_ids.len(), progress);
            }
            
            if completed == document_ids.len() {
                break;
            }
            
            sleep(Duration::from_secs(1)).await;
        }
        
        // Get final results
        let all_docs = self.get_all_documents().await;
        all_docs.into_iter()
            .filter(|doc| {
                let doc_id_str = doc["id"].as_str().unwrap_or("");
                document_ids.iter().any(|id| id.to_string() == doc_id_str)
            })
            .collect()
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
async fn stress_test_50_plus_documents() {
    println!("🚀 EXTREME STRESS TEST: 50+ DOCUMENTS");
    println!("=====================================");
    
    let tester = SimpleStressTester::new().await;
    
    // Create 50+ documents with unique content
    let mut documents = Vec::new();
    for i in 1..=55 {
        let content = format!("STRESS-TEST-DOCUMENT-{:03}-UNIQUE-SIGNATURE-{:03}", i, i);
        let filename = format!("stress_test_{:03}.txt", i);
        documents.push((content, filename));
    }
    
    println!("📊 Total Documents: {}", documents.len());
    
    // Phase 1: Upload all documents concurrently
    println!("\n🏁 PHASE 1: SIMULTANEOUS UPLOAD");
    let upload_start = Instant::now();
    
    let uploaded_docs = futures::future::join_all(
        documents.iter().map(|(content, filename)| {
            tester.upload_document(content, filename)
        }).collect::<Vec<_>>()
    ).await;
    
    let upload_duration = upload_start.elapsed();
    println!("✅ All {} documents uploaded in {:?}", uploaded_docs.len(), upload_duration);
    
    // Phase 2: Wait for OCR completion
    println!("\n🔬 PHASE 2: OCR PROCESSING");
    let processing_start = Instant::now();
    let document_ids: Vec<Uuid> = uploaded_docs.iter().map(|doc| doc.id).collect();
    
    let final_docs = tester.wait_for_completion(&document_ids).await;
    let processing_duration = processing_start.elapsed();
    println!("✅ All OCR processing completed in {:?}", processing_duration);
    
    // Phase 3: Corruption Analysis
    println!("\n📊 PHASE 3: CORRUPTION ANALYSIS");
    let mut successful = 0;
    let mut corrupted = 0;
    let mut corrupted_details = Vec::new();
    
    for (i, doc) in final_docs.iter().enumerate() {
        let expected_content = &documents[i].0;
        let actual_text = doc["ocr_text"].as_str().unwrap_or("");
        let status = doc["ocr_status"].as_str().unwrap_or("");
        let doc_id = doc["id"].as_str().unwrap_or("");
        
        if status == "completed" {
            if actual_text == expected_content {
                successful += 1;
            } else {
                corrupted += 1;
                corrupted_details.push((doc_id.to_string(), expected_content.clone(), actual_text.to_string()));
                
                // Only show first few corruption details to avoid spam
                if corrupted <= 3 {
                    println!("  ❌ CORRUPTION: {} expected '{}' got '{}'", doc_id, expected_content, actual_text);
                }
            }
        } else {
            println!("  ⚠️  NON-COMPLETED: {} status={}", doc_id, status);
        }
    }
    
    // Final Results
    println!("\n🏆 FINAL RESULTS");
    println!("================");
    println!("📊 Total Documents: {}", documents.len());
    println!("✅ Successful: {}", successful);
    println!("❌ Corrupted: {}", corrupted);
    println!("📈 Success Rate: {:.1}%", (successful as f64 / documents.len() as f64) * 100.0);
    println!("⏱️  Total Time: {:?}", upload_duration + processing_duration);
    
    if corrupted == 0 {
        println!("🎉 NO CORRUPTION DETECTED! ALL {} DOCUMENTS PROCESSED CORRECTLY!", documents.len());
    } else {
        println!("🚨 CORRUPTION DETECTED IN {} DOCUMENTS:", corrupted);
        
        // Analyze corruption patterns
        if corrupted_details.iter().all(|(_, _, actual)| actual.is_empty()) {
            println!("🔍 PATTERN: All corrupted documents have EMPTY content");
        } else if corrupted_details.len() > 1 && corrupted_details.iter().all(|(_, _, actual)| actual == &corrupted_details[0].2) {
            println!("🔍 PATTERN: All corrupted documents have IDENTICAL content: '{}'", corrupted_details[0].2);
        } else {
            println!("🔍 PATTERN: Mixed corruption types detected");
        }
        
        panic!("CORRUPTION DETECTED in {} out of {} documents", corrupted, documents.len());
    }
}