use anyhow::Result;
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tower::ServiceExt;
use uuid::Uuid;

use readur::{
    models::UserRole,
    services::user_watch_service::UserWatchService,
    test_utils::{TestContext, TestAuthHelper},
    AppState,
};


#[tokio::test]
async fn test_per_user_watch_directory_lifecycle() -> Result<()> {
    let ctx = TestContext::new().await;
    
    // Enable per-user watch for this test
    let mut config = ctx.state.config.clone();
    let temp_user_watch = TempDir::new()?;
    config.user_watch_base_dir = temp_user_watch.path().to_string_lossy().to_string();
    config.enable_per_user_watch = true;
    
    // Update the state with the new config and user watch service
    let user_watch_service = Some(Arc::new(UserWatchService::new(&config.user_watch_base_dir)));
    let updated_state = Arc::new(AppState {
        db: ctx.state.db.clone(),
        config,
        file_service: ctx.state.file_service.clone(),
        webdav_scheduler: None,
        source_scheduler: None,
        queue_service: ctx.state.queue_service.clone(),
        oidc_client: None,
        sync_progress_tracker: ctx.state.sync_progress_tracker.clone(),
        user_watch_service,
        webdav_metrics_collector: None,
    });
    
    let app = Router::new()
        .nest("/api/users", readur::routes::users::router())
        .nest("/api/auth", readur::routes::auth::router())
        .with_state(updated_state.clone());

    // Create admin user and regular user using TestAuthHelper
    let auth_helper = TestAuthHelper::new(app.clone());
    let admin_user = auth_helper.create_admin_user().await;
    let admin_token = auth_helper.login_user(&admin_user.username, &admin_user.password).await;
    let admin_id = admin_user.user_response.id;
    
    let regular_user = auth_helper.create_test_user().await;
    let user_token = auth_helper.login_user(&regular_user.username, &regular_user.password).await;
    let user_id = regular_user.user_response.id;

    // Test 1: Get user watch directory info (should not exist initially)
    let get_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("/api/users/{}/watch-directory", user_id))
                .header("Authorization", format!("Bearer {}", admin_token))
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(get_response.status(), StatusCode::OK);
    
    let get_body = axum::body::to_bytes(get_response.into_body(), usize::MAX).await?;
    let watch_info: Value = serde_json::from_slice(&get_body)?;
    
    assert_eq!(watch_info["username"], regular_user.username);
    assert_eq!(watch_info["exists"], false);
    assert_eq!(watch_info["enabled"], true);
    assert!(watch_info["watch_directory_path"].as_str().unwrap().contains(&regular_user.username));

    // Test 2: Create user watch directory
    let create_req = json!({
        "ensure_created": true
    });

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(&format!("/api/users/{}/watch-directory", user_id))
                .header("Authorization", format!("Bearer {}", admin_token))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&create_req)?))?,
        )
        .await?;

    assert_eq!(create_response.status(), StatusCode::OK);
    
    let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX).await?;
    let create_result: Value = serde_json::from_slice(&create_body)?;
    
    assert_eq!(create_result["success"], true);
    assert!(create_result["message"].as_str().unwrap().contains(&regular_user.username));
    assert!(create_result["watch_directory_path"].is_string());

    // Verify directory was created on filesystem
    let expected_path = temp_user_watch.path().join(&regular_user.username);
    assert!(expected_path.exists());
    assert!(expected_path.is_dir());

    // Test 3: Get user watch directory info again (should exist now)
    let get_response2 = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("/api/users/{}/watch-directory", user_id))
                .header("Authorization", format!("Bearer {}", admin_token))
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(get_response2.status(), StatusCode::OK);
    
    let get_body2 = axum::body::to_bytes(get_response2.into_body(), usize::MAX).await?;
    let watch_info2: Value = serde_json::from_slice(&get_body2)?;
    
    assert_eq!(watch_info2["exists"], true);

    // Test 4: Regular user can access their own watch directory
    let user_get_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("/api/users/{}/watch-directory", user_id))
                .header("Authorization", format!("Bearer {}", user_token))
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(user_get_response.status(), StatusCode::OK);

    // Test 5: Regular user cannot access another user's watch directory
    let forbidden_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("/api/users/{}/watch-directory", admin_id))
                .header("Authorization", format!("Bearer {}", user_token))
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(forbidden_response.status(), StatusCode::FORBIDDEN);

    // Test 6: Delete user watch directory (admin only)
    let delete_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(&format!("/api/users/{}/watch-directory", user_id))
                .header("Authorization", format!("Bearer {}", admin_token))
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(delete_response.status(), StatusCode::OK);
    
    let delete_body = axum::body::to_bytes(delete_response.into_body(), usize::MAX).await?;
    let delete_result: Value = serde_json::from_slice(&delete_body)?;
    
    assert_eq!(delete_result["success"], true);

    // Verify directory was removed from filesystem
    assert!(!expected_path.exists());

    // Test 7: Regular user cannot delete watch directories
    let user_delete_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(&format!("/api/users/{}/watch-directory", user_id))
                .header("Authorization", format!("Bearer {}", user_token))
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(user_delete_response.status(), StatusCode::FORBIDDEN);

    // Cleanup
    if let Err(e) = ctx.cleanup_and_close().await {
        eprintln!("Warning: Test cleanup failed: {}", e);
    }

    Ok(())
}

#[tokio::test]
async fn test_user_watch_service_security() -> Result<()> {
    let temp_user_watch = TempDir::new()?;
    let user_watch_service = UserWatchService::new(temp_user_watch.path());
    
    // Create test user
    let test_user = readur::models::User {
        id: Uuid::new_v4(),
        username: "testuser".to_string(),
        email: "test@test.com".to_string(),
        password_hash: Some("hash".to_string()),
        role: UserRole::User,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        oidc_subject: None,
        oidc_issuer: None,
        oidc_email: None,
        auth_provider: readur::models::user::AuthProvider::Local,
    };

    // Test 1: Normal username works
    let result = user_watch_service.ensure_user_directory(&test_user).await;
    assert!(result.is_ok());

    let user_dir = temp_user_watch.path().join("testuser");
    assert!(user_dir.exists());

    // Test 2: Security - usernames with path traversal attempts should be rejected
    let malicious_user = readur::models::User {
        id: Uuid::new_v4(),
        username: "../malicious".to_string(),
        email: "mal@test.com".to_string(),
        password_hash: Some("hash".to_string()),
        role: UserRole::User,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        oidc_subject: None,
        oidc_issuer: None,
        oidc_email: None,
        auth_provider: readur::models::user::AuthProvider::Local,
    };

    let malicious_result = user_watch_service.ensure_user_directory(&malicious_user).await;
    assert!(malicious_result.is_err());

    // Verify no malicious directory was created outside the base directory
    let malicious_dir = temp_user_watch.path().parent().unwrap().join("malicious");
    assert!(!malicious_dir.exists());

    // Test 3: Security - usernames with null bytes should be rejected
    let null_user = readur::models::User {
        id: Uuid::new_v4(),
        username: "test\0user".to_string(),
        email: "null@test.com".to_string(),
        password_hash: Some("hash".to_string()),
        role: UserRole::User,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        oidc_subject: None,
        oidc_issuer: None,
        oidc_email: None,
        auth_provider: readur::models::user::AuthProvider::Local,
    };

    let null_result = user_watch_service.ensure_user_directory(&null_user).await;
    assert!(null_result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_user_watch_directory_file_processing_simulation() -> Result<()> {
    let ctx = TestContext::new().await;
    
    // Enable per-user watch for this test
    let mut config = ctx.state.config.clone();
    let temp_user_watch = TempDir::new()?;
    config.user_watch_base_dir = temp_user_watch.path().to_string_lossy().to_string();
    config.enable_per_user_watch = true;
    
    // Update the state with the new config and user watch service
    let user_watch_service = Some(Arc::new(UserWatchService::new(&config.user_watch_base_dir)));
    let state = Arc::new(AppState {
        db: ctx.state.db.clone(),
        config: config.clone(),
        file_service: ctx.state.file_service.clone(),
        webdav_scheduler: None,
        source_scheduler: None,
        queue_service: ctx.state.queue_service.clone(),
        oidc_client: None,
        sync_progress_tracker: ctx.state.sync_progress_tracker.clone(),
        user_watch_service,
        webdav_metrics_collector: None,
    });
    
    // Create user watch manager to test file path mapping
    let user_watch_service = state.user_watch_service.as_ref().unwrap();
    let user_watch_manager = readur::scheduling::user_watch_manager::UserWatchManager::new(state.db.clone(), (**user_watch_service).clone());
    
    // Create test user
    let test_user = readur::models::User {
        id: Uuid::new_v4(),
        username: "filetest".to_string(),
        email: "filetest@test.com".to_string(),
        password_hash: Some("hash".to_string()),
        role: UserRole::User,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        oidc_subject: None,
        oidc_issuer: None,
        oidc_email: None,
        auth_provider: readur::models::user::AuthProvider::Local,
    };

    // Insert user into database
    let created_user = state.db.create_user(readur::models::CreateUser {
        username: test_user.username.clone(),
        email: test_user.email.clone(), 
        password: "test_password".to_string(),
        role: Some(UserRole::User),
    }).await?;

    // Create user watch directory
    let user_watch_service = state.user_watch_service.as_ref().unwrap();
    let user_dir_path = user_watch_service.ensure_user_directory(&created_user).await?;

    // Test file path to user mapping
    let test_file_path = user_dir_path.join("test_document.pdf");
    std::fs::File::create(&test_file_path)?;

    // Wait a moment for caching
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Test that the user watch manager can map file paths to users
    let mapped_user_result = user_watch_manager.get_user_by_file_path(&test_file_path).await?;
    let mapped_user_id = mapped_user_result.as_ref().map(|user| user.id);
    
    // The user should be discoverable via file path
    assert!(mapped_user_id.is_some());
    if let Some(user_id) = mapped_user_id {
        assert_eq!(user_id, created_user.id);
    }

    // Test invalid path (should not map to any user)
    let invalid_path = PathBuf::from("/invalid/path/document.pdf");
    let invalid_mapping_result = user_watch_manager.get_user_by_file_path(&invalid_path).await?;
    assert!(invalid_mapping_result.is_none());

    // Cleanup
    if let Err(e) = ctx.cleanup_and_close().await {
        eprintln!("Warning: Test cleanup failed: {}", e);
    }

    Ok(())
}

#[tokio::test]  
async fn test_per_user_watch_disabled() -> Result<()> {
    let ctx = TestContext::new().await;
    
    // Ensure per-user watch is disabled
    let mut config = ctx.state.config.clone();
    config.enable_per_user_watch = false;
    
    // Update the state with the disabled config (no user watch service)
    let updated_state = Arc::new(AppState {
        db: ctx.state.db.clone(),
        config,
        file_service: ctx.state.file_service.clone(),
        webdav_scheduler: None,
        source_scheduler: None,
        queue_service: ctx.state.queue_service.clone(),
        oidc_client: None,
        sync_progress_tracker: ctx.state.sync_progress_tracker.clone(),
        user_watch_service: None, // Disabled
        webdav_metrics_collector: None,
    });
    
    let app = Router::new()
        .nest("/api/users", readur::routes::users::router())
        .nest("/api/auth", readur::routes::auth::router())
        .with_state(updated_state.clone());

    // Create admin user and regular user using TestAuthHelper
    let auth_helper = TestAuthHelper::new(app.clone());
    let admin_user = auth_helper.create_admin_user().await;
    let admin_token = auth_helper.login_user(&admin_user.username, &admin_user.password).await;
    
    let regular_user = auth_helper.create_test_user().await;
    let user_id = regular_user.user_response.id;

    // Try to get user watch directory info when feature is disabled
    let get_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("/api/users/{}/watch-directory", user_id))
                .header("Authorization", format!("Bearer {}", admin_token))
                .body(Body::empty())?,
        )
        .await?;

    // Should return internal server error when feature is disabled
    assert_eq!(get_response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    // Cleanup
    if let Err(e) = ctx.cleanup_and_close().await {
        eprintln!("Warning: Test cleanup failed: {}", e);
    }

    Ok(())
}