//! Parsing and validation of individual environment settings.

use anyhow::Result;
use std::env;

/// Values shipped in example configs, compose files and docs. A JWT secret
/// equal to any of these is treated as unset.
pub(super) const KNOWN_WEAK_JWT_SECRETS: &[&str] = &[
    "your-secret-key",
    "your-secret-key-change-this",
    "your-secret-key-change-this-in-production",
    "your-super-secret-jwt-key-change-this-in-production",
    "dev-secret-key-change-in-production",
    "change-this-in-production",
    "change-me",
    "changeme",
    "secret",
    "test-secret",
    "test-secret-key",
    "test-jwt-secret-key",
    "test-jwt-secret-key-not-for-production",
    // Values committed in this repository for development, tests and CI.
    "readur-local-development-only-jwt-secret-0123456789",
    "readur-local-testing-only-jwt-secret-0123456789",
    "readur-test-environment-only-jwt-secret-0123456789",
    "readur-ci-testing-only-jwt-secret-0123456789",
    "55d90d87f068819258508dec17e962cdf8d2d458c90589c4978e2dc5cb96e5e7",
    "9b3f6c1e2a7d4058b6e1c3f2a9d8e7b4c5a6f1e0d2b3c4a5f6e7d8c9b0a1f2e3",
];

/// Minimum JWT secret length in bytes (HS256 key should be >= 256 bits).
pub const MIN_JWT_SECRET_BYTES: usize = 32;

/// Reject missing, short or well-known JWT signing secrets.
pub fn validate_jwt_secret(secret: &str) -> Result<()> {
    let trimmed = secret.trim();
    if trimmed.is_empty() {
        return Err(anyhow::anyhow!("JWT_SECRET must be set"));
    }
    let is_placeholder = trimmed.starts_with('<') && trimmed.ends_with('>');
    if is_placeholder || KNOWN_WEAK_JWT_SECRETS.iter().any(|weak| trimmed.eq_ignore_ascii_case(weak)) {
        return Err(anyhow::anyhow!(
            "JWT_SECRET is set to a published example value; generate a random secret (e.g. `openssl rand -hex 32`)"
        ));
    }
    if trimmed.len() < MIN_JWT_SECRET_BYTES {
        return Err(anyhow::anyhow!(
            "JWT_SECRET must be at least {} bytes; generate one with `openssl rand -hex 32`",
            MIN_JWT_SECRET_BYTES
        ));
    }
    Ok(())
}

/// `JWT_SECRET`, validated. Empty when unset: the server then signs tokens
/// with a key stored in the database (see `crate::jwt_signing_key`).
pub(super) fn jwt_secret_from_env() -> Result<String> {
    let secret = env::var("JWT_SECRET").unwrap_or_default();
    if secret.trim().is_empty() {
        println!("🔐 JWT_SECRET: not set (a signing key stored in the database is used)");
        return Ok(String::new());
    }
    let insecure_dev_mode = env_flag("READUR_INSECURE_DEV_MODE", false)?;
    match validate_jwt_secret(&secret) {
        Ok(()) => {
            println!("✅ JWT_SECRET: set (loaded from env)");
            Ok(secret)
        }
        // Escape hatch for throwaway local/CI environments only.
        Err(e) if insecure_dev_mode => {
            println!("🚨 JWT_SECRET: {} (allowed because READUR_INSECURE_DEV_MODE=true — never use in production)", e);
            Ok(secret)
        }
        Err(e) => {
            println!("❌ JWT_SECRET: {}", e);
            Err(e)
        }
    }
}

/// Read a boolean setting. An unset or empty variable yields `default`; any
/// value other than true/false/1/0/yes/no/on/off is a configuration error.
pub(super) fn env_flag(name: &str, default: bool) -> Result<bool> {
    match env::var(name) {
        Ok(val) => parse_flag(&val)
            .or_else(|| val.trim().is_empty().then_some(default))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "{} has an unrecognized value '{}'; use true or false",
                    name,
                    val.trim()
                )
            }),
        Err(_) => Ok(default),
    }
}

pub(super) fn parse_flag(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

/// Normalize a `CORS_ALLOWED_ORIGINS` entry to its serialized origin
/// (`scheme://host[:port]`, no trailing slash). Anything that is not a plain
/// http(s) origin is rejected.
pub(super) fn normalize_cors_origin(raw: &str) -> Result<String> {
    let invalid = || {
        anyhow::anyhow!(
            "CORS_ALLOWED_ORIGINS entry '{}' is not a valid origin (expected e.g. https://app.example.com)",
            raw
        )
    };
    let trimmed = raw.trim().trim_end_matches('/');
    let url = url::Url::parse(trimmed).map_err(|_| invalid())?;
    let plain = matches!(url.scheme(), "http" | "https")
        && url.host().is_some()
        && url.username().is_empty()
        && url.password().is_none()
        && url.path() == "/"
        && url.query().is_none()
        && url.fragment().is_none();
    if !plain {
        return Err(invalid());
    }
    Ok(url.origin().ascii_serialization())
}

pub(super) fn env_list(name: &str) -> Vec<String> {
    env::var(name)
        .map(|v| {
            v.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}
