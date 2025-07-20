use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use std::sync::Arc;

use crate::{
    auth::AuthUser,
    errors::settings::SettingsError,
    models::{SettingsResponse, UpdateSettings, UserRole},
    AppState,
};
use serde::Serialize;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_settings).put(update_settings))
        .route("/config", get(get_server_configuration))
}

#[utoipa::path(
    get,
    path = "/api/settings",
    tag = "settings",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "User settings", body = SettingsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_settings(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<SettingsResponse>, SettingsError> {
    let settings = state
        .db
        .get_user_settings(auth_user.user.id)
        .await
        .map_err(|e| SettingsError::invalid_value("database", &format!("Failed to fetch settings: {}", e), "Settings must be accessible"))?;

    let response = match settings {
        Some(s) => s.into(),
        None => {
            let default = crate::models::Settings::default();
            SettingsResponse {
                ocr_language: default.ocr_language,
                preferred_languages: default.preferred_languages,
                primary_language: default.primary_language,
                auto_detect_language_combination: default.auto_detect_language_combination,
                concurrent_ocr_jobs: default.concurrent_ocr_jobs,
                ocr_timeout_seconds: default.ocr_timeout_seconds,
                max_file_size_mb: default.max_file_size_mb,
                allowed_file_types: default.allowed_file_types,
                auto_rotate_images: default.auto_rotate_images,
                enable_image_preprocessing: default.enable_image_preprocessing,
                search_results_per_page: default.search_results_per_page,
                search_snippet_length: default.search_snippet_length,
                fuzzy_search_threshold: default.fuzzy_search_threshold,
                retention_days: default.retention_days,
                enable_auto_cleanup: default.enable_auto_cleanup,
                enable_compression: default.enable_compression,
                memory_limit_mb: default.memory_limit_mb,
                cpu_priority: default.cpu_priority,
                enable_background_ocr: default.enable_background_ocr,
                ocr_page_segmentation_mode: default.ocr_page_segmentation_mode,
                ocr_engine_mode: default.ocr_engine_mode,
                ocr_min_confidence: default.ocr_min_confidence,
                ocr_dpi: default.ocr_dpi,
                ocr_enhance_contrast: default.ocr_enhance_contrast,
                ocr_remove_noise: default.ocr_remove_noise,
                ocr_detect_orientation: default.ocr_detect_orientation,
                ocr_whitelist_chars: default.ocr_whitelist_chars,
                ocr_blacklist_chars: default.ocr_blacklist_chars,
                ocr_brightness_boost: default.ocr_brightness_boost,
                ocr_contrast_multiplier: default.ocr_contrast_multiplier,
                ocr_noise_reduction_level: default.ocr_noise_reduction_level,
                ocr_sharpening_strength: default.ocr_sharpening_strength,
                ocr_morphological_operations: default.ocr_morphological_operations,
                ocr_adaptive_threshold_window_size: default.ocr_adaptive_threshold_window_size,
                ocr_histogram_equalization: default.ocr_histogram_equalization,
                ocr_upscale_factor: default.ocr_upscale_factor,
                ocr_max_image_width: default.ocr_max_image_width,
                ocr_max_image_height: default.ocr_max_image_height,
                save_processed_images: default.save_processed_images,
                ocr_quality_threshold_brightness: default.ocr_quality_threshold_brightness,
                ocr_quality_threshold_contrast: default.ocr_quality_threshold_contrast,
                ocr_quality_threshold_noise: default.ocr_quality_threshold_noise,
                ocr_quality_threshold_sharpness: default.ocr_quality_threshold_sharpness,
                ocr_skip_enhancement: default.ocr_skip_enhancement,
                webdav_enabled: default.webdav_enabled,
                webdav_server_url: default.webdav_server_url,
                webdav_username: default.webdav_username,
                webdav_password: default.webdav_password,
                webdav_watch_folders: default.webdav_watch_folders,
                webdav_file_extensions: default.webdav_file_extensions,
                webdav_auto_sync: default.webdav_auto_sync,
                webdav_sync_interval_minutes: default.webdav_sync_interval_minutes,
            }
        },
    };

    Ok(Json(response))
}

#[utoipa::path(
    put,
    path = "/api/settings",
    tag = "settings",
    security(
        ("bearer_auth" = [])
    ),
    request_body = UpdateSettings,
    responses(
        (status = 200, description = "Settings updated successfully", body = SettingsResponse),
        (status = 400, description = "Bad request - invalid settings data"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn update_settings(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(update_data): Json<UpdateSettings>,
) -> Result<Json<SettingsResponse>, StatusCode> {
    let settings = state
        .db
        .create_or_update_settings(auth_user.user.id, &update_data)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Json(settings.into()))
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
struct ServerConfiguration {
    max_file_size_mb: u64,
    concurrent_ocr_jobs: i32,
    ocr_timeout_seconds: i32,
    memory_limit_mb: u64,
    cpu_priority: String,
    server_host: String,
    server_port: u16,
    jwt_secret_set: bool,
    upload_path: String,
    watch_folder: Option<String>,
    ocr_language: String,
    allowed_file_types: Vec<String>,
    watch_interval_seconds: Option<u64>,
    file_stability_check_ms: Option<u64>,
    max_file_age_hours: Option<u64>,
    enable_background_ocr: bool,
    version: String,
    build_info: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/settings/config",
    tag = "settings",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Server configuration", body = ServerConfiguration),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin access required"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_server_configuration(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ServerConfiguration>, StatusCode> {
    // Only allow admin users to view server configuration
    if auth_user.user.role != UserRole::Admin {
        return Err(StatusCode::FORBIDDEN);
    }

    let config = &state.config;
    
    // Get default settings for reference
    let default_settings = crate::models::Settings::default();
    
    // Parse server_address to get host and port
    let (server_host, server_port) = if let Some(colon_pos) = config.server_address.rfind(':') {
        let host = config.server_address[..colon_pos].to_string();
        let port = config.server_address[colon_pos + 1..].parse::<u16>().unwrap_or(8000);
        (host, port)
    } else {
        (config.server_address.clone(), 8000)
    };
    
    let server_config = ServerConfiguration {
        max_file_size_mb: config.max_file_size_mb,
        concurrent_ocr_jobs: default_settings.concurrent_ocr_jobs,
        ocr_timeout_seconds: default_settings.ocr_timeout_seconds,
        memory_limit_mb: default_settings.memory_limit_mb as u64,
        cpu_priority: default_settings.cpu_priority,
        server_host,
        server_port,
        jwt_secret_set: !config.jwt_secret.is_empty(),
        upload_path: config.upload_path.clone(),
        watch_folder: Some(config.watch_folder.clone()),
        ocr_language: default_settings.ocr_language,
        allowed_file_types: default_settings.allowed_file_types,
        watch_interval_seconds: config.watch_interval_seconds,
        file_stability_check_ms: config.file_stability_check_ms,
        max_file_age_hours: config.max_file_age_hours,
        enable_background_ocr: default_settings.enable_background_ocr,
        version: env!("CARGO_PKG_VERSION").to_string(),
        build_info: option_env!("BUILD_INFO").map(|s| s.to_string()),
    };

    Ok(Json(server_config))
}