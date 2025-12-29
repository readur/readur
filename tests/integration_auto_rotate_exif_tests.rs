/*!
 * Integration tests for EXIF-based auto-rotation during document ingestion.
 *
 * These tests verify that images with EXIF orientation tags are correctly
 * rotated during ingestion when the user's auto_rotate_images setting is enabled.
 */

use anyhow::Result;
use std::sync::Arc;
use uuid::Uuid;

#[cfg(feature = "ocr")]
use image::GenericImageView;

use readur::{
    AppState,
    db::Database,
    models::{CreateUser, UserRole, UpdateSettings},
    services::file_service::FileService,
    storage::{StorageConfig, factory::create_storage_backend},
    test_helpers::create_test_config_with_db,
    ingestion::document_ingestion::{DocumentIngestionService, DocumentIngestionRequest, DeduplicationPolicy, IngestionResult},
};

/// Helper function to create test user with unique identifier
fn create_test_user_with_suffix(suffix: &str) -> CreateUser {
    CreateUser {
        username: format!("autorotate_test_{}", suffix),
        email: format!("autorotate_{}@example.com", suffix),
        password: "test_password".to_string(),
        role: Some(UserRole::User),
    }
}

/// Create test app state with database connection
async fn create_test_app_state() -> Result<Arc<AppState>> {
    let database_url = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("TEST_DATABASE_URL"))
        .unwrap_or_else(|_| "postgresql://readur:readur@localhost:5432/readur".to_string());

    let mut config = create_test_config_with_db(&database_url);
    config.server_address = "127.0.0.1:8000".to_string();
    config.jwt_secret = "test-secret".to_string();
    config.upload_path = "./test-uploads-autorotate".to_string();
    config.watch_folder = "./test-watch".to_string();

    // Create upload directory if it doesn't exist
    std::fs::create_dir_all(&config.upload_path).ok();

    let db = Database::new(&config.database_url).await?;

    let storage_config = StorageConfig::Local {
        upload_path: config.upload_path.clone(),
    };
    let storage_backend = create_storage_backend(storage_config)
        .await
        .expect("Failed to create test storage backend");
    let file_service = Arc::new(FileService::with_storage(
        config.upload_path.clone(),
        storage_backend,
    ));

    let queue_service = std::sync::Arc::new(readur::ocr::queue::OcrQueueService::new(
        db.clone(),
        db.get_pool().clone(),
        1,
        file_service.clone(),
        100,
        100,
    ));

    Ok(Arc::new(AppState {
        db: db.clone(),
        config,
        file_service,
        webdav_scheduler: None,
        source_scheduler: None,
        queue_service,
        oidc_client: None,
        sync_progress_tracker: std::sync::Arc::new(
            readur::services::sync_progress_tracker::SyncProgressTracker::new(),
        ),
        user_watch_service: None,
        webdav_metrics_collector: None,
    }))
}

/// Create an UpdateSettings with auto_rotate_images set
fn create_settings_with_auto_rotate(enabled: bool) -> UpdateSettings {
    UpdateSettings {
        ocr_language: None,
        preferred_languages: None,
        primary_language: None,
        auto_detect_language_combination: None,
        concurrent_ocr_jobs: None,
        ocr_timeout_seconds: None,
        max_file_size_mb: None,
        allowed_file_types: None,
        auto_rotate_images: Some(enabled),
        enable_image_preprocessing: None,
        search_results_per_page: None,
        search_snippet_length: None,
        fuzzy_search_threshold: None,
        retention_days: None,
        enable_auto_cleanup: None,
        enable_compression: None,
        memory_limit_mb: None,
        cpu_priority: None,
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
        office_extraction_enable_detailed_logging: None,
    }
}

#[tokio::test]
async fn test_auto_rotate_enabled_rotates_image_with_exif_orientation_6() -> Result<()> {
    let state = create_test_app_state().await?;
    let user = create_test_user_with_suffix(&format!("{}", Uuid::new_v4().simple()));
    let created_user = state.db.create_user(user).await?;
    let user_id = created_user.id;

    // Enable auto-rotate in user settings
    let settings = create_settings_with_auto_rotate(true);
    state.db.create_or_update_settings(user_id, &settings).await?;

    // Load test image with EXIF orientation 6 (90 CW rotation needed)
    // Original image is 40x20, after rotation should be 20x40
    let image_data = std::fs::read("test_files/exif_orientation_6_rotate_90_cw.jpg")
        .expect("read test image");

    let ingestion_service = DocumentIngestionService::new(
        state.db.clone(),
        (*state.file_service).clone(),
    );

    let request = DocumentIngestionRequest {
        filename: "test_rotate_90.jpg".to_string(),
        original_filename: "test_rotate_90.jpg".to_string(),
        file_data: image_data.clone(),
        mime_type: "image/jpeg".to_string(),
        user_id,
        deduplication_policy: DeduplicationPolicy::AllowDuplicateContent,
        source_type: Some("test".to_string()),
        source_id: None,
        original_created_at: None,
        original_modified_at: None,
        source_path: None,
        file_permissions: None,
        file_owner: None,
        file_group: None,
        source_metadata: None,
    };

    let result = ingestion_service.ingest_document(request).await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    match result {
        IngestionResult::Created(doc) => {
            // Read the stored file and verify dimensions changed
            let stored_path = format!(
                "{}/{}/{}/{}",
                state.config.upload_path,
                user_id,
                doc.id,
                doc.filename
            );

            if let Ok(stored_data) = std::fs::read(&stored_path) {
                #[cfg(feature = "ocr")]
                {
                    let stored_img = image::load_from_memory(&stored_data)
                        .expect("load stored image");
                    let (width, height) = stored_img.dimensions();

                    // Original was 40x20, after 90 CW rotation should be 20x40
                    assert_eq!(
                        width, 20,
                        "Width should be 20 after 90 CW rotation (original height)"
                    );
                    assert_eq!(
                        height, 40,
                        "Height should be 40 after 90 CW rotation (original width)"
                    );
                }
            }
        }
        _ => panic!("Expected document to be created"),
    }

    Ok(())
}

#[tokio::test]
async fn test_auto_rotate_disabled_preserves_original_orientation() -> Result<()> {
    let state = create_test_app_state().await?;
    let user = create_test_user_with_suffix(&format!("{}", Uuid::new_v4().simple()));
    let created_user = state.db.create_user(user).await?;
    let user_id = created_user.id;

    // Disable auto-rotate in user settings
    let settings = create_settings_with_auto_rotate(false);
    state.db.create_or_update_settings(user_id, &settings).await?;

    // Load test image with EXIF orientation 6 (would need rotation)
    let image_data = std::fs::read("test_files/exif_orientation_6_rotate_90_cw.jpg")
        .expect("read test image");

    let ingestion_service = DocumentIngestionService::new(
        state.db.clone(),
        (*state.file_service).clone(),
    );

    let request = DocumentIngestionRequest {
        filename: "test_no_rotate.jpg".to_string(),
        original_filename: "test_no_rotate.jpg".to_string(),
        file_data: image_data.clone(),
        mime_type: "image/jpeg".to_string(),
        user_id,
        deduplication_policy: DeduplicationPolicy::AllowDuplicateContent,
        source_type: Some("test".to_string()),
        source_id: None,
        original_created_at: None,
        original_modified_at: None,
        source_path: None,
        file_permissions: None,
        file_owner: None,
        file_group: None,
        source_metadata: None,
    };

    let result = ingestion_service.ingest_document(request).await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    match result {
        IngestionResult::Created(doc) => {
            // Read the stored file and verify dimensions are unchanged
            let stored_path = format!(
                "{}/{}/{}/{}",
                state.config.upload_path,
                user_id,
                doc.id,
                doc.filename
            );

            if let Ok(stored_data) = std::fs::read(&stored_path) {
                #[cfg(feature = "ocr")]
                {
                    let stored_img = image::load_from_memory(&stored_data)
                        .expect("load stored image");
                    let (width, height) = stored_img.dimensions();

                    // Original was 40x20, should remain 40x20 when rotation disabled
                    assert_eq!(
                        width, 40,
                        "Width should be preserved at 40 when auto-rotate disabled"
                    );
                    assert_eq!(
                        height, 20,
                        "Height should be preserved at 20 when auto-rotate disabled"
                    );
                }
            }
        }
        _ => panic!("Expected document to be created"),
    }

    Ok(())
}

#[tokio::test]
async fn test_auto_rotate_with_orientation_8_rotate_270() -> Result<()> {
    let state = create_test_app_state().await?;
    let user = create_test_user_with_suffix(&format!("{}", Uuid::new_v4().simple()));
    let created_user = state.db.create_user(user).await?;
    let user_id = created_user.id;

    // Enable auto-rotate
    let settings = create_settings_with_auto_rotate(true);
    state.db.create_or_update_settings(user_id, &settings).await?;

    // Orientation 8 = Rotate 270 CW (or 90 CCW)
    let image_data = std::fs::read("test_files/exif_orientation_8_rotate_270_cw.jpg")
        .expect("read test image");

    let ingestion_service = DocumentIngestionService::new(
        state.db.clone(),
        (*state.file_service).clone(),
    );

    let request = DocumentIngestionRequest {
        filename: "test_rotate_270.jpg".to_string(),
        original_filename: "test_rotate_270.jpg".to_string(),
        file_data: image_data.clone(),
        mime_type: "image/jpeg".to_string(),
        user_id,
        deduplication_policy: DeduplicationPolicy::AllowDuplicateContent,
        source_type: Some("test".to_string()),
        source_id: None,
        original_created_at: None,
        original_modified_at: None,
        source_path: None,
        file_permissions: None,
        file_owner: None,
        file_group: None,
        source_metadata: None,
    };

    let result = ingestion_service.ingest_document(request).await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    match result {
        IngestionResult::Created(doc) => {
            let stored_path = format!(
                "{}/{}/{}/{}",
                state.config.upload_path,
                user_id,
                doc.id,
                doc.filename
            );

            if let Ok(stored_data) = std::fs::read(&stored_path) {
                #[cfg(feature = "ocr")]
                {
                    let stored_img = image::load_from_memory(&stored_data)
                        .expect("load stored image");
                    let (width, height) = stored_img.dimensions();

                    // Original was 40x20, after 270 CW rotation should be 20x40
                    assert_eq!(width, 20, "Width should be 20 after 270 CW rotation");
                    assert_eq!(height, 40, "Height should be 40 after 270 CW rotation");
                }
            }
        }
        _ => panic!("Expected document to be created"),
    }

    Ok(())
}

#[tokio::test]
async fn test_auto_rotate_with_orientation_3_rotate_180() -> Result<()> {
    let state = create_test_app_state().await?;
    let user = create_test_user_with_suffix(&format!("{}", Uuid::new_v4().simple()));
    let created_user = state.db.create_user(user).await?;
    let user_id = created_user.id;

    // Enable auto-rotate
    let settings = create_settings_with_auto_rotate(true);
    state.db.create_or_update_settings(user_id, &settings).await?;

    // Orientation 3 = Rotate 180 degrees (dimensions stay same, but content rotated)
    let image_data = std::fs::read("test_files/exif_orientation_3_rotate_180.jpg")
        .expect("read test image");

    let ingestion_service = DocumentIngestionService::new(
        state.db.clone(),
        (*state.file_service).clone(),
    );

    let request = DocumentIngestionRequest {
        filename: "test_rotate_180.jpg".to_string(),
        original_filename: "test_rotate_180.jpg".to_string(),
        file_data: image_data.clone(),
        mime_type: "image/jpeg".to_string(),
        user_id,
        deduplication_policy: DeduplicationPolicy::AllowDuplicateContent,
        source_type: Some("test".to_string()),
        source_id: None,
        original_created_at: None,
        original_modified_at: None,
        source_path: None,
        file_permissions: None,
        file_owner: None,
        file_group: None,
        source_metadata: None,
    };

    let result = ingestion_service.ingest_document(request).await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    match result {
        IngestionResult::Created(doc) => {
            let stored_path = format!(
                "{}/{}/{}/{}",
                state.config.upload_path,
                user_id,
                doc.id,
                doc.filename
            );

            if let Ok(stored_data) = std::fs::read(&stored_path) {
                #[cfg(feature = "ocr")]
                {
                    let stored_img = image::load_from_memory(&stored_data)
                        .expect("load stored image");
                    let (width, height) = stored_img.dimensions();

                    // 180 rotation preserves dimensions
                    assert_eq!(width, 40, "Width should be 40 after 180 rotation");
                    assert_eq!(height, 20, "Height should be 20 after 180 rotation");

                    // But the stored data should differ from original
                    assert_ne!(
                        stored_data, image_data,
                        "Rotated image should differ from original"
                    );
                }
            }
        }
        _ => panic!("Expected document to be created"),
    }

    Ok(())
}

#[tokio::test]
async fn test_auto_rotate_no_exif_preserves_image() -> Result<()> {
    let state = create_test_app_state().await?;
    let user = create_test_user_with_suffix(&format!("{}", Uuid::new_v4().simple()));
    let created_user = state.db.create_user(user).await?;
    let user_id = created_user.id;

    // Enable auto-rotate
    let settings = create_settings_with_auto_rotate(true);
    state.db.create_or_update_settings(user_id, &settings).await?;

    // Load image without EXIF data
    let image_data = std::fs::read("test_files/exif_orientation_none.jpg")
        .expect("read test image");

    let ingestion_service = DocumentIngestionService::new(
        state.db.clone(),
        (*state.file_service).clone(),
    );

    let request = DocumentIngestionRequest {
        filename: "test_no_exif.jpg".to_string(),
        original_filename: "test_no_exif.jpg".to_string(),
        file_data: image_data.clone(),
        mime_type: "image/jpeg".to_string(),
        user_id,
        deduplication_policy: DeduplicationPolicy::AllowDuplicateContent,
        source_type: Some("test".to_string()),
        source_id: None,
        original_created_at: None,
        original_modified_at: None,
        source_path: None,
        file_permissions: None,
        file_owner: None,
        file_group: None,
        source_metadata: None,
    };

    let result = ingestion_service.ingest_document(request).await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    match result {
        IngestionResult::Created(doc) => {
            let stored_path = format!(
                "{}/{}/{}/{}",
                state.config.upload_path,
                user_id,
                doc.id,
                doc.filename
            );

            if let Ok(stored_data) = std::fs::read(&stored_path) {
                #[cfg(feature = "ocr")]
                {
                    let stored_img = image::load_from_memory(&stored_data)
                        .expect("load stored image");
                    let (width, height) = stored_img.dimensions();

                    // No EXIF, dimensions should be preserved
                    assert_eq!(width, 40, "Width should be 40 (no rotation needed)");
                    assert_eq!(height, 20, "Height should be 20 (no rotation needed)");
                }
            }
        }
        _ => panic!("Expected document to be created"),
    }

    Ok(())
}

#[tokio::test]
async fn test_auto_rotate_non_image_file_not_affected() -> Result<()> {
    let state = create_test_app_state().await?;
    let user = create_test_user_with_suffix(&format!("{}", Uuid::new_v4().simple()));
    let created_user = state.db.create_user(user).await?;
    let user_id = created_user.id;

    // Enable auto-rotate
    let settings = create_settings_with_auto_rotate(true);
    state.db.create_or_update_settings(user_id, &settings).await?;

    // Use a text file (non-image)
    let file_data = b"This is a text file, not an image.".to_vec();

    let ingestion_service = DocumentIngestionService::new(
        state.db.clone(),
        (*state.file_service).clone(),
    );

    let request = DocumentIngestionRequest {
        filename: "test_document.txt".to_string(),
        original_filename: "test_document.txt".to_string(),
        file_data: file_data.clone(),
        mime_type: "text/plain".to_string(),
        user_id,
        deduplication_policy: DeduplicationPolicy::AllowDuplicateContent,
        source_type: Some("test".to_string()),
        source_id: None,
        original_created_at: None,
        original_modified_at: None,
        source_path: None,
        file_permissions: None,
        file_owner: None,
        file_group: None,
        source_metadata: None,
    };

    let result = ingestion_service.ingest_document(request).await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    match result {
        IngestionResult::Created(doc) => {
            let stored_path = format!(
                "{}/{}/{}/{}",
                state.config.upload_path,
                user_id,
                doc.id,
                doc.filename
            );

            if let Ok(stored_data) = std::fs::read(&stored_path) {
                // Text file content should be unchanged
                assert_eq!(
                    stored_data, file_data,
                    "Non-image file content should be unchanged"
                );
            }
        }
        _ => panic!("Expected document to be created"),
    }

    Ok(())
}

#[tokio::test]
async fn test_auto_rotate_fallback_on_corrupted_image() -> Result<()> {
    let state = create_test_app_state().await?;
    let user = create_test_user_with_suffix(&format!("{}", Uuid::new_v4().simple()));
    let created_user = state.db.create_user(user).await?;
    let user_id = created_user.id;

    // Enable auto-rotate
    let settings = create_settings_with_auto_rotate(true);
    state.db.create_or_update_settings(user_id, &settings).await?;

    // Create corrupted "image" data (valid JPEG header but invalid content)
    let corrupted_data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];

    let ingestion_service = DocumentIngestionService::new(
        state.db.clone(),
        (*state.file_service).clone(),
    );

    let request = DocumentIngestionRequest {
        filename: "corrupted.jpg".to_string(),
        original_filename: "corrupted.jpg".to_string(),
        file_data: corrupted_data.clone(),
        mime_type: "image/jpeg".to_string(),
        user_id,
        deduplication_policy: DeduplicationPolicy::AllowDuplicateContent,
        source_type: Some("test".to_string()),
        source_id: None,
        original_created_at: None,
        original_modified_at: None,
        source_path: None,
        file_permissions: None,
        file_owner: None,
        file_group: None,
        source_metadata: None,
    };

    // Should not fail - should fall back to using original data
    let result = ingestion_service.ingest_document(request).await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    match result {
        IngestionResult::Created(doc) => {
            let stored_path = format!(
                "{}/{}/{}/{}",
                state.config.upload_path,
                user_id,
                doc.id,
                doc.filename
            );

            if let Ok(stored_data) = std::fs::read(&stored_path) {
                // Original corrupted data should be stored as-is (fallback)
                assert_eq!(
                    stored_data, corrupted_data,
                    "Corrupted image should be stored unchanged (fallback behavior)"
                );
            }
        }
        _ => panic!("Expected document to be created even with corrupted image"),
    }

    Ok(())
}

#[tokio::test]
async fn test_auto_rotate_with_all_orientations() -> Result<()> {
    let state = create_test_app_state().await?;
    let user = create_test_user_with_suffix(&format!("{}", Uuid::new_v4().simple()));
    let created_user = state.db.create_user(user).await?;
    let user_id = created_user.id;

    // Enable auto-rotate
    let settings = create_settings_with_auto_rotate(true);
    state.db.create_or_update_settings(user_id, &settings).await?;

    let ingestion_service = DocumentIngestionService::new(
        state.db.clone(),
        (*state.file_service).clone(),
    );

    // Test all 8 EXIF orientations plus no-EXIF case
    let test_cases = [
        ("exif_orientation_1_normal.jpg", (40, 20)),        // No change
        ("exif_orientation_2_flip_horizontal.jpg", (40, 20)), // Flip H
        ("exif_orientation_3_rotate_180.jpg", (40, 20)),    // 180 rotation
        ("exif_orientation_4_flip_vertical.jpg", (40, 20)), // Flip V
        ("exif_orientation_5_transpose.jpg", (20, 40)),     // Transpose
        ("exif_orientation_6_rotate_90_cw.jpg", (20, 40)),  // 90 CW
        ("exif_orientation_7_transverse.jpg", (20, 40)),    // Transverse
        ("exif_orientation_8_rotate_270_cw.jpg", (20, 40)), // 270 CW
        ("exif_orientation_none.jpg", (40, 20)),            // No EXIF
    ];

    for (filename, expected_dims) in test_cases {
        let image_data = std::fs::read(format!("test_files/{}", filename))
            .expect(&format!("read {}", filename));

        let request = DocumentIngestionRequest {
            filename: filename.to_string(),
            original_filename: filename.to_string(),
            file_data: image_data,
            mime_type: "image/jpeg".to_string(),
            user_id,
            deduplication_policy: DeduplicationPolicy::AllowDuplicateContent,
            source_type: Some("test".to_string()),
            source_id: None,
            original_created_at: None,
            original_modified_at: None,
            source_path: None,
            file_permissions: None,
            file_owner: None,
            file_group: None,
            source_metadata: None,
        };

        let result = ingestion_service.ingest_document(request).await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        match result {
            IngestionResult::Created(doc) => {
                let stored_path = format!(
                    "{}/{}/{}/{}",
                    state.config.upload_path,
                    user_id,
                    doc.id,
                    doc.filename
                );

                if let Ok(stored_data) = std::fs::read(&stored_path) {
                    #[cfg(feature = "ocr")]
                    {
                        let stored_img = image::load_from_memory(&stored_data)
                            .expect(&format!("load stored image for {}", filename));
                        let (width, height) = stored_img.dimensions();

                        assert_eq!(
                            (width, height),
                            expected_dims,
                            "Dimensions mismatch for {}: expected {:?}, got ({}, {})",
                            filename,
                            expected_dims,
                            width,
                            height
                        );
                    }
                }
            }
            _ => panic!("Expected document to be created for {}", filename),
        }
    }

    Ok(())
}
