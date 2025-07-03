use anyhow::{anyhow, Result};
use reqwest::{Client, Method};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use crate::models::{WebDAVConnectionResult, WebDAVTestConnection};
use super::config::{WebDAVConfig, RetryConfig};

pub struct WebDAVConnection {
    client: Client,
    config: WebDAVConfig,
    retry_config: RetryConfig,
}

impl WebDAVConnection {
    pub fn new(config: WebDAVConfig, retry_config: RetryConfig) -> Result<Self> {
        // Validate configuration first
        config.validate()?;
        let client = Client::builder()
            .timeout(config.timeout())
            .build()?;

        Ok(Self {
            client,
            config,
            retry_config,
        })
    }

    /// Tests WebDAV connection with the provided configuration
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

    /// Tests connection with provided credentials (for configuration testing)
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

        let connection = Self::new(config, RetryConfig::default())?;
        connection.test_connection().await
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
        let url = format!("{}{}", self.config.webdav_url(), path);
        
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

        let response = self.client
            .request(Method::from_bytes(b"PROPFIND")?)
            .url(&url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .header("Depth", "1")
            .header("Content-Type", "application/xml")
            .body(propfind_body)
            .send()
            .await?;

        if response.status().as_u16() == 207 {
            debug!("PROPFIND successful for path: {}", path);
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

    /// Performs authenticated request with retry logic
    pub async fn authenticated_request(
        &self,
        method: Method,
        url: &str,
        body: Option<String>,
        headers: Option<Vec<(&str, &str)>>,
    ) -> Result<reqwest::Response> {
        let mut attempt = 0;
        let mut delay = self.retry_config.initial_delay_ms;

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

            match request.send().await {
                Ok(response) => {
                    let status = response.status();
                    
                    if status.is_success() || status.as_u16() == 207 {
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
                        return Err(anyhow!("Client error: {} - {}", status, 
                            response.text().await.unwrap_or_default()));
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

    /// Gets the WebDAV URL for a specific path
    pub fn get_url_for_path(&self, path: &str) -> String {
        let base_url = self.config.webdav_url();
        let clean_path = path.trim_start_matches('/');
        
        if clean_path.is_empty() {
            base_url
        } else {
            format!("{}/{}", base_url.trim_end_matches('/'), clean_path)
        }
    }
}