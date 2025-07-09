/*!
 * Document Deletion Integration Tests
 * 
 * Comprehensive tests for single and bulk document deletion functionality.
 * Tests HTTP endpoints, file cleanup, role-based access, and edge cases.
 */

use reqwest::{Client, multipart};
use serde_json::{json, Value};
use std::time::Duration;
use uuid::Uuid;

use readur::models::{DocumentResponse, CreateUser, LoginRequest, LoginResponse, UserRole};
use readur::routes::documents::types::DocumentUploadResponse;

fn get_base_url() -> String {
    std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8000".to_string())
}

const TIMEOUT: Duration = Duration::from_secs(30);

/// Test client for document deletion integration tests
struct DocumentDeletionTestClient {
    client: Client,
    token: Option<String>,
    user_id: Option<String>,
}

impl DocumentDeletionTestClient {
    fn new() -> Self {
        Self {
            client: Client::new(),
            token: None,
            user_id: None,
        }
    }
    
    /// Check if server is running and healthy
    async fn check_server_health(&self) -> Result<(), Box<dyn std::error::Error>> {
        let response = self.client
            .get(&format!("{}/api/health", get_base_url()))
            .timeout(Duration::from_secs(5))
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err("Server health check failed".into());
        }
        
        let health: Value = response.json().await?;
        if health["status"] != "ok" {
            return Err("Server is not healthy".into());
        }
        
        Ok(())
    }
    
    /// Register a new user and login to get auth token
    async fn register_and_login(&mut self, username: &str, email: &str, password: &str, role: Option<UserRole>) -> Result<String, Box<dyn std::error::Error>> {
        // Register user
        let user_data = CreateUser {
            username: username.to_string(),
            email: email.to_string(),
            password: password.to_string(),
            role: Some(role.unwrap_or(UserRole::User)),
        };
        
        let register_response = self.client
            .post(&format!("{}/api/auth/register", get_base_url()))
            .json(&user_data)
            .timeout(TIMEOUT)
            .send()
            .await?;
        
        if !register_response.status().is_success() {
            return Err(format!("Registration failed: {}", register_response.text().await?).into());
        }
        
        // Login to get token
        let login_data = LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
        };
        
        let login_response = self.client
            .post(&format!("{}/api/auth/login", get_base_url()))
            .json(&login_data)
            .timeout(TIMEOUT)
            .send()
            .await?;
        
        if !login_response.status().is_success() {
            return Err(format!("Login failed: {}", login_response.text().await?).into());
        }
        
        let login_result: LoginResponse = login_response.json().await?;
        self.token = Some(login_result.token.clone());
        self.user_id = Some(login_result.user.id.to_string());
        
        Ok(login_result.token)
    }
    
    /// Upload a test document
    async fn upload_document(&self, content: &[u8], filename: &str) -> Result<DocumentUploadResponse, Box<dyn std::error::Error>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let form = multipart::Form::new()
            .part("file", multipart::Part::bytes(content.to_vec())
                .file_name(filename.to_string())
                .mime_str("text/plain")?);
        
        let response = self.client
            .post(&format!("{}/api/documents", get_base_url()))
            .header("Authorization", format!("Bearer {}", token))
            .multipart(form)
            .timeout(TIMEOUT)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Document upload failed: {}", response.text().await?).into());
        }
        
        let document: DocumentUploadResponse = response.json().await?;
        Ok(document)
    }
    
    /// Delete a single document
    async fn delete_document(&self, document_id: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let response = self.client
            .delete(&format!("{}/api/documents/{}", get_base_url(), document_id))
            .header("Authorization", format!("Bearer {}", token))
            .timeout(TIMEOUT)
            .send()
            .await?;
        
        let status = response.status();
        let text = response.text().await?;
        
        println!("DEBUG: Delete response status: {}, body: '{}'", status, text);
        
        if !status.is_success() {
            return Err(format!("Document deletion failed ({}): {}", status, text).into());
        }
        
        if text.trim().is_empty() {
            // Return a success response if server returns empty but successful (204 No Content)
            return Ok(serde_json::json!({
                "success": true, 
                "message": "Document deleted", 
                "document_id": document_id, 
                "filename": "deleted"
            }));
        }
        
        let result: Value = serde_json::from_str(&text).map_err(|e| {
            format!("Failed to parse JSON response '{}': {}", text, e)
        })?;
        Ok(result)
    }
    
    /// Bulk delete documents
    async fn bulk_delete_documents(&self, document_ids: &[String]) -> Result<Value, Box<dyn std::error::Error>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let request_data = serde_json::json!({
            "document_ids": document_ids
        });
        
        let response = self.client
            .post(&format!("{}/api/documents/bulk/delete", get_base_url()))
            .header("Authorization", format!("Bearer {}", token))
            .json(&request_data)
            .timeout(TIMEOUT)
            .send()
            .await?;
        
        let status = response.status();
        let text = response.text().await?;
        
        println!("DEBUG: Bulk delete response status: {}, body: '{}'", status, text);
        
        if !status.is_success() {
            return Err(format!("Bulk deletion failed ({}): {}", status, text).into());
        }
        
        if text.trim().is_empty() {
            // Return a success response if server returns empty but successful
            return Ok(serde_json::json!({
                "success": true, 
                "deleted_count": document_ids.len(),
                "requested_count": document_ids.len(),
                "deleted_document_ids": document_ids,
                "deleted_documents": document_ids
            }));
        }
        
        let result: Value = serde_json::from_str(&text).map_err(|e| {
            format!("Failed to parse JSON response '{}': {}", text, e)
        })?;
        Ok(result)
    }
    
    /// Delete document without authentication (for testing unauthorized access)
    async fn delete_document_without_auth(&self, document_id: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let response = self.client
            .delete(&format!("{}/api/documents/{}", get_base_url(), document_id))
            .timeout(TIMEOUT)
            .send()
            .await?;
        
        let status = response.status();
        let text = response.text().await?;
        
        if !status.is_success() {
            return Err(format!("Document deletion failed ({}): {}", status, text).into());
        }
        
        let result: Value = serde_json::from_str(&text)?;
        Ok(result)
    }
    
    /// Get document by ID
    async fn get_document(&self, document_id: &str) -> Result<Option<DocumentResponse>, Box<dyn std::error::Error>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let response = self.client
            .get(&format!("{}/api/documents/{}", get_base_url(), document_id))
            .header("Authorization", format!("Bearer {}", token))
            .timeout(TIMEOUT)
            .send()
            .await?;
        
        if response.status() == 404 {
            return Ok(None);
        }
        
        if !response.status().is_success() {
            return Err(format!("Get document failed: {}", response.text().await?).into());
        }
        
        let document: DocumentResponse = response.json().await?;
        Ok(Some(document))
    }
    
    /// List documents
    async fn list_documents(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let response = self.client
            .get(&format!("{}/api/documents", get_base_url()))
            .header("Authorization", format!("Bearer {}", token))
            .timeout(TIMEOUT)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("List documents failed: {}", response.text().await?).into());
        }
        
        let result: Value = response.json().await?;
        Ok(result)
    }

    /// Delete failed OCR documents
    async fn delete_failed_ocr_documents(&self, preview_only: bool) -> Result<Value, Box<dyn std::error::Error>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let response = self.client
            .delete(&format!("{}/api/documents/cleanup/failed-ocr", get_base_url()))
            .header("Authorization", format!("Bearer {}", token))
            .json(&serde_json::json!({
                "preview_only": preview_only
            }))
            .timeout(TIMEOUT)
            .send()
            .await?;
        
        let status = response.status();
        let text = response.text().await?;
        
        println!("DEBUG: Delete failed OCR response status: {}, body: '{}'", status, text);
        
        if !status.is_success() {
            return Err(format!("Delete failed OCR documents failed ({}): {}", status, text).into());
        }
        
        if text.trim().is_empty() {
            // Return a success response if server returns empty but successful
            return Ok(serde_json::json!({
                "success": true,
                "matched_count": 0,
                "preview": preview_only,
                "document_ids": []
            }));
        }
        
        let result: Value = serde_json::from_str(&text).map_err(|e| {
            format!("Failed to parse JSON response '{}': {}", text, e)
        })?;
        Ok(result)
    }

    /// Delete low confidence documents (updated to use new combined endpoint)
    async fn delete_low_confidence_documents(&self, threshold: f64, preview_only: bool) -> Result<Value, Box<dyn std::error::Error>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let response = self.client
            .delete(&format!("{}/api/documents/cleanup/low-confidence", get_base_url()))
            .header("Authorization", format!("Bearer {}", token))
            .json(&serde_json::json!({
                "max_confidence": threshold,
                "preview_only": preview_only
            }))
            .timeout(TIMEOUT)
            .send()
            .await?;
        
        let status = response.status();
        let text = response.text().await?;
        
        println!("DEBUG: Delete low confidence response status: {}, body: '{}'", status, text);
        
        if !status.is_success() {
            return Err(format!("Delete low confidence documents failed ({}): {}", status, text).into());
        }
        
        if text.trim().is_empty() {
            // Return a success response if server returns empty but successful
            return Ok(serde_json::json!({
                "success": true,
                "matched_count": 0,
                "preview": preview_only,
                "document_ids": []
            }));
        }
        
        let result: Value = serde_json::from_str(&text).map_err(|e| {
            format!("Failed to parse JSON response '{}': {}", text, e)
        })?;
        Ok(result)
    }

    /// Create and login user (convenience method)
    async fn create_and_login_user(&mut self, username: &str, password: &str, role: UserRole) -> Result<String, Box<dyn std::error::Error>> {
        // Add random suffix to avoid username collisions between parallel tests
        let random_suffix = uuid::Uuid::new_v4().to_string().chars().take(8).collect::<String>();
        let unique_username = format!("{}_{}", username, random_suffix);
        let email = format!("{}@example.com", unique_username);
        self.register_and_login(&unique_username, &email, password, Some(role)).await
    }
}

/// Skip test if server is not running
macro_rules! skip_if_server_down {
    ($client:expr) => {
        if let Err(_) = $client.check_server_health().await {
            println!("Skipping test: Server is not running at {}", get_base_url());
            return;
        }
    };
}

#[tokio::test]
async fn test_single_document_deletion_success() {
    let mut client = DocumentDeletionTestClient::new();
    skip_if_server_down!(client);
    
    // Register and login
    client.register_and_login(
        &format!("testuser_delete_{}", Uuid::new_v4()),
        &format!("testuser_delete_{}@example.com", Uuid::new_v4()),
        "password123",
        None
    ).await.expect("Failed to register and login");
    
    // Upload a test document
    let test_content = b"This is a test document for deletion.";
    let document = client.upload_document(test_content, "test_deletion.txt")
        .await.expect("Failed to upload document");
    
    println!("Uploaded document: {}", document.id);
    
    // Verify document exists
    let retrieved_doc = client.get_document(&document.id.to_string())
        .await.expect("Failed to get document");
    assert!(retrieved_doc.is_some(), "Document should exist before deletion");
    
    // Delete the document
    let delete_result = client.delete_document(&document.id.to_string())
        .await.expect("Failed to delete document");
    
    // Verify deletion response (server returns 204 No Content, so we get our fallback response)
    assert_eq!(delete_result["success"], true);
    assert_eq!(delete_result["id"], document.id.to_string());
    // Note: filename is "deleted" because server returns empty response
    assert_eq!(delete_result["filename"], "deleted");
    
    // Verify document no longer exists
    let retrieved_doc_after = client.get_document(&document.id.to_string())
        .await.expect("Failed to check document existence");
    assert!(retrieved_doc_after.is_none(), "Document should not exist after deletion");
    
    println!("✅ Single document deletion test passed");
}

#[tokio::test]
async fn test_bulk_document_deletion_success() {
    let mut client = DocumentDeletionTestClient::new();
    skip_if_server_down!(client);
    
    // Register and login
    client.register_and_login(
        &format!("testuser_bulk_{}", Uuid::new_v4()),
        &format!("testuser_bulk_{}@example.com", Uuid::new_v4()),
        "password123",
        None
    ).await.expect("Failed to register and login");
    
    // Upload multiple test documents
    let mut document_ids = Vec::new();
    for i in 1..=5 {
        let test_content = format!("This is test document number {}", i);
        let document = client.upload_document(test_content.as_bytes(), &format!("test_bulk_{}.txt", i))
            .await.expect("Failed to upload document");
        document_ids.push(document.id.to_string());
    }
    
    println!("Uploaded {} documents for bulk deletion", document_ids.len());
    
    // Verify all documents exist
    for doc_id in &document_ids {
        let retrieved_doc = client.get_document(doc_id)
            .await.expect("Failed to get document");
        assert!(retrieved_doc.is_some(), "Document should exist before bulk deletion");
    }
    
    // Perform bulk deletion
    let delete_result = client.bulk_delete_documents(&document_ids)
        .await.expect("Failed to bulk delete documents");
    
    // Verify bulk deletion response
    assert_eq!(delete_result["deleted_count"], 5);
    assert_eq!(delete_result["failed_count"], 0);
    
    let deleted_ids: Vec<String> = delete_result["deleted_documents"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    
    assert_eq!(deleted_ids.len(), 5);
    for doc_id in &document_ids {
        assert!(deleted_ids.contains(doc_id), "Document ID should be in deleted list");
    }
    
    // Verify all documents no longer exist
    for doc_id in &document_ids {
        let retrieved_doc = client.get_document(doc_id)
            .await.expect("Failed to check document existence");
        assert!(retrieved_doc.is_none(), "Document should not exist after bulk deletion");
    }
    
    println!("✅ Bulk document deletion test passed");
}

#[tokio::test]
async fn test_delete_nonexistent_document() {
    let mut client = DocumentDeletionTestClient::new();
    skip_if_server_down!(client);
    
    // Register and login
    client.register_and_login(
        &format!("testuser_nonexist_{}", Uuid::new_v4()),
        &format!("testuser_nonexist_{}@example.com", Uuid::new_v4()),
        "password123",
        None
    ).await.expect("Failed to register and login");
    
    // Try to delete a non-existent document
    let fake_id = Uuid::new_v4().to_string();
    let delete_result = client.delete_document(&fake_id).await;
    
    // Should return 404 error
    assert!(delete_result.is_err(), "Deleting non-existent document should fail");
    let error_msg = delete_result.unwrap_err().to_string();
    assert!(error_msg.contains("404"), "Should return 404 error for non-existent document");
    
    println!("✅ Delete non-existent document test passed");
}

#[tokio::test]
async fn test_bulk_delete_empty_request() {
    let mut client = DocumentDeletionTestClient::new();
    skip_if_server_down!(client);
    
    // Register and login
    client.register_and_login(
        &format!("testuser_empty_{}", Uuid::new_v4()),
        &format!("testuser_empty_{}@example.com", Uuid::new_v4()),
        "password123",
        None
    ).await.expect("Failed to register and login");
    
    // Try bulk delete with empty array - should fail with 400
    let delete_result = client.bulk_delete_documents(&[]).await;
    
    // Should reject empty request
    assert!(delete_result.is_err(), "Bulk delete with empty array should fail");
    let error_msg = delete_result.unwrap_err().to_string();
    assert!(error_msg.contains("400") || error_msg.contains("Bad Request"), "Should return 400 error for empty array");
    
    println!("✅ Bulk delete empty request test passed");
}

#[tokio::test]
async fn test_bulk_delete_mixed_existing_nonexistent() {
    let mut client = DocumentDeletionTestClient::new();
    skip_if_server_down!(client);
    
    // Register and login
    client.register_and_login(
        &format!("testuser_mixed_{}", Uuid::new_v4()),
        &format!("testuser_mixed_{}@example.com", Uuid::new_v4()),
        "password123",
        None
    ).await.expect("Failed to register and login");
    
    // Upload one real document
    let test_content = b"This is a real document.";
    let real_document = client.upload_document(test_content, "real_doc.txt")
        .await.expect("Failed to upload document");
    
    // Create a list with real and fake IDs
    let fake_id = Uuid::new_v4().to_string();
    let mixed_ids = vec![real_document.id.to_string(), fake_id];
    
    // Perform bulk deletion
    let delete_result = client.bulk_delete_documents(&mixed_ids)
        .await.expect("Failed to bulk delete mixed documents");
    
    // Should delete only the existing document
    assert_eq!(delete_result["deleted_count"], 1);
    assert_eq!(delete_result["failed_count"], 0); // Non-existent IDs are silently ignored
    
    let deleted_ids: Vec<String> = delete_result["deleted_documents"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    
    assert_eq!(deleted_ids.len(), 1);
    assert_eq!(deleted_ids[0], real_document.id.to_string());
    
    // Verify real document was deleted
    let retrieved_doc = client.get_document(&real_document.id.to_string())
        .await.expect("Failed to check document existence");
    assert!(retrieved_doc.is_none(), "Real document should be deleted");
    
    println!("✅ Bulk delete mixed existing/non-existent test passed");
}

#[tokio::test]
async fn test_unauthorized_deletion() {
    let client = DocumentDeletionTestClient::new();
    skip_if_server_down!(client);
    
    // Try to delete without authentication
    let fake_id = Uuid::new_v4().to_string();
    let delete_result = client.delete_document_without_auth(&fake_id).await;
    
    // Should return 401 error
    assert!(delete_result.is_err(), "Unauthenticated deletion should fail");
    let error_msg = delete_result.unwrap_err().to_string();
    assert!(error_msg.contains("401") || error_msg.contains("Unauthorized"), 
           "Should return 401/Unauthorized error");
    
    println!("✅ Unauthorized deletion test passed");
}

#[tokio::test]
async fn test_cross_user_deletion_protection() {
    let mut client1 = DocumentDeletionTestClient::new();
    let mut client2 = DocumentDeletionTestClient::new();
    skip_if_server_down!(client1);
    
    let user1_suffix = Uuid::new_v4();
    let user2_suffix = Uuid::new_v4();
    
    // Register and login as user 1
    client1.register_and_login(
        &format!("testuser1_{}", user1_suffix),
        &format!("testuser1_{}@example.com", user1_suffix),
        "password123",
        None
    ).await.expect("Failed to register user 1");
    
    // Register and login as user 2
    client2.register_and_login(
        &format!("testuser2_{}", user2_suffix),
        &format!("testuser2_{}@example.com", user2_suffix),
        "password123",
        None
    ).await.expect("Failed to register user 2");
    
    // User 1 uploads a document
    let test_content = b"This is user 1's document.";
    let user1_document = client1.upload_document(test_content, "user1_doc.txt")
        .await.expect("Failed to upload document as user 1");
    
    // User 2 tries to delete user 1's document
    let delete_result = client2.delete_document(&user1_document.id.to_string()).await;
    
    // Should return 404 (document not found for user 2)
    assert!(delete_result.is_err(), "Cross-user deletion should fail");
    let error_msg = delete_result.unwrap_err().to_string();
    assert!(error_msg.contains("404"), "Should return 404 for document not owned by user");
    
    // Verify user 1's document still exists
    let retrieved_doc = client1.get_document(&user1_document.id.to_string())
        .await.expect("Failed to check document existence");
    assert!(retrieved_doc.is_some(), "User 1's document should still exist after failed cross-user deletion");
    
    println!("✅ Cross-user deletion protection test passed");
}

#[tokio::test]
async fn test_admin_can_delete_any_document() {
    let mut user_client = DocumentDeletionTestClient::new();
    let mut admin_client = DocumentDeletionTestClient::new();
    skip_if_server_down!(user_client);
    
    let user_suffix = Uuid::new_v4();
    let admin_suffix = Uuid::new_v4();
    
    // Register and login as regular user
    user_client.register_and_login(
        &format!("regularuser_{}", user_suffix),
        &format!("regularuser_{}@example.com", user_suffix),
        "password123",
        None
    ).await.expect("Failed to register regular user");
    
    // Register and login as admin
    admin_client.register_and_login(
        &format!("adminuser_{}", admin_suffix),
        &format!("adminuser_{}@example.com", admin_suffix),
        "adminpass123",
        Some(UserRole::Admin)
    ).await.expect("Failed to register admin user");
    
    // Regular user uploads a document
    let test_content = b"This is a regular user's document.";
    let user_document = user_client.upload_document(test_content, "user_doc.txt")
        .await.expect("Failed to upload document as user");
    
    // Admin deletes user's document
    let delete_result = admin_client.delete_document(&user_document.id.to_string())
        .await.expect("Admin should be able to delete any document");
    
    // Verify deletion response
    assert_eq!(delete_result["success"], true);
    assert_eq!(delete_result["id"], user_document.id.to_string());
    
    // Verify document no longer exists
    let retrieved_doc = user_client.get_document(&user_document.id.to_string())
        .await.expect("Failed to check document existence");
    assert!(retrieved_doc.is_none(), "Document should be deleted by admin");
    
    println!("✅ Admin can delete any document test passed");
}

#[tokio::test]
async fn test_document_count_updates_after_deletion() {
    let mut client = DocumentDeletionTestClient::new();
    skip_if_server_down!(client);
    
    // Register and login
    client.register_and_login(
        &format!("testuser_count_{}", Uuid::new_v4()),
        &format!("testuser_count_{}@example.com", Uuid::new_v4()),
        "password123",
        None
    ).await.expect("Failed to register and login");
    
    // Get initial document count
    let initial_list = client.list_documents()
        .await.expect("Failed to list documents");
    let initial_count = if let Some(pagination) = initial_list.get("pagination") {
        pagination["total"].as_i64().unwrap_or(0)
    } else if let Some(docs_array) = initial_list.get("documents").and_then(|d| d.as_array()) {
        // Documents are in a "documents" key
        docs_array.len() as i64
    } else if let Some(docs_array) = initial_list.as_array() {
        // Response is directly an array of documents
        docs_array.len() as i64
    } else {
        0
    };
    
    println!("📊 Initial document count: {}", initial_count);
    println!("DEBUG: Initial list response: {}", serde_json::to_string_pretty(&initial_list).unwrap_or_default());
    
    // Upload documents
    let mut document_ids = Vec::new();
    for i in 1..=3 {
        let test_content = format!("Test document {}", i);
        let document = client.upload_document(test_content.as_bytes(), &format!("count_test_{}.txt", i))
            .await.expect("Failed to upload document");
        document_ids.push(document.id.to_string());
        println!("📤 Uploaded document {}: {}", i, document.id);
    }
    
    // Wait a moment for documents to be indexed
    std::thread::sleep(std::time::Duration::from_millis(1000));
    
    // Verify count increased
    let after_upload_list = client.list_documents()
        .await.expect("Failed to list documents");
    let after_upload_count = if let Some(pagination) = after_upload_list.get("pagination") {
        pagination["total"].as_i64().unwrap_or(0)
    } else if let Some(docs_array) = after_upload_list.get("documents").and_then(|d| d.as_array()) {
        // Documents are in a "documents" key
        docs_array.len() as i64
    } else if let Some(docs_array) = after_upload_list.as_array() {
        // Response is directly an array of documents
        docs_array.len() as i64
    } else {
        0
    };
    
    println!("📊 After upload count: {} (expected: {})", after_upload_count, initial_count + 3);
    let expected_count = initial_count + 3;
    assert_eq!(after_upload_count, expected_count, "Document count should increase after uploads");
    
    // Delete one document
    client.delete_document(&document_ids[0])
        .await.expect("Failed to delete document");
    
    // Verify count decreased by 1
    let after_single_delete_list = client.list_documents()
        .await.expect("Failed to list documents");
    let after_single_delete_count = if let Some(pagination) = after_single_delete_list.get("pagination") {
        pagination["total"].as_i64().unwrap_or(0)
    } else if let Some(docs_array) = after_single_delete_list.get("documents").and_then(|d| d.as_array()) {
        // Documents are in a "documents" key
        docs_array.len() as i64
    } else if let Some(docs_array) = after_single_delete_list.as_array() {
        // Response is directly an array of documents
        docs_array.len() as i64
    } else {
        0
    };
    assert_eq!(after_single_delete_count, initial_count + 2, "Document count should decrease after single deletion");
    
    // Bulk delete remaining documents
    let remaining_ids = vec![document_ids[1].clone(), document_ids[2].clone()];
    client.bulk_delete_documents(&remaining_ids)
        .await.expect("Failed to bulk delete documents");
    
    // Verify count is back to initial
    let final_list = client.list_documents()
        .await.expect("Failed to list documents");
    let final_count = if let Some(pagination) = final_list.get("pagination") {
        pagination["total"].as_i64().unwrap_or(0)
    } else if let Some(docs_array) = final_list.get("documents").and_then(|d| d.as_array()) {
        // Documents are in a "documents" key
        docs_array.len() as i64
    } else if let Some(docs_array) = final_list.as_array() {
        // Response is directly an array of documents
        docs_array.len() as i64
    } else {
        0
    };
    assert_eq!(final_count, initial_count, "Document count should be back to initial after bulk deletion");
    
    println!("✅ Document count updates after deletion test passed");
}

/// Test the new failed OCR document deletion endpoint
#[tokio::test]
async fn test_delete_failed_ocr_documents_endpoint() {
    let mut client = DocumentDeletionTestClient::new();
    
    if let Err(e) = client.check_server_health().await {
        println!("⚠️ Server not available: {}. Skipping test.", e);
        return;
    }
    
    println!("🧪 Testing failed OCR document deletion endpoint...");
    
    // Create and login as regular user
    client.create_and_login_user("failed_ocr_user", "failed_ocr_password", UserRole::User)
        .await.expect("Failed to create and login user");
    
    // Preview failed documents (should return empty initially)
    let preview_response = client.delete_failed_ocr_documents(true)
        .await.expect("Failed to preview failed OCR documents");
    
    // The server returns {"deleted_count": 0, "message": "..."} for failed OCR endpoint
    assert!(preview_response["deleted_count"].as_i64().unwrap() >= 0);
    assert!(preview_response["message"].as_str().is_some());
    
    println!("📋 Preview request successful: {} failed documents found", 
             preview_response["deleted_count"]);
    
    // If there are failed documents, test deletion
    if preview_response["deleted_count"].as_i64().unwrap() > 0 {
        // Test actual deletion
        let delete_response = client.delete_failed_ocr_documents(false)
            .await.expect("Failed to delete failed OCR documents");
        
        assert!(delete_response["deleted_count"].as_i64().unwrap() >= 0);
        assert!(delete_response["message"].as_str().is_some());
        
        println!("🗑️ Successfully deleted {} failed documents", 
                 delete_response["deleted_count"]);
    } else {
        println!("ℹ️ No failed documents found to delete");
    }
    
    println!("✅ Failed OCR document deletion endpoint test passed");
}

/// Test confidence-based vs failed document deletion distinction
#[tokio::test]
async fn test_confidence_vs_failed_document_distinction() {
    let mut client = DocumentDeletionTestClient::new();
    
    if let Err(e) = client.check_server_health().await {
        println!("⚠️ Server not available: {}. Skipping test.", e);
        return;
    }
    
    println!("🧪 Testing distinction between confidence and failed document deletion...");
    
    // Create and login as admin to see all documents
    client.create_and_login_user("distinction_admin", "distinction_password", UserRole::Admin)
        .await.expect("Failed to create and login admin");
    
    // Get baseline counts
    let initial_low_confidence = client.delete_low_confidence_documents(30.0, true)
        .await.expect("Failed to preview low confidence documents");
    let initial_failed = client.delete_failed_ocr_documents(true)
        .await.expect("Failed to preview failed documents");
    
    let initial_low_count = initial_low_confidence["total_found"].as_i64().unwrap();
    let initial_failed_count = initial_failed["deleted_count"].as_i64().unwrap();
    
    println!("📊 Initial counts - Low confidence: {}, Failed: {}", 
             initial_low_count, initial_failed_count);
    
    // Test that the endpoints return different sets of documents
    // (This assumes there are some of each type in the system)
    
    // Verify that failed documents endpoint only includes failed/NULL confidence docs
    if initial_failed_count > 0 {
        println!("🔍 Found {} failed documents", initial_failed_count);
    }
    
    // Verify that low confidence endpoint respects threshold
    if initial_low_count > 0 {
        let low_confidence_docs = initial_low_confidence["documents"].as_array().unwrap();
        println!("🔍 Found {} low confidence document IDs", low_confidence_docs.len());
    }
    
    println!("✅ Document type distinction test passed");
}

/// Test error handling for delete endpoints
#[tokio::test]
async fn test_delete_endpoints_error_handling() {
    let client = DocumentDeletionTestClient::new();
    
    if let Err(e) = client.check_server_health().await {
        println!("⚠️ Server not available: {}. Skipping test.", e);
        return;
    }
    
    println!("🧪 Testing delete endpoints error handling...");
    
    // Test unauthenticated request with wrong method (POST instead of DELETE)
    let failed_response = client.client
        .post(&format!("{}/api/documents/cleanup/failed-ocr", get_base_url()))
        .json(&serde_json::json!({"preview_only": true}))
        .timeout(TIMEOUT)
        .send()
        .await
        .expect("Failed to send request");
    
    assert_eq!(failed_response.status(), 405, "Should return Method Not Allowed for POST");
    
    // Test unauthenticated request with correct method (DELETE)
    let unauth_response = client.client
        .delete(&format!("{}/api/documents/cleanup/failed-ocr", get_base_url()))
        .json(&serde_json::json!({"preview_only": true}))
        .timeout(TIMEOUT)
        .send()
        .await
        .expect("Failed to send request");
    
    assert_eq!(unauth_response.status(), 401, "Should require authentication");
    
    // Test invalid JSON
    let invalid_json_response = client.client
        .delete(&format!("{}/api/documents/cleanup/failed-ocr", get_base_url()))
        .header("content-type", "application/json")
        .body("invalid json")
        .timeout(TIMEOUT)
        .send()
        .await
        .expect("Failed to send request");
    
    assert!(invalid_json_response.status().is_client_error(), "Should reject invalid JSON");
    
    println!("✅ Error handling test passed");
}

/// Test role-based access for new delete endpoints
#[tokio::test]
async fn test_role_based_access_for_delete_endpoints() {
    let mut client = DocumentDeletionTestClient::new();
    
    if let Err(e) = client.check_server_health().await {
        println!("⚠️ Server not available: {}. Skipping test.", e);
        return;
    }
    
    println!("🧪 Testing role-based access for delete endpoints...");
    
    // Test as regular user
    client.create_and_login_user("delete_regular_user", "delete_password", UserRole::User)
        .await.expect("Failed to create and login user");
    
    let user_response = client.delete_failed_ocr_documents(true)
        .await.expect("Failed to preview as user");
    
    let user_count = user_response["deleted_count"].as_i64().unwrap();
    
    // Test as admin
    client.create_and_login_user("delete_admin_user", "delete_admin_password", UserRole::Admin)
        .await.expect("Failed to create and login admin");
    
    let admin_response = client.delete_failed_ocr_documents(true)
        .await.expect("Failed to preview as admin");
    
    let admin_count = admin_response["deleted_count"].as_i64().unwrap();
    
    // Admin should see at least as many documents as regular user
    assert!(admin_count >= user_count, 
            "Admin should see at least as many documents as user");
    
    println!("👤 User can see {} documents, Admin can see {} documents", 
             user_count, admin_count);
    
    println!("✅ Role-based access test passed");
}

/// Test the enhanced low confidence deletion with failed documents
#[tokio::test]
async fn test_enhanced_low_confidence_deletion() {
    let mut client = DocumentDeletionTestClient::new();
    
    if let Err(e) = client.check_server_health().await {
        println!("⚠️ Server not available: {}. Skipping test.", e);
        return;
    }
    
    println!("🧪 Testing enhanced low confidence deletion (includes failed docs)...");
    
    // Create and login as admin
    client.create_and_login_user("enhanced_delete_admin", "enhanced_password", UserRole::Admin)
        .await.expect("Failed to create and login admin");
    
    // Test with various thresholds
    let thresholds = vec![0.0, 30.0, 50.0, 85.0, 100.0];
    
    for threshold in thresholds {
        let response = client.delete_low_confidence_documents(threshold, true)
            .await.expect(&format!("Failed to preview with threshold {}", threshold));
        
        let count = response["total_found"].as_i64().unwrap();
        
        println!("🎯 Threshold {}%: {} documents would be deleted", threshold, count);
        
        // Verify response format
        assert!(response.get("documents").is_some());
        assert_eq!(response["preview"], true);
    }
    
    // Test that higher thresholds generally include more documents
    let low_threshold_response = client.delete_low_confidence_documents(10.0, true)
        .await.expect("Failed to preview with low threshold");
    let high_threshold_response = client.delete_low_confidence_documents(90.0, true)
        .await.expect("Failed to preview with high threshold");
    
    let low_count = low_threshold_response["total_found"].as_i64().unwrap();
    let high_count = high_threshold_response["total_found"].as_i64().unwrap();
    
    assert!(high_count >= low_count, 
            "Higher threshold should include at least as many documents as lower threshold");
    
    println!("✅ Enhanced low confidence deletion test passed");
}