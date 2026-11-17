use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use ts_rs::TS;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, FromRow, ToSchema, TS)]
#[ts(export)]
pub struct Settings {
    pub id: Uuid,
    pub user_id: Uuid,
    pub ocr_language: String,
    pub preferred_languages: Vec<String>,
    pub primary_language: String,
    /// Longest one of this user's documents may spend in OCR before the job
    /// fails as timed out (see `crate::ocr::user_settings`).
    pub ocr_timeout_seconds: i32,
    /// Largest file this user may upload, in MB; `None` means the server's
    /// `MAX_FILE_SIZE_MB`, which also caps any value set here (see
    /// `crate::ocr::user_settings::max_upload_bytes`).
    pub max_file_size_mb: Option<i32>,
    pub auto_rotate_images: bool,
    pub enable_image_preprocessing: bool,
    pub fuzzy_search_threshold: f32,
    /// Queue this user's uploads and synced files for OCR automatically.
    pub enable_background_ocr: bool,
    /// Tesseract page segmentation mode (0-13), for images and scanned PDFs.
    pub ocr_page_segmentation_mode: i32,
    /// Tesseract engine mode (0 legacy, 1 LSTM, 2 both, 3 default).
    pub ocr_engine_mode: i32,
    pub ocr_min_confidence: f32,
    /// Resolution PDF pages are rendered at and images are assumed to have.
    pub ocr_dpi: i32,
    pub ocr_enhance_contrast: bool,
    pub ocr_remove_noise: bool,
    pub ocr_detect_orientation: bool,
    pub ocr_whitelist_chars: Option<String>,
    pub ocr_blacklist_chars: Option<String>,
    pub ocr_brightness_boost: f32,
    pub ocr_contrast_multiplier: f32,
    pub ocr_noise_reduction_level: i32,
    pub ocr_sharpening_strength: f32,
    pub ocr_morphological_operations: bool,
    pub ocr_adaptive_threshold_window_size: i32,
    pub ocr_histogram_equalization: bool,
    pub ocr_upscale_factor: f32,
    pub ocr_max_image_width: i32,
    pub ocr_max_image_height: i32,
    pub save_processed_images: bool,
    pub ocr_quality_threshold_brightness: f32,
    pub ocr_quality_threshold_contrast: f32,
    pub ocr_quality_threshold_noise: f32,
    pub ocr_quality_threshold_sharpness: f32,
    pub ocr_skip_enhancement: bool,
    pub webdav_enabled: bool,
    pub webdav_server_url: Option<String>,
    pub webdav_username: Option<String>,
    pub webdav_password: Option<String>,
    pub webdav_watch_folders: Vec<String>,
    pub webdav_file_extensions: Vec<String>,
    pub webdav_auto_sync: bool,
    pub webdav_sync_interval_minutes: i32,
    /// Longest an Office document extraction may take, in seconds.
    pub office_extraction_timeout_seconds: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct SettingsResponse {
    pub ocr_language: String,
    pub preferred_languages: Vec<String>,
    pub primary_language: String,
    pub ocr_timeout_seconds: i32,
    /// The user's upload limit in MB; null means the server's limit.
    pub max_file_size_mb: Option<i32>,
    pub auto_rotate_images: bool,
    pub enable_image_preprocessing: bool,
    pub fuzzy_search_threshold: f32,
    pub enable_background_ocr: bool,
    pub ocr_page_segmentation_mode: i32,
    pub ocr_engine_mode: i32,
    pub ocr_min_confidence: f32,
    pub ocr_dpi: i32,
    pub ocr_enhance_contrast: bool,
    pub ocr_remove_noise: bool,
    pub ocr_detect_orientation: bool,
    pub ocr_whitelist_chars: Option<String>,
    pub ocr_blacklist_chars: Option<String>,
    pub ocr_brightness_boost: f32,
    pub ocr_contrast_multiplier: f32,
    pub ocr_noise_reduction_level: i32,
    pub ocr_sharpening_strength: f32,
    pub ocr_morphological_operations: bool,
    pub ocr_adaptive_threshold_window_size: i32,
    pub ocr_histogram_equalization: bool,
    pub ocr_upscale_factor: f32,
    pub ocr_max_image_width: i32,
    pub ocr_max_image_height: i32,
    pub save_processed_images: bool,
    pub ocr_quality_threshold_brightness: f32,
    pub ocr_quality_threshold_contrast: f32,
    pub ocr_quality_threshold_noise: f32,
    pub ocr_quality_threshold_sharpness: f32,
    pub ocr_skip_enhancement: bool,
    pub webdav_enabled: bool,
    pub webdav_server_url: Option<String>,
    pub webdav_username: Option<String>,
    /// Whether a WebDAV password is stored. The password itself is never returned.
    pub has_webdav_password: bool,
    pub webdav_watch_folders: Vec<String>,
    pub webdav_file_extensions: Vec<String>,
    pub webdav_auto_sync: bool,
    pub webdav_sync_interval_minutes: i32,
    // Office document extraction configuration
    pub office_extraction_timeout_seconds: i32,
}

#[derive(Default, Serialize, Deserialize, ToSchema, TS)]
#[ts(export, optional_fields)]
pub struct UpdateSettings {
    pub ocr_language: Option<String>,
    pub preferred_languages: Option<Vec<String>>,
    pub primary_language: Option<String>,
    pub ocr_timeout_seconds: Option<i32>,
    /// `Some(None)` (JSON null) returns the user to the server's limit.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "nullable"
    )]
    pub max_file_size_mb: Option<Option<i32>>,
    pub auto_rotate_images: Option<bool>,
    pub enable_image_preprocessing: Option<bool>,
    pub fuzzy_search_threshold: Option<f32>,
    pub enable_background_ocr: Option<bool>,
    pub ocr_page_segmentation_mode: Option<i32>,
    pub ocr_engine_mode: Option<i32>,
    pub ocr_min_confidence: Option<f32>,
    pub ocr_dpi: Option<i32>,
    pub ocr_enhance_contrast: Option<bool>,
    pub ocr_remove_noise: Option<bool>,
    pub ocr_detect_orientation: Option<bool>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "nullable"
    )]
    pub ocr_whitelist_chars: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "nullable"
    )]
    pub ocr_blacklist_chars: Option<Option<String>>,
    pub ocr_brightness_boost: Option<f32>,
    pub ocr_contrast_multiplier: Option<f32>,
    pub ocr_noise_reduction_level: Option<i32>,
    pub ocr_sharpening_strength: Option<f32>,
    pub ocr_morphological_operations: Option<bool>,
    pub ocr_adaptive_threshold_window_size: Option<i32>,
    pub ocr_histogram_equalization: Option<bool>,
    pub ocr_upscale_factor: Option<f32>,
    pub ocr_max_image_width: Option<i32>,
    pub ocr_max_image_height: Option<i32>,
    pub save_processed_images: Option<bool>,
    pub ocr_quality_threshold_brightness: Option<f32>,
    pub ocr_quality_threshold_contrast: Option<f32>,
    pub ocr_quality_threshold_noise: Option<f32>,
    pub ocr_quality_threshold_sharpness: Option<f32>,
    pub ocr_skip_enhancement: Option<bool>,
    pub webdav_enabled: Option<bool>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "nullable"
    )]
    pub webdav_server_url: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "nullable"
    )]
    pub webdav_username: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "nullable"
    )]
    pub webdav_password: Option<Option<String>>,
    pub webdav_watch_folders: Option<Vec<String>>,
    pub webdav_file_extensions: Option<Vec<String>>,
    pub webdav_auto_sync: Option<bool>,
    pub webdav_sync_interval_minutes: Option<i32>,
    // Office document extraction configuration
    pub office_extraction_timeout_seconds: Option<i32>,
}

/// Reads a field of [`UpdateSettings`] that can be cleared: JSON `null`
/// becomes `Some(None)` (clear it), while a missing field stays `None`
/// (leave it unchanged) through `#[serde(default)]`.
fn nullable<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

impl std::fmt::Debug for Settings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Settings")
            .field("id", &self.id)
            .field("user_id", &self.user_id)
            .field("ocr_language", &self.ocr_language)
            .field("webdav_enabled", &self.webdav_enabled)
            .field("webdav_server_url", &self.webdav_server_url)
            .field("webdav_username", &self.webdav_username)
            .field(
                "webdav_password",
                &self.webdav_password.as_ref().map(|_| "***"),
            )
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for UpdateSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UpdateSettings")
            .field("ocr_language", &self.ocr_language)
            .field("webdav_enabled", &self.webdav_enabled)
            .field("webdav_server_url", &self.webdav_server_url)
            .field("webdav_username", &self.webdav_username)
            .field(
                "webdav_password",
                &self
                    .webdav_password
                    .as_ref()
                    .map(|p| p.as_ref().map(|_| "***")),
            )
            .finish_non_exhaustive()
    }
}

impl From<Settings> for SettingsResponse {
    fn from(settings: Settings) -> Self {
        Self {
            ocr_language: settings.ocr_language,
            preferred_languages: settings.preferred_languages,
            primary_language: settings.primary_language,
            ocr_timeout_seconds: settings.ocr_timeout_seconds,
            max_file_size_mb: settings.max_file_size_mb,
            auto_rotate_images: settings.auto_rotate_images,
            enable_image_preprocessing: settings.enable_image_preprocessing,
            fuzzy_search_threshold: settings.fuzzy_search_threshold,
            enable_background_ocr: settings.enable_background_ocr,
            ocr_page_segmentation_mode: settings.ocr_page_segmentation_mode,
            ocr_engine_mode: settings.ocr_engine_mode,
            ocr_min_confidence: settings.ocr_min_confidence,
            ocr_dpi: settings.ocr_dpi,
            ocr_enhance_contrast: settings.ocr_enhance_contrast,
            ocr_remove_noise: settings.ocr_remove_noise,
            ocr_detect_orientation: settings.ocr_detect_orientation,
            ocr_whitelist_chars: settings.ocr_whitelist_chars,
            ocr_blacklist_chars: settings.ocr_blacklist_chars,
            ocr_brightness_boost: settings.ocr_brightness_boost,
            ocr_contrast_multiplier: settings.ocr_contrast_multiplier,
            ocr_noise_reduction_level: settings.ocr_noise_reduction_level,
            ocr_sharpening_strength: settings.ocr_sharpening_strength,
            ocr_morphological_operations: settings.ocr_morphological_operations,
            ocr_adaptive_threshold_window_size: settings.ocr_adaptive_threshold_window_size,
            ocr_histogram_equalization: settings.ocr_histogram_equalization,
            ocr_upscale_factor: settings.ocr_upscale_factor,
            ocr_max_image_width: settings.ocr_max_image_width,
            ocr_max_image_height: settings.ocr_max_image_height,
            save_processed_images: settings.save_processed_images,
            ocr_quality_threshold_brightness: settings.ocr_quality_threshold_brightness,
            ocr_quality_threshold_contrast: settings.ocr_quality_threshold_contrast,
            ocr_quality_threshold_noise: settings.ocr_quality_threshold_noise,
            ocr_quality_threshold_sharpness: settings.ocr_quality_threshold_sharpness,
            ocr_skip_enhancement: settings.ocr_skip_enhancement,
            webdav_enabled: settings.webdav_enabled,
            webdav_server_url: settings.webdav_server_url,
            webdav_username: settings.webdav_username,
            has_webdav_password: settings
                .webdav_password
                .as_deref()
                .is_some_and(|p| !p.is_empty()),
            webdav_watch_folders: settings.webdav_watch_folders,
            webdav_file_extensions: settings.webdav_file_extensions,
            webdav_auto_sync: settings.webdav_auto_sync,
            webdav_sync_interval_minutes: settings.webdav_sync_interval_minutes,
            // Office document extraction configuration
            office_extraction_timeout_seconds: settings.office_extraction_timeout_seconds,
        }
    }
}

impl UpdateSettings {
    /// Create an UpdateSettings that only updates language preferences
    pub fn language_update(
        preferred_languages: Vec<String>,
        primary_language: String,
        ocr_language: String,
    ) -> Self {
        Self {
            preferred_languages: Some(preferred_languages),
            primary_language: Some(primary_language),
            ocr_language: Some(ocr_language),
            ..Self::default()
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id: Uuid::nil(),
            ocr_language: "eng".to_string(),
            preferred_languages: vec!["eng".to_string()],
            primary_language: "eng".to_string(),
            ocr_timeout_seconds: 300,
            max_file_size_mb: None, // the server's MAX_FILE_SIZE_MB
            auto_rotate_images: true,
            enable_image_preprocessing: false,
            fuzzy_search_threshold: crate::models::FuzzyThreshold::DEFAULT,
            enable_background_ocr: true,
            ocr_page_segmentation_mode: 3, // PSM_AUTO_OSD - Fully automatic page segmentation, but no OSD
            ocr_engine_mode: 3,            // OEM_DEFAULT - Default, based on what is available
            ocr_min_confidence: 30.0,      // Minimum confidence threshold (0-100)
            ocr_dpi: 300,                  // Optimal DPI for OCR
            ocr_enhance_contrast: true,    // Enable contrast enhancement
            ocr_remove_noise: true,        // Enable noise removal
            ocr_detect_orientation: true,  // Enable orientation detection
            ocr_whitelist_chars: None,     // No character whitelist by default
            ocr_blacklist_chars: None,     // No character blacklist by default
            ocr_brightness_boost: 1.0,     // Conservative brightness boost
            ocr_contrast_multiplier: 1.2,  // Conservative contrast enhancement
            ocr_noise_reduction_level: 1,  // Light noise reduction
            ocr_sharpening_strength: 0.5,  // Light sharpening
            ocr_morphological_operations: false, // Conservative - no morphological ops by default
            ocr_adaptive_threshold_window_size: 15, // Small window for adaptive threshold
            ocr_histogram_equalization: false, // Conservative - no histogram equalization by default
            ocr_upscale_factor: 1.0,           // No upscaling by default
            ocr_max_image_width: 3000,         // Reasonable max width
            ocr_max_image_height: 3000,        // Reasonable max height
            save_processed_images: false,      // Conservative - don't save by default
            ocr_quality_threshold_brightness: 0.3, // Conservative threshold
            ocr_quality_threshold_contrast: 0.2, // Conservative threshold
            ocr_quality_threshold_noise: 0.7,  // Conservative threshold
            ocr_quality_threshold_sharpness: 0.3, // Conservative threshold
            ocr_skip_enhancement: false,       // Allow enhancement by default
            webdav_enabled: false,
            webdav_server_url: None,
            webdav_username: None,
            webdav_password: None,
            webdav_watch_folders: vec!["/Documents".to_string()],
            webdav_file_extensions: vec![
                "pdf".to_string(),
                "png".to_string(),
                "jpg".to_string(),
                "jpeg".to_string(),
                "tiff".to_string(),
                "bmp".to_string(),
                "txt".to_string(),
            ],
            webdav_auto_sync: false,
            webdav_sync_interval_minutes: 60,
            // Office document extraction configuration defaults
            office_extraction_timeout_seconds: 120, // 2 minutes default timeout
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_clears_a_field_and_a_missing_field_keeps_it() {
        let cleared: UpdateSettings =
            serde_json::from_str(r#"{"max_file_size_mb": null, "ocr_whitelist_chars": null}"#)
                .unwrap();
        assert_eq!(cleared.max_file_size_mb, Some(None));
        assert_eq!(cleared.ocr_whitelist_chars, Some(None));
        assert_eq!(cleared.webdav_password, None);

        let set: UpdateSettings = serde_json::from_str(r#"{"max_file_size_mb": 20}"#).unwrap();
        assert_eq!(set.max_file_size_mb, Some(Some(20)));
        assert_eq!(set.ocr_whitelist_chars, None);
    }

    #[test]
    fn unchanged_fields_are_left_out_when_serialized() {
        let json = serde_json::to_value(UpdateSettings {
            ocr_dpi: Some(200),
            ..UpdateSettings::default()
        })
        .unwrap();
        assert!(json.get("max_file_size_mb").is_none(), "{json}");
        assert!(json.get("webdav_password").is_none(), "{json}");
    }
}
