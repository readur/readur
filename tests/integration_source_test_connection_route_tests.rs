/*!
 * Integration Tests for Source Test Connection Route
 *
 * These tests verify that the /api/sources/test-connection endpoint
 * is correctly registered and accessible. This prevents route mismatch
 * bugs between frontend and backend (Issue #431).
 *
 * The test-connection endpoint allows users to verify source configurations
 * (WebDAV, S3, Local Folder) before creating them.
 */

use reqwest::Client;
use serde_json::json;
use std::time::Duration;

use readur::models::{CreateUser, LoginRequest, LoginResponse, UserRole};

fn get_base_url() -> String {
    std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8000".to_string())
}

/// Helper to register and login a test user
async fn setup_authenticated_client() -> Result<(Client, String), Box<dyn std::error::Error>> {
    let client = Client::new();

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let random_suffix = uuid::Uuid::new_v4().to_string().replace("-", "")[..8].to_string();
    let username = format!("test_conn_route_{}_{}", timestamp, random_suffix);
    let email = format!("test_conn_route_{}@example.com", timestamp);
    let password = "testpassword123";

    // Register user
    let user_data = CreateUser {
        username: username.clone(),
        email: email.clone(),
        password: password.to_string(),
        role: Some(UserRole::User),
    };

    let register_response = client
        .post(&format!("{}/api/auth/register", get_base_url()))
        .json(&user_data)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;

    if !register_response.status().is_success() {
        let status = register_response.status();
        let text = register_response.text().await.unwrap_or_else(|_| "No response body".to_string());
        return Err(format!("Registration failed with status {}: {}", status, text).into());
    }

    // Login to get token
    let login_data = LoginRequest {
        username: username.clone(),
        password: password.to_string(),
    };

    let login_response = client
        .post(&format!("{}/api/auth/login", get_base_url()))
        .json(&login_data)
        .send()
        .await?;

    if !login_response.status().is_success() {
        return Err(format!("Login failed: {}", login_response.text().await?).into());
    }

    let login_result: LoginResponse = login_response.json().await?;
    Ok((client, login_result.token))
}

/// Test that POST /api/sources/test-connection route exists and doesn't return 405
///
/// This test verifies that the route is correctly registered.
/// A 405 (Method Not Allowed) would indicate a route mismatch bug.
#[tokio::test]
async fn test_test_connection_route_exists_webdav() {
    let (client, token) = match setup_authenticated_client().await {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Setup failed: {}", e);
            return;
        }
    };

    let request_body = json!({
        "source_type": "webdav",
        "config": {
            "server_url": "https://example.com/webdav",
            "username": "testuser",
            "password": "testpass",
            "server_type": "generic",
            "watch_folders": ["/Documents"],
            "file_extensions": ["pdf"]
        }
    });

    let response = client
        .post(&format!("{}/api/sources/test-connection", get_base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&request_body)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .expect("Failed to send request");

    let status = response.status();

    // The key assertion: we should NOT get a 405 Method Not Allowed
    // A 405 would indicate the route doesn't exist and fell through to static file serving
    assert_ne!(
        status.as_u16(),
        405,
        "Route /api/sources/test-connection returned 405 Method Not Allowed - route mismatch bug!"
    );

    // We expect either 200 (success), 400 (bad config), or connection error (server not reachable)
    // Any of these indicate the route exists and is being handled by the correct handler
    assert!(
        status.as_u16() == 200 || status.as_u16() == 400 || status.as_u16() == 500,
        "Expected 200, 400, or 500 but got {} - route may not exist",
        status.as_u16()
    );

    println!("✓ WebDAV test-connection route exists (status: {})", status);
}

/// Test that POST /api/sources/test-connection works for local_folder type
#[tokio::test]
async fn test_test_connection_route_exists_local_folder() {
    let (client, token) = match setup_authenticated_client().await {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Setup failed: {}", e);
            return;
        }
    };

    let request_body = json!({
        "source_type": "local_folder",
        "config": {
            "watch_folders": ["/tmp/test-folder"],
            "file_extensions": ["pdf", "txt"],
            "recursive": true,
            "follow_symlinks": false
        }
    });

    let response = client
        .post(&format!("{}/api/sources/test-connection", get_base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&request_body)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .expect("Failed to send request");

    let status = response.status();

    // The key assertion: we should NOT get a 405 Method Not Allowed
    assert_ne!(
        status.as_u16(),
        405,
        "Route /api/sources/test-connection returned 405 Method Not Allowed - route mismatch bug!"
    );

    // Route exists if we get any of these responses
    assert!(
        status.as_u16() == 200 || status.as_u16() == 400 || status.as_u16() == 500,
        "Expected 200, 400, or 500 but got {} - route may not exist",
        status.as_u16()
    );

    println!("✓ Local folder test-connection route exists (status: {})", status);
}

/// Test that POST /api/sources/test-connection works for s3 type
#[tokio::test]
async fn test_test_connection_route_exists_s3() {
    let (client, token) = match setup_authenticated_client().await {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Setup failed: {}", e);
            return;
        }
    };

    let request_body = json!({
        "source_type": "s3",
        "config": {
            "bucket_name": "test-bucket",
            "region": "us-east-1",
            "access_key_id": "AKIAIOSFODNN7EXAMPLE",
            "secret_access_key": "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            "prefix": "documents/",
            "endpoint_url": null
        }
    });

    let response = client
        .post(&format!("{}/api/sources/test-connection", get_base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&request_body)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .expect("Failed to send request");

    let status = response.status();

    // The key assertion: we should NOT get a 405 Method Not Allowed
    assert_ne!(
        status.as_u16(),
        405,
        "Route /api/sources/test-connection returned 405 Method Not Allowed - route mismatch bug!"
    );

    // Route exists if we get any of these responses
    assert!(
        status.as_u16() == 200 || status.as_u16() == 400 || status.as_u16() == 500,
        "Expected 200, 400, or 500 but got {} - route may not exist",
        status.as_u16()
    );

    println!("✓ S3 test-connection route exists (status: {})", status);
}

/// Test that the OLD route /api/sources/test returns 404 (not found)
/// This ensures we don't have duplicate routes
#[tokio::test]
async fn test_old_test_route_does_not_exist() {
    let (client, token) = match setup_authenticated_client().await {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Setup failed: {}", e);
            return;
        }
    };

    let request_body = json!({
        "source_type": "webdav",
        "config": {
            "server_url": "https://example.com/webdav",
            "username": "testuser",
            "password": "testpass",
            "server_type": "generic",
            "watch_folders": ["/Documents"],
            "file_extensions": ["pdf"]
        }
    });

    let response = client
        .post(&format!("{}/api/sources/test", get_base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&request_body)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .expect("Failed to send request");

    let status = response.status();

    // The old /test route should NOT exist - expect 404 or 405 (falls through to static)
    assert!(
        status.as_u16() == 404 || status.as_u16() == 405,
        "Old route /api/sources/test should not exist, but got status {} - possible duplicate route",
        status.as_u16()
    );

    println!("✓ Old /api/sources/test route correctly does not exist (status: {})", status);
}

/// Test that unauthenticated requests return 401
#[tokio::test]
async fn test_test_connection_requires_authentication() {
    let client = Client::new();

    let request_body = json!({
        "source_type": "webdav",
        "config": {
            "server_url": "https://example.com/webdav",
            "username": "testuser",
            "password": "testpass",
            "server_type": "generic",
            "watch_folders": ["/Documents"],
            "file_extensions": ["pdf"]
        }
    });

    let response = client
        .post(&format!("{}/api/sources/test-connection", get_base_url()))
        // No Authorization header
        .json(&request_body)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .expect("Failed to send request");

    let status = response.status();

    // Should get 401 Unauthorized, NOT 405
    assert_ne!(
        status.as_u16(),
        405,
        "Route returned 405 instead of 401 - route mismatch bug!"
    );

    assert_eq!(
        status.as_u16(),
        401,
        "Expected 401 Unauthorized for unauthenticated request, got {}",
        status.as_u16()
    );

    println!("✓ test-connection correctly requires authentication (status: 401)");
}
