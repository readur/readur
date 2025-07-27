use anyhow::Result;
use crate::models::FileIngestionInfo;
use super::config::WebDAVConfig;

/// Centralized URL and path management for WebDAV operations
/// 
/// This module handles all the messy WebDAV URL construction, path normalization,
/// and conversion between full WebDAV paths and relative paths. It's designed to
/// prevent the URL doubling issues that plague WebDAV integrations.
pub struct WebDAVUrlManager {
    config: WebDAVConfig,
}

impl WebDAVUrlManager {
    pub fn new(config: WebDAVConfig) -> Self {
        Self { config }
    }

    /// Get the base WebDAV URL for the configured server
    /// Returns something like: "https://nas.example.com/remote.php/dav/files/username"
    pub fn base_url(&self) -> String {
        self.config.webdav_url()
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
            _ => href.to_string(),
        }
    }

    /// Convert relative path to full URL for WebDAV requests
    /// 
    /// Input:  "/Photos/image.jpg"
    /// Output: "https://nas.example.com/remote.php/dav/files/username/Photos/image.jpg"
    pub fn relative_path_to_url(&self, relative_path: &str) -> String {
        let base_url = self.base_url();
        let clean_path = relative_path.trim_start_matches('/');
        
        if clean_path.is_empty() {
            base_url
        } else {
            let normalized_base = base_url.trim_end_matches('/');
            format!("{}/{}", normalized_base, clean_path)
        }
    }

    /// Process FileIngestionInfo from XML parser to set correct paths
    /// 
    /// This takes the raw XML parser output and fixes the path fields:
    /// - Sets relative_path from href conversion
    /// - Keeps full_path as the original href
    /// - Sets legacy path field for backward compatibility
    pub fn process_file_info(&self, mut file_info: FileIngestionInfo) -> FileIngestionInfo {
        // The XML parser puts the href in full_path (which is correct)
        let href = &file_info.full_path;
        
        // Convert to relative path
        file_info.relative_path = self.href_to_relative_path(href);
        
        // Legacy path field should be relative for backward compatibility
        #[allow(deprecated)]
        {
            file_info.path = file_info.relative_path.clone();
        }
        
        file_info
    }

    /// Process a collection of FileIngestionInfo items
    pub fn process_file_infos(&self, file_infos: Vec<FileIngestionInfo>) -> Vec<FileIngestionInfo> {
        file_infos.into_iter()
            .map(|file_info| self.process_file_info(file_info))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_nextcloud_config() -> WebDAVConfig {
        WebDAVConfig {
            server_url: "https://nas.example.com".to_string(),
            username: "testuser".to_string(),
            password: "password".to_string(),
            watch_folders: vec!["/Photos".to_string()],
            file_extensions: vec!["jpg".to_string(), "pdf".to_string()],
            timeout_seconds: 30,
            server_type: Some("nextcloud".to_string()),
        }
    }

    #[test]
    fn test_nextcloud_href_to_relative_path() {
        let manager = WebDAVUrlManager::new(create_nextcloud_config());
        
        // Test file path conversion
        let href = "/remote.php/dav/files/testuser/Photos/image.jpg";
        let relative = manager.href_to_relative_path(href);
        assert_eq!(relative, "/Photos/image.jpg");
        
        // Test directory path conversion
        let href = "/remote.php/dav/files/testuser/Photos/";
        let relative = manager.href_to_relative_path(href);
        assert_eq!(relative, "/Photos/");
        
        // Test root path
        let href = "/remote.php/dav/files/testuser";
        let relative = manager.href_to_relative_path(href);
        assert_eq!(relative, "/");
    }

    #[test]
    fn test_relative_path_to_url() {
        let manager = WebDAVUrlManager::new(create_nextcloud_config());
        
        // Test file URL construction
        let relative = "/Photos/image.jpg";
        let url = manager.relative_path_to_url(relative);
        assert_eq!(url, "https://nas.example.com/remote.php/dav/files/testuser/Photos/image.jpg");
        
        // Test root URL
        let relative = "/";
        let url = manager.relative_path_to_url(relative);
        assert_eq!(url, "https://nas.example.com/remote.php/dav/files/testuser");
    }

    #[test]
    fn test_process_file_info() {
        let manager = WebDAVUrlManager::new(create_nextcloud_config());
        
        let file_info = FileIngestionInfo {
            relative_path: "TEMP".to_string(), // Will be overwritten
            full_path: "/remote.php/dav/files/testuser/Photos/image.jpg".to_string(),
            #[allow(deprecated)]
            path: "OLD".to_string(), // Will be overwritten
            name: "image.jpg".to_string(),
            size: 1024,
            mime_type: "image/jpeg".to_string(),
            last_modified: None,
            etag: "abc123".to_string(),
            is_directory: false,
            created_at: None,
            permissions: None,
            owner: None,
            group: None,
            metadata: None,
        };
        
        let processed = manager.process_file_info(file_info);
        
        assert_eq!(processed.relative_path, "/Photos/image.jpg");
        assert_eq!(processed.full_path, "/remote.php/dav/files/testuser/Photos/image.jpg");
        #[allow(deprecated)]
        assert_eq!(processed.path, "/Photos/image.jpg");
    }
}