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
    
    let ctx = TestContext::new().await;
    
    // Create test user and get token 
    let auth_helper = readur::test_utils::TestAuthHelper::new(ctx.app().clone());
    let mut test_user = auth_helper.create_test_user().await;
    let user_id = test_user.user_response.id;
    let token = test_user.login(&auth_helper).await.unwrap();
    
    // Create user settings
    sqlx::query(
        "INSERT INTO settings (user_id, ocr_language) VALUES ($1, $2)
         ON CONFLICT (user_id) DO UPDATE SET ocr_language = $2"
    )
    .bind(user_id)
    .bind("eng")
    .execute(&ctx.state().db.pool)
    .await
    .expect("Failed to create user settings");

    let request = Request::builder()
        .method("GET")
        .uri("/api/ocr/languages")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = ctx.app().clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    
    assert!(body.get("available_languages").is_some());
    let languages = body["available_languages"].as_array().unwrap();
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
}

#[tokio::test]
async fn test_get_available_languages_unauthorized() {
    // Create temporary directory for tessdata
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let tessdata_path = temp_dir.path().join("tessdata");
    fs::create_dir_all(&tessdata_path).expect("Failed to create tessdata directory");
    
    // Create mock language files
    fs::write(tessdata_path.join("eng.traineddata"), "mock").unwrap();
    std::env::set_var("TESSDATA_PREFIX", &tessdata_path);
    
    let ctx = TestContext::new().await;

    let request = Request::builder()
        .method("GET")
        .uri("/api/ocr/languages")
        .body(Body::empty())
        .unwrap();

    let response = ctx.app().clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_retry_ocr_with_language_success() {
    // Create temporary directory for tessdata
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let tessdata_path = temp_dir.path().join("tessdata");
    fs::create_dir_all(&tessdata_path).expect("Failed to create tessdata directory");
    
    // Create mock language files
    fs::write(tessdata_path.join("eng.traineddata"), "mock").unwrap();
    fs::write(tessdata_path.join("spa.traineddata"), "mock").unwrap();
    std::env::set_var("TESSDATA_PREFIX", &tessdata_path);
    
    let ctx = TestContext::new().await;
    
    // Create test user and get token
    let auth_helper = readur::test_utils::TestAuthHelper::new(ctx.app().clone());
    let mut test_user = auth_helper.create_test_user().await;
    let user_id = test_user.user_response.id;
    let token = test_user.login(&auth_helper).await.unwrap();

    // Create a test document
    let document_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO documents (id, user_id, filename, original_filename, file_size, mime_type, ocr_status, created_at, updated_at) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())"
    )
    .bind(document_id)
    .bind(user_id)
    .bind("test.pdf")
    .bind("test.pdf")
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
        .uri(&format!("/api/documents/{}/retry-ocr", document_id))
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
    let tessdata_path = temp_dir.path().join("tessdata");
    fs::create_dir_all(&tessdata_path).expect("Failed to create tessdata directory");
    
    // Create mock language files
    fs::write(tessdata_path.join("eng.traineddata"), "mock").unwrap();
    std::env::set_var("TESSDATA_PREFIX", &tessdata_path);
    
    let ctx = TestContext::new().await;
    
    // Create test user and get token
    let auth_helper = readur::test_utils::TestAuthHelper::new(ctx.app().clone());
    let mut test_user = auth_helper.create_test_user().await;
    let user_id = test_user.user_response.id;
    let token = test_user.login(&auth_helper).await.unwrap();

    // Create a test document
    let document_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO documents (id, user_id, filename, original_filename, file_size, mime_type, ocr_status, created_at, updated_at) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())"
    )
    .bind(document_id)
    .bind(user_id)
    .bind("test.pdf")
    .bind("test.pdf")
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
        .uri(&format!("/api/documents/{}/retry-ocr", document_id))
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
    let tessdata_path = temp_dir.path().join("tessdata");
    fs::create_dir_all(&tessdata_path).expect("Failed to create tessdata directory");
    
    // Create mock language files
    fs::write(tessdata_path.join("eng.traineddata"), "mock").unwrap();
    fs::write(tessdata_path.join("spa.traineddata"), "mock").unwrap();
    fs::write(tessdata_path.join("fra.traineddata"), "mock").unwrap();
    std::env::set_var("TESSDATA_PREFIX", &tessdata_path);
    
    let ctx = TestContext::new().await;
    
    // Create test user and get token
    let auth_helper = readur::test_utils::TestAuthHelper::new(ctx.app().clone());
    let mut test_user = auth_helper.create_test_user().await;
    let user_id = test_user.user_response.id;
    let token = test_user.login(&auth_helper).await.unwrap();

    // Create a test document
    let document_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO documents (id, user_id, filename, original_filename, file_size, mime_type, ocr_status, created_at, updated_at) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())"
    )
    .bind(document_id)
    .bind(user_id)
    .bind("test.pdf")
    .bind("test.pdf")
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
        .uri(&format!("/api/documents/{}/retry-ocr", document_id))
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
    let tessdata_path = temp_dir.path().join("tessdata");
    fs::create_dir_all(&tessdata_path).expect("Failed to create tessdata directory");
    
    // Create mock language files
    fs::write(tessdata_path.join("eng.traineddata"), "mock").unwrap();
    fs::write(tessdata_path.join("spa.traineddata"), "mock").unwrap();
    fs::write(tessdata_path.join("fra.traineddata"), "mock").unwrap();
    fs::write(tessdata_path.join("deu.traineddata"), "mock").unwrap();
    fs::write(tessdata_path.join("ita.traineddata"), "mock").unwrap();
    std::env::set_var("TESSDATA_PREFIX", &tessdata_path);
    
    let ctx = TestContext::new().await;
    
    // Create test user and get token
    let auth_helper = readur::test_utils::TestAuthHelper::new(ctx.app().clone());
    let mut test_user = auth_helper.create_test_user().await;
    let user_id = test_user.user_response.id;
    let token = test_user.login(&auth_helper).await.unwrap();

    // Create a test document
    let document_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO documents (id, user_id, filename, original_filename, file_size, mime_type, ocr_status, created_at, updated_at) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())"
    )
    .bind(document_id)
    .bind(user_id)
    .bind("test.pdf")
    .bind("test.pdf")
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
        .uri(&format!("/api/documents/{}/retry-ocr", document_id))
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
    let tessdata_path = temp_dir.path().join("tessdata");
    fs::create_dir_all(&tessdata_path).expect("Failed to create tessdata directory");
    
    // Create mock language files
    fs::write(tessdata_path.join("eng.traineddata"), "mock").unwrap();
    fs::write(tessdata_path.join("spa.traineddata"), "mock").unwrap();
    std::env::set_var("TESSDATA_PREFIX", &tessdata_path);
    
    let ctx = TestContext::new().await;
    
    // Create test user and get token
    let auth_helper = readur::test_utils::TestAuthHelper::new(ctx.app().clone());
    let mut test_user = auth_helper.create_test_user().await;
    let user_id = test_user.user_response.id;
    let token = test_user.login(&auth_helper).await.unwrap();

    // Create a test document
    let document_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO documents (id, user_id, filename, original_filename, file_size, mime_type, ocr_status, created_at, updated_at) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())"
    )
    .bind(document_id)
    .bind(user_id)
    .bind("test.pdf")
    .bind("test.pdf")
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
        .uri(&format!("/api/documents/{}/retry-ocr", document_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&retry_request).unwrap()))
        .unwrap();

    let response = ctx.app().clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}