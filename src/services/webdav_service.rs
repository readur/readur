use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use reqwest::{Client, Method, Url};
use std::collections::HashSet;
use std::time::Duration;
use tokio::time::sleep;
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
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000, // 1 second
            max_delay_ms: 30000,    // 30 seconds
            backoff_multiplier: 2.0,
            timeout_seconds: 300,   // 5 minutes total timeout for crawl operations
        }
    }
}



#[derive(Clone)]
pub struct WebDAVService {
    client: Client,
    config: WebDAVConfig,
    base_webdav_url: String,
    retry_config: RetryConfig,
}

impl WebDAVService {
    pub fn new(config: WebDAVConfig) -> Result<Self> {
        Self::new_with_retry(config, RetryConfig::default())
    }

    pub fn new_with_retry(config: WebDAVConfig, retry_config: RetryConfig) -> Result<Self> {
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
                info!("🔗 Constructed Nextcloud/ownCloud WebDAV URL: {}", url);
                url
            },
            _ => {
                let url = format!(
                    "{}/webdav",
                    config.server_url.trim_end_matches('/')
                );
                info!("🔗 Constructed generic WebDAV URL: {}", url);
                url
            },
        };

        Ok(Self {
            client,
            config,
            base_webdav_url,
            retry_config,
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

                    warn!("{} failed (attempt {}), retrying in {}ms: {}", 
                        operation_name, attempt, delay, err);
                    
                    sleep(Duration::from_millis(delay)).await;
                    
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
                    .map(|s| s.is_server_error() || s == 429) // 429 = Too Many Requests
                    .unwrap_or(true);
        }
        
        // For other errors, check the error message for common temporary issues
        let error_str = error.to_string().to_lowercase();
        error_str.contains("timeout") 
            || error_str.contains("connection") 
            || error_str.contains("network")
            || error_str.contains("temporary")
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
        
        info!("🔗 Constructed test URL: {}", test_url);

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
            info!("Analyzing folder: {}", folder_path);
            
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
        info!("🔍 Starting optimized discovery for folder: {}", folder_path);
        
        // Check if we should use smart scanning
        let use_smart_scan = match self.config.server_type.as_deref() {
            Some("nextcloud") | Some("owncloud") => {
                info!("🚀 Using smart scanning for Nextcloud/ownCloud server");
                true
            }
            _ => {
                info!("📁 Using traditional scanning for generic WebDAV server");
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
            
            // Use smart scanning with depth-1 traversal
            return self.smart_directory_scan(folder_path, stored_etag.as_deref(), user_id, state).await;
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
                    info!("✅ Directory {} unchanged (ETag: {}), checking subdirectories individually", folder_path, current_dir_etag);
                    
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
                    info!("🔄 Directory {} changed (old ETag: {}, new ETag: {}), performing deep scan", 
                        folder_path, stored_dir.directory_etag, current_dir_etag);
                }
            }
            Ok(None) => {
                info!("🆕 New directory {}, performing initial scan", folder_path);
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
            info!("📊 Updated directory tracking: {} files, {} bytes, ETag: {}", 
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
        
        info!("🗂️ Found {} unique directories at all levels", all_directories.len());
        
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
        
        info!("✅ Completed tracking {} directories at all depth levels", all_directories.len());
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
        info!("🎯 Starting targeted re-scan for {} specific paths", paths_to_scan.len());
        
        let mut all_files = Vec::new();
        
        for path in paths_to_scan {
            info!("🔍 Targeted scan of: {}", path);
            
            // Check if this specific path has changed
            match self.check_directory_etag(path).await {
                Ok(current_etag) => {
                    // Check cached ETag
                    let needs_scan = match state.db.get_webdav_directory(user_id, path).await {
                        Ok(Some(stored_dir)) => {
                            if stored_dir.directory_etag != current_etag {
                                info!("🔄 Path {} changed (old: {}, new: {})", path, stored_dir.directory_etag, current_etag);
                                true
                            } else {
                                debug!("✅ Path {} unchanged (ETag: {})", path, current_etag);
                                false
                            }
                        }
                        Ok(None) => {
                            info!("🆕 New path {} detected", path);
                            true
                        }
                        Err(e) => {
                            warn!("Database error for path {}: {}", path, e);
                            true // Scan on error to be safe
                        }
                    };
                    
                    if needs_scan {
                        // Use shallow scan for this specific directory only
                        match self.discover_files_in_folder_shallow(path).await {
                            Ok(mut path_files) => {
                                info!("📂 Found {} files in changed path {}", path_files.len(), path);
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
        
        info!("🎯 Targeted re-scan completed: {} total files found", all_files.len());
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
                info!("📊 Updated single directory tracking: {} ({} files, {} bytes, ETag: {})", 
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
                
                info!("🕒 Found {} directories not scanned in last {} hours", stale_dirs.len(), max_age_hours);
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
        info!("🧠 Starting smart sync for {} watch folders", watch_folders.len());
        
        let mut all_files = Vec::new();
        
        for folder_path in watch_folders {
            info!("🔍 Smart sync processing folder: {}", folder_path);
            
            // Step 1: Try optimized discovery first (checks directory ETag)
            let optimized_result = self.discover_files_in_folder_optimized(folder_path, user_id, state).await;
            
            match optimized_result {
                Ok(files) => {
                    if !files.is_empty() {
                        info!("✅ Optimized discovery found {} files in {}", files.len(), folder_path);
                        all_files.extend(files);
                    } else {
                        info!("🔍 Directory {} unchanged, checking for stale subdirectories", folder_path);
                        
                        // Step 2: Check for stale subdirectories that need targeted scanning
                        let stale_dirs = self.get_stale_subdirectories(folder_path, user_id, state, 24).await?;
                        
                        if !stale_dirs.is_empty() {
                            info!("🎯 Found {} stale subdirectories, performing targeted scan", stale_dirs.len());
                            let targeted_files = self.discover_files_targeted_rescan(&stale_dirs, user_id, state).await?;
                            all_files.extend(targeted_files);
                        } else {
                            info!("✅ All subdirectories of {} are fresh, no scan needed", folder_path);
                        }
                    }
                }
                Err(e) => {
                    warn!("Optimized discovery failed for {}, falling back to full scan: {}", folder_path, e);
                    // Fallback to traditional full scan
                    match self.discover_files_in_folder(folder_path).await {
                        Ok(files) => {
                            info!("📂 Fallback scan found {} files in {}", files.len(), folder_path);
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
        
        info!("🧠 Smart sync completed: {} total files discovered", all_files.len());
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
        info!("⚡ Starting incremental sync for {} watch folders", watch_folders.len());
        
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
                                info!("🔄 Directory {} changed (ETag: {} → {})", folder_path, stored_dir.directory_etag, current_etag);
                                changed_count += 1;
                                true
                            } else {
                                debug!("✅ Directory {} unchanged (ETag: {})", folder_path, current_etag);
                                unchanged_count += 1;
                                false
                            }
                        }
                        Ok(None) => {
                            info!("🆕 New directory {} detected", folder_path);
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
                                info!("📂 Incremental scan found {} files in changed directory {}", files.len(), folder_path);
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
        
        info!("⚡ Incremental sync completed: {} unchanged, {} changed, {} total files found", 
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
            info!("🚀 Server supports recursive ETags - parent {} unchanged means all contents unchanged", parent_path);
            return Ok(Vec::new());
        }
        
        // For servers without recursive ETags, fall back to checking each subdirectory
        info!("📁 Server doesn't support recursive ETags, checking subdirectories individually");
        
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
            info!("📁 No known subdirectories for {}, performing initial scan to discover structure", parent_path);
            return self.discover_files_in_folder_impl(parent_path).await;
        }
        
        info!("🔍 Checking {} known subdirectories for changes", subdirectories.len());
        
        let mut changed_files = Vec::new();
        let subdirectory_count = subdirectories.len();
        
        // Check each subdirectory individually
        for subdir in subdirectories {
            let subdir_path = &subdir.directory_path;
            
            // Check if this subdirectory has changed
            match self.check_directory_etag(subdir_path).await {
                Ok(current_etag) => {
                    if current_etag != subdir.directory_etag {
                        info!("🔄 Subdirectory {} changed (old: {}, new: {}), scanning recursively", 
                            subdir_path, subdir.directory_etag, current_etag);
                        
                        // This subdirectory changed - get all its files recursively
                        match self.discover_files_in_folder_impl(subdir_path).await {
                            Ok(mut subdir_files) => {
                                info!("📂 Found {} files in changed subdirectory {}", subdir_files.len(), subdir_path);
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
        
        info!("🎯 Found {} changed files across {} subdirectories", changed_files.len(), subdirectory_count);
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
        info!("🔬 Testing recursive ETag support using existing directory structure");
        
        // Find a directory with subdirectories from our watch folders
        for watch_folder in &self.config.watch_folders {
            // Get the directory structure with depth 1
            match self.discover_files_in_folder_shallow(watch_folder).await {
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
                    info!("Testing with directory: {} and subdirectory: {}", watch_folder, test_subdir.path);
                    
                    // Step 1: Get parent directory ETag
                    let parent_etag = self.check_directory_etag(watch_folder).await?;
                    
                    // Step 2: Get subdirectory ETag  
                    let subdir_etag = self.check_directory_etag(&test_subdir.path).await?;
                    
                    // Step 3: Check if parent has a different ETag than child
                    // In a recursive ETag system, they should be different but related
                    // The key test is: if we check the parent again after some time,
                    // and a file deep inside changed, did the parent ETag change?
                    
                    // For now, we'll just check if the server provides ETags at all
                    if !parent_etag.is_empty() && !subdir_etag.is_empty() {
                        info!("✅ Server provides ETags for directories");
                        info!("   Parent ETag: {}", parent_etag);
                        info!("   Subdir ETag: {}", subdir_etag);
                        
                        // Without write access, we can't definitively test recursive propagation
                        // But we can make an educated guess based on the server type
                        let likely_supports_recursive = match self.config.server_type.as_deref() {
                            Some("nextcloud") | Some("owncloud") => {
                                info!("   Nextcloud/ownCloud servers typically support recursive ETags");
                                true
                            }
                            _ => {
                                info!("   Unknown server type - recursive ETag support uncertain");
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
        
        info!("❓ Could not determine recursive ETag support - no suitable directories found");
        Ok(false)
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
        info!("🧠 Smart scan starting for path: {}", path);
        
        // Step 1: Check current directory ETag
        let current_etag = match self.check_directory_etag(path).await {
            Ok(etag) => etag,
            Err(e) => {
                warn!("Failed to get directory ETag for {}, falling back to full scan: {}", path, e);
                return self.discover_files_in_folder_impl(path).await;
            }
        };
        
        // Step 2: If unchanged and we support recursive ETags, nothing to do
        if known_etag == Some(&current_etag) {
            let supports_recursive = match self.config.server_type.as_deref() {
                Some("nextcloud") | Some("owncloud") => true,
                _ => false
            };
            
            if supports_recursive {
                info!("✅ Directory {} unchanged (recursive ETag: {}), skipping scan", path, current_etag);
                return Ok(Vec::new());
            } else {
                info!("📁 Directory {} ETag unchanged but server doesn't support recursive ETags, checking subdirectories", path);
            }
        } else {
            info!("🔄 Directory {} changed (old: {:?}, new: {})", path, known_etag, current_etag);
        }
        
        // Step 3: Directory changed or we need to check subdirectories - do depth-1 scan
        let entries = match self.discover_files_in_folder_shallow(path).await {
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
        
        // Update tracking for this directory
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
        
        // Step 4: For each subdirectory, check if it needs scanning
        for subdir in subdirs_to_scan {
            // Get stored ETag for this subdirectory
            let stored_etag = match state.db.get_webdav_directory(user_id, &subdir.path).await {
                Ok(Some(dir)) => Some(dir.directory_etag),
                Ok(None) => {
                    info!("🆕 New subdirectory discovered: {}", subdir.path);
                    None
                }
                Err(e) => {
                    warn!("Database error checking subdirectory {}: {}", subdir.path, e);
                    None
                }
            };
            
            // If ETag changed or new directory, scan it recursively
            if stored_etag.as_deref() != Some(&subdir.etag) {
                info!("🔄 Subdirectory {} needs scanning (old: {:?}, new: {})", 
                    subdir.path, stored_etag, subdir.etag);
                    
                match self.smart_directory_scan(&subdir.path, stored_etag.as_deref(), user_id, state).await {
                    Ok(mut subdir_files) => {
                        info!("📂 Found {} entries in subdirectory {}", subdir_files.len(), subdir.path);
                        all_files.append(&mut subdir_files);
                    }
                    Err(e) => {
                        error!("Failed to scan subdirectory {}: {}", subdir.path, e);
                        // Continue with other subdirectories
                    }
                }
            } else {
                debug!("✅ Subdirectory {} unchanged (ETag: {})", subdir.path, subdir.etag);
                // Update last_scanned_at
                let update = crate::models::UpdateWebDAVDirectory {
                    directory_etag: subdir.etag.clone(),
                    last_scanned_at: chrono::Utc::now(),
                    file_count: 0, // Will be preserved by database
                    total_size_bytes: 0,
                };
                
                if let Err(e) = state.db.update_webdav_directory(user_id, &subdir.path, &update).await {
                    warn!("Failed to update scan time for {}: {}", subdir.path, e);
                }
            }
        }
        
        info!("🧠 Smart scan completed for {}: {} total entries found", path, all_files.len());
        Ok(all_files)
        })
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