use anyhow::Result;
use sqlx::Row;
use uuid::Uuid;

use super::Database;

impl Database {
    pub async fn get_user_settings(&self, user_id: Uuid) -> Result<Option<crate::models::Settings>> {
        self.with_retry(|| async {
            let row = sqlx::query(
                r#"SELECT id, user_id, ocr_language, concurrent_ocr_jobs, ocr_timeout_seconds,
                   max_file_size_mb, allowed_file_types, auto_rotate_images, enable_image_preprocessing,
                   search_results_per_page, search_snippet_length, fuzzy_search_threshold,
                   retention_days, enable_auto_cleanup, enable_compression, memory_limit_mb,
                   cpu_priority, enable_background_ocr, ocr_page_segmentation_mode, ocr_engine_mode,
                   ocr_min_confidence, ocr_dpi, ocr_enhance_contrast, ocr_remove_noise,
                   ocr_detect_orientation, ocr_whitelist_chars, ocr_blacklist_chars,
                   ocr_brightness_boost, ocr_contrast_multiplier, ocr_noise_reduction_level, ocr_sharpening_strength,
                   ocr_morphological_operations, ocr_adaptive_threshold_window_size, ocr_histogram_equalization,
                   ocr_upscale_factor, ocr_max_image_width, ocr_max_image_height, save_processed_images,
                   ocr_quality_threshold_brightness, ocr_quality_threshold_contrast, ocr_quality_threshold_noise,
                   ocr_quality_threshold_sharpness, ocr_skip_enhancement,
                   webdav_enabled, webdav_server_url, webdav_username, webdav_password,
                   webdav_watch_folders, webdav_file_extensions, webdav_auto_sync, webdav_sync_interval_minutes,
                   created_at, updated_at
                   FROM settings WHERE user_id = $1"#
            )
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Database query failed: {}", e))?;

        match row {
            Some(row) => Ok(Some(crate::models::Settings {
                id: row.get("id"),
                user_id: row.get("user_id"),
                ocr_language: row.get("ocr_language"),
                concurrent_ocr_jobs: row.get("concurrent_ocr_jobs"),
                ocr_timeout_seconds: row.get("ocr_timeout_seconds"),
                max_file_size_mb: row.get("max_file_size_mb"),
                allowed_file_types: row.get("allowed_file_types"),
                auto_rotate_images: row.get("auto_rotate_images"),
                enable_image_preprocessing: row.get("enable_image_preprocessing"),
                search_results_per_page: row.get("search_results_per_page"),
                search_snippet_length: row.get("search_snippet_length"),
                fuzzy_search_threshold: row.get("fuzzy_search_threshold"),
                retention_days: row.get("retention_days"),
                enable_auto_cleanup: row.get("enable_auto_cleanup"),
                enable_compression: row.get("enable_compression"),
                memory_limit_mb: row.get("memory_limit_mb"),
                cpu_priority: row.get("cpu_priority"),
                enable_background_ocr: row.get("enable_background_ocr"),
                ocr_page_segmentation_mode: row.get("ocr_page_segmentation_mode"),
                ocr_engine_mode: row.get("ocr_engine_mode"),
                ocr_min_confidence: row.get("ocr_min_confidence"),
                ocr_dpi: row.get("ocr_dpi"),
                ocr_enhance_contrast: row.get("ocr_enhance_contrast"),
                ocr_remove_noise: row.get("ocr_remove_noise"),
                ocr_detect_orientation: row.get("ocr_detect_orientation"),
                ocr_whitelist_chars: row.get("ocr_whitelist_chars"),
                ocr_blacklist_chars: row.get("ocr_blacklist_chars"),
                ocr_brightness_boost: row.get("ocr_brightness_boost"),
                ocr_contrast_multiplier: row.get("ocr_contrast_multiplier"),
                ocr_noise_reduction_level: row.get("ocr_noise_reduction_level"),
                ocr_sharpening_strength: row.get("ocr_sharpening_strength"),
                ocr_morphological_operations: row.get("ocr_morphological_operations"),
                ocr_adaptive_threshold_window_size: row.get("ocr_adaptive_threshold_window_size"),
                ocr_histogram_equalization: row.get("ocr_histogram_equalization"),
                ocr_upscale_factor: row.get("ocr_upscale_factor"),
                ocr_max_image_width: row.get("ocr_max_image_width"),
                ocr_max_image_height: row.get("ocr_max_image_height"),
                save_processed_images: row.get("save_processed_images"),
                ocr_quality_threshold_brightness: row.get("ocr_quality_threshold_brightness"),
                ocr_quality_threshold_contrast: row.get("ocr_quality_threshold_contrast"),
                ocr_quality_threshold_noise: row.get("ocr_quality_threshold_noise"),
                ocr_quality_threshold_sharpness: row.get("ocr_quality_threshold_sharpness"),
                ocr_skip_enhancement: row.get("ocr_skip_enhancement"),
                webdav_enabled: row.get("webdav_enabled"),
                webdav_server_url: row.get("webdav_server_url"),
                webdav_username: row.get("webdav_username"),
                webdav_password: row.get("webdav_password"),
                webdav_watch_folders: row.get("webdav_watch_folders"),
                webdav_file_extensions: row.get("webdav_file_extensions"),
                webdav_auto_sync: row.get("webdav_auto_sync"),
                webdav_sync_interval_minutes: row.get("webdav_sync_interval_minutes"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })),
            None => Ok(None),
        }
        }).await
    }

    pub async fn get_all_user_settings(&self) -> Result<Vec<crate::models::Settings>> {
        let rows = sqlx::query(
            r#"SELECT id, user_id, ocr_language, concurrent_ocr_jobs, ocr_timeout_seconds,
               max_file_size_mb, allowed_file_types, auto_rotate_images, enable_image_preprocessing,
               search_results_per_page, search_snippet_length, fuzzy_search_threshold,
               retention_days, enable_auto_cleanup, enable_compression, memory_limit_mb,
               cpu_priority, enable_background_ocr, ocr_page_segmentation_mode, ocr_engine_mode,
               ocr_min_confidence, ocr_dpi, ocr_enhance_contrast, ocr_remove_noise,
               ocr_detect_orientation, ocr_whitelist_chars, ocr_blacklist_chars,
               ocr_brightness_boost, ocr_contrast_multiplier, ocr_noise_reduction_level, ocr_sharpening_strength,
               ocr_morphological_operations, ocr_adaptive_threshold_window_size, ocr_histogram_equalization,
               ocr_upscale_factor, ocr_max_image_width, ocr_max_image_height, save_processed_images,
               ocr_quality_threshold_brightness, ocr_quality_threshold_contrast, ocr_quality_threshold_noise,
               ocr_quality_threshold_sharpness, ocr_skip_enhancement,
               webdav_enabled, webdav_server_url, webdav_username, webdav_password,
               webdav_watch_folders, webdav_file_extensions, webdav_auto_sync, webdav_sync_interval_minutes,
               created_at, updated_at
               FROM settings
               WHERE webdav_enabled = true AND webdav_auto_sync = true"#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut settings_list = Vec::new();
        for row in rows {
            settings_list.push(crate::models::Settings {
                id: row.get("id"),
                user_id: row.get("user_id"),
                ocr_language: row.get("ocr_language"),
                concurrent_ocr_jobs: row.get("concurrent_ocr_jobs"),
                ocr_timeout_seconds: row.get("ocr_timeout_seconds"),
                max_file_size_mb: row.get("max_file_size_mb"),
                allowed_file_types: row.get("allowed_file_types"),
                auto_rotate_images: row.get("auto_rotate_images"),
                enable_image_preprocessing: row.get("enable_image_preprocessing"),
                search_results_per_page: row.get("search_results_per_page"),
                search_snippet_length: row.get("search_snippet_length"),
                fuzzy_search_threshold: row.get("fuzzy_search_threshold"),
                retention_days: row.get("retention_days"),
                enable_auto_cleanup: row.get("enable_auto_cleanup"),
                enable_compression: row.get("enable_compression"),
                memory_limit_mb: row.get("memory_limit_mb"),
                cpu_priority: row.get("cpu_priority"),
                enable_background_ocr: row.get("enable_background_ocr"),
                ocr_page_segmentation_mode: row.get("ocr_page_segmentation_mode"),
                ocr_engine_mode: row.get("ocr_engine_mode"),
                ocr_min_confidence: row.get("ocr_min_confidence"),
                ocr_dpi: row.get("ocr_dpi"),
                ocr_enhance_contrast: row.get("ocr_enhance_contrast"),
                ocr_remove_noise: row.get("ocr_remove_noise"),
                ocr_detect_orientation: row.get("ocr_detect_orientation"),
                ocr_whitelist_chars: row.get("ocr_whitelist_chars"),
                ocr_blacklist_chars: row.get("ocr_blacklist_chars"),
                ocr_brightness_boost: row.get("ocr_brightness_boost"),
                ocr_contrast_multiplier: row.get("ocr_contrast_multiplier"),
                ocr_noise_reduction_level: row.get("ocr_noise_reduction_level"),
                ocr_sharpening_strength: row.get("ocr_sharpening_strength"),
                ocr_morphological_operations: row.get("ocr_morphological_operations"),
                ocr_adaptive_threshold_window_size: row.get("ocr_adaptive_threshold_window_size"),
                ocr_histogram_equalization: row.get("ocr_histogram_equalization"),
                ocr_upscale_factor: row.get("ocr_upscale_factor"),
                ocr_max_image_width: row.get("ocr_max_image_width"),
                ocr_max_image_height: row.get("ocr_max_image_height"),
                save_processed_images: row.get("save_processed_images"),
                ocr_quality_threshold_brightness: row.get("ocr_quality_threshold_brightness"),
                ocr_quality_threshold_contrast: row.get("ocr_quality_threshold_contrast"),
                ocr_quality_threshold_noise: row.get("ocr_quality_threshold_noise"),
                ocr_quality_threshold_sharpness: row.get("ocr_quality_threshold_sharpness"),
                ocr_skip_enhancement: row.get("ocr_skip_enhancement"),
                webdav_enabled: row.get("webdav_enabled"),
                webdav_server_url: row.get("webdav_server_url"),
                webdav_username: row.get("webdav_username"),
                webdav_password: row.get("webdav_password"),
                webdav_watch_folders: row.get("webdav_watch_folders"),
                webdav_file_extensions: row.get("webdav_file_extensions"),
                webdav_auto_sync: row.get("webdav_auto_sync"),
                webdav_sync_interval_minutes: row.get("webdav_sync_interval_minutes"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(settings_list)
    }

    pub async fn create_or_update_settings(&self, user_id: Uuid, settings: &crate::models::UpdateSettings) -> Result<crate::models::Settings> {
        // Get existing settings to merge with updates
        let existing = self.get_user_settings(user_id).await?;
        let defaults = crate::models::Settings::default();
        
        // Merge existing/defaults with updates
        let current = existing.unwrap_or_else(|| {
            let mut s = defaults;
            s.user_id = user_id;
            s
        });
        
        let row = sqlx::query(
            r#"
            INSERT INTO settings (
                user_id, ocr_language, concurrent_ocr_jobs, ocr_timeout_seconds,
                max_file_size_mb, allowed_file_types, auto_rotate_images, enable_image_preprocessing,
                search_results_per_page, search_snippet_length, fuzzy_search_threshold,
                retention_days, enable_auto_cleanup, enable_compression, memory_limit_mb,
                cpu_priority, enable_background_ocr, ocr_page_segmentation_mode, ocr_engine_mode,
                ocr_min_confidence, ocr_dpi, ocr_enhance_contrast, ocr_remove_noise,
                ocr_detect_orientation, ocr_whitelist_chars, ocr_blacklist_chars,
                ocr_brightness_boost, ocr_contrast_multiplier, ocr_noise_reduction_level, ocr_sharpening_strength,
                ocr_morphological_operations, ocr_adaptive_threshold_window_size, ocr_histogram_equalization,
                ocr_upscale_factor, ocr_max_image_width, ocr_max_image_height, save_processed_images,
                ocr_quality_threshold_brightness, ocr_quality_threshold_contrast, ocr_quality_threshold_noise,
                ocr_quality_threshold_sharpness, ocr_skip_enhancement,
                webdav_enabled, webdav_server_url, webdav_username, webdav_password,
                webdav_watch_folders, webdav_file_extensions, webdav_auto_sync, webdav_sync_interval_minutes
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, $33, $34, $35, $36, $37, $38, $39, $40, $41, $42, $43, $44, $45, $46, $47, $48, $49, $50)
            ON CONFLICT (user_id) DO UPDATE SET
                ocr_language = $2,
                concurrent_ocr_jobs = $3,
                ocr_timeout_seconds = $4,
                max_file_size_mb = $5,
                allowed_file_types = $6,
                auto_rotate_images = $7,
                enable_image_preprocessing = $8,
                search_results_per_page = $9,
                search_snippet_length = $10,
                fuzzy_search_threshold = $11,
                retention_days = $12,
                enable_auto_cleanup = $13,
                enable_compression = $14,
                memory_limit_mb = $15,
                cpu_priority = $16,
                enable_background_ocr = $17,
                ocr_page_segmentation_mode = $18,
                ocr_engine_mode = $19,
                ocr_min_confidence = $20,
                ocr_dpi = $21,
                ocr_enhance_contrast = $22,
                ocr_remove_noise = $23,
                ocr_detect_orientation = $24,
                ocr_whitelist_chars = $25,
                ocr_blacklist_chars = $26,
                ocr_brightness_boost = $27,
                ocr_contrast_multiplier = $28,
                ocr_noise_reduction_level = $29,
                ocr_sharpening_strength = $30,
                ocr_morphological_operations = $31,
                ocr_adaptive_threshold_window_size = $32,
                ocr_histogram_equalization = $33,
                ocr_upscale_factor = $34,
                ocr_max_image_width = $35,
                ocr_max_image_height = $36,
                save_processed_images = $37,
                ocr_quality_threshold_brightness = $38,
                ocr_quality_threshold_contrast = $39,
                ocr_quality_threshold_noise = $40,
                ocr_quality_threshold_sharpness = $41,
                ocr_skip_enhancement = $42,
                webdav_enabled = $43,
                webdav_server_url = $44,
                webdav_username = $45,
                webdav_password = $46,
                webdav_watch_folders = $47,
                webdav_file_extensions = $48,
                webdav_auto_sync = $49,
                webdav_sync_interval_minutes = $50,
                updated_at = NOW()
            RETURNING id, user_id, ocr_language, concurrent_ocr_jobs, ocr_timeout_seconds,
                      max_file_size_mb, allowed_file_types, auto_rotate_images, enable_image_preprocessing,
                      search_results_per_page, search_snippet_length, fuzzy_search_threshold,
                      retention_days, enable_auto_cleanup, enable_compression, memory_limit_mb,
                      cpu_priority, enable_background_ocr, ocr_page_segmentation_mode, ocr_engine_mode,
                      ocr_min_confidence, ocr_dpi, ocr_enhance_contrast, ocr_remove_noise,
                      ocr_detect_orientation, ocr_whitelist_chars, ocr_blacklist_chars,
                      ocr_brightness_boost, ocr_contrast_multiplier, ocr_noise_reduction_level, ocr_sharpening_strength,
                      ocr_morphological_operations, ocr_adaptive_threshold_window_size, ocr_histogram_equalization,
                      ocr_upscale_factor, ocr_max_image_width, ocr_max_image_height, save_processed_images,
                      ocr_quality_threshold_brightness, ocr_quality_threshold_contrast, ocr_quality_threshold_noise,
                      ocr_quality_threshold_sharpness, ocr_skip_enhancement,
                      webdav_enabled, webdav_server_url, webdav_username, webdav_password,
                      webdav_watch_folders, webdav_file_extensions, webdav_auto_sync, webdav_sync_interval_minutes,
                      created_at, updated_at
            "#
        )
        .bind(user_id)
        .bind(settings.ocr_language.as_ref().unwrap_or(&current.ocr_language))
        .bind(settings.concurrent_ocr_jobs.unwrap_or(current.concurrent_ocr_jobs))
        .bind(settings.ocr_timeout_seconds.unwrap_or(current.ocr_timeout_seconds))
        .bind(settings.max_file_size_mb.unwrap_or(current.max_file_size_mb))
        .bind(settings.allowed_file_types.as_ref().unwrap_or(&current.allowed_file_types))
        .bind(settings.auto_rotate_images.unwrap_or(current.auto_rotate_images))
        .bind(settings.enable_image_preprocessing.unwrap_or(current.enable_image_preprocessing))
        .bind(settings.search_results_per_page.unwrap_or(current.search_results_per_page))
        .bind(settings.search_snippet_length.unwrap_or(current.search_snippet_length))
        .bind(settings.fuzzy_search_threshold.unwrap_or(current.fuzzy_search_threshold))
        .bind(settings.retention_days.unwrap_or(current.retention_days))
        .bind(settings.enable_auto_cleanup.unwrap_or(current.enable_auto_cleanup))
        .bind(settings.enable_compression.unwrap_or(current.enable_compression))
        .bind(settings.memory_limit_mb.unwrap_or(current.memory_limit_mb))
        .bind(settings.cpu_priority.as_ref().unwrap_or(&current.cpu_priority))
        .bind(settings.enable_background_ocr.unwrap_or(current.enable_background_ocr))
        .bind(settings.ocr_page_segmentation_mode.unwrap_or(current.ocr_page_segmentation_mode))
        .bind(settings.ocr_engine_mode.unwrap_or(current.ocr_engine_mode))
        .bind(settings.ocr_min_confidence.unwrap_or(current.ocr_min_confidence))
        .bind(settings.ocr_dpi.unwrap_or(current.ocr_dpi))
        .bind(settings.ocr_enhance_contrast.unwrap_or(current.ocr_enhance_contrast))
        .bind(settings.ocr_remove_noise.unwrap_or(current.ocr_remove_noise))
        .bind(settings.ocr_detect_orientation.unwrap_or(current.ocr_detect_orientation))
        .bind(settings.ocr_whitelist_chars.as_ref().unwrap_or(&current.ocr_whitelist_chars))
        .bind(settings.ocr_blacklist_chars.as_ref().unwrap_or(&current.ocr_blacklist_chars))
        .bind(settings.ocr_brightness_boost.unwrap_or(current.ocr_brightness_boost))
        .bind(settings.ocr_contrast_multiplier.unwrap_or(current.ocr_contrast_multiplier))
        .bind(settings.ocr_noise_reduction_level.unwrap_or(current.ocr_noise_reduction_level))
        .bind(settings.ocr_sharpening_strength.unwrap_or(current.ocr_sharpening_strength))
        .bind(settings.ocr_morphological_operations.unwrap_or(current.ocr_morphological_operations))
        .bind(settings.ocr_adaptive_threshold_window_size.unwrap_or(current.ocr_adaptive_threshold_window_size))
        .bind(settings.ocr_histogram_equalization.unwrap_or(current.ocr_histogram_equalization))
        .bind(settings.ocr_upscale_factor.unwrap_or(current.ocr_upscale_factor))
        .bind(settings.ocr_max_image_width.unwrap_or(current.ocr_max_image_width))
        .bind(settings.ocr_max_image_height.unwrap_or(current.ocr_max_image_height))
        .bind(settings.save_processed_images.unwrap_or(current.save_processed_images))
        .bind(settings.ocr_quality_threshold_brightness.unwrap_or(current.ocr_quality_threshold_brightness))
        .bind(settings.ocr_quality_threshold_contrast.unwrap_or(current.ocr_quality_threshold_contrast))
        .bind(settings.ocr_quality_threshold_noise.unwrap_or(current.ocr_quality_threshold_noise))
        .bind(settings.ocr_quality_threshold_sharpness.unwrap_or(current.ocr_quality_threshold_sharpness))
        .bind(settings.ocr_skip_enhancement.unwrap_or(current.ocr_skip_enhancement))
        .bind(settings.webdav_enabled.unwrap_or(current.webdav_enabled))
        .bind(settings.webdav_server_url.as_ref().unwrap_or(&current.webdav_server_url))
        .bind(settings.webdav_username.as_ref().unwrap_or(&current.webdav_username))
        .bind(settings.webdav_password.as_ref().unwrap_or(&current.webdav_password))
        .bind(settings.webdav_watch_folders.as_ref().unwrap_or(&current.webdav_watch_folders))
        .bind(settings.webdav_file_extensions.as_ref().unwrap_or(&current.webdav_file_extensions))
        .bind(settings.webdav_auto_sync.unwrap_or(current.webdav_auto_sync))
        .bind(settings.webdav_sync_interval_minutes.unwrap_or(current.webdav_sync_interval_minutes))
        .fetch_one(&self.pool)
        .await?;

        Ok(crate::models::Settings {
            id: row.get("id"),
            user_id: row.get("user_id"),
            ocr_language: row.get("ocr_language"),
            concurrent_ocr_jobs: row.get("concurrent_ocr_jobs"),
            ocr_timeout_seconds: row.get("ocr_timeout_seconds"),
            max_file_size_mb: row.get("max_file_size_mb"),
            allowed_file_types: row.get("allowed_file_types"),
            auto_rotate_images: row.get("auto_rotate_images"),
            enable_image_preprocessing: row.get("enable_image_preprocessing"),
            search_results_per_page: row.get("search_results_per_page"),
            search_snippet_length: row.get("search_snippet_length"),
            fuzzy_search_threshold: row.get("fuzzy_search_threshold"),
            retention_days: row.get("retention_days"),
            enable_auto_cleanup: row.get("enable_auto_cleanup"),
            enable_compression: row.get("enable_compression"),
            memory_limit_mb: row.get("memory_limit_mb"),
            cpu_priority: row.get("cpu_priority"),
            enable_background_ocr: row.get("enable_background_ocr"),
            ocr_page_segmentation_mode: row.get("ocr_page_segmentation_mode"),
            ocr_engine_mode: row.get("ocr_engine_mode"),
            ocr_min_confidence: row.get("ocr_min_confidence"),
            ocr_dpi: row.get("ocr_dpi"),
            ocr_enhance_contrast: row.get("ocr_enhance_contrast"),
            ocr_remove_noise: row.get("ocr_remove_noise"),
            ocr_detect_orientation: row.get("ocr_detect_orientation"),
            ocr_whitelist_chars: row.get("ocr_whitelist_chars"),
            ocr_blacklist_chars: row.get("ocr_blacklist_chars"),
            ocr_brightness_boost: row.get("ocr_brightness_boost"),
            ocr_contrast_multiplier: row.get("ocr_contrast_multiplier"),
            ocr_noise_reduction_level: row.get("ocr_noise_reduction_level"),
            ocr_sharpening_strength: row.get("ocr_sharpening_strength"),
            ocr_morphological_operations: row.get("ocr_morphological_operations"),
            ocr_adaptive_threshold_window_size: row.get("ocr_adaptive_threshold_window_size"),
            ocr_histogram_equalization: row.get("ocr_histogram_equalization"),
            ocr_upscale_factor: row.get("ocr_upscale_factor"),
            ocr_max_image_width: row.get("ocr_max_image_width"),
            ocr_max_image_height: row.get("ocr_max_image_height"),
            save_processed_images: row.get("save_processed_images"),
            ocr_quality_threshold_brightness: row.get("ocr_quality_threshold_brightness"),
            ocr_quality_threshold_contrast: row.get("ocr_quality_threshold_contrast"),
            ocr_quality_threshold_noise: row.get("ocr_quality_threshold_noise"),
            ocr_quality_threshold_sharpness: row.get("ocr_quality_threshold_sharpness"),
            ocr_skip_enhancement: row.get("ocr_skip_enhancement"),
            webdav_enabled: row.get("webdav_enabled"),
            webdav_server_url: row.get("webdav_server_url"),
            webdav_username: row.get("webdav_username"),
            webdav_password: row.get("webdav_password"),
            webdav_watch_folders: row.get("webdav_watch_folders"),
            webdav_file_extensions: row.get("webdav_file_extensions"),
            webdav_auto_sync: row.get("webdav_auto_sync"),
            webdav_sync_interval_minutes: row.get("webdav_sync_interval_minutes"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }
}