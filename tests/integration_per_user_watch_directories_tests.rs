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
    config::Config,
    db::Database,
    models::{CreateUser, UserRole},
    services::user_watch_service::UserWatchService,
    AppState,
};

/// Helper to create test configuration with per-user watch enabled
async fn create_test_config() -> Result<(Config, TempDir, TempDir)> {
    let temp_upload_dir = TempDir::new()?;
    let temp_watch_dir = TempDir::new()?;
    let temp_user_watch_dir = TempDir::new()?;
    
    let config = Config {
        database_url: std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://readur:readur@localhost/readur_test".to_string()),
        server_address: "127.0.0.1:0".to_string(),
        jwt_secret: "test_secret".to_string(),
        upload_path: temp_upload_dir.path().to_string_lossy().to_string(),
        watch_folder: temp_watch_dir.path().to_string_lossy().to_string(),
        user_watch_base_dir: temp_user_watch_dir.path().to_string_lossy().to_string(),
        enable_per_user_watch: true,
        allowed_file_types: vec!["pdf".to_string(), "txt".to_string(), "png".to_string()],
        watch_interval_seconds: Some(10),
        file_stability_check_ms: Some(1000),
        max_file_age_hours: None,
        ocr_language: "eng".to_string(),
        concurrent_ocr_jobs: 1,
        ocr_timeout_seconds: 30,
        max_file_size_mb: 10,
        memory_limit_mb: 512,
        cpu_priority: "normal".to_string(),
        oidc_enabled: false,
        oidc_client_id: None,
        oidc_client_secret: None,
        oidc_issuer_url: None,
        oidc_redirect_uri: None,
    };
    
    Ok((config, temp_upload_dir, temp_user_watch_dir))
}

/// Helper to create test app state
async fn create_test_app_state(config: Config) -> Result<Arc<AppState>> {
    let db = Database::new(&config.database_url).await?;
    let queue_service = Arc::new(readur::ocr::queue::OcrQueueService::new(
        db.clone(),
        db.get_pool().clone(),
        1,
    ));
    
    let user_watch_service = if config.enable_per_user_watch {
        Some(Arc::new(UserWatchService::new(&config.user_watch_base_dir)))
    } else {
        None
    };

    Ok(Arc::new(AppState {
        db,
        config,
        webdav_scheduler: None,
        source_scheduler: None,
        queue_service,
        oidc_client: None,
        sync_progress_tracker: Arc::new(readur::services::sync_progress_tracker::SyncProgressTracker::new()),
        user_watch_service,
    }))
}

/// Helper to create test user and get auth token
async fn create_test_user_and_login(
    app: &Router,
    username: &str,
    email: &str,
    role: UserRole,
) -> Result<(String, Uuid)> {
    // Create user
    let create_user_req = CreateUser {
        username: username.to_string(),
        email: email.to_string(),
        password: "test_password".to_string(),
        role: Some(role),
    };

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/users")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&create_user_req)?))?,
        )
        .await?;

    assert_eq!(create_response.status(), StatusCode::OK);
    
    let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX).await?;
    let user_response: Value = serde_json::from_slice(&create_body)?;
    let user_id = Uuid::parse_str(user_response["id"].as_str().unwrap())?;

    // Login to get token
    let login_req = json!({
        "username": username,
        "password": "test_password"
    });

    let login_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&login_req)?))?,
        )
        .await?;

    assert_eq!(login_response.status(), StatusCode::OK);
    
    let login_body = axum::body::to_bytes(login_response.into_body(), usize::MAX).await?;
    let login_response: Value = serde_json::from_slice(&login_body)?;
    let token = login_response["token"].as_str().unwrap().to_string();

    Ok((token, user_id))
}

#[tokio::test]
async fn test_per_user_watch_directory_lifecycle() -> Result<()> {
    let (config, _temp_upload, temp_user_watch) = create_test_config().await?;
    let state = create_test_app_state(config).await?;
    
    let app = Router::new()
        .nest("/api/users", readur::routes::users::router())
        .nest("/api/auth", readur::routes::auth::router())
        .with_state(state.clone());

    // Create admin user and regular user
    let (admin_token, admin_id) = create_test_user_and_login(&app, "admin", "admin@test.com", UserRole::Admin).await?;
    let (user_token, user_id) = create_test_user_and_login(&app, "testuser", "test@test.com", UserRole::User).await?;

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
    
    assert_eq!(watch_info["username"], "testuser");
    assert_eq!(watch_info["exists"], false);
    assert_eq!(watch_info["enabled"], true);
    assert!(watch_info["watch_directory_path"].as_str().unwrap().contains("testuser"));

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
    assert!(create_result["message"].as_str().unwrap().contains("testuser"));
    assert!(create_result["watch_directory_path"].is_string());

    // Verify directory was created on filesystem
    let expected_path = temp_user_watch.path().join("testuser");
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

    Ok(())
}

#[tokio::test]
async fn test_user_watch_service_security() -> Result<()> {
    let (config, _temp_upload, temp_user_watch) = create_test_config().await?;
    
    let user_watch_service = UserWatchService::new(&config.user_watch_base_dir);
    
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
    let (config, _temp_upload, temp_user_watch) = create_test_config().await?;
    let state = create_test_app_state(config.clone()).await?;
    
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

    Ok(())
}

#[tokio::test]  
async fn test_per_user_watch_disabled() -> Result<()> {
    // Create config with per-user watch disabled
    let (mut config, _temp_upload, _temp_user_watch) = create_test_config().await?;
    config.enable_per_user_watch = false;
    
    let state = create_test_app_state(config).await?;
    
    let app = Router::new()
        .nest("/api/users", readur::routes::users::router())
        .nest("/api/auth", readur::routes::auth::router())
        .with_state(state.clone());

    // Create admin user
    let (admin_token, _admin_id) = create_test_user_and_login(&app, "admin", "admin@test.com", UserRole::Admin).await?;
    let (_user_token, user_id) = create_test_user_and_login(&app, "testuser", "test@test.com", UserRole::User).await?;

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

    Ok(())
}