use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;
use uuid::Uuid;

use readur::models::{CreateUser, LoginRequest, LoginResponse, UserRole};

fn get_base_url() -> String {
    std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8000".to_string())
}

const TIMEOUT: Duration = Duration::from_secs(60);

struct OcrRetryRegressionTestHelper {
    client: Client,
    token: String,
}

impl OcrRetryRegressionTestHelper {
    async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let client = Client::new();
        
        // Health check
        let health_check = client
            .get(&format!("{}/api/health", get_base_url()))
            .timeout(Duration::from_secs(10))
            .send()
            .await;
        
        match health_check {
            Ok(response) => {
                if !response.status().is_success() {
                    let status = response.status();
                    let text = response.text().await.unwrap_or_else(|_| "Unable to read response".to_string());
                    return Err(format!("Health check failed with status {}: {}. Is the server running at {}?", status, text, get_base_url()).into());
                }
                println!("✅ Server health check passed at {}", get_base_url());
            }
            Err(e) => {
                eprintln!("❌ Cannot connect to server at {}: {}", get_base_url(), e);
                eprintln!("💡 To run integration tests, start the server first:");
                eprintln!("   cargo run");
                eprintln!("   Then run tests in another terminal:");
                eprintln!("   cargo test --test integration_ocr_retry_bug_regression_tests");
                return Err(format!("Server not reachable: {}", e).into());
            }
        }
        
        // Create and login as admin user
        let test_id = Uuid::new_v4().simple().to_string();
        let username = format!("test_admin_{}", &test_id[0..8]);
        let password = "test_password_123";
        let email = format!("{}@test.com", username);

        let create_user = CreateUser {
            username: username.clone(),
            password: password.to_string(),
            email: email.clone(),
            role: Some(UserRole::Admin),
        };

        let _create_response = client
            .post(&format!("{}/api/users", get_base_url()))
            .json(&create_user)
            .timeout(TIMEOUT)
            .send()
            .await?;

        let login_request = LoginRequest {
            username: username.clone(),
            password: password.to_string(),
        };

        let login_response = client
            .post(&format!("{}/api/auth/login", get_base_url()))
            .json(&login_request)
            .timeout(TIMEOUT)
            .send()
            .await?;

        if !login_response.status().is_success() {
            let status = login_response.status();
            let error_text = login_response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Login failed with status {}: {}", status, error_text).into());
        }

        let login_data: LoginResponse = login_response.json().await?;
        let token = login_data.token;

        Ok(Self { client, token })
    }

    async fn create_test_document(&self, filename: &str, ocr_status: &str) -> Result<Uuid, Box<dyn std::error::Error + Send + Sync>> {
        // Create a document directly in the database via API
        let document_data = json!({
            "filename": filename,
            "original_filename": filename,
            "mime_type": "application/pdf",
            "file_size": 1024,
            "ocr_status": ocr_status
        });

        let response = self.client
            .post(&format!("{}/api/internal/test/documents", get_base_url()))
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&document_data)
            .timeout(TIMEOUT)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Failed to create test document with status {}: {}", status, error_text).into());
        }

        let response_data: Value = response.json().await?;
        let doc_id_str = response_data["id"].as_str()
            .ok_or("Document ID not found in response")?;
        let doc_id = Uuid::parse_str(doc_id_str)?;
        
        Ok(doc_id)
    }

    async fn get_bulk_retry_preview(&self, mode: &str) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let request_body = json!({
            "mode": mode,
            "preview_only": true
        });

        let response = self.client
            .post(&format!("{}/api/documents/ocr/bulk-retry", get_base_url()))
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&request_body)
            .timeout(TIMEOUT)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Bulk retry preview failed with status {}: {}", status, error_text).into());
        }

        let response_data: Value = response.json().await?;
        Ok(response_data)
    }

    async fn execute_bulk_retry(&self, mode: &str) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let request_body = json!({
            "mode": mode,
            "preview_only": false
        });

        let response = self.client
            .post(&format!("{}/api/documents/ocr/bulk-retry", get_base_url()))
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&request_body)
            .timeout(TIMEOUT)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Bulk retry execution failed with status {}: {}", status, error_text).into());
        }

        let response_data: Value = response.json().await?;
        Ok(response_data)
    }
}

#[tokio::test]
async fn test_bulk_retry_only_targets_failed_documents_regression() {
    let helper = match OcrRetryRegressionTestHelper::new().await {
        Ok(h) => h,
        Err(e) => {
            eprintln!("⚠️  Skipping test due to setup failure: {}", e);
            return;
        }
    };

    println!("🧪 Testing regression: Bulk retry should only target failed documents");

    // Create a mix of documents with different OCR statuses
    let failed_doc1 = helper.create_test_document("failed_doc_1.pdf", "failed").await.expect("Failed to create failed document 1");
    let failed_doc2 = helper.create_test_document("failed_doc_2.pdf", "failed").await.expect("Failed to create failed document 2");
    let completed_doc = helper.create_test_document("completed_doc.pdf", "completed").await.expect("Failed to create completed document");
    let pending_doc = helper.create_test_document("pending_doc.pdf", "pending").await.expect("Failed to create pending document");

    println!("📄 Created test documents:");
    println!("  - Failed: {}, {}", failed_doc1, failed_doc2);
    println!("  - Completed: {}", completed_doc);
    println!("  - Pending: {}", pending_doc);

    // Test 1: Preview should only show failed documents
    println!("🔍 Testing bulk retry preview...");
    let preview_result = helper.get_bulk_retry_preview("all").await.expect("Failed to get preview");
    
    let matched_count = preview_result["matched_count"].as_u64().expect("matched_count not found");
    assert_eq!(matched_count, 2, "Preview should only match 2 failed documents, but matched {}", matched_count);

    let queued_count = preview_result["queued_count"].as_u64().unwrap_or(0);
    assert_eq!(queued_count, 0, "Preview should not queue any documents, but queued {}", queued_count);

    println!("✅ Preview correctly identified {} failed documents", matched_count);

    // Test 2: Execution should only process failed documents and not error on completed ones
    println!("🚀 Testing bulk retry execution...");
    let execution_result = helper.execute_bulk_retry("all").await.expect("Failed to execute bulk retry");

    let execution_matched_count = execution_result["matched_count"].as_u64().expect("matched_count not found in execution");
    let execution_queued_count = execution_result["queued_count"].as_u64().expect("queued_count not found in execution");

    assert_eq!(execution_matched_count, 2, "Execution should only match 2 failed documents, but matched {}", execution_matched_count);
    assert_eq!(execution_queued_count, 2, "Execution should queue 2 failed documents, but queued {}", execution_queued_count);

    let success = execution_result["success"].as_bool().expect("success not found in execution");
    assert!(success, "Bulk retry execution should succeed");

    println!("✅ Execution successfully processed {} failed documents", execution_queued_count);
    println!("🎉 Regression test passed: Bulk retry correctly targets only failed documents");
}

#[tokio::test]
async fn test_bulk_retry_no_database_constraint_errors() {
    let helper = match OcrRetryRegressionTestHelper::new().await {
        Ok(h) => h,
        Err(e) => {
            eprintln!("⚠️  Skipping test due to setup failure: {}", e);
            return;
        }
    };

    println!("🧪 Testing regression: No database constraint errors during retry");

    // Create only failed documents to ensure we test the constraint logic
    let failed_doc1 = helper.create_test_document("constraint_test_1.pdf", "failed").await.expect("Failed to create test document");
    let failed_doc2 = helper.create_test_document("constraint_test_2.pdf", "failed").await.expect("Failed to create test document");

    println!("📄 Created {} failed documents for constraint testing", 2);

    // Execute bulk retry - this should not produce any database constraint errors
    println!("🚀 Executing bulk retry to test database constraints...");
    let result = helper.execute_bulk_retry("all").await;

    match result {
        Ok(response) => {
            let success = response["success"].as_bool().expect("success field not found");
            let queued_count = response["queued_count"].as_u64().expect("queued_count not found");
            let message = response["message"].as_str().unwrap_or("No message");

            assert!(success, "Bulk retry should succeed without constraint errors");
            assert_eq!(queued_count, 2, "Should queue both failed documents");
            
            println!("✅ Bulk retry succeeded: queued {} documents", queued_count);
            println!("📝 Response message: {}", message);
        }
        Err(e) => {
            // Check if the error contains the specific constraint violation we were experiencing
            let error_msg = e.to_string();
            if error_msg.contains("Cannot modify completed OCR data") {
                panic!("❌ REGRESSION DETECTED: Database constraint error occurred: {}", error_msg);
            } else {
                panic!("❌ Unexpected error during bulk retry: {}", error_msg);
            }
        }
    }

    println!("🎉 Regression test passed: No database constraint errors during retry");
}

#[tokio::test]
async fn test_bulk_retry_with_mixed_documents_no_errors() {
    let helper = match OcrRetryRegressionTestHelper::new().await {
        Ok(h) => h,
        Err(e) => {
            eprintln!("⚠️  Skipping test due to setup failure: {}", e);
            return;
        }
    };

    println!("🧪 Testing regression: Mixed document statuses should not cause errors");

    // Create a realistic mix of documents that might exist in production
    let mut created_docs = Vec::new();
    
    // Create failed documents (should be included in retry)
    for i in 0..3 {
        let doc_id = helper.create_test_document(&format!("failed_{}.pdf", i), "failed").await.expect("Failed to create failed document");
        created_docs.push((doc_id, "failed"));
    }
    
    // Create completed documents (should be ignored, not cause errors)
    for i in 0..10 {
        let doc_id = helper.create_test_document(&format!("completed_{}.pdf", i), "completed").await.expect("Failed to create completed document");
        created_docs.push((doc_id, "completed"));
    }
    
    // Create pending documents (should be ignored)
    for i in 0..5 {
        let doc_id = helper.create_test_document(&format!("pending_{}.pdf", i), "pending").await.expect("Failed to create pending document");
        created_docs.push((doc_id, "pending"));
    }

    println!("📄 Created {} total documents with mixed statuses", created_docs.len());
    println!("  - 3 failed (should be retried)");
    println!("  - 10 completed (should be ignored)");
    println!("  - 5 pending (should be ignored)");

    // Test preview first
    println!("🔍 Testing preview with mixed document statuses...");
    let preview_result = helper.get_bulk_retry_preview("all").await.expect("Failed to get preview");
    
    let preview_matched = preview_result["matched_count"].as_u64().expect("matched_count not found");
    assert_eq!(preview_matched, 3, "Preview should only match 3 failed documents from mix");
    
    // Test execution
    println!("🚀 Testing execution with mixed document statuses...");
    let execution_result = helper.execute_bulk_retry("all").await.expect("Failed to execute bulk retry");

    let success = execution_result["success"].as_bool().expect("success not found");
    let matched_count = execution_result["matched_count"].as_u64().expect("matched_count not found");
    let queued_count = execution_result["queued_count"].as_u64().expect("queued_count not found");

    assert!(success, "Bulk retry should succeed with mixed document statuses");
    assert_eq!(matched_count, 3, "Should only match 3 failed documents from the mix");
    assert_eq!(queued_count, 3, "Should queue all 3 failed documents");

    println!("✅ Successfully handled mixed documents: matched {}, queued {}", matched_count, queued_count);
    println!("🎉 Regression test passed: Mixed document statuses handled correctly");
}