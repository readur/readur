#[cfg(test)]
mod tests {
    use anyhow::Result;
    use readur::test_utils::{TestContext, TestConfigBuilder, TestAuthHelper};
    use axum::http::StatusCode;
    use tower::util::ServiceExt;

    /// Build a multipart/form-data body with a single file part.
    /// Returns (boundary, body_bytes).
    fn create_multipart_body(content: &[u8], filename: &str, mime_type: &str) -> (String, Vec<u8>) {
        let boundary = format!("----boundary{}", uuid::Uuid::new_v4());
        let mut body = Vec::new();
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"file\"; filename=\"{}\"\r\n",
                filename
            )
            .as_bytes(),
        );
        body.extend_from_slice(format!("Content-Type: {}\r\n", mime_type).as_bytes());
        body.extend_from_slice(b"\r\n");
        body.extend_from_slice(content);
        body.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());
        (boundary, body)
    }

    fn upload_request(
        token: &str,
        boundary: &str,
        body: Vec<u8>,
    ) -> axum::http::Request<axum::body::Body> {
        axum::http::Request::builder()
            .method("POST")
            .uri("/api/documents")
            .header("Authorization", format!("Bearer {}", token))
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(axum::body::Body::from(body))
            .unwrap()
    }

    /// Test uploading files of various sizes to verify body limit configuration.
    /// Uses a 50MB body limit so we can test small, medium, large (all under limit)
    /// and oversized (over limit).
    #[tokio::test]
    async fn test_file_size_limits() {
        let config = TestConfigBuilder::default().with_max_file_size_mb(50);
        let ctx = TestContext::with_config(config).await;

        let result: Result<()> = async {
            let auth_helper = TestAuthHelper::new(ctx.app.clone());
            let user = auth_helper.create_test_user().await;
            let token = auth_helper.login_user(&user.username, "password123").await;

            // Test 1: Small file (should succeed)
            let small_content = "Small test file content.".repeat(100).into_bytes(); // ~2.5KB
            let (boundary, body) = create_multipart_body(&small_content, "small_test.txt", "text/plain");
            let response = ctx.app.clone()
                .oneshot(upload_request(&token, &boundary, body))
                .await
                .unwrap();
            assert!(response.status().is_success(), "Small file upload should succeed, got: {}", response.status());

            // Test 2: Medium file (should succeed) - 3MB
            let medium_content = "Medium test file content. ".repeat(125_000).into_bytes();
            let (boundary, body) = create_multipart_body(&medium_content, "medium_test.txt", "text/plain");
            let response = ctx.app.clone()
                .oneshot(upload_request(&token, &boundary, body))
                .await
                .unwrap();
            assert!(response.status().is_success(), "Medium file upload should succeed, got: {}", response.status());

            // Test 3: Large file (should succeed) - 15MB
            let large_content = "Large test file content. ".repeat(625_000).into_bytes();
            let (boundary, body) = create_multipart_body(&large_content, "large_test.txt", "text/plain");
            let response = ctx.app.clone()
                .oneshot(upload_request(&token, &boundary, body))
                .await
                .unwrap();
            assert!(response.status().is_success(), "Large file upload should succeed, got: {}", response.status());

            // Test 4: Oversized file (should fail) - 60MB exceeds the 50MB limit
            let oversized_content = vec![b'X'; 60 * 1024 * 1024];
            let (boundary, body) = create_multipart_body(&oversized_content, "oversized_test.bin", "application/octet-stream");
            let response = ctx.app.clone()
                .oneshot(upload_request(&token, &boundary, body))
                .await
                .unwrap();
            assert!(
                response.status() == StatusCode::PAYLOAD_TOO_LARGE
                    || response.status() == StatusCode::BAD_REQUEST,
                "Oversized file upload should fail with 413 or 400, got: {}",
                response.status()
            );

            Ok(())
        }.await;

        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }

        result.unwrap();
    }

    /// Test specifically with the problematic PDF from the GitHub issue
    #[tokio::test]
    async fn test_problematic_pdf_upload() {
        let pdf_path = "test_files/porters-handbook_en.pdf";
        if !std::path::Path::new(pdf_path).exists() {
            println!("Problematic PDF file not found at {}, skipping test", pdf_path);
            return;
        }

        let config = TestConfigBuilder::default().with_max_file_size_mb(50);
        let ctx = TestContext::with_config(config).await;

        let result: Result<()> = async {
            let auth_helper = TestAuthHelper::new(ctx.app.clone());
            let user = auth_helper.create_test_user().await;
            let token = auth_helper.login_user(&user.username, "password123").await;

            let pdf_data = std::fs::read(pdf_path)
                .expect("Should be able to read PDF file");

            let (boundary, body) = create_multipart_body(&pdf_data, "porters-handbook_en.pdf", "application/pdf");
            let response = ctx.app.clone()
                .oneshot(upload_request(&token, &boundary, body))
                .await
                .unwrap();

            if response.status().is_success() {
                let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
                    .await
                    .unwrap();
                let response_body: serde_json::Value = serde_json::from_slice(&body_bytes)
                    .expect("Should get JSON response");

                assert!(response_body.get("id").is_some(), "Response should contain document ID");
                assert_eq!(
                    response_body.get("filename").and_then(|v| v.as_str()),
                    Some("porters-handbook_en.pdf"),
                    "Filename should match"
                );
            } else {
                let status = response.status();
                let error_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
                    .await
                    .unwrap_or_default();
                let error_text = String::from_utf8_lossy(&error_bytes);
                panic!("PDF upload failed with status: {} - {}", status, error_text);
            }

            Ok(())
        }.await;

        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }

        result.unwrap();
    }

    /// Test that error messages are helpful for oversized files
    #[tokio::test]
    async fn test_oversized_file_error_handling() {
        let config = TestConfigBuilder::default().with_max_file_size_mb(50);
        let ctx = TestContext::with_config(config).await;

        let result: Result<()> = async {
            let auth_helper = TestAuthHelper::new(ctx.app.clone());
            let user = auth_helper.create_test_user().await;
            let token = auth_helper.login_user(&user.username, "password123").await;

            // Create a file that exceeds the 50MB limit
            let oversized_content = vec![b'X'; 60 * 1024 * 1024];
            let (boundary, body) = create_multipart_body(&oversized_content, "huge_file.bin", "application/octet-stream");
            let response = ctx.app.clone()
                .oneshot(upload_request(&token, &boundary, body))
                .await
                .unwrap();

            assert!(
                response.status() == StatusCode::PAYLOAD_TOO_LARGE
                    || response.status() == StatusCode::BAD_REQUEST,
                "Should return 413 or 400 for oversized files, got: {}",
                response.status()
            );

            Ok(())
        }.await;

        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }

        result.unwrap();
    }
}
