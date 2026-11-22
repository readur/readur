use anyhow::Result;
use std::env;

use crate::models::S3SourceConfig;

mod logged;
mod mcp;
mod parsing;
mod security;
mod server;
mod storage;
mod temp;
mod validation;
use logged::{
    logged_number, logged_optional, logged_optional_number, logged_string, logged_switch,
    logged_tristate,
};
pub use mcp::McpConfig;
use parsing::read_jwt_setting;
pub use parsing::{
    validate_jwt_secret, DEFAULT_FILE_STABILITY_CHECK_MS, DEFAULT_WATCH_INTERVAL_SECONDS,
    MIN_JWT_SECRET_BYTES,
};
pub use security::SecurityConfig;
#[cfg(test)]
use storage::{parse_force_path_style, s3_storage_enabled};
pub use temp::{resolve_temp_dir, temp_dir, temp_dir_string};

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub server_address: String,
    pub jwt_secret: String,
    pub upload_path: String,
    pub watch_folder: String,
    pub user_watch_base_dir: String,
    pub enable_per_user_watch: bool,
    pub allowed_file_types: Vec<String>,
    pub watch_interval_seconds: Option<u64>,
    pub file_stability_check_ms: Option<u64>,
    pub max_file_age_hours: Option<u64>,

    // OCR Configuration
    pub ocr_language: String,
    pub concurrent_ocr_jobs: usize,
    pub ocr_timeout_seconds: u64,
    pub max_file_size_mb: u64,
    pub max_pdf_size_mb: u64,
    pub max_office_document_size_mb: u64,

    // Performance

    // OIDC Configuration
    pub oidc_enabled: bool,
    pub oidc_client_id: Option<String>,
    pub oidc_client_secret: Option<String>,
    pub oidc_issuer_url: Option<String>,
    pub oidc_redirect_uri: Option<String>,
    pub oidc_auto_register: Option<bool>,

    // Authentication Configuration
    pub allow_local_auth: Option<bool>,

    // S3 Configuration
    pub s3_enabled: bool,
    pub s3_config: Option<S3SourceConfig>,

    // Public URL for generating shared links
    pub public_url: Option<String>,

    // Authentication / authorization hardening knobs
    pub security: SecurityConfig,

    // Model Context Protocol endpoint (/api/mcp)
    pub mcp: McpConfig,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        // Load .env file if present
        match dotenvy::dotenv() {
            Ok(path) => println!("🔧 Loaded environment variables from: {}", path.display()),
            Err(_) => println!("🔧 No .env file found, using system environment variables"),
        }

        // Log all environment variable loading with detailed information
        println!("\n📋 CONFIGURATION LOADING:");
        println!("{}", "=".repeat(50));

        let database_url = server::database_url();
        // S3 storage can be enabled via S3_ENABLED=true or the documented
        // STORAGE_BACKEND=s3 alias; computed once and reused below.
        let s3_storage_on = storage::s3_storage_enabled(
            env::var("S3_ENABLED").ok().as_deref(),
            env::var("STORAGE_BACKEND").ok().as_deref(),
        );

        let config = Config {
            database_url,
            server_address: server::server_address(),
            jwt_secret: read_jwt_setting()?,
            upload_path: logged_string("UPLOAD_PATH", env::var("UPLOAD_PATH"), "./uploads"),
            watch_folder: logged_string("WATCH_FOLDER", env::var("WATCH_FOLDER"), "./watch"),
            user_watch_base_dir: logged_string(
                "USER_WATCH_BASE_DIR",
                env::var("USER_WATCH_BASE_DIR"),
                "./user_watch",
            ),
            enable_per_user_watch: logged_switch(
                "ENABLE_PER_USER_WATCH",
                env::var("ENABLE_PER_USER_WATCH"),
            ),
            allowed_file_types: server::allowed_file_types(),
            // Watcher Configuration
            watch_interval_seconds: logged_optional_number(
                "WATCH_INTERVAL_SECONDS",
                env::var("WATCH_INTERVAL_SECONDS"),
                "using default",
                "Not set, using default behavior",
            ),
            file_stability_check_ms: logged_optional_number(
                "FILE_STABILITY_CHECK_MS",
                env::var("FILE_STABILITY_CHECK_MS"),
                "using default",
                "Not set, using default behavior",
            ),
            max_file_age_hours: logged_optional_number(
                "MAX_FILE_AGE_HOURS",
                env::var("MAX_FILE_AGE_HOURS"),
                "using unlimited",
                "Not set, files will not expire",
            ),

            // OCR Configuration
            ocr_language: logged_string("OCR_LANGUAGE", env::var("OCR_LANGUAGE"), "eng"),
            concurrent_ocr_jobs: logged_number(
                "CONCURRENT_OCR_JOBS",
                env::var("CONCURRENT_OCR_JOBS"),
                4,
            ),
            ocr_timeout_seconds: logged_number(
                "OCR_TIMEOUT_SECONDS",
                env::var("OCR_TIMEOUT_SECONDS"),
                300,
            ),
            max_file_size_mb: logged_number("MAX_FILE_SIZE_MB", env::var("MAX_FILE_SIZE_MB"), 50),
            max_pdf_size_mb: logged_number("MAX_PDF_SIZE_MB", env::var("MAX_PDF_SIZE_MB"), 100),
            max_office_document_size_mb: logged_number(
                "MAX_OFFICE_DOCUMENT_SIZE_MB",
                env::var("MAX_OFFICE_DOCUMENT_SIZE_MB"),
                100,
            ),

            // Performance Configuration

            // OIDC Configuration
            oidc_enabled: logged_switch("OIDC_ENABLED", env::var("OIDC_ENABLED")),
            oidc_client_id: logged_optional("OIDC_CLIENT_ID", env::var("OIDC_CLIENT_ID"), false),
            oidc_client_secret: logged_optional(
                "OIDC_CLIENT_SECRET",
                env::var("OIDC_CLIENT_SECRET"),
                true,
            ),
            oidc_issuer_url: logged_optional("OIDC_ISSUER_URL", env::var("OIDC_ISSUER_URL"), false),
            oidc_redirect_uri: logged_optional(
                "OIDC_REDIRECT_URI",
                env::var("OIDC_REDIRECT_URI"),
                false,
            ),
            oidc_auto_register: logged_tristate(
                "OIDC_AUTO_REGISTER",
                env::var("OIDC_AUTO_REGISTER"),
                "false",
            ),

            // Authentication Configuration
            allow_local_auth: logged_tristate(
                "ALLOW_LOCAL_AUTH",
                env::var("ALLOW_LOCAL_AUTH"),
                "true",
            ),

            // S3 Configuration
            s3_enabled: storage::s3_enabled_from_env(),
            s3_config: storage::s3_config_from_env(s3_storage_on),
            public_url: env::var("PUBLIC_URL").ok().map(|url| {
                let url = url.trim_end_matches('/').to_string();
                println!("✅ PUBLIC_URL: {} (loaded from env)", url);
                url
            }),
            security: SecurityConfig::from_env()?,
            mcp: McpConfig::from_env()?,
        };

        config.validate_and_report()?;
        Ok(config)
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
