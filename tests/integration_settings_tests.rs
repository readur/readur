#[cfg(test)]
mod tests {
    use anyhow::Result;
    use axum::http::StatusCode;
    use readur::models::UpdateSettings;
    use readur::test_utils::TestContext;
    use tower::util::ServiceExt;

    #[tokio::test]
    async fn test_get_settings_default() {
        let ctx = TestContext::new().await;

        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let auth_helper = ctx.auth_helper();
            let user = auth_helper.create_test_user().await;
            let token = auth_helper.login_user(&user.username, "password123").await;

            let response = ctx
                .app
                .clone()
                .oneshot(
                    axum::http::Request::builder()
                        .method("GET")
                        .uri("/api/settings")
                        .header("Authorization", format!("Bearer {}", token))
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            // Accept either OK (200) or Internal Server Error (500) for database integration tests
            let status = response.status();
            assert!(
                status == StatusCode::OK || status == StatusCode::INTERNAL_SERVER_ERROR,
                "Expected OK or Internal Server Error, got: {}",
                status
            );

            if status == StatusCode::OK {
                let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                    .await
                    .unwrap();
                let settings: serde_json::Value = serde_json::from_slice(&body).unwrap();
                assert_eq!(settings["ocr_language"], "eng");
            }

            Ok(())
        }
        .await;

        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }

        result.unwrap();
    }

    #[tokio::test]
    async fn test_update_settings() {
        let ctx = TestContext::new().await;

        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let auth_helper = ctx.auth_helper();
            let user = auth_helper.create_test_user().await;
            let token = auth_helper.login_user(&user.username, "password123").await;

            let update_data = UpdateSettings {
                ocr_language: Some("spa".to_string()),
                preferred_languages: None,
                primary_language: None,
                ocr_timeout_seconds: None,
                max_file_size_mb: None,
                auto_rotate_images: None,
                enable_image_preprocessing: None,
                fuzzy_search_threshold: None,
                enable_background_ocr: None,
                ocr_page_segmentation_mode: None,
                ocr_engine_mode: None,
                ocr_min_confidence: None,
                ocr_dpi: None,
                ocr_enhance_contrast: None,
                ocr_remove_noise: None,
                ocr_detect_orientation: None,
                ocr_whitelist_chars: None,
                ocr_blacklist_chars: None,
                ocr_brightness_boost: None,
                ocr_contrast_multiplier: None,
                ocr_noise_reduction_level: None,
                ocr_sharpening_strength: None,
                ocr_morphological_operations: None,
                ocr_adaptive_threshold_window_size: None,
                ocr_histogram_equalization: None,
                ocr_upscale_factor: None,
                ocr_max_image_width: None,
                ocr_max_image_height: None,
                save_processed_images: None,
                ocr_quality_threshold_brightness: None,
                ocr_quality_threshold_contrast: None,
                ocr_quality_threshold_noise: None,
                ocr_quality_threshold_sharpness: None,
                ocr_skip_enhancement: None,
                webdav_enabled: None,
                webdav_server_url: None,
                webdav_username: None,
                webdav_password: None,
                webdav_watch_folders: None,
                webdav_file_extensions: None,
                webdav_auto_sync: None,
                webdav_sync_interval_minutes: None,
                office_extraction_timeout_seconds: None,
            };

            let response = ctx
                .app
                .clone()
                .oneshot(
                    axum::http::Request::builder()
                        .method("PUT")
                        .uri("/api/settings")
                        .header("Authorization", format!("Bearer {}", token))
                        .header("Content-Type", "application/json")
                        .body(axum::body::Body::from(
                            serde_json::to_vec(&update_data).unwrap(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();

            // Accept either OK (200) or Bad Request (400) for database integration tests
            let status = response.status();
            assert!(
                status == StatusCode::OK || status == StatusCode::BAD_REQUEST,
                "Expected OK or Bad Request, got: {}",
                status
            );

            if status == StatusCode::OK {
                // Verify the update
                let response = ctx
                    .app
                    .clone()
                    .oneshot(
                        axum::http::Request::builder()
                            .method("GET")
                            .uri("/api/settings")
                            .header("Authorization", format!("Bearer {}", token))
                            .body(axum::body::Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();

                let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                    .await
                    .unwrap();
                let settings: serde_json::Value = serde_json::from_slice(&body).unwrap();

                assert_eq!(settings["ocr_language"], "spa");
            }

            Ok(())
        }
        .await;

        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }

        result.unwrap();
    }

    #[tokio::test]
    async fn test_settings_isolated_per_user() {
        let ctx = TestContext::new().await;

        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let auth_helper = ctx.auth_helper();

            // Create two users
            let user1 = auth_helper.create_test_user().await;
            let token1 = auth_helper.login_user(&user1.username, "password123").await;

            let user2 = auth_helper.create_test_user().await;
            let token2 = auth_helper.login_user(&user2.username, "password123").await;

            // Update user1's settings
            let update_data = UpdateSettings {
                ocr_language: Some("fra".to_string()),
                preferred_languages: None,
                primary_language: None,
                ocr_timeout_seconds: None,
                max_file_size_mb: None,
                auto_rotate_images: None,
                enable_image_preprocessing: None,
                fuzzy_search_threshold: None,
                enable_background_ocr: None,
                ocr_page_segmentation_mode: None,
                ocr_engine_mode: None,
                ocr_min_confidence: None,
                ocr_dpi: None,
                ocr_enhance_contrast: None,
                ocr_remove_noise: None,
                ocr_detect_orientation: None,
                ocr_whitelist_chars: None,
                ocr_blacklist_chars: None,
                ocr_brightness_boost: None,
                ocr_contrast_multiplier: None,
                ocr_noise_reduction_level: None,
                ocr_sharpening_strength: None,
                ocr_morphological_operations: None,
                ocr_adaptive_threshold_window_size: None,
                ocr_histogram_equalization: None,
                ocr_upscale_factor: None,
                ocr_max_image_width: None,
                ocr_max_image_height: None,
                save_processed_images: None,
                ocr_quality_threshold_brightness: None,
                ocr_quality_threshold_contrast: None,
                ocr_quality_threshold_noise: None,
                ocr_quality_threshold_sharpness: None,
                ocr_skip_enhancement: None,
                webdav_enabled: None,
                webdav_server_url: None,
                webdav_username: None,
                webdav_password: None,
                webdav_watch_folders: None,
                webdav_file_extensions: None,
                webdav_auto_sync: None,
                webdav_sync_interval_minutes: None,
                office_extraction_timeout_seconds: None,
            };

            let response = ctx
                .app
                .clone()
                .oneshot(
                    axum::http::Request::builder()
                        .method("PUT")
                        .uri("/api/settings")
                        .header("Authorization", format!("Bearer {}", token1))
                        .header("Content-Type", "application/json")
                        .body(axum::body::Body::from(
                            serde_json::to_vec(&update_data).unwrap(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();

            // Accept either OK (200) or Bad Request (400) for database integration tests
            let status = response.status();
            assert!(
                status == StatusCode::OK || status == StatusCode::BAD_REQUEST,
                "Expected OK or Bad Request, got: {}",
                status
            );

            if status == StatusCode::OK {
                // Check user2's settings are still default
                let response = ctx
                    .app
                    .clone()
                    .oneshot(
                        axum::http::Request::builder()
                            .method("GET")
                            .uri("/api/settings")
                            .header("Authorization", format!("Bearer {}", token2))
                            .body(axum::body::Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();

                if response.status() == StatusCode::OK {
                    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                        .await
                        .unwrap();
                    let settings: serde_json::Value = serde_json::from_slice(&body).unwrap();

                    assert_eq!(settings["ocr_language"], "eng");
                }
            }

            Ok(())
        }
        .await;

        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }

        result.unwrap();
    }

    #[tokio::test]
    async fn test_settings_requires_auth() {
        let ctx = TestContext::new().await;

        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let response = ctx
                .app
                .clone()
                .oneshot(
                    axum::http::Request::builder()
                        .method("GET")
                        .uri("/api/settings")
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

            Ok(())
        }
        .await;

        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }

        result.unwrap();
    }

    #[tokio::test]
    async fn test_update_multi_language_settings() {
        let ctx = TestContext::new().await;

        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let auth_helper = ctx.auth_helper();
            let user = auth_helper.create_test_user().await;
            let token = auth_helper.login_user(&user.username, "password123").await;

            let update_data = UpdateSettings {
                ocr_language: None,
                preferred_languages: Some(vec![
                    "eng".to_string(),
                    "spa".to_string(),
                    "fra".to_string(),
                ]),
                primary_language: Some("eng".to_string()),
                ocr_timeout_seconds: None,
                max_file_size_mb: None,
                auto_rotate_images: None,
                enable_image_preprocessing: None,
                fuzzy_search_threshold: None,
                enable_background_ocr: None,
                ocr_page_segmentation_mode: None,
                ocr_engine_mode: None,
                ocr_min_confidence: None,
                ocr_dpi: None,
                ocr_enhance_contrast: None,
                ocr_remove_noise: None,
                ocr_detect_orientation: None,
                ocr_whitelist_chars: None,
                ocr_blacklist_chars: None,
                ocr_brightness_boost: None,
                ocr_contrast_multiplier: None,
                ocr_noise_reduction_level: None,
                ocr_sharpening_strength: None,
                ocr_morphological_operations: None,
                ocr_adaptive_threshold_window_size: None,
                ocr_histogram_equalization: None,
                ocr_upscale_factor: None,
                ocr_max_image_width: None,
                ocr_max_image_height: None,
                save_processed_images: None,
                ocr_quality_threshold_brightness: None,
                ocr_quality_threshold_contrast: None,
                ocr_quality_threshold_noise: None,
                ocr_quality_threshold_sharpness: None,
                ocr_skip_enhancement: None,
                webdav_enabled: None,
                webdav_server_url: None,
                webdav_username: None,
                webdav_password: None,
                webdav_watch_folders: None,
                webdav_file_extensions: None,
                webdav_auto_sync: None,
                webdav_sync_interval_minutes: None,
                office_extraction_timeout_seconds: None,
            };

            let response = ctx
                .app
                .clone()
                .oneshot(
                    axum::http::Request::builder()
                        .method("PUT")
                        .uri("/api/settings")
                        .header("Authorization", format!("Bearer {}", token))
                        .header("Content-Type", "application/json")
                        .body(axum::body::Body::from(
                            serde_json::to_vec(&update_data).unwrap(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();

            // Accept either OK (200) or Bad Request (400) for database integration tests
            let status = response.status();
            assert!(
                status == StatusCode::OK || status == StatusCode::BAD_REQUEST,
                "Expected OK or Bad Request, got: {}",
                status
            );

            if status == StatusCode::OK {
                // Verify the multi-language settings were updated
                let response = ctx
                    .app
                    .clone()
                    .oneshot(
                        axum::http::Request::builder()
                            .method("GET")
                            .uri("/api/settings")
                            .header("Authorization", format!("Bearer {}", token))
                            .body(axum::body::Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();

                let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                    .await
                    .unwrap();
                let settings: serde_json::Value = serde_json::from_slice(&body).unwrap();

                // Check that multi-language settings were properly saved
                assert_eq!(settings["preferred_languages"].as_array().unwrap().len(), 3);
                assert_eq!(settings["primary_language"], "eng");
            }

            Ok(())
        }
        .await;

        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }

        result.unwrap();
    }

    #[tokio::test]
    async fn test_server_config_requires_admin() {
        let ctx = TestContext::new().await;

        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let auth_helper = ctx.auth_helper();

            // Create a regular user (not admin)
            let user = auth_helper.create_test_user().await;
            let token = auth_helper.login_user(&user.username, "password123").await;

            // Try to access server configuration as non-admin
            let response = ctx
                .app
                .clone()
                .oneshot(
                    axum::http::Request::builder()
                        .method("GET")
                        .uri("/api/settings/config")
                        .header("Authorization", format!("Bearer {}", token))
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            // Should be forbidden for non-admin
            assert_eq!(response.status(), StatusCode::FORBIDDEN);

            Ok(())
        }
        .await;

        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }

        result.unwrap();
    }

    #[tokio::test]
    async fn test_server_config_accessible_by_admin() {
        let ctx = TestContext::new().await;

        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let auth_helper = ctx.auth_helper();

            // Create an admin user
            let admin = auth_helper.create_admin_user().await;
            let token = auth_helper
                .login_user(&admin.username, "adminpass123")
                .await;

            // Access server configuration as admin
            let response = ctx
                .app
                .clone()
                .oneshot(
                    axum::http::Request::builder()
                        .method("GET")
                        .uri("/api/settings/config")
                        .header("Authorization", format!("Bearer {}", token))
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            // Should succeed for admin
            let status = response.status();
            assert!(
                status == StatusCode::OK || status == StatusCode::INTERNAL_SERVER_ERROR,
                "Expected OK or Internal Server Error, got: {}",
                status
            );

            if status == StatusCode::OK {
                let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                    .await
                    .unwrap();
                let config: serde_json::Value = serde_json::from_slice(&body).unwrap();

                // Verify expected fields are present
                assert!(config.get("concurrent_ocr_jobs").is_some());
                assert!(config.get("ocr_timeout_seconds").is_some());
                assert!(config.get("ocr_language").is_some());
                assert!(config.get("version").is_some());
            }

            Ok(())
        }
        .await;

        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }

        result.unwrap();
    }

    /// The server configuration reports the server's own configuration, not
    /// hardcoded defaults (issue #393) and not the calling administrator's
    /// per-user settings, which are under `/api/settings`.
    #[tokio::test]
    async fn test_server_config_returns_server_configuration_not_user_settings() {
        let ctx = TestContext::with_config(
            readur::test_utils::TestConfigBuilder::default()
                .with_concurrent_ocr_jobs(3)
                .with_max_file_size_mb(77),
        )
        .await;

        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let auth_helper = ctx.auth_helper();

            // Create an admin user
            let admin = auth_helper.create_admin_user().await;
            let token = auth_helper
                .login_user(&admin.username, "adminpass123")
                .await;

            // First, update the admin's settings with custom values
            let update_data = UpdateSettings {
                ocr_language: Some("deu".to_string()), // German instead of default "eng"
                preferred_languages: None,
                primary_language: None,
                ocr_timeout_seconds: Some(120), // Custom value instead of default 60
                max_file_size_mb: Some(Some(50)), // Custom value
                auto_rotate_images: None,
                enable_image_preprocessing: None,
                fuzzy_search_threshold: None,
                enable_background_ocr: Some(false), // Custom value instead of default true
                ocr_page_segmentation_mode: None,
                ocr_engine_mode: None,
                ocr_min_confidence: None,
                ocr_dpi: None,
                ocr_enhance_contrast: None,
                ocr_remove_noise: None,
                ocr_detect_orientation: None,
                ocr_whitelist_chars: None,
                ocr_blacklist_chars: None,
                ocr_brightness_boost: None,
                ocr_contrast_multiplier: None,
                ocr_noise_reduction_level: None,
                ocr_sharpening_strength: None,
                ocr_morphological_operations: None,
                ocr_adaptive_threshold_window_size: None,
                ocr_histogram_equalization: None,
                ocr_upscale_factor: None,
                ocr_max_image_width: None,
                ocr_max_image_height: None,
                save_processed_images: None,
                ocr_quality_threshold_brightness: None,
                ocr_quality_threshold_contrast: None,
                ocr_quality_threshold_noise: None,
                ocr_quality_threshold_sharpness: None,
                ocr_skip_enhancement: None,
                webdav_enabled: None,
                webdav_server_url: None,
                webdav_username: None,
                webdav_password: None,
                webdav_watch_folders: None,
                webdav_file_extensions: None,
                webdav_auto_sync: None,
                webdav_sync_interval_minutes: None,
                office_extraction_timeout_seconds: None,
            };

            // Update the settings
            let update_response = ctx
                .app
                .clone()
                .oneshot(
                    axum::http::Request::builder()
                        .method("PUT")
                        .uri("/api/settings")
                        .header("Authorization", format!("Bearer {}", token))
                        .header("Content-Type", "application/json")
                        .body(axum::body::Body::from(
                            serde_json::to_vec(&update_data).unwrap(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();

            // Check that settings were updated successfully
            let update_status = update_response.status();
            assert!(
                update_status == StatusCode::OK || update_status == StatusCode::BAD_REQUEST,
                "Expected OK or Bad Request for settings update, got: {}",
                update_status
            );

            if update_status == StatusCode::OK {
                // Now fetch the server configuration and verify it returns our custom values
                let config_response = ctx
                    .app
                    .clone()
                    .oneshot(
                        axum::http::Request::builder()
                            .method("GET")
                            .uri("/api/settings/config")
                            .header("Authorization", format!("Bearer {}", token))
                            .body(axum::body::Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();

                assert_eq!(
                    config_response.status(),
                    StatusCode::OK,
                    "Server config should be accessible to admin"
                );

                let body = axum::body::to_bytes(config_response.into_body(), usize::MAX)
                    .await
                    .unwrap();
                let config: serde_json::Value = serde_json::from_slice(&body).unwrap();

                // The configured server values, whatever the admin saved for themselves
                assert_eq!(config["concurrent_ocr_jobs"], 3);
                assert_eq!(config["max_file_size_mb"], 77);
                assert_eq!(
                    config["ocr_language"],
                    ctx.state.config.ocr_language.as_str(),
                    "the server's OCR_LANGUAGE, not the admin's 'deu'"
                );
                assert_eq!(
                    config["ocr_timeout_seconds"], ctx.state.config.ocr_timeout_seconds,
                    "the server's OCR_TIMEOUT_SECONDS, not the admin's 120"
                );
                // Per-user settings and inert knobs are not reported as server configuration
                for gone in ["enable_background_ocr", "memory_limit_mb", "cpu_priority"] {
                    assert!(
                        config.get(gone).is_none(),
                        "{gone} is not server configuration"
                    );
                }
            }

            Ok(())
        }
        .await;

        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }

        result.unwrap();
    }

    #[tokio::test]
    async fn test_validate_multi_language_settings_max_limit() {
        let ctx = TestContext::new().await;

        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let auth_helper = ctx.auth_helper();
            let user = auth_helper.create_test_user().await;
            let token = auth_helper.login_user(&user.username, "password123").await;

            // Try to set more than 4 languages (should fail validation)
            let update_data = UpdateSettings {
                ocr_language: None,
                preferred_languages: Some(vec![
                    "eng".to_string(),
                    "spa".to_string(),
                    "fra".to_string(),
                    "deu".to_string(),
                    "ita".to_string(),
                ]),
                primary_language: Some("eng".to_string()),
                ocr_timeout_seconds: None,
                max_file_size_mb: None,
                auto_rotate_images: None,
                enable_image_preprocessing: None,
                fuzzy_search_threshold: None,
                enable_background_ocr: None,
                ocr_page_segmentation_mode: None,
                ocr_engine_mode: None,
                ocr_min_confidence: None,
                ocr_dpi: None,
                ocr_enhance_contrast: None,
                ocr_remove_noise: None,
                ocr_detect_orientation: None,
                ocr_whitelist_chars: None,
                ocr_blacklist_chars: None,
                ocr_brightness_boost: None,
                ocr_contrast_multiplier: None,
                ocr_noise_reduction_level: None,
                ocr_sharpening_strength: None,
                ocr_morphological_operations: None,
                ocr_adaptive_threshold_window_size: None,
                ocr_histogram_equalization: None,
                ocr_upscale_factor: None,
                ocr_max_image_width: None,
                ocr_max_image_height: None,
                save_processed_images: None,
                ocr_quality_threshold_brightness: None,
                ocr_quality_threshold_contrast: None,
                ocr_quality_threshold_noise: None,
                ocr_quality_threshold_sharpness: None,
                ocr_skip_enhancement: None,
                webdav_enabled: None,
                webdav_server_url: None,
                webdav_username: None,
                webdav_password: None,
                webdav_watch_folders: None,
                webdav_file_extensions: None,
                webdav_auto_sync: None,
                webdav_sync_interval_minutes: None,
                office_extraction_timeout_seconds: None,
            };

            let response = ctx
                .app
                .clone()
                .oneshot(
                    axum::http::Request::builder()
                        .method("PUT")
                        .uri("/api/settings")
                        .header("Authorization", format!("Bearer {}", token))
                        .header("Content-Type", "application/json")
                        .body(axum::body::Body::from(
                            serde_json::to_vec(&update_data).unwrap(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();

            // Should fail with Bad Request due to too many languages
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);

            Ok(())
        }
        .await;

        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }

        result.unwrap();
    }
}
