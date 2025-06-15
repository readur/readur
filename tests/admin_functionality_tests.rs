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

use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;
use uuid::Uuid;

use readur::models::{CreateUser, LoginRequest, LoginResponse, UserRole};

const BASE_URL: &str = "http://localhost:8000";

/// Test client with admin capabilities
struct AdminTestClient {
    client: Client,
    admin_token: Option<String>,
    user_token: Option<String>,
    admin_user_id: Option<String>,
    regular_user_id: Option<String>,
}

impl AdminTestClient {
    fn new() -> Self {
        Self {
            client: Client::new(),
            admin_token: None,
            user_token: None,
            admin_user_id: None,
            regular_user_id: None,
        }
    }
    
    /// Register and login as admin user
    async fn setup_admin(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let username = format!("admin_test_{}", timestamp);
        let email = format!("admin_test_{}@example.com", timestamp);
        let password = "adminpassword123";
        
        // Register admin user
        let admin_data = CreateUser {
            username: username.clone(),
            email: email.clone(),
            password: password.to_string(),
            role: Some(UserRole::Admin),
        };
        
        let register_response = self.client
            .post(&format!("{}/api/auth/register", BASE_URL))
            .json(&admin_data)
            .send()
            .await?;
        
        if !register_response.status().is_success() {
            return Err(format!("Admin registration failed: {}", register_response.text().await?).into());
        }
        
        // Login admin
        let login_data = LoginRequest {
            username: username.clone(),
            password: password.to_string(),
        };
        
        let login_response = self.client
            .post(&format!("{}/api/auth/login", BASE_URL))
            .json(&login_data)
            .send()
            .await?;
        
        if !login_response.status().is_success() {
            return Err(format!("Admin login failed: {}", login_response.text().await?).into());
        }
        
        let login_result: LoginResponse = login_response.json().await?;
        self.admin_token = Some(login_result.token.clone());
        
        // Get admin user info
        let me_response = self.client
            .get(&format!("{}/api/auth/me", BASE_URL))
            .header("Authorization", format!("Bearer {}", login_result.token))
            .send()
            .await?;
        
        if me_response.status().is_success() {
            let user_info: Value = me_response.json().await?;
            self.admin_user_id = user_info["id"].as_str().map(|s| s.to_string());
        }
        
        Ok(login_result.token)
    }
    
    /// Register and login as regular user
    async fn setup_regular_user(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let username = format!("user_test_{}", timestamp);
        let email = format!("user_test_{}@example.com", timestamp);
        let password = "userpassword123";
        
        // Register regular user
        let user_data = CreateUser {
            username: username.clone(),
            email: email.clone(),
            password: password.to_string(),
            role: Some(UserRole::User),
        };
        
        let register_response = self.client
            .post(&format!("{}/api/auth/register", BASE_URL))
            .json(&user_data)
            .send()
            .await?;
        
        if !register_response.status().is_success() {
            return Err(format!("User registration failed: {}", register_response.text().await?).into());
        }
        
        // Login user
        let login_data = LoginRequest {
            username: username.clone(),
            password: password.to_string(),
        };
        
        let login_response = self.client
            .post(&format!("{}/api/auth/login", BASE_URL))
            .json(&login_data)
            .send()
            .await?;
        
        if !login_response.status().is_success() {
            return Err(format!("User login failed: {}", login_response.text().await?).into());
        }
        
        let login_result: LoginResponse = login_response.json().await?;
        self.user_token = Some(login_result.token.clone());
        
        // Get user info
        let me_response = self.client
            .get(&format!("{}/api/auth/me", BASE_URL))
            .header("Authorization", format!("Bearer {}", login_result.token))
            .send()
            .await?;
        
        if me_response.status().is_success() {
            let user_info: Value = me_response.json().await?;
            self.regular_user_id = user_info["id"].as_str().map(|s| s.to_string());
        }
        
        Ok(login_result.token)
    }
    
    /// Get all users (admin only)
    async fn get_all_users(&self, as_admin: bool) -> Result<Value, Box<dyn std::error::Error>> {
        let token = if as_admin {
            self.admin_token.as_ref().ok_or("Admin not logged in")?
        } else {
            self.user_token.as_ref().ok_or("User not logged in")?
        };
        
        let response = self.client
            .get(&format!("{}/api/users", BASE_URL))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Get users failed: {} - {}", response.status(), response.text().await?).into());
        }
        
        let users: Value = response.json().await?;
        Ok(users)
    }
    
    /// Create a new user (admin only)
    async fn create_user(&self, username: &str, email: &str, role: UserRole) -> Result<Value, Box<dyn std::error::Error>> {
        let token = self.admin_token.as_ref().ok_or("Admin not logged in")?;
        
        let user_data = json!({
            "username": username,
            "email": email,
            "password": "temporarypassword123",
            "role": role.to_string()
        });
        
        let response = self.client
            .post(&format!("{}/api/users", BASE_URL))
            .header("Authorization", format!("Bearer {}", token))
            .json(&user_data)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Create user failed: {} - {}", response.status(), response.text().await?).into());
        }
        
        let user: Value = response.json().await?;
        Ok(user)
    }
    
    /// Get specific user (admin only)
    async fn get_user(&self, user_id: &str, as_admin: bool) -> Result<Value, Box<dyn std::error::Error>> {
        let token = if as_admin {
            self.admin_token.as_ref().ok_or("Admin not logged in")?
        } else {
            self.user_token.as_ref().ok_or("User not logged in")?
        };
        
        let response = self.client
            .get(&format!("{}/api/users/{}", BASE_URL, user_id))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Get user failed: {} - {}", response.status(), response.text().await?).into());
        }
        
        let user: Value = response.json().await?;
        Ok(user)
    }
    
    /// Update user (admin only)
    async fn update_user(&self, user_id: &str, updates: Value) -> Result<Value, Box<dyn std::error::Error>> {
        let token = self.admin_token.as_ref().ok_or("Admin not logged in")?;
        
        let response = self.client
            .put(&format!("{}/api/users/{}", BASE_URL, user_id))
            .header("Authorization", format!("Bearer {}", token))
            .json(&updates)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Update user failed: {} - {}", response.status(), response.text().await?).into());
        }
        
        let user: Value = response.json().await?;
        Ok(user)
    }
    
    /// Delete user (admin only)
    async fn delete_user(&self, user_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let token = self.admin_token.as_ref().ok_or("Admin not logged in")?;
        
        let response = self.client
            .delete(&format!("{}/api/users/{}", BASE_URL, user_id))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Delete user failed: {} - {}", response.status(), response.text().await?).into());
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
        
        let response = self.client
            .get(&format!("{}/api/metrics", BASE_URL))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Get metrics failed: {} - {}", response.status(), response.text().await?).into());
        }
        
        let metrics: Value = response.json().await?;
        Ok(metrics)
    }
    
    /// Get Prometheus metrics (usually public)
    async fn get_prometheus_metrics(&self) -> Result<String, Box<dyn std::error::Error>> {
        let response = self.client
            .get(&format!("{}/metrics", BASE_URL))
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(format!("Get Prometheus metrics failed: {}", response.status()).into());
        }
        
        let metrics_text = response.text().await?;
        Ok(metrics_text)
    }
}

#[tokio::test]
async fn test_admin_user_management_crud() {
    let mut client = AdminTestClient::new();
    
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
    
    // Create a new user via admin API
    let created_user = client.create_user("test_managed_user", "managed@example.com", UserRole::User).await
        .expect("Failed to create user as admin");
    
    let created_user_id = created_user["id"].as_str().expect("User should have ID");
    assert_eq!(created_user["username"], "test_managed_user");
    assert_eq!(created_user["email"], "managed@example.com");
    assert_eq!(created_user["role"], "user");
    
    println!("✅ Admin can create new users");
    
    // Get the created user details
    let user_details = client.get_user(created_user_id, true).await
        .expect("Failed to get user details as admin");
    
    assert_eq!(user_details["id"], created_user["id"]);
    assert_eq!(user_details["username"], "test_managed_user");
    
    println!("✅ Admin can get user details");
    
    // Update the user
    let updates = json!({
        "username": "updated_managed_user",
        "email": "updated_managed@example.com",
        "role": "user"
    });
    
    let updated_user = client.update_user(created_user_id, updates).await
        .expect("Failed to update user as admin");
    
    assert_eq!(updated_user["username"], "updated_managed_user");
    assert_eq!(updated_user["email"], "updated_managed@example.com");
    
    println!("✅ Admin can update users");
    
    // Verify the update persisted
    let updated_user_details = client.get_user(created_user_id, true).await
        .expect("Failed to get updated user details");
    
    assert_eq!(updated_user_details["username"], "updated_managed_user");
    
    // Delete the user
    client.delete_user(created_user_id).await
        .expect("Failed to delete user as admin");
    
    println!("✅ Admin can delete users");
    
    // Verify deletion
    let delete_verification = client.get_user(created_user_id, true).await;
    assert!(delete_verification.is_err());
    
    println!("🎉 Admin user management CRUD test passed!");
}

#[tokio::test]
async fn test_role_based_access_control() {
    let mut client = AdminTestClient::new();
    
    // Setup both admin and regular user
    client.setup_admin().await
        .expect("Failed to setup admin user");
    
    client.setup_regular_user().await
        .expect("Failed to setup regular user");
    
    println!("✅ Both admin and regular user setup complete");
    
    // Test that regular user CANNOT access user management endpoints
    
    // Regular user should not be able to list all users
    let user_list_attempt = client.get_all_users(false).await;
    assert!(user_list_attempt.is_err());
    println!("✅ Regular user cannot list all users");
    
    // Regular user should not be able to get specific user details
    let admin_user_id = client.admin_user_id.as_ref().unwrap();
    let user_details_attempt = client.get_user(admin_user_id, false).await;
    assert!(user_details_attempt.is_err());
    println!("✅ Regular user cannot access other user details");
    
    // Regular user should not be able to create users
    let token = client.user_token.as_ref().unwrap();
    let create_user_data = json!({
        "username": "unauthorized_user",
        "email": "unauthorized@example.com",
        "password": "password123",
        "role": "user"
    });
    
    let create_response = client.client
        .post(&format!("{}/api/users", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&create_user_data)
        .send()
        .await
        .expect("Request should complete");
    
    assert!(!create_response.status().is_success());
    println!("✅ Regular user cannot create users");
    
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
    let mut client = AdminTestClient::new();
    
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
async fn test_admin_user_role_management() {
    let mut client = AdminTestClient::new();
    
    client.setup_admin().await
        .expect("Failed to setup admin user");
    
    println!("✅ Admin user setup complete");
    
    // Create a regular user
    let regular_user = client.create_user("role_test_user", "roletest@example.com", UserRole::User).await
        .expect("Failed to create regular user");
    
    let user_id = regular_user["id"].as_str().unwrap();
    assert_eq!(regular_user["role"], "user");
    
    println!("✅ Regular user created");
    
    // Promote user to admin
    let promotion_updates = json!({
        "username": "role_test_user",
        "email": "roletest@example.com", 
        "role": "admin"
    });
    
    let promoted_user = client.update_user(user_id, promotion_updates).await
        .expect("Failed to promote user to admin");
    
    assert_eq!(promoted_user["role"], "admin");
    println!("✅ User promoted to admin");
    
    // Demote back to regular user
    let demotion_updates = json!({
        "username": "role_test_user",
        "email": "roletest@example.com",
        "role": "user"
    });
    
    let demoted_user = client.update_user(user_id, demotion_updates).await
        .expect("Failed to demote user back to regular user");
    
    assert_eq!(demoted_user["role"], "user");
    println!("✅ User demoted back to regular user");
    
    // Clean up
    client.delete_user(user_id).await
        .expect("Failed to delete test user");
    
    println!("🎉 Admin user role management test passed!");
}

#[tokio::test]
async fn test_admin_bulk_operations() {
    let mut client = AdminTestClient::new();
    
    client.setup_admin().await
        .expect("Failed to setup admin user");
    
    println!("✅ Admin user setup complete");
    
    // Create multiple users
    let mut created_user_ids = Vec::new();
    
    for i in 1..=5 {
        let user = client.create_user(
            &format!("bulk_user_{}", i),
            &format!("bulk_user_{}@example.com", i),
            UserRole::User
        ).await.expect("Failed to create bulk user");
        
        created_user_ids.push(user["id"].as_str().unwrap().to_string());
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
            .any(|u| u["id"].as_str() == Some(user_id));
        assert!(user_exists);
    }
    
    println!("✅ All created users found in list");
    
    // Update all users
    for (i, user_id) in created_user_ids.iter().enumerate() {
        let updates = json!({
            "username": format!("updated_bulk_user_{}", i + 1),
            "email": format!("updated_bulk_user_{}@example.com", i + 1),
            "role": "user"
        });
        
        client.update_user(user_id, updates).await
            .expect("Failed to update bulk user");
    }
    
    println!("✅ All users updated");
    
    // Verify updates
    let updated_users = client.get_all_users(true).await
        .expect("Failed to get updated users");
    
    let updated_count = updated_users.as_array().unwrap().iter()
        .filter(|u| u["username"].as_str().unwrap_or("").starts_with("updated_bulk_user_"))
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
            .any(|u| u["id"].as_str() == Some(user_id));
        assert!(!user_still_exists);
    }
    
    println!("🎉 Admin bulk operations test passed!");
}

#[tokio::test]
async fn test_admin_error_handling() {
    let mut client = AdminTestClient::new();
    
    client.setup_admin().await
        .expect("Failed to setup admin user");
    
    println!("✅ Admin user setup complete");
    
    // Test creating user with invalid data
    let invalid_user_data = json!({
        "username": "", // Empty username
        "email": "invalid-email", // Invalid email format
        "password": "123", // Too short password
        "role": "invalid_role" // Invalid role
    });
    
    let token = client.admin_token.as_ref().unwrap();
    let invalid_create_response = client.client
        .post(&format!("{}/api/users", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&invalid_user_data)
        .send()
        .await
        .expect("Request should complete");
    
    assert!(!invalid_create_response.status().is_success());
    println!("✅ Invalid user creation properly rejected");
    
    // Test accessing non-existent user
    let fake_user_id = Uuid::new_v4().to_string();
    let non_existent_user_result = client.get_user(&fake_user_id, true).await;
    assert!(non_existent_user_result.is_err());
    println!("✅ Non-existent user access properly handled");
    
    // Test updating non-existent user
    let update_non_existent = client.update_user(&fake_user_id, json!({"username": "test"})).await;
    assert!(update_non_existent.is_err());
    println!("✅ Non-existent user update properly handled");
    
    // Test deleting non-existent user
    let delete_non_existent = client.delete_user(&fake_user_id).await;
    assert!(delete_non_existent.is_err());
    println!("✅ Non-existent user deletion properly handled");
    
    // Test creating duplicate username
    let user1 = client.create_user("duplicate_test", "test1@example.com", UserRole::User).await
        .expect("Failed to create first user");
    
    let duplicate_result = client.create_user("duplicate_test", "test2@example.com", UserRole::User).await;
    // Should fail due to duplicate username
    assert!(duplicate_result.is_err());
    println!("✅ Duplicate username creation properly rejected");
    
    // Clean up
    let user1_id = user1["id"].as_str().unwrap();
    client.delete_user(user1_id).await
        .expect("Failed to cleanup user");
    
    println!("🎉 Admin error handling test passed!");
}