
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
            max_delay_ms: 10000,    // 10 seconds (reduced from 30s to prevent excessive wait times)
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

    /// Normalizes a server URL by adding protocol if missing
    /// Prefers HTTPS over HTTP for security reasons
    pub fn normalize_server_url(url: &str) -> String {
        let trimmed = url.trim();
        
        // If protocol is already specified, return as-is
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            return trimmed.to_string();
        }
        
        // If no protocol specified, default to HTTPS for security
        format!("https://{}", trimmed)
    }

    /// Generates alternative protocol URL for fallback attempts
    /// If input has HTTPS, returns HTTP version and vice versa
    pub fn get_alternative_protocol_url(url: &str) -> Option<String> {
        if url.starts_with("https://") {
            Some(url.replacen("https://", "http://", 1))
        } else if url.starts_with("http://") {
            Some(url.replacen("http://", "https://", 1))
        } else {
            None
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

        // Validate URL format - now accepts URLs without protocol
        // Protocol detection and fallback will be handled during connection testing
        let normalized_url = Self::normalize_server_url(&self.server_url);
        
        // Basic URL validation - check if it looks like a valid domain/IP
        let url_without_protocol = normalized_url
            .trim_start_matches("https://")
            .trim_start_matches("http://");
            
        if url_without_protocol.is_empty() {
            return Err(anyhow::anyhow!("Server URL must contain a valid domain or IP address"));
        }

        // Check for obviously invalid URLs
        if url_without_protocol.contains("://") {
            return Err(anyhow::anyhow!("Invalid URL format: contains multiple protocols"));
        }

        // Reject relative URLs (paths that start with /)
        if url_without_protocol.starts_with('/') {
            return Err(anyhow::anyhow!("Server URL cannot be a relative path. Please provide a full server URL like 'https://server.example.com' or 'server.example.com'"));
        }

        Ok(())
    }

    /// Returns the base URL for WebDAV operations
    pub fn webdav_url(&self) -> String {
        // Normalize the server URL by adding protocol if missing and removing trailing slashes
        let normalized_url = Self::normalize_server_url(&self.server_url).trim_end_matches('/').to_string();
        
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

    /// Returns alternative WebDAV URLs to try if the primary one fails
    /// This is used for fallback mechanisms when encountering 405 errors
    pub fn webdav_fallback_urls(&self) -> Vec<String> {
        let normalized_url = Self::normalize_server_url(&self.server_url).trim_end_matches('/').to_string();
        let mut fallback_urls = Vec::new();
        
        match self.server_type.as_deref() {
            Some("nextcloud") => {
                // Primary: /remote.php/dav/files/{username}
                // Fallback 1: /remote.php/webdav (legacy ownCloud style)  
                // Fallback 2: /webdav (generic)
                fallback_urls.push(format!("{}/remote.php/webdav", normalized_url));
                fallback_urls.push(format!("{}/webdav", normalized_url));
            }
            Some("owncloud") => {
                // Primary: /remote.php/webdav
                // Fallback 1: /remote.php/dav/files/{username} (newer Nextcloud style)
                // Fallback 2: /webdav (generic)
                fallback_urls.push(format!("{}/remote.php/dav/files/{}", normalized_url, self.username));
                fallback_urls.push(format!("{}/webdav", normalized_url));
            }
            _ => {
                // Generic WebDAV - try common patterns
                // Fallback 1: /remote.php/webdav (ownCloud/Nextcloud)
                // Fallback 2: /remote.php/dav/files/{username} (Nextcloud)
                // Fallback 3: /dav (alternative)
                fallback_urls.push(format!("{}/remote.php/webdav", normalized_url));
                fallback_urls.push(format!("{}/remote.php/dav/files/{}", normalized_url, self.username));
                fallback_urls.push(format!("{}/dav", normalized_url));
            }
        }
        
        fallback_urls
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