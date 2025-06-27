//! Test utilities for loading and working with test images and data
//! 
//! This module provides utilities for loading test images from the tests/test_images/
//! directory and working with them in unit and integration tests.

use std::path::Path;

#[cfg(any(test, feature = "test-utils"))]
use std::sync::Arc;
#[cfg(any(test, feature = "test-utils"))]
use crate::{AppState, models::UserResponse};
#[cfg(any(test, feature = "test-utils"))]
use axum::Router;
#[cfg(any(test, feature = "test-utils"))]
use serde_json::json;
#[cfg(any(test, feature = "test-utils"))]
use testcontainers::{runners::AsyncRunner, ContainerAsync, ImageExt};
#[cfg(any(test, feature = "test-utils"))]
use testcontainers_modules::postgres::Postgres;
#[cfg(any(test, feature = "test-utils"))]
use tower::util::ServiceExt;
#[cfg(any(test, feature = "test-utils"))]
use uuid;

/// Test image information with expected OCR content
#[derive(Debug, Clone)]
pub struct TestImage {
    pub filename: &'static str,
    pub path: String,
    pub mime_type: &'static str,
    pub expected_content: &'static str,
}

impl TestImage {
    pub fn new(filename: &'static str, mime_type: &'static str, expected_content: &'static str) -> Self {
        Self {
            filename,
            path: format!("tests/test_images/{}", filename),
            mime_type,
            expected_content,
        }
    }
    
    pub fn exists(&self) -> bool {
        Path::new(&self.path).exists()
    }
    
    pub async fn load_data(&self) -> Result<Vec<u8>, std::io::Error> {
        tokio::fs::read(&self.path).await
    }
}

/// Get all available test images with their expected OCR content
pub fn get_test_images() -> Vec<TestImage> {
    vec![
        TestImage::new("test1.png", "image/png", "Test 1\nThis is some text from text 1"),
        TestImage::new("test2.jpg", "image/jpeg", "Test 2\nThis is some text from text 2"),
        TestImage::new("test3.jpeg", "image/jpeg", "Test 3\nThis is some text from text 3"),
        TestImage::new("test4.png", "image/png", "Test 4\nThis is some text from text 4"),
        TestImage::new("test5.jpg", "image/jpeg", "Test 5\nThis is some text from text 5"),
        TestImage::new("test6.jpeg", "image/jpeg", "Test 6\nThis is some text from text 6"),
        TestImage::new("test7.png", "image/png", "Test 7\nThis is some text from text 7"),
        TestImage::new("test8.jpeg", "image/jpeg", "Test 8\nThis is some text from text 8"),
        TestImage::new("test9.png", "image/png", "Test 9\nThis is some text from text 9"),
    ]
}

/// Get a specific test image by number (1-9)
pub fn get_test_image(number: u8) -> Option<TestImage> {
    if number < 1 || number > 9 {
        return None;
    }
    
    get_test_images().into_iter().nth((number - 1) as usize)
}

/// Load test image data by filename
pub async fn load_test_image(filename: &str) -> Result<Vec<u8>, std::io::Error> {
    let path = format!("tests/test_images/{}", filename);
    tokio::fs::read(path).await
}

/// Check if test images directory exists and is accessible
pub fn test_images_available() -> bool {
    Path::new("tests/test_images").exists()
}

/// Get available test images (only those that exist on filesystem)
pub fn get_available_test_images() -> Vec<TestImage> {
    get_test_images()
        .into_iter()
        .filter(|img| img.exists())
        .collect()
}

/// Skip test macro for conditional testing based on test image availability
#[macro_export]
macro_rules! skip_if_no_test_images {
    () => {
        if !crate::test_utils::test_images_available() {
            println!("Skipping test: test images directory not available");
            return;
        }
    };
}

/// Skip test macro for specific test image
#[macro_export]
macro_rules! skip_if_test_image_missing {
    ($image:expr) => {
        if !$image.exists() {
            println!("Skipping test: {} not found", $image.filename);
            return;
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_paths_are_valid() {
        let images = get_test_images();
        assert_eq!(images.len(), 9);
        
        for (i, image) in images.iter().enumerate() {
            assert_eq!(image.filename, format!("test{}.{}", i + 1, 
                if image.mime_type == "image/png" { "png" } 
                else if image.filename.ends_with(".jpg") { "jpg" }
                else { "jpeg" }
            ));
            assert!(image.expected_content.starts_with(&format!("Test {}", i + 1)));
        }
    }

    #[test]
    fn test_get_specific_image() {
        let image1 = get_test_image(1).unwrap();
        assert_eq!(image1.filename, "test1.png");
        assert_eq!(image1.mime_type, "image/png");
        assert!(image1.expected_content.contains("Test 1"));

        let image5 = get_test_image(5).unwrap();
        assert_eq!(image5.filename, "test5.jpg");
        assert_eq!(image5.mime_type, "image/jpeg");
        assert!(image5.expected_content.contains("Test 5"));

        // Invalid numbers should return None
        assert!(get_test_image(0).is_none());
        assert!(get_test_image(10).is_none());
    }
}

/// Helper functions for integration tests
#[cfg(any(test, feature = "test-utils"))]
pub async fn create_test_app() -> (Router, ContainerAsync<Postgres>) {
    let postgres_image = Postgres::default()
        .with_env_var("POSTGRES_USER", "test")
        .with_env_var("POSTGRES_PASSWORD", "test")
        .with_env_var("POSTGRES_DB", "test");
    
    let container = postgres_image.start().await.expect("Failed to start postgres container");
    let port = container.get_host_port_ipv4(5432).await.expect("Failed to get postgres port");
    
    // Use TEST_DATABASE_URL if available, otherwise use the container
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| format!("postgresql://test:test@localhost:{}/test", port));
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
        
        // OIDC Configuration
        oidc_enabled: false,
        oidc_client_id: None,
        oidc_client_secret: None,
        oidc_issuer_url: None,
        oidc_redirect_uri: None,
    };
    
    let queue_service = Arc::new(crate::ocr_queue::OcrQueueService::new(db.clone(), db.pool.clone(), 2));
    
    let state = Arc::new(AppState { 
        db, 
        config,
        webdav_scheduler: None,
        source_scheduler: None,
        queue_service,
        oidc_client: None,
    });
    
    let app = Router::new()
        .nest("/api/auth", crate::routes::auth::router())
        .nest("/api/documents", crate::routes::documents::router())
        .nest("/api/search", crate::routes::search::router())
        .nest("/api/settings", crate::routes::settings::router())
        .nest("/api/users", crate::routes::users::router())
        .nest("/api/ignored-files", crate::routes::ignored_files::ignored_files_routes())
        .with_state(state);
    
    (app, container)
}

#[cfg(any(test, feature = "test-utils"))]
pub async fn create_test_user(app: &Router) -> UserResponse {
    // Generate random identifiers to avoid test interference
    let test_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let test_username = format!("testuser_{}", test_id);
    let test_email = format!("test_{}@example.com", test_id);
    
    let user_data = json!({
        "username": test_username,
        "email": test_email,
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

#[cfg(any(test, feature = "test-utils"))]
pub async fn create_admin_user(app: &Router) -> UserResponse {
    // Generate random identifiers to avoid test interference
    let test_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let admin_username = format!("adminuser_{}", test_id);
    let admin_email = format!("admin_{}@example.com", test_id);
    
    let admin_data = json!({
        "username": admin_username,
        "email": admin_email,
        "password": "adminpass123",
        "role": "admin"
    });
    
    let response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(serde_json::to_vec(&admin_data).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body).unwrap()
}

#[cfg(any(test, feature = "test-utils"))]
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