/*!
 * Comprehensive Source Management Integration Tests
 * 
 * Tests complete CRUD operations and workflows for all source types:
 * - WebDAV sources
 * - S3 sources  
 * - Local Folder sources
 * 
 * Covers:
 * - Source creation, update, deletion
 * - Connection testing and validation
 * - Sync operations and status monitoring
 * - Error handling and edge cases
 * - Multi-user source isolation
 */

use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;
use uuid::Uuid;

use readur::models::{CreateUser, LoginRequest, LoginResponse, UserRole, SourceType};

fn get_base_url() -> String {
    std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8000".to_string())
}
const TIMEOUT: Duration = Duration::from_secs(30);

/// Test client for source management operations
struct SourceTestClient {
    client: Client,
    token: Option<String>,
    user_id: Option<String>,
}

impl SourceTestClient {
    fn new() -> Self {
        Self {
            client: Client::new(),
            token: None,
            user_id: None,
        }
    }
    
    /// Register and login a test user
    async fn register_and_login(&mut self, role: UserRole) -> Result<String, Box<dyn std::error::Error>> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let random_suffix = uuid::Uuid::new_v4().to_string().replace("-", "")[..8].to_string();
        let username = format!("source_test_{}_{}_{}", role.to_string(), timestamp, random_suffix);
        let email = format!("source_test_{}@example.com", timestamp);
        let password = "testpassword123";
        
        // Register user with retry logic
        let user_data = CreateUser {
            username: username.clone(),
            email: email.clone(),
            password: password.to_string(),
            role: Some(role),
        };
        
        let mut retry_count = 0;
        let register_response = loop {
            match self.client
                .post(&format!("{}/api/auth/register", get_base_url()))
                .json(&user_data)
                .timeout(Duration::from_secs(10))
                .send()
                .await
            {
                Ok(resp) => break resp,
                Err(e) => {
                    retry_count += 1;
                    if retry_count >= 3 {
                        return Err(format!("Registration failed after 3 retries: {}", e).into());
                    }
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        };
        
        if !register_response.status().is_success() {
            let status = register_response.status();
            let text = register_response.text().await.unwrap_or_else(|_| "No response body".to_string());
            return Err(format!("Registration failed with status {}: {}", status, text).into());
        }
        
        // Login to get token
        let login_data = LoginRequest {
            username: username.clone(),
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
        
        // Get user info to store user_id
        let me_response = self.client
            .get(&format!("{}/api/auth/me", get_base_url()))
            .header("Authorization", format!("Bearer {}", login_result.token))
            .send()
            .await?;
        
        if me_response.status().is_success() {
            let user_info: Value = me_response.json().await?;
            self.user_id = user_info["id"].as_str().map(|s| s.to_string());
        }
        
        Ok(login_result.token)
    }
    
    /// Create a WebDAV source
    async fn create_webdav_source(&self, name: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let source_data = json!({
            "name": name,
            "source_type": "webdav",
            "enabled": true,
            "config": {
                "server_url": "https://cloud.example.com/remote.php/dav/files/testuser/",
                "username": "testuser",
                "password": "testpass",
                "watch_folders": ["/Documents", "/Pictures"],
                "file_extensions": [".pdf", ".txt", ".docx", ".jpg", ".png"],
                "auto_sync": true,
                "sync_interval_minutes": 60,
                "server_type": "nextcloud"
            }
        });
        
        let response = self.client
            .post(&format!("{}/api/sources", get_base_url()))
            .header("Authorization", format!("Bearer {}", token))
            .json(&source_data)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("WebDAV source creation failed: {}", error_text).into());
        }
        
        let source: Value = response.json().await?;
        Ok(source)
    }
    
    /// Create an S3 source
    async fn create_s3_source(&self, name: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let source_data = json!({
            "name": name,
            "source_type": "s3",
            "enabled": true,
            "config": {
                "bucket_name": "test-documents-bucket",
                "region": "us-east-1",
                "access_key_id": "AKIAIOSFODNN7EXAMPLE",
                "secret_access_key": "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
                "prefix": "documents/",
                "endpoint_url": null,
                "watch_folders": ["/documents", "/uploads"],
                "auto_sync": true,
                "sync_interval_minutes": 120,
                "file_extensions": [".pdf", ".txt", ".docx"]
            }
        });
        
        let response = self.client
            .post(&format!("{}/api/sources", get_base_url()))
            .header("Authorization", format!("Bearer {}", token))
            .json(&source_data)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("S3 source creation failed: {}", error_text).into());
        }
        
        let source: Value = response.json().await?;
        Ok(source)
    }
    
    /// Create a Local Folder source
    async fn create_local_folder_source(&self, name: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        // Create the test directory first to ensure it exists
        std::fs::create_dir_all("/tmp/test_documents").ok();
        
        let source_data = json!({
            "name": name,
            "source_type": "local_folder",
            "enabled": true,
            "config": {
                "watch_folders": ["/tmp/test_documents"],
                "file_extensions": [".pdf", ".txt", ".jpg"],
                "auto_sync": true,
                "sync_interval_minutes": 30,
                "recursive": true,
                "follow_symlinks": false
            }
        });
        
        let response = self.client
            .post(&format!("{}/api/sources", get_base_url()))
            .header("Authorization", format!("Bearer {}", token))
            .json(&source_data)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Local folder source creation failed: {}", error_text).into());
        }
        
        let source: Value = response.json().await?;
        Ok(source)
    }
    
    /// Get all sources for the authenticated user
    async fn get_sources(&self) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let response = self.client
            .get(&format!("{}/api/sources", get_base_url()))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Get sources failed: {}", response.text().await?).into());
        }
        
        let sources: Vec<Value> = response.json().await?;
        Ok(sources)
    }
    
    /// Get a specific source by ID
    async fn get_source(&self, source_id: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let response = self.client
            .get(&format!("{}/api/sources/{}", get_base_url(), source_id))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Get source failed: {}", response.text().await?).into());
        }
        
        let source: Value = response.json().await?;
        Ok(source)
    }
    
    /// Update a source
    async fn update_source(&self, source_id: &str, updates: Value) -> Result<Value, Box<dyn std::error::Error>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let response = self.client
            .put(&format!("{}/api/sources/{}", get_base_url(), source_id))
            .header("Authorization", format!("Bearer {}", token))
            .json(&updates)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Update source failed: {}", response.text().await?).into());
        }
        
        let source: Value = response.json().await?;
        Ok(source)
    }
    
    /// Delete a source
    async fn delete_source(&self, source_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let response = self.client
            .delete(&format!("{}/api/sources/{}", get_base_url(), source_id))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Delete source failed: {}", response.text().await?).into());
        }
        
        Ok(())
    }
    
    /// Test source connection
    async fn test_source_connection(&self, source_id: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let response = self.client
            .post(&format!("{}/api/sources/{}/test", get_base_url(), source_id))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Test connection failed: {}", response.text().await?).into());
        }
        
        let result: Value = response.json().await?;
        Ok(result)
    }
    
    /// Start source sync
    async fn start_source_sync(&self, source_id: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let response = self.client
            .post(&format!("{}/api/sources/{}/sync", get_base_url(), source_id))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Start sync failed: {}", response.text().await?).into());
        }
        
        let result: Value = response.json().await?;
        Ok(result)
    }
    
    /// Stop source sync
    async fn stop_source_sync(&self, source_id: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let response = self.client
            .post(&format!("{}/api/sources/{}/sync/stop", get_base_url(), source_id))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Stop sync failed: {}", response.text().await?).into());
        }
        
        let result: Value = response.json().await?;
        Ok(result)
    }
    
    /// Estimate source crawl
    async fn estimate_source_crawl(&self, source_id: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let token = self.token.as_ref().ok_or("Not authenticated")?;
        
        let response = self.client
            .post(&format!("{}/api/sources/{}/estimate", get_base_url(), source_id))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Estimate crawl failed: {}", response.text().await?).into());
        }
        
        let result: Value = response.json().await?;
        Ok(result)
    }
}

#[tokio::test]
async fn test_webdav_source_crud_operations() {
    let mut client = SourceTestClient::new();
    
    // Register and login as regular user
    client.register_and_login(UserRole::User).await
        .expect("Failed to register and login");
    
    println!("✅ User registered and logged in");
    
    // Create WebDAV source
    let source = client.create_webdav_source("Test WebDAV Source").await
        .expect("Failed to create WebDAV source");
    
    let source_id = source["id"].as_str().expect("Source should have ID");
    println!("✅ WebDAV source created: {}", source_id);
    
    // Validate source structure
    assert_eq!(source["name"], "Test WebDAV Source");
    assert_eq!(source["source_type"], "webdav");
    assert_eq!(source["status"], "idle");
    assert!(source["config"]["server_url"].as_str().unwrap().contains("cloud.example.com"));
    assert_eq!(source["config"]["auto_sync"], true);
    assert_eq!(source["config"]["sync_interval_minutes"], 60);
    assert_eq!(source["enabled"], true);
    
    // Get source by ID
    let retrieved_source = client.get_source(source_id).await
        .expect("Failed to get source by ID");
    
    // The get_source endpoint returns a SourceWithStats structure
    let retrieved_source_data = &retrieved_source["source"];
    
    assert_eq!(retrieved_source_data["id"], source["id"]);
    assert_eq!(retrieved_source_data["name"], source["name"]);
    assert!(retrieved_source["recent_documents"].is_array());
    println!("✅ Source retrieved by ID");
    
    // Update source
    let updates = json!({
        "name": "Updated WebDAV Source",
        "enabled": true,
        "config": {
            "server_url": "https://cloud.example.com/remote.php/dav/files/testuser/",
            "username": "testuser",
            "password": "testpass",
            "watch_folders": ["/Documents", "/Pictures", "/Videos"],
            "file_extensions": [".pdf", ".txt", ".docx", ".jpg", ".png", ".mp4"],
            "auto_sync": false,
            "sync_interval_minutes": 120,
            "server_type": "nextcloud"
        }
    });
    
    let updated_source = client.update_source(source_id, updates).await
        .expect("Failed to update source");
    
    assert_eq!(updated_source["name"], "Updated WebDAV Source");
    assert_eq!(updated_source["config"]["auto_sync"], false);
    assert_eq!(updated_source["config"]["sync_interval_minutes"], 120);
    assert_eq!(updated_source["config"]["watch_folders"].as_array().unwrap().len(), 3);
    println!("✅ Source updated successfully");
    
    // List sources
    let sources = client.get_sources().await
        .expect("Failed to get sources list");
    
    assert!(sources.len() >= 1);
    let found_source = sources.iter().find(|s| s["id"] == source["id"])
        .expect("Created source should be in list");
    assert_eq!(found_source["name"], "Updated WebDAV Source");
    println!("✅ Source found in list");
    
    // Delete source
    client.delete_source(source_id).await
        .expect("Failed to delete source");
    
    // Verify deletion
    let sources_after_delete = client.get_sources().await
        .expect("Failed to get sources after delete");
    
    let deleted_source = sources_after_delete.iter().find(|s| s["id"] == source["id"]);
    assert!(deleted_source.is_none());
    println!("✅ Source deleted successfully");
    
    println!("🎉 WebDAV source CRUD operations test passed!");
}

#[tokio::test]
async fn test_s3_source_operations() {
    let mut client = SourceTestClient::new();
    
    client.register_and_login(UserRole::User).await
        .expect("Failed to register and login");
    
    // Create S3 source
    let source = client.create_s3_source("Test S3 Source").await
        .expect("Failed to create S3 source");
    
    let source_id = source["id"].as_str().expect("Source should have ID");
    println!("✅ S3 source created: {}", source_id);
    
    // Validate S3-specific configuration
    assert_eq!(source["source_type"], "s3");
    assert_eq!(source["config"]["bucket_name"], "test-documents-bucket");
    assert_eq!(source["config"]["region"], "us-east-1");
    assert_eq!(source["config"]["prefix"], "documents/");
    assert!(source["config"]["endpoint_url"].is_null());
    
    // Test with MinIO configuration update
    let minio_updates = json!({
        "name": "MinIO S3 Source",
        "config": {
            "bucket_name": "minio-test-bucket",
            "region": "us-east-1",
            "access_key_id": "minioadmin",
            "secret_access_key": "minioadmin",
            "prefix": "",
            "endpoint_url": "https://minio.example.com",
            "watch_folders": ["/"],
            "auto_sync": true,
            "sync_interval_minutes": 60,
            "file_extensions": [".pdf", ".jpg"]
        }
    });
    
    let updated_source = client.update_source(source_id, minio_updates).await
        .expect("Failed to update S3 source to MinIO");
    
    assert_eq!(updated_source["name"], "MinIO S3 Source");
    assert_eq!(updated_source["config"]["endpoint_url"], "https://minio.example.com");
    assert_eq!(updated_source["config"]["prefix"], "");
    println!("✅ S3 source updated to MinIO configuration");
    
    // Clean up
    client.delete_source(source_id).await
        .expect("Failed to delete S3 source");
    
    println!("🎉 S3 source operations test passed!");
}

#[tokio::test]
async fn test_local_folder_source_operations() {
    let mut client = SourceTestClient::new();
    
    client.register_and_login(UserRole::User).await
        .expect("Failed to register and login");
    
    // Create Local Folder source
    let source = client.create_local_folder_source("Test Local Folder").await
        .expect("Failed to create local folder source");
    
    let source_id = source["id"].as_str().expect("Source should have ID");
    println!("✅ Local folder source created: {}", source_id);
    
    // Validate Local Folder-specific configuration
    assert_eq!(source["source_type"], "local_folder");
    assert_eq!(source["config"]["watch_folders"][0], "/tmp/test_documents");
    assert_eq!(source["config"]["recursive"], true);
    assert_eq!(source["config"]["sync_interval_minutes"], 30);
    
    // Update with different path and settings
    let updates = json!({
        "name": "Updated Local Folder",
        "enabled": true,
        "config": {
            "watch_folders": ["/tmp/updated_documents", "/tmp/more_documents"],
            "file_extensions": [".pdf", ".txt", ".docx", ".xlsx"],
            "auto_sync": false,
            "sync_interval_minutes": 15,
            "recursive": false,
            "follow_symlinks": true
        }
    });
    
    let updated_source = client.update_source(source_id, updates).await
        .expect("Failed to update local folder source");
    
    assert_eq!(updated_source["config"]["watch_folders"][0], "/tmp/updated_documents");
    assert_eq!(updated_source["config"]["recursive"], false);
    assert_eq!(updated_source["config"]["auto_sync"], false);
    println!("✅ Local folder source updated");
    
    // Clean up
    client.delete_source(source_id).await
        .expect("Failed to delete local folder source");
    
    println!("🎉 Local folder source operations test passed!");
}

#[tokio::test]
async fn test_source_isolation_between_users() {
    let mut user1_client = SourceTestClient::new();
    let mut user2_client = SourceTestClient::new();
    
    // Register two different users
    user1_client.register_and_login(UserRole::User).await
        .expect("Failed to register user1");
    user2_client.register_and_login(UserRole::User).await
        .expect("Failed to register user2");
    
    println!("✅ Two users registered");
    
    // User 1 creates a source
    let user1_source = user1_client.create_webdav_source("User1 WebDAV").await
        .expect("Failed to create source for user1");
    
    let user1_source_id = user1_source["id"].as_str().unwrap();
    
    // User 2 creates a source
    let user2_source = user2_client.create_s3_source("User2 S3").await
        .expect("Failed to create source for user2");
    
    let user2_source_id = user2_source["id"].as_str().unwrap();
    
    println!("✅ Both users created sources");
    
    // User 1 should only see their own source
    let user1_sources = user1_client.get_sources().await
        .expect("Failed to get user1 sources");
    
    assert_eq!(user1_sources.len(), 1);
    assert_eq!(user1_sources[0]["id"], user1_source["id"]);
    assert_eq!(user1_sources[0]["name"], "User1 WebDAV");
    
    // User 2 should only see their own source
    let user2_sources = user2_client.get_sources().await
        .expect("Failed to get user2 sources");
    
    assert_eq!(user2_sources.len(), 1);
    assert_eq!(user2_sources[0]["id"], user2_source["id"]);
    assert_eq!(user2_sources[0]["name"], "User2 S3");
    
    println!("✅ Source isolation verified");
    
    // User 1 should not be able to access User 2's source
    let user1_access_user2_result = user1_client.get_source(user2_source_id).await;
    assert!(user1_access_user2_result.is_err());
    
    // User 2 should not be able to access User 1's source
    let user2_access_user1_result = user2_client.get_source(user1_source_id).await;
    assert!(user2_access_user1_result.is_err());
    
    println!("✅ Cross-user access prevention verified");
    
    // Clean up
    user1_client.delete_source(user1_source_id).await
        .expect("Failed to delete user1 source");
    user2_client.delete_source(user2_source_id).await
        .expect("Failed to delete user2 source");
    
    println!("🎉 Source isolation test passed!");
}

#[tokio::test]
async fn test_source_sync_operations() {
    let mut client = SourceTestClient::new();
    
    client.register_and_login(UserRole::User).await
        .expect("Failed to register and login");
    
    // Create a WebDAV source for sync testing
    let source = client.create_webdav_source("Sync Test Source").await
        .expect("Failed to create source");
    
    let source_id = source["id"].as_str().unwrap();
    println!("✅ Source created for sync testing");
    
    // Test connection (this will likely fail due to fake server, but should return structured response)
    let test_result = client.test_source_connection(source_id).await;
    // Don't assert success since we're using fake credentials, just verify it returns a result
    println!("✅ Connection test attempted: {:?}", test_result.is_ok());
    
    // Try to start sync
    let sync_result = client.start_source_sync(source_id).await;
    println!("✅ Sync start attempted: {:?}", sync_result.is_ok());
    
    // Try to get estimate
    let estimate_result = client.estimate_source_crawl(source_id).await;
    println!("✅ Crawl estimate attempted: {:?}", estimate_result.is_ok());
    
    // Try to stop sync
    let stop_result = client.stop_source_sync(source_id).await;
    println!("✅ Sync stop attempted: {:?}", stop_result.is_ok());
    
    // Get updated source to check if status changed
    let updated_source = client.get_source(source_id).await
        .expect("Failed to get updated source");
    
    // The get_source endpoint returns a SourceWithStats structure
    let source_data = &updated_source["source"];
    
    // Source should still exist with some status
    if let Some(status) = source_data["status"].as_str() {
        println!("✅ Source status after operations: {}", status);
    } else {
        println!("⚠️ Source status field is missing or null");
    }
    // The source should still exist
    assert!(source_data["id"].as_str().is_some());
    
    // Clean up
    client.delete_source(source_id).await
        .expect("Failed to delete source");
    
    println!("🎉 Source sync operations test passed!");
}

#[tokio::test]
async fn test_source_error_handling() {
    let mut client = SourceTestClient::new();
    
    client.register_and_login(UserRole::User).await
        .expect("Failed to register and login");
    
    // Test creating source with invalid configuration
    let invalid_source_data = json!({
        "name": "",  // Empty name should fail
        "source_type": "webdav",
        "config": {
            "server_url": "invalid-url",  // Invalid URL
            "username": "",  // Empty username
            "password": "",  // Empty password
        }
    });
    
    let token = client.token.as_ref().unwrap();
    let invalid_response = client.client
        .post(&format!("{}/api/sources", get_base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&invalid_source_data)
        .send()
        .await
        .expect("Request should complete");
    
    // Should return error for invalid data
    assert!(!invalid_response.status().is_success());
    println!("✅ Invalid source creation properly rejected");
    
    // Test accessing non-existent source
    let fake_id = Uuid::new_v4().to_string();
    let non_existent_result = client.get_source(&fake_id).await;
    assert!(non_existent_result.is_err());
    println!("✅ Non-existent source access properly handled");
    
    // Test operations without authentication
    let unauth_client = Client::new();
    let unauth_response = unauth_client
        .get(&format!("{}/api/sources", get_base_url()))
        .send()
        .await
        .expect("Request should complete");
    
    assert_eq!(unauth_response.status(), 401);
    println!("✅ Unauthenticated access properly rejected");
    
    println!("🎉 Source error handling test passed!");
}

#[tokio::test]
async fn test_all_source_types_comprehensive() {
    let mut client = SourceTestClient::new();
    
    client.register_and_login(UserRole::User).await
        .expect("Failed to register and login");
    
    // Create all three source types
    let _webdav_source = client.create_webdav_source("Comprehensive WebDAV").await
        .expect("Failed to create WebDAV source");
    
    let _s3_source = client.create_s3_source("Comprehensive S3").await
        .expect("Failed to create S3 source");
    
    let _local_source = client.create_local_folder_source("Comprehensive Local").await
        .expect("Failed to create local folder source");
    
    println!("✅ All three source types created");
    
    // Verify all sources are in the list
    let all_sources = client.get_sources().await
        .expect("Failed to get all sources");
    
    assert_eq!(all_sources.len(), 3);
    
    let webdav_found = all_sources.iter().any(|s| s["source_type"] == "webdav");
    let s3_found = all_sources.iter().any(|s| s["source_type"] == "s3");
    let local_found = all_sources.iter().any(|s| s["source_type"] == "local_folder");
    
    assert!(webdav_found && s3_found && local_found);
    println!("✅ All source types found in list");
    
    // Test operations on each source type
    for source in &all_sources {
        let source_id = source["id"].as_str().unwrap();
        let source_type = source["source_type"].as_str().unwrap();
        
        // Get individual source details
        let detailed_source = client.get_source(source_id).await
            .expect(&format!("Failed to get {} source details", source_type));
        
        assert_eq!(detailed_source["source"]["id"], source["id"]);
        assert_eq!(detailed_source["source"]["source_type"], source_type);
        
        // Test connection for each source
        let _test_result = client.test_source_connection(source_id).await;
        // Don't assert success since we're using test credentials
        
        println!("✅ {} source operations tested", source_type);
    }
    
    // Clean up all sources
    for source in &all_sources {
        let source_id = source["id"].as_str().unwrap();
        client.delete_source(source_id).await
            .expect("Failed to delete source during cleanup");
    }
    
    // Verify all sources deleted
    let sources_after_cleanup = client.get_sources().await
        .expect("Failed to get sources after cleanup");
    
    assert_eq!(sources_after_cleanup.len(), 0);
    
    println!("🎉 Comprehensive source types test passed!");
}