use crate::{AppState, models::UserResponse};
use axum::Router;
use serde_json::json;
use std::sync::Arc;
use testcontainers::{core::WaitFor, runners::AsyncRunner, ContainerAsync, GenericImage, ImageExt};
use testcontainers_modules::postgres::Postgres;
use tower::util::ServiceExt;

pub async fn create_test_app() -> (Router, ContainerAsync<Postgres>) {
    let postgres_image = Postgres::default()
        .with_env_var(("POSTGRES_USER", "test"))
        .with_env_var(("POSTGRES_PASSWORD", "test"))
        .with_env_var(("POSTGRES_DB", "test"));
    
    let container = postgres_image.start().await.expect("Failed to start postgres container");
    let port = container.get_host_port_ipv4(5432).await.expect("Failed to get postgres port");
    
    let database_url = format!("postgresql://test:test@localhost:{}/test", port);
    let db = crate::db::Database::new(&database_url).await.unwrap();
    db.migrate().await.unwrap();
    
    let config = crate::config::Config {
        database_url: database_url.clone(),
        server_address: "127.0.0.1:0".to_string(),
        jwt_secret: "test-secret".to_string(),
        upload_path: "./test-uploads".to_string(),
        watch_folder: "./test-watch".to_string(),
        allowed_file_types: vec!["pdf".to_string(), "txt".to_string(), "png".to_string()],
        watch_interval_seconds: Some(30),
        file_stability_check_ms: Some(500),
        max_file_age_hours: None,
        
        // OCR Configuration
        ocr_language: "eng".to_string(),
        concurrent_ocr_jobs: 2, // Lower for tests
        ocr_timeout_seconds: 60, // Shorter for tests
        max_file_size_mb: 10, // Smaller for tests
        
        // Performance
        memory_limit_mb: 256, // Lower for tests
        cpu_priority: "normal".to_string(),
    };
    
    let state = Arc::new(AppState { 
        db, 
        config,
        webdav_scheduler: None,
        source_scheduler: None,
    });
    
    let app = Router::new()
        .nest("/api/auth", crate::routes::auth::router())
        .nest("/api/documents", crate::routes::documents::router())
        .nest("/api/search", crate::routes::search::router())
        .nest("/api/settings", crate::routes::settings::router())
        .nest("/api/users", crate::routes::users::router())
        .with_state(state);
    
    (app, container)
}

pub async fn create_test_user(app: &Router) -> UserResponse {
    let user_data = json!({
        "username": "testuser",
        "email": "test@example.com",
        "password": "password123"
    });
    
    let response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(serde_json::to_vec(&user_data).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body).unwrap()
}

pub async fn login_user(app: &Router, username: &str, password: &str) -> String {
    let login_data = json!({
        "username": username,
        "password": password
    });
    
    let response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(serde_json::to_vec(&login_data).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let login_response: serde_json::Value = serde_json::from_slice(&body).unwrap();
    login_response["token"].as_str().unwrap().to_string()
}