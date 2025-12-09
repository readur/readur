use readur::test_utils::{TestContext, AssertRequest};
use axum::http::StatusCode;
use axum::body::Body;
use axum::http::Request;
use tower::ServiceExt;
use serde_json::json;
use uuid::Uuid;

// Helper function for tests - no longer needs tessdata setup since we use system tesseract
async fn setup_simple_test_context() -> TestContext {
    TestContext::new().await
}

#[tokio::test]
async fn test_get_available_languages_success() {
    let ctx = setup_simple_test_context().await;

    // Create test user and get token
    let auth_helper = readur::test_utils::TestAuthHelper::new(ctx.app().clone());
    let mut test_user = auth_helper.create_test_user().await;
    let token = test_user.login(&auth_helper).await.unwrap();

    let request = Request::builder()
        .method("GET")
        .uri("/api/ocr/languages")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = ctx.app().clone().oneshot(request).await.unwrap();

    let body = AssertRequest::assert_response(
        response,
        StatusCode::OK,
        "get available languages",
        "/api/ocr/languages",
        None::<&serde_json::Value>,
    ).await.expect("Response assertion failed");

    if body.get("available_languages").is_none() {
        println!("🔍 AssertRequest Debug Info for: available_languages field check");
        println!("🔗 Request URL: /api/ocr/languages");
        println!("📤 Request Payload: (empty - GET request)");
        println!("📊 Response Status: 200");
        println!("📝 Response Body:");
        println!("{}", serde_json::to_string_pretty(&body).unwrap_or_else(|_| body.to_string()));
        panic!("Response missing 'available_languages' field");
    }

    let languages = body["available_languages"].as_array().unwrap();
    if languages.is_empty() {
        println!("🔍 AssertRequest Debug Info for: minimum languages check");
        println!("🔗 Request URL: /api/ocr/languages");
        println!("📤 Request Payload: (empty - GET request)");
        println!("📊 Response Status: 200");
        println!("📝 Response Body:");
        println!("{}", serde_json::to_string_pretty(&body).unwrap_or_else(|_| body.to_string()));
        panic!("Expected at least 1 language, got 0");
    }

    // Check that languages have the expected structure
    for (i, lang) in languages.iter().enumerate() {
        if lang.get("code").is_none() || lang.get("name").is_none() {
            println!("🔍 AssertRequest Debug Info for: language structure check");
            println!("🔗 Request URL: /api/ocr/languages");
            println!("📤 Request Payload: (empty - GET request)");
            println!("📊 Response Status: 200");
            println!("📝 Response Body:");
            println!("{}", serde_json::to_string_pretty(&body).unwrap_or_else(|_| body.to_string()));
            println!("❌ Language at index {} missing required fields 'code' or 'name': {}", i, lang);
            panic!("Language structure validation failed");
        }
    }

    // Check that English is included
    let has_english = languages.iter().any(|lang| {
        lang.get("code").unwrap().as_str().unwrap() == "eng"
    });
    if !has_english {
        println!("🔍 AssertRequest Debug Info for: English language check");
        println!("🔗 Request URL: /api/ocr/languages");
        println!("📤 Request Payload: (empty - GET request)");
        println!("📊 Response Status: 200");
        println!("📝 Response Body:");
        println!("{}", serde_json::to_string_pretty(&body).unwrap_or_else(|_| body.to_string()));
        let available_codes: Vec<&str> = languages.iter()
            .filter_map(|lang| lang.get("code")?.as_str())
            .collect();
        println!("Available language codes: {:?}", available_codes);
        panic!("English language 'eng' not found in available languages");
    }
}

#[tokio::test]
async fn test_get_available_languages_unauthorized() {
    let ctx = setup_simple_test_context().await;

    // Test against the running server since the test environment has issues
    let client = reqwest::Client::new();
    let response = client
        .get("http://localhost:8000/api/ocr/languages")
        .send()
        .await
        .expect("Failed to make request");

    let status = response.status();
    if status != 401 {
        println!("🔍 AssertRequest Debug Info for: unauthorized access check");
        println!("🔗 Request URL: http://localhost:8000/api/ocr/languages");
        println!("📤 Request Payload: (empty - GET request without auth)");
        println!("📊 Response Status: {} (expected: 401)", status);
        println!("📝 Response Body:");
        let error_text = response.text().await.unwrap_or_else(|_| "Unable to read response body".to_string());
        println!("{}", error_text);
        panic!("Expected status 401 (unauthorized), got {}. Response: {}", status, error_text);
    }
}

#[tokio::test]
async fn test_retry_ocr_with_language_success() {
    let ctx = setup_simple_test_context().await;
    
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
    
    let body = AssertRequest::assert_response(
        response,
        StatusCode::OK,
        "retry OCR with language success",
        &format!("/api/documents/{}/ocr/retry", document_id),
        Some(&retry_request),
    ).await.expect("Response assertion failed");

    if body["success"].as_bool() != Some(true) {
        println!("🔍 AssertRequest Debug Info for: success field check");
        println!("🔗 Request URL: /api/documents/{}/ocr/retry", document_id);
        println!("📤 Request Payload:");
        println!("{}", serde_json::to_string_pretty(&retry_request).unwrap());
        println!("📝 Response Body:");
        println!("{}", serde_json::to_string_pretty(&body).unwrap_or_else(|_| body.to_string()));
        panic!("Expected success=true in response body");
    }

    if body.get("message").is_none() {
        println!("🔍 AssertRequest Debug Info for: message field check");
        println!("🔗 Request URL: /api/documents/{}/ocr/retry", document_id);
        println!("📤 Request Payload:");
        println!("{}", serde_json::to_string_pretty(&retry_request).unwrap());
        println!("📝 Response Body:");
        println!("{}", serde_json::to_string_pretty(&body).unwrap_or_else(|_| body.to_string()));
        panic!("Expected 'message' field in response body");
    }
}

#[tokio::test]
async fn test_retry_ocr_with_invalid_language() {
    let ctx = setup_simple_test_context().await;
    
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
    
    AssertRequest::assert_response(
        response,
        StatusCode::BAD_REQUEST,
        "retry OCR with invalid language",
        &format!("/api/documents/{}/ocr/retry", document_id),
        Some(&retry_request),
    ).await.expect("Response assertion failed");
}

#[tokio::test]
async fn test_retry_ocr_with_multiple_languages_success() {
    let ctx = setup_simple_test_context().await;
    
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
    
    let body = AssertRequest::assert_response(
        response,
        StatusCode::OK,
        "retry OCR with multiple languages success",
        &format!("/api/documents/{}/ocr/retry", document_id),
        Some(&retry_request),
    ).await.expect("Response assertion failed");

    if body["success"].as_bool() != Some(true) {
        println!("🔍 AssertRequest Debug Info for: multiple languages success field check");
        println!("🔗 Request URL: /api/documents/{}/ocr/retry", document_id);
        println!("📤 Request Payload:");
        println!("{}", serde_json::to_string_pretty(&retry_request).unwrap());
        println!("📝 Response Body:");
        println!("{}", serde_json::to_string_pretty(&body).unwrap_or_else(|_| body.to_string()));
        panic!("Expected success=true in response body");
    }

    if body.get("message").is_none() {
        println!("🔍 AssertRequest Debug Info for: multiple languages message field check");
        println!("🔗 Request URL: /api/documents/{}/ocr/retry", document_id);
        println!("📤 Request Payload:");
        println!("{}", serde_json::to_string_pretty(&retry_request).unwrap());
        println!("📝 Response Body:");
        println!("{}", serde_json::to_string_pretty(&body).unwrap_or_else(|_| body.to_string()));
        panic!("Expected 'message' field in response body");
    }
}

#[tokio::test]
async fn test_retry_ocr_with_too_many_languages() {
    let ctx = setup_simple_test_context().await;
    
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
    
    AssertRequest::assert_response(
        response,
        StatusCode::BAD_REQUEST,
        "retry OCR with too many languages",
        &format!("/api/documents/{}/ocr/retry", document_id),
        Some(&retry_request),
    ).await.expect("Response assertion failed");
}

#[tokio::test]
async fn test_retry_ocr_with_invalid_language_in_array() {
    let ctx = setup_simple_test_context().await;
    
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
    
    AssertRequest::assert_response(
        response,
        StatusCode::BAD_REQUEST,
        "retry OCR with invalid language in array",
        &format!("/api/documents/{}/ocr/retry", document_id),
        Some(&retry_request),
    ).await.expect("Response assertion failed");
}