//! Connectivity checks used by periodic source health validation.
//!
//! Errors are returned as short, categorized messages suitable for storing on
//! the source; upstream error details are only logged.

use tracing::warn;

use crate::{
    models::{source::WebDAVTestConnection, S3SourceConfig, Source, WebDAVSourceConfig},
    services::{s3_service::S3Service, webdav::WebDAVConfig, webdav::WebDAVService},
    utils::outbound::{categorize_connection_error, validate_outbound_url},
};

pub(super) async fn check_webdav(source: &Source) -> Result<(), String> {
    let config: WebDAVSourceConfig = serde_json::from_value(source.config.clone())
        .map_err(|e| format!("Config parse error: {}", e))?;

    validate_outbound_url(&WebDAVConfig::normalize_server_url(&config.server_url))
        .await
        .map_err(|e| e.to_string())?;

    let test_config = WebDAVTestConnection {
        server_url: config.server_url,
        username: config.username,
        password: config.password,
        server_type: config.server_type,
    };
    let result = WebDAVService::test_connection_with_config(&test_config)
        .await
        .map_err(|e| {
            warn!("WebDAV health check for source {} failed: {}", source.id, e);
            categorize_connection_error(&e.to_string()).to_string()
        })?;
    if !result.success {
        warn!("WebDAV health check for source {} failed: {}", source.id, result.message);
        return Err(categorize_connection_error(&result.message).to_string());
    }
    Ok(())
}

pub(super) async fn check_local_folder(_source: &Source) -> Result<(), String> {
    // Local folders are validated when they are configured and at sync time.
    Ok(())
}

/// Only the endpoint destination is checked; a full S3 request requires more
/// setup than a periodic health check warrants.
pub(super) async fn check_s3(source: &Source) -> Result<(), String> {
    let config: S3SourceConfig = serde_json::from_value(source.config.clone())
        .map_err(|e| format!("Config parse error: {}", e))?;
    for url in S3Service::outbound_urls(&config) {
        validate_outbound_url(&url).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}
