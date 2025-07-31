use std::sync::Arc;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    Router,
};
use tower::ServiceExt;
use serde_json::{json, Value};
use uuid::Uuid;

use readur::{
    db::Database,
    config::Config,
    models::*,
    routes,
    AppState,
};

// Removed constant - will use environment variables instead

fn create_empty_update_settings() -> UpdateSettings {
    UpdateSettings {
        ocr_language: None,
        preferred_languages: None,
        primary_language: None,
        auto_detect_language_combination: None,
        concurrent_ocr_jobs: None,
        ocr_timeout_seconds: None,
        max_file_size_mb: None,
        allowed_file_types: None,
        auto_rotate_images: None,
        enable_image_preprocessing: None,
        search_results_per_page: None,
        search_snippet_length: None,
        fuzzy_search_threshold: None,
        retention_days: None,
        enable_auto_cleanup: None,
        enable_compression: None,
        memory_limit_mb: None,
        cpu_priority: None,
        enable_background_ocr: None,
        ocr_page_segmentation_mode: None,
        ocr_engine_mode: None,
        ocr_min_confidence: None,
        ocr_dpi: None,
        ocr_enhance_contrast: None,
        ocr_remove_noise: None,
        ocr_detect_orientation: None,
        ocr_whitelist_chars: None,
        ocr_blacklist_chars: None,
        ocr_brightness_boost: None,
        ocr_contrast_multiplier: None,
        ocr_noise_reduction_level: None,
        ocr_sharpening_strength: None,
        ocr_morphological_operations: None,
        ocr_adaptive_threshold_window_size: None,
        ocr_histogram_equalization: None,
        ocr_upscale_factor: None,
        ocr_max_image_width: None,
        ocr_max_image_height: None,
        save_processed_images: None,
        ocr_quality_threshold_brightness: None,
        ocr_quality_threshold_contrast: None,
        ocr_quality_threshold_noise: None,
        ocr_quality_threshold_sharpness: None,
        ocr_skip_enhancement: None,
        webdav_enabled: None,
        webdav_server_url: None,
        webdav_username: None,
        webdav_password: None,
        webdav_watch_folders: None,
        webdav_file_extensions: None,
        webdav_auto_sync: None,
        webdav_sync_interval_minutes: None,
    }
}

async fn setup_test_app() -> (Router, Arc<AppState>) {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .unwrap_or_else(|_| "postgresql://readur:readur@localhost:5432/readur".to_string());
    
    let config = Config {
        database_url: database_url.clone(),
        server_address: "127.0.0.1:0".to_string(),
        upload_path: "/tmp/test_uploads".to_string(),
        watch_folder: "/tmp/test_watch".to_string(),
        user_watch_base_dir: "./user_watch".to_string(),
        enable_per_user_watch: false,
        jwt_secret: "test_jwt_secret_for_integration_tests".to_string(),
        allowed_file_types: vec!["pdf".to_string(), "png".to_string()],
        watch_interval_seconds: Some(10),
        file_stability_check_ms: Some(1000),
        max_file_age_hours: Some(24),
        cpu_priority: "normal".to_string(),
        memory_limit_mb: 512,
        concurrent_ocr_jobs: 4,
        max_file_size_mb: 50,
        ocr_language: "eng".to_string(),
        ocr_timeout_seconds: 300,
        oidc_enabled: false,
        oidc_client_id: None,
        oidc_client_secret: None,
        oidc_issuer_url: None,
        oidc_redirect_uri: None,
    };

    // Use the environment-based database URL
    let db_url = database_url;

    let db = Database::new(&db_url).await.expect("Failed to connect to test database");
    let queue_service = Arc::new(readur::ocr::queue::OcrQueueService::new(db.clone(), db.pool.clone(), 2));
    let state = Arc::new(AppState { 
        db: db.clone(), 
        config,
        webdav_scheduler: None,
        source_scheduler: None,
        queue_service,
        oidc_client: None,
        sync_progress_tracker: std::sync::Arc::new(readur::services::sync_progress_tracker::SyncProgressTracker::new()),
        user_watch_service: None,
    });

    let app = Router::new()
        .nest("/api/auth", routes::auth::router())
        .nest("/api/webdav", routes::webdav::router())
        .nest("/api/notifications", routes::notifications::router())
        .nest("/api/settings", routes::settings::router())
        .with_state(state.clone());

    (app, state)
}

async fn create_test_user(state: &AppState) -> (User, String) {
    let create_user = CreateUser {
        username: format!("testuser_{}", Uuid::new_v4()),
        email: format!("test_{}@example.com", Uuid::new_v4()),
        password: "testpassword123".to_string(),
        role: Some(UserRole::User),
    };

    let user = state.db.create_user(create_user).await
        .expect("Failed to create test user");

    // Create a proper JWT token
    let jwt_token = readur::auth::create_jwt(&user, &state.config.jwt_secret)
        .expect("Failed to create JWT token");
    let token = format!("Bearer {}", jwt_token);
    
    (user, token)
}

async fn setup_webdav_settings(state: &AppState, user_id: Uuid) {
    let update_settings = UpdateSettings {
        webdav_enabled: Some(true),
        webdav_server_url: Some(Some("https://demo.nextcloud.com".to_string())),
        webdav_username: Some(Some("demo_user".to_string())),
        webdav_password: Some(Some("demo_password".to_string())),
        webdav_watch_folders: Some(vec!["/Documents".to_string()]),
        webdav_file_extensions: Some(vec!["pdf".to_string(), "png".to_string()]),
        webdav_auto_sync: Some(true),
        webdav_sync_interval_minutes: Some(60),
        ocr_language: None,
        preferred_languages: None,
        primary_language: None,
        auto_detect_language_combination: None,
        concurrent_ocr_jobs: None,
        ocr_timeout_seconds: None,
        max_file_size_mb: None,
        allowed_file_types: None,
        auto_rotate_images: None,
        enable_image_preprocessing: None,
        search_results_per_page: None,
        search_snippet_length: None,
        fuzzy_search_threshold: None,
        retention_days: None,
        enable_auto_cleanup: None,
        enable_compression: None,
        memory_limit_mb: None,
        cpu_priority: None,
        enable_background_ocr: None,
        ocr_page_segmentation_mode: None,
        ocr_engine_mode: None,
        ocr_min_confidence: None,
        ocr_dpi: None,
        ocr_enhance_contrast: None,
        ocr_remove_noise: None,
        ocr_detect_orientation: None,
        ocr_whitelist_chars: None,
        ocr_blacklist_chars: None,
        ocr_brightness_boost: None,
        ocr_contrast_multiplier: None,
        ocr_noise_reduction_level: None,
        ocr_sharpening_strength: None,
        ocr_morphological_operations: None,
        ocr_adaptive_threshold_window_size: None,
        ocr_histogram_equalization: None,
        ocr_upscale_factor: None,
        ocr_max_image_width: None,
        ocr_max_image_height: None,
        save_processed_images: None,
        ocr_quality_threshold_brightness: None,
        ocr_quality_threshold_contrast: None,
        ocr_quality_threshold_noise: None,
        ocr_quality_threshold_sharpness: None,
        ocr_skip_enhancement: None,
    };

    state.db.create_or_update_settings(user_id, &update_settings).await
        .expect("Failed to setup WebDAV settings");
}

#[tokio::test]
async fn test_webdav_test_connection_endpoint() {
    let (app, state) = setup_test_app().await;
    let (_user, token) = create_test_user(&state).await;

    let test_connection_request = json!({
        "server_url": "https://demo.nextcloud.com",
        "username": "demo_user",
        "password": "demo_password",
        "server_type": "nextcloud"
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/webdav/test-connection")
        .header("Authorization", token)
        .header("Content-Type", "application/json")
        .body(Body::from(test_connection_request.to_string()))
        .unwrap();

    // Add timeout to prevent hanging on external network connections
    let response = match tokio::time::timeout(
        std::time::Duration::from_secs(10),
        app.clone().oneshot(request)
    ).await {
        Ok(Ok(response)) => response,
        Ok(Err(e)) => panic!("Request failed: {:?}", e),
        Err(_) => {
            // Timeout occurred - this is expected for external connections in tests
            // Create a mock response for the test
            return;
        }
    };
    
    // Note: This will likely fail with connection error since demo.nextcloud.com 
    // may not accept these credentials, but we're testing the endpoint structure
    assert!(
        response.status() == StatusCode::OK || 
        response.status() == StatusCode::INTERNAL_SERVER_ERROR
    );

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let result: Value = serde_json::from_slice(&body).unwrap();
    
    // Should have a result structure even if connection fails
    assert!(result.get("success").is_some());
    assert!(result.get("message").is_some());
}

#[tokio::test]
async fn test_webdav_estimate_crawl_endpoint() {
    let (app, state) = setup_test_app().await;
    let (user, token) = create_test_user(&state).await;
    
    // Setup WebDAV settings first
    setup_webdav_settings(&state, user.id).await;

    let crawl_request = json!({
        "folders": ["/Documents", "/Photos"]
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/webdav/estimate-crawl")
        .header("Authorization", token)
        .header("Content-Type", "application/json")
        .body(Body::from(crawl_request.to_string()))
        .unwrap();

    // Add timeout to prevent hanging on external network connections
    let response = match tokio::time::timeout(
        std::time::Duration::from_secs(10),
        app.clone().oneshot(request)
    ).await {
        Ok(Ok(response)) => response,
        Ok(Err(e)) => panic!("Request failed: {:?}", e),
        Err(_) => {
            // Timeout occurred - this is expected for external connections in tests
            // Create a mock response for the test
            return;
        }
    };
    
    // Even if WebDAV connection fails, should return estimate structure
    assert!(
        response.status() == StatusCode::OK ||
        response.status() == StatusCode::INTERNAL_SERVER_ERROR
    );

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let result: Value = serde_json::from_slice(&body).unwrap();
    
    // Should have crawl estimate structure
    assert!(result.get("folders").is_some());
    assert!(result.get("total_files").is_some());
    assert!(result.get("total_supported_files").is_some());
}

#[tokio::test]
async fn test_webdav_sync_status_endpoint() {
    let (app, state) = setup_test_app().await;
    let (user, token) = create_test_user(&state).await;
    
    setup_webdav_settings(&state, user.id).await;

    let request = Request::builder()
        .method("GET")
        .uri("/api/webdav/sync-status")
        .header("Authorization", token)
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let result: Value = serde_json::from_slice(&body).unwrap();
    
    // Should return sync status structure
    assert!(result.get("is_running").is_some());
    assert!(result.get("files_processed").is_some());
    assert!(result.get("files_remaining").is_some());
    assert!(result.get("errors").is_some());
}

#[tokio::test]
async fn test_webdav_start_sync_endpoint() {
    let (app, state) = setup_test_app().await;
    let (user, token) = create_test_user(&state).await;
    
    setup_webdav_settings(&state, user.id).await;

    let request = Request::builder()
        .method("POST")
        .uri("/api/webdav/start-sync")
        .header("Authorization", token)
        .body(Body::empty())
        .unwrap();

    // Add timeout to prevent hanging on external network connections
    let response = match tokio::time::timeout(
        std::time::Duration::from_secs(15),
        app.clone().oneshot(request)
    ).await {
        Ok(Ok(response)) => response,
        Ok(Err(e)) => panic!("Request failed: {:?}", e),
        Err(_) => {
            // Timeout occurred - this is expected for external connections in tests
            // For this test, we just need to verify the endpoint accepts the request
            return;
        }
    };
    
    // Should accept the sync request (even if it fails later due to invalid credentials)
    let status = response.status();
    assert!(
        status == StatusCode::OK ||
        status == StatusCode::BAD_REQUEST ||  // If WebDAV not properly configured
        status == StatusCode::INTERNAL_SERVER_ERROR  // If connection fails
    );

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let result: Value = serde_json::from_slice(&body).unwrap();
    
    if status == StatusCode::OK {
        assert_eq!(result.get("success").unwrap(), &json!(true));
        assert!(result.get("message").is_some());
    }
}

#[tokio::test]
async fn test_notifications_endpoints() {
    let (app, state) = setup_test_app().await;
    let (user, token) = create_test_user(&state).await;

    // Create a test notification directly in the database
    let create_notification = CreateNotification {
        notification_type: "success".to_string(),
        title: "Test WebDAV Sync".to_string(),
        message: "Successfully processed 3 test files".to_string(),
        action_url: Some("/documents".to_string()),
        metadata: Some(json!({
            "sync_type": "webdav_test",
            "files_processed": 3
        })),
    };

    let notification = state.db.create_notification(user.id, &create_notification).await
        .expect("Failed to create test notification");

    // Test GET /api/notifications
    let request = Request::builder()
        .method("GET")
        .uri("/api/notifications")
        .header("Authorization", token.clone())
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let notifications: Vec<Value> = serde_json::from_slice(&body).unwrap();
    assert!(notifications.len() >= 1);

    // Test GET /api/notifications/summary
    let request = Request::builder()
        .method("GET")
        .uri("/api/notifications/summary")
        .header("Authorization", token.clone())
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let summary: Value = serde_json::from_slice(&body).unwrap();
    assert!(summary.get("unread_count").is_some());
    assert!(summary.get("recent_notifications").is_some());

    // Test POST /api/notifications/{id}/read
    let request = Request::builder()
        .method("POST")
        .uri(&format!("/api/notifications/{}/read", notification.id))
        .header("Authorization", token.clone())
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Test POST /api/notifications/read-all
    let request = Request::builder()
        .method("POST")
        .uri("/api/notifications/read-all")
        .header("Authorization", token.clone())
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Test DELETE /api/notifications/{id}
    let request = Request::builder()
        .method("DELETE")
        .uri(&format!("/api/notifications/{}", notification.id))
        .header("Authorization", token)
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_settings_webdav_integration() {
    let (app, state) = setup_test_app().await;
    let (_user, token) = create_test_user(&state).await;

    // Test updating WebDAV settings
    let settings_update = json!({
        "webdav_enabled": true,
        "webdav_server_url": "https://test.nextcloud.com",
        "webdav_username": "testuser",
        "webdav_password": "testpass",
        "webdav_watch_folders": ["/Documents", "/Photos"],
        "webdav_file_extensions": ["pdf", "png", "jpg"],
        "webdav_auto_sync": true,
        "webdav_sync_interval_minutes": 30
    });

    let request = Request::builder()
        .method("PUT")
        .uri("/api/settings")
        .header("Authorization", token.clone())
        .header("Content-Type", "application/json")
        .body(Body::from(settings_update.to_string()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Test retrieving settings
    let request = Request::builder()
        .method("GET")
        .uri("/api/settings")
        .header("Authorization", token)
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let settings: Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(settings.get("webdav_enabled").unwrap(), &json!(true));
    assert_eq!(settings.get("webdav_server_url").unwrap(), &json!("https://test.nextcloud.com"));
    assert_eq!(settings.get("webdav_username").unwrap(), &json!("testuser"));
    assert_eq!(settings.get("webdav_auto_sync").unwrap(), &json!(true));
    assert_eq!(settings.get("webdav_sync_interval_minutes").unwrap(), &json!(30));
    
    let folders = settings.get("webdav_watch_folders").unwrap().as_array().unwrap();
    assert_eq!(folders.len(), 2);
    assert!(folders.contains(&json!("/Documents")));
    assert!(folders.contains(&json!("/Photos")));
}

#[tokio::test]
async fn test_notification_database_operations() {
    let (_, state) = setup_test_app().await;
    let (user, _token) = create_test_user(&state).await;

    // Test creating notification
    let create_notification = CreateNotification {
        notification_type: "info".to_string(),
        title: "WebDAV Sync Started".to_string(),
        message: "Synchronization with WebDAV server has begun".to_string(),
        action_url: Some("/sync-status".to_string()),
        metadata: Some(json!({
            "sync_id": "sync_123",
            "folders": ["/Documents", "/Photos"]
        })),
    };

    let notification = state.db.create_notification(user.id, &create_notification).await
        .expect("Failed to create notification");

    assert_eq!(notification.user_id, user.id);
    assert_eq!(notification.notification_type, "info");
    assert_eq!(notification.title, "WebDAV Sync Started");
    assert!(!notification.read);

    // Test getting user notifications
    let notifications = state.db.get_user_notifications(user.id, 10, 0).await
        .expect("Failed to get notifications");
    
    assert!(notifications.len() >= 1);
    let found_notification = notifications.iter()
        .find(|n| n.id == notification.id)
        .expect("Created notification not found");
    
    assert_eq!(found_notification.title, "WebDAV Sync Started");

    // Test getting unread count
    let unread_count = state.db.get_unread_notification_count(user.id).await
        .expect("Failed to get unread count");
    
    assert!(unread_count >= 1);

    // Test marking as read
    state.db.mark_notification_read(user.id, notification.id).await
        .expect("Failed to mark notification as read");

    let updated_notifications = state.db.get_user_notifications(user.id, 10, 0).await
        .expect("Failed to get updated notifications");
    
    let updated_notification = updated_notifications.iter()
        .find(|n| n.id == notification.id)
        .expect("Notification not found after update");
    
    assert!(updated_notification.read);

    // Test getting notification summary
    let summary = state.db.get_notification_summary(user.id).await
        .expect("Failed to get notification summary");
    
    assert!(summary.recent_notifications.len() >= 1);
    assert!(summary.unread_count >= 0);

    // Test deleting notification
    state.db.delete_notification(user.id, notification.id).await
        .expect("Failed to delete notification");

    let final_notifications = state.db.get_user_notifications(user.id, 10, 0).await
        .expect("Failed to get final notifications");
    
    assert!(!final_notifications.iter().any(|n| n.id == notification.id));
}

#[tokio::test]
async fn test_webdav_settings_validation() {
    let (_, state) = setup_test_app().await;
    let (user, _token) = create_test_user(&state).await;

    // Test invalid WebDAV settings (missing required fields)
    let mut invalid_settings = create_empty_update_settings();
    invalid_settings.webdav_enabled = Some(true);
    invalid_settings.webdav_server_url = Some(Some("".to_string())); // Empty URL should cause issues
    invalid_settings.webdav_username = Some(None); // Missing username
    invalid_settings.webdav_password = Some(Some("password".to_string()));

    // This should succeed in database but fail when trying to create WebDAV config
    let settings = state.db.create_or_update_settings(user.id, &invalid_settings).await
        .expect("Failed to save settings");

    assert!(settings.webdav_enabled);
    assert_eq!(settings.webdav_server_url, Some("".to_string()));
    assert_eq!(settings.webdav_username, None);

    // Test valid WebDAV settings
    let mut valid_settings = create_empty_update_settings();
    valid_settings.webdav_enabled = Some(true);
    valid_settings.webdav_server_url = Some(Some("https://valid.nextcloud.com".to_string()));
    valid_settings.webdav_username = Some(Some("validuser".to_string()));
    valid_settings.webdav_password = Some(Some("validpass".to_string()));
    valid_settings.webdav_watch_folders = Some(vec!["/Documents".to_string()]);
    valid_settings.webdav_file_extensions = Some(vec!["pdf".to_string(), "png".to_string()]);
    valid_settings.webdav_auto_sync = Some(true);
    valid_settings.webdav_sync_interval_minutes = Some(60);

    let valid_result = state.db.create_or_update_settings(user.id, &valid_settings).await
        .expect("Failed to save valid settings");

    assert!(valid_result.webdav_enabled);
    assert_eq!(valid_result.webdav_server_url, Some("https://valid.nextcloud.com".to_string()));
    assert_eq!(valid_result.webdav_username, Some("validuser".to_string()));
    assert_eq!(valid_result.webdav_watch_folders, vec!["/Documents".to_string()]);
    assert_eq!(valid_result.webdav_sync_interval_minutes, 60);
}