mod tests {
    use readur::models::{AuthProvider, CreateUser, UserRole};
    use axum::http::StatusCode;
    use serde_json::json;
    use tower::util::ServiceExt;
    use wiremock::{matchers::{method, path, query_param, header}, Mock, MockServer, ResponseTemplate};
    use std::sync::Arc;
    use readur::{AppState, oidc::OidcClient};
    use uuid;

    async fn create_test_app_simple() -> (axum::Router, ()) {
        // Use TEST_DATABASE_URL directly, no containers
        let database_url = std::env::var("TEST_DATABASE_URL")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .unwrap_or_else(|_| "postgresql://readur:readur@localhost:5432/readur".to_string());
        
        let config = readur::config::Config {
            database_url: database_url.clone(),
            server_address: "127.0.0.1:0".to_string(),
            jwt_secret: "test-secret".to_string(),
            upload_path: "./test-uploads".to_string(),
            watch_folder: "./test-watch".to_string(),
            user_watch_base_dir: "./user_watch".to_string(),
            enable_per_user_watch: false,
            allowed_file_types: vec!["pdf".to_string()],
            watch_interval_seconds: Some(30),
            file_stability_check_ms: Some(500),
            max_file_age_hours: None,
            ocr_language: "eng".to_string(),
            concurrent_ocr_jobs: 2,
            ocr_timeout_seconds: 60,
            max_file_size_mb: 10,
            memory_limit_mb: 256,
            cpu_priority: "normal".to_string(),
            oidc_enabled: false,
            oidc_client_id: None,
            oidc_client_secret: None,
            oidc_issuer_url: None,
            oidc_redirect_uri: None,
            oidc_auto_register: None,
            allow_local_auth: None,
            s3_enabled: false,
            s3_config: None,
        };

        let db = readur::db::Database::new(&config.database_url).await.unwrap();
        
        // Retry migration up to 3 times to handle concurrent test execution
        for attempt in 1..=3 {
            match db.migrate().await {
                Ok(_) => break,
                Err(e) if attempt < 3 && e.to_string().contains("tuple concurrently updated") => {
                    // Wait a bit and retry
                    tokio::time::sleep(tokio::time::Duration::from_millis(100 * attempt)).await;
                    continue;
                }
                Err(e) => panic!("Migration failed after {} attempts: {}", attempt, e),
            }
        }
        
        // Create file service
        let storage_config = readur::storage::StorageConfig::Local { upload_path: config.upload_path.clone() };
        let storage_backend = readur::storage::factory::create_storage_backend(storage_config).await.unwrap();
        let file_service = Arc::new(readur::services::file_service::FileService::with_storage(config.upload_path.clone(), storage_backend));
        
        let app = axum::Router::new()
            .nest("/api/auth", readur::routes::auth::router())
            .with_state(Arc::new(AppState {
                db: db.clone(),
                config,
                file_service: file_service.clone(),
                webdav_scheduler: None,
                source_scheduler: None,
                queue_service: Arc::new(readur::ocr::queue::OcrQueueService::new(
                    db.clone(),
                    db.pool.clone(),
                    2,
                    file_service.clone()
                )),
                oidc_client: None,
                sync_progress_tracker: std::sync::Arc::new(readur::services::sync_progress_tracker::SyncProgressTracker::new()),
                user_watch_service: None,
        webdav_metrics_collector: None,
            }));

        (app, ())
    }

    async fn create_test_app_with_oidc() -> (axum::Router, MockServer) {
        let mock_server = MockServer::start().await;
        
        // Mock OIDC discovery endpoint
        let discovery_response = json!({
            "issuer": mock_server.uri(),
            "authorization_endpoint": format!("{}/auth", mock_server.uri()),
            "token_endpoint": format!("{}/token", mock_server.uri()),
            "userinfo_endpoint": format!("{}/userinfo", mock_server.uri())
        });

        Mock::given(method("GET"))
            .and(path("/.well-known/openid-configuration"))
            .respond_with(ResponseTemplate::new(200).set_body_json(discovery_response))
            .mount(&mock_server)
            .await;

        // Use TEST_DATABASE_URL directly, no containers
        let database_url = std::env::var("TEST_DATABASE_URL")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .unwrap_or_else(|_| "postgresql://readur:readur@localhost:5432/readur".to_string());
        
        // Update the app state to include OIDC client
        let config = readur::config::Config {
            database_url: database_url.clone(),
            server_address: "127.0.0.1:0".to_string(),
            jwt_secret: "test-secret".to_string(),
            upload_path: "./test-uploads".to_string(),
            watch_folder: "./test-watch".to_string(),
            user_watch_base_dir: "./user_watch".to_string(),
            enable_per_user_watch: false,
            allowed_file_types: vec!["pdf".to_string()],
            watch_interval_seconds: Some(30),
            file_stability_check_ms: Some(500),
            max_file_age_hours: None,
            ocr_language: "eng".to_string(),
            concurrent_ocr_jobs: 2,
            ocr_timeout_seconds: 60,
            max_file_size_mb: 10,
            memory_limit_mb: 256,
            cpu_priority: "normal".to_string(),
            oidc_enabled: true,
            oidc_client_id: Some("test-client-id".to_string()),
            oidc_client_secret: Some("test-client-secret".to_string()),
            oidc_issuer_url: Some(mock_server.uri()),
            oidc_redirect_uri: Some("http://localhost:8000/auth/oidc/callback".to_string()),
            oidc_auto_register: Some(true),
            allow_local_auth: Some(true),
            s3_enabled: false,
            s3_config: None,
        };

        let oidc_client = match OidcClient::new(&config).await {
            Ok(client) => Some(Arc::new(client)),
            Err(e) => {
                panic!("OIDC client creation failed: {}", e);
            }
        };
        
        // Connect to the database and run migrations with retry logic for concurrency
        let db = readur::db::Database::new(&config.database_url).await.unwrap();
        
        // Retry migration up to 3 times to handle concurrent test execution
        for attempt in 1..=3 {
            match db.migrate().await {
                Ok(_) => break,
                Err(e) if attempt < 3 && e.to_string().contains("tuple concurrently updated") => {
                    // Wait a bit and retry
                    tokio::time::sleep(tokio::time::Duration::from_millis(100 * attempt)).await;
                    continue;
                }
                Err(e) => panic!("Migration failed after {} attempts: {}", attempt, e),
            }
        }
        
        // Create file service for OIDC app
        let storage_config = readur::storage::StorageConfig::Local { upload_path: config.upload_path.clone() };
        let storage_backend = readur::storage::factory::create_storage_backend(storage_config).await.unwrap();
        let file_service = Arc::new(readur::services::file_service::FileService::with_storage(config.upload_path.clone(), storage_backend));
        
        // Create app with OIDC configuration
        let app = axum::Router::new()
            .nest("/api/auth", readur::routes::auth::router())
            .with_state(Arc::new(AppState {
                db: db.clone(),
                config,
                file_service: file_service.clone(),
                webdav_scheduler: None,
                source_scheduler: None,
                queue_service: Arc::new(readur::ocr::queue::OcrQueueService::new(
                    db.clone(),
                    db.pool.clone(),
                    2,
                    file_service.clone()
                )),
                oidc_client,
                sync_progress_tracker: std::sync::Arc::new(readur::services::sync_progress_tracker::SyncProgressTracker::new()),
                user_watch_service: None,
        webdav_metrics_collector: None,
            }));

        (app, mock_server)
    }

    #[tokio::test]
    async fn test_oidc_login_redirect() {
        let (app, _mock_server) = create_test_app_with_oidc().await;

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("GET")
                    .uri("/api/auth/oidc/login")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        
        let location = response.headers().get("location").unwrap().to_str().unwrap();
        assert!(location.contains("/auth"));
        assert!(location.contains("client_id=test-client-id"));
        assert!(location.contains("scope=openid"));
    }

    #[tokio::test]
    async fn test_oidc_login_disabled() {
        let (app, _container) = create_test_app_simple().await; // Regular app without OIDC

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("GET")
                    .uri("/api/auth/oidc/login")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_oidc_callback_missing_code() {
        let (app, _mock_server) = create_test_app_with_oidc().await;

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("GET")
                    .uri("/api/auth/oidc/callback")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_oidc_callback_with_error() {
        let (app, _mock_server) = create_test_app_with_oidc().await;

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("GET")
                    .uri("/api/auth/oidc/callback?error=access_denied")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_oidc_callback_success_new_user() {
        let (app, mock_server) = create_test_app_with_oidc().await;
        
        // Generate random identifiers to avoid test interference
        let test_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        let test_username = format!("oidcuser_{}", test_id);
        let test_email = format!("oidc_{}@example.com", test_id);
        let test_subject = format!("oidc-user-{}", test_id);
        
        // Clean up any existing test user to ensure test isolation
        let database_url = std::env::var("TEST_DATABASE_URL")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .unwrap_or_else(|_| "postgresql://readur:readur@localhost:5432/readur".to_string());
        let db = readur::db::Database::new(&database_url).await.unwrap();
        
        // Delete any existing user with the test username or OIDC subject
        let _ = sqlx::query("DELETE FROM users WHERE username = $1 OR oidc_subject = $2")
            .bind(&test_username)
            .bind(&test_subject)
            .execute(&db.pool)
            .await;
        

        // Mock token exchange
        let token_response = json!({
            "access_token": "test-access-token",
            "token_type": "Bearer",
            "expires_in": 3600
        });

        Mock::given(method("POST"))
            .and(path("/token"))
            .and(header("content-type", "application/x-www-form-urlencoded"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(token_response)
                .insert_header("content-type", "application/json"))
            .mount(&mock_server)
            .await;

        // Mock user info
        let user_info_response = json!({
            "sub": test_subject,
            "email": test_email,
            "name": "OIDC User",
            "preferred_username": test_username
        });

        Mock::given(method("GET"))
            .and(path("/userinfo"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(user_info_response)
                .insert_header("content-type", "application/json"))
            .mount(&mock_server)
            .await;

        // Add a small delay to make sure everything is set up
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("GET")
                    .uri("/api/auth/oidc/callback?code=test-auth-code&state=test-state")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        
        if status != StatusCode::OK {
            let error_text = String::from_utf8_lossy(&body);
            eprintln!("Response status: {}", status);
            eprintln!("Response body: {}", error_text);
            
            // Also check if we made the expected API calls to the mock server
            eprintln!("Mock server received calls:");
            let received_requests = mock_server.received_requests().await.unwrap();
            for req in received_requests {
                eprintln!("  {} {} - {}", req.method, req.url.path(), String::from_utf8_lossy(&req.body));
            }
            
            // Try to parse as JSON to see if there's a more detailed error message
            if let Ok(error_json) = serde_json::from_slice::<serde_json::Value>(&body) {
                eprintln!("Error JSON: {:#}", error_json);
            }
        }
        
        assert_eq!(status, StatusCode::OK);
        
        let login_response: serde_json::Value = serde_json::from_slice(&body).unwrap();
        
        assert!(login_response["token"].is_string());
        assert_eq!(login_response["user"]["username"], test_username);
        assert_eq!(login_response["user"]["email"], test_email);
    }

    #[tokio::test]
    async fn test_oidc_callback_invalid_token() {
        let (app, mock_server) = create_test_app_with_oidc().await;

        // Mock failed token exchange
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "error": "invalid_grant"
            })))
            .mount(&mock_server)
            .await;

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("GET")
                    .uri("/api/auth/oidc/callback?code=invalid-auth-code")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_oidc_callback_invalid_user_info() {
        let (app, mock_server) = create_test_app_with_oidc().await;
        
        // Generate random identifiers to avoid test interference
        let test_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        let test_username = format!("oidcuser_{}", test_id);
        let test_subject = format!("oidc-user-{}", test_id);
        
        // Clean up any existing test user to ensure test isolation
        let database_url = std::env::var("TEST_DATABASE_URL")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .unwrap_or_else(|_| "postgresql://readur:readur@localhost:5432/readur".to_string());
        let db = readur::db::Database::new(&database_url).await.unwrap();
        
        // Delete any existing user that might conflict
        let _ = sqlx::query("DELETE FROM users WHERE username = $1 OR oidc_subject = $2")
            .bind(&test_username)
            .bind(&test_subject)
            .execute(&db.pool)
            .await;

        // Mock successful token exchange
        let token_response = json!({
            "access_token": "test-access-token",
            "token_type": "Bearer",
            "expires_in": 3600
        });

        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(token_response))
            .mount(&mock_server)
            .await;

        // Mock failed user info
        Mock::given(method("GET"))
            .and(path("/userinfo"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("GET")
                    .uri("/api/auth/oidc/callback?code=test-auth-code")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
