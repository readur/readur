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

/// Unified test context that eliminates duplication across integration tests
#[cfg(any(test, feature = "test-utils"))]
pub struct TestContext {
    pub app: Router,
    pub container: ContainerAsync<Postgres>,
    pub state: Arc<AppState>,
}

#[cfg(any(test, feature = "test-utils"))]
impl TestContext {
    /// Create a new test context with default test configuration
    pub async fn new() -> Self {
        Self::with_config(TestConfigBuilder::default()).await
    }
    
    /// Create a test context with custom configuration
    pub async fn with_config(config_builder: TestConfigBuilder) -> Self {
        let postgres_image = Postgres::default()
            .with_env_var("POSTGRES_USER", "test")
            .with_env_var("POSTGRES_PASSWORD", "test")
            .with_env_var("POSTGRES_DB", "test");
        
        let container = postgres_image.start().await.expect("Failed to start postgres container");
        let port = container.get_host_port_ipv4(5432).await.expect("Failed to get postgres port");
        
        let database_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| format!("postgresql://test:test@localhost:{}/test", port));
        let db = crate::db::Database::new(&database_url).await.unwrap();
        db.migrate().await.unwrap();
        
        let config = config_builder.build(database_url);
        let queue_service = Arc::new(crate::ocr::queue::OcrQueueService::new(db.clone(), db.pool.clone(), 2));
        
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
            .with_state(state.clone());
        
        Self { app, container, state }
    }
    
    /// Get the app router for making requests
    pub fn app(&self) -> &Router {
        &self.app
    }
    
    /// Get the application state
    pub fn state(&self) -> &Arc<AppState> {
        &self.state
    }
}

/// Builder pattern for test configuration to eliminate config duplication
#[cfg(any(test, feature = "test-utils"))]
pub struct TestConfigBuilder {
    upload_path: String,
    watch_folder: String,
    jwt_secret: String,
    concurrent_ocr_jobs: usize,
    ocr_timeout_seconds: u64,
    max_file_size_mb: u64,
    memory_limit_mb: u64,
    oidc_enabled: bool,
}

#[cfg(any(test, feature = "test-utils"))]
impl Default for TestConfigBuilder {
    fn default() -> Self {
        Self {
            upload_path: "./test-uploads".to_string(),
            watch_folder: "./test-watch".to_string(),
            jwt_secret: "test-secret".to_string(),
            concurrent_ocr_jobs: 2,
            ocr_timeout_seconds: 60,
            max_file_size_mb: 10,
            memory_limit_mb: 256,
            oidc_enabled: false,
        }
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl TestConfigBuilder {
    pub fn with_upload_path(mut self, path: &str) -> Self {
        self.upload_path = path.to_string();
        self
    }
    
    pub fn with_watch_folder(mut self, folder: &str) -> Self {
        self.watch_folder = folder.to_string();
        self
    }
    
    pub fn with_concurrent_ocr_jobs(mut self, jobs: usize) -> Self {
        self.concurrent_ocr_jobs = jobs;
        self
    }
    
    pub fn with_oidc_enabled(mut self, enabled: bool) -> Self {
        self.oidc_enabled = enabled;
        self
    }
    
    fn build(self, database_url: String) -> crate::config::Config {
        crate::config::Config {
            database_url,
            server_address: "127.0.0.1:0".to_string(),
            jwt_secret: self.jwt_secret,
            upload_path: self.upload_path,
            watch_folder: self.watch_folder,
            allowed_file_types: vec!["pdf".to_string(), "txt".to_string(), "png".to_string()],
            watch_interval_seconds: Some(30),
            file_stability_check_ms: Some(500),
            max_file_age_hours: None,
            
            // OCR Configuration
            ocr_language: "eng".to_string(),
            concurrent_ocr_jobs: self.concurrent_ocr_jobs,
            ocr_timeout_seconds: self.ocr_timeout_seconds,
            max_file_size_mb: self.max_file_size_mb,
            
            // Performance
            memory_limit_mb: self.memory_limit_mb as usize,
            cpu_priority: "normal".to_string(),
            
            // OIDC Configuration
            oidc_enabled: self.oidc_enabled,
            oidc_client_id: None,
            oidc_client_secret: None,
            oidc_issuer_url: None,
            oidc_redirect_uri: None,
        }
    }
}

/// Legacy function for backward compatibility - will be deprecated
#[cfg(any(test, feature = "test-utils"))]
pub async fn create_test_app() -> (Router, ContainerAsync<Postgres>) {
    let ctx = TestContext::new().await;
    (ctx.app, ctx.container)
}

/// Unified test authentication helper that replaces TestClient/AdminTestClient patterns
#[cfg(any(test, feature = "test-utils"))]
pub struct TestAuthHelper {
    app: Router,
}

#[cfg(any(test, feature = "test-utils"))]
impl TestAuthHelper {
    pub fn new(app: Router) -> Self {
        Self { app }
    }
    
    /// Create a regular test user with unique credentials
    pub async fn create_test_user(&self) -> TestUser {
        let test_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .to_string()[..8]
            .to_string();
        let username = format!("testuser_{}", test_id);
        let email = format!("test_{}@example.com", test_id);
        let password = "password123";
        
        let user_data = json!({
            "username": username,
            "email": email,
            "password": password
        });
        
        let response = self.make_request("POST", "/api/auth/register", Some(user_data), None).await;
        let user_response: UserResponse = serde_json::from_slice(&response).unwrap();
        
        TestUser {
            user_response,
            username,
            password: password.to_string(),
            token: None,
        }
    }
    
    /// Create an admin test user with unique credentials
    pub async fn create_admin_user(&self) -> TestUser {
        let test_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .to_string()[..8]
            .to_string();
        let username = format!("adminuser_{}", test_id);
        let email = format!("admin_{}@example.com", test_id);
        let password = "adminpass123";
        
        let admin_data = json!({
            "username": username,
            "email": email,
            "password": password,
            "role": "admin"
        });
        
        let response = self.make_request("POST", "/api/auth/register", Some(admin_data), None).await;
        let user_response: UserResponse = serde_json::from_slice(&response).unwrap();
        
        TestUser {
            user_response,
            username,
            password: password.to_string(),
            token: None,
        }
    }
    
    /// Login a user and return their authentication token
    pub async fn login_user(&self, username: &str, password: &str) -> String {
        let login_data = json!({
            "username": username,
            "password": password
        });
        
        let response = self.make_request("POST", "/api/auth/login", Some(login_data), None).await;
        let login_response: serde_json::Value = serde_json::from_slice(&response).unwrap();
        login_response["token"].as_str().unwrap().to_string()
    }
    
    /// Make an authenticated HTTP request
    pub async fn make_authenticated_request(&self, method: &str, uri: &str, body: Option<serde_json::Value>, token: &str) -> Vec<u8> {
        self.make_request(method, uri, body, Some(token)).await
    }
    
    /// Make an HTTP request (internal helper)
    async fn make_request(&self, method: &str, uri: &str, body: Option<serde_json::Value>, token: Option<&str>) -> Vec<u8> {
        let mut builder = axum::http::Request::builder()
            .method(method)
            .uri(uri)
            .header("Content-Type", "application/json");
        
        if let Some(token) = token {
            builder = builder.header("Authorization", format!("Bearer {}", token));
        }
        
        let request_body = if let Some(body) = body {
            axum::body::Body::from(serde_json::to_vec(&body).unwrap())
        } else {
            axum::body::Body::empty()
        };
        
        let response = self.app
            .clone()
            .oneshot(builder.body(request_body).unwrap())
            .await
            .unwrap();
        
        axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec()
    }
}

/// Test user with authentication capabilities
#[cfg(any(test, feature = "test-utils"))]
pub struct TestUser {
    pub user_response: UserResponse,
    pub username: String,
    pub password: String,
    pub token: Option<String>,
}

#[cfg(any(test, feature = "test-utils"))]
impl TestUser {
    /// Login this user and store the authentication token
    pub async fn login(&mut self, auth_helper: &TestAuthHelper) -> Result<&str, Box<dyn std::error::Error>> {
        let token = auth_helper.login_user(&self.username, &self.password).await;
        self.token = Some(token);
        Ok(self.token.as_ref().unwrap())
    }
    
    /// Make an authenticated request as this user
    pub async fn make_request(&self, auth_helper: &TestAuthHelper, method: &str, uri: &str, body: Option<serde_json::Value>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let token = self.token.as_ref().ok_or("User not logged in")?;
        Ok(auth_helper.make_authenticated_request(method, uri, body, token).await)
    }
    
    /// Get user ID
    pub fn id(&self) -> String {
        self.user_response.id.to_string()
    }
    
    /// Check if user is admin
    pub fn is_admin(&self) -> bool {
        matches!(self.user_response.role, crate::models::UserRole::Admin)
    }
}

/// Legacy functions for backward compatibility - will be deprecated
#[cfg(any(test, feature = "test-utils"))]
pub async fn create_test_user(app: &Router) -> UserResponse {
    let auth_helper = TestAuthHelper::new(app.clone());
    let test_user = auth_helper.create_test_user().await;
    test_user.user_response
}

#[cfg(any(test, feature = "test-utils"))]
pub async fn create_admin_user(app: &Router) -> UserResponse {
    let auth_helper = TestAuthHelper::new(app.clone());
    let admin_user = auth_helper.create_admin_user().await;
    admin_user.user_response
}

#[cfg(any(test, feature = "test-utils"))]
pub async fn login_user(app: &Router, username: &str, password: &str) -> String {
    let auth_helper = TestAuthHelper::new(app.clone());
    auth_helper.login_user(username, password).await
}