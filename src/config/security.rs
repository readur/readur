//! Authentication and authorization settings (`Config::security`).

use anyhow::Result;
use std::env;

use super::parsing::{env_flag, env_list, normalize_cors_origin};

/// Authentication and authorization settings. `Default` is the most
/// restrictive configuration.
#[derive(Clone)]
pub struct SecurityConfig {
    /// Allow unauthenticated self-registration (`ALLOW_REGISTRATION`).
    pub allow_registration: bool,
    /// Self-registered accounts start disabled until an admin enables them.
    /// Always on in production; only test harnesses turn it off.
    pub registration_requires_approval: bool,
    /// Lifetime of issued session tokens (`JWT_TTL_HOURS`).
    pub jwt_ttl_hours: i64,
    /// Canonicalized roots under which local-folder sources may point
    /// (`LOCAL_SOURCE_ALLOWED_PATHS`). Empty means admin-only, any path.
    pub local_source_allowed_paths: Vec<std::path::PathBuf>,
    /// Reverse proxies whose `X-Forwarded-For` is trusted (`TRUSTED_PROXIES`).
    pub trusted_proxies: Vec<ipnet::IpNet>,
    /// Origins allowed to make cross-origin requests (`CORS_ALLOWED_ORIGINS`).
    pub cors_allowed_origins: Vec<String>,
    /// Bearer token accepted on `/metrics` (`METRICS_TOKEN`).
    pub metrics_token: Option<String>,
    /// Link an OIDC identity to an existing local account with the same
    /// verified email (`OIDC_LINK_EXISTING_BY_EMAIL`).
    pub oidc_link_existing_by_email: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            allow_registration: false,
            registration_requires_approval: true,
            jwt_ttl_hours: 12,
            local_source_allowed_paths: Vec::new(),
            trusted_proxies: Vec::new(),
            cors_allowed_origins: Vec::new(),
            metrics_token: None,
            oidc_link_existing_by_email: false,
        }
    }
}

impl std::fmt::Debug for SecurityConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecurityConfig")
            .field("allow_registration", &self.allow_registration)
            .field(
                "registration_requires_approval",
                &self.registration_requires_approval,
            )
            .field("jwt_ttl_hours", &self.jwt_ttl_hours)
            .field(
                "local_source_allowed_paths",
                &self.local_source_allowed_paths,
            )
            .field("trusted_proxies", &self.trusted_proxies)
            .field("cors_allowed_origins", &self.cors_allowed_origins)
            .field("metrics_token", &self.metrics_token.as_ref().map(|_| "***"))
            .field(
                "oidc_link_existing_by_email",
                &self.oidc_link_existing_by_email,
            )
            .finish()
    }
}

impl SecurityConfig {
    pub fn from_env() -> Result<Self> {
        let defaults = Self::default();

        let allow_registration = env_flag("ALLOW_REGISTRATION", defaults.allow_registration)?;
        println!("🔑 ALLOW_REGISTRATION: {}", allow_registration);

        let jwt_ttl_hours = match env::var("JWT_TTL_HOURS") {
            Ok(v) => match v.trim().parse::<i64>() {
                Ok(h) if (1..=24 * 30).contains(&h) => h,
                _ => {
                    return Err(anyhow::anyhow!(
                        "JWT_TTL_HOURS must be an integer between 1 and 720"
                    ))
                }
            },
            Err(_) => defaults.jwt_ttl_hours,
        };
        println!("🔑 JWT_TTL_HOURS: {}", jwt_ttl_hours);

        let mut local_source_allowed_paths = Vec::new();
        for raw in env_list("LOCAL_SOURCE_ALLOWED_PATHS") {
            let canonical = std::path::Path::new(&raw).canonicalize().map_err(|e| {
                anyhow::anyhow!(
                    "LOCAL_SOURCE_ALLOWED_PATHS entry '{}' is not accessible: {}",
                    raw,
                    e
                )
            })?;
            local_source_allowed_paths.push(canonical);
        }
        if local_source_allowed_paths.is_empty() {
            println!(
                "📂 LOCAL_SOURCE_ALLOWED_PATHS: not set (local folder sources are admin-only)"
            );
        } else {
            println!(
                "📂 LOCAL_SOURCE_ALLOWED_PATHS: {:?}",
                local_source_allowed_paths
            );
        }

        let proxy_setting = env::var("TRUSTED_PROXIES").ok();
        let proxy_ranges = crate::utils::client_ip::parse_proxy_ranges(proxy_setting.as_deref())?;
        println!(
            "🌐 TRUSTED_PROXIES: {}",
            crate::utils::client_ip::describe_proxy_ranges(proxy_setting.as_deref())
        );

        let cors_allowed_origins = env_list("CORS_ALLOWED_ORIGINS")
            .iter()
            .map(|origin| normalize_cors_origin(origin))
            .collect::<Result<Vec<_>>>()?;
        println!("🌐 CORS_ALLOWED_ORIGINS: {:?}", cors_allowed_origins);

        let metrics_token = env::var("METRICS_TOKEN")
            .ok()
            .filter(|t| !t.trim().is_empty());
        if let Some(token) = &metrics_token {
            if token.len() < 16 {
                return Err(anyhow::anyhow!(
                    "METRICS_TOKEN must be at least 16 characters"
                ));
            }
        }
        println!(
            "📈 METRICS_TOKEN: {}",
            if metrics_token.is_some() {
                "configured"
            } else {
                "not set (/metrics requires an admin session)"
            }
        );

        let oidc_link_existing_by_email = env_flag(
            "OIDC_LINK_EXISTING_BY_EMAIL",
            defaults.oidc_link_existing_by_email,
        )?;

        Ok(Self {
            allow_registration,
            registration_requires_approval: true,
            jwt_ttl_hours,
            local_source_allowed_paths,
            trusted_proxies: proxy_ranges,
            cors_allowed_origins,
            metrics_token,
            oidc_link_existing_by_email,
        })
    }
}
