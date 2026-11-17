//! Object storage (S3) settings.

use std::env;

use crate::models::S3SourceConfig;

/// S3 storage is enabled by S3_ENABLED=true or the documented STORAGE_BACKEND=s3.
pub(super) fn s3_storage_enabled(s3_enabled: Option<&str>, storage_backend: Option<&str>) -> bool {
    s3_enabled
        .map(|v| v.trim().eq_ignore_ascii_case("true"))
        .unwrap_or(false)
        || storage_backend
            .map(|v| v.trim().eq_ignore_ascii_case("s3"))
            .unwrap_or(false)
}

/// S3_FORCE_PATH_STYLE with documented legacy alias S3_PATH_STYLE.
/// None (unset) means path-style for custom endpoints, virtual-hosted otherwise.
pub(super) fn parse_force_path_style(primary: Option<&str>, legacy: Option<&str>) -> Option<bool> {
    primary
        .or(legacy)
        .map(|v| v.trim().eq_ignore_ascii_case("true"))
}

/// Whether S3 storage is on (S3_ENABLED=true or STORAGE_BACKEND=s3), logged.
pub(super) fn s3_enabled_from_env() -> bool {
    let s3_storage_on = s3_storage_enabled(
        env::var("S3_ENABLED").ok().as_deref(),
        env::var("STORAGE_BACKEND").ok().as_deref(),
    );
    if s3_storage_on {
        if env::var("S3_ENABLED")
            .ok()
            .as_deref()
            .map(|v| v.trim().eq_ignore_ascii_case("true"))
            .unwrap_or(false)
        {
            println!("✅ S3_ENABLED: true (loaded from env)");
        } else {
            println!("✅ STORAGE_BACKEND: s3 (loaded from env, enables S3 storage)");
        }
    } else if env::var("S3_ENABLED").is_ok() {
        // Explicitly set but not "true" (e.g. S3_ENABLED=false) - distinct
        // from the var simply being unset.
        println!("✅ S3_ENABLED: false (loaded from env)");
    } else {
        println!("⚠️  S3_ENABLED: false (using default - env var not set)");
    }
    s3_storage_on
}

/// The S3 storage settings, read only when S3 storage is on. `None` when it is
/// off or when the bucket or credentials are missing.
pub(super) fn s3_config_from_env(s3_storage_on: bool) -> Option<S3SourceConfig> {
    if !s3_storage_on {
        return None;
    }
    let bucket_name = env::var("S3_BUCKET_NAME").unwrap_or_default();
    let region = env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string());
    let access_key_id = env::var("S3_ACCESS_KEY_ID").unwrap_or_default();
    let secret_access_key = env::var("S3_SECRET_ACCESS_KEY").unwrap_or_default();
    // S3_ENDPOINT is the documented var name; S3_ENDPOINT_URL is kept for
    // backward compatibility and takes precedence if both are set.
    let endpoint_url = env::var("S3_ENDPOINT_URL")
        .ok()
        .or_else(|| env::var("S3_ENDPOINT").ok());
    // S3_PREFIX places every object the storage backend writes
    // (documents, thumbnails, processed images) below this key
    // prefix. Existing documents are read and deleted by the key
    // recorded at upload, so setting or changing it later does
    // not strand objects already stored.
    let prefix = env::var("S3_PREFIX").ok();
    let force_path_style = parse_force_path_style(
        env::var("S3_FORCE_PATH_STYLE").ok().as_deref(),
        env::var("S3_PATH_STYLE").ok().as_deref(),
    );

    if bucket_name.is_empty() || access_key_id.is_empty() || secret_access_key.is_empty() {
        println!("❌ S3 enabled but missing required configuration (bucket_name, access_key_id, or secret_access_key)");
        return None;
    }
    println!("✅ S3_BUCKET_NAME: {} (loaded from env)", bucket_name);
    println!("✅ S3_REGION: {} (loaded from env)", region);
    println!("✅ S3_ACCESS_KEY_ID: configured (loaded from env)");
    println!("✅ S3_SECRET_ACCESS_KEY: configured (loaded from env)");
    if let Some(ref endpoint) = endpoint_url {
        println!("✅ S3_ENDPOINT_URL: {} (loaded from env)", endpoint);
    }
    if let Some(ref pref) = prefix {
        println!(
            "✅ S3_PREFIX: {} (loaded from env; prefix for new stored object keys)",
            pref
        );
    }
    if let Some(force_path_style_value) = force_path_style {
        println!(
            "✅ S3_FORCE_PATH_STYLE: {} (loaded from env)",
            force_path_style_value
        );
    }

    Some(S3SourceConfig {
        bucket_name,
        region,
        access_key_id,
        secret_access_key,
        endpoint_url,
        force_path_style,
        prefix,
        watch_folders: vec![],    // Will be configured separately for sources
        file_extensions: vec![],  // Will be configured separately for sources
        auto_sync: false,         // Not used for general storage
        sync_interval_minutes: 0, // Not used for general storage
    })
}
