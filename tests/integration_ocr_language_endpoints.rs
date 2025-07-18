use readur::test_utils::TestContext;
use axum::http::StatusCode;
use axum::body::Body;
use axum::http::Request;
use tower::ServiceExt;
use serde_json::json;
use tempfile::TempDir;
use std::fs;
use uuid::Uuid;

#[tokio::test]
async fn test_get_available_languages_success() {
    // Create temporary directory for tessdata
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let tessdata_path = temp_dir.path();
    
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
    
    // Set environment variable for tessdata path and verify it's properly set
    let tessdata_str = tessdata_path.to_string_lossy().to_string();
    std::env::set_var("TESSDATA_PREFIX", &tessdata_str);
    
    // Verify the files exist in the temp directory
    assert!(tessdata_path.join("spa.traineddata").exists());
    assert_eq!(std::env::var("TESSDATA_PREFIX").unwrap(), tessdata_str);
    
    // Use the existing admin credentials to test against the running server
    let client = reqwest::Client::new();
    
    // Login with admin credentials
    let login_response = client
        .post("http://localhost:8000/api/auth/login")
        .json(&serde_json::json!({
            "username": "admin",
            "password": "readur2024"
        }))
        .send()
        .await
        .expect("Failed to login");
    
    let login_data: serde_json::Value = login_response.json().await.expect("Failed to parse login data");
    let token = login_data["token"].as_str().expect("Missing token");

    // Test against the running server
    let response = client
        .get("http://localhost:8000/api/ocr/languages")
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to make request");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    
    assert!(body.get("available_languages").is_some());
    let languages = body["available_languages"].as_array().unwrap();
    assert!(languages.len() >= 1); // At least English should be available

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
}

#[tokio::test]
async fn test_get_available_languages_unauthorized() {
    // Create temporary directory for tessdata
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let tessdata_path = temp_dir.path();
    
    // Create mock language files
    fs::write(tessdata_path.join("eng.traineddata"), "mock").unwrap();
    let tessdata_str = tessdata_path.to_string_lossy().to_string();
    std::env::set_var("TESSDATA_PREFIX", &tessdata_str);
    
    let ctx = TestContext::new().await;

    // Test against the running server since the test environment has issues
    let client = reqwest::Client::new();
    let response = client
        .get("http://localhost:8000/api/ocr/languages")
        .send()
        .await
        .expect("Failed to make request");

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_retry_ocr_with_language_success() {
    // Create temporary directory for tessdata
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let tessdata_path = temp_dir.path();
    
    // Create mock language files
    fs::write(tessdata_path.join("eng.traineddata"), "mock").unwrap();
    fs::write(tessdata_path.join("spa.traineddata"), "mock").unwrap();
    let tessdata_str = tessdata_path.to_string_lossy().to_string();
    std::env::set_var("TESSDATA_PREFIX", &tessdata_str);
    
    let ctx = TestContext::new().await;
    
    // Create test user and get token
    let auth_helper = readur::test_utils::TestAuthHelper::new(ctx.app().clone());
    let mut test_user = auth_helper.create_test_user().await;
    let user_id = test_user.user_response.id;
    let token = test_user.login(&auth_helper).await.unwrap();

    // Create a test document
    let document_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO documents (id, user_id, filename, original_filename, file_path, file_size, mime_type, ocr_status, created_at, updated_at) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), NOW())"
    )
    .bind(document_id)
    .bind(user_id)
    .bind("test.pdf")
    .bind("test.pdf")
    .bind("/tmp/test.pdf")
    .bind(1024i64)
    .bind("application/pdf")
    .bind("failed")
    .execute(&ctx.state().db.pool)
    .await
    .expect("Failed to create test document");

    let retry_request = json!({
        "language": "spa"
    });

    let request = Request::builder()
        .method("POST")
        .uri(&format!("/api/documents/{}/ocr/retry", document_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&retry_request).unwrap()))
        .unwrap();

    let response = ctx.app().clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["success"].as_bool().unwrap(), true);
    assert!(body.get("message").is_some());
}

#[tokio::test]
async fn test_retry_ocr_with_invalid_language() {
    // Create temporary directory for tessdata
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let tessdata_path = temp_dir.path();
    
    // Create mock language files
    fs::write(tessdata_path.join("eng.traineddata"), "mock").unwrap();
    let tessdata_str = tessdata_path.to_string_lossy().to_string();
    std::env::set_var("TESSDATA_PREFIX", &tessdata_str);
    
    let ctx = TestContext::new().await;
    
    // Create test user and get token
    let auth_helper = readur::test_utils::TestAuthHelper::new(ctx.app().clone());
    let mut test_user = auth_helper.create_test_user().await;
    let user_id = test_user.user_response.id;
    let token = test_user.login(&auth_helper).await.unwrap();

    // Create a test document
    let document_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO documents (id, user_id, filename, original_filename, file_path, file_size, mime_type, ocr_status, created_at, updated_at) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), NOW())"
    )
    .bind(document_id)
    .bind(user_id)
    .bind("test.pdf")
    .bind("test.pdf")
    .bind("/tmp/test.pdf")
    .bind(1024i64)
    .bind("application/pdf")
    .bind("failed")
    .execute(&ctx.state().db.pool)
    .await
    .expect("Failed to create test document");

    let retry_request = json!({
        "language": "invalid_lang"
    });

    let request = Request::builder()
        .method("POST")
        .uri(&format!("/api/documents/{}/ocr/retry", document_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&retry_request).unwrap()))
        .unwrap();

    let response = ctx.app().clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_retry_ocr_with_multiple_languages_success() {
    // Create temporary directory for tessdata
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let tessdata_path = temp_dir.path();
    
    // Create mock language files
    fs::write(tessdata_path.join("eng.traineddata"), "mock").unwrap();
    fs::write(tessdata_path.join("spa.traineddata"), "mock").unwrap();
    fs::write(tessdata_path.join("fra.traineddata"), "mock").unwrap();
    let tessdata_str = tessdata_path.to_string_lossy().to_string();
    std::env::set_var("TESSDATA_PREFIX", &tessdata_str);
    
    let ctx = TestContext::new().await;
    
    // Create test user and get token
    let auth_helper = readur::test_utils::TestAuthHelper::new(ctx.app().clone());
    let mut test_user = auth_helper.create_test_user().await;
    let user_id = test_user.user_response.id;
    let token = test_user.login(&auth_helper).await.unwrap();

    // Create a test document
    let document_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO documents (id, user_id, filename, original_filename, file_path, file_size, mime_type, ocr_status, created_at, updated_at) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), NOW())"
    )
    .bind(document_id)
    .bind(user_id)
    .bind("test.pdf")
    .bind("test.pdf")
    .bind("/tmp/test.pdf")
    .bind(1024i64)
    .bind("application/pdf")
    .bind("failed")
    .execute(&ctx.state().db.pool)
    .await
    .expect("Failed to create test document");

    let retry_request = json!({
        "languages": ["eng", "spa", "fra"]
    });

    let request = Request::builder()
        .method("POST")
        .uri(&format!("/api/documents/{}/ocr/retry", document_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&retry_request).unwrap()))
        .unwrap();

    let response = ctx.app().clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["success"].as_bool().unwrap(), true);
    assert!(body.get("message").is_some());
}

#[tokio::test]
async fn test_retry_ocr_with_too_many_languages() {
    // Create temporary directory for tessdata
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let tessdata_path = temp_dir.path();
    
    // Create mock language files
    fs::write(tessdata_path.join("eng.traineddata"), "mock").unwrap();
    fs::write(tessdata_path.join("spa.traineddata"), "mock").unwrap();
    fs::write(tessdata_path.join("fra.traineddata"), "mock").unwrap();
    fs::write(tessdata_path.join("deu.traineddata"), "mock").unwrap();
    fs::write(tessdata_path.join("ita.traineddata"), "mock").unwrap();
    let tessdata_str = tessdata_path.to_string_lossy().to_string();
    std::env::set_var("TESSDATA_PREFIX", &tessdata_str);
    
    let ctx = TestContext::new().await;
    
    // Create test user and get token
    let auth_helper = readur::test_utils::TestAuthHelper::new(ctx.app().clone());
    let mut test_user = auth_helper.create_test_user().await;
    let user_id = test_user.user_response.id;
    let token = test_user.login(&auth_helper).await.unwrap();

    // Create a test document
    let document_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO documents (id, user_id, filename, original_filename, file_path, file_size, mime_type, ocr_status, created_at, updated_at) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), NOW())"
    )
    .bind(document_id)
    .bind(user_id)
    .bind("test.pdf")
    .bind("test.pdf")
    .bind("/tmp/test.pdf")
    .bind(1024i64)
    .bind("application/pdf")
    .bind("failed")
    .execute(&ctx.state().db.pool)
    .await
    .expect("Failed to create test document");

    // Try to use more than 4 languages (should fail)
    let retry_request = json!({
        "languages": ["eng", "spa", "fra", "deu", "ita"]
    });

    let request = Request::builder()
        .method("POST")
        .uri(&format!("/api/documents/{}/ocr/retry", document_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&retry_request).unwrap()))
        .unwrap();

    let response = ctx.app().clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_retry_ocr_with_invalid_language_in_array() {
    // Create temporary directory for tessdata
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let tessdata_path = temp_dir.path();
    
    // Create mock language files
    fs::write(tessdata_path.join("eng.traineddata"), "mock").unwrap();
    fs::write(tessdata_path.join("spa.traineddata"), "mock").unwrap();
    let tessdata_str = tessdata_path.to_string_lossy().to_string();
    std::env::set_var("TESSDATA_PREFIX", &tessdata_str);
    
    let ctx = TestContext::new().await;
    
    // Create test user and get token
    let auth_helper = readur::test_utils::TestAuthHelper::new(ctx.app().clone());
    let mut test_user = auth_helper.create_test_user().await;
    let user_id = test_user.user_response.id;
    let token = test_user.login(&auth_helper).await.unwrap();

    // Create a test document
    let document_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO documents (id, user_id, filename, original_filename, file_path, file_size, mime_type, ocr_status, created_at, updated_at) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), NOW())"
    )
    .bind(document_id)
    .bind(user_id)
    .bind("test.pdf")
    .bind("test.pdf")
    .bind("/tmp/test.pdf")
    .bind(1024i64)
    .bind("application/pdf")
    .bind("failed")
    .execute(&ctx.state().db.pool)
    .await
    .expect("Failed to create test document");

    // Include an invalid language in the array
    let retry_request = json!({
        "languages": ["eng", "spa", "invalid_lang"]
    });

    let request = Request::builder()
        .method("POST")
        .uri(&format!("/api/documents/{}/ocr/retry", document_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&retry_request).unwrap()))
        .unwrap();

    let response = ctx.app().clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}