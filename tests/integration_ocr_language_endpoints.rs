use readur2::app::AppState;
use readur2::config::Config;
use readur2::db::Database;
use readur2::ocr::health::OcrHealthChecker;
use axum::http::StatusCode;
use axum_test::TestServer;
use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;
use std::fs;
use uuid::Uuid;

struct TestHarness {
    server: TestServer,
    _temp_dir: TempDir,
    user_id: Uuid,
    token: String,
}

impl TestHarness {
    async fn new() -> Self {
        // Create temporary directory for tessdata
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let tessdata_path = temp_dir.path().join("tessdata");
        fs::create_dir_all(&tessdata_path).expect("Failed to create tessdata directory");
        
        // Create mock language files
        let language_files = vec![
            "eng.traineddata",
            "spa.traineddata",
            "fra.traineddata",
            "deu.traineddata",
            "ita.traineddata",
            "por.traineddata",
        ];
        
        for file in language_files {
            fs::write(tessdata_path.join(file), "mock language data")
                .expect("Failed to create mock language file");
        }
        
        // Set environment variable for tessdata path
        std::env::set_var("TESSDATA_PREFIX", &tessdata_path);
        
        // Create test database
        let config = Config::from_env().expect("Failed to load config");
        let db = Database::new(&config.database_url)
            .await
            .expect("Failed to connect to database");
        
        // Create test user
        let user_id = Uuid::new_v4();
        let username = format!("testuser_{}", user_id);
        let email = format!("{}@test.com", username);
        
        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash) VALUES ($1, $2, $3, $4)"
        )
        .bind(user_id)
        .bind(&username)
        .bind(&email)
        .bind("dummy_hash")
        .execute(&db.pool)
        .await
        .expect("Failed to create test user");
        
        // Create user settings
        sqlx::query(
            "INSERT INTO settings (user_id, ocr_language) VALUES ($1, $2)"
        )
        .bind(user_id)
        .bind("eng")
        .execute(&db.pool)
        .await
        .expect("Failed to create user settings");
        
        // Create app state
        let app_state = Arc::new(AppState {
            db,
            config,
            ocr_health_checker: OcrHealthChecker::new(tessdata_path),
        });
        
        // Create test server
        let app = readur2::app::create_app(app_state);
        let server = TestServer::new(app).expect("Failed to create test server");
        
        // Generate a test token (simplified for testing)
        let token = format!("test_token_{}", user_id);
        
        Self {
            server,
            _temp_dir: temp_dir,
            user_id,
            token,
        }
    }
    
    async fn cleanup(&self) {
        // Clean up test user
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(self.user_id)
            .execute(&self.server.into_inner().extract::<Arc<AppState>>().unwrap().db.pool)
            .await
            .expect("Failed to cleanup test user");
    }
}

#[tokio::test]
async fn test_get_available_languages_success() {
    let harness = TestHarness::new().await;
    
    let response = harness
        .server
        .get("/api/ocr/languages")
        .add_header("Authorization", &format!("Bearer {}", harness.token))
        .await;
    
    assert_eq!(response.status_code(), StatusCode::OK);
    
    let body: serde_json::Value = response.json();
    assert!(body.get("languages").is_some());
    
    let languages = body["languages"].as_array().unwrap();
    assert!(languages.len() >= 6); // We created 6 mock languages
    
    // Check that languages have the expected structure
    for lang in languages {
        assert!(lang.get("code").is_some());
        assert!(lang.get("name").is_some());
    }
    
    // Check that English is included
    let has_english = languages.iter().any(|lang| {
        lang.get("code").unwrap().as_str().unwrap() == "eng"
    });
    assert!(has_english);
    
    harness.cleanup().await;
}

#[tokio::test]
async fn test_get_available_languages_unauthorized() {
    let harness = TestHarness::new().await;
    
    let response = harness
        .server
        .get("/api/ocr/languages")
        .await;
    
    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
    
    harness.cleanup().await;
}

#[tokio::test]
async fn test_get_available_languages_includes_current_user_language() {
    let harness = TestHarness::new().await;
    
    let response = harness
        .server
        .get("/api/ocr/languages")
        .add_header("Authorization", &format!("Bearer {}", harness.token))
        .await;
    
    assert_eq!(response.status_code(), StatusCode::OK);
    
    let body: serde_json::Value = response.json();
    assert_eq!(body["current_user_language"].as_str().unwrap(), "eng");
    
    harness.cleanup().await;
}

#[tokio::test]
async fn test_retry_ocr_with_language_success() {
    let harness = TestHarness::new().await;
    
    // First, create a test document
    let document_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO documents (id, user_id, filename, original_filename, file_size, mime_type, ocr_status, created_at, updated_at) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())"
    )
    .bind(document_id)
    .bind(harness.user_id)
    .bind("test.pdf")
    .bind("test.pdf")
    .bind(1024i64)
    .bind("application/pdf")
    .bind("failed")
    .execute(&harness.server.into_inner().extract::<Arc<AppState>>().unwrap().db.pool)
    .await
    .expect("Failed to create test document");
    
    let retry_request = json!({
        "language": "spa"
    });
    
    let response = harness
        .server
        .post(&format!("/documents/{}/retry-ocr", document_id))
        .add_header("Authorization", &format!("Bearer {}", harness.token))
        .add_header("Content-Type", "application/json")
        .json(&retry_request)
        .await;
    
    assert_eq!(response.status_code(), StatusCode::OK);
    
    let body: serde_json::Value = response.json();
    assert_eq!(body["success"].as_bool().unwrap(), true);
    assert!(body.get("message").is_some());
    
    harness.cleanup().await;
}

#[tokio::test]
async fn test_retry_ocr_without_language_uses_default() {
    let harness = TestHarness::new().await;
    
    // Create a test document
    let document_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO documents (id, user_id, filename, original_filename, file_size, mime_type, ocr_status, created_at, updated_at) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())"
    )
    .bind(document_id)
    .bind(harness.user_id)
    .bind("test.pdf")
    .bind("test.pdf")
    .bind(1024i64)
    .bind("application/pdf")
    .bind("failed")
    .execute(&harness.server.into_inner().extract::<Arc<AppState>>().unwrap().db.pool)
    .await
    .expect("Failed to create test document");
    
    let retry_request = json!({});
    
    let response = harness
        .server
        .post(&format!("/documents/{}/retry-ocr", document_id))
        .add_header("Authorization", &format!("Bearer {}", harness.token))
        .add_header("Content-Type", "application/json")
        .json(&retry_request)
        .await;
    
    assert_eq!(response.status_code(), StatusCode::OK);
    
    let body: serde_json::Value = response.json();
    assert_eq!(body["success"].as_bool().unwrap(), true);
    
    harness.cleanup().await;
}

#[tokio::test]
async fn test_retry_ocr_with_invalid_language() {
    let harness = TestHarness::new().await;
    
    // Create a test document
    let document_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO documents (id, user_id, filename, original_filename, file_size, mime_type, ocr_status, created_at, updated_at) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())"
    )
    .bind(document_id)
    .bind(harness.user_id)
    .bind("test.pdf")
    .bind("test.pdf")
    .bind(1024i64)
    .bind("application/pdf")
    .bind("failed")
    .execute(&harness.server.into_inner().extract::<Arc<AppState>>().unwrap().db.pool)
    .await
    .expect("Failed to create test document");
    
    let retry_request = json!({
        "language": "invalid_lang"
    });
    
    let response = harness
        .server
        .post(&format!("/documents/{}/retry-ocr", document_id))
        .add_header("Authorization", &format!("Bearer {}", harness.token))
        .add_header("Content-Type", "application/json")
        .json(&retry_request)
        .await;
    
    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
    
    let body: serde_json::Value = response.json();
    assert!(body.get("error").is_some());
    
    harness.cleanup().await;
}

#[tokio::test]
async fn test_retry_ocr_nonexistent_document() {
    let harness = TestHarness::new().await;
    
    let nonexistent_id = Uuid::new_v4();
    let retry_request = json!({
        "language": "spa"
    });
    
    let response = harness
        .server
        .post(&format!("/documents/{}/retry-ocr", nonexistent_id))
        .add_header("Authorization", &format!("Bearer {}", harness.token))
        .add_header("Content-Type", "application/json")
        .json(&retry_request)
        .await;
    
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    
    harness.cleanup().await;
}

#[tokio::test]
async fn test_retry_ocr_unauthorized_user() {
    let harness = TestHarness::new().await;
    
    // Create a document owned by a different user
    let other_user_id = Uuid::new_v4();
    let document_id = Uuid::new_v4();
    
    sqlx::query(
        "INSERT INTO users (id, username, email, password_hash) VALUES ($1, $2, $3, $4)"
    )
    .bind(other_user_id)
    .bind("otheruser")
    .bind("other@test.com")
    .bind("dummy_hash")
    .execute(&harness.server.into_inner().extract::<Arc<AppState>>().unwrap().db.pool)
    .await
    .expect("Failed to create other user");
    
    sqlx::query(
        "INSERT INTO documents (id, user_id, filename, original_filename, file_size, mime_type, ocr_status, created_at, updated_at) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())"
    )
    .bind(document_id)
    .bind(other_user_id)
    .bind("test.pdf")
    .bind("test.pdf")
    .bind(1024i64)
    .bind("application/pdf")
    .bind("failed")
    .execute(&harness.server.into_inner().extract::<Arc<AppState>>().unwrap().db.pool)
    .await
    .expect("Failed to create test document");
    
    let retry_request = json!({
        "language": "spa"
    });
    
    let response = harness
        .server
        .post(&format!("/documents/{}/retry-ocr", document_id))
        .add_header("Authorization", &format!("Bearer {}", harness.token))
        .add_header("Content-Type", "application/json")
        .json(&retry_request)
        .await;
    
    assert_eq!(response.status_code(), StatusCode::FORBIDDEN);
    
    // Cleanup other user
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(other_user_id)
        .execute(&harness.server.into_inner().extract::<Arc<AppState>>().unwrap().db.pool)
        .await
        .expect("Failed to cleanup other user");
    
    harness.cleanup().await;
}

#[tokio::test]
async fn test_document_upload_with_language_validation() {
    let harness = TestHarness::new().await;
    
    // Create a multipart form with a document and language
    let file_content = b"Mock PDF content";
    let form = reqwest::multipart::Form::new()
        .part("file", reqwest::multipart::Part::bytes(file_content.to_vec())
            .file_name("test.pdf")
            .mime_str("application/pdf").unwrap())
        .part("language", reqwest::multipart::Part::text("spa"));
    
    let response = harness
        .server
        .post("/documents")
        .add_header("Authorization", &format!("Bearer {}", harness.token))
        .multipart(form)
        .await;
    
    // Should succeed with valid language
    assert_eq!(response.status_code(), StatusCode::OK);
    
    harness.cleanup().await;
}

#[tokio::test]
async fn test_document_upload_with_invalid_language() {
    let harness = TestHarness::new().await;
    
    // Create a multipart form with invalid language
    let file_content = b"Mock PDF content";
    let form = reqwest::multipart::Form::new()
        .part("file", reqwest::multipart::Part::bytes(file_content.to_vec())
            .file_name("test.pdf")
            .mime_str("application/pdf").unwrap())
        .part("language", reqwest::multipart::Part::text("invalid_lang"));
    
    let response = harness
        .server
        .post("/documents")
        .add_header("Authorization", &format!("Bearer {}", harness.token))
        .multipart(form)
        .await;
    
    // Should fail with invalid language
    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
    
    harness.cleanup().await;
}

#[tokio::test]
async fn test_settings_update_with_ocr_language() {
    let harness = TestHarness::new().await;
    
    let settings_update = json!({
        "ocrLanguage": "fra",
        "concurrentOcrJobs": 2,
        "ocrTimeoutSeconds": 300
    });
    
    let response = harness
        .server
        .put("/settings")
        .add_header("Authorization", &format!("Bearer {}", harness.token))
        .add_header("Content-Type", "application/json")
        .json(&settings_update)
        .await;
    
    assert_eq!(response.status_code(), StatusCode::OK);
    
    // Verify the setting was updated
    let get_response = harness
        .server
        .get("/settings")
        .add_header("Authorization", &format!("Bearer {}", harness.token))
        .await;
    
    assert_eq!(get_response.status_code(), StatusCode::OK);
    
    let body: serde_json::Value = get_response.json();
    assert_eq!(body["ocrLanguage"].as_str().unwrap(), "fra");
    
    harness.cleanup().await;
}

#[tokio::test]
async fn test_settings_update_with_invalid_ocr_language() {
    let harness = TestHarness::new().await;
    
    let settings_update = json!({
        "ocrLanguage": "invalid_lang",
        "concurrentOcrJobs": 2
    });
    
    let response = harness
        .server
        .put("/settings")
        .add_header("Authorization", &format!("Bearer {}", harness.token))
        .add_header("Content-Type", "application/json")
        .json(&settings_update)
        .await;
    
    // Should fail with invalid language
    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
    
    harness.cleanup().await;
}

#[tokio::test]
async fn test_ocr_health_endpoint() {
    let harness = TestHarness::new().await;
    
    let response = harness
        .server
        .get("/api/ocr/health")
        .add_header("Authorization", &format!("Bearer {}", harness.token))
        .await;
    
    assert_eq!(response.status_code(), StatusCode::OK);
    
    let body: serde_json::Value = response.json();
    assert!(body.get("status").is_some());
    assert!(body.get("available_languages").is_some());
    
    harness.cleanup().await;
}

#[tokio::test]
async fn test_concurrent_language_requests() {
    let harness = TestHarness::new().await;
    
    // Make multiple concurrent requests to the languages endpoint
    let mut handles = vec![];
    
    for _ in 0..5 {
        let server_clone = harness.server.clone();
        let token_clone = harness.token.clone();
        let handle = tokio::spawn(async move {
            server_clone
                .get("/api/ocr/languages")
                .add_header("Authorization", &format!("Bearer {}", token_clone))
                .await
        });
        handles.push(handle);
    }
    
    // All requests should succeed
    for handle in handles {
        let response = handle.await.expect("Task panicked");
        assert_eq!(response.status_code(), StatusCode::OK);
    }
    
    harness.cleanup().await;
}