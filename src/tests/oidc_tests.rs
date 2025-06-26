#[cfg(test)]
mod tests {
    use crate::models::{AuthProvider, CreateUser, UserRole};
    use super::super::helpers::{create_test_app};
    use axum::http::StatusCode;
    use serde_json::json;
    use tower::util::ServiceExt;
    use wiremock::{matchers::{method, path, query_param}, Mock, MockServer, ResponseTemplate};
    use std::sync::Arc;
    use crate::{AppState, oidc::OidcClient};

    async fn create_test_app_with_oidc() -> (axum::Router, testcontainers::ContainerAsync<testcontainers_modules::postgres::Postgres>, MockServer) {
        let (mut app, container) = create_test_app().await;
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

        // Update the app state to include OIDC client
        let config = crate::config::Config {
            database_url: "postgresql://test:test@localhost/test".to_string(),
            server_address: "127.0.0.1:0".to_string(),
            jwt_secret: "test-secret".to_string(),
            upload_path: "./test-uploads".to_string(),
            watch_folder: "./test-watch".to_string(),
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
        };

        let oidc_client = OidcClient::new(&config).await.ok().map(Arc::new);
        
        // We need to extract the state from the existing app and recreate it
        // This is a bit hacky, but necessary for testing
        app = axum::Router::new()
            .nest("/api/auth", crate::routes::auth::router())
            .with_state(Arc::new(AppState {
                db: crate::db::Database::new(&format!("postgresql://test:test@localhost:{}/test", 
                    container.get_host_port_ipv4(5432).await.unwrap())).await.unwrap(),
                config,
                webdav_scheduler: None,
                source_scheduler: None,
                queue_service: Arc::new(crate::ocr_queue::OcrQueueService::new(
                    crate::db::Database::new(&format!("postgresql://test:test@localhost:{}/test", 
                        container.get_host_port_ipv4(5432).await.unwrap())).await.unwrap(),
                    sqlx::PgPool::connect(&format!("postgresql://test:test@localhost:{}/test", 
                        container.get_host_port_ipv4(5432).await.unwrap())).await.unwrap(),
                    2
                )),
                oidc_client,
            }));

        (app, container, mock_server)
    }

    #[tokio::test]
    async fn test_oidc_login_redirect() {
        let (app, _container, _mock_server) = create_test_app_with_oidc().await;

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
        let (app, _container) = create_test_app().await; // Regular app without OIDC

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
        let (app, _container, _mock_server) = create_test_app_with_oidc().await;

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
        let (app, _container, _mock_server) = create_test_app_with_oidc().await;

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
        let (app, _container, mock_server) = create_test_app_with_oidc().await;

        // Mock token exchange
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

        // Mock user info
        let user_info_response = json!({
            "sub": "oidc-user-123",
            "email": "oidc@example.com",
            "name": "OIDC User",
            "preferred_username": "oidcuser"
        });

        Mock::given(method("GET"))
            .and(path("/userinfo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(user_info_response))
            .mount(&mock_server)
            .await;

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

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let login_response: serde_json::Value = serde_json::from_slice(&body).unwrap();
        
        assert!(login_response["token"].is_string());
        assert_eq!(login_response["user"]["username"], "oidcuser");
        assert_eq!(login_response["user"]["email"], "oidc@example.com");
    }

    #[tokio::test]
    async fn test_oidc_callback_invalid_token() {
        let (app, _container, mock_server) = create_test_app_with_oidc().await;

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
        let (app, _container, mock_server) = create_test_app_with_oidc().await;

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