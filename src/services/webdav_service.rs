use anyhow::{anyhow, Result};
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Duration;
use tokio::time::sleep;
use tokio::sync::Semaphore;
use futures_util::stream::{self, StreamExt};
use tracing::{debug, error, info, warn};

use crate::models::{
    FileInfo, WebDAVConnectionResult, WebDAVCrawlEstimate, WebDAVFolderInfo,
    WebDAVTestConnection,
};
use crate::webdav_xml_parser::{parse_propfind_response, parse_propfind_response_with_directories};

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

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
    pub timeout_seconds: u64,
    pub rate_limit_backoff_ms: u64, // Additional backoff for 429 responses
}

#[derive(Debug, Clone)]
pub struct ConcurrencyConfig {
    pub max_concurrent_scans: usize,
    pub max_concurrent_downloads: usize,
    pub adaptive_rate_limiting: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000, // 1 second
            max_delay_ms: 30000,    // 30 seconds
            backoff_multiplier: 2.0,
            timeout_seconds: 300,   // 5 minutes total timeout for crawl operations
            rate_limit_backoff_ms: 5000, // 5 seconds extra for rate limits
        }
    }
}

impl Default for ConcurrencyConfig {
    fn default() -> Self {
        Self {
            max_concurrent_scans: 10,
            max_concurrent_downloads: 5,
            adaptive_rate_limiting: true,
        }
    }
}



#[derive(Clone)]
pub struct WebDAVService {
    client: Client,
    config: WebDAVConfig,
    base_webdav_url: String,
    retry_config: RetryConfig,
    concurrency_config: ConcurrencyConfig,
}

/// Report of ETag validation and directory integrity checks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub validation_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub total_directories_checked: u32,
    pub issues_found: Vec<ValidationIssue>,
    pub recommendations: Vec<ValidationRecommendation>,
    pub etag_support_verified: bool,
    pub server_health_score: u8, // 0-100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub issue_type: ValidationIssueType,
    pub directory_path: String,
    pub severity: ValidationSeverity,
    pub description: String,
    pub discovered_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl WebDAVService {
    pub fn new(config: WebDAVConfig) -> Result<Self> {
        Self::new_with_configs(config, RetryConfig::default(), ConcurrencyConfig::default())
    }

    pub fn new_with_retry(config: WebDAVConfig, retry_config: RetryConfig) -> Result<Self> {
        Self::new_with_configs(config, retry_config, ConcurrencyConfig::default())
    }

    pub fn new_with_configs(config: WebDAVConfig, retry_config: RetryConfig, concurrency_config: ConcurrencyConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()?;

        // Validate server URL before constructing WebDAV URLs
        if config.server_url.trim().is_empty() {
            return Err(anyhow!("❌ WebDAV Configuration Error: server_url is empty"));
        }
        
        if !config.server_url.starts_with("http://") && !config.server_url.starts_with("https://") {
            return Err(anyhow!(
                "❌ WebDAV Configuration Error: server_url must start with 'http://' or 'https://'. \
                 Current value: '{}'. \
                 Examples: \
                 - https://cloud.example.com \
                 - http://192.168.1.100:8080 \
                 - https://nextcloud.mydomain.com", 
                config.server_url
            ));
        }
        
        // Validate that server_url can be parsed as a proper URL
        if let Err(e) = reqwest::Url::parse(&config.server_url) {
            return Err(anyhow!(
                "❌ WebDAV Configuration Error: server_url is not a valid URL: {}. \
                 Current value: '{}'. \
                 The URL must be absolute and include the full domain. \
                 Examples: \
                 - https://cloud.example.com \
                 - http://192.168.1.100:8080/webdav \
                 - https://nextcloud.mydomain.com", 
                e, config.server_url
            ));
        }

        // Construct WebDAV URL based on server type
        let base_webdav_url = match config.server_type.as_deref() {
            Some("nextcloud") | Some("owncloud") => {
                let url = format!(
                    "{}/remote.php/dav/files/{}",
                    config.server_url.trim_end_matches('/'),
                    config.username
                );
                debug!("🔗 Constructed Nextcloud/ownCloud WebDAV URL: {}", url);
                url
            },
            _ => {
                let url = format!(
                    "{}/webdav",
                    config.server_url.trim_end_matches('/')
                );
                debug!("🔗 Constructed generic WebDAV URL: {}", url);
                url
            },
        };

        Ok(Self {
            client,
            config,
            base_webdav_url,
            retry_config,
            concurrency_config,
        })
    }

    async fn retry_with_backoff<T, F, Fut>(&self, operation_name: &str, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut attempt = 0;
        let mut delay = self.retry_config.initial_delay_ms;

        loop {
            match operation().await {
                Ok(result) => {
                    if attempt > 0 {
                        info!("{} succeeded after {} retries", operation_name, attempt);
                    }
                    return Ok(result);
                }
                Err(err) => {
                    attempt += 1;
                    
                    if attempt > self.retry_config.max_retries {
                        error!("{} failed after {} attempts: {}", operation_name, attempt - 1, err);
                        return Err(err);
                    }

                    // Check if error is retryable
                    if !Self::is_retryable_error(&err) {
                        error!("{} failed with non-retryable error: {}", operation_name, err);
                        return Err(err);
                    }

                    // Apply adaptive backoff for rate limiting
                    let actual_delay = if Self::is_rate_limit_error(&err) && self.concurrency_config.adaptive_rate_limiting {
                        let rate_limit_delay = delay + self.retry_config.rate_limit_backoff_ms;
                        warn!("{} rate limited (attempt {}), retrying in {}ms with extra backoff: {}", 
                            operation_name, attempt, rate_limit_delay, err);
                        rate_limit_delay
                    } else {
                        warn!("{} failed (attempt {}), retrying in {}ms: {}", 
                            operation_name, attempt, delay, err);
                        delay
                    };
                    
                    sleep(Duration::from_millis(actual_delay)).await;
                    
                    // Calculate next delay with exponential backoff
                    delay = ((delay as f64 * self.retry_config.backoff_multiplier) as u64)
                        .min(self.retry_config.max_delay_ms);
                }
            }
        }
    }

    fn is_retryable_error(error: &anyhow::Error) -> bool {
        // Check if error is network-related or temporary
        if let Some(reqwest_error) = error.downcast_ref::<reqwest::Error>() {
            // Retry on network errors, timeouts, and server errors (5xx)
            return reqwest_error.is_timeout() 
                || reqwest_error.is_connect() 
                || reqwest_error.is_request()
                || reqwest_error.status()
                    .map(|s| {
                        s.is_server_error() // 5xx errors (including server restart scenarios)
                        || s == 429 // Too Many Requests
                        || s == 502 // Bad Gateway (server restarting)
                        || s == 503 // Service Unavailable (server restarting/overloaded)
                        || s == 504 // Gateway Timeout (server slow to respond)
                    })
                    .unwrap_or(true);
        }
        
        // For other errors, check the error message for common temporary issues
        let error_str = error.to_string().to_lowercase();
        error_str.contains("timeout") 
            || error_str.contains("connection") 
            || error_str.contains("network")
            || error_str.contains("temporary")
            || error_str.contains("rate limit")
            || error_str.contains("too many requests")
            || error_str.contains("connection reset")
            || error_str.contains("connection aborted")
            || error_str.contains("server unavailable")
            || error_str.contains("bad gateway")
            || error_str.contains("service unavailable")
    }

    fn is_rate_limit_error(error: &anyhow::Error) -> bool {
        if let Some(reqwest_error) = error.downcast_ref::<reqwest::Error>() {
            return reqwest_error.status()
                .map(|s| s == 429)
                .unwrap_or(false);
        }
        
        let error_str = error.to_string().to_lowercase();
        error_str.contains("rate limit") || error_str.contains("too many requests")
    }

    fn is_server_restart_error(&self, error: &anyhow::Error) -> bool {
        if let Some(reqwest_error) = error.downcast_ref::<reqwest::Error>() {
            if let Some(status) = reqwest_error.status() {
                return status == 502 // Bad Gateway 
                    || status == 503 // Service Unavailable
                    || status == 504; // Gateway Timeout
            }
            
            // Network-level connection issues often indicate server restart
            return reqwest_error.is_connect() || reqwest_error.is_timeout();
        }
        
        let error_str = error.to_string().to_lowercase();
        error_str.contains("connection reset")
            || error_str.contains("connection aborted")
            || error_str.contains("bad gateway")
            || error_str.contains("service unavailable")
            || error_str.contains("server unreachable")
    }

    pub async fn test_connection(&self, test_config: WebDAVTestConnection) -> Result<WebDAVConnectionResult> {
        info!("Testing WebDAV connection to {} ({})", 
            test_config.server_url, 
            test_config.server_type.as_deref().unwrap_or("generic"));
        
        // Validate server URL before constructing test URL
        if test_config.server_url.trim().is_empty() {
            return Ok(WebDAVConnectionResult {
                success: false,
                message: "❌ WebDAV server_url is empty".to_string(),
                server_version: None,
                server_type: None,
            });
        }
        
        if !test_config.server_url.starts_with("http://") && !test_config.server_url.starts_with("https://") {
            return Ok(WebDAVConnectionResult {
                success: false,
                message: format!(
                    "❌ WebDAV server_url must start with 'http://' or 'https://'. \
                     Current value: '{}'. \
                     Examples: https://cloud.example.com, http://192.168.1.100:8080", 
                    test_config.server_url
                ),
                server_version: None,
                server_type: None,
            });
        }
        
        // Validate URL can be parsed
        if let Err(e) = reqwest::Url::parse(&test_config.server_url) {
            return Ok(WebDAVConnectionResult {
                success: false,
                message: format!(
                    "❌ WebDAV server_url is not a valid URL: {}. \
                     Current value: '{}'. \
                     Must be absolute URL like: https://cloud.example.com", 
                    e, test_config.server_url
                ),
                server_version: None,
                server_type: None,
            });
        }
        
        let test_url = match test_config.server_type.as_deref() {
            Some("nextcloud") | Some("owncloud") => format!(
                "{}/remote.php/dav/files/{}/",
                test_config.server_url.trim_end_matches('/'),
                test_config.username
            ),
            _ => format!(
                "{}/webdav/",
                test_config.server_url.trim_end_matches('/')
            ),
        };
        
        debug!("🔗 Constructed test URL: {}", test_url);

        let resp = self.client
            .request(Method::from_bytes(b"PROPFIND").unwrap(), &test_url)
            .basic_auth(&test_config.username, Some(&test_config.password))
            .header("Depth", "0")
            .body(r#"<?xml version="1.0"?>
                <d:propfind xmlns:d="DAV:">
                    <d:prop>
                        <d:displayname/>
                    </d:prop>
                </d:propfind>"#)
            .send()
            .await
            .map_err(|e| {
                error!("❌ WebDAV HTTP request failed for URL '{}': {}", test_url, e);
                anyhow!("WebDAV HTTP request failed for URL '{}': {}. \
                         This often indicates a URL configuration issue. \
                         Verify the server_url is correct and accessible.", test_url, e)
            })?;

        if resp.status().is_success() {
            info!("✅ WebDAV connection successful");
            
            // Try to get server info
            let (version, server_type) = self.get_server_info(&test_config).await;
            
            Ok(WebDAVConnectionResult {
                success: true,
                message: format!("Successfully connected to WebDAV server ({})", 
                    server_type.as_deref().unwrap_or("Generic WebDAV")),
                server_version: version,
                server_type,
            })
        } else {
            error!("❌ WebDAV connection failed with status: {} for URL: {}", resp.status(), test_url);
            Ok(WebDAVConnectionResult {
                success: false,
                message: format!("Connection failed: HTTP {} for URL: {}", resp.status(), test_url),
                server_version: None,
                server_type: None,
            })
        }
    }

    async fn get_server_info(&self, test_config: &WebDAVTestConnection) -> (Option<String>, Option<String>) {
        // Try Nextcloud/ownCloud capabilities first
        if let Some(server_type) = &test_config.server_type {
            if server_type == "nextcloud" || server_type == "owncloud" {
                let capabilities_url = format!(
                    "{}/ocs/v1.php/cloud/capabilities?format=json",
                    test_config.server_url.trim_end_matches('/')
                );

                if let Ok(response) = self.client
                    .get(&capabilities_url)
                    .basic_auth(&test_config.username, Some(&test_config.password))
                    .send()
                    .await 
                {
                    if response.status().is_success() {
                        if let Ok(text) = response.text().await {
                            // Simple version extraction
                            if let Some(start) = text.find("\"version\":\"") {
                                let version_start = start + 11;
                                if let Some(end) = text[version_start..].find('"') {
                                    let version = text[version_start..version_start + end].to_string();
                                    return (Some(version), Some(server_type.clone()));
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Fallback: try to detect server type from headers
        (Some("Unknown".to_string()), test_config.server_type.clone())
    }

    pub async fn estimate_crawl(&self, folders: &[String]) -> Result<WebDAVCrawlEstimate> {
        info!("Estimating crawl for {} folders", folders.len());
        
        let mut folder_infos = Vec::new();
        let supported_extensions: HashSet<String> = self.config.file_extensions
            .iter()
            .map(|ext| ext.to_lowercase())
            .collect();

        for folder_path in folders {
            debug!("Analyzing folder: {}", folder_path);
            
            match self.analyze_folder(folder_path, &supported_extensions).await {
                Ok(folder_info) => {
                    debug!("Folder {} has {} files ({} supported)", 
                        folder_path, folder_info.total_files, folder_info.supported_files);
                    folder_infos.push(folder_info);
                }
                Err(e) => {
                    warn!("Failed to analyze folder {}: {}", folder_path, e);
                    // Add empty folder info so UI can show the error
                    folder_infos.push(WebDAVFolderInfo {
                        path: folder_path.clone(),
                        total_files: 0,
                        supported_files: 0,
                        estimated_time_hours: 0.0,
                        total_size_mb: 0.0,
                    });
                }
            }
        }

        let total_files: i64 = folder_infos.iter().map(|f| f.total_files).sum();
        let total_supported_files: i64 = folder_infos.iter().map(|f| f.supported_files).sum();
        let total_estimated_time_hours: f32 = folder_infos.iter().map(|f| f.estimated_time_hours).sum();
        let total_size_mb: f64 = folder_infos.iter().map(|f| f.total_size_mb).sum();

        info!("Crawl estimate complete: {} total files, {} supported files, {:.2} hours estimated", 
            total_files, total_supported_files, total_estimated_time_hours);

        Ok(WebDAVCrawlEstimate {
            folders: folder_infos,
            total_files,
            total_supported_files,
            total_estimated_time_hours,
            total_size_mb,
        })
    }

    async fn analyze_folder(&self, folder_path: &str, supported_extensions: &HashSet<String>) -> Result<WebDAVFolderInfo> {
        let files = self.discover_files_in_folder(folder_path).await?;
        
        let mut total_files = 0i64;
        let mut supported_files = 0i64;
        let mut total_size_bytes = 0i64;

        for file in files {
            if !file.is_directory {
                total_files += 1;
                total_size_bytes += file.size;

                // Check if file extension is supported
                if let Some(extension) = std::path::Path::new(&file.name)
                    .extension()
                    .and_then(|ext| ext.to_str())
                {
                    if supported_extensions.contains(&extension.to_lowercase()) {
                        supported_files += 1;
                    }
                }
            }
        }

        // Estimate processing time: ~2 seconds per file for OCR
        // This is a rough estimate - actual time depends on file size and complexity
        let estimated_time_hours = (supported_files as f32 * 2.0) / 3600.0;
        let total_size_mb = total_size_bytes as f64 / (1024.0 * 1024.0);

        Ok(WebDAVFolderInfo {
            path: folder_path.to_string(),
            total_files,
            supported_files,
            estimated_time_hours,
            total_size_mb,
        })
    }

    pub async fn discover_files_in_folder(&self, folder_path: &str) -> Result<Vec<FileInfo>> {
        self.retry_with_backoff("discover_files_in_folder", || {
            self.discover_files_in_folder_impl(folder_path)
        }).await
    }

    /// Optimized discovery that checks directory ETag first to avoid unnecessary deep scans
    pub async fn discover_files_in_folder_optimized(&self, folder_path: &str, user_id: uuid::Uuid, state: &crate::AppState) -> Result<Vec<FileInfo>> {
        self.discover_files_in_folder_optimized_with_recovery(folder_path, user_id, state, true).await
    }

    async fn discover_files_in_folder_optimized_with_recovery(&self, folder_path: &str, user_id: uuid::Uuid, state: &crate::AppState, enable_crash_recovery: bool) -> Result<Vec<FileInfo>> {
        debug!("🔍 Starting optimized discovery for folder: {}", folder_path);
        
        // Check for incomplete scans that need recovery
        if enable_crash_recovery {
            if let Ok(incomplete_scans) = self.detect_incomplete_scans(user_id, state).await {
                if !incomplete_scans.is_empty() {
                    info!("🔄 Detected {} incomplete scans from previous session, resuming...", incomplete_scans.len());
                    for incomplete_path in incomplete_scans {
                        if incomplete_path.starts_with(folder_path) {
                            info!("🔄 Resuming incomplete scan for: {}", incomplete_path);
                            match self.resume_deep_scan_internal(&incomplete_path, user_id, state).await {
                                Ok(resumed_files) => {
                                    info!("✅ Successfully resumed scan for {}: {} files found", incomplete_path, resumed_files.len());
                                }
                                Err(e) => {
                                    warn!("⚠️ Failed to resume scan for {}: {}", incomplete_path, e);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Check if we should use smart scanning
        let use_smart_scan = match self.config.server_type.as_deref() {
            Some("nextcloud") | Some("owncloud") => {
                debug!("🚀 Using smart scanning for Nextcloud/ownCloud server");
                true
            }
            _ => {
                debug!("📁 Using traditional scanning for generic WebDAV server");
                false
            }
        };
        
        if use_smart_scan {
            // Get stored ETag for this directory
            let stored_etag = match state.db.get_webdav_directory(user_id, folder_path).await {
                Ok(Some(dir)) => Some(dir.directory_etag),
                Ok(None) => None,
                Err(e) => {
                    warn!("Database error checking directory {}: {}", folder_path, e);
                    None
                }
            };
            
            // Use smart scanning with depth-1 traversal and checkpoint recovery
            return self.smart_directory_scan_with_checkpoints(folder_path, stored_etag.as_deref(), user_id, state).await;
        }
        
        // Fall back to traditional optimization for other servers
        // Step 1: Check directory ETag first (lightweight PROPFIND with Depth: 0)
        let current_dir_etag = match self.check_directory_etag(folder_path).await {
            Ok(etag) => etag,
            Err(e) => {
                warn!("Failed to get directory ETag for {}, falling back to full scan: {}", folder_path, e);
                return self.discover_files_in_folder_impl(folder_path).await;
            }
        };
        
        // Step 2: Check if we have this directory cached
        match state.db.get_webdav_directory(user_id, folder_path).await {
            Ok(Some(stored_dir)) => {
                if stored_dir.directory_etag == current_dir_etag {
                    debug!("✅ Directory {} unchanged (ETag: {}), checking subdirectories individually", folder_path, current_dir_etag);
                    
                    // Update last_scanned_at to show we checked
                    let update = crate::models::UpdateWebDAVDirectory {
                        directory_etag: current_dir_etag,
                        last_scanned_at: chrono::Utc::now(),
                        file_count: stored_dir.file_count,
                        total_size_bytes: stored_dir.total_size_bytes,
                    };
                    
                    if let Err(e) = state.db.update_webdav_directory(user_id, folder_path, &update).await {
                        warn!("Failed to update directory scan time: {}", e);
                    }
                    
                    // Step 2a: Check subdirectories individually for changes
                    let changed_files = self.check_subdirectories_for_changes(folder_path, user_id, state).await?;
                    return Ok(changed_files);
                } else {
                    debug!("🔄 Directory {} changed (old ETag: {}, new ETag: {}), performing deep scan", 
                        folder_path, stored_dir.directory_etag, current_dir_etag);
                }
            }
            Ok(None) => {
                debug!("🆕 New directory {}, performing initial scan", folder_path);
            }
            Err(e) => {
                warn!("Database error checking directory {}: {}, proceeding with scan", folder_path, e);
            }
        }
        
        // Step 3: Directory has changed or is new - perform full discovery
        let files = self.discover_files_in_folder_impl(folder_path).await?;
        
        // Step 4: Update directory tracking info for main directory
        let file_count = files.iter().filter(|f| !f.is_directory).count() as i64;
        let total_size_bytes = files.iter().filter(|f| !f.is_directory).map(|f| f.size).sum::<i64>();
        
        let directory_record = crate::models::CreateWebDAVDirectory {
            user_id,
            directory_path: folder_path.to_string(),
            directory_etag: current_dir_etag.clone(),
            file_count,
            total_size_bytes,
        };
        
        if let Err(e) = state.db.create_or_update_webdav_directory(&directory_record).await {
            error!("Failed to update directory tracking for {}: {}", folder_path, e);
        } else {
            debug!("📊 Updated directory tracking: {} files, {} bytes, ETag: {}", 
                file_count, total_size_bytes, current_dir_etag);
        }
        
        // Step 5: Track ALL subdirectories found during the scan (n-depth)
        self.track_subdirectories_recursively(&files, user_id, state).await;
        
        Ok(files)
    }

    /// Track all subdirectories recursively with rock-solid n-depth support
    async fn track_subdirectories_recursively(&self, files: &[FileInfo], user_id: uuid::Uuid, state: &crate::AppState) {
        use std::collections::{HashMap, BTreeSet};
        
        // Step 1: Extract all unique directory paths from the file list
        let mut all_directories = BTreeSet::new();
        
        for file in files {
            if file.is_directory {
                // Add the directory itself
                all_directories.insert(file.path.clone());
            } else {
                // Extract all parent directories from file paths
                let mut path_parts: Vec<&str> = file.path.split('/').collect();
                path_parts.pop(); // Remove the filename
                
                // Build directory paths from root down to immediate parent
                let mut current_path = String::new();
                for part in path_parts {
                    if !part.is_empty() {
                        if !current_path.is_empty() {
                            current_path.push('/');
                        }
                        current_path.push_str(part);
                        all_directories.insert(current_path.clone());
                    }
                }
            }
        }
        
        debug!("🗂️ Found {} unique directories at all levels", all_directories.len());
        
        // Step 2: Create a mapping of directory -> ETag from the files list
        let mut directory_etags: HashMap<String, String> = HashMap::new();
        for file in files {
            if file.is_directory {
                directory_etags.insert(file.path.clone(), file.etag.clone());
            }
        }
        
        // Step 3: For each directory, calculate its direct content (files and immediate subdirs)
        for dir_path in &all_directories {
            let dir_etag = match directory_etags.get(dir_path) {
                Some(etag) => etag.clone(),
                None => {
                    debug!("⚠️ No ETag found for directory: {}", dir_path);
                    continue; // Skip directories without ETags
                }
            };
            
            // Count direct files in this directory (not in subdirectories)
            let direct_files: Vec<_> = files.iter()
                .filter(|f| {
                    !f.is_directory && 
                    self.is_direct_child(&f.path, dir_path)
                })
                .collect();
            
            // Count direct subdirectories  
            let direct_subdirs: Vec<_> = files.iter()
                .filter(|f| {
                    f.is_directory && 
                    self.is_direct_child(&f.path, dir_path)
                })
                .collect();
            
            let file_count = direct_files.len() as i64;
            let total_size_bytes = direct_files.iter().map(|f| f.size).sum::<i64>();
            
            // Create or update directory tracking record
            let directory_record = crate::models::CreateWebDAVDirectory {
                user_id,
                directory_path: dir_path.clone(),
                directory_etag: dir_etag.clone(),
                file_count,
                total_size_bytes,
            };
            
            match state.db.create_or_update_webdav_directory(&directory_record).await {
                Ok(_) => {
                    debug!("📁 Tracked directory: {} ({} files, {} subdirs, {} bytes, ETag: {})", 
                        dir_path, file_count, direct_subdirs.len(), total_size_bytes, dir_etag);
                }
                Err(e) => {
                    warn!("Failed to update directory tracking for {}: {}", dir_path, e);
                }
            }
        }
        
        debug!("✅ Completed tracking {} directories at all depth levels", all_directories.len());
    }
    
    /// Check if a path is a direct child of a directory (not nested deeper)
    pub fn is_direct_child(&self, child_path: &str, parent_path: &str) -> bool {
        // Normalize paths by removing trailing slashes
        let child_normalized = child_path.trim_end_matches('/');
        let parent_normalized = parent_path.trim_end_matches('/');
        
        if !child_normalized.starts_with(parent_normalized) {
            return false;
        }
        
        // Same path is not a direct child of itself
        if child_normalized == parent_normalized {
            return false;
        }
        
        // Handle root directory case
        if parent_normalized.is_empty() || parent_normalized == "/" {
            let child_without_leading_slash = child_normalized.trim_start_matches('/');
            return !child_without_leading_slash.is_empty() && !child_without_leading_slash.contains('/');
        }
        
        // Remove parent path prefix and check if remainder has exactly one more path segment
        let remaining = child_normalized.strip_prefix(parent_normalized)
            .unwrap_or("")
            .trim_start_matches('/');
            
        // Direct child means no more slashes in the remaining path
        !remaining.contains('/') && !remaining.is_empty()
    }
    
    /// Perform targeted re-scanning of only specific paths that have changed
    pub async fn discover_files_targeted_rescan(&self, paths_to_scan: &[String], user_id: uuid::Uuid, state: &crate::AppState) -> Result<Vec<FileInfo>> {
        debug!("🎯 Starting targeted re-scan for {} specific paths", paths_to_scan.len());
        
        let mut all_files = Vec::new();
        
        for path in paths_to_scan {
            debug!("🔍 Targeted scan of: {}", path);
            
            // Convert to relative path for API calls
            let relative_path = self.convert_to_relative_path(path);
            
            // Check if this specific path has changed
            match self.check_directory_etag(&relative_path).await {
                Ok(current_etag) => {
                    // Check cached ETag
                    let needs_scan = match state.db.get_webdav_directory(user_id, path).await {
                        Ok(Some(stored_dir)) => {
                            if stored_dir.directory_etag != current_etag {
                                debug!("🔄 Path {} changed (old: {}, new: {})", path, stored_dir.directory_etag, current_etag);
                                true
                            } else {
                                debug!("✅ Path {} unchanged (ETag: {})", path, current_etag);
                                false
                            }
                        }
                        Ok(None) => {
                            debug!("🆕 New path {} detected", path);
                            true
                        }
                        Err(e) => {
                            warn!("Database error for path {}: {}", path, e);
                            true // Scan on error to be safe
                        }
                    };
                    
                    if needs_scan {
                        // Use shallow scan for this specific directory only
                        match self.discover_files_in_folder_shallow(&relative_path).await {
                            Ok(mut path_files) => {
                                debug!("📂 Found {} files in changed path {}", path_files.len(), path);
                                all_files.append(&mut path_files);
                                
                                // Update tracking for this specific path
                                self.update_single_directory_tracking(path, &path_files, user_id, state).await;
                            }
                            Err(e) => {
                                error!("Failed to scan changed path {}: {}", path, e);
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to check ETag for path {}: {}, skipping", path, e);
                }
            }
        }
        
        debug!("🎯 Targeted re-scan completed: {} total files found", all_files.len());
        Ok(all_files)
    }
    
    /// Discover files in a single directory only (shallow scan, no recursion)
    async fn discover_files_in_folder_shallow(&self, folder_path: &str) -> Result<Vec<FileInfo>> {
        let folder_url = format!("{}{}", self.base_webdav_url, folder_path);
        
        debug!("Shallow scan of directory: {}", folder_url);

        let propfind_body = r#"<?xml version="1.0"?>
            <d:propfind xmlns:d="DAV:">
                <d:allprop/>
            </d:propfind>"#;

        let response = self.client
            .request(Method::from_bytes(b"PROPFIND").unwrap(), &folder_url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .header("Depth", "1")  // Only direct children, not recursive
            .header("Content-Type", "application/xml")
            .body(propfind_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("PROPFIND request failed: {}", response.status()));
        }

        let response_text = response.text().await?;
        debug!("Shallow WebDAV response received, parsing...");

        // Use the parser that includes directories for shallow scans
        self.parse_webdav_response_with_directories(&response_text)
    }
    
    /// Update tracking for a single directory without recursive processing
    async fn update_single_directory_tracking(&self, directory_path: &str, files: &[FileInfo], user_id: uuid::Uuid, state: &crate::AppState) {
        // Get the directory's own ETag
        let dir_etag = files.iter()
            .find(|f| f.is_directory && f.path == directory_path)
            .map(|f| f.etag.clone())
            .unwrap_or_else(|| {
                warn!("No ETag found for directory {}, using timestamp-based fallback", directory_path);
                chrono::Utc::now().timestamp().to_string()
            });
        
        // Count direct files in this directory only
        let direct_files: Vec<_> = files.iter()
            .filter(|f| !f.is_directory && self.is_direct_child(&f.path, directory_path))
            .collect();
        
        let file_count = direct_files.len() as i64;
        let total_size_bytes = direct_files.iter().map(|f| f.size).sum::<i64>();
        
        let directory_record = crate::models::CreateWebDAVDirectory {
            user_id,
            directory_path: directory_path.to_string(),
            directory_etag: dir_etag.clone(),
            file_count,
            total_size_bytes,
        };
        
        match state.db.create_or_update_webdav_directory(&directory_record).await {
            Ok(_) => {
                debug!("📊 Updated single directory tracking: {} ({} files, {} bytes, ETag: {})", 
                    directory_path, file_count, total_size_bytes, dir_etag);
            }
            Err(e) => {
                error!("Failed to update single directory tracking for {}: {}", directory_path, e);
            }
        }
    }
    
    /// Get a list of directories that need targeted scanning based on recent changes
    pub async fn get_directories_needing_scan(&self, user_id: uuid::Uuid, state: &crate::AppState, max_age_hours: i64) -> Result<Vec<String>> {
        let cutoff_time = chrono::Utc::now() - chrono::Duration::hours(max_age_hours);
        
        match state.db.list_webdav_directories(user_id).await {
            Ok(directories) => {
                let stale_dirs: Vec<String> = directories.iter()
                    .filter(|dir| dir.last_scanned_at < cutoff_time)
                    .map(|dir| dir.directory_path.clone())
                    .collect();
                
                debug!("🕒 Found {} directories not scanned in last {} hours", stale_dirs.len(), max_age_hours);
                Ok(stale_dirs)
            }
            Err(e) => {
                error!("Failed to get directories needing scan: {}", e);
                Err(e.into())
            }
        }
    }
    
    /// Smart sync mode that combines multiple optimization strategies
    pub async fn discover_files_smart_sync(&self, watch_folders: &[String], user_id: uuid::Uuid, state: &crate::AppState) -> Result<Vec<FileInfo>> {
        debug!("🧠 Starting smart sync for {} watch folders", watch_folders.len());
        
        let mut all_files = Vec::new();
        
        for folder_path in watch_folders {
            debug!("🔍 Smart sync processing folder: {}", folder_path);
            
            // Step 1: Try optimized discovery first (checks directory ETag)
            let optimized_result = self.discover_files_in_folder_optimized(folder_path, user_id, state).await;
            
            match optimized_result {
                Ok(files) => {
                    if !files.is_empty() {
                        debug!("✅ Optimized discovery found {} files in {}", files.len(), folder_path);
                        all_files.extend(files);
                    } else {
                        debug!("🔍 Directory {} unchanged, checking for stale subdirectories", folder_path);
                        
                        // Step 2: Check for stale subdirectories that need targeted scanning
                        let stale_dirs = self.get_stale_subdirectories(folder_path, user_id, state, 24).await?;
                        
                        if !stale_dirs.is_empty() {
                            debug!("🎯 Found {} stale subdirectories, performing targeted scan", stale_dirs.len());
                            let targeted_files = self.discover_files_targeted_rescan(&stale_dirs, user_id, state).await?;
                            all_files.extend(targeted_files);
                        } else {
                            debug!("✅ All subdirectories of {} are fresh, no scan needed", folder_path);
                        }
                    }
                }
                Err(e) => {
                    warn!("Optimized discovery failed for {}, falling back to full scan: {}", folder_path, e);
                    // Fallback to traditional full scan
                    match self.discover_files_in_folder(folder_path).await {
                        Ok(files) => {
                            debug!("📂 Fallback scan found {} files in {}", files.len(), folder_path);
                            all_files.extend(files);
                        }
                        Err(fallback_error) => {
                            error!("Both optimized and fallback scans failed for {}: {}", folder_path, fallback_error);
                            return Err(fallback_error);
                        }
                    }
                }
            }
        }
        
        debug!("🧠 Smart sync completed: {} total files discovered", all_files.len());
        Ok(all_files)
    }
    
    /// Get subdirectories of a parent that haven't been scanned recently
    async fn get_stale_subdirectories(&self, parent_path: &str, user_id: uuid::Uuid, state: &crate::AppState, max_age_hours: i64) -> Result<Vec<String>> {
        let cutoff_time = chrono::Utc::now() - chrono::Duration::hours(max_age_hours);
        
        match state.db.list_webdav_directories(user_id).await {
            Ok(directories) => {
                let stale_subdirs: Vec<String> = directories.iter()
                    .filter(|dir| {
                        dir.directory_path.starts_with(parent_path) && 
                        dir.directory_path != parent_path &&
                        dir.last_scanned_at < cutoff_time
                    })
                    .map(|dir| dir.directory_path.clone())
                    .collect();
                
                debug!("🕒 Found {} stale subdirectories under {} (not scanned in {} hours)", 
                    stale_subdirs.len(), parent_path, max_age_hours);
                Ok(stale_subdirs)
            }
            Err(e) => {
                error!("Failed to get stale subdirectories: {}", e);
                Err(e.into())
            }
        }
    }
    
    /// Perform incremental sync - only scan directories that have actually changed
    pub async fn discover_files_incremental(&self, watch_folders: &[String], user_id: uuid::Uuid, state: &crate::AppState) -> Result<Vec<FileInfo>> {
        debug!("⚡ Starting incremental sync for {} watch folders", watch_folders.len());
        
        let mut changed_files = Vec::new();
        let mut unchanged_count = 0;
        let mut changed_count = 0;
        
        for folder_path in watch_folders {
            // Check directory ETag to see if it changed
            match self.check_directory_etag(folder_path).await {
                Ok(current_etag) => {
                    let needs_scan = match state.db.get_webdav_directory(user_id, folder_path).await {
                        Ok(Some(stored_dir)) => {
                            if stored_dir.directory_etag != current_etag {
                                debug!("🔄 Directory {} changed (ETag: {} → {})", folder_path, stored_dir.directory_etag, current_etag);
                                changed_count += 1;
                                true
                            } else {
                                debug!("✅ Directory {} unchanged (ETag: {})", folder_path, current_etag);
                                unchanged_count += 1;
                                false
                            }
                        }
                        Ok(None) => {
                            debug!("🆕 New directory {} detected", folder_path);
                            changed_count += 1;
                            true
                        }
                        Err(e) => {
                            warn!("Database error for {}: {}, scanning to be safe", folder_path, e);
                            changed_count += 1;
                            true
                        }
                    };
                    
                    if needs_scan {
                        // Directory changed - perform targeted scan
                        match self.discover_files_in_folder_optimized(folder_path, user_id, state).await {
                            Ok(mut files) => {
                                debug!("📂 Incremental scan found {} files in changed directory {}", files.len(), folder_path);
                                changed_files.append(&mut files);
                            }
                            Err(e) => {
                                error!("Failed incremental scan of {}: {}", folder_path, e);
                            }
                        }
                    } else {
                        // Directory unchanged - just update scan timestamp
                        let update = crate::models::UpdateWebDAVDirectory {
                            directory_etag: current_etag,
                            last_scanned_at: chrono::Utc::now(),
                            file_count: 0, // Will be updated by the database layer
                            total_size_bytes: 0,
                        };
                        
                        if let Err(e) = state.db.update_webdav_directory(user_id, folder_path, &update).await {
                            warn!("Failed to update scan timestamp for {}: {}", folder_path, e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to check directory ETag for {}: {}", folder_path, e);
                }
            }
        }
        
        debug!("⚡ Incremental sync completed: {} unchanged, {} changed, {} total files found", 
            unchanged_count, changed_count, changed_files.len());
        
        Ok(changed_files)
    }

    /// Check subdirectories individually for changes when parent directory is unchanged
    async fn check_subdirectories_for_changes(&self, parent_path: &str, user_id: uuid::Uuid, state: &crate::AppState) -> Result<Vec<FileInfo>> {
        // First, check if this server supports recursive ETags
        let supports_recursive_etags = match self.config.server_type.as_deref() {
            Some("nextcloud") | Some("owncloud") => true,
            _ => false
        };
        
        if supports_recursive_etags {
            // With recursive ETags, if parent hasn't changed, nothing inside has changed
            debug!("🚀 Server supports recursive ETags - parent {} unchanged means all contents unchanged", parent_path);
            return Ok(Vec::new());
        }
        
        // For servers without recursive ETags, fall back to checking each subdirectory
        debug!("📁 Server doesn't support recursive ETags, checking subdirectories individually");
        
        // Get all known subdirectories from database
        let known_directories = match state.db.list_webdav_directories(user_id).await {
            Ok(dirs) => dirs,
            Err(e) => {
                warn!("Failed to get known directories, falling back to full scan: {}", e);
                return self.discover_files_in_folder_impl(parent_path).await;
            }
        };
        
        // Filter to subdirectories of this parent
        let subdirectories: Vec<_> = known_directories.iter()
            .filter(|dir| dir.directory_path.starts_with(parent_path) && dir.directory_path != parent_path)
            .collect();
            
        if subdirectories.is_empty() {
            debug!("📁 No known subdirectories for {}, performing initial scan to discover structure", parent_path);
            return self.discover_files_in_folder_impl(parent_path).await;
        }
        
        debug!("🔍 Checking {} known subdirectories for changes", subdirectories.len());
        
        let mut changed_files = Vec::new();
        let subdirectory_count = subdirectories.len();
        
        // Check each subdirectory individually
        for subdir in subdirectories {
            let subdir_path = &subdir.directory_path;
            
            // Check if this subdirectory has changed
            match self.check_directory_etag(subdir_path).await {
                Ok(current_etag) => {
                    if current_etag != subdir.directory_etag {
                        debug!("🔄 Subdirectory {} changed (old: {}, new: {}), scanning recursively", 
                            subdir_path, subdir.directory_etag, current_etag);
                        
                        // This subdirectory changed - get all its files recursively
                        match self.discover_files_in_folder_impl(subdir_path).await {
                            Ok(mut subdir_files) => {
                                debug!("📂 Found {} files in changed subdirectory {}", subdir_files.len(), subdir_path);
                                changed_files.append(&mut subdir_files);
                                
                                // Update tracking for this subdirectory and its children
                                self.track_subdirectories_recursively(&subdir_files, user_id, state).await;
                            }
                            Err(e) => {
                                error!("Failed to scan changed subdirectory {}: {}", subdir_path, e);
                            }
                        }
                    } else {
                        debug!("✅ Subdirectory {} unchanged (ETag: {})", subdir_path, current_etag);
                        
                        // Update last_scanned_at even for unchanged directories
                        let update = crate::models::UpdateWebDAVDirectory {
                            directory_etag: current_etag,
                            last_scanned_at: chrono::Utc::now(),
                            file_count: subdir.file_count,
                            total_size_bytes: subdir.total_size_bytes,
                        };
                        
                        if let Err(e) = state.db.update_webdav_directory(user_id, subdir_path, &update).await {
                            warn!("Failed to update scan time for {}: {}", subdir_path, e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to check ETag for subdirectory {}: {}", subdir_path, e);
                    // Don't fail the entire operation, just log and continue
                }
            }
        }
        
        debug!("🎯 Found {} changed files across {} subdirectories", changed_files.len(), subdirectory_count);
        Ok(changed_files)
    }

    /// Check directory ETag without performing deep scan - used for optimization
    pub async fn check_directory_etag(&self, folder_path: &str) -> Result<String> {
        self.retry_with_backoff("check_directory_etag", || {
            self.check_directory_etag_impl(folder_path)
        }).await
    }

    async fn check_directory_etag_impl(&self, folder_path: &str) -> Result<String> {
        let folder_url = format!("{}{}", self.base_webdav_url, folder_path);
        
        debug!("Checking directory ETag for: {}", folder_url);

        let propfind_body = r#"<?xml version="1.0"?>
            <d:propfind xmlns:d="DAV:">
                <d:prop>
                    <d:getetag/>
                </d:prop>
            </d:propfind>"#;

        let response = self.client
            .request(Method::from_bytes(b"PROPFIND").unwrap(), &folder_url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .header("Depth", "0")  // Only check the directory itself, not contents
            .header("Content-Type", "application/xml")
            .body(propfind_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("PROPFIND request failed: {}", response.status()));
        }

        let response_text = response.text().await?;
        debug!("Directory ETag response received, parsing...");

        // Parse the response to extract directory ETag
        self.parse_directory_etag(&response_text)
    }

    pub fn parse_directory_etag(&self, xml_text: &str) -> Result<String> {
        use quick_xml::events::Event;
        use quick_xml::reader::Reader;
        
        let mut reader = Reader::from_str(xml_text);
        reader.config_mut().trim_text(true);
        
        let mut current_element = String::new();
        let mut etag = String::new();
        let mut buf = Vec::new();
        
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                    let local_name = e.local_name();
                    let name = std::str::from_utf8(local_name.as_ref())?;
                    current_element = name.to_lowercase();
                }
                Ok(Event::Text(e)) => {
                    if current_element == "getetag" {
                        etag = e.unescape()?.to_string();
                        break;
                    }
                }
                Ok(Event::End(_)) => {
                    current_element.clear();
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(anyhow!("XML parsing error: {}", e)),
                _ => {}
            }
        }
        
        if etag.is_empty() {
            return Err(anyhow!("No ETag found in directory response"));
        }
        
        // Use existing ETag normalization function from parser module
        let normalized_etag = crate::webdav_xml_parser::normalize_etag(&etag);
        debug!("Directory ETag: {}", normalized_etag);
        
        Ok(normalized_etag)
    }

    async fn discover_files_in_folder_impl(&self, folder_path: &str) -> Result<Vec<FileInfo>> {
        let folder_url = format!("{}{}", self.base_webdav_url, folder_path);
        
        debug!("Discovering files in: {}", folder_url);

        let propfind_body = r#"<?xml version="1.0"?>
            <d:propfind xmlns:d="DAV:">
                <d:allprop/>
            </d:propfind>"#;

        let response = self.client
            .request(Method::from_bytes(b"PROPFIND").unwrap(), &folder_url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .header("Depth", "infinity")  // Get all files recursively
            .header("Content-Type", "application/xml")
            .body(propfind_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("PROPFIND request failed: {}", response.status()));
        }

        let response_text = response.text().await?;
        debug!("WebDAV response received, parsing...");

        self.parse_webdav_response(&response_text)
    }

    pub fn parse_webdav_response(&self, xml_text: &str) -> Result<Vec<FileInfo>> {
        parse_propfind_response(xml_text)
    }

    /// Parse WebDAV response including both files and directories
    /// Used for shallow directory scans where we need to track directory structure
    pub fn parse_webdav_response_with_directories(&self, xml_text: &str) -> Result<Vec<FileInfo>> {
        parse_propfind_response_with_directories(xml_text)
    }
    
    /// Test if the WebDAV server supports recursive ETag propagation
    /// (i.e., parent directory ETags change when child content changes)
    /// This test is read-only and checks existing directory structures
    pub async fn test_recursive_etag_support(&self) -> Result<bool> {
        debug!("🔬 Testing recursive ETag support using existing directory structure");
        
        // Find a directory with subdirectories from our watch folders
        for watch_folder in &self.config.watch_folders {
            // Convert to relative path for API calls
            let relative_watch_folder = self.convert_to_relative_path(watch_folder);
            
            // Get the directory structure with depth 1
            match self.discover_files_in_folder_shallow(&relative_watch_folder).await {
                Ok(entries) => {
                    // Find a subdirectory to test with
                    let subdirs: Vec<_> = entries.iter()
                        .filter(|e| e.is_directory && &e.path != watch_folder)
                        .collect();
                    
                    if subdirs.is_empty() {
                        continue; // Try next watch folder
                    }
                    
                    // Use the first subdirectory for testing
                    let test_subdir = &subdirs[0];
                    debug!("Testing with directory: {} and subdirectory: {}", watch_folder, test_subdir.path);
                    
                    // Step 1: Get parent directory ETag
                    let parent_etag = self.check_directory_etag(&relative_watch_folder).await?;
                    
                    // Step 2: Get subdirectory ETag (convert to relative path)
                    let relative_subdir_path = self.convert_to_relative_path(&test_subdir.path);
                    let subdir_etag = self.check_directory_etag(&relative_subdir_path).await?;
                    
                    // Step 3: Check if parent has a different ETag than child
                    // In a recursive ETag system, they should be different but related
                    // The key test is: if we check the parent again after some time,
                    // and a file deep inside changed, did the parent ETag change?
                    
                    // For now, we'll just check if the server provides ETags at all
                    if !parent_etag.is_empty() && !subdir_etag.is_empty() {
                        debug!("✅ Server provides ETags for directories");
                        debug!("   Parent ETag: {}", parent_etag);
                        debug!("   Subdir ETag: {}", subdir_etag);
                        
                        // Without write access, we can't definitively test recursive propagation
                        // But we can make an educated guess based on the server type
                        let likely_supports_recursive = match self.config.server_type.as_deref() {
                            Some("nextcloud") | Some("owncloud") => {
                                debug!("   Nextcloud/ownCloud servers typically support recursive ETags");
                                true
                            }
                            _ => {
                                debug!("   Unknown server type - recursive ETag support uncertain");
                                false
                            }
                        };
                        
                        return Ok(likely_supports_recursive);
                    }
                }
                Err(e) => {
                    warn!("Failed to scan directory {}: {}", watch_folder, e);
                    continue;
                }
            }
        }
        
        debug!("❓ Could not determine recursive ETag support - no suitable directories found");
        Ok(false)
    }
    
    /// Convert full WebDAV path to relative path for use with base_webdav_url
    pub fn convert_to_relative_path(&self, full_webdav_path: &str) -> String {
        // For Nextcloud/ownCloud paths like "/remote.php/dav/files/username/folder/subfolder/"
        // We need to extract just the "folder/subfolder/" part
        let webdav_prefix = match self.config.server_type.as_deref() {
            Some("nextcloud") | Some("owncloud") => {
                format!("/remote.php/dav/files/{}/", self.config.username)
            },
            _ => "/webdav/".to_string()
        };
        
        if let Some(relative_part) = full_webdav_path.strip_prefix(&webdav_prefix) {
            format!("/{}", relative_part)
        } else {
            // If path doesn't match expected format, return as-is
            full_webdav_path.to_string()
        }
    }

    /// Smart directory scan that uses depth-1 traversal for efficient synchronization
    /// Only scans directories whose ETags have changed, avoiding unnecessary deep scans
    pub fn smart_directory_scan<'a>(
        &'a self, 
        path: &'a str, 
        known_etag: Option<&'a str>,
        user_id: uuid::Uuid,
        state: &'a crate::AppState
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<FileInfo>>> + Send + 'a>> {
        Box::pin(async move {
        debug!("🧠 Smart scan starting for path: {}", path);
        
        // Convert full WebDAV path to relative path for existing functions
        let relative_path = self.convert_to_relative_path(path);
        debug!("🔄 Converted {} to relative path: {}", path, relative_path);
        
        // Step 1: Check current directory ETag
        let current_etag = match self.check_directory_etag(&relative_path).await {
            Ok(etag) => etag,
            Err(e) => {
                warn!("Failed to get directory ETag for {}, falling back to full scan: {}", path, e);
                return self.discover_files_in_folder_impl(&relative_path).await;
            }
        };
        
        // Step 2: If unchanged and we support recursive ETags, nothing to do
        if known_etag == Some(&current_etag) {
            let supports_recursive = match self.config.server_type.as_deref() {
                Some("nextcloud") | Some("owncloud") => true,
                _ => false
            };
            
            if supports_recursive {
                debug!("✅ Directory {} unchanged (recursive ETag: {}), skipping scan", path, current_etag);
                return Ok(Vec::new());
            } else {
                debug!("📁 Directory {} ETag unchanged but server doesn't support recursive ETags, checking subdirectories", path);
            }
        } else {
            debug!("🔄 Directory {} changed (old: {:?}, new: {})", path, known_etag, current_etag);
        }
        
        // Step 3: Directory changed or we need to check subdirectories - do depth-1 scan
        let entries = match self.discover_files_in_folder_shallow(&relative_path).await {
            Ok(files) => files,
            Err(e) => {
                error!("Failed shallow scan of {}: {}", path, e);
                return Err(e);
            }
        };
        
        let mut all_files = Vec::new();
        let mut subdirs_to_scan = Vec::new();
        
        // Separate files and directories
        for entry in entries {
            if entry.is_directory && entry.path != path {
                subdirs_to_scan.push(entry.clone());
            }
            all_files.push(entry);
        }
        
        // Note: We'll update the directory tracking at the end after processing all subdirectories
        // to avoid ETag race conditions during the scan
        
        // Step 4: Process subdirectories concurrently with controlled parallelism
        if !subdirs_to_scan.is_empty() {
            let semaphore = std::sync::Arc::new(Semaphore::new(self.concurrency_config.max_concurrent_scans));
            let subdirs_stream = stream::iter(subdirs_to_scan)
                .map(|subdir| {
                    let semaphore = semaphore.clone();
                    let service = self.clone();
                    async move {
                        let _permit = semaphore.acquire().await.map_err(|e| anyhow!("Semaphore error: {}", e))?;
                        
                        // Get stored ETag for this subdirectory
                        let stored_etag = match state.db.get_webdav_directory(user_id, &subdir.path).await {
                            Ok(Some(dir)) => Some(dir.directory_etag),
                            Ok(None) => {
                                debug!("🆕 New subdirectory discovered: {}", subdir.path);
                                None
                            }
                            Err(e) => {
                                warn!("Database error checking subdirectory {}: {}", subdir.path, e);
                                None
                            }
                        };
                        
                        // If ETag changed or new directory, scan it recursively  
                        if stored_etag.as_deref() != Some(&subdir.etag) {
                            debug!("🔄 Subdirectory {} needs scanning (old: {:?}, new: {})", 
                                subdir.path, stored_etag, subdir.etag);
                                
                            match service.smart_directory_scan_internal(&subdir.path, stored_etag.as_deref(), user_id, state).await {
                                Ok(subdir_files) => {
                                    debug!("📂 Found {} entries in subdirectory {}", subdir_files.len(), subdir.path);
                                    Result::<Vec<FileInfo>, anyhow::Error>::Ok(subdir_files)
                                }
                                Err(e) => {
                                    error!("Failed to scan subdirectory {}: {}", subdir.path, e);
                                    Result::<Vec<FileInfo>, anyhow::Error>::Ok(Vec::new()) // Continue with other subdirectories
                                }
                            }
                        } else {
                            debug!("✅ Subdirectory {} unchanged (ETag: {})", subdir.path, subdir.etag);
                            // Don't update database during scan - will be handled by top-level caller
                            Result::<Vec<FileInfo>, anyhow::Error>::Ok(Vec::new())
                        }
                    }
                })
                .buffer_unordered(self.concurrency_config.max_concurrent_scans);
            
            // Collect all results concurrently
            let mut subdirs_stream = std::pin::pin!(subdirs_stream);
            while let Some(result) = subdirs_stream.next().await {
                match result {
                    Ok(mut subdir_files) => {
                        all_files.append(&mut subdir_files);
                    }
                    Err(e) => {
                        warn!("Concurrent subdirectory scan error: {}", e);
                        // Continue processing other subdirectories
                    }
                }
            }
        }
        
        // Only update database if this is the top-level call (not a recursive subdirectory scan)
        let file_count = all_files.iter().filter(|f| !f.is_directory && self.is_direct_child(&f.path, path)).count() as i64;
        let total_size = all_files.iter()
            .filter(|f| !f.is_directory && self.is_direct_child(&f.path, path))
            .map(|f| f.size)
            .sum::<i64>();
            
        let dir_record = crate::models::CreateWebDAVDirectory {
            user_id,
            directory_path: path.to_string(),
            directory_etag: current_etag.clone(),
            file_count,
            total_size_bytes: total_size,
        };
        
        if let Err(e) = state.db.create_or_update_webdav_directory(&dir_record).await {
            warn!("Failed to update directory tracking for {}: {}", path, e);
        }
        
        debug!("🧠 Smart scan completed for {}: {} total entries found", path, all_files.len());
        Ok(all_files)
        })
    }

    /// Internal version of smart_directory_scan that doesn't update the database
    /// Used for recursive subdirectory scanning to avoid race conditions
    fn smart_directory_scan_internal<'a>(
        &'a self, 
        path: &'a str, 
        known_etag: Option<&'a str>,
        user_id: uuid::Uuid,
        state: &'a crate::AppState
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<FileInfo>>> + Send + 'a>> {
        Box::pin(async move {
        debug!("🧠 Smart scan (internal) starting for path: {}", path);
        
        // Convert full WebDAV path to relative path for existing functions
        let relative_path = self.convert_to_relative_path(path);
        debug!("🔄 Converted {} to relative path: {}", path, relative_path);
        
        // Step 1: Check current directory ETag
        let current_etag = match self.check_directory_etag(&relative_path).await {
            Ok(etag) => etag,
            Err(e) => {
                warn!("Failed to get directory ETag for {}, falling back to full scan: {}", path, e);
                return self.discover_files_in_folder_impl(&relative_path).await;
            }
        };
        
        // Step 2: If unchanged and we support recursive ETags, nothing to do
        if known_etag == Some(&current_etag) {
            let supports_recursive = match self.config.server_type.as_deref() {
                Some("nextcloud") | Some("owncloud") => true,
                _ => false
            };
            
            if supports_recursive {
                debug!("✅ Directory {} unchanged (recursive ETag: {}), skipping scan", path, current_etag);
                return Ok(Vec::new());
            } else {
                debug!("📁 Directory {} ETag unchanged but server doesn't support recursive ETags, checking subdirectories", path);
            }
        } else {
            debug!("🔄 Directory {} changed (old: {:?}, new: {})", path, known_etag, current_etag);
        }
        
        // Step 3: Directory changed or we need to check subdirectories - do depth-1 scan
        let entries = match self.discover_files_in_folder_shallow(&relative_path).await {
            Ok(files) => files,
            Err(e) => {
                error!("Failed shallow scan of {}: {}", path, e);
                return Err(e);
            }
        };
        
        let mut all_files = Vec::new();
        let mut subdirs_to_scan = Vec::new();
        
        // Separate files and directories
        for entry in entries {
            if entry.is_directory && entry.path != path {
                subdirs_to_scan.push(entry.clone());
            }
            all_files.push(entry);
        }
        
        // Note: No database update in internal function to avoid race conditions
        
        // Step 4: Process subdirectories concurrently with controlled parallelism
        if !subdirs_to_scan.is_empty() {
            let semaphore = std::sync::Arc::new(Semaphore::new(self.concurrency_config.max_concurrent_scans));
            let subdirs_stream = stream::iter(subdirs_to_scan)
                .map(|subdir| {
                    let semaphore = semaphore.clone();
                    let service = self.clone();
                    async move {
                        let _permit = semaphore.acquire().await.map_err(|e| anyhow!("Semaphore error: {}", e))?;
                        
                        // Get stored ETag for this subdirectory
                        let stored_etag = match state.db.get_webdav_directory(user_id, &subdir.path).await {
                            Ok(Some(dir)) => Some(dir.directory_etag),
                            Ok(None) => {
                                debug!("🆕 New subdirectory discovered: {}", subdir.path);
                                None
                            }
                            Err(e) => {
                                warn!("Database error checking subdirectory {}: {}", subdir.path, e);
                                None
                            }
                        };
                        
                        // If ETag changed or new directory, scan it recursively
                        if stored_etag.as_deref() != Some(&subdir.etag) {
                            debug!("🔄 Subdirectory {} needs scanning (old: {:?}, new: {})", 
                                subdir.path, stored_etag, subdir.etag);
                                
                            match service.smart_directory_scan_internal(&subdir.path, stored_etag.as_deref(), user_id, state).await {
                                Ok(subdir_files) => {
                                    debug!("📂 Found {} entries in subdirectory {}", subdir_files.len(), subdir.path);
                                    Result::<Vec<FileInfo>, anyhow::Error>::Ok(subdir_files)
                                }
                                Err(e) => {
                                    error!("Failed to scan subdirectory {}: {}", subdir.path, e);
                                    Result::<Vec<FileInfo>, anyhow::Error>::Ok(Vec::new()) // Continue with other subdirectories
                                }
                            }
                        } else {
                            debug!("✅ Subdirectory {} unchanged (ETag: {})", subdir.path, subdir.etag);
                            // Don't update database during internal scan
                            Result::<Vec<FileInfo>, anyhow::Error>::Ok(Vec::new())
                        }
                    }
                })
                .buffer_unordered(self.concurrency_config.max_concurrent_scans);
            
            // Collect all results concurrently
            let mut subdirs_stream = std::pin::pin!(subdirs_stream);
            while let Some(result) = subdirs_stream.next().await {
                match result {
                    Ok(mut subdir_files) => {
                        all_files.append(&mut subdir_files);
                    }
                    Err(e) => {
                        warn!("Concurrent subdirectory scan error: {}", e);
                        // Continue processing other subdirectories
                    }
                }
            }
        }
        
        debug!("🧠 Smart scan (internal) completed for {}: {} total entries found", path, all_files.len());
        Ok(all_files)
        })
    }

    /// Smart directory scan with checkpoint-based crash recovery
    pub fn smart_directory_scan_with_checkpoints<'a>(
        &'a self, 
        path: &'a str, 
        known_etag: Option<&'a str>,
        user_id: uuid::Uuid,
        state: &'a crate::AppState
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<FileInfo>>> + Send + 'a>> {
        Box::pin(async move {
            debug!("🧠 Smart scan with checkpoints starting for path: {}", path);
            
            // Mark scan as in progress (checkpoint)
            if let Err(e) = self.mark_scan_in_progress(user_id, path, state).await {
                warn!("Failed to mark scan in progress for {}: {}", path, e);
            }
            
            // Perform the actual scan
            let result = self.smart_directory_scan_internal(path, known_etag, user_id, state).await;
            
            match &result {
                Ok(files) => {
                    debug!("✅ Smart scan completed for {}: {} files", path, files.len());
                    
                    // Update directory tracking and mark scan complete
                    let file_count = files.iter().filter(|f| !f.is_directory && self.is_direct_child(&f.path, path)).count() as i64;
                    let total_size = files.iter()
                        .filter(|f| !f.is_directory && self.is_direct_child(&f.path, path))
                        .map(|f| f.size)
                        .sum::<i64>();
                    
                    let current_etag = known_etag.unwrap_or("unknown").to_string();
                    let dir_record = crate::models::CreateWebDAVDirectory {
                        user_id,
                        directory_path: path.to_string(),
                        directory_etag: current_etag.clone(),
                        file_count,
                        total_size_bytes: total_size,
                    };
                    
                    if let Err(e) = state.db.create_or_update_webdav_directory(&dir_record).await {
                        warn!("Failed to update directory tracking for {}: {}", path, e);
                    }
                    
                    // Mark scan as complete (remove checkpoint)
                    if let Err(e) = self.mark_scan_complete(user_id, path, state).await {
                        warn!("Failed to mark scan complete for {}: {}", path, e);
                    }
                }
                Err(e) => {
                    error!("❌ Smart scan failed for {}: {}", path, e);
                    // Mark scan as failed for better tracking
                    if let Err(mark_err) = state.db.mark_webdav_scan_failed(user_id, path, &e.to_string()).await {
                        warn!("Failed to mark scan as failed for {}: {}", path, mark_err);
                    }
                }
            }
            
            result
        })
    }

    /// Detect directories with incomplete scans that need recovery
    async fn detect_incomplete_scans(&self, user_id: uuid::Uuid, state: &crate::AppState) -> Result<Vec<String>> {
        debug!("🔍 Checking for incomplete scans...");
        
        // Check for both incomplete scans and stale scans (running too long, likely crashed)
        let mut incomplete_scans = state.db.get_incomplete_webdav_scans(user_id).await.unwrap_or_default();
        let stale_scans = state.db.get_stale_webdav_scans(user_id, 30).await.unwrap_or_default(); // 30 minute timeout
        
        // Combine and deduplicate
        incomplete_scans.extend(stale_scans);
        incomplete_scans.sort();
        incomplete_scans.dedup();
        
        if !incomplete_scans.is_empty() {
            info!("Found {} incomplete/stale scans to recover", incomplete_scans.len());
        }
        
        Ok(incomplete_scans)
    }

    /// Mark a directory scan as in progress (for crash recovery)
    async fn mark_scan_in_progress(&self, user_id: uuid::Uuid, path: &str, state: &crate::AppState) -> Result<()> {
        debug!("📝 Marking scan in progress for: {}", path);
        state.db.mark_webdav_scan_in_progress(user_id, path).await
    }

    /// Mark a directory scan as complete (remove crash recovery checkpoint)
    async fn mark_scan_complete(&self, user_id: uuid::Uuid, path: &str, state: &crate::AppState) -> Result<()> {
        debug!("✅ Marking scan complete for: {}", path);
        state.db.mark_webdav_scan_complete(user_id, path).await
    }

    /// Resume a deep scan from a checkpoint after server restart/interruption
    pub async fn resume_deep_scan(&self, checkpoint_path: &str, user_id: uuid::Uuid, state: &crate::AppState) -> Result<Vec<FileInfo>> {
        self.resume_deep_scan_internal(checkpoint_path, user_id, state).await
    }

    /// Internal resume function that doesn't trigger crash recovery detection (to avoid recursion)
    async fn resume_deep_scan_internal(&self, checkpoint_path: &str, user_id: uuid::Uuid, state: &crate::AppState) -> Result<Vec<FileInfo>> {
        info!("🔄 Resuming deep scan from checkpoint: {}", checkpoint_path);
        
        // Check if the checkpoint directory is still accessible
        let relative_checkpoint_path = self.convert_to_relative_path(checkpoint_path);
        match self.check_directory_etag(&relative_checkpoint_path).await {
            Ok(current_etag) => {
                info!("✅ Checkpoint directory accessible, resuming scan");
                
                // Check if directory changed since checkpoint
                match state.db.get_webdav_directory(user_id, checkpoint_path).await {
                    Ok(Some(stored_dir)) => {
                        if stored_dir.directory_etag != current_etag {
                            info!("🔄 Directory changed since checkpoint, performing full rescan");
                        } else {
                            info!("✅ Directory unchanged since checkpoint, can skip");
                            return Ok(Vec::new());
                        }
                    }
                    Ok(None) => {
                        info!("🆕 New checkpoint directory, performing full scan");
                    }
                    Err(e) => {
                        warn!("Database error checking checkpoint {}: {}, performing full scan", checkpoint_path, e);
                    }
                }
                
                // Resume with smart scanning from this point
                self.smart_directory_scan_with_checkpoints(checkpoint_path, None, user_id, state).await
            }
            Err(e) => {
                warn!("Checkpoint directory {} inaccessible after restart: {}", checkpoint_path, e);
                // Server might have restarted, wait a bit and retry
                tokio::time::sleep(Duration::from_secs(5)).await;
                
                match self.check_directory_etag(&relative_checkpoint_path).await {
                    Ok(_) => {
                        info!("🔄 Server recovered, resuming scan");
                        self.smart_directory_scan_with_checkpoints(checkpoint_path, None, user_id, state).await
                    }
                    Err(e2) => {
                        error!("Failed to resume deep scan after server restart: {}", e2);
                        Err(anyhow!("Cannot resume deep scan: server unreachable after restart"))
                    }
                }
            }
        }
    }

    /// Discover files in multiple folders concurrently with rate limiting
    pub async fn discover_files_concurrent(&self, folders: &[String], user_id: uuid::Uuid, state: &crate::AppState) -> Result<Vec<FileInfo>> {
        if folders.is_empty() {
            return Ok(Vec::new());
        }
        
        info!("🚀 Starting concurrent discovery for {} folders", folders.len());
        
        let semaphore = std::sync::Arc::new(Semaphore::new(self.concurrency_config.max_concurrent_scans));
        let folders_stream = stream::iter(folders.iter())
            .map(|folder_path| {
                let semaphore = semaphore.clone();
                let service = self.clone();
                let folder_path = folder_path.clone();
                async move {
                    let _permit = semaphore.acquire().await.map_err(|e| anyhow!("Semaphore error: {}", e))?;
                    
                    info!("📂 Scanning folder: {}", folder_path);
                    let start_time = std::time::Instant::now();
                    
                    // Save checkpoint for resumption after interruption
                    let checkpoint_record = crate::models::CreateWebDAVDirectory {
                        user_id,
                        directory_path: folder_path.clone(),
                        directory_etag: "scanning".to_string(), // Temporary marker
                        file_count: 0,
                        total_size_bytes: 0,
                    };
                    
                    if let Err(e) = state.db.create_or_update_webdav_directory(&checkpoint_record).await {
                        warn!("Failed to save scan checkpoint for {}: {}", folder_path, e);
                    }
                    
                    let result = service.discover_files_in_folder_optimized(&folder_path, user_id, state).await;
                    
                    match &result {
                        Ok(files) => {
                            let duration = start_time.elapsed();
                            info!("✅ Completed folder {} in {:?}: {} files found", 
                                folder_path, duration, files.len());
                        }
                        Err(e) => {
                            // Check if this was a server restart/connection issue
                            if service.is_server_restart_error(e) {
                                warn!("🔄 Server restart detected during scan of {}, will resume later", folder_path);
                                // Keep checkpoint for resumption
                                return Err(anyhow!("Server restart detected: {}", e));
                            } else {
                                error!("❌ Failed to scan folder {}: {}", folder_path, e);
                            }
                        }
                    }
                    
                    result.map(|files| (folder_path, files))
                }
            })
            .buffer_unordered(self.concurrency_config.max_concurrent_scans);
        
        let mut all_files = Vec::new();
        let mut success_count = 0;
        let mut error_count = 0;
        
        let mut folders_stream = std::pin::pin!(folders_stream);
        while let Some(result) = folders_stream.next().await {
            match result {
                Ok((folder_path, mut files)) => {
                    debug!("📁 Folder {} contributed {} files", folder_path, files.len());
                    all_files.append(&mut files);
                    success_count += 1;
                }
                Err(e) => {
                    warn!("Folder scan error: {}", e);
                    error_count += 1;
                }
            }
        }
        
        info!("🎯 Concurrent discovery completed: {} folders successful, {} failed, {} total files", 
            success_count, error_count, all_files.len());
        
        Ok(all_files)
    }

    pub async fn download_file(&self, file_path: &str) -> Result<Vec<u8>> {
        self.retry_with_backoff("download_file", || {
            self.download_file_impl(file_path)
        }).await
    }

    async fn download_file_impl(&self, file_path: &str) -> Result<Vec<u8>> {
        // For Nextcloud/ownCloud, the file_path might already be an absolute WebDAV path
        // The path comes from href which is already URL-encoded
        let file_url = if file_path.starts_with("/remote.php/dav/") {
            // Use the server URL + the full WebDAV path
            // Don't double-encode - the path from href is already properly encoded
            format!("{}{}", self.config.server_url.trim_end_matches('/'), file_path)
        } else {
            // Traditional approach for other WebDAV servers or relative paths
            format!("{}{}", self.base_webdav_url, file_path)
        };
        
        debug!("Downloading file: {}", file_url);
        debug!("Original file_path: {}", file_path);

        let response = self.client
            .get(&file_url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("File download failed: {}", response.status()));
        }

        let bytes = response.bytes().await?;
        debug!("Downloaded {} bytes", bytes.len());
        
        Ok(bytes.to_vec())
    }

}

pub async fn test_webdav_connection(
    server_url: &str,
    username: &str,
    password: &str,
) -> Result<bool> {
    let client = Client::new();
    
    // Try to list the root directory to test connectivity
    let response = client
        .request(Method::from_bytes(b"PROPFIND")?, server_url)
        .header("Depth", "0")
        .basic_auth(username, Some(password))
        .timeout(Duration::from_secs(10))
        .send()
        .await?;

    Ok(response.status().is_success())
}

impl WebDAVService {
    /// Validate ETag tracking integrity and directory consistency
    /// This replaces the need for periodic deep scans with intelligent validation
    pub async fn validate_etag_tracking(&self, user_id: uuid::Uuid, state: &crate::AppState) -> Result<ValidationReport> {
        let validation_id = uuid::Uuid::new_v4();
        let started_at = chrono::Utc::now();
        
        info!("🔍 Starting ETag validation for user {} (validation_id: {})", user_id, validation_id);
        
        let mut report = ValidationReport {
            validation_id,
            user_id,
            started_at,
            completed_at: None,
            total_directories_checked: 0,
            issues_found: Vec::new(),
            recommendations: Vec::new(),
            etag_support_verified: false,
            server_health_score: 100,
        };

        // Step 1: Verify ETag support is still working
        match self.test_recursive_etag_support().await {
            Ok(supports_etags) => {
                report.etag_support_verified = supports_etags;
                if !supports_etags {
                    report.issues_found.push(ValidationIssue {
                        issue_type: ValidationIssueType::ETagUnreliable,
                        directory_path: "server".to_string(),
                        severity: ValidationSeverity::Critical,
                        description: "Server no longer supports recursive ETags reliably".to_string(),
                        discovered_at: chrono::Utc::now(),
                    });
                    report.server_health_score = 30;
                }
            }
            Err(e) => {
                warn!("Failed to test ETag support: {}", e);
                report.issues_found.push(ValidationIssue {
                    issue_type: ValidationIssueType::ETagUnreliable,
                    directory_path: "server".to_string(),
                    severity: ValidationSeverity::Error,
                    description: format!("Cannot verify ETag support: {}", e),
                    discovered_at: chrono::Utc::now(),
                });
                report.server_health_score = 50;
            }
        }

        // Step 2: Check tracked directories for issues
        match state.db.list_webdav_directories(user_id).await {
            Ok(tracked_dirs) => {
                report.total_directories_checked = tracked_dirs.len() as u32;
                
                for tracked_dir in tracked_dirs {
                    self.validate_single_directory(&tracked_dir, &mut report, state).await;
                }
            }
            Err(e) => {
                error!("Failed to load tracked directories: {}", e);
                report.issues_found.push(ValidationIssue {
                    issue_type: ValidationIssueType::Missing,
                    directory_path: "database".to_string(),
                    severity: ValidationSeverity::Critical,
                    description: format!("Cannot access directory tracking database: {}", e),
                    discovered_at: chrono::Utc::now(),
                });
                report.server_health_score = 10;
            }
        }

        // Step 3: Sample a few watch directories to check for untracked directories
        for watch_folder in &self.config.watch_folders {
            if let Err(e) = self.check_for_untracked_directories(watch_folder, &mut report, user_id, state).await {
                warn!("Failed to check for untracked directories in {}: {}", watch_folder, e);
            }
        }

        // Step 4: Generate recommendations based on issues found
        self.generate_validation_recommendations(&mut report);

        report.completed_at = Some(chrono::Utc::now());
        let duration = report.completed_at.unwrap() - report.started_at;
        
        info!("✅ ETag validation completed in {:.2}s. Health score: {}/100, {} issues found", 
              duration.num_milliseconds() as f64 / 1000.0, 
              report.server_health_score,
              report.issues_found.len());

        Ok(report)
    }

    /// Validate a single tracked directory
    async fn validate_single_directory(
        &self, 
        tracked_dir: &crate::models::WebDAVDirectory, 
        report: &mut ValidationReport,
        state: &crate::AppState
    ) {
        let relative_path = self.convert_to_relative_path(&tracked_dir.directory_path);
        
        // Check if directory still exists and get current ETag
        match self.check_directory_etag(&relative_path).await {
            Ok(current_etag) => {
                // Check for ETag mismatch
                if current_etag != tracked_dir.directory_etag {
                    report.issues_found.push(ValidationIssue {
                        issue_type: ValidationIssueType::ETagMismatch,
                        directory_path: tracked_dir.directory_path.clone(),
                        severity: ValidationSeverity::Warning,
                        description: format!("ETag changed from '{}' to '{}' - directory may need rescanning", 
                                           tracked_dir.directory_etag, current_etag),
                        discovered_at: chrono::Utc::now(),
                    });
                    report.server_health_score = report.server_health_score.saturating_sub(5);
                }
                
                // Check for stale directories (not scanned in >7 days)
                let last_scanned = tracked_dir.last_scanned_at;
                let duration = chrono::Utc::now() - last_scanned;
                let days_old = duration.num_days();
                if days_old > 7 {
                    report.issues_found.push(ValidationIssue {
                        issue_type: ValidationIssueType::Stale,
                        directory_path: tracked_dir.directory_path.clone(),
                        severity: if days_old > 30 { ValidationSeverity::Warning } else { ValidationSeverity::Info },
                        description: format!("Directory not scanned for {} days", days_old),
                        discovered_at: chrono::Utc::now(),
                    });
                    if days_old > 30 {
                        report.server_health_score = report.server_health_score.saturating_sub(3);
                    }
                }
            }
            Err(e) => {
                // Directory inaccessible or missing
                report.issues_found.push(ValidationIssue {
                    issue_type: ValidationIssueType::Inaccessible,
                    directory_path: tracked_dir.directory_path.clone(),
                    severity: ValidationSeverity::Error,
                    description: format!("Cannot access directory: {}", e),
                    discovered_at: chrono::Utc::now(),
                });
                report.server_health_score = report.server_health_score.saturating_sub(10);
            }
        }
    }

    /// Check for directories that exist on server but aren't tracked
    async fn check_for_untracked_directories(
        &self, 
        watch_folder: &str,
        report: &mut ValidationReport,
        user_id: uuid::Uuid,
        state: &crate::AppState
    ) -> Result<()> {
        let relative_watch_folder = self.convert_to_relative_path(watch_folder);
        
        // Get shallow listing of watch folder
        match self.discover_files_in_folder_shallow(&relative_watch_folder).await {
            Ok(entries) => {
                // Find directories
                let server_dirs: Vec<_> = entries.iter()
                    .filter(|e| e.is_directory)
                    .collect();
                
                // Check if each directory is tracked
                for server_dir in server_dirs {
                    match state.db.get_webdav_directory(user_id, &server_dir.path).await {
                        Ok(None) => {
                            // Directory exists on server but not tracked
                            report.issues_found.push(ValidationIssue {
                                issue_type: ValidationIssueType::Untracked,
                                directory_path: server_dir.path.clone(),
                                severity: ValidationSeverity::Info,
                                description: "Directory exists on server but not in tracking database".to_string(),
                                discovered_at: chrono::Utc::now(),
                            });
                            report.server_health_score = report.server_health_score.saturating_sub(2);
                        }
                        Ok(Some(_)) => {
                            // Directory is tracked, all good
                        }
                        Err(e) => {
                            warn!("Database error checking directory {}: {}", server_dir.path, e);
                        }
                    }
                }
            }
            Err(e) => {
                return Err(anyhow!("Failed to list watch folder {}: {}", watch_folder, e));
            }
        }
        
        Ok(())
    }

    /// Generate actionable recommendations based on validation issues
    fn generate_validation_recommendations(&self, report: &mut ValidationReport) {
        let mut etag_mismatches = Vec::new();
        let mut untracked_dirs = Vec::new();
        let mut inaccessible_dirs = Vec::new();
        let mut stale_dirs = Vec::new();
        let mut etag_unreliable = false;

        // Categorize issues
        for issue in &report.issues_found {
            match issue.issue_type {
                ValidationIssueType::ETagMismatch => etag_mismatches.push(issue.directory_path.clone()),
                ValidationIssueType::Untracked => untracked_dirs.push(issue.directory_path.clone()),
                ValidationIssueType::Inaccessible => inaccessible_dirs.push(issue.directory_path.clone()),
                ValidationIssueType::Stale => stale_dirs.push(issue.directory_path.clone()),
                ValidationIssueType::ETagUnreliable => etag_unreliable = true,
                _ => {}
            }
        }

        // Generate recommendations
        if etag_unreliable {
            report.recommendations.push(ValidationRecommendation {
                action: ValidationAction::DisableETagOptimization,
                reason: "ETag support is unreliable, consider switching to periodic deep scans".to_string(),
                affected_directories: vec!["all".to_string()],
                priority: ValidationSeverity::Critical,
            });
        } else if !etag_mismatches.is_empty() {
            report.recommendations.push(ValidationRecommendation {
                action: ValidationAction::DeepScanRequired,
                reason: format!("{} directories have ETag mismatches and need rescanning", etag_mismatches.len()),
                affected_directories: etag_mismatches,
                priority: ValidationSeverity::Warning,
            });
        }

        if !untracked_dirs.is_empty() {
            report.recommendations.push(ValidationRecommendation {
                action: ValidationAction::DeepScanRequired,
                reason: format!("{} untracked directories found on server", untracked_dirs.len()),
                affected_directories: untracked_dirs,
                priority: ValidationSeverity::Info,
            });
        }

        if !inaccessible_dirs.is_empty() {
            report.recommendations.push(ValidationRecommendation {
                action: ValidationAction::CheckServerConfiguration,
                reason: format!("{} directories are inaccessible", inaccessible_dirs.len()),
                affected_directories: inaccessible_dirs,
                priority: ValidationSeverity::Error,
            });
        }

        if !stale_dirs.is_empty() && stale_dirs.len() > 10 {
            report.recommendations.push(ValidationRecommendation {
                action: ValidationAction::DeepScanRequired,
                reason: format!("{} directories haven't been scanned recently", stale_dirs.len()),
                affected_directories: stale_dirs,
                priority: ValidationSeverity::Info,
            });
        }

        // If no major issues, everything is healthy
        if report.recommendations.is_empty() {
            report.recommendations.push(ValidationRecommendation {
                action: ValidationAction::NoActionRequired,
                reason: "ETag tracking system is healthy and working correctly".to_string(),
                affected_directories: Vec::new(),
                priority: ValidationSeverity::Info,
            });
        }
    }

    /// Check if we should trigger a deep scan based on validation results
    pub fn should_trigger_deep_scan(&self, report: &ValidationReport) -> (bool, String) {
        // Critical issues always trigger deep scan
        let critical_issues = report.issues_found.iter()
            .filter(|issue| matches!(issue.severity, ValidationSeverity::Critical))
            .count();
        
        if critical_issues > 0 {
            return (true, format!("{} critical issues detected", critical_issues));
        }

        // Multiple ETag mismatches suggest systematic issues
        let etag_mismatches = report.issues_found.iter()
            .filter(|issue| matches!(issue.issue_type, ValidationIssueType::ETagMismatch))
            .count();
        
        if etag_mismatches > 5 {
            return (true, format!("{} ETag mismatches suggest synchronization issues", etag_mismatches));
        }

        // Many untracked directories suggest incomplete initial scan
        let untracked = report.issues_found.iter()
            .filter(|issue| matches!(issue.issue_type, ValidationIssueType::Untracked))
            .count();
        
        if untracked > 10 {
            return (true, format!("{} untracked directories found", untracked));
        }

        // Low health score indicates general problems
        if report.server_health_score < 70 {
            return (true, format!("Low server health score: {}/100", report.server_health_score));
        }

        (false, "System appears healthy, no deep scan needed".to_string())
    }

    /// Ensure complete directory tree discovery before marking deep scan as complete
    /// This is the MOST CRITICAL function - guarantees we've found ALL subdirectories
    pub async fn ensure_complete_directory_discovery(&self, user_id: uuid::Uuid, state: &crate::AppState) -> Result<DirectoryDiscoveryReport> {
        info!("🔍 Starting complete directory tree discovery verification");
        
        let mut report = DirectoryDiscoveryReport {
            discovery_id: uuid::Uuid::new_v4(),
            user_id,
            started_at: chrono::Utc::now(),
            completed_at: None,
            watch_folders_processed: Vec::new(),
            total_directories_discovered: 0,
            new_directories_found: 0,
            missing_directories_detected: 0,
            is_complete: false,
            issues: Vec::new(),
        };

        // Process each watch folder to ensure complete discovery
        for watch_folder in &self.config.watch_folders {
            info!("📂 Ensuring complete discovery for watch folder: {}", watch_folder);
            
            match self.ensure_watch_folder_complete_discovery(watch_folder, user_id, state, &mut report).await {
                Ok(folder_report) => {
                    report.watch_folders_processed.push(folder_report);
                }
                Err(e) => {
                    error!("❌ Failed to ensure complete discovery for {}: {}", watch_folder, e);
                    report.issues.push(format!("Failed to process {}: {}", watch_folder, e));
                }
            }
        }

        // Verify completeness by checking for any gaps
        self.verify_directory_tree_completeness(&mut report, user_id, state).await?;

        report.completed_at = Some(chrono::Utc::now());
        let duration = report.completed_at.unwrap() - report.started_at;
        
        if report.is_complete {
            info!("✅ Complete directory discovery verified in {:.2}s. {} total directories, {} newly discovered", 
                  duration.num_milliseconds() as f64 / 1000.0,
                  report.total_directories_discovered,
                  report.new_directories_found);
        } else {
            warn!("⚠️ Directory discovery incomplete after {:.2}s. {} issues found", 
                  duration.num_milliseconds() as f64 / 1000.0,
                  report.issues.len());
        }

        Ok(report)
    }

    /// Ensure a single watch folder has complete n-depth directory discovery
    async fn ensure_watch_folder_complete_discovery(
        &self, 
        watch_folder: &str, 
        user_id: uuid::Uuid,
        state: &crate::AppState,
        main_report: &mut DirectoryDiscoveryReport
    ) -> Result<WatchFolderDiscoveryReport> {
        let mut folder_report = WatchFolderDiscoveryReport {
            watch_folder: watch_folder.to_string(),
            total_directories: 0,
            new_directories: 0,
            depth_levels_scanned: 0,
            is_complete: false,
        };

        // Use PROPFIND with Depth: infinity to get COMPLETE directory tree
        let relative_watch_folder = self.convert_to_relative_path(watch_folder);
        let all_entries = self.discover_files_in_folder_impl(&relative_watch_folder).await?;
        
        // Extract ALL directories from the complete scan
        let all_server_directories: Vec<_> = all_entries.iter()
            .filter(|entry| entry.is_directory)
            .collect();

        folder_report.total_directories = all_server_directories.len();
        main_report.total_directories_discovered += all_server_directories.len();

        // Calculate depth levels
        let max_depth = all_server_directories.iter()
            .map(|dir| dir.path.chars().filter(|&c| c == '/').count())
            .max()
            .unwrap_or(0);
        folder_report.depth_levels_scanned = max_depth;

        info!("📊 Found {} directories across {} depth levels in {}", 
              all_server_directories.len(), max_depth, watch_folder);

        // Check each directory against our tracking database
        for server_dir in &all_server_directories {
            match state.db.get_webdav_directory(user_id, &server_dir.path).await {
                Ok(Some(tracked_dir)) => {
                    // Directory is already tracked - verify ETag is current
                    if tracked_dir.directory_etag != server_dir.etag {
                        debug!("🔄 Updating ETag for tracked directory: {}", server_dir.path);
                        let update = crate::models::UpdateWebDAVDirectory {
                            directory_etag: server_dir.etag.clone(),
                            last_scanned_at: chrono::Utc::now(),
                            file_count: 0, // Will be calculated separately
                            total_size_bytes: 0,
                        };
                        if let Err(e) = state.db.update_webdav_directory(user_id, &server_dir.path, &update).await {
                            warn!("Failed to update directory {}: {}", server_dir.path, e);
                        }
                    }
                }
                Ok(None) => {
                    // NEW DIRECTORY DISCOVERED - this is critical to track
                    info!("🆕 NEW directory discovered: {}", server_dir.path);
                    folder_report.new_directories += 1;
                    main_report.new_directories_found += 1;

                    // Immediately add to tracking database
                    let new_dir = crate::models::CreateWebDAVDirectory {
                        user_id,
                        directory_path: server_dir.path.clone(),
                        directory_etag: server_dir.etag.clone(),
                        file_count: 0, // Will be calculated when files are processed
                        total_size_bytes: 0,
                    };

                    if let Err(e) = state.db.create_or_update_webdav_directory(&new_dir).await {
                        error!("❌ CRITICAL: Failed to track new directory {}: {}", server_dir.path, e);
                        main_report.issues.push(format!("Failed to track new directory {}: {}", server_dir.path, e));
                    } else {
                        debug!("✅ Successfully tracking new directory: {}", server_dir.path);
                    }
                }
                Err(e) => {
                    error!("Database error checking directory {}: {}", server_dir.path, e);
                    main_report.issues.push(format!("Database error for {}: {}", server_dir.path, e));
                }
            }
        }

        // Check for orphaned tracking entries (directories we track but don't exist on server)
        match state.db.list_webdav_directories(user_id).await {
            Ok(tracked_dirs) => {
                let server_paths: HashSet<String> = all_server_directories.iter()
                    .map(|d| d.path.clone())
                    .collect();

                for tracked_dir in tracked_dirs {
                    if tracked_dir.directory_path.starts_with(watch_folder) && !server_paths.contains(&tracked_dir.directory_path) {
                        warn!("🗑️ Orphaned directory tracking detected: {} (exists in DB but not on server)", tracked_dir.directory_path);
                        main_report.missing_directories_detected += 1;
                        
                        // Could optionally clean up orphaned entries here
                        // For now, just report them
                    }
                }
            }
            Err(e) => {
                error!("Failed to check for orphaned directories: {}", e);
                main_report.issues.push(format!("Failed to check orphaned directories: {}", e));
            }
        }

        folder_report.is_complete = folder_report.new_directories == 0 || main_report.issues.is_empty();
        Ok(folder_report)
    }

    /// Final verification that directory tree coverage is complete
    async fn verify_directory_tree_completeness(
        &self,
        report: &mut DirectoryDiscoveryReport,
        user_id: uuid::Uuid,
        state: &crate::AppState
    ) -> Result<()> {
        info!("🔍 Performing final completeness verification");

        // Check that we have no scan_in_progress flags left over
        match state.db.get_incomplete_webdav_scans(user_id).await {
            Ok(incomplete) => {
                if !incomplete.is_empty() {
                    warn!("⚠️ Found {} incomplete scans still in progress", incomplete.len());
                    report.issues.push(format!("{} scans still marked as in progress", incomplete.len()));
                    report.is_complete = false;
                    return Ok(());
                }
            }
            Err(e) => {
                error!("Failed to check incomplete scans: {}", e);
                report.issues.push(format!("Cannot verify scan completeness: {}", e));
                report.is_complete = false;
                return Ok(());
            }
        }

        // Verify each watch folder has at least some tracked directories
        for watch_folder in &self.config.watch_folders {
            match state.db.list_webdav_directories(user_id).await {
                Ok(dirs) => {
                    let watch_folder_dirs = dirs.iter()
                        .filter(|d| d.directory_path.starts_with(watch_folder))
                        .count();
                    
                    if watch_folder_dirs == 0 {
                        warn!("⚠️ No directories tracked for watch folder: {}", watch_folder);
                        report.issues.push(format!("No directories tracked for watch folder: {}", watch_folder));
                        report.is_complete = false;
                    } else {
                        debug!("✅ Watch folder {} has {} tracked directories", watch_folder, watch_folder_dirs);
                    }
                }
                Err(e) => {
                    error!("Failed to verify watch folder {}: {}", watch_folder, e);
                    report.issues.push(format!("Cannot verify watch folder {}: {}", watch_folder, e));
                    report.is_complete = false;
                }
            }
        }

        // If no issues found, mark as complete
        if report.issues.is_empty() {
            report.is_complete = true;
            info!("✅ Directory tree completeness verified - all {} watch folders fully discovered", self.config.watch_folders.len());
        } else {
            warn!("❌ Directory tree completeness verification failed: {} issues", report.issues.len());
        }

        Ok(())
    }

    /// Modified deep scan that REQUIRES complete directory discovery
    pub async fn deep_scan_with_guaranteed_completeness(&self, user_id: uuid::Uuid, state: &crate::AppState) -> Result<Vec<FileInfo>> {
        info!("🚀 Starting deep scan with guaranteed directory completeness");
        
        let scan_id = uuid::Uuid::new_v4();
        let started_at = chrono::Utc::now();

        // STEP 1: CRITICAL - Ensure complete directory discovery FIRST
        let discovery_report = self.ensure_complete_directory_discovery(user_id, state).await?;
        
        if !discovery_report.is_complete {
            return Err(anyhow!("Cannot proceed with deep scan: Directory discovery incomplete. {} issues found: {:?}", 
                              discovery_report.issues.len(), discovery_report.issues));
        }

        info!("✅ Directory discovery complete - proceeding with file processing");

        // STEP 2: Only now process files, knowing we have complete directory coverage
        let mut all_files = Vec::new();
        for watch_folder in &self.config.watch_folders {
            match self.smart_directory_scan_with_checkpoints(watch_folder, None, user_id, state).await {
                Ok(mut files) => {
                    info!("📁 Processed {} files from {}", files.len(), watch_folder);
                    all_files.append(&mut files);
                }
                Err(e) => {
                    error!("Failed to process files in {}: {}", watch_folder, e);
                    return Err(anyhow!("File processing failed for {}: {}", watch_folder, e));
                }
            }
        }

        // STEP 3: Final verification that nothing was missed
        let final_verification = self.ensure_complete_directory_discovery(user_id, state).await?;
        let is_complete = final_verification.is_complete && final_verification.new_directories_found == 0;
        
        if final_verification.new_directories_found > 0 {
            warn!("⚠️ Found {} additional directories during final verification - scan may need to restart", 
                  final_verification.new_directories_found);
        }

        let completed_at = chrono::Utc::now();
        let duration = completed_at - started_at;

        if is_complete {
            info!("🎉 DEEP SCAN COMPLETE WITH GUARANTEED COMPLETENESS: {} files processed, {} directories tracked in {:.2}s", 
                  all_files.len(),
                  discovery_report.total_directories_discovered,
                  duration.num_milliseconds() as f64 / 1000.0);
        } else {
            warn!("⚠️ Deep scan completed but completeness not guaranteed: {:.2}s", 
                  duration.num_milliseconds() as f64 / 1000.0);
        }

        Ok(all_files)
    }
}

/// Report of complete directory tree discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryDiscoveryReport {
    pub discovery_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub watch_folders_processed: Vec<WatchFolderDiscoveryReport>,
    pub total_directories_discovered: usize,
    pub new_directories_found: usize,
    pub missing_directories_detected: usize,
    pub is_complete: bool,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchFolderDiscoveryReport {
    pub watch_folder: String,
    pub total_directories: usize,
    pub new_directories: usize,
    pub depth_levels_scanned: usize,
    pub is_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteDeepScanReport {
    pub scan_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: chrono::DateTime<chrono::Utc>,
    pub directory_discovery_report: DirectoryDiscoveryReport,
    pub final_verification_report: DirectoryDiscoveryReport,
    pub total_files_processed: usize,
    pub scan_duration_seconds: i64,
    pub is_guaranteed_complete: bool,
}