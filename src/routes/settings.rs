use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::get,
    Router,
};
use std::sync::Arc;
use ts_rs::TS;

use crate::{
    auth::{AdminUser, AuthUser},
    errors::settings::SettingsError,
    models::{
        normalize_endpoint_for_comparison, SecretReuseRefused, Settings, SettingsResponse,
        UpdateSettings,
    },
    services::settings::settings_response_for_user,
    utils::outbound::validate_outbound_url_for_config,
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
    let response = settings_response_for_user(&state.db, auth_user.user.id)
        .await
        .map_err(|e| {
            SettingsError::invalid_value(
                "database",
                &format!("Failed to fetch settings: {}", e),
                "Settings must be accessible",
            )
        })?;

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
        (status = 400, description = "Bad request - invalid settings data, or the stored WebDAV password would be reused for a different server or account"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
async fn update_settings(
    auth_user: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(mut update_data): Json<UpdateSettings>,
) -> Result<Response, StatusCode> {
    // The stored password is never sent to clients, so an empty value means
    // "unchanged". An explicit null still clears it.
    if matches!(&update_data.webdav_password, Some(Some(p)) if p.is_empty()) {
        update_data.webdav_password = None;
    }
    if update_data.webdav_password.is_none() {
        let stored = state
            .db
            .get_user_settings(auth_user.user.id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if !stored_webdav_password_reusable(stored.as_ref(), &update_data) {
            return Ok(crate::errors::error_response(
                StatusCode::BAD_REQUEST,
                "SECRET_REUSE_REFUSED",
                SecretReuseRefused::MESSAGE,
            ));
        }
    }
    if let Some(Some(url)) = &update_data.webdav_server_url {
        if !url.trim().is_empty() {
            let normalized = crate::services::webdav::WebDAVConfig::normalize_server_url(url);
            if validate_outbound_url_for_config(&normalized).await.is_err() {
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    }

    let settings = state
        .db
        .create_or_update_settings(auth_user.user.id, &update_data)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Json(SettingsResponse::from(settings)).into_response())
}

/// When an update keeps the stored WebDAV password, the server URL and
/// username it was entered for must stay the same.
fn stored_webdav_password_reusable(stored: Option<&Settings>, update: &UpdateSettings) -> bool {
    let Some(stored) = stored else { return true };
    if stored.webdav_password.as_deref().is_none_or(str::is_empty) {
        return true;
    }
    let effective = |update: &Option<Option<String>>, current: &Option<String>| match update {
        Some(value) => value.clone().unwrap_or_default(),
        None => current.clone().unwrap_or_default(),
    };
    let new_url = effective(&update.webdav_server_url, &stored.webdav_server_url);
    let new_username = effective(&update.webdav_username, &stored.webdav_username);
    normalize_endpoint_for_comparison(&new_url)
        == normalize_endpoint_for_comparison(
            stored.webdav_server_url.as_deref().unwrap_or_default(),
        )
        && new_username.trim() == stored.webdav_username.as_deref().unwrap_or_default().trim()
}

/// The server-wide configuration (environment variables), for administrators.
/// Per-user settings are under `GET /api/settings`.
#[derive(Debug, Serialize, utoipa::ToSchema, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
struct ServerConfiguration {
    /// `MAX_FILE_SIZE_MB`: no upload may be larger, whatever a user's own limit.
    max_file_size_mb: u64,
    /// `CONCURRENT_OCR_JOBS`: OCR jobs the worker runs at once.
    concurrent_ocr_jobs: u64,
    /// `OCR_TIMEOUT_SECONDS`: the OCR timeout for documents whose owner has no settings saved.
    ocr_timeout_seconds: u64,
    server_host: String,
    server_port: u16,
    jwt_secret_set: bool,
    upload_path: String,
    watch_folder: Option<String>,
    /// `OCR_LANGUAGE`: the default OCR language.
    ocr_language: String,
    /// `ALLOWED_FILE_TYPES`: extensions the watch folder and batch import accept.
    allowed_file_types: Vec<String>,
    watch_interval_seconds: Option<u64>,
    file_stability_check_ms: Option<u64>,
    max_file_age_hours: Option<u64>,
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
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ServerConfiguration>, StatusCode> {
    Ok(Json(server_configuration(&state.config)))
}

/// What `GET /api/settings/config` reports: the server's own configuration,
/// never the calling administrator's user settings.
fn server_configuration(config: &crate::config::Config) -> ServerConfiguration {
    // Parse server_address to get host and port
    let (server_host, server_port) = if let Some(colon_pos) = config.server_address.rfind(':') {
        let host = config.server_address[..colon_pos].to_string();
        let port = config.server_address[colon_pos + 1..]
            .parse::<u16>()
            .unwrap_or(8000);
        (host, port)
    } else {
        (config.server_address.clone(), 8000)
    };

    ServerConfiguration {
        max_file_size_mb: config.max_file_size_mb,
        concurrent_ocr_jobs: config.concurrent_ocr_jobs as u64,
        ocr_timeout_seconds: config.ocr_timeout_seconds,
        server_host,
        server_port,
        jwt_secret_set: !config.jwt_secret.is_empty(),
        upload_path: config.upload_path.clone(),
        watch_folder: Some(config.watch_folder.clone()),
        ocr_language: config.ocr_language.clone(),
        allowed_file_types: config.allowed_file_types.clone(),
        watch_interval_seconds: Some(config.effective_watch_interval_seconds()),
        file_stability_check_ms: Some(config.effective_file_stability_check_ms()),
        max_file_age_hours: config.max_file_age_hours,
        version: env!("CARGO_PKG_VERSION").to_string(),
        build_info: option_env!("BUILD_INFO").map(|s| s.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stored(url: &str, username: &str, password: Option<&str>) -> Settings {
        Settings {
            webdav_server_url: Some(url.to_string()),
            webdav_username: Some(username.to_string()),
            webdav_password: password.map(str::to_string),
            ..Settings::default()
        }
    }

    fn update(value: serde_json::Value) -> UpdateSettings {
        serde_json::from_value(value).expect("valid settings update")
    }

    #[test]
    fn server_configuration_reports_the_environment_configuration() {
        let config = crate::test_utils::TestConfigBuilder::default()
            .with_concurrent_ocr_jobs(3)
            .with_max_file_size_mb(77)
            .build("postgres://unused".to_string());
        let reported = serde_json::to_value(server_configuration(&config)).unwrap();
        assert_eq!(reported["concurrent_ocr_jobs"], 3);
        assert_eq!(reported["max_file_size_mb"], 77);
        assert_eq!(reported["ocr_timeout_seconds"], config.ocr_timeout_seconds);
        assert_eq!(reported["ocr_language"], config.ocr_language.as_str());
        assert_eq!(
            reported["allowed_file_types"],
            serde_json::json!(config.allowed_file_types)
        );
        for gone in ["memory_limit_mb", "cpu_priority", "enable_background_ocr"] {
            assert!(
                reported.get(gone).is_none(),
                "{gone} is not server configuration"
            );
        }
    }

    #[test]
    fn stored_password_is_kept_for_the_same_server_and_account() {
        let s = stored("https://dav.example/remote.php/dav", "alice", Some("pw"));
        assert!(stored_webdav_password_reusable(
            Some(&s),
            &update(serde_json::json!({}))
        ));
        assert!(stored_webdav_password_reusable(
            Some(&s),
            &update(serde_json::json!({
                "webdav_server_url": "https://dav.example/remote.php/dav/",
                "webdav_username": "alice"
            }))
        ));
    }

    #[test]
    fn stored_password_is_not_reused_for_another_server_or_account() {
        let s = stored("https://dav.example", "alice", Some("pw"));
        assert!(!stored_webdav_password_reusable(
            Some(&s),
            &update(serde_json::json!({"webdav_server_url": "https://other.example"}))
        ));
        assert!(!stored_webdav_password_reusable(
            Some(&s),
            &update(serde_json::json!({"webdav_username": "bob"}))
        ));
    }

    #[test]
    fn no_stored_password_means_nothing_to_protect() {
        let s = stored("https://dav.example", "alice", None);
        assert!(stored_webdav_password_reusable(
            Some(&s),
            &update(serde_json::json!({"webdav_server_url": "https://other.example"}))
        ));
        assert!(stored_webdav_password_reusable(
            None,
            &update(serde_json::json!({"webdav_username": "bob"}))
        ));
    }
}
