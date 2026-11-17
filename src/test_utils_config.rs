//! `TestConfigBuilder`: the server config `TestContext` runs with.

use std::path::PathBuf;

/// Builder pattern for test configuration to eliminate config duplication
pub struct TestConfigBuilder {
    /// Where stored files go. `None` until `with_upload_path` sets one, or
    /// `TestContext::with_config` claims a fresh per-context directory
    /// outside the repository (removed when the context drops), so test runs
    /// never leave files in the working tree.
    pub(super) upload_path: Option<String>,
    pub(super) watch_folder: String,
    pub(super) jwt_secret: String,
    pub(super) concurrent_ocr_jobs: usize,
    pub(super) ocr_timeout_seconds: u64,
    pub(super) max_file_size_mb: u64,
    pub(super) oidc_enabled: bool,
    pub(super) mcp: crate::config::McpConfig,
    /// Whether self-registered accounts start inactive, as they always do in
    /// production. Off by default because tests register users and log in.
    pub(super) registration_requires_approval: bool,
}

impl Default for TestConfigBuilder {
    fn default() -> Self {
        Self {
            upload_path: None,
            watch_folder: "./test-watch".to_string(),
            jwt_secret: "test-secret".to_string(),
            concurrent_ocr_jobs: 2,
            ocr_timeout_seconds: 60,
            max_file_size_mb: 10,
            oidc_enabled: false,
            mcp: crate::config::McpConfig::default(),
            registration_requires_approval: false,
        }
    }
}

impl TestConfigBuilder {
    pub fn with_upload_path(mut self, path: &str) -> Self {
        self.upload_path = Some(path.to_string());
        self
    }

    pub fn with_watch_folder(mut self, folder: &str) -> Self {
        self.watch_folder = folder.to_string();
        self
    }

    pub fn with_concurrent_ocr_jobs(mut self, jobs: usize) -> Self {
        self.concurrent_ocr_jobs = jobs;
        self
    }

    pub fn with_oidc_enabled(mut self, enabled: bool) -> Self {
        self.oidc_enabled = enabled;
        self
    }

    pub fn with_max_file_size_mb(mut self, size_mb: u64) -> Self {
        self.max_file_size_mb = size_mb;
        self
    }

    pub fn with_registration_requires_approval(mut self, required: bool) -> Self {
        self.registration_requires_approval = required;
        self
    }

    /// When no upload path was set, points this builder at a new directory
    /// under the system temp dir, unique to `context_id`, and returns it for
    /// the caller to remove. Returns `None` for an explicit upload path.
    pub(crate) fn claim_upload_dir(&mut self, context_id: &str) -> Option<PathBuf> {
        if self.upload_path.is_some() {
            return None;
        }
        let dir = std::env::temp_dir().join(format!("readur-test-uploads-{context_id}"));
        self.upload_path = Some(dir.to_string_lossy().into_owned());
        Some(dir)
    }

    /// The config for a test server. Without an upload path (and no
    /// `claim_upload_dir`), files go to a fresh temp directory nobody removes;
    /// `TestContext::with_config` always claims one first.
    pub(crate) fn build(self, database_url: String) -> crate::config::Config {
        let upload_path = self.upload_path.unwrap_or_else(|| {
            let dir =
                std::env::temp_dir().join(format!("readur-test-uploads-{}", uuid::Uuid::new_v4()));
            dir.to_string_lossy().into_owned()
        });
        crate::config::Config {
            database_url,
            server_address: "127.0.0.1:0".to_string(),
            jwt_secret: self.jwt_secret,
            upload_path,
            watch_folder: self.watch_folder,
            user_watch_base_dir: "./test-user-watch".to_string(),
            enable_per_user_watch: false,
            allowed_file_types: vec!["pdf".to_string(), "txt".to_string(), "png".to_string()],
            watch_interval_seconds: Some(30),
            file_stability_check_ms: Some(500),
            max_file_age_hours: None,

            // OCR Configuration
            ocr_language: "eng".to_string(),
            concurrent_ocr_jobs: self.concurrent_ocr_jobs,
            ocr_timeout_seconds: self.ocr_timeout_seconds,
            max_file_size_mb: self.max_file_size_mb,
            max_pdf_size_mb: 100,
            max_office_document_size_mb: 100,

            // Performance

            // OIDC Configuration
            oidc_enabled: self.oidc_enabled,
            oidc_client_id: None,
            oidc_client_secret: None,
            oidc_issuer_url: None,
            oidc_redirect_uri: None,
            oidc_auto_register: None,
            allow_local_auth: None,

            // S3 Configuration
            s3_enabled: false,
            s3_config: None,

            // Public URL
            public_url: None,
            // Tests create users through the public registration endpoint.
            security: crate::config::SecurityConfig {
                allow_registration: true,
                registration_requires_approval: self.registration_requires_approval,
                ..Default::default()
            },
            mcp: self.mcp,
        }
    }
}
