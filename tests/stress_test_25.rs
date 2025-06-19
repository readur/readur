/*!
 * Moderate Stress Test - 25 Documents for Complete Verification
 */

use reqwest::Client;
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use uuid::Uuid;
use futures;

use readur::models::{DocumentResponse, CreateUser, LoginRequest, LoginResponse};

fn get_base_url() -> String {
    std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8000".to_string())
}
const TIMEOUT: Duration = Duration::from_secs(120);

struct StressTester {
    client: Client,
    token: String,
}

impl StressTester {
    async fn new() -> Self {
        let client = Client::new();
        
        // Check server health
        client.get(&format!("{}/api/health", get_base_url()))
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .expect("Server should be running");
        
        // Create test user
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let username = format!("stress_25_{}", timestamp);
        let email = format!("stress_25_{}@test.com", timestamp);
        
        // Register user
        let user_data = CreateUser {
            username: username.clone(),
            email: email.clone(),
            password: "testpass123".to_string(),
            role: Some(readur::models::UserRole::User),
        };
        
        client.post(&format!("{}/api/auth/register", get_base_url()))
            .json(&user_data)
            .send()
            .await
            .expect("Registration should work");
        
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
        
        let login_result: LoginResponse = login_response.json().await.expect("Login should return JSON");
        let token = login_result.token;
        
        println!("✅ Stress tester initialized");
        
        Self { client, token }
    }
    
    async fn upload_document(&self, content: &str, filename: &str) -> DocumentResponse {
        let part = reqwest::multipart::Part::text(content.to_string())
            .file_name(filename.to_string())
            .mime_str("text/plain")
            .expect("Valid mime type");
        let form = reqwest::multipart::Form::new().part("file", part);
        
        let response = self.client
            .post(&format!("{}/api/documents", get_base_url()))
            .header("Authorization", format!("Bearer {}", self.token))
            .multipart(form)
            .send()
            .await
            .expect("Upload should work");
        
        response.json().await.expect("Valid JSON")
    }
    
    async fn wait_for_ocr_completion(&self, document_ids: &[Uuid]) -> Vec<Value> {
        let start = Instant::now();
        
        while start.elapsed() < TIMEOUT {
            let all_docs = self.get_all_documents().await;
            let completed = all_docs.iter()
                .filter(|doc| {
                    let doc_id_str = doc["id"].as_str().unwrap_or("");
                    let status = doc["ocr_status"].as_str().unwrap_or("");
                    document_ids.iter().any(|id| id.to_string() == doc_id_str) && status == "completed"
                })
                .count();
            
            if completed == document_ids.len() {
                return all_docs.into_iter()
                    .filter(|doc| {
                        let doc_id_str = doc["id"].as_str().unwrap_or("");
                        document_ids.iter().any(|id| id.to_string() == doc_id_str)
                    })
                    .collect();
            }
            
            sleep(Duration::from_millis(500)).await;
        }
        
        panic!("OCR processing did not complete within timeout");
    }
    
    async fn get_all_documents(&self) -> Vec<Value> {
        let response = self.client
            .get(&format!("{}/api/documents", get_base_url()))
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await
            .expect("Documents endpoint should work");
        
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
async fn stress_test_25_documents() {
    println!("🚀 MODERATE STRESS TEST: 25 DOCUMENTS");
    println!("======================================");
    
    let tester = StressTester::new().await;
    
    // Create 25 documents with unique content
    let mut documents = Vec::new();
    for i in 1..=25 {
        let content = format!("STRESS-DOC-{:02}-SIGNATURE-{:02}-UNIQUE-CONTENT", i, i);
        let filename = format!("stress_{:02}.txt", i);
        documents.push((content, filename));
    }
    
    println!("📊 Testing {} documents concurrently", documents.len());
    
    // Phase 1: Upload all documents concurrently
    println!("\n🏁 UPLOADING...");
    let upload_start = Instant::now();
    
    let uploaded_docs = futures::future::join_all(
        documents.iter().map(|(content, filename)| {
            tester.upload_document(content, filename)
        }).collect::<Vec<_>>()
    ).await;
    
    let upload_duration = upload_start.elapsed();
    println!("✅ {} uploads completed in {:?}", uploaded_docs.len(), upload_duration);
    
    // Phase 2: Wait for OCR completion
    println!("\n🔬 PROCESSING OCR...");
    let processing_start = Instant::now();
    let document_ids: Vec<Uuid> = uploaded_docs.iter().map(|doc| doc.id).collect();
    
    let final_docs = tester.wait_for_ocr_completion(&document_ids).await;
    let processing_duration = processing_start.elapsed();
    println!("✅ OCR processing completed in {:?}", processing_duration);
    
    // Phase 3: Corruption Analysis
    println!("\n📊 VERIFYING RESULTS...");
    let mut successful = 0;
    let mut corrupted = 0;
    let mut corruption_details = Vec::new();
    
    for (i, doc) in final_docs.iter().enumerate() {
        let expected_content = &documents[i].0;
        let actual_text = doc["ocr_text"].as_str().unwrap_or("");
        let doc_id = doc["id"].as_str().unwrap_or("");
        
        if actual_text == expected_content {
            successful += 1;
        } else {
            corrupted += 1;
            corruption_details.push((doc_id.to_string(), expected_content.clone(), actual_text.to_string()));
        }
    }
    
    // Final Results
    println!("\n🏆 STRESS TEST RESULTS");
    println!("======================");
    println!("📊 Total Documents: {}", documents.len());
    println!("✅ Successful: {}", successful);
    println!("❌ Corrupted: {}", corrupted);
    println!("📈 Success Rate: {:.1}%", (successful as f64 / documents.len() as f64) * 100.0);
    println!("⏱️  Upload Time: {:?}", upload_duration);
    println!("⏱️  OCR Time: {:?}", processing_duration);
    println!("⏱️  Total Time: {:?}", upload_duration + processing_duration);
    
    if corrupted == 0 {
        println!("\n🎉 STRESS TEST PASSED!");
        println!("🎯 ALL {} DOCUMENTS PROCESSED WITHOUT CORRUPTION!", documents.len());
        println!("🚀 HIGH CONCURRENCY OCR CORRUPTION ISSUE IS FULLY RESOLVED!");
    } else {
        println!("\n🚨 STRESS TEST FAILED!");
        println!("❌ CORRUPTION DETECTED IN {} DOCUMENTS:", corrupted);
        
        for (doc_id, expected, actual) in &corruption_details {
            println!("  📄 {}: expected '{}' got '{}'", doc_id, expected, actual);
        }
        
        panic!("CORRUPTION DETECTED in {} out of {} documents", corrupted, documents.len());
    }
}