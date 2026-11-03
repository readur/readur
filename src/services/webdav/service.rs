use anyhow::{anyhow, Result};
use reqwest::{Client, Method};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashSet;
use tokio::sync::Semaphore;
use tokio::time::sleep;
// futures_util::stream import removed as unused
use tracing::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use rand::RngExt;

use crate::models::{
    FileIngestionInfo,
};
use crate::models::source::{
    WebDAVConnectionResult, WebDAVCrawlEstimate, WebDAVTestConnection,
};
use crate::models::source_error::{ErrorSourceType, ErrorContext};
use crate::services::source_error_tracker::SourceErrorTracker;
use crate::webdav_xml_parser::{parse_propfind_response, parse_propfind_response_with_directories};
use crate::mime_detection::{detect_mime_from_content, MimeDetectionResult};
use crate::utils::outbound::{self, check_url_without_dns};

use super::{config::{WebDAVConfig, RetryConfig, ConcurrencyConfig}, SyncProgress};
use super::common::{build_user_agent, retry_after_delay, verify_download};
use super::paths::{decode_href, encode_path};

/// Longest wait a server-supplied Retry-After header can impose.
const MAX_RETRY_AFTER: Duration = Duration::from_secs(120);

/// Results from WebDAV discovery including both files and directories
#[derive(Debug, Clone)]
pub struct WebDAVDiscoveryResult {
    pub files: Vec<FileIngestionInfo>,
    pub directories: Vec<FileIngestionInfo>,
}

/// Result of a directory scan that also reports which subdirectories could not
/// be listed, so callers never treat an incompletely scanned subtree as current.
#[derive(Debug, Clone, Default)]
pub struct WebDAVScanResult {
    pub files: Vec<FileIngestionInfo>,
    pub directories: Vec<FileIngestionInfo>,
    /// The scanned directory's own entry from its listing (carries its ETag).
    pub root: Option<FileIngestionInfo>,
    /// Directories whose listing failed or was skipped; their descendants are absent.
    pub failed_directories: Vec<String>,
}

impl From<WebDAVScanResult> for WebDAVDiscoveryResult {
    fn from(scan: WebDAVScanResult) -> Self {
        Self { files: scan.files, directories: scan.directories }
    }
}

/// Error-tracker context for discovery: records per-directory outcomes and
/// skips directories that are in failure cooldown.
#[derive(Clone)]
pub struct DiscoveryTracking {
    pub user_id: uuid::Uuid,
    pub source_id: Option<uuid::Uuid>,
    pub tracker: SourceErrorTracker,
}

/// PROPFIND body requesting the properties needed for directory listings.
const PROPFIND_LISTING_BODY: &str = r#"<?xml version="1.0" encoding="utf-8"?>
            <D:propfind xmlns:D="DAV:">
                <D:prop>
                    <D:displayname/>
                    <D:getcontentlength/>
                    <D:getlastmodified/>
                    <D:getetag/>
                    <D:resourcetype/>
                    <D:creationdate/>
                </D:prop>
            </D:propfind>"#;

/// Joins a base URL and a decoded relative path with exactly one slash between
/// them, percent-encoding the path's segments for the request line.
fn join_url(base_url: &str, path: &str) -> String {
    let clean_path = path.trim_start_matches('/');
    if clean_path.is_empty() {
        base_url.trim_end_matches('/').to_string()
    } else {
        format!("{}/{}", base_url.trim_end_matches('/'), encode_path(clean_path))
    }
}

/// Result of downloading a file with MIME type detection
#[derive(Debug, Clone)]
pub struct WebDAVDownloadResult {
    pub content: Vec<u8>,
    pub file_info: FileIngestionInfo,
    pub mime_detection: MimeDetectionResult,
    pub mime_type_updated: bool,
}

/// Server capabilities information
#[derive(Debug, Clone)]
pub struct ServerCapabilities {
    pub dav_compliance: String,
    pub allowed_methods: String,
    pub server_software: Option<String>,
    pub supports_etag: bool,
    pub supports_depth_infinity: bool,
    /// Infinity depth support verified through testing
    pub infinity_depth_tested: bool,
    /// Whether infinity depth actually works in practice
    pub infinity_depth_works: bool,
    /// Timestamp when capabilities were last checked
    pub last_checked: std::time::Instant,
}

/// Health status information
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub healthy: bool,
    pub message: String,
    pub response_time_ms: u64,
    pub details: Option<serde_json::Value>,
}

/// Validation report structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub overall_health_score: i32, // 0-100
    pub issues: Vec<ValidationIssue>,
    pub recommendations: Vec<ValidationRecommendation>,
    pub summary: ValidationSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub issue_type: ValidationIssueType,
    pub severity: ValidationSeverity,
    pub directory_path: String,
    pub description: String,
    pub details: Option<serde_json::Value>,
    pub detected_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, Hash, PartialEq)]
pub enum ValidationIssueType {
    /// Directory exists on server but not in our tracking
    Untracked,
    /// Directory in our tracking but missing on server  
    Missing,
    /// ETag mismatch between server and our cache
    ETagMismatch,
    /// Directory hasn't been scanned in a very long time
    Stale,
    /// Server errors when accessing directory
    Inaccessible,
    /// ETag support seems unreliable for this directory
    ETagUnreliable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationSeverity {
    Info,    // No action needed, just FYI
    Warning, // Should investigate but not urgent
    Error,   // Needs immediate attention
    Critical, // System integrity at risk
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRecommendation {
    pub action: ValidationAction,
    pub reason: String,
    pub affected_directories: Vec<String>,
    pub priority: ValidationSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationAction {
    /// Run a deep scan of specific directories
    DeepScanRequired,
    /// Clear and rebuild directory tracking
    RebuildTracking,
    /// ETag support is unreliable, switch to periodic scans
    DisableETagOptimization,
    /// Clean up orphaned database entries
    CleanupDatabase,
    /// Server configuration issue needs attention
    CheckServerConfiguration,
    /// No action needed, system is healthy
    NoActionRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSummary {
    pub total_directories_checked: usize,
    pub healthy_directories: usize,
    pub directories_with_issues: usize,
    pub critical_issues: usize,
    pub warning_issues: usize,
    pub info_issues: usize,
    pub validation_duration_ms: u64,
}

/// Main WebDAV service that handles all WebDAV operations in a single, unified interface
pub struct WebDAVService {
    client: Client,
    config: WebDAVConfig,
    retry_config: RetryConfig,
    concurrency_config: ConcurrencyConfig,
    scan_semaphore: Arc<Semaphore>,
    download_semaphore: Arc<Semaphore>,
    /// Stores the working protocol (updated after successful protocol detection)
    working_protocol: Arc<std::sync::RwLock<Option<String>>>,
}

impl WebDAVService {
    /// Creates a new WebDAV service with default configurations
    pub fn new(config: WebDAVConfig) -> Result<Self> {
        Self::new_with_configs(config, RetryConfig::default(), ConcurrencyConfig::default())
    }

    /// Creates a new WebDAV service with custom retry configuration
    pub fn new_with_retry(config: WebDAVConfig, retry_config: RetryConfig) -> Result<Self> {
        Self::new_with_configs(config, retry_config, ConcurrencyConfig::default())
    }

    /// Creates a new WebDAV service with all custom configurations
    pub fn new_with_configs(
        config: WebDAVConfig, 
        retry_config: RetryConfig, 
        concurrency_config: ConcurrencyConfig
    ) -> Result<Self> {
        // Validate configuration
        config.validate()?;

        // The configured server must be a permitted destination. Host names
        // are also filtered on every DNS lookup and redirect by the client.
        if let Ok(url) = url::Url::parse(&WebDAVConfig::normalize_server_url(&config.server_url)) {
            check_url_without_dns(&url).map_err(|e| anyhow!("{}", e))?;
        }

        // Bound connection setup and idle time between received bytes rather
        // than the whole transfer, so large files are not cut off and retried.
        let client = outbound::client_builder()
            .connect_timeout(config.timeout().min(Duration::from_secs(30)))
            .read_timeout(config.timeout())
            .build()?;

        // Create semaphores for concurrency control
        let scan_semaphore = Arc::new(Semaphore::new(concurrency_config.max_concurrent_scans));
        let download_semaphore = Arc::new(Semaphore::new(concurrency_config.max_concurrent_downloads));

        Ok(Self {
            client,
            config,
            retry_config,
            concurrency_config,
            scan_semaphore,
            download_semaphore,
            working_protocol: Arc::new(std::sync::RwLock::new(None)),
        })
    }


    // ============================================================================
    // Protocol Detection Methods
    // ============================================================================

    /// Detects the working protocol by trying HTTPS first, then HTTP
    /// This method handles smart protocol detection for URLs without explicit protocols
    async fn detect_working_protocol(&self) -> Result<String> {
        info!("🔍 Starting smart protocol detection for: {}", self.config.server_url);

        // If URL already has a protocol, use it directly
        if self.config.server_url.starts_with("http://") || self.config.server_url.starts_with("https://") {
            let protocol = if self.config.server_url.starts_with("https://") { "https" } else { "http" };
            info!("✅ Protocol already specified: {}", protocol);
            return Ok(protocol.to_string());
        }

        // Try HTTPS first (more secure default)
        let https_url = format!("https://{}", self.config.server_url.trim());
        info!("🔐 Trying HTTPS first: {}", https_url);
        
        match self.test_protocol_connection(&https_url).await {
            Ok(()) => {
                info!("✅ HTTPS connection successful");
                // Store the working protocol for future use
                if let Ok(mut working_protocol) = self.working_protocol.write() {
                    *working_protocol = Some("https".to_string());
                }
                return Ok("https".to_string());
            }
            Err(https_error) => {
                warn!("❌ HTTPS connection failed: {}", https_error);
                
                // Check if this is a connection-related error (not auth error)
                if self.is_connection_error(&https_error) {
                    info!("🔄 HTTPS failed with connection error, trying HTTP fallback");
                    
                    // Try HTTP fallback
                    let http_url = format!("http://{}", self.config.server_url.trim());
                    info!("🔓 Trying HTTP fallback: {}", http_url);
                    
                    match self.test_protocol_connection(&http_url).await {
                        Ok(()) => {
                            warn!("⚠️ HTTP connection successful - consider configuring HTTPS for security");
                            // Store the working protocol for future use
                            if let Ok(mut working_protocol) = self.working_protocol.write() {
                                *working_protocol = Some("http".to_string());
                            }
                            return Ok("http".to_string());
                        }
                        Err(http_error) => {
                            error!("❌ Both HTTPS and HTTP failed");
                            error!("   HTTPS error: {}", https_error);
                            error!("   HTTP error: {}", http_error);
                            return Err(anyhow!(
                                "Protocol detection failed. Both HTTPS and HTTP connections failed. \
                                HTTPS error: {}. HTTP error: {}. \
                                Please verify the server URL and ensure WebDAV is properly configured.",
                                https_error, http_error
                            ));
                        }
                    }
                } else {
                    // Auth or other non-connection error with HTTPS - don't try HTTP
                    error!("❌ HTTPS failed with non-connection error (likely auth or server config): {}", https_error);
                    return Err(anyhow!(
                        "HTTPS connection failed with authentication or server configuration error: {}. \
                        Please check your credentials and server settings.", 
                        https_error
                    ));
                }
            }
        }
    }

    /// Tests connection with a specific protocol URL
    async fn test_protocol_connection(&self, full_url: &str) -> Result<()> {
        debug!("🧪 Testing protocol connection to: {}", full_url);
        
        // Create a temporary config with the full URL for testing
        let temp_config = WebDAVConfig {
            server_url: full_url.to_string(),
            username: self.config.username.clone(),
            password: self.config.password.clone(),
            watch_folders: self.config.watch_folders.clone(),
            file_extensions: self.config.file_extensions.clone(),
            timeout_seconds: self.config.timeout_seconds,
            server_type: self.config.server_type.clone(),
        };

        // Test basic OPTIONS request
        let webdav_url = temp_config.webdav_url();
        debug!("📍 Testing WebDAV URL: {}", webdav_url);
        
        let response = self.client
            .request(Method::OPTIONS, &webdav_url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .send()
            .await
            .map_err(|e| anyhow!("Connection failed: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Protocol test failed with status: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        debug!("✅ Protocol connection test successful");
        Ok(())
    }

    /// Determines if an error is connection-related (vs auth or other errors)
    pub fn is_connection_error(&self, error: &anyhow::Error) -> bool {
        let error_str = error.to_string().to_lowercase();
        
        // Connection-related errors that suggest trying different protocol
        error_str.contains("connection refused") ||
        error_str.contains("timeout") ||
        error_str.contains("dns") ||
        error_str.contains("network") ||
        error_str.contains("unreachable") ||
        error_str.contains("tls") ||
        error_str.contains("ssl") ||
        error_str.contains("certificate") ||
        error_str.contains("handshake")
    }

    /// Gets the currently working protocol (if detected)
    pub fn get_working_protocol(&self) -> Option<String> {
        self.working_protocol.read().ok().and_then(|p| p.clone())
    }

    /// Gets the effective server URL with the working protocol
    pub fn get_effective_server_url(&self) -> String {
        // If we have a detected working protocol, use it
        if let Some(protocol) = self.get_working_protocol() {
            if !self.config.server_url.starts_with("http://") && !self.config.server_url.starts_with("https://") {
                return format!("{}://{}", protocol, self.config.server_url.trim());
            }
        }
        
        // Otherwise use the configured URL (normalized)
        WebDAVConfig::normalize_server_url(&self.config.server_url)
    }

    // ============================================================================
    // Connection and Testing Methods
    // ============================================================================

    /// Tests the WebDAV connection
    pub async fn test_connection(&self) -> Result<WebDAVConnectionResult> {
        info!("🔍 Testing WebDAV connection to: {}", self.config.server_url);

        // Validate configuration first
        if let Err(e) = self.config.validate() {
            return Ok(WebDAVConnectionResult {
                success: false,
                message: format!("Configuration error: {}", e),
                server_version: None,
                server_type: None,
            });
        }

        // Perform protocol detection if needed
        let working_protocol = match self.detect_working_protocol().await {
            Ok(protocol) => {
                info!("✅ Protocol detection successful: {}", protocol);
                protocol
            }
            Err(e) => {
                error!("❌ Protocol detection failed: {}", e);
                return Ok(WebDAVConnectionResult {
                    success: false,
                    message: format!("Protocol detection failed: {}", e),
                    server_version: None,
                    server_type: None,
                });
            }
        };

        // Test basic connectivity with OPTIONS request using detected protocol
        match self.test_options_request().await {
            Ok((server_version, server_type)) => {
                let effective_url = self.get_effective_server_url();
                info!("✅ WebDAV connection successful using {} ({})", working_protocol.to_uppercase(), effective_url);
                Ok(WebDAVConnectionResult {
                    success: true,
                    message: format!("Connection successful using {}", working_protocol.to_uppercase()),
                    server_version,
                    server_type,
                })
            }
            Err(e) => {
                error!("❌ WebDAV connection failed: {}", e);
                Ok(WebDAVConnectionResult {
                    success: false,
                    message: format!("Connection failed: {}", e),
                    server_version: None,
                    server_type: None,
                })
            }
        }
    }

    /// Tests WebDAV connection with provided configuration (static method)
    pub async fn test_connection_with_config(test_config: &WebDAVTestConnection) -> Result<WebDAVConnectionResult> {
        let config = WebDAVConfig {
            server_url: test_config.server_url.clone(),
            username: test_config.username.clone(),
            password: test_config.password.clone(),
            watch_folders: vec!["/".to_string()],
            file_extensions: vec![],
            timeout_seconds: 30,
            server_type: test_config.server_type.clone(),
        };

        let service = Self::new(config)?;
        service.test_connection().await
    }

    /// Performs OPTIONS request to test basic connectivity
    async fn test_options_request(&self) -> Result<(Option<String>, Option<String>)> {
        // Create a temporary config with the effective server URL for WebDAV operations
        let effective_server_url = self.get_effective_server_url();
        let temp_config = WebDAVConfig {
            server_url: effective_server_url,
            username: self.config.username.clone(),
            password: self.config.password.clone(),
            watch_folders: self.config.watch_folders.clone(),
            file_extensions: self.config.file_extensions.clone(),
            timeout_seconds: self.config.timeout_seconds,
            server_type: self.config.server_type.clone(),
        };
        let webdav_url = temp_config.webdav_url();
        
        let response = self.client
            .request(Method::OPTIONS, &webdav_url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "OPTIONS request failed with status: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        // Extract server information from headers
        let server_version = response
            .headers()
            .get("server")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let server_type = self.detect_server_type(&response, &server_version).await;

        Ok((server_version, server_type))
    }

    /// Detects the WebDAV server type based on response headers and capabilities
    async fn detect_server_type(
        &self,
        response: &reqwest::Response,
        server_version: &Option<String>,
    ) -> Option<String> {
        // Check server header first
        if let Some(ref server) = server_version {
            let server_lower = server.to_lowercase();
            if server_lower.contains("nextcloud") {
                return Some("nextcloud".to_string());
            }
            if server_lower.contains("owncloud") {
                return Some("owncloud".to_string());
            }
            if server_lower.contains("apache") || server_lower.contains("nginx") {
                // Could be generic WebDAV
            }
        }

        // Check DAV capabilities
        if let Some(dav_header) = response.headers().get("dav") {
            if let Ok(dav_str) = dav_header.to_str() {
                debug!("DAV capabilities: {}", dav_str);
                // Different servers expose different DAV levels
                if dav_str.contains("3") {
                    return Some("webdav_level_3".to_string());
                }
            }
        }

        // Test for Nextcloud/ownCloud specific endpoints
        if self.test_nextcloud_capabilities().await.is_ok() {
            return Some("nextcloud".to_string());
        }

        Some("generic".to_string())
    }

    /// Tests for Nextcloud-specific capabilities
    async fn test_nextcloud_capabilities(&self) -> Result<()> {
        let effective_server_url = self.get_effective_server_url();
        let capabilities_url = format!("{}/ocs/v1.php/cloud/capabilities", 
            effective_server_url.trim_end_matches('/'));

        let response = self.client
            .get(&capabilities_url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .header("OCS-APIRequest", "true")
            .send()
            .await?;

        if response.status().is_success() {
            debug!("Nextcloud capabilities endpoint accessible");
            Ok(())
        } else {
            Err(anyhow!("Nextcloud capabilities not accessible"))
        }
    }

    /// Tests PROPFIND request on root directory
    pub async fn test_propfind(&self, path: &str) -> Result<()> {
        let url = self.get_url_for_path(path);
        
        debug!("🧪 Testing PROPFIND for path '{}' at URL '{}'", path, url);
        
        // First, check server capabilities if this is the first PROPFIND
        if path == "/" || path.is_empty() {
            match self.validate_webdav_capabilities(&url).await {
                Ok(capabilities) => {
                    info!("✅ WebDAV capabilities validated: DAV={}, Methods={}", 
                          capabilities.dav_compliance, capabilities.allowed_methods);
                }
                Err(e) => {
                    warn!("⚠️ WebDAV capability validation failed (continuing anyway): {}", e);
                }
            }
        }
        
        let propfind_body = r#"<?xml version="1.0" encoding="utf-8"?>
            <D:propfind xmlns:D="DAV:">
                <D:prop>
                    <D:displayname/>
                    <D:getcontentlength/>
                    <D:getlastmodified/>
                    <D:getetag/>
                    <D:resourcetype/>
                </D:prop>
            </D:propfind>"#;

        let response = self.authenticated_request(
            Method::from_bytes(b"PROPFIND")?,
            &url,
            Some(propfind_body.to_string()),
            Some(vec![
                ("Depth", "1"),
                ("Content-Type", "application/xml"),
            ]),
        ).await?;

        if response.status().as_u16() == 207 {
            debug!("✅ PROPFIND successful for path: {}", path);
            Ok(())
        } else {
            Err(anyhow!(
                "PROPFIND failed for path '{}' with status: {} - {}",
                path,
                response.status(),
                response.text().await.unwrap_or_default()
            ))
        }
    }

    /// Validates WebDAV server capabilities to help diagnose configuration issues
    async fn validate_webdav_capabilities(&self, url: &str) -> Result<ServerCapabilities> {
        debug!("🔍 Validating WebDAV capabilities for URL: {}", url);
        
        let options_response = self.authenticated_request(
            reqwest::Method::OPTIONS,
            url,
            None,
            None,
        ).await?;

        let dav_header = options_response
            .headers()
            .get("dav")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let allow_header = options_response
            .headers()
            .get("allow")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let server_header = options_response
            .headers()
            .get("server")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Check if PROPFIND is in the allowed methods
        if !allow_header.to_uppercase().contains("PROPFIND") {
            warn!("⚠️ PROPFIND method not listed in server's Allow header: {}", allow_header);
            warn!("💡 This suggests WebDAV may not be properly enabled on this endpoint");
        }

        // Check DAV compliance level
        if dav_header.is_empty() {
            warn!("⚠️ No DAV header found - this endpoint may not support WebDAV");
        } else {
            debug!("📋 Server DAV compliance: {}", dav_header);
        }

        if let Some(ref server) = server_header {
            debug!("🖥️ Server software: {}", server);
        }

        Ok(ServerCapabilities {
            dav_compliance: dav_header.clone(),
            allowed_methods: allow_header,
            server_software: server_header,
            supports_etag: dav_header.contains("1") || dav_header.contains("2"),
            supports_depth_infinity: dav_header.contains("1"),
            infinity_depth_tested: false,
            infinity_depth_works: false,
            last_checked: std::time::Instant::now(),
        })
    }

    // ============================================================================
    // HTTP Request Methods with Simple Retry Logic
    // ============================================================================

    /// Calculates retry delay with jitter to prevent thundering herd problems
    /// 
    /// Uses the formula: delay = base_delay * multiplier^attempt * (0.9 + rand(0.0..0.2))
    /// This adds 10-20% randomization to break up synchronized retries while maintaining
    /// exponential backoff behavior.
    fn calculate_retry_delay_with_jitter(&self, attempt: u32, base_delay: u64) -> u64 {
        let exponential_delay = (base_delay as f64 * self.retry_config.backoff_multiplier.powi(attempt as i32)) as u64;
        
        // Add jitter: 0.9 to 1.1 multiplier (10-20% randomization)
        let jitter_multiplier = 0.9 + rand::rng().random::<f64>() * 0.2;
        
        let jittered_delay = (exponential_delay as f64 * jitter_multiplier) as u64;
        
        // Ensure we don't exceed max_delay_ms
        std::cmp::min(jittered_delay, self.retry_config.max_delay_ms)
    }

    /// Performs authenticated request with simple retry logic (simplified from complex error recovery)
    pub async fn authenticated_request(
        &self,
        method: Method,
        url: &str,
        body: Option<String>,
        headers: Option<Vec<(&str, &str)>>,
    ) -> Result<reqwest::Response> {
        // URLs may be built from server-provided hrefs; apply the same
        // destination check as for the configured server.
        let parsed = url::Url::parse(url).map_err(|_| anyhow!("Invalid request URL"))?;
        check_url_without_dns(&parsed).map_err(|e| anyhow!("{}", e))?;

        let mut attempt = 0;
        let mut delay = self.retry_config.initial_delay_ms;

        // Build custom User-Agent header using centralized function
        let user_agent = build_user_agent();

        // Enhanced debug logging for HTTP requests
        debug!("🌐 HTTP Request Details:");
        debug!("   Method: {}", method);
        debug!("   URL: {}", url);
        debug!("   Username: {}", self.config.username);
        debug!("   User-Agent: {}", user_agent);
        if let Some(ref headers_list) = headers {
            debug!("   Headers: {:?}", headers_list);
        }
        if let Some(ref body_content) = body {
            debug!("   Body length: {} bytes", body_content.len());
            debug!("   Body preview: {}", 
                if body_content.len() > 200 { 
                    format!("{}...", &body_content[..200])
                } else { 
                    body_content.clone() 
                });
        }

        loop {
            let mut request = self.client
                .request(method.clone(), url)
                .basic_auth(&self.config.username, Some(&self.config.password))
                .header("User-Agent", &user_agent);

            if let Some(ref body_content) = body {
                request = request.body(body_content.clone());
            }

            if let Some(ref headers_list) = headers {
                for (key, value) in headers_list {
                    request = request.header(*key, *value);
                }
            }

            debug!("📤 Sending HTTP {} request to: {}", method, url);
            match request.send().await {
                Ok(response) => {
                    let status = response.status();
                    debug!("📥 HTTP Response: {} {}", status.as_u16(), status.canonical_reason().unwrap_or(""));
                    
                    // Log response headers for debugging
                    for (key, value) in response.headers() {
                        if key.as_str().to_lowercase().contains("allow") || 
                           key.as_str().to_lowercase().contains("dav") ||
                           key.as_str().to_lowercase().contains("server") {
                            debug!("   Response header: {}: {:?}", key, value);
                        }
                    }
                    
                    if status.is_success() || status.as_u16() == 207 {
                        debug!("✅ HTTP request successful: {} {}", status.as_u16(), status.canonical_reason().unwrap_or(""));
                        return Ok(response);
                    }

                    // Rate limited or temporarily unavailable: wait as the server
                    // asks (bounded), and give up after max_retries like any other retry.
                    if status.as_u16() == 429 || status.as_u16() == 503 {
                        if attempt >= self.retry_config.max_retries {
                            return Err(anyhow!("Request failed: {} after {} attempts - {}", status, attempt + 1,
                                response.text().await.unwrap_or_default()));
                        }
                        let fallback_ms = if status.as_u16() == 429 {
                            self.retry_config.rate_limit_backoff_ms
                        } else {
                            self.calculate_retry_delay_with_jitter(attempt, self.retry_config.initial_delay_ms)
                        };
                        let wait = Self::retry_after_wait(&response).unwrap_or(Duration::from_millis(fallback_ms));
                        warn!("{} from {} {}, waiting {:?} before retry (attempt {}/{})",
                              status, method, url, wait, attempt + 1, self.retry_config.max_retries);
                        sleep(wait).await;
                        attempt += 1;
                        continue;
                    }

                    // Handle client errors (don't retry)
                    if status.is_client_error() && status.as_u16() != 429 {
                        let error_body = response.text().await.unwrap_or_default();
                        
                        if status.as_u16() == 405 {
                            error!("🚫 HTTP 405 Method Not Allowed for {} {} (server type: {:?}, WebDAV base URL: {}). \
                                    WebDAV may not be enabled on the server or the endpoint URL may be wrong.",
                                   method, url, self.config.server_type, self.config.webdav_url());
                            return Err(anyhow!(
                                "WebDAV {} method not allowed (405) at URL: {}. This typically means WebDAV is not properly enabled on the server or the URL is incorrect. \
                                Server type: {:?}, Base URL: {}, WebDAV URL: {}. Error details: {}", 
                                method, url, self.config.server_type, self.config.server_url, self.config.webdav_url(), error_body
                            ));
                        }
                        
                        return Err(anyhow!("Client error: {} - {}", status, error_body));
                    }

                    // Handle server errors (retry)
                    if status.is_server_error() && attempt < self.retry_config.max_retries {
                        let jittered_delay = self.calculate_retry_delay_with_jitter(attempt, self.retry_config.initial_delay_ms);
                        warn!("Server error {}, retrying in {}ms with jitter (attempt {}/{}, base exponential: {}ms)", 
                            status, jittered_delay, attempt + 1, self.retry_config.max_retries, delay);
                        
                        sleep(Duration::from_millis(jittered_delay)).await;
                        delay = std::cmp::min(
                            (delay as f64 * self.retry_config.backoff_multiplier) as u64,
                            self.retry_config.max_delay_ms
                        );
                        attempt += 1;
                        continue;
                    }

                    return Err(anyhow!("Request failed: {} - {}", status,
                        response.text().await.unwrap_or_default()));
                }
                Err(e) => {
                    if attempt < self.retry_config.max_retries {
                        let jittered_delay = self.calculate_retry_delay_with_jitter(attempt, self.retry_config.initial_delay_ms);
                        warn!("Request error: {}, retrying in {}ms with jitter (attempt {}/{}, base exponential: {}ms)", 
                            e, jittered_delay, attempt + 1, self.retry_config.max_retries, delay);
                        
                        sleep(Duration::from_millis(jittered_delay)).await;
                        delay = std::cmp::min(
                            (delay as f64 * self.retry_config.backoff_multiplier) as u64,
                            self.retry_config.max_delay_ms
                        );
                        attempt += 1;
                        continue;
                    }

                    return Err(anyhow!("Request failed after {} attempts: {}", 
                        self.retry_config.max_retries, e));
                }
            }
        }
    }

    /// Wait requested by a response's Retry-After header, if present and parseable.
    fn retry_after_wait(response: &reqwest::Response) -> Option<Duration> {
        let value = response.headers().get(reqwest::header::RETRY_AFTER)?.to_str().ok()?;
        retry_after_delay(value, chrono::Utc::now(), MAX_RETRY_AFTER)
    }

    // ============================================================================
    // URL Management Helper Methods (Previously separate module)
    // ============================================================================

    /// Base WebDAV URL built from the protocol that was detected to work.
    fn effective_webdav_url(&self) -> String {
        let mut config = self.config.clone();
        config.server_url = self.get_effective_server_url();
        config.webdav_url()
    }

    /// Gets the WebDAV URL for a specific path
    pub fn get_url_for_path(&self, path: &str) -> String {
        let final_url = join_url(&self.effective_webdav_url(), path);
        debug!("🔗 URL for path '{}' (server type {:?}): '{}'", path, self.config.server_type, final_url);
        final_url
    }

    /// Converts a percent-encoded href from a PROPFIND response into the decoded
    /// relative path used internally.
    ///
    /// Input:  "/remote.php/dav/files/username/My%20Photos/image.jpg"
    /// Output: "/My Photos/image.jpg"
    pub fn href_to_relative_path(&self, href: &str) -> String {
        self.convert_to_relative_path(&decode_href(href))
    }

    /// Convert file paths to the proper URL format for the server
    pub fn path_to_url(&self, relative_path: &str) -> String {
        self.get_url_for_path(relative_path)
    }

    /// Removes the server-specific prefix from an already-decoded WebDAV path.
    /// Paths without the prefix are returned unchanged.
    pub fn convert_to_relative_path(&self, full_webdav_path: &str) -> String {
        // For Nextcloud/ownCloud, remove the server-specific prefixes
        if let Some(server_type) = &self.config.server_type {
            if server_type == "nextcloud" {
                let username = &self.config.username;
                let prefix = format!("/remote.php/dav/files/{}", username);
                
                if full_webdav_path.starts_with(&prefix) {
                    let relative = &full_webdav_path[prefix.len()..];
                    return if relative.is_empty() { "/" } else { relative }.to_string();
                }
            } else if server_type == "owncloud" {
                // ownCloud uses /remote.php/webdav prefix
                if full_webdav_path.starts_with("/remote.php/webdav") {
                    let relative = &full_webdav_path[18..]; // Remove "/remote.php/webdav"
                    return if relative.is_empty() { "/" } else { relative }.to_string();
                }
            } else if server_type == "generic" {
                // For generic servers, remove the /webdav prefix if present
                if full_webdav_path.starts_with("/webdav") {
                    let relative = &full_webdav_path[7..]; // Remove "/webdav"
                    return if relative.is_empty() { "/" } else { relative }.to_string();
                }
            }
        }
        
        // For other servers, return as-is
        full_webdav_path.to_string()
    }

    // ============================================================================
    // File Discovery Methods (Previously separate discovery module)
    // ============================================================================


    /// Discovers files in a directory (optionally recursive); directory entries are dropped.
    pub async fn discover_files(&self, directory_path: &str, recursive: bool) -> Result<Vec<FileIngestionInfo>> {
        Ok(self.discover_tree(directory_path, recursive, None).await?.files)
    }

    /// Discovers both files and directories with their ETags for directory tracking
    pub async fn discover_files_and_directories(&self, directory_path: &str, recursive: bool) -> Result<WebDAVDiscoveryResult> {
        self.discover_tree(directory_path, recursive, None).await.map(Into::into)
    }

    /// Discovers both files and directories with basic progress tracking (simplified)
    pub async fn discover_files_and_directories_with_progress(
        &self,
        directory_path: &str,
        recursive: bool,
        _progress: Option<&SyncProgress> // Simplified: just placeholder for API compatibility
    ) -> Result<WebDAVDiscoveryResult> {
        self.discover_files_and_directories(directory_path, recursive).await
    }

    /// Discovery that records per-directory outcomes in the `SourceErrorTracker` so
    /// directories in failure cooldown are skipped and new failures are tracked.
    pub async fn discover_files_and_directories_with_error_tracking(
        &self,
        directory_path: &str,
        recursive: bool,
        user_id: uuid::Uuid,
        error_tracker: &SourceErrorTracker,
        source_id: Option<uuid::Uuid>,
    ) -> Result<WebDAVDiscoveryResult> {
        let tracking = DiscoveryTracking { user_id, source_id, tracker: error_tracker.clone() };
        self.discover_tree(directory_path, recursive, Some(&tracking)).await.map(Into::into)
    }

    /// Scans `directory_path` (recursively when requested) and reports which
    /// subdirectories could not be listed. The root directory itself failing to
    /// list is an error; a failing subdirectory is recorded in
    /// `failed_directories` and the rest of the tree is still returned.
    pub async fn discover_tree(
        &self,
        directory_path: &str,
        recursive: bool,
        tracking: Option<&DiscoveryTracking>,
    ) -> Result<WebDAVScanResult> {
        let request_id = uuid::Uuid::new_v4();
        let start_time = Instant::now();
        info!("[{}] 🔍 Discovering '{}' (recursive: {}, tracked: {}, user_agent: '{}')",
              request_id, directory_path, recursive, tracking.is_some(), build_user_agent());

        let result = if recursive {
            self.scan_tree(directory_path, tracking).await
        } else {
            match self.scan_directory_tracked(directory_path, tracking).await {
                Ok(Some(scan)) => Ok(scan),
                Ok(None) => Err(anyhow!("Directory '{}' is skipped due to previous failures", directory_path)),
                Err(e) => Err(e),
            }
        };

        match &result {
            Ok(scan) => info!("[{}] ✅ Discovery completed for '{}': {} files, {} directories, {} failed directories in {:.2}s",
                              request_id, directory_path, scan.files.len(), scan.directories.len(),
                              scan.failed_directories.len(), start_time.elapsed().as_secs_f64()),
            Err(e) => error!("[{}] ❌ Discovery failed for '{}' after {:.2}s: {}",
                             request_id, directory_path, start_time.elapsed().as_secs_f64(), e),
        }
        result
    }

    /// Breadth-first walk of `directory_path` in bounded concurrent batches.
    async fn scan_tree(&self, directory_path: &str, tracking: Option<&DiscoveryTracking>) -> Result<WebDAVScanResult> {
        let mut result = WebDAVScanResult::default();
        let mut directories_to_scan = vec![directory_path.to_string()];
        let mut scanned_directories: HashSet<String> = HashSet::new();
        let semaphore = Arc::new(Semaphore::new(self.concurrency_config.max_concurrent_scans));

        while !directories_to_scan.is_empty() {
            let batch_size = std::cmp::min(directories_to_scan.len(), self.concurrency_config.max_concurrent_scans);
            let current_batch: Vec<String> = directories_to_scan.drain(..batch_size).collect();
            debug!("Processing batch of {} directories, {} remaining in queue",
                   current_batch.len(), directories_to_scan.len());

            let tasks = current_batch.into_iter().filter_map(|dir| {
                if !scanned_directories.insert(dir.clone()) {
                    return None;
                }
                let permit = semaphore.clone();
                let service = self.clone();
                let tracking = tracking.cloned();
                Some(async move {
                    let _permit = permit.acquire().await.unwrap();
                    let outcome = service.scan_directory_tracked(&dir, tracking.as_ref()).await;
                    (dir, outcome)
                })
            });

            for (scanned_dir, outcome) in futures_util::future::join_all(tasks).await {
                let is_root = scanned_dir == directory_path;
                match outcome {
                    Ok(Some(scan)) => {
                        debug!("Directory '{}' scan complete: {} files, {} subdirectories",
                               scanned_dir, scan.files.len(), scan.directories.len());
                        result.files.extend(scan.files);
                        for dir in scan.directories {
                            if !scanned_directories.contains(&dir.relative_path) {
                                directories_to_scan.push(dir.relative_path.clone());
                            }
                            result.directories.push(dir);
                        }
                        if is_root {
                            result.root = scan.root;
                        }
                    }
                    Ok(None) if is_root => {
                        return Err(anyhow!("Directory '{}' is skipped due to previous failures", scanned_dir));
                    }
                    Ok(None) => {
                        info!("⏭️ Skipping directory '{}' due to previous failures; its subtree is not scanned", scanned_dir);
                        result.failed_directories.push(scanned_dir);
                    }
                    Err(e) if is_root => return Err(e),
                    Err(e) => {
                        warn!("Failed to scan directory '{}': {}; its subtree is not scanned", scanned_dir, e);
                        result.failed_directories.push(scanned_dir);
                    }
                }
            }
        }

        info!("Recursive scan completed. Found {} files and {} directories ({} directories failed)",
              result.files.len(), result.directories.len(), result.failed_directories.len());
        Ok(result)
    }

    /// Scans one directory, consulting and updating the error tracker when present.
    /// Returns `Ok(None)` when the tracker says the directory is in failure cooldown.
    async fn scan_directory_tracked(
        &self,
        directory_path: &str,
        tracking: Option<&DiscoveryTracking>,
    ) -> Result<Option<WebDAVScanResult>> {
        let Some(tracking) = tracking else {
            return self.scan_directory(directory_path).await.map(Some);
        };

        match tracking.tracker.should_skip_resource(
            tracking.user_id, ErrorSourceType::WebDAV, tracking.source_id, directory_path,
        ).await {
            Ok(true) => return Ok(None),
            Ok(false) => {}
            Err(e) => warn!("Failed to check failure status for directory '{}': {}", directory_path, e),
        }

        let started = Instant::now();
        let result = self.scan_directory(directory_path).await;
        match &result {
            Ok(_) => {
                if let Err(e) = tracking.tracker.mark_success(
                    tracking.user_id, ErrorSourceType::WebDAV, tracking.source_id, directory_path,
                ).await {
                    debug!("Failed to mark directory scan as successful: {}", e);
                }
            }
            Err(e) => {
                let context = ErrorContext {
                    resource_path: directory_path.to_string(),
                    source_id: tracking.source_id,
                    operation: "discover_directory".to_string(),
                    response_time: Some(started.elapsed()),
                    response_size: None,
                    server_type: self.config.server_type.clone(),
                    server_version: None,
                    additional_context: std::collections::HashMap::new(),
                };
                if let Err(track_error) = tracking.tracker.track_error(
                    tracking.user_id, ErrorSourceType::WebDAV, tracking.source_id, directory_path, e, context,
                ).await {
                    warn!("Failed to track error for directory '{}': {}", directory_path, track_error);
                }
            }
        }
        result.map(Some)
    }

    /// Lists one directory (Depth: 1), trying the fallback base URLs on 405.
    async fn scan_directory(&self, directory_path: &str) -> Result<WebDAVScanResult> {
        let primary_url = self.get_url_for_path(directory_path);
        let primary_error = match self.scan_directory_at_url(directory_path, &primary_url).await {
            Ok(result) => return Ok(result),
            Err(e) if e.to_string().contains("405") || e.to_string().contains("Method Not Allowed") => e,
            Err(e) => return Err(e),
        };
        warn!("🔄 Primary WebDAV URL failed with 405 ({}), trying fallback URLs...", primary_error);

        let fallback_urls = self.config.webdav_fallback_urls();
        for (i, fallback_base_url) in fallback_urls.iter().enumerate() {
            let fallback_url = join_url(fallback_base_url, directory_path);
            info!("🔄 Trying fallback URL #{}: {}", i + 1, fallback_url);
            match self.scan_directory_at_url(directory_path, &fallback_url).await {
                Ok(result) => {
                    info!("✅ Fallback URL #{} succeeded: {}", i + 1, fallback_url);
                    warn!("💡 Consider updating your server type configuration to use this URL pattern");
                    return Ok(result);
                }
                Err(e) => warn!("❌ Fallback URL #{} failed: {} - {}", i + 1, fallback_url, e),
            }
        }

        Err(anyhow!(
            "All WebDAV URLs failed for directory '{}'. Primary URL and {} fallback URLs were tried. \
            This suggests WebDAV is not properly configured on the server or the server type is incorrect.",
            directory_path, fallback_urls.len()
        ))
    }

    /// PROPFIND Depth: 1 at `url`; splits the response into the directory's own
    /// entry, its subdirectories and its files.
    async fn scan_directory_at_url(&self, directory_path: &str, url: &str) -> Result<WebDAVScanResult> {
        debug!("🔍 WebDAV directory scan - Path: '{}', URL: '{}', Server type: {:?}",
               directory_path, url, self.config.server_type);

        let response = self.authenticated_request(
            Method::from_bytes(b"PROPFIND")?,
            url,
            Some(PROPFIND_LISTING_BODY.to_string()),
            Some(vec![("Depth", "1"), ("Content-Type", "application/xml")]),
        ).await.map_err(|e| {
            error!("❌ PROPFIND request failed for directory '{}' at URL '{}': {}", directory_path, url, e);
            e
        })?;

        let body = response.text().await?;
        let items = self.process_file_infos(parse_propfind_response_with_directories(&body)?);

        let mut result = WebDAVScanResult::default();
        let normalized_directory_path = directory_path.trim_end_matches('/');
        for item in items {
            if item.relative_path.trim_end_matches('/') == normalized_directory_path {
                result.root = Some(item);
            } else if item.is_directory {
                result.directories.push(item);
            } else {
                result.files.push(item);
            }
        }

        debug!("Found {} files and {} directories in: {}", result.files.len(), result.directories.len(), directory_path);
        Ok(result)
    }

    /// Estimates crawl time and resource requirements
    pub async fn estimate_crawl(&self) -> Result<WebDAVCrawlEstimate> {
        info!("📊 Estimating WebDAV crawl requirements");
        
        let start_time = Instant::now();
        let mut total_directories = 0;
        let mut total_files = 0;
        let mut sample_scan_time = Duration::from_millis(0);
        
        // Sample the first few watch folders to estimate
        for (index, watch_folder) in self.config.watch_folders.iter().enumerate() {
            if index >= 3 { break; } // Only sample first 3 folders
            
            let scan_start = Instant::now();
            match self.discover_files_and_directories(watch_folder, false).await {
                Ok(result) => {
                    total_directories += result.directories.len();
                    total_files += result.files.len();
                    sample_scan_time += scan_start.elapsed();
                }
                Err(e) => {
                    warn!("Failed to scan folder '{}' for estimation: {}", watch_folder, e);
                }
            }
        }
        
        // Simple estimation based on sample
        let avg_scan_time_per_folder = if total_directories > 0 {
            sample_scan_time.as_millis() as f64 / total_directories as f64
        } else {
            100.0 // Default 100ms per folder
        };
        
        let estimated_total_scan_time = Duration::from_millis(
            (avg_scan_time_per_folder * total_directories as f64 * self.config.watch_folders.len() as f64) as u64
        );
        
        Ok(WebDAVCrawlEstimate {
            folders: vec![], // Simplified: not building detailed folder info for basic estimation
            total_files: (total_files * self.config.watch_folders.len()) as i64,
            total_supported_files: (total_files * self.config.watch_folders.len()) as i64, // Assume all files are supported
            total_estimated_time_hours: (estimated_total_scan_time.as_secs_f64() / 3600.0) as f32,
            total_size_mb: (total_files * 2) as f64, // Rough estimate in MB
        })
    }

    /// Deduplicates files across multiple folders
    pub fn deduplicate_files(&self, files: Vec<FileIngestionInfo>) -> Vec<FileIngestionInfo> {
        let mut seen = HashSet::new();
        files.into_iter().filter(|file| {
            seen.insert(file.relative_path.clone())
        }).collect()
    }

    /// Filters files by date for incremental syncs
    pub fn filter_files_by_date(&self, files: Vec<FileIngestionInfo>, since: chrono::DateTime<chrono::Utc>) -> Vec<FileIngestionInfo> {
        files.into_iter().filter(|file| {
            file.last_modified.map_or(false, |modified| modified > since)
        }).collect()
    }

    // ============================================================================
    // Error Tracking Helper Methods
    // ============================================================================

    /// Creates a SourceErrorTracker for use with enhanced discovery methods
    pub fn create_error_tracker(&self, db: crate::db::Database) -> SourceErrorTracker {
        SourceErrorTracker::new(db)
    }

    /// Helper method to check if a directory should be skipped due to previous failures
    /// This is useful for individual operations that want to integrate error tracking
    pub async fn should_skip_directory_for_failures(
        &self,
        directory_path: &str,
        user_id: uuid::Uuid,
        error_tracker: &SourceErrorTracker,
        source_id: Option<uuid::Uuid>,
    ) -> Result<bool> {
        error_tracker.should_skip_resource(
            user_id,
            ErrorSourceType::WebDAV,
            source_id,
            directory_path,
        ).await
    }

    /// Helper method to mark a directory operation as successful
    /// This resolves any previous failure tracking for the directory
    pub async fn mark_directory_operation_success(
        &self,
        directory_path: &str,
        user_id: uuid::Uuid,
        error_tracker: &SourceErrorTracker,
        source_id: Option<uuid::Uuid>,
    ) -> Result<()> {
        error_tracker.mark_success(
            user_id,
            ErrorSourceType::WebDAV,
            source_id,
            directory_path,
        ).await
    }

    /// Helper method to track a directory operation failure
    /// This records the error and applies appropriate retry logic
    pub async fn mark_directory_operation_failure(
        &self,
        directory_path: &str,
        user_id: uuid::Uuid,
        error_tracker: &SourceErrorTracker,
        source_id: Option<uuid::Uuid>,
        error: &anyhow::Error,
        operation: &str,
        duration: std::time::Duration,
    ) -> Result<uuid::Uuid> {
        let context = ErrorContext {
            resource_path: directory_path.to_string(),
            source_id,
            operation: operation.to_string(),
            response_time: Some(duration),
            response_size: None,
            server_type: self.config.server_type.clone(),
            server_version: None,
            additional_context: std::collections::HashMap::new(),
        };

        error_tracker.track_error(
            user_id,
            ErrorSourceType::WebDAV,
            source_id,
            directory_path,
            error,
            context,
        ).await
    }

    // ============================================================================
    // File Operations
    // ============================================================================

    /// Discovers all files in watch folders
    pub async fn discover_all_files(&self) -> Result<Vec<FileIngestionInfo>> {
        info!("🔍 Discovering all files in watch folders");
        let mut all_files = Vec::new();

        for watch_folder in &self.config.watch_folders {
            info!("📁 Scanning watch folder: {}", watch_folder);
            
            match self.discover_files(watch_folder, true).await {
                Ok(files) => {
                    info!("✅ Found {} files in {}", files.len(), watch_folder);
                    all_files.extend(files);
                }
                Err(e) => {
                    error!("❌ Failed to scan watch folder '{}': {}", watch_folder, e);
                    return Err(anyhow!("Failed to scan watch folder '{}': {}", watch_folder, e));
                }
            }
        }

        // Deduplicate files across folders
        let deduplicated_files = self.deduplicate_files(all_files);
        
        info!("🎯 Total unique files discovered: {}", deduplicated_files.len());
        Ok(deduplicated_files)
    }

    /// Discovers files changed since a specific date (for incremental syncs)
    pub async fn discover_changed_files(&self, since: chrono::DateTime<chrono::Utc>) -> Result<Vec<FileIngestionInfo>> {
        info!("🔍 Discovering files changed since: {}", since);
        
        let all_files = self.discover_all_files().await?;
        let changed_files = self.filter_files_by_date(all_files, since);
        
        info!("📈 Found {} files changed since {}", changed_files.len(), since);
        Ok(changed_files)
    }

    /// Discovers files in a specific directory
    pub async fn discover_files_in_directory(&self, directory_path: &str, recursive: bool) -> Result<Vec<FileIngestionInfo>> {
        info!("🔍 Discovering files in directory: {} (recursive: {})", directory_path, recursive);
        self.discover_files(directory_path, recursive).await
    }

    /// Downloads a file from WebDAV server by path
    pub async fn download_file(&self, file_path: &str) -> Result<Vec<u8>> {
        // Convert full WebDAV paths to relative paths to prevent double path construction
        let relative_path = self.convert_to_relative_path(file_path);
        let file_name = relative_path.trim_end_matches('/').rsplit('/').next().unwrap_or("").to_string();
        Ok(self.fetch_file(&relative_path, &file_name).await?.0)
    }

    /// Downloads a file from WebDAV server using FileIngestionInfo
    pub async fn download_file_info(&self, file_info: &FileIngestionInfo) -> Result<Vec<u8>> {
        Ok(self.fetch_file(&file_info.relative_path, &file_info.name).await?.0)
    }

    /// GETs `relative_path` and returns the body with the server's media type,
    /// after checking the body is complete and is not an HTML page standing in
    /// for the file.
    async fn fetch_file(&self, relative_path: &str, file_name: &str) -> Result<(Vec<u8>, Option<String>)> {
        let _permit = self.download_semaphore.acquire().await?;
        debug!("⬇️ Downloading file: {}", relative_path);

        let url = self.get_url_for_path(relative_path);
        let response = self.authenticated_request(reqwest::Method::GET, &url, None, None).await?;
        if !response.status().is_success() {
            return Err(anyhow!("Failed to download file '{}': HTTP {}", relative_path, response.status()));
        }

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|header| header.to_str().ok())
            .map(|s| s.split(';').next().unwrap_or(s).trim().to_ascii_lowercase());
        let content_length = response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|header| header.to_str().ok())
            .and_then(|s| s.trim().parse::<u64>().ok());

        let content = response.bytes().await
            .map_err(|e| anyhow!("Failed to read body of '{}': {}", relative_path, e))?;
        verify_download(relative_path, file_name, &content, content_length, content_type.as_deref())?;
        debug!("✅ Downloaded {} bytes for file: {}", content.len(), relative_path);
        Ok((content.to_vec(), content_type))
    }

    /// Downloads multiple files concurrently
    pub async fn download_files(&self, files: &[FileIngestionInfo]) -> Result<Vec<(FileIngestionInfo, Result<Vec<u8>>)>> {
        info!("⬇️ Downloading {} files concurrently", files.len());
        
        let tasks = files.iter().map(|file| {
            let file_clone = file.clone();
            let service_clone = self.clone();
            
            async move {
                let result = service_clone.download_file_info(&file_clone).await;
                (file_clone, result)
            }
        });

        let results = futures_util::future::join_all(tasks).await;
        
        let success_count = results.iter().filter(|(_, result)| result.is_ok()).count();
        let failure_count = results.len() - success_count;
        
        info!("📊 Download completed: {} successful, {} failed", success_count, failure_count);
        
        Ok(results)
    }

    /// Downloads a file with enhanced MIME type detection based on content
    /// 
    /// This method downloads the file and performs content-based MIME type detection
    /// using magic bytes, providing more accurate type identification than the initial
    /// discovery phase which only has access to filenames and server-provided types.
    /// 
    /// # Arguments
    /// * `file_info` - The file information from WebDAV discovery
    /// 
    /// # Returns
    /// A `WebDAVDownloadResult` containing the file content, updated file info, and MIME detection details
    pub async fn download_file_with_mime_detection(&self, file_info: &FileIngestionInfo) -> Result<WebDAVDownloadResult> {
        debug!("⬇️🔍 Downloading file with MIME detection: {}", file_info.relative_path);
        let (content, server_content_type) = self.fetch_file(&file_info.relative_path, &file_info.name).await?;

        // Perform content-based MIME type detection
        let mime_detection_result = detect_mime_from_content(
            &content,
            &file_info.name,
            server_content_type.as_deref()
        );

        // Check if MIME type should be updated
        let mime_type_updated = mime_detection_result.mime_type != file_info.mime_type;
        
        if mime_type_updated {
            debug!("🔄 MIME type updated for {}: '{}' -> '{}' (method: {:?}, confidence: {:?})",
                   file_info.name,
                   file_info.mime_type,
                   mime_detection_result.mime_type,
                   mime_detection_result.detection_method,
                   mime_detection_result.confidence);
        } else {
            debug!("✅ MIME type confirmed for {}: '{}' (method: {:?}, confidence: {:?})",
                   file_info.name,
                   mime_detection_result.mime_type,
                   mime_detection_result.detection_method,
                   mime_detection_result.confidence);
        }

        // Create updated file info if MIME type changed
        let updated_file_info = if mime_type_updated {
            let mut updated = file_info.clone();
            updated.mime_type = mime_detection_result.mime_type.clone();
            updated
        } else {
            file_info.clone()
        };

        Ok(WebDAVDownloadResult {
            content,
            file_info: updated_file_info,
            mime_detection: mime_detection_result,
            mime_type_updated,
        })
    }

    /// Downloads multiple files with MIME type detection concurrently
    /// 
    /// Similar to `download_files` but includes content-based MIME type detection
    /// for each downloaded file.
    /// 
    /// # Arguments
    /// * `files` - The files to download
    /// 
    /// # Returns
    /// A vector of tuples containing the original file info and download result
    pub async fn download_files_with_mime_detection(&self, files: &[FileIngestionInfo]) -> Result<Vec<(FileIngestionInfo, Result<WebDAVDownloadResult>)>> {
        info!("⬇️🔍 Downloading {} files with MIME detection concurrently", files.len());
        
        let tasks = files.iter().map(|file| {
            let file_clone = file.clone();
            let service_clone = self.clone();
            
            async move {
                let result = service_clone.download_file_with_mime_detection(&file_clone).await;
                (file_clone, result)
            }
        });

        let results = futures_util::future::join_all(tasks).await;
        
        let success_count = results.iter().filter(|(_, result)| result.is_ok()).count();
        let failure_count = results.len() - success_count;
        let mime_updated_count = results.iter()
            .filter_map(|(_, result)| result.as_ref().ok())
            .filter(|download_result| download_result.mime_type_updated)
            .count();
        
        info!("📊 Download with MIME detection completed: {} successful, {} failed, {} MIME types updated", 
              success_count, failure_count, mime_updated_count);
        
        Ok(results)
    }

    /// Gets file metadata without downloading content
    pub async fn get_file_metadata(&self, file_path: &str) -> Result<FileIngestionInfo> {
        debug!("📋 Getting metadata for file: {}", file_path);
        
        // Convert full WebDAV paths to relative paths to prevent double path construction
        let relative_path = self.convert_to_relative_path(file_path);
        let url = self.get_url_for_path(&relative_path);
        
        let propfind_body = r#"<?xml version="1.0" encoding="utf-8"?>
            <D:propfind xmlns:D="DAV:">
                <D:prop>
                    <D:displayname/>
                    <D:getcontentlength/>
                    <D:getlastmodified/>
                    <D:getetag/>
                    <D:resourcetype/>
                    <D:creationdate/>
                </D:prop>
            </D:propfind>"#;

        let response = self.authenticated_request(
            reqwest::Method::from_bytes(b"PROPFIND")?,
            &url,
            Some(propfind_body.to_string()),
            Some(vec![
                ("Depth", "0"),
                ("Content-Type", "application/xml"),
            ]),
        ).await?;

        let body = response.text().await?;
        let files = parse_propfind_response(&body)?;
        
        files.into_iter()
            .find(|f| f.relative_path == file_path)
            .ok_or_else(|| anyhow!("File metadata not found: {}", file_path))
    }

    /// Checks if a file exists on the WebDAV server
    pub async fn file_exists(&self, file_path: &str) -> Result<bool> {
        match self.get_file_metadata(file_path).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    // ============================================================================
    // Server Capabilities and Health Checks
    // ============================================================================

    /// Gets the server capabilities and features
    pub async fn get_server_capabilities(&self) -> Result<ServerCapabilities> {
        debug!("🔍 Checking server capabilities");
        
        // Create a temporary config with the effective server URL
        let effective_server_url = self.get_effective_server_url();
        let temp_config = WebDAVConfig {
            server_url: effective_server_url,
            username: self.config.username.clone(),
            password: self.config.password.clone(),
            watch_folders: self.config.watch_folders.clone(),
            file_extensions: self.config.file_extensions.clone(),
            timeout_seconds: self.config.timeout_seconds,
            server_type: self.config.server_type.clone(),
        };
        
        let options_response = self.authenticated_request(
            reqwest::Method::OPTIONS,
            &temp_config.webdav_url(),
            None,
            None,
        ).await?;

        let dav_header = options_response
            .headers()
            .get("dav")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let allow_header = options_response
            .headers()
            .get("allow")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let server_header = options_response
            .headers()
            .get("server")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        Ok(ServerCapabilities {
            dav_compliance: dav_header.clone(),
            allowed_methods: allow_header,
            server_software: server_header,
            supports_etag: dav_header.contains("1") || dav_header.contains("2"),
            supports_depth_infinity: dav_header.contains("1"),
            infinity_depth_tested: false, // Will be tested separately if needed
            infinity_depth_works: false,  // Will be updated after testing
            last_checked: std::time::Instant::now(),
        })
    }

    /// Performs a health check on the WebDAV service
    pub async fn health_check(&self) -> Result<HealthStatus> {
        info!("🏥 Performing WebDAV service health check");
        
        let start_time = std::time::Instant::now();
        
        // Test basic connectivity
        let connection_result = self.test_connection().await?;
        if !connection_result.success {
            return Ok(HealthStatus {
                healthy: false,
                message: format!("Connection failed: {}", connection_result.message),
                response_time_ms: start_time.elapsed().as_millis() as u64,
                details: None,
            });
        }

        // Test each watch folder
        for folder in &self.config.watch_folders {
            if let Err(e) = self.test_propfind(folder).await {
                return Ok(HealthStatus {
                    healthy: false,
                    message: format!("Watch folder '{}' is inaccessible: {}", folder, e),
                    response_time_ms: start_time.elapsed().as_millis() as u64,
                    details: Some(serde_json::json!({
                        "failed_folder": folder,
                        "error": e.to_string()
                    })),
                });
            }
        }

        let response_time = start_time.elapsed().as_millis() as u64;
        
        Ok(HealthStatus {
            healthy: true,
            message: "All systems operational".to_string(),
            response_time_ms: response_time,
            details: Some(serde_json::json!({
                "tested_folders": self.config.watch_folders,
                "server_type": connection_result.server_type,
                "server_version": connection_result.server_version
            })),
        })
    }

    // ============================================================================
    // Validation Methods (Previously separate validation module)
    // ============================================================================

    /// Performs comprehensive validation of WebDAV setup and directory tracking
    pub async fn validate_system(&self) -> Result<ValidationReport> {
        let start_time = std::time::Instant::now();
        info!("🔍 Starting WebDAV system validation");

        let mut issues = Vec::new();
        let mut recommendations = Vec::new();
        let mut directories_checked = 0;
        let mut healthy_directories = 0;

        // Test basic connectivity first
        match self.test_connection().await {
            Ok(result) if !result.success => {
                issues.push(ValidationIssue {
                    issue_type: ValidationIssueType::Inaccessible,
                    severity: ValidationSeverity::Critical,
                    directory_path: "/".to_string(),
                    description: "WebDAV server connection failed".to_string(),
                    details: Some(serde_json::json!({
                        "error": result.message
                    })),
                    detected_at: chrono::Utc::now(),
                });
            }
            Err(e) => {
                issues.push(ValidationIssue {
                    issue_type: ValidationIssueType::Inaccessible,
                    severity: ValidationSeverity::Critical,
                    directory_path: "/".to_string(),
                    description: "WebDAV server connection error".to_string(),
                    details: Some(serde_json::json!({
                        "error": e.to_string()
                    })),
                    detected_at: chrono::Utc::now(),
                });
            }
            _ => {}
        }

        // Test each watch folder
        for folder in &self.config.watch_folders {
            directories_checked += 1;
            
            match self.test_propfind(folder).await {
                Ok(_) => {
                    healthy_directories += 1;
                    debug!("✅ Watch folder accessible: {}", folder);
                }
                Err(e) => {
                    issues.push(ValidationIssue {
                        issue_type: ValidationIssueType::Inaccessible,
                        severity: ValidationSeverity::Error,
                        directory_path: folder.clone(),
                        description: format!("Watch folder '{}' is not accessible", folder),
                        details: Some(serde_json::json!({
                            "error": e.to_string()
                        })),
                        detected_at: chrono::Utc::now(),
                    });
                }
            }
        }

        // Generate recommendations based on issues
        if issues.iter().any(|i| matches!(i.severity, ValidationSeverity::Critical)) {
            recommendations.push(ValidationRecommendation {
                action: ValidationAction::CheckServerConfiguration,
                reason: "Critical connectivity issues detected".to_string(),
                affected_directories: issues.iter()
                    .filter(|i| matches!(i.severity, ValidationSeverity::Critical))
                    .map(|i| i.directory_path.clone())
                    .collect(),
                priority: ValidationSeverity::Critical,
            });
        }

        if issues.iter().any(|i| matches!(i.issue_type, ValidationIssueType::Inaccessible)) {
            recommendations.push(ValidationRecommendation {
                action: ValidationAction::DeepScanRequired,
                reason: "Some directories are inaccessible and may need re-scanning".to_string(),
                affected_directories: issues.iter()
                    .filter(|i| matches!(i.issue_type, ValidationIssueType::Inaccessible))
                    .map(|i| i.directory_path.clone())
                    .collect(),
                priority: ValidationSeverity::Warning,
            });
        }

        if issues.is_empty() {
            recommendations.push(ValidationRecommendation {
                action: ValidationAction::NoActionRequired,
                reason: "System is healthy and functioning normally".to_string(),
                affected_directories: vec![],
                priority: ValidationSeverity::Info,
            });
        }

        // Calculate health score
        let health_score = if directories_checked == 0 {
            0
        } else {
            (healthy_directories * 100 / directories_checked) as i32
        };

        let critical_issues = issues.iter().filter(|i| matches!(i.severity, ValidationSeverity::Critical)).count();
        let warning_issues = issues.iter().filter(|i| matches!(i.severity, ValidationSeverity::Warning)).count();
        let info_issues = issues.iter().filter(|i| matches!(i.severity, ValidationSeverity::Info)).count();

        let summary = ValidationSummary {
            total_directories_checked: directories_checked,
            healthy_directories,
            directories_with_issues: directories_checked - healthy_directories,
            critical_issues,
            warning_issues,
            info_issues,
            validation_duration_ms: start_time.elapsed().as_millis() as u64,
        };

        info!("✅ WebDAV validation completed in {}ms. Health score: {}/100", 
              summary.validation_duration_ms, health_score);

        Ok(ValidationReport {
            overall_health_score: health_score,
            issues,
            recommendations,
            summary,
        })
    }

    // ============================================================================
    // Utility Methods
    // ============================================================================

    /// Tests if the server supports recursive ETag scanning
    pub async fn test_recursive_etag_support(&self) -> Result<bool> {
        debug!("🔍 Testing recursive ETag support");
        
        // Get server capabilities to check ETag support
        let capabilities = self.get_server_capabilities().await?;
        
        // Check if server supports ETags at all
        if !capabilities.supports_etag {
            debug!("❌ Server does not support ETags");
            return Ok(false);
        }

        // Check server type for known recursive ETag support
        if let Some(server_software) = &capabilities.server_software {
            let server_lower = server_software.to_lowercase();
            
            // Nextcloud and ownCloud support recursive ETags
            if server_lower.contains("nextcloud") || server_lower.contains("owncloud") {
                debug!("✅ Server supports recursive ETags (Nextcloud/ownCloud)");
                return Ok(true);
            }
            
            // Apache mod_dav typically supports recursive ETags
            if server_lower.contains("apache") && capabilities.dav_compliance.contains("1") {
                debug!("✅ Server likely supports recursive ETags (Apache WebDAV)");
                return Ok(true);
            }
        }

        // For unknown servers, assume basic ETag support but not recursive
        debug!("⚠️ Unknown server type, assuming no recursive ETag support");
        Ok(false)
    }

    /// Checks if a path is a direct child of a parent directory
    pub fn is_direct_child(&self, file_path: &str, parent_path: &str) -> bool {
        // Normalize paths by removing trailing slashes
        let normalized_parent = parent_path.trim_end_matches('/');
        let normalized_file = file_path.trim_end_matches('/');
        
        // Handle root case
        if normalized_parent.is_empty() || normalized_parent == "/" {
            return !normalized_file.is_empty() && normalized_file.matches('/').count() == 1;
        }
        
        // Check if file path starts with parent path
        if !normalized_file.starts_with(normalized_parent) {
            return false;
        }
        
        // Get the remainder after the parent path
        let remainder = &normalized_file[normalized_parent.len()..];
        
        // Should start with '/' and contain no additional '/' characters
        remainder.starts_with('/') && remainder[1..].find('/').is_none()
    }

    /// Gets configuration information
    pub fn get_config(&self) -> &WebDAVConfig {
        &self.config
    }

    /// Gets retry configuration
    pub fn get_retry_config(&self) -> &RetryConfig {
        &self.retry_config
    }

    /// Gets concurrency configuration
    pub fn get_concurrency_config(&self) -> &ConcurrencyConfig {
        &self.concurrency_config
    }

    // ============================================================================
    // URL Management Methods (for backward compatibility with WebDAVUrlManager)
    // ============================================================================

    /// Processes a single FileIngestionInfo to convert full paths to relative paths
    pub fn process_file_info(&self, mut file_info: FileIngestionInfo) -> FileIngestionInfo {
        // Convert full_path to relative_path
        file_info.relative_path = self.href_to_relative_path(&file_info.full_path);
        
        // For backward compatibility, set the deprecated path field to relative_path
        #[allow(deprecated)]
        {
            file_info.path = file_info.relative_path.clone();
        }
        
        file_info
    }

    /// Processes multiple FileIngestionInfo objects to convert full paths to relative paths
    pub fn process_file_infos(&self, file_infos: Vec<FileIngestionInfo>) -> Vec<FileIngestionInfo> {
        file_infos.into_iter().map(|file_info| self.process_file_info(file_info)).collect()
    }

    /// Converts a relative path to a full URL (alias for path_to_url for compatibility)
    pub fn relative_path_to_url(&self, relative_path: &str) -> String {
        self.path_to_url(relative_path)
    }
}


// Implement Clone to allow sharing the service
impl Clone for WebDAVService {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            config: self.config.clone(),
            retry_config: self.retry_config.clone(),
            concurrency_config: self.concurrency_config.clone(),
            scan_semaphore: Arc::clone(&self.scan_semaphore),
            download_semaphore: Arc::clone(&self.download_semaphore),
            working_protocol: Arc::clone(&self.working_protocol),
        }
    }
}

/// Tests WebDAV connection with provided configuration (standalone function for backward compatibility)
pub async fn test_webdav_connection(test_config: &WebDAVTestConnection) -> Result<WebDAVConnectionResult> {
    WebDAVService::test_connection_with_config(test_config).await
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
