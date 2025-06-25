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

        let response = self.client
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

    async fn discover_files_in_folder_impl(&self, folder_path: &str) -> Result<Vec<FileInfo>> {
        let folder_url = format!("{}{}", self.base_webdav_url, folder_path);
        
        debug!("Discovering files in: {}", folder_url);

        let propfind_body = r#"<?xml version="1.0"?>
            <d:propfind xmlns:d="DAV:">
                <d:prop>
                    <d:displayname/>
                    <d:getcontentlength/>
                    <d:getlastmodified/>
                    <d:getcontenttype/>
                    <d:getetag/>
                    <d:resourcetype/>
                </d:prop>
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