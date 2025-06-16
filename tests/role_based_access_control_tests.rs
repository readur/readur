/*!
 * Role-Based Access Control (RBAC) Integration Tests
 * 
 * Tests comprehensive role-based access control including:
 * - Admin vs User permission boundaries
 * - Resource ownership and isolation
 * - Cross-user access prevention
 * - Privilege escalation prevention
 * - Administrative operations access control
 * - Data visibility and privacy
 * - Role transition scenarios
 * - Security boundary enforcement
 */

use reqwest::Client;
use serde_json::{json, Value};
use uuid::Uuid;

use readur::models::{CreateUser, LoginRequest, LoginResponse, UserRole};

const BASE_URL: &str = "http://localhost:8000";

/// Test client for RBAC scenarios with multiple user contexts
struct RBACTestClient {
    client: Client,
    admin_token: Option<String>,
    admin_user_id: Option<String>,
    user1_token: Option<String>,
    user1_user_id: Option<String>,
    user2_token: Option<String>,
    user2_user_id: Option<String>,
}

impl RBACTestClient {
    fn new() -> Self {
        Self {
            client: Client::new(),
            admin_token: None,
            admin_user_id: None,
            user1_token: None,
            user1_user_id: None,
            user2_token: None,
            user2_user_id: None,
        }
    }
    
    /// Setup all test users (admin, user1, user2)
    async fn setup_all_users(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        
        // Setup admin user
        let admin_username = format!("rbac_admin_{}", timestamp);
        let admin_email = format!("rbac_admin_{}@example.com", timestamp);
        let (admin_token, admin_id) = self.register_and_login_user(&admin_username, &admin_email, UserRole::Admin).await?;
        self.admin_token = Some(admin_token);
        self.admin_user_id = admin_id;
        
        // Setup first regular user
        let user1_username = format!("rbac_user1_{}", timestamp);
        let user1_email = format!("rbac_user1_{}@example.com", timestamp);
        let (user1_token, user1_id) = self.register_and_login_user(&user1_username, &user1_email, UserRole::User).await?;
        self.user1_token = Some(user1_token);
        self.user1_user_id = user1_id;
        
        // Setup second regular user
        let user2_username = format!("rbac_user2_{}", timestamp);
        let user2_email = format!("rbac_user2_{}@example.com", timestamp);
        let (user2_token, user2_id) = self.register_and_login_user(&user2_username, &user2_email, UserRole::User).await?;
        self.user2_token = Some(user2_token);
        self.user2_user_id = user2_id;
        
        Ok(())
    }
    
    /// Helper to register and login a single user
    async fn register_and_login_user(&self, username: &str, email: &str, role: UserRole) -> Result<(String, Option<String>), Box<dyn std::error::Error>> {
        let password = "rbacpassword123";
        
        // Register user
        let user_data = CreateUser {
            username: username.to_string(),
            email: email.to_string(),
            password: password.to_string(),
            role: Some(role),
        };
        
        let register_response = self.client
            .post(&format!("{}/api/auth/register", BASE_URL))
            .json(&user_data)
            .send()
            .await?;
        
        if !register_response.status().is_success() {
            return Err(format!("Registration failed for {}: {}", username, register_response.text().await?).into());
        }
        
        // Login to get token
        let login_data = LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
        };
        
        let login_response = self.client
            .post(&format!("{}/api/auth/login", BASE_URL))
            .json(&login_data)
            .send()
            .await?;
        
        if !login_response.status().is_success() {
            return Err(format!("Login failed for {}: {}", username, login_response.text().await?).into());
        }
        
        let login_result: LoginResponse = login_response.json().await?;
        
        // Get user info to extract user ID
        let me_response = self.client
            .get(&format!("{}/api/auth/me", BASE_URL))
            .header("Authorization", format!("Bearer {}", login_result.token))
            .send()
            .await?;
        
        let user_id = if me_response.status().is_success() {
            let user_info: Value = me_response.json().await?;
            user_info["id"].as_str().map(|s| s.to_string())
        } else {
            None
        };
        
        Ok((login_result.token, user_id))
    }
    
    /// Upload a document as a specific user
    async fn upload_document_as_user(&self, user: UserType, content: &str, filename: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let token = match user {
            UserType::Admin => self.admin_token.as_ref(),
            UserType::User1 => self.user1_token.as_ref(),
            UserType::User2 => self.user2_token.as_ref(),
        }.ok_or("User not set up")?;
        
        let part = reqwest::multipart::Part::text(content.to_string())
            .file_name(filename.to_string())
            .mime_str("text/plain")?;
        let form = reqwest::multipart::Form::new()
            .part("file", part);
        
        let response = self.client
            .post(&format!("{}/api/documents", BASE_URL))
            .header("Authorization", format!("Bearer {}", token))
            .multipart(form)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Upload failed: {}", response.text().await?).into());
        }
        
        let document: Value = response.json().await?;
        Ok(document)
    }
    
    /// Get documents list as a specific user
    async fn get_documents_as_user(&self, user: UserType) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
        let token = match user {
            UserType::Admin => self.admin_token.as_ref(),
            UserType::User1 => self.user1_token.as_ref(),
            UserType::User2 => self.user2_token.as_ref(),
        }.ok_or("User not set up")?;
        
        let response = self.client
            .get(&format!("{}/api/documents", BASE_URL))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Get documents failed: {}", response.text().await?).into());
        }
        
        let documents: Vec<Value> = response.json().await?;
        Ok(documents)
    }
    
    /// Try to access a specific document as a user
    async fn try_access_document(&self, user: UserType, document_id: &str) -> Result<reqwest::StatusCode, Box<dyn std::error::Error>> {
        let token = match user {
            UserType::Admin => self.admin_token.as_ref(),
            UserType::User1 => self.user1_token.as_ref(),
            UserType::User2 => self.user2_token.as_ref(),
        }.ok_or("User not set up")?;
        
        let response = self.client
            .get(&format!("{}/api/documents/{}/ocr", BASE_URL, document_id))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        Ok(response.status())
    }
    
    /// Create a source as a specific user
    async fn create_source_as_user(&self, user: UserType, source_name: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let token = match user {
            UserType::Admin => self.admin_token.as_ref(),
            UserType::User1 => self.user1_token.as_ref(),
            UserType::User2 => self.user2_token.as_ref(),
        }.ok_or("User not set up")?;
        
        let source_data = json!({
            "name": source_name,
            "source_type": "webdav",
            "config": {
                "server_url": "https://example.com",
                "username": "testuser",
                "password": "testpass",
                "auto_sync": false,
                "sync_interval_minutes": 60,
                "watch_folders": ["/Documents"],
                "file_extensions": [".pdf"]
            }
        });
        
        let response = self.client
            .post(&format!("{}/api/sources", BASE_URL))
            .header("Authorization", format!("Bearer {}", token))
            .json(&source_data)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Source creation failed: {}", response.text().await?).into());
        }
        
        let source: Value = response.json().await?;
        Ok(source)
    }
    
    /// Try to access a source as a user
    async fn try_access_source(&self, user: UserType, source_id: &str) -> Result<reqwest::StatusCode, Box<dyn std::error::Error>> {
        let token = match user {
            UserType::Admin => self.admin_token.as_ref(),
            UserType::User1 => self.user1_token.as_ref(),
            UserType::User2 => self.user2_token.as_ref(),
        }.ok_or("User not set up")?;
        
        let response = self.client
            .get(&format!("{}/api/sources/{}", BASE_URL, source_id))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        Ok(response.status())
    }
    
    /// Try to access admin endpoints as a user
    async fn try_admin_operation(&self, user: UserType, operation: AdminOperation) -> Result<reqwest::StatusCode, Box<dyn std::error::Error>> {
        let token = match user {
            UserType::Admin => self.admin_token.as_ref(),
            UserType::User1 => self.user1_token.as_ref(),
            UserType::User2 => self.user2_token.as_ref(),
        }.ok_or("User not set up")?;
        
        let response = match operation {
            AdminOperation::ListUsers => {
                self.client
                    .get(&format!("{}/api/users", BASE_URL))
                    .header("Authorization", format!("Bearer {}", token))
                    .send()
                    .await?
            }
            AdminOperation::CreateUser => {
                self.client
                    .post(&format!("{}/api/users", BASE_URL))
                    .header("Authorization", format!("Bearer {}", token))
                    .json(&json!({
                        "username": "test_admin_created",
                        "email": "admin_created@example.com",
                        "password": "password123",
                        "role": "user"
                    }))
                    .send()
                    .await?
            }
            AdminOperation::GetMetrics => {
                self.client
                    .get(&format!("{}/api/metrics", BASE_URL))
                    .header("Authorization", format!("Bearer {}", token))
                    .send()
                    .await?
            }
            AdminOperation::GetQueueStats => {
                self.client
                    .get(&format!("{}/api/queue/stats", BASE_URL))
                    .header("Authorization", format!("Bearer {}", token))
                    .send()
                    .await?
            }
            AdminOperation::RequeueFailedJobs => {
                self.client
                    .post(&format!("{}/api/queue/requeue-failed", BASE_URL))
                    .header("Authorization", format!("Bearer {}", token))
                    .send()
                    .await?
            }
        };
        
        Ok(response.status())
    }
    
    /// Try to modify another user's resource
    async fn try_modify_user_resource(&self, actor: UserType, target_user_id: &str) -> Result<reqwest::StatusCode, Box<dyn std::error::Error>> {
        let token = match actor {
            UserType::Admin => self.admin_token.as_ref(),
            UserType::User1 => self.user1_token.as_ref(),
            UserType::User2 => self.user2_token.as_ref(),
        }.ok_or("User not set up")?;
        
        let response = self.client
            .put(&format!("{}/api/users/{}", BASE_URL, target_user_id))
            .header("Authorization", format!("Bearer {}", token))
            .json(&json!({
                "username": "modified_user",
                "email": "modified@example.com",
                "role": "user"
            }))
            .send()
            .await?;
        
        Ok(response.status())
    }
}

#[derive(Clone, Copy)]
enum UserType {
    Admin,
    User1,
    User2,
}

#[derive(Clone, Copy)]
enum AdminOperation {
    ListUsers,
    CreateUser,
    GetMetrics,
    GetQueueStats,
    RequeueFailedJobs,
}

#[tokio::test]
async fn test_document_ownership_isolation() {
    println!("📄 Testing document ownership and isolation...");
    
    let mut client = RBACTestClient::new();
    client.setup_all_users().await
        .expect("Failed to setup test users");
    
    println!("✅ Setup complete: admin, user1, user2");
    
    // User1 uploads a document
    let user1_doc = client.upload_document_as_user(
        UserType::User1,
        "User1's private document content",
        "user1_private.txt"
    ).await.expect("Failed to upload User1 document");
    
    let user1_doc_id = user1_doc["id"].as_str().expect("Document should have ID");
    println!("✅ User1 uploaded document: {}", user1_doc_id);
    
    // User2 uploads a document
    let user2_doc = client.upload_document_as_user(
        UserType::User2,
        "User2's private document content",
        "user2_private.txt"
    ).await.expect("Failed to upload User2 document");
    
    let user2_doc_id = user2_doc["id"].as_str().expect("Document should have ID");
    println!("✅ User2 uploaded document: {}", user2_doc_id);
    
    // Test document list isolation
    let user1_docs = client.get_documents_as_user(UserType::User1).await
        .expect("Failed to get User1 documents");
    
    let user2_docs = client.get_documents_as_user(UserType::User2).await
        .expect("Failed to get User2 documents");
    
    // User1 should only see their own document
    let user1_sees_own = user1_docs.iter().any(|d| d["id"] == user1_doc_id);
    let user1_sees_user2 = user1_docs.iter().any(|d| d["id"] == user2_doc_id);
    
    assert!(user1_sees_own, "User1 should see their own document");
    assert!(!user1_sees_user2, "User1 should NOT see User2's document");
    
    // User2 should only see their own document
    let user2_sees_own = user2_docs.iter().any(|d| d["id"] == user2_doc_id);
    let user2_sees_user1 = user2_docs.iter().any(|d| d["id"] == user1_doc_id);
    
    assert!(user2_sees_own, "User2 should see their own document");
    assert!(!user2_sees_user1, "User2 should NOT see User1's document");
    
    println!("✅ Document list isolation verified");
    
    // Test direct document access
    let user1_access_own = client.try_access_document(UserType::User1, user1_doc_id).await
        .expect("Failed to test User1 access to own document");
    
    let user1_access_user2 = client.try_access_document(UserType::User1, user2_doc_id).await
        .expect("Failed to test User1 access to User2 document");
    
    assert!(user1_access_own.is_success(), "User1 should access their own document");
    assert!(!user1_access_user2.is_success(), "User1 should NOT access User2's document");
    
    let user2_access_own = client.try_access_document(UserType::User2, user2_doc_id).await
        .expect("Failed to test User2 access to own document");
    
    let user2_access_user1 = client.try_access_document(UserType::User2, user1_doc_id).await
        .expect("Failed to test User2 access to User1 document");
    
    assert!(user2_access_own.is_success(), "User2 should access their own document");
    assert!(!user2_access_user1.is_success(), "User2 should NOT access User1's document");
    
    println!("✅ Direct document access isolation verified");
    
    // Test admin access to all documents
    let admin_access_user1 = client.try_access_document(UserType::Admin, user1_doc_id).await
        .expect("Failed to test admin access to User1 document");
    
    let admin_access_user2 = client.try_access_document(UserType::Admin, user2_doc_id).await
        .expect("Failed to test admin access to User2 document");
    
    // Admin access depends on implementation - might have access or might not
    println!("ℹ️  Admin access to User1 doc: {}", admin_access_user1);
    println!("ℹ️  Admin access to User2 doc: {}", admin_access_user2);
    
    println!("🎉 Document ownership isolation test passed!");
}

#[tokio::test]
async fn test_source_ownership_isolation() {
    println!("🗂️ Testing source ownership and isolation...");
    
    let mut client = RBACTestClient::new();
    client.setup_all_users().await
        .expect("Failed to setup test users");
    
    println!("✅ Setup complete: admin, user1, user2");
    
    // User1 creates a source
    let user1_source = client.create_source_as_user(UserType::User1, "User1 WebDAV Source").await
        .expect("Failed to create User1 source");
    
    let user1_source_id = user1_source["id"].as_str().expect("Source should have ID");
    println!("✅ User1 created source: {}", user1_source_id);
    
    // User2 creates a source
    let user2_source = client.create_source_as_user(UserType::User2, "User2 WebDAV Source").await
        .expect("Failed to create User2 source");
    
    let user2_source_id = user2_source["id"].as_str().expect("Source should have ID");
    println!("✅ User2 created source: {}", user2_source_id);
    
    // Test cross-user source access
    let user1_access_user2_source = client.try_access_source(UserType::User1, user2_source_id).await
        .expect("Failed to test User1 access to User2 source");
    
    let user2_access_user1_source = client.try_access_source(UserType::User2, user1_source_id).await
        .expect("Failed to test User2 access to User1 source");
    
    assert!(!user1_access_user2_source.is_success(), "User1 should NOT access User2's source");
    assert!(!user2_access_user1_source.is_success(), "User2 should NOT access User1's source");
    
    println!("✅ Source cross-access prevention verified");
    
    // Test own source access
    let user1_access_own_source = client.try_access_source(UserType::User1, user1_source_id).await
        .expect("Failed to test User1 access to own source");
    
    let user2_access_own_source = client.try_access_source(UserType::User2, user2_source_id).await
        .expect("Failed to test User2 access to own source");
    
    assert!(user1_access_own_source.is_success(), "User1 should access their own source");
    assert!(user2_access_own_source.is_success(), "User2 should access their own source");
    
    println!("✅ Own source access verified");
    
    // Test admin access to user sources
    let admin_access_user1_source = client.try_access_source(UserType::Admin, user1_source_id).await
        .expect("Failed to test admin access to User1 source");
    
    let admin_access_user2_source = client.try_access_source(UserType::Admin, user2_source_id).await
        .expect("Failed to test admin access to User2 source");
    
    println!("ℹ️  Admin access to User1 source: {}", admin_access_user1_source);
    println!("ℹ️  Admin access to User2 source: {}", admin_access_user2_source);
    
    println!("🎉 Source ownership isolation test passed!");
}

#[tokio::test]
async fn test_admin_only_operations() {
    println!("👨‍💼 Testing admin-only operations...");
    
    let mut client = RBACTestClient::new();
    client.setup_all_users().await
        .expect("Failed to setup test users");
    
    println!("✅ Setup complete: admin, user1, user2");
    
    let admin_operations = vec![
        AdminOperation::ListUsers,
        AdminOperation::CreateUser,
        AdminOperation::GetMetrics,
        AdminOperation::GetQueueStats,
        AdminOperation::RequeueFailedJobs,
    ];
    
    for operation in admin_operations {
        let operation_name = match operation {
            AdminOperation::ListUsers => "List Users",
            AdminOperation::CreateUser => "Create User",
            AdminOperation::GetMetrics => "Get Metrics",
            AdminOperation::GetQueueStats => "Get Queue Stats",
            AdminOperation::RequeueFailedJobs => "Requeue Failed Jobs",
        };
        
        println!("🔍 Testing operation: {}", operation_name);
        
        // Test admin access
        let admin_result = client.try_admin_operation(UserType::Admin, operation).await
            .expect("Failed to test admin operation as admin");
        
        // Test regular user access
        let user1_result = client.try_admin_operation(UserType::User1, operation).await
            .expect("Failed to test admin operation as user1");
        
        let user2_result = client.try_admin_operation(UserType::User2, operation).await
            .expect("Failed to test admin operation as user2");
        
        println!("  Admin access: {}", admin_result);
        println!("  User1 access: {}", user1_result);
        println!("  User2 access: {}", user2_result);
        
        // Admin should have access (or at least not be forbidden due to role)
        // Regular users should be denied (401 Unauthorized or 403 Forbidden)
        if user1_result.is_success() || user2_result.is_success() {
            println!("⚠️  WARNING: Regular users have access to admin operation: {}", operation_name);
        } else {
            println!("✅ Regular users properly denied access to: {}", operation_name);
        }
        
        // Users should get 401 (Unauthorized) or 403 (Forbidden)
        assert!(
            user1_result == reqwest::StatusCode::UNAUTHORIZED || 
            user1_result == reqwest::StatusCode::FORBIDDEN,
            "User1 should be denied access to {}", operation_name
        );
        
        assert!(
            user2_result == reqwest::StatusCode::UNAUTHORIZED || 
            user2_result == reqwest::StatusCode::FORBIDDEN,
            "User2 should be denied access to {}", operation_name
        );
    }
    
    println!("🎉 Admin-only operations test passed!");
}

#[tokio::test]
async fn test_privilege_escalation_prevention() {
    println!("🔐 Testing privilege escalation prevention...");
    
    let mut client = RBACTestClient::new();
    client.setup_all_users().await
        .expect("Failed to setup test users");
    
    println!("✅ Setup complete: admin, user1, user2");
    
    // Get user IDs for testing
    let user1_id = client.user1_user_id.as_ref().expect("User1 ID should be set");
    let user2_id = client.user2_user_id.as_ref().expect("User2 ID should be set");
    let admin_id = client.admin_user_id.as_ref().expect("Admin ID should be set");
    
    // Test 1: Regular user trying to modify another user
    println!("🔍 Testing user1 trying to modify user2...");
    
    let user1_modify_user2 = client.try_modify_user_resource(UserType::User1, user2_id).await
        .expect("Failed to test user1 modifying user2");
    
    assert!(
        user1_modify_user2 == reqwest::StatusCode::UNAUTHORIZED || 
        user1_modify_user2 == reqwest::StatusCode::FORBIDDEN ||
        user1_modify_user2 == reqwest::StatusCode::NOT_FOUND,
        "User1 should not be able to modify User2"
    );
    
    println!("✅ User1 cannot modify User2: {}", user1_modify_user2);
    
    // Test 2: Regular user trying to modify admin
    println!("🔍 Testing user1 trying to modify admin...");
    
    let user1_modify_admin = client.try_modify_user_resource(UserType::User1, admin_id).await
        .expect("Failed to test user1 modifying admin");
    
    assert!(
        user1_modify_admin == reqwest::StatusCode::UNAUTHORIZED || 
        user1_modify_admin == reqwest::StatusCode::FORBIDDEN ||
        user1_modify_admin == reqwest::StatusCode::NOT_FOUND,
        "User1 should not be able to modify Admin"
    );
    
    println!("✅ User1 cannot modify Admin: {}", user1_modify_admin);
    
    // Test 3: Admin can modify users (should succeed)
    println!("🔍 Testing admin modifying user1...");
    
    let admin_modify_user1 = client.try_modify_user_resource(UserType::Admin, user1_id).await
        .expect("Failed to test admin modifying user1");
    
    // Admin should have permission (200 OK or similar success)
    println!("ℹ️  Admin modifying User1: {}", admin_modify_user1);
    
    // Test 4: Try to create admin user as regular user
    println!("🔍 Testing regular user trying to create admin user...");
    
    let user1_token = client.user1_token.as_ref().unwrap();
    let create_admin_attempt = client.client
        .post(&format!("{}/api/users", BASE_URL))
        .header("Authorization", format!("Bearer {}", user1_token))
        .json(&json!({
            "username": "malicious_admin",
            "email": "malicious@example.com",
            "password": "password123",
            "role": "admin"  // Trying to create admin user
        }))
        .send()
        .await
        .expect("Create admin attempt should complete");
    
    assert!(
        !create_admin_attempt.status().is_success(),
        "Regular user should not be able to create admin users"
    );
    
    println!("✅ User1 cannot create admin user: {}", create_admin_attempt.status());
    
    // Test 5: Try to promote self to admin
    println!("🔍 Testing self-promotion attempt...");
    
    // This would typically be done through updating own user profile
    // The exact endpoint depends on the API design
    let self_promotion_attempt = client.client
        .put(&format!("{}/api/users/{}", BASE_URL, user1_id))
        .header("Authorization", format!("Bearer {}", user1_token))
        .json(&json!({
            "username": "user1_promoted",
            "email": "user1@example.com",
            "role": "admin"  // Trying to promote self
        }))
        .send()
        .await
        .expect("Self promotion attempt should complete");
    
    assert!(
        !self_promotion_attempt.status().is_success(),
        "User should not be able to promote themselves to admin"
    );
    
    println!("✅ User1 cannot promote self: {}", self_promotion_attempt.status());
    
    println!("🎉 Privilege escalation prevention test passed!");
}

#[tokio::test]
async fn test_data_visibility_boundaries() {
    println!("👁️ Testing data visibility boundaries...");
    
    let mut client = RBACTestClient::new();
    client.setup_all_users().await
        .expect("Failed to setup test users");
    
    println!("✅ Setup complete: admin, user1, user2");
    
    // Create data for each user
    let user1_doc = client.upload_document_as_user(
        UserType::User1,
        "User1 confidential data",
        "user1_confidential.txt"
    ).await.expect("Failed to upload User1 document");
    
    let user2_doc = client.upload_document_as_user(
        UserType::User2,
        "User2 confidential data", 
        "user2_confidential.txt"
    ).await.expect("Failed to upload User2 document");
    
    let user1_source = client.create_source_as_user(UserType::User1, "User1 Confidential Source").await
        .expect("Failed to create User1 source");
    
    let user2_source = client.create_source_as_user(UserType::User2, "User2 Confidential Source").await
        .expect("Failed to create User2 source");
    
    println!("✅ Created test data for both users");
    
    // Test document visibility
    let user1_docs = client.get_documents_as_user(UserType::User1).await
        .expect("Failed to get User1 documents");
    
    let user2_docs = client.get_documents_as_user(UserType::User2).await
        .expect("Failed to get User2 documents");
    
    // Verify isolation
    let user1_doc_id = user1_doc["id"].as_str().unwrap();
    let user2_doc_id = user2_doc["id"].as_str().unwrap();
    
    let user1_sees_only_own = user1_docs.iter().all(|d| {
        // Check if this document belongs to user1 by checking if it's the one they uploaded
        // or by checking user association if available in the response
        d["id"] == user1_doc_id || 
        d.get("user_id").and_then(|uid| uid.as_str()) == client.user1_user_id.as_deref()
    });
    
    let user2_sees_only_own = user2_docs.iter().all(|d| {
        d["id"] == user2_doc_id || 
        d.get("user_id").and_then(|uid| uid.as_str()) == client.user2_user_id.as_deref()
    });
    
    assert!(user1_sees_only_own, "User1 should only see their own documents");
    assert!(user2_sees_only_own, "User2 should only see their own documents");
    
    println!("✅ Document visibility boundaries verified");
    
    // Test search isolation (if available)
    let search_response = client.client
        .get(&format!("{}/api/search", BASE_URL))
        .header("Authorization", format!("Bearer {}", client.user1_token.as_ref().unwrap()))
        .query(&[("q", "confidential")])
        .send()
        .await;
    
    if let Ok(response) = search_response {
        let status = response.status();
        if let Ok(user1_search) = response.json::<Value>().await {
            if let Some(results) = user1_search["documents"].as_array() {
                let user1_search_sees_user2 = results.iter().any(|doc| {
                    doc["id"] == user2_doc_id
                });
                
                assert!(!user1_search_sees_user2, "User1 search should not return User2 documents");
                println!("✅ Search isolation verified");
            }
        }
    }
    
    // Test that users cannot enumerate other users' resources through API exploration
    println!("🔍 Testing API enumeration prevention...");
    
    // Try to access source with incremental IDs (if predictable)
    let user1_source_id = user1_source["id"].as_str().unwrap();
    let user2_source_id = user2_source["id"].as_str().unwrap();
    
    // User1 tries to access User2's source
    let cross_access_result = client.try_access_source(UserType::User1, user2_source_id).await
        .expect("Failed to test cross-source access");
    
    assert!(!cross_access_result.is_success(), "Cross-user source access should be denied");
    
    // Try with non-existent but valid UUID format
    let fake_id = Uuid::new_v4().to_string();
    let fake_access_result = client.try_access_source(UserType::User1, &fake_id).await
        .expect("Failed to test fake source access");
    
    // Should return 404 Not Found, not 403 Forbidden (to avoid information leakage)
    assert_eq!(fake_access_result, reqwest::StatusCode::NOT_FOUND, "Non-existent resource should return 404");
    
    println!("✅ API enumeration prevention verified");
    
    println!("🎉 Data visibility boundaries test passed!");
}

#[tokio::test]
async fn test_token_and_session_security() {
    println!("🎫 Testing token and session security...");
    
    let mut client = RBACTestClient::new();
    client.setup_all_users().await
        .expect("Failed to setup test users");
    
    println!("✅ Setup complete: admin, user1, user2");
    
    // Test 1: Invalid token format
    println!("🔍 Testing invalid token formats...");
    
    let invalid_tokens = vec![
        "invalid-token",
        "Bearer invalid-token",
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.invalid.signature",
        "",
        "null",
        "undefined",
    ];
    
    for invalid_token in invalid_tokens {
        let response = client.client
            .get(&format!("{}/api/documents", BASE_URL))
            .header("Authorization", format!("Bearer {}", invalid_token))
            .send()
            .await
            .expect("Invalid token request should complete");
        
        assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED, 
                  "Invalid token '{}' should return 401", invalid_token);
    }
    
    println!("✅ Invalid tokens properly rejected");
    
    // Test 2: Token for one user accessing another user's resources
    println!("🔍 Testing token cross-contamination...");
    
    let _user1_token = client.user1_token.as_ref().unwrap();
    let user2_token = client.user2_token.as_ref().unwrap();
    
    // Upload documents with each user
    let user1_doc = client.upload_document_as_user(
        UserType::User1,
        "User1 token test doc",
        "user1_token_test.txt"
    ).await.expect("Failed to upload User1 doc");
    
    let user1_doc_id = user1_doc["id"].as_str().unwrap();
    
    // Try to access User1's document with User2's token
    let cross_token_access = client.client
        .get(&format!("{}/api/documents/{}/ocr", BASE_URL, user1_doc_id))
        .header("Authorization", format!("Bearer {}", user2_token))
        .send()
        .await
        .expect("Cross-token access should complete");
    
    assert!(!cross_token_access.status().is_success(), 
           "User2 token should not access User1 document");
    
    println!("✅ Token cross-contamination prevention verified");
    
    // Test 3: Expired/revoked token simulation
    println!("🔍 Testing token revocation scenarios...");
    
    // This test would require actual token expiration or revocation mechanisms
    // For now, we test that a completely invalid token structure is rejected
    let malformed_jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.malformed_signature";
    
    let malformed_response = client.client
        .get(&format!("{}/api/documents", BASE_URL))
        .header("Authorization", format!("Bearer {}", malformed_jwt))
        .send()
        .await
        .expect("Malformed JWT request should complete");
    
    assert_eq!(malformed_response.status(), reqwest::StatusCode::UNAUTHORIZED,
              "Malformed JWT should be rejected");
    
    println!("✅ Malformed JWT properly rejected");
    
    // Test 4: Missing Authorization header
    println!("🔍 Testing missing authorization...");
    
    let no_auth_response = client.client
        .get(&format!("{}/api/documents", BASE_URL))
        .send()
        .await
        .expect("No auth request should complete");
    
    assert_eq!(no_auth_response.status(), reqwest::StatusCode::UNAUTHORIZED,
              "Missing authorization should return 401");
    
    println!("✅ Missing authorization properly handled");
    
    println!("🎉 Token and session security test passed!");
}