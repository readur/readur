use anyhow::{anyhow, Result};
use reqwest::{Client, Method, Response};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::{HashMap, HashSet};
use tokio::sync::Semaphore;
use tokio::time::sleep;
use futures_util::stream;
use tracing::{debug, error, info, warn};
use serde::{Deserialize, Serialize};

use crate::models::{
    FileIngestionInfo, WebDAVConnectionResult, WebDAVCrawlEstimate, WebDAVTestConnection,
    WebDAVFolderInfo,
};
use crate::webdav_xml_parser::{parse_propfind_response, parse_propfind_response_with_directories};

use super::{config::{WebDAVConfig, RetryConfig, ConcurrencyConfig}, SyncProgress};

/// Results from WebDAV discovery including both files and directories
#[derive(Debug, Clone)]
pub struct WebDAVDiscoveryResult {
    pub files: Vec<FileIngestionInfo>,
    pub directories: Vec<FileIngestionInfo>,
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

        // Create HTTP client with timeout
        let client = Client::builder()
            .timeout(config.timeout())
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
        })
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

        // Test basic connectivity with OPTIONS request
        match self.test_options_request().await {
            Ok((server_version, server_type)) => {
                info!("✅ WebDAV connection successful");
                Ok(WebDAVConnectionResult {
                    success: true,
                    message: "Connection successful".to_string(),
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
        let webdav_url = self.config.webdav_url();
        
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
        let capabilities_url = format!("{}/ocs/v1.php/cloud/capabilities", 
            self.config.server_url.trim_end_matches('/'));

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

    /// Performs authenticated request with simple retry logic (simplified from complex error recovery)
    pub async fn authenticated_request(
        &self,
        method: Method,
        url: &str,
        body: Option<String>,
        headers: Option<Vec<(&str, &str)>>,
    ) -> Result<reqwest::Response> {
        let mut attempt = 0;
        let mut delay = self.retry_config.initial_delay_ms;

        // Enhanced debug logging for HTTP requests
        debug!("🌐 HTTP Request Details:");
        debug!("   Method: {}", method);
        debug!("   URL: {}", url);
        debug!("   Username: {}", self.config.username);
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
                .basic_auth(&self.config.username, Some(&self.config.password));

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

                    // Handle rate limiting
                    if status.as_u16() == 429 {
                        warn!("Rate limited, backing off for {}ms", self.retry_config.rate_limit_backoff_ms);
                        sleep(Duration::from_millis(self.retry_config.rate_limit_backoff_ms)).await;
                        continue;
                    }

                    // Handle client errors (don't retry)
                    if status.is_client_error() && status.as_u16() != 429 {
                        let error_body = response.text().await.unwrap_or_default();
                        
                        // Provide specific guidance for 405 Method Not Allowed errors
                        if status.as_u16() == 405 {
                            error!("🚫 HTTP 405 Method Not Allowed for {} {}", method, url);
                            error!("🔍 Request Details:");
                            error!("   Method: {}", method);
                            error!("   URL: {}", url);
                            error!("   Server type: {:?}", self.config.server_type);
                            error!("   Username: {}", self.config.username);
                            error!("   Server base URL: {}", self.config.server_url);
                            error!("   WebDAV base URL: {}", self.config.webdav_url());
                            if let Some(ref headers_list) = headers {
                                error!("   Request headers: {:?}", headers_list);
                            }
                            error!("📝 This usually indicates:");
                            error!("   1. WebDAV is not enabled on the server");
                            error!("   2. The URL endpoint doesn't support {} method", method);
                            error!("   3. Incorrect WebDAV endpoint URL");
                            error!("   4. Authentication issues or insufficient permissions");
                            error!("💡 Troubleshooting steps:");
                            error!("   - Verify WebDAV is enabled in your server settings");
                            error!("   - Check if the WebDAV endpoint URL is correct");
                            error!("   - Try testing with a WebDAV client like Cyberduck");
                            error!("   - Verify your user has WebDAV access permissions");
                            
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
                        warn!("Server error {}, retrying in {}ms (attempt {}/{})", 
                            status, delay, attempt + 1, self.retry_config.max_retries);
                        
                        sleep(Duration::from_millis(delay)).await;
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
                        warn!("Request error: {}, retrying in {}ms (attempt {}/{})", 
                            e, delay, attempt + 1, self.retry_config.max_retries);
                        
                        sleep(Duration::from_millis(delay)).await;
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

    // ============================================================================
    // URL Management Helper Methods (Previously separate module)
    // ============================================================================

    /// Gets the WebDAV URL for a specific path
    pub fn get_url_for_path(&self, path: &str) -> String {
        let base_url = self.config.webdav_url();
        let clean_path = path.trim_start_matches('/');
        
        let final_url = if clean_path.is_empty() {
            base_url.clone()
        } else {
            // Ensure no double slashes by normalizing the base URL
            let normalized_base = base_url.trim_end_matches('/');
            format!("{}/{}", normalized_base, clean_path)
        };
        
        debug!("🔗 URL Construction:");
        debug!("   Input path: '{}'", path);
        debug!("   Clean path: '{}'", clean_path);
        debug!("   Base WebDAV URL: '{}'", base_url);
        debug!("   Final URL: '{}'", final_url);
        debug!("   Server type: {:?}", self.config.server_type);
        debug!("   Server base URL: '{}'", self.config.server_url);
        
        final_url
    }

    /// Convert full WebDAV href (from XML response) to relative path
    /// 
    /// Input:  "/remote.php/dav/files/username/Photos/image.jpg"
    /// Output: "/Photos/image.jpg"
    pub fn href_to_relative_path(&self, href: &str) -> String {
        match self.config.server_type.as_deref() {
            Some("nextcloud") => {
                let prefix = format!("/remote.php/dav/files/{}", self.config.username);
                if href.starts_with(&prefix) {
                    let relative = &href[prefix.len()..];
                    if relative.is_empty() { "/" } else { relative }.to_string()
                } else {
                    href.to_string()
                }
            }
            Some("owncloud") => {
                if href.starts_with("/remote.php/webdav") {
                    let relative = &href[18..]; // Remove "/remote.php/webdav"
                    if relative.is_empty() { "/" } else { relative }.to_string()
                } else {
                    href.to_string()
                }
            }
            Some("generic") => {
                if href.starts_with("/webdav") {
                    let relative = &href[7..]; // Remove "/webdav"
                    if relative.is_empty() { "/" } else { relative }.to_string()
                } else {
                    href.to_string()
                }
            }
            _ => href.to_string()
        }
    }

    /// Convert file paths to the proper URL format for the server
    pub fn path_to_url(&self, relative_path: &str) -> String {
        let clean_path = relative_path.trim_start_matches('/');
        let base_url = self.config.webdav_url();
        
        if clean_path.is_empty() {
            base_url
        } else {
            format!("{}/{}", base_url.trim_end_matches('/'), clean_path)
        }
    }

    /// Converts a full WebDAV path to a relative path by removing server-specific prefixes
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

    /// Discovers files in a directory with support for pagination and filtering
    pub async fn discover_files(&self, directory_path: &str, recursive: bool) -> Result<Vec<FileIngestionInfo>> {
        info!("🔍 Discovering files in directory: {}", directory_path);
        
        if recursive {
            self.discover_files_recursive(directory_path).await
        } else {
            self.discover_files_single_directory(directory_path).await
        }
    }

    /// Discovers both files and directories with their ETags for directory tracking
    pub async fn discover_files_and_directories(&self, directory_path: &str, recursive: bool) -> Result<WebDAVDiscoveryResult> {
        info!("🔍 Discovering files and directories in: {}", directory_path);
        
        if recursive {
            self.discover_files_and_directories_recursive(directory_path).await
        } else {
            self.discover_files_and_directories_single(directory_path).await
        }
    }

    /// Discovers both files and directories with basic progress tracking (simplified)
    pub async fn discover_files_and_directories_with_progress(
        &self, 
        directory_path: &str, 
        recursive: bool, 
        _progress: Option<&SyncProgress> // Simplified: just placeholder for API compatibility
    ) -> Result<WebDAVDiscoveryResult> {
        info!("🔍 Discovering files and directories in: {} (progress tracking simplified)", directory_path);
        
        if recursive {
            self.discover_files_and_directories_recursive(directory_path).await
        } else {
            self.discover_files_and_directories_single(directory_path).await
        }
    }

    /// Discovers files in a single directory (non-recursive)
    async fn discover_files_single_directory(&self, directory_path: &str) -> Result<Vec<FileIngestionInfo>> {
        let url = self.get_url_for_path(directory_path);
        
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
            Method::from_bytes(b"PROPFIND")?,
            &url,
            Some(propfind_body.to_string()),
            Some(vec![
                ("Depth", "1"),
                ("Content-Type", "application/xml"),
            ]),
        ).await?;

        let body = response.text().await?;
        let files = parse_propfind_response(&body)?;
        
        // Filter out the directory itself and only return files
        let filtered_files: Vec<FileIngestionInfo> = files
            .into_iter()
            .filter(|file| !file.is_directory && file.relative_path != directory_path)
            .collect();

        debug!("Found {} files in directory: {}", filtered_files.len(), directory_path);
        Ok(filtered_files)
    }

    /// Discovers files recursively in all subdirectories
    async fn discover_files_recursive(&self, directory_path: &str) -> Result<Vec<FileIngestionInfo>> {
        let mut all_files = Vec::new();
        let mut directories_to_scan = vec![directory_path.to_string()];
        let semaphore = Arc::new(Semaphore::new(self.concurrency_config.max_concurrent_scans));
        
        while !directories_to_scan.is_empty() {
            let current_directories = directories_to_scan.clone();
            directories_to_scan.clear();

            // Process directories concurrently
            let tasks = current_directories.into_iter().map(|dir| {
                let permit = semaphore.clone();
                let service = self.clone();
                
                async move {
                    let _permit = permit.acquire().await.unwrap();
                    service.discover_files_and_directories_single(&dir).await
                }
            });

            let results = futures_util::future::join_all(tasks).await;

            for result in results {
                match result {
                    Ok(discovery_result) => {
                        all_files.extend(discovery_result.files);
                        
                        // Add subdirectories to the queue for the next iteration
                        for dir in discovery_result.directories {
                            if dir.is_directory {
                                directories_to_scan.push(dir.relative_path);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to scan directory: {}", e);
                    }
                }
            }
        }

        info!("Recursive scan completed. Found {} files total", all_files.len());
        Ok(all_files)
    }

    /// Discovers both files and directories in a single directory
    async fn discover_files_and_directories_single(&self, directory_path: &str) -> Result<WebDAVDiscoveryResult> {
        // Try the primary URL first, then fallback URLs if we get a 405 error
        match self.discover_files_and_directories_single_with_url(directory_path, &self.get_url_for_path(directory_path)).await {
            Ok(result) => Ok(result),
            Err(e) => {
                // Check if this is a 405 Method Not Allowed error
                if e.to_string().contains("405") || e.to_string().contains("Method Not Allowed") {
                    warn!("🔄 Primary WebDAV URL failed with 405 error, trying fallback URLs...");
                    self.try_fallback_discovery(directory_path).await
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Tries fallback URLs when the primary WebDAV URL fails with 405
    async fn try_fallback_discovery(&self, directory_path: &str) -> Result<WebDAVDiscoveryResult> {
        let fallback_urls = self.config.webdav_fallback_urls();
        
        for (i, fallback_base_url) in fallback_urls.iter().enumerate() {
            let fallback_url = if directory_path == "/" || directory_path.is_empty() {
                fallback_base_url.clone()
            } else {
                format!("{}/{}", fallback_base_url.trim_end_matches('/'), directory_path.trim_start_matches('/'))
            };
            
            info!("🔄 Trying fallback URL #{}: {}", i + 1, fallback_url);
            
            match self.discover_files_and_directories_single_with_url(directory_path, &fallback_url).await {
                Ok(result) => {
                    info!("✅ Fallback URL #{} succeeded: {}", i + 1, fallback_url);
                    warn!("💡 Consider updating your server type configuration to use this URL pattern");
                    return Ok(result);
                }
                Err(e) => {
                    warn!("❌ Fallback URL #{} failed: {} - {}", i + 1, fallback_url, e);
                }
            }
        }
        
        Err(anyhow!(
            "All WebDAV URLs failed for directory '{}'. Primary URL and {} fallback URLs were tried. \
            This suggests WebDAV is not properly configured on the server or the server type is incorrect.",
            directory_path, fallback_urls.len()
        ))
    }

    /// Performs the actual discovery with a specific URL
    async fn discover_files_and_directories_single_with_url(&self, directory_path: &str, url: &str) -> Result<WebDAVDiscoveryResult> {
        // Enhanced debug logging for WebDAV URL construction
        debug!("🔍 WebDAV directory scan - Path: '{}', URL: '{}', Server type: {:?}", 
               directory_path, url, self.config.server_type);
        debug!("🔧 WebDAV config - Server URL: '{}', Username: '{}', WebDAV base URL: '{}'", 
               self.config.server_url, self.config.username, self.config.webdav_url());
        
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

        debug!("📤 Sending PROPFIND request to URL: {}", url);
        debug!("📋 PROPFIND body length: {} bytes", propfind_body.len());

        let response = self.authenticated_request(
            Method::from_bytes(b"PROPFIND")?,
            url,
            Some(propfind_body.to_string()),
            Some(vec![
                ("Depth", "1"),
                ("Content-Type", "application/xml"),
            ]),
        ).await.map_err(|e| {
            error!("❌ PROPFIND request failed for directory '{}' at URL '{}': {}", 
                   directory_path, url, e);
            e
        })?;

        let body = response.text().await?;
        let all_items = parse_propfind_response_with_directories(&body)?;
        
        // Separate files and directories, excluding the parent directory itself
        let mut files = Vec::new();
        let mut directories = Vec::new();
        
        for item in all_items {
            if item.relative_path == directory_path {
                continue; // Skip the directory itself
            }
            
            if item.is_directory {
                directories.push(item);
            } else {
                files.push(item);
            }
        }

        debug!("Found {} files and {} directories in: {}", files.len(), directories.len(), directory_path);
        Ok(WebDAVDiscoveryResult { files, directories })
    }

    /// Discovers files and directories recursively
    async fn discover_files_and_directories_recursive(&self, directory_path: &str) -> Result<WebDAVDiscoveryResult> {
        let mut all_files = Vec::new();
        let mut all_directories = Vec::new();
        let mut directories_to_scan = vec![directory_path.to_string()];
        let semaphore = Arc::new(Semaphore::new(self.concurrency_config.max_concurrent_scans));
        
        while !directories_to_scan.is_empty() {
            let current_directories = directories_to_scan.clone();
            directories_to_scan.clear();

            // Process directories concurrently
            let tasks = current_directories.into_iter().map(|dir| {
                let permit = semaphore.clone();
                let service = self.clone();
                
                async move {
                    let _permit = permit.acquire().await.unwrap();
                    service.discover_files_and_directories_single(&dir).await
                }
            });

            let results = futures_util::future::join_all(tasks).await;

            for result in results {
                match result {
                    Ok(discovery_result) => {
                        all_files.extend(discovery_result.files);
                        
                        // Add directories to our results and to the scan queue
                        for dir in discovery_result.directories {
                            directories_to_scan.push(dir.relative_path.clone());
                            all_directories.push(dir);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to scan directory: {}", e);
                    }
                }
            }
        }

        info!("Recursive scan completed. Found {} files and {} directories", all_files.len(), all_directories.len());
        Ok(WebDAVDiscoveryResult { 
            files: all_files, 
            directories: all_directories 
        })
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
            total_estimated_time_hours: estimated_total_scan_time.as_secs_f32() / 3600.0,
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
        let _permit = self.download_semaphore.acquire().await?;
        
        debug!("⬇️ Downloading file: {}", file_path);
        
        // Convert full WebDAV paths to relative paths to prevent double path construction
        let relative_path = self.convert_to_relative_path(file_path);
        let url = self.get_url_for_path(&relative_path);
        
        let response = self.authenticated_request(
            reqwest::Method::GET,
            &url,
            None,
            None,
        ).await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to download file '{}': HTTP {}",
                file_path,
                response.status()
            ));
        }

        let content = response.bytes().await?;
        debug!("✅ Downloaded {} bytes for file: {}", content.len(), file_path);
        
        Ok(content.to_vec())
    }

    /// Downloads a file from WebDAV server using FileIngestionInfo
    pub async fn download_file_info(&self, file_info: &FileIngestionInfo) -> Result<Vec<u8>> {
        let _permit = self.download_semaphore.acquire().await?;
        
        debug!("⬇️ Downloading file: {}", file_info.relative_path);
        
        // Use the relative path directly since it's already processed
        let relative_path = &file_info.relative_path;
        let url = self.get_url_for_path(&relative_path);
        
        let response = self.authenticated_request(
            reqwest::Method::GET,
            &url,
            None,
            None,
        ).await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to download file '{}': HTTP {}",
                file_info.relative_path,
                response.status()
            ));
        }

        let content = response.bytes().await?;
        debug!("✅ Downloaded {} bytes for file: {}", content.len(), file_info.relative_path);
        
        Ok(content.to_vec())
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
        
        let options_response = self.authenticated_request(
            reqwest::Method::OPTIONS,
            &self.config.webdav_url(),
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
        }
    }
}

/// Tests WebDAV connection with provided configuration (standalone function for backward compatibility)
pub async fn test_webdav_connection(test_config: &WebDAVTestConnection) -> Result<WebDAVConnectionResult> {
    WebDAVService::test_connection_with_config(test_config).await
}