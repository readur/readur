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
use crate::webdav_xml_parser::parse_propfind_response;

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
    
    /// Check subdirectories individually for changes when parent directory is unchanged
    async fn check_subdirectories_for_changes(&self, parent_path: &str, user_id: uuid::Uuid, state: &crate::AppState) -> Result<Vec<FileInfo>> {
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
            info!("📁 No known subdirectories for {}, no changes to process", parent_path);
            return Ok(Vec::new());
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