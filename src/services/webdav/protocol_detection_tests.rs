#[cfg(test)]
mod tests {
    use super::super::{WebDAVService, WebDAVConfig};

    /// Helper function to create test WebDAV config without protocol
    fn create_test_config_without_protocol() -> WebDAVConfig {
        WebDAVConfig {
            server_url: "nas.example.com".to_string(), // No protocol
            username: "testuser".to_string(),
            password: "testpass".to_string(),
            watch_folders: vec!["/Documents".to_string()],
            file_extensions: vec!["pdf".to_string(), "txt".to_string()],
            timeout_seconds: 30,
            server_type: Some("nextcloud".to_string()),
        }
    }

    /// Helper function to create test WebDAV config with HTTPS protocol
    fn create_test_config_with_https() -> WebDAVConfig {
        WebDAVConfig {
            server_url: "https://nas.example.com".to_string(),
            username: "testuser".to_string(),
            password: "testpass".to_string(),
            watch_folders: vec!["/Documents".to_string()],
            file_extensions: vec!["pdf".to_string(), "txt".to_string()],
            timeout_seconds: 30,
            server_type: Some("nextcloud".to_string()),
        }
    }

    /// Helper function to create test WebDAV config with HTTP protocol
    fn create_test_config_with_http() -> WebDAVConfig {
        WebDAVConfig {
            server_url: "http://nas.example.com".to_string(),
            username: "testuser".to_string(),
            password: "testpass".to_string(),
            watch_folders: vec!["/Documents".to_string()],
            file_extensions: vec!["pdf".to_string(), "txt".to_string()],
            timeout_seconds: 30,
            server_type: Some("nextcloud".to_string()),
        }
    }

    #[tokio::test]
    async fn test_config_validation_accepts_url_without_protocol() {
        let config = create_test_config_without_protocol();
        
        // Should not fail validation
        assert!(config.validate().is_ok());
    }

    #[tokio::test]
    async fn test_config_validation_accepts_url_with_https() {
        let config = create_test_config_with_https();
        
        // Should not fail validation
        assert!(config.validate().is_ok());
    }

    #[tokio::test]
    async fn test_config_validation_accepts_url_with_http() {
        let config = create_test_config_with_http();
        
        // Should not fail validation
        assert!(config.validate().is_ok());
    }

    #[tokio::test]
    async fn test_normalize_server_url_adds_https_by_default() {
        let normalized = WebDAVConfig::normalize_server_url("nas.example.com");
        assert_eq!(normalized, "https://nas.example.com");
    }

    #[tokio::test]
    async fn test_normalize_server_url_preserves_existing_protocol() {
        let https_url = WebDAVConfig::normalize_server_url("https://nas.example.com");
        assert_eq!(https_url, "https://nas.example.com");
        
        let http_url = WebDAVConfig::normalize_server_url("http://nas.example.com");
        assert_eq!(http_url, "http://nas.example.com");
    }

    #[tokio::test]
    async fn test_get_alternative_protocol_url() {
        // HTTPS to HTTP
        let alt_http = WebDAVConfig::get_alternative_protocol_url("https://nas.example.com");
        assert_eq!(alt_http, Some("http://nas.example.com".to_string()));
        
        // HTTP to HTTPS
        let alt_https = WebDAVConfig::get_alternative_protocol_url("http://nas.example.com");
        assert_eq!(alt_https, Some("https://nas.example.com".to_string()));
        
        // No protocol - should return None
        let no_protocol = WebDAVConfig::get_alternative_protocol_url("nas.example.com");
        assert_eq!(no_protocol, None);
    }

    #[tokio::test]
    async fn test_webdav_url_uses_normalized_url() {
        let config = create_test_config_without_protocol();
        let webdav_url = config.webdav_url();
        
        // Should start with https:// (normalized)
        assert!(webdav_url.starts_with("https://"));
        assert_eq!(webdav_url, "https://nas.example.com/remote.php/dav/files/testuser");
    }

    #[tokio::test]
    async fn test_service_creation_with_protocol_detection() {
        let config = create_test_config_without_protocol();
        
        // Should be able to create service without errors
        let service = WebDAVService::new(config);
        assert!(service.is_ok());
    }

    #[tokio::test]
    async fn test_effective_server_url_defaults_to_normalized() {
        let config = create_test_config_without_protocol();
        let service = WebDAVService::new(config).unwrap();
        
        let effective_url = service.get_effective_server_url();
        assert_eq!(effective_url, "https://nas.example.com");
    }

    #[tokio::test]
    async fn test_effective_server_url_with_existing_protocol() {
        let config = create_test_config_with_http();
        let service = WebDAVService::new(config).unwrap();
        
        let effective_url = service.get_effective_server_url();
        assert_eq!(effective_url, "http://nas.example.com");
    }

    #[tokio::test]
    async fn test_working_protocol_initially_none() {
        let config = create_test_config_without_protocol();
        let service = WebDAVService::new(config).unwrap();
        
        // Initially, no working protocol should be detected
        assert!(service.get_working_protocol().is_none());
    }

    #[tokio::test]
    async fn test_is_connection_error_detection() {
        let config = create_test_config_without_protocol();
        let service = WebDAVService::new(config).unwrap();
        
        // Test various connection error patterns
        let connection_errors = vec![
            anyhow::anyhow!("connection refused"),
            anyhow::anyhow!("timeout occurred"),
            anyhow::anyhow!("DNS resolution failed"),
            anyhow::anyhow!("TLS handshake failed"),
            anyhow::anyhow!("SSL certificate error"),
        ];
        
        for error in connection_errors {
            assert!(service.is_connection_error(&error), "Should detect '{}' as connection error", error);
        }
        
        // Test non-connection errors
        let non_connection_errors = vec![
            anyhow::anyhow!("401 Unauthorized"),
            anyhow::anyhow!("403 Forbidden"),
            anyhow::anyhow!("invalid credentials"),
        ];
        
        for error in non_connection_errors {
            assert!(!service.is_connection_error(&error), "Should NOT detect '{}' as connection error", error);
        }
    }

    #[tokio::test]
    async fn test_config_validation_rejects_empty_url() {
        let mut config = create_test_config_without_protocol();
        config.server_url = "".to_string();
        
        assert!(config.validate().is_err());
    }

    #[tokio::test]
    async fn test_config_validation_rejects_invalid_url() {
        let mut config = create_test_config_without_protocol();
        config.server_url = "http://https://invalid".to_string();
        
        assert!(config.validate().is_err());
    }

    #[tokio::test]
    async fn test_webdav_fallback_urls_use_normalized_url() {
        let config = create_test_config_without_protocol();
        let fallback_urls = config.webdav_fallback_urls();
        
        // All fallback URLs should start with https:// (normalized)
        for url in fallback_urls {
            assert!(url.starts_with("https://"), "Fallback URL should be normalized: {}", url);
        }
    }

    #[tokio::test]
    async fn test_backward_compatibility_with_existing_protocols() {
        // Existing URLs with protocols should work unchanged
        let https_config = create_test_config_with_https();
        let http_config = create_test_config_with_http();
        
        let https_service = WebDAVService::new(https_config).unwrap();
        let http_service = WebDAVService::new(http_config).unwrap();
        
        assert_eq!(https_service.get_effective_server_url(), "https://nas.example.com");
        assert_eq!(http_service.get_effective_server_url(), "http://nas.example.com");
    }

    #[tokio::test]
    async fn test_url_construction_with_protocol_detection() {
        let config = create_test_config_without_protocol();
        let service = WebDAVService::new(config).unwrap();
        
        // Test URL construction for different paths
        let test_paths = vec![
            "/Documents/file.pdf",
            "Photos/image.jpg",
            "/",
            "",
        ];
        
        for path in test_paths {
            let url = service.get_url_for_path(path);
            // Should start with https:// (normalized default)
            assert!(url.starts_with("https://"), "URL should be normalized for path '{}': {}", path, url);
        }
    }
}