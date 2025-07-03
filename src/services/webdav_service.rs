use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use reqwest::{Client, Method, Url};
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