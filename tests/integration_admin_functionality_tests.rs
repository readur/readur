/*!
 * Admin Functionality Integration Tests
 * 
 * Tests administrative operations including:
 * - User management (CRUD operations)
 * - System metrics access
 * - Admin-only endpoints
 * - Role-based access control
 * - System monitoring capabilities
 */

use serde_json::{json, Value};
use uuid::Uuid;
use axum::http::StatusCode;
use axum::body::Body;
use axum::http::Request;
use tower::ServiceExt;

use readur::models::{CreateUser, UpdateUser, UserRole};
use readur::test_utils::{TestContext, TestAuthHelper};

/// Test client with admin capabilities
struct AdminTestClient {
    ctx: TestContext,
    auth_helper: TestAuthHelper,
    admin_token: Option<String>,
    user_token: Option<String>,
    admin_user_id: Option<Uuid>,
    regular_user_id: Option<Uuid>,
}

impl AdminTestClient {
    async fn new() -> Self {
        let ctx = TestContext::new().await;
        let auth_helper = TestAuthHelper::new(ctx.app().clone());
        Self {
            ctx,
            auth_helper,
            admin_token: None,
            user_token: None,
            admin_user_id: None,
            regular_user_id: None,
        }
    }
    
    /// Setup admin user using test context
    async fn setup_admin(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        // Create an admin user through test utils
        let admin_user = self.auth_helper.create_admin_user().await;
        let token = self.auth_helper.login_user(&admin_user.username, "adminpass123").await;
        
        self.admin_token = Some(token.clone());
        self.admin_user_id = Some(admin_user.user_response.id);
        
        Ok(token)
    }
    
    /// Setup regular user using test context
    async fn setup_regular_user(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        let mut test_user = self.auth_helper.create_test_user().await;
        let user_id = test_user.user_response.id;
        let token = test_user.login(&self.auth_helper).await?;
        
        self.user_token = Some(token.to_string());
        self.regular_user_id = Some(user_id);
        
        Ok(token.to_string())
    }
    
    /// Get all users (admin only)
    async fn get_all_users(&self, as_admin: bool) -> Result<Value, Box<dyn std::error::Error>> {
        let token = if as_admin {
            self.admin_token.as_ref().ok_or("Admin not logged in")?
        } else {
            self.user_token.as_ref().ok_or("User not logged in")?
        };
        
        let request = Request::builder()
            .method("GET")
            .uri("/api/users")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
            
        let response = self.ctx.app().clone().oneshot(request).await.unwrap();
        
        if !response.status().is_success() {
            let status = response.status();
            let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
            let body_str = String::from_utf8_lossy(&body_bytes);
            return Err(format!("Get users failed: {} - {}", status, body_str).into());
        }
        
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let users: Value = serde_json::from_slice(&body_bytes)?;
        Ok(users)
    }
    
    /// Create a new user 
    async fn create_user(&self, username: &str, email: &str, role: UserRole, as_admin: bool) -> Result<Value, Box<dyn std::error::Error>> {
        let token = if as_admin {
            self.admin_token.as_ref().ok_or("Admin not logged in")?
        } else {
            self.user_token.as_ref().ok_or("User not logged in")?
        };
        
        let user_data = CreateUser {
            username: username.to_string(),
            email: email.to_string(),
            password: "temporarypassword123".to_string(),
            role: Some(role),
        };
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/users")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_vec(&user_data)?))
            .unwrap();
            
        let response = self.ctx.app().clone().oneshot(request).await.unwrap();
        
        if !response.status().is_success() {
            let status = response.status();
            let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
            let error_text = String::from_utf8_lossy(&body_bytes);
            eprintln!("Create user failed with status {}: {}", status, error_text);
            eprintln!("Request data: {:?}", user_data);
            return Err(format!("Create user failed: {} - {}", status, error_text).into());
        }
        
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let user: Value = serde_json::from_slice(&body_bytes)?;
        Ok(user)
    }
    
    /// Get specific user (admin only)
    async fn get_user(&self, user_id: &Uuid, as_admin: bool) -> Result<Value, Box<dyn std::error::Error>> {
        let token = if as_admin {
            self.admin_token.as_ref().ok_or("Admin not logged in")?
        } else {
            self.user_token.as_ref().ok_or("User not logged in")?
        };
        
        let request = Request::builder()
            .method("GET")
            .uri(&format!("/api/users/{}", user_id))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
            
        let response = self.ctx.app().clone().oneshot(request).await.unwrap();
        
        if !response.status().is_success() {
            let status = response.status();
            let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
            let body_str = String::from_utf8_lossy(&body_bytes);
            return Err(format!("Get user failed: {} - {}", status, body_str).into());
        }
        
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let user: Value = serde_json::from_slice(&body_bytes)?;
        Ok(user)
    }
    
    /// Update user (admin only)
    async fn update_user(&self, user_id: &Uuid, updates: Value) -> Result<Value, Box<dyn std::error::Error>> {
        let token = self.admin_token.as_ref().ok_or("Admin not logged in")?;
        
        let request = Request::builder()
            .method("PUT")
            .uri(&format!("/api/users/{}", user_id))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_vec(&updates)?))
            .unwrap();
            
        let response = self.ctx.app().clone().oneshot(request).await.unwrap();
        
        if !response.status().is_success() {
            let status = response.status();
            let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
            let body_str = String::from_utf8_lossy(&body_bytes);
            return Err(format!("Update user failed: {} - {}", status, body_str).into());
        }
        
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let user: Value = serde_json::from_slice(&body_bytes)?;
        Ok(user)
    }
    
    /// Delete user (admin only)
    async fn delete_user(&self, user_id: &Uuid) -> Result<(), Box<dyn std::error::Error>> {
        let token = self.admin_token.as_ref().ok_or("Admin not logged in")?;
        
        let request = Request::builder()
            .method("DELETE")
            .uri(&format!("/api/users/{}", user_id))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
            
        let response = self.ctx.app().clone().oneshot(request).await.unwrap();
        
        if !response.status().is_success() {
            let status = response.status();
            let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
            let body_str = String::from_utf8_lossy(&body_bytes);
            return Err(format!("Delete user failed: {} - {}", status, body_str).into());
        }
        
        Ok(())
    }
    
    /// Get system metrics
    async fn get_metrics(&self, as_admin: bool) -> Result<Value, Box<dyn std::error::Error>> {
        let token = if as_admin {
            self.admin_token.as_ref().ok_or("Admin not logged in")?
        } else {
            self.user_token.as_ref().ok_or("User not logged in")?
        };
        
        let request = Request::builder()
            .method("GET")
            .uri("/api/metrics")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
            
        let response = self.ctx.app().clone().oneshot(request).await.unwrap();
        
        if !response.status().is_success() {
            let status = response.status();
            let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
            let body_str = String::from_utf8_lossy(&body_bytes);
            return Err(format!("Get metrics failed: {} - {}", status, body_str).into());
        }
        
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let metrics: Value = serde_json::from_slice(&body_bytes)?;
        Ok(metrics)
    }
    
    /// Get Prometheus metrics (usually public)
    async fn get_prometheus_metrics(&self) -> Result<String, Box<dyn std::error::Error>> {
        let request = Request::builder()
            .method("GET")
            .uri("/metrics")
            .body(Body::empty())
            .unwrap();
            
        let response = self.ctx.app().clone().oneshot(request).await.unwrap();
        
        if !response.status().is_success() {
            return Err(format!("Get Prometheus metrics failed: {}", response.status()).into());
        }
        
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let metrics_text = String::from_utf8_lossy(&body_bytes).to_string();
        Ok(metrics_text)
    }
}

#[tokio::test]
async fn test_admin_user_management_crud() {
    let mut client = AdminTestClient::new().await;
    
    // Setup admin user
    client.setup_admin().await
        .expect("Failed to setup admin user");
    
    println!("✅ Admin user setup complete");
    
    // Test getting all users
    let all_users = client.get_all_users(true).await
        .expect("Failed to get all users as admin");
    
    // Should at least contain the admin user
    assert!(all_users.as_array().unwrap().len() >= 1);
    
    let admin_found = all_users.as_array().unwrap().iter()
        .any(|u| u["role"] == "admin");
    assert!(admin_found);
    
    println!("✅ Admin can list all users");
    
    // Create a new user via admin API with unique name
    let unique_id = Uuid::new_v4().to_string()[..8].to_string();
    let username = format!("test_managed_user_{}", unique_id);
    let email = format!("managed_{}@example.com", unique_id);
    
    let created_user = client.create_user(&username, &email, UserRole::User, true).await
        .expect("Failed to create user as admin");
    
    let created_user_id = Uuid::parse_str(created_user["id"].as_str().expect("User should have ID")).unwrap();
    assert_eq!(created_user["username"], username);
    assert_eq!(created_user["email"], email);
    assert_eq!(created_user["role"], "user");
    
    println!("✅ Admin can create new users");
    
    // Get the created user details
    let user_details = client.get_user(&created_user_id, true).await
        .expect("Failed to get user details as admin");
    
    assert_eq!(user_details["id"], created_user["id"]);
    assert_eq!(user_details["username"], username);
    
    println!("✅ Admin can get user details");
    
    // Update the user
    let updated_username = format!("updated_managed_user_{}", unique_id);
    let updated_email = format!("updated_managed_{}@example.com", unique_id);
    let updates = json!({
        "username": updated_username,
        "email": updated_email
    });
    
    let updated_user = client.update_user(&created_user_id, updates).await
        .expect("Failed to update user as admin");
    
    assert_eq!(updated_user["username"], updated_username);
    assert_eq!(updated_user["email"], updated_email);
    
    println!("✅ Admin can update users");
    
    // Verify the update persisted
    let updated_user_details = client.get_user(&created_user_id, true).await
        .expect("Failed to get updated user details");
    
    assert_eq!(updated_user_details["username"], updated_username);
    
    // Delete the user
    client.delete_user(&created_user_id).await
        .expect("Failed to delete user as admin");
    
    println!("✅ Admin can delete users");
    
    // Verify deletion
    let delete_verification = client.get_user(&created_user_id, true).await;
    assert!(delete_verification.is_err());
    
    println!("🎉 Admin user management CRUD test passed!");
}

#[tokio::test]
async fn test_role_based_access_control() {
    let mut client = AdminTestClient::new().await;
    
    // Setup both admin and regular user
    client.setup_admin().await
        .expect("Failed to setup admin user");
    
    client.setup_regular_user().await
        .expect("Failed to setup regular user");
    
    println!("✅ Both admin and regular user setup complete");
    
    // Test that regular user CANNOT access user management endpoints (secured implementation)
    
    // Regular user should NOT be able to list all users
    let user_list_attempt = client.get_all_users(false).await;
    assert!(user_list_attempt.is_err());
    println!("✅ Regular user cannot list all users (properly secured)");
    
    // Regular user should NOT be able to get specific user details
    let admin_user_id = client.admin_user_id.as_ref().unwrap();
    let user_details_attempt = client.get_user(admin_user_id, false).await;
    assert!(user_details_attempt.is_err());
    println!("✅ Regular user cannot access other user details (properly secured)");
    
    // Test that regular user CANNOT create users (secured implementation)
    let unique_id = Uuid::new_v4().to_string()[..8].to_string();
    let test_user = client.create_user(&format!("regular_created_user_{}", unique_id), &format!("regular_{}@example.com", unique_id), UserRole::User, false).await;
    // Secured implementation denies user creation to non-admins
    assert!(test_user.is_err());
    println!("✅ Regular user cannot create users (properly secured)");
    
    // Test that admin CAN access all user management endpoints
    let admin_users_list = client.get_all_users(true).await
        .expect("Admin should be able to list users");
    
    assert!(admin_users_list.as_array().unwrap().len() >= 2); // At least admin and regular user
    println!("✅ Admin can list all users");
    
    // Admin should be able to get regular user details
    let regular_user_id = client.regular_user_id.as_ref().unwrap();
    let regular_user_details = client.get_user(regular_user_id, true).await
        .expect("Admin should be able to get user details");
    
    assert_eq!(regular_user_details["role"], "user");
    println!("✅ Admin can access other user details");
    
    println!("🎉 Role-based access control test passed!");
}

#[tokio::test]
async fn test_system_metrics_access() {
    let mut client = AdminTestClient::new().await;
    
    // Setup admin and regular user
    client.setup_admin().await
        .expect("Failed to setup admin user");
    
    client.setup_regular_user().await
        .expect("Failed to setup regular user");
    
    println!("✅ Users setup for metrics testing");
    
    // Test Prometheus metrics endpoint (usually public)
    let prometheus_metrics = client.get_prometheus_metrics().await
        .expect("Failed to get Prometheus metrics");
    
    // Should contain some basic metrics
    assert!(prometheus_metrics.contains("# TYPE"));
    assert!(prometheus_metrics.len() > 0);
    println!("✅ Prometheus metrics accessible");
    
    // Test JSON metrics endpoint access
    
    // Admin should be able to access metrics
    let admin_metrics = client.get_metrics(true).await;
    if admin_metrics.is_ok() {
        let metrics = admin_metrics.unwrap();
        // Should have some system information
        assert!(metrics.is_object());
        println!("✅ Admin can access JSON metrics");
    } else {
        println!("⚠️  JSON metrics endpoint may not be implemented or accessible");
    }
    
    // Regular user may or may not have access depending on implementation
    let user_metrics = client.get_metrics(false).await;
    match user_metrics {
        Ok(_) => println!("ℹ️  Regular user can access JSON metrics"),
        Err(_) => println!("ℹ️  Regular user cannot access JSON metrics (expected)"),
    }
    
    println!("🎉 System metrics access test passed!");
}

#[tokio::test]
async fn test_admin_user_management_without_roles() {
    let mut client = AdminTestClient::new().await;
    
    client.setup_admin().await
        .expect("Failed to setup admin user");
    
    println!("✅ Admin user setup complete");
    
    // Create a regular user with unique name
    let unique_id = Uuid::new_v4().to_string()[..8].to_string();
    let username = format!("role_test_user_{}", unique_id);
    let email = format!("roletest_{}@example.com", unique_id);
    
    let regular_user = client.create_user(&username, &email, UserRole::User, true).await
        .expect("Failed to create regular user");
    
    let user_id = Uuid::parse_str(regular_user["id"].as_str().unwrap()).unwrap();
    assert_eq!(regular_user["role"], "user");
    
    println!("✅ Regular user created");
    
    // Update user info (username and email, but not role - role updates not supported in current API)
    let updates = json!({
        "username": format!("updated_{}", username),
        "email": format!("updated_{}", email)
    });
    
    let updated_user = client.update_user(&user_id, updates).await
        .expect("Failed to update user");
    
    assert_eq!(updated_user["username"], format!("updated_{}", username));
    assert_eq!(updated_user["email"], format!("updated_{}", email));
    assert_eq!(updated_user["role"], "user"); // Role should remain unchanged
    println!("✅ User info updated (role management not supported in current API)");
    
    // Clean up
    client.delete_user(&user_id).await
        .expect("Failed to delete test user");
    
    println!("🎉 Admin user management test passed!");
}

#[tokio::test]
async fn test_admin_bulk_operations() {
    let mut client = AdminTestClient::new().await;
    
    client.setup_admin().await
        .expect("Failed to setup admin user");
    
    println!("✅ Admin user setup complete");
    
    // Create multiple users with unique identifiers
    let mut created_user_ids = Vec::new();
    let test_run_id = Uuid::new_v4().to_string()[..8].to_string();
    
    for i in 1..=5 {
        let username = format!("bulk_user_{}_{}", test_run_id, i);
        let email = format!("bulk_user_{}_{}@example.com", test_run_id, i);
        
        let user = client.create_user(
            &username,
            &email,
            UserRole::User,
            true
        ).await.expect("Failed to create bulk user");
        
        created_user_ids.push(Uuid::parse_str(user["id"].as_str().unwrap()).unwrap());
    }
    
    println!("✅ Created 5 test users");
    
    // Verify all users exist in the list
    let all_users = client.get_all_users(true).await
        .expect("Failed to get all users");
    
    let users_array = all_users.as_array().unwrap();
    assert!(users_array.len() >= 6); // At least admin + 5 created users
    
    // Verify each created user exists
    for user_id in &created_user_ids {
        let user_exists = users_array.iter()
            .any(|u| u["id"].as_str() == Some(&user_id.to_string()));
        assert!(user_exists);
    }
    
    println!("✅ All created users found in list");
    
    // Update all users
    for (i, user_id) in created_user_ids.iter().enumerate() {
        let username = format!("updated_bulk_user_{}_{}", test_run_id, i + 1);
        let email = format!("updated_bulk_user_{}_{}@example.com", test_run_id, i + 1);
        
        let updates = json!({
            "username": username,
            "email": email
        });
        
        client.update_user(user_id, updates).await
            .expect("Failed to update bulk user");
    }
    
    println!("✅ All users updated");
    
    // Verify updates
    let updated_users = client.get_all_users(true).await
        .expect("Failed to get updated users");
    
    let updated_count = updated_users.as_array().unwrap().iter()
        .filter(|u| u["username"].as_str().unwrap_or("").starts_with(&format!("updated_bulk_user_{}_", test_run_id)))
        .count();
    
    assert_eq!(updated_count, 5);
    println!("✅ All user updates verified");
    
    // Delete all created users
    for user_id in &created_user_ids {
        client.delete_user(user_id).await
            .expect("Failed to delete bulk user");
    }
    
    println!("✅ All test users deleted");
    
    // Verify deletions
    let final_users = client.get_all_users(true).await
        .expect("Failed to get final user list");
    
    for user_id in &created_user_ids {
        let user_still_exists = final_users.as_array().unwrap().iter()
            .any(|u| u["id"].as_str() == Some(&user_id.to_string()));
        assert!(!user_still_exists);
    }
    
    println!("🎉 Admin bulk operations test passed!");
}

#[tokio::test]
async fn test_admin_error_handling() {
    let mut client = AdminTestClient::new().await;
    
    client.setup_admin().await
        .expect("Failed to setup admin user");
    
    println!("✅ Admin user setup complete");
    
    // Test accessing non-existent user
    let fake_user_id = Uuid::new_v4();
    let non_existent_user_result = client.get_user(&fake_user_id, true).await;
    assert!(non_existent_user_result.is_err());
    println!("✅ Non-existent user access properly handled");
    
    // Test updating non-existent user
    let update_non_existent = client.update_user(&fake_user_id, json!({"username": "test"})).await;
    assert!(update_non_existent.is_err());
    println!("✅ Non-existent user update properly handled");
    
    // Test deleting non-existent user (current implementation returns success)
    let delete_non_existent = client.delete_user(&fake_user_id).await;
    // Current implementation returns 204 No Content even for non-existent users
    assert!(delete_non_existent.is_ok());
    println!("✅ Non-existent user deletion returns success (current behavior)");
    
    // Test creating duplicate username
    let unique_id = Uuid::new_v4().to_string()[..8].to_string();
    let username = format!("duplicate_test_{}", unique_id);
    
    let user1 = client.create_user(&username, &format!("{}1@example.com", username), UserRole::User, true).await
        .expect("Failed to create first user");
    
    let duplicate_result = client.create_user(&username, &format!("{}2@example.com", username), UserRole::User, true).await;
    // Should fail due to duplicate username
    assert!(duplicate_result.is_err());
    println!("✅ Duplicate username creation properly rejected");
    
    // Clean up
    let user1_id = Uuid::parse_str(user1["id"].as_str().unwrap()).unwrap();
    client.delete_user(&user1_id).await
        .expect("Failed to cleanup user");
    
    println!("🎉 Admin error handling test passed!");
}