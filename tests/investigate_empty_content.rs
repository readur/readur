/*!
 * Investigate why high document volumes return empty OCR content
 */

use reqwest::Client;
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use uuid::Uuid;
use futures;

use readur::models::{DocumentResponse, CreateUser, LoginRequest, LoginResponse};

const BASE_URL: &str = "http://localhost:8000";

struct Investigator {
    client: Client,
    token: String,
}

impl Investigator {
    async fn new() -> Self {
        let client = Client::new();
        
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let username = format!("investigator_{}", timestamp);
        let email = format!("investigator_{}@test.com", timestamp);
        
        // Register and login
        let user_data = CreateUser {
            username: username.clone(),
            email: email.clone(),
            password: "testpass123".to_string(),
            role: Some(readur::models::UserRole::User),
        };
        
        client.post(&format!("{}/api/auth/register", BASE_URL))
            .json(&user_data)
            .send()
            .await
            .expect("Registration should work");
        
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
        
        let login_result: LoginResponse = login_response.json().await.expect("Login should return JSON");
        let token = login_result.token;
        
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
        
        response.json().await.expect("Valid JSON")
    }
    
    async fn get_document_details(&self, doc_id: &str) -> Value {
        let response = self.client
            .get(&format!("{}/api/documents/{}/ocr", BASE_URL, doc_id))
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await
            .expect("Should get document details");
        
        response.json().await.expect("Valid JSON")
    }
    
    async fn get_queue_stats(&self) -> Value {
        let response = self.client
            .get(&format!("{}/api/queue/stats", BASE_URL))
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await;
        
        match response {
            Ok(resp) => resp.json().await.unwrap_or_else(|_| serde_json::json!({"error": "Failed to parse"})),
            Err(_) => serde_json::json!({"error": "Failed to get queue stats"})
        }
    }
}

#[tokio::test]
async fn investigate_empty_content_issue() {
    println!("🔍 INVESTIGATING EMPTY CONTENT ISSUE");
    println!("===================================");
    
    let investigator = Investigator::new().await;
    
    // Test with different document counts to find the threshold
    let test_cases = vec![
        ("Low concurrency", 3),
        ("Medium concurrency", 10), 
        ("High concurrency", 20),
    ];
    
    for (test_name, doc_count) in test_cases {
        println!("\n📊 TEST: {} ({} documents)", test_name, doc_count);
        println!("{}=", "=".repeat(50));
        
        // Upload documents
        let mut documents = Vec::new();
        for i in 1..=doc_count {
            let content = format!("TEST-{}-CONTENT-{:02}", test_name.replace(" ", "_").to_uppercase(), i);
            let filename = format!("test_{}_{:02}.txt", test_name.replace(" ", "_"), i);
            documents.push((content, filename));
        }
        
        println!("📤 Uploading {} documents...", doc_count);
        let upload_start = Instant::now();
        
        let uploaded_docs = futures::future::join_all(
            documents.iter().map(|(content, filename)| {
                investigator.upload_document(content, filename)
            }).collect::<Vec<_>>()
        ).await;
        
        let upload_time = upload_start.elapsed();
        println!("✅ Upload completed in {:?}", upload_time);
        
        // Check queue stats immediately after upload
        let queue_stats = investigator.get_queue_stats().await;
        println!("📊 Queue stats after upload: {}", serde_json::to_string_pretty(&queue_stats).unwrap_or_default());
        
        // Wait for processing with detailed monitoring
        println!("🔄 Monitoring OCR processing...");
        let mut completed_count = 0;
        let process_start = Instant::now();
        
        while completed_count < doc_count && process_start.elapsed() < Duration::from_secs(60) {
            sleep(Duration::from_secs(2)).await;
            
            let mut current_completed = 0;
            let mut sample_results = Vec::new();
            
            for (i, doc) in uploaded_docs.iter().enumerate().take(3) { // Sample first 3 docs
                let details = investigator.get_document_details(&doc.id.to_string()).await;
                let status = details["ocr_status"].as_str().unwrap_or("unknown");
                let ocr_text = details["ocr_text"].as_str().unwrap_or("");
                let expected = &documents[i].0;
                
                if status == "completed" {
                    current_completed += 1;
                }
                
                sample_results.push((doc.id.to_string(), status.to_string(), expected.clone(), ocr_text.to_string()));
            }
            
            // Estimate total completed (this is rough but gives us an idea)
            let estimated_total_completed = if current_completed > 0 {
                (current_completed as f64 / 3.0 * doc_count as f64) as usize
            } else {
                0
            };
            
            if estimated_total_completed != completed_count {
                completed_count = estimated_total_completed;
                println!("  📈 Progress: ~{}/{} completed", completed_count, doc_count);
                
                // Show sample results
                for (doc_id, status, expected, actual) in sample_results {
                    if status == "completed" {
                        let is_correct = actual == expected;
                        let result_icon = if is_correct { "✅" } else if actual.is_empty() { "❌📄" } else { "❌🔄" };
                        println!("    {} {}: expected='{}' actual='{}'", result_icon, &doc_id[..8], expected, actual);
                    }
                }
            }
            
            if estimated_total_completed >= doc_count {
                break;
            }
        }
        
        let process_time = process_start.elapsed();
        println!("⏱️  Processing time: {:?}", process_time);
        
        // Final analysis
        let mut success_count = 0;
        let mut empty_count = 0;
        let mut other_corruption = 0;
        
        for (i, doc) in uploaded_docs.iter().enumerate() {
            let details = investigator.get_document_details(&doc.id.to_string()).await;
            let status = details["ocr_status"].as_str().unwrap_or("unknown");
            let ocr_text = details["ocr_text"].as_str().unwrap_or("");
            let expected = &documents[i].0;
            
            if status == "completed" {
                if ocr_text == expected {
                    success_count += 1;
                } else if ocr_text.is_empty() {
                    empty_count += 1;
                } else {
                    other_corruption += 1;
                }
            }
        }
        
        println!("\n📊 RESULTS for {} documents:", doc_count);
        println!("  ✅ Successful: {}", success_count);
        println!("  ❌ Empty content: {}", empty_count);
        println!("  🔄 Other corruption: {}", other_corruption);
        println!("  📈 Success rate: {:.1}%", (success_count as f64 / doc_count as f64) * 100.0);
        
        // Get final queue stats
        let final_queue_stats = investigator.get_queue_stats().await;
        println!("📊 Final queue stats: {}", serde_json::to_string_pretty(&final_queue_stats).unwrap_or_default());
        
        if empty_count > 0 {
            println!("⚠️  EMPTY CONTENT THRESHOLD FOUND AT {} DOCUMENTS", doc_count);
        }
    }
}