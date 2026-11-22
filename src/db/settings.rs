use anyhow::{anyhow, Result};
use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

use super::sanitize::{DbJson, ToDb};
use super::Database;

// Helper function to parse JSONB array to Vec<String>
fn parse_jsonb_string_array(value: Value) -> Vec<String> {
    match value {
        Value::Array(arr) => arr
            .into_iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        _ => vec!["eng".to_string()], // fallback to English
    }
}

/// Every column `settings_from_row` reads, with the fallbacks for columns
/// added after the table was created. A macro so queries can `concat!` it
/// into a static string.
macro_rules! settings_columns {
    () => {
        r#"id, user_id, ocr_language,
    COALESCE(preferred_languages, '["eng"]'::jsonb) as preferred_languages,
    COALESCE(primary_language, 'eng') as primary_language,
    ocr_timeout_seconds, max_file_size_mb, auto_rotate_images, enable_image_preprocessing,
    fuzzy_search_threshold, enable_background_ocr, ocr_page_segmentation_mode, ocr_engine_mode,
    ocr_min_confidence, ocr_dpi, ocr_enhance_contrast, ocr_remove_noise,
    ocr_detect_orientation, ocr_whitelist_chars, ocr_blacklist_chars,
    ocr_brightness_boost, ocr_contrast_multiplier, ocr_noise_reduction_level, ocr_sharpening_strength,
    ocr_morphological_operations, ocr_adaptive_threshold_window_size, ocr_histogram_equalization,
    ocr_upscale_factor, ocr_max_image_width, ocr_max_image_height, save_processed_images,
    ocr_quality_threshold_brightness, ocr_quality_threshold_contrast, ocr_quality_threshold_noise,
    ocr_quality_threshold_sharpness, ocr_skip_enhancement,
    webdav_enabled, webdav_server_url, webdav_username, webdav_password,
    webdav_watch_folders, webdav_file_extensions, webdav_auto_sync, webdav_sync_interval_minutes,
    COALESCE(office_extraction_timeout_seconds, 120) as office_extraction_timeout_seconds,
    created_at, updated_at"#
    };
}

// Helper function to create Settings from database row
fn settings_from_row(row: &sqlx::postgres::PgRow) -> crate::models::Settings {
    let preferred_languages_json: Value = row.get("preferred_languages");
    let preferred_languages = parse_jsonb_string_array(preferred_languages_json);

    crate::models::Settings {
        id: row.get("id"),
        user_id: row.get("user_id"),
        ocr_language: row.get("ocr_language"),
        preferred_languages,
        primary_language: row.get("primary_language"),
        ocr_timeout_seconds: row.get("ocr_timeout_seconds"),
        max_file_size_mb: row.get("max_file_size_mb"),
        auto_rotate_images: row.get("auto_rotate_images"),
        enable_image_preprocessing: row.get("enable_image_preprocessing"),
        fuzzy_search_threshold: row.get("fuzzy_search_threshold"),
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
        // Office document extraction configuration
        office_extraction_timeout_seconds: row.get("office_extraction_timeout_seconds"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

impl Database {
    pub async fn get_user_settings(
        &self,
        user_id: Uuid,
    ) -> Result<Option<crate::models::Settings>> {
        self.with_retry(|| async {
            let row = sqlx::query(concat!(
                "SELECT ",
                settings_columns!(),
                " FROM settings WHERE user_id = $1"
            ))
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Database query failed: {}", e))?;

            match row {
                Some(row) => Ok(Some(settings_from_row(&row))),
                None => Ok(None),
            }
        })
        .await
    }

    /// The user's stored fuzzy search threshold: `None` when the user has no
    /// settings row or the column is NULL. Read on its own (not through
    /// `get_user_settings`) because fuzzy search needs only this value.
    pub async fn get_fuzzy_search_threshold(&self, user_id: Uuid) -> Result<Option<f32>> {
        let threshold: Option<Option<f32>> =
            sqlx::query_scalar("SELECT fuzzy_search_threshold FROM settings WHERE user_id = $1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(threshold.flatten())
    }

    pub async fn get_all_user_settings(&self) -> Result<Vec<crate::models::Settings>> {
        let rows = sqlx::query(concat!(
            "SELECT ",
            settings_columns!(),
            " FROM settings WHERE webdav_enabled = true AND webdav_auto_sync = true"
        ))
        .fetch_all(&self.pool)
        .await?;

        let settings_list = rows
            .into_iter()
            .map(|row| settings_from_row(&row))
            .collect();

        Ok(settings_list)
    }

    /// Reject values the OCR pipeline, upload limit or scheduler cannot use.
    /// Every bound matches the field's range in the settings UI.
    fn validate_settings_constraints(settings: &crate::models::UpdateSettings) -> Result<()> {
        fn check<T: PartialOrd + std::fmt::Display + Copy>(
            value: Option<T>,
            range: std::ops::RangeInclusive<T>,
            what: &str,
        ) -> Result<()> {
            match value {
                Some(v) if !range.contains(&v) => Err(anyhow!(
                    "{} must be between {} and {}, got: {}",
                    what,
                    range.start(),
                    range.end(),
                    v
                )),
                _ => Ok(()),
            }
        }
        check(
            settings.ocr_timeout_seconds,
            10..=crate::ocr::user_settings::MAX_OCR_TIMEOUT_SECONDS,
            "OCR timeout (seconds)",
        )?;
        check(
            settings.max_file_size_mb.flatten(),
            1..=500,
            "Maximum file size (MB)",
        )?;
        check(
            settings.fuzzy_search_threshold,
            0.0..=1.0,
            "Fuzzy search threshold",
        )?;
        check(
            settings.ocr_page_segmentation_mode,
            0..=13,
            "OCR page segmentation mode",
        )?;
        check(settings.ocr_engine_mode, 0..=3, "OCR engine mode")?;
        check(settings.ocr_dpi, 70..=1200, "OCR DPI")?;
        check(
            settings.ocr_min_confidence,
            0.0..=100.0,
            "OCR minimum confidence",
        )?;
        check(settings.ocr_upscale_factor, 0.1..=5.0, "OCR upscale factor")?;
        check(
            settings.ocr_max_image_width,
            100..=50000,
            "OCR maximum image width",
        )?;
        check(
            settings.ocr_max_image_height,
            100..=50000,
            "OCR maximum image height",
        )?;
        // Office extraction is capped at 10 minutes for system stability.
        check(
            settings.office_extraction_timeout_seconds,
            1..=600,
            "Office extraction timeout (seconds)",
        )?;
        check(
            settings.webdav_sync_interval_minutes,
            1..=10080,
            "WebDAV sync interval (minutes)",
        )?;
        Ok(())
    }

    pub async fn create_or_update_settings(
        &self,
        user_id: Uuid,
        settings: &crate::models::UpdateSettings,
    ) -> Result<crate::models::Settings> {
        // Validate settings before saving
        Self::validate_settings_constraints(settings)?;
        // Get existing settings to merge with updates
        let existing = self.get_user_settings(user_id).await?;
        let defaults = crate::models::Settings::default();

        // Merge existing/defaults with updates
        let current = existing.unwrap_or_else(|| {
            let mut s = defaults;
            s.user_id = user_id;
            s
        });

        let row = sqlx::query(concat!(
            r#"
            INSERT INTO settings (
                user_id, ocr_language, preferred_languages, primary_language, ocr_timeout_seconds,
                max_file_size_mb, auto_rotate_images, enable_image_preprocessing,
                fuzzy_search_threshold, enable_background_ocr, ocr_page_segmentation_mode,
                ocr_engine_mode, ocr_min_confidence, ocr_dpi, ocr_enhance_contrast,
                ocr_remove_noise, ocr_detect_orientation, ocr_whitelist_chars, ocr_blacklist_chars,
                ocr_brightness_boost, ocr_contrast_multiplier, ocr_noise_reduction_level,
                ocr_sharpening_strength, ocr_morphological_operations,
                ocr_adaptive_threshold_window_size, ocr_histogram_equalization, ocr_upscale_factor,
                ocr_max_image_width, ocr_max_image_height, save_processed_images,
                ocr_quality_threshold_brightness, ocr_quality_threshold_contrast,
                ocr_quality_threshold_noise, ocr_quality_threshold_sharpness, ocr_skip_enhancement,
                webdav_enabled, webdav_server_url, webdav_username, webdav_password,
                webdav_watch_folders, webdav_file_extensions, webdav_auto_sync,
                webdav_sync_interval_minutes, office_extraction_timeout_seconds
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18,
                $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, $33, $34, $35,
                $36, $37, $38, $39, $40, $41, $42, $43, $44)
            ON CONFLICT (user_id) DO UPDATE SET
                ocr_language = $2,
                preferred_languages = $3,
                primary_language = $4,
                ocr_timeout_seconds = $5,
                max_file_size_mb = $6,
                auto_rotate_images = $7,
                enable_image_preprocessing = $8,
                fuzzy_search_threshold = $9,
                enable_background_ocr = $10,
                ocr_page_segmentation_mode = $11,
                ocr_engine_mode = $12,
                ocr_min_confidence = $13,
                ocr_dpi = $14,
                ocr_enhance_contrast = $15,
                ocr_remove_noise = $16,
                ocr_detect_orientation = $17,
                ocr_whitelist_chars = $18,
                ocr_blacklist_chars = $19,
                ocr_brightness_boost = $20,
                ocr_contrast_multiplier = $21,
                ocr_noise_reduction_level = $22,
                ocr_sharpening_strength = $23,
                ocr_morphological_operations = $24,
                ocr_adaptive_threshold_window_size = $25,
                ocr_histogram_equalization = $26,
                ocr_upscale_factor = $27,
                ocr_max_image_width = $28,
                ocr_max_image_height = $29,
                save_processed_images = $30,
                ocr_quality_threshold_brightness = $31,
                ocr_quality_threshold_contrast = $32,
                ocr_quality_threshold_noise = $33,
                ocr_quality_threshold_sharpness = $34,
                ocr_skip_enhancement = $35,
                webdav_enabled = $36,
                webdav_server_url = $37,
                webdav_username = $38,
                webdav_password = $39,
                webdav_watch_folders = $40,
                webdav_file_extensions = $41,
                webdav_auto_sync = $42,
                webdav_sync_interval_minutes = $43,
                office_extraction_timeout_seconds = $44,
                updated_at = NOW()
            RETURNING "#,
            settings_columns!()
        ))
        .bind(user_id)
        .bind(
            settings
                .ocr_language
                .as_ref()
                .unwrap_or(&current.ocr_language)
                .for_db(),
        )
        .bind(DbJson(
            settings
                .preferred_languages
                .as_ref()
                .unwrap_or(&current.preferred_languages),
        ))
        .bind(
            settings
                .primary_language
                .as_ref()
                .unwrap_or(&current.primary_language)
                .for_db(),
        )
        .bind(
            settings
                .ocr_timeout_seconds
                .unwrap_or(current.ocr_timeout_seconds),
        )
        .bind(
            settings
                .max_file_size_mb
                .unwrap_or(current.max_file_size_mb),
        )
        .bind(
            settings
                .auto_rotate_images
                .unwrap_or(current.auto_rotate_images),
        )
        .bind(
            settings
                .enable_image_preprocessing
                .unwrap_or(current.enable_image_preprocessing),
        )
        .bind(
            settings
                .fuzzy_search_threshold
                .unwrap_or(current.fuzzy_search_threshold),
        )
        .bind(
            settings
                .enable_background_ocr
                .unwrap_or(current.enable_background_ocr),
        )
        .bind(
            settings
                .ocr_page_segmentation_mode
                .unwrap_or(current.ocr_page_segmentation_mode),
        )
        .bind(settings.ocr_engine_mode.unwrap_or(current.ocr_engine_mode))
        .bind(
            settings
                .ocr_min_confidence
                .unwrap_or(current.ocr_min_confidence),
        )
        .bind(settings.ocr_dpi.unwrap_or(current.ocr_dpi))
        .bind(
            settings
                .ocr_enhance_contrast
                .unwrap_or(current.ocr_enhance_contrast),
        )
        .bind(
            settings
                .ocr_remove_noise
                .unwrap_or(current.ocr_remove_noise),
        )
        .bind(
            settings
                .ocr_detect_orientation
                .unwrap_or(current.ocr_detect_orientation),
        )
        .bind(
            settings
                .ocr_whitelist_chars
                .as_ref()
                .unwrap_or(&current.ocr_whitelist_chars)
                .for_db(),
        )
        .bind(
            settings
                .ocr_blacklist_chars
                .as_ref()
                .unwrap_or(&current.ocr_blacklist_chars)
                .for_db(),
        )
        .bind(
            settings
                .ocr_brightness_boost
                .unwrap_or(current.ocr_brightness_boost),
        )
        .bind(
            settings
                .ocr_contrast_multiplier
                .unwrap_or(current.ocr_contrast_multiplier),
        )
        .bind(
            settings
                .ocr_noise_reduction_level
                .unwrap_or(current.ocr_noise_reduction_level),
        )
        .bind(
            settings
                .ocr_sharpening_strength
                .unwrap_or(current.ocr_sharpening_strength),
        )
        .bind(
            settings
                .ocr_morphological_operations
                .unwrap_or(current.ocr_morphological_operations),
        )
        .bind(
            settings
                .ocr_adaptive_threshold_window_size
                .unwrap_or(current.ocr_adaptive_threshold_window_size),
        )
        .bind(
            settings
                .ocr_histogram_equalization
                .unwrap_or(current.ocr_histogram_equalization),
        )
        .bind(
            settings
                .ocr_upscale_factor
                .unwrap_or(current.ocr_upscale_factor),
        )
        .bind(
            settings
                .ocr_max_image_width
                .unwrap_or(current.ocr_max_image_width),
        )
        .bind(
            settings
                .ocr_max_image_height
                .unwrap_or(current.ocr_max_image_height),
        )
        .bind(
            settings
                .save_processed_images
                .unwrap_or(current.save_processed_images),
        )
        .bind(
            settings
                .ocr_quality_threshold_brightness
                .unwrap_or(current.ocr_quality_threshold_brightness),
        )
        .bind(
            settings
                .ocr_quality_threshold_contrast
                .unwrap_or(current.ocr_quality_threshold_contrast),
        )
        .bind(
            settings
                .ocr_quality_threshold_noise
                .unwrap_or(current.ocr_quality_threshold_noise),
        )
        .bind(
            settings
                .ocr_quality_threshold_sharpness
                .unwrap_or(current.ocr_quality_threshold_sharpness),
        )
        .bind(
            settings
                .ocr_skip_enhancement
                .unwrap_or(current.ocr_skip_enhancement),
        )
        .bind(settings.webdav_enabled.unwrap_or(current.webdav_enabled))
        .bind(
            settings
                .webdav_server_url
                .as_ref()
                .unwrap_or(&current.webdav_server_url)
                .for_db(),
        )
        .bind(
            settings
                .webdav_username
                .as_ref()
                .unwrap_or(&current.webdav_username)
                .for_db(),
        )
        .bind(
            settings
                .webdav_password
                .as_ref()
                .unwrap_or(&current.webdav_password)
                .for_db(),
        )
        .bind(
            settings
                .webdav_watch_folders
                .as_ref()
                .unwrap_or(&current.webdav_watch_folders)
                .for_db(),
        )
        .bind(
            settings
                .webdav_file_extensions
                .as_ref()
                .unwrap_or(&current.webdav_file_extensions)
                .for_db(),
        )
        .bind(
            settings
                .webdav_auto_sync
                .unwrap_or(current.webdav_auto_sync),
        )
        .bind(
            settings
                .webdav_sync_interval_minutes
                .unwrap_or(current.webdav_sync_interval_minutes),
        )
        .bind(
            settings
                .office_extraction_timeout_seconds
                .unwrap_or(current.office_extraction_timeout_seconds),
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(settings_from_row(&row))
    }

    pub async fn update_user_ocr_language(&self, user_id: Uuid, language: &str) -> Result<()> {
        self.with_retry(|| async {
            sqlx::query(
                r#"
                INSERT INTO settings (user_id, ocr_language)
                VALUES ($1, $2)
                ON CONFLICT (user_id) DO UPDATE SET
                    ocr_language = $2,
                    updated_at = NOW()
                "#,
            )
            .bind(user_id)
            .bind(language.for_db())
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to update OCR language: {}", e))?;

            Ok(())
        })
        .await
    }
}
