
/// WebDAV server configuration
#[derive(Debug, Clone)]
pub struct WebDAVConfig {
    pub server_url: String,
    pub username: String,
    pub password: String,
    pub watch_folders: Vec<String>,
    pub file_extensions: Vec<String>,
    pub timeout_seconds: u64,
    pub server_type: Option<String>, // "nextcloud", "owncloud", "generic"
}

/// Retry configuration for WebDAV operations
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
    pub timeout_seconds: u64,
    pub rate_limit_backoff_ms: u64, // Additional backoff for 429 responses
}

/// Concurrency configuration for WebDAV operations
#[derive(Debug, Clone)]
pub struct ConcurrencyConfig {
    pub max_concurrent_scans: usize,
    pub max_concurrent_downloads: usize,
    pub adaptive_rate_limiting: bool,
}

/// Configuration for Depth infinity PROPFIND optimizations
#[derive(Debug, Clone)]
pub struct DepthInfinityConfig {
    /// Whether to attempt Depth infinity PROPFIND requests
    pub enabled: bool,
    /// Maximum response size in bytes before falling back to recursive approach
    pub max_response_size_bytes: usize,
    /// Timeout for infinity depth requests in seconds
    pub timeout_seconds: u64,
    /// Cache server capability detection results for this duration (seconds)
    pub capability_cache_duration_seconds: u64,
    /// Whether to automatically fallback to recursive approach on failure
    pub auto_fallback: bool,
    /// Maximum directory depth to attempt infinity for (0 = no limit)
    pub max_depth_for_infinity: u32,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000, // 1 second
            max_delay_ms: 30000,    // 30 seconds
            backoff_multiplier: 2.0,
            timeout_seconds: 30,
            rate_limit_backoff_ms: 5000, // 5 seconds
        }
    }
}

impl Default for ConcurrencyConfig {
    fn default() -> Self {
        Self {
            max_concurrent_scans: 4,
            max_concurrent_downloads: 8,
            adaptive_rate_limiting: true,
        }
    }
}

impl Default for DepthInfinityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_response_size_bytes: 50 * 1024 * 1024, // 50MB
            timeout_seconds: 120, // 2 minutes for large directories
            capability_cache_duration_seconds: 3600, // 1 hour
            auto_fallback: true,
            max_depth_for_infinity: 0, // No limit by default
        }
    }
}

impl WebDAVConfig {
    /// Creates a new WebDAV configuration
    pub fn new(
        server_url: String,
        username: String,
        password: String,
        watch_folders: Vec<String>,
        file_extensions: Vec<String>,
    ) -> Self {
        Self {
            server_url,
            username,
            password,
            watch_folders,
            file_extensions,
            timeout_seconds: 30,
            server_type: None,
        }
    }

    /// Validates the configuration
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.server_url.is_empty() {
            return Err(anyhow::anyhow!("Server URL cannot be empty"));
        }

        if self.username.is_empty() {
            return Err(anyhow::anyhow!("Username cannot be empty"));
        }

        if self.password.is_empty() {
            return Err(anyhow::anyhow!("Password cannot be empty"));
        }

        if self.watch_folders.is_empty() {
            return Err(anyhow::anyhow!("At least one watch folder must be specified"));
        }

        // Validate URL format
        if !self.server_url.starts_with("http://") && !self.server_url.starts_with("https://") {
            return Err(anyhow::anyhow!("Server URL must start with http:// or https://"));
        }

        Ok(())
    }

    /// Returns the base URL for WebDAV operations
    pub fn webdav_url(&self) -> String {
        // Normalize the server URL by removing trailing slashes
        let normalized_url = self.server_url.trim_end_matches('/').to_string();
        
        // Add WebDAV path based on server type
        match self.server_type.as_deref() {
            Some("nextcloud") => {
                if !normalized_url.contains("/remote.php/dav/files/") {
                    format!("{}/remote.php/dav/files/{}", normalized_url, self.username)
                } else {
                    normalized_url
                }
            }
            Some("owncloud") => {
                if !normalized_url.contains("/remote.php/webdav") {
                    format!("{}/remote.php/webdav", normalized_url)
                } else {
                    normalized_url
                }
            }
            _ => {
                // Generic WebDAV - use the normalized URL as provided
                normalized_url
            }
        }
    }

    /// Checks if a file extension is supported
    pub fn is_supported_extension(&self, filename: &str) -> bool {
        if self.file_extensions.is_empty() {
            return true; // If no extensions specified, support all
        }

        let extension = filename.split('.').last().unwrap_or("");
        self.file_extensions.iter().any(|ext| ext.eq_ignore_ascii_case(extension))
    }

    /// Gets the timeout duration
    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.timeout_seconds)
    }
}