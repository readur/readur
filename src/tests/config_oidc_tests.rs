#[cfg(test)]
mod tests {
    use crate::config::Config;
    use std::env;

    fn create_base_config() -> Config {
        Config {
            database_url: "postgresql://test:test@localhost/test".to_string(),
            server_address: "127.0.0.1:8000".to_string(),
            jwt_secret: "test-secret".to_string(),
            upload_path: "./test-uploads".to_string(),
            watch_folder: "./test-watch".to_string(),
            allowed_file_types: vec!["pdf".to_string()],
            watch_interval_seconds: Some(30),
            file_stability_check_ms: Some(500),
            max_file_age_hours: None,
            ocr_language: "eng".to_string(),
            concurrent_ocr_jobs: 2,
            ocr_timeout_seconds: 60,
            max_file_size_mb: 10,
            memory_limit_mb: 256,
            cpu_priority: "normal".to_string(),
            oidc_enabled: false,
            oidc_client_id: None,
            oidc_client_secret: None,
            oidc_issuer_url: None,
            oidc_redirect_uri: None,
        }
    }

    #[test]
    fn test_oidc_disabled_by_default() {
        let config = create_base_config();
        assert!(!config.oidc_enabled);
        assert!(config.oidc_client_id.is_none());
        assert!(config.oidc_client_secret.is_none());
        assert!(config.oidc_issuer_url.is_none());
        assert!(config.oidc_redirect_uri.is_none());
    }

    #[test]
    fn test_oidc_enabled_from_env() {
        // Clean up environment first to ensure test isolation
        env::remove_var("OIDC_ENABLED");
        env::remove_var("OIDC_CLIENT_ID");
        env::remove_var("OIDC_CLIENT_SECRET");
        env::remove_var("OIDC_ISSUER_URL");
        env::remove_var("OIDC_REDIRECT_URI");
        env::remove_var("DATABASE_URL");
        env::remove_var("JWT_SECRET");
        
        env::set_var("OIDC_ENABLED", "true");
        env::set_var("OIDC_CLIENT_ID", "test-client-id");
        env::set_var("OIDC_CLIENT_SECRET", "test-client-secret");
        env::set_var("OIDC_ISSUER_URL", "https://provider.example.com");
        env::set_var("OIDC_REDIRECT_URI", "http://localhost:8000/auth/oidc/callback");
        env::set_var("DATABASE_URL", "postgresql://test:test@localhost/test");
        env::set_var("JWT_SECRET", "test-secret");

        let config = Config::from_env().unwrap();

        assert!(config.oidc_enabled);
        assert_eq!(config.oidc_client_id, Some("test-client-id".to_string()));
        assert_eq!(config.oidc_client_secret, Some("test-client-secret".to_string()));
        assert_eq!(config.oidc_issuer_url, Some("https://provider.example.com".to_string()));
        assert_eq!(config.oidc_redirect_uri, Some("http://localhost:8000/auth/oidc/callback".to_string()));

        // Clean up
        env::remove_var("OIDC_ENABLED");
        env::remove_var("OIDC_CLIENT_ID");
        env::remove_var("OIDC_CLIENT_SECRET");
        env::remove_var("OIDC_ISSUER_URL");
        env::remove_var("OIDC_REDIRECT_URI");
        env::remove_var("DATABASE_URL");
        env::remove_var("JWT_SECRET");
    }

    #[test]
    fn test_oidc_enabled_variations() {
        let test_cases = vec![
            ("true", true),
            ("TRUE", true),
            ("1", true),
            ("yes", true),
            ("YES", true),
            ("on", true),
            ("ON", true),
            ("false", false),
            ("FALSE", false),
            ("0", false),
            ("no", false),
            ("NO", false),
            ("off", false),
            ("OFF", false),
            ("invalid", false),
        ];

        for (value, expected) in test_cases {
            // Clean up environment first for each iteration
            env::remove_var("OIDC_ENABLED");
            env::remove_var("OIDC_CLIENT_ID");
            env::remove_var("OIDC_CLIENT_SECRET");
            env::remove_var("OIDC_ISSUER_URL");
            env::remove_var("OIDC_REDIRECT_URI");
            env::remove_var("DATABASE_URL");
            env::remove_var("JWT_SECRET");
            
            env::set_var("OIDC_ENABLED", value);
            env::set_var("DATABASE_URL", "postgresql://test:test@localhost/test");
            env::set_var("JWT_SECRET", "test-secret");

            let config = Config::from_env().unwrap();
            assert_eq!(config.oidc_enabled, expected, "Failed for value: {}", value);

            env::remove_var("OIDC_ENABLED");
            env::remove_var("DATABASE_URL");
            env::remove_var("JWT_SECRET");
        }
    }

    #[test]
    fn test_oidc_partial_config() {
        // Only set some OIDC vars
        env::set_var("OIDC_ENABLED", "true");
        env::set_var("OIDC_CLIENT_ID", "test-client-id");
        // Missing OIDC_CLIENT_SECRET, OIDC_ISSUER_URL, OIDC_REDIRECT_URI
        env::set_var("DATABASE_URL", "postgresql://test:test@localhost/test");
        env::set_var("JWT_SECRET", "test-secret");

        let config = Config::from_env().unwrap();

        assert!(config.oidc_enabled);
        assert_eq!(config.oidc_client_id, Some("test-client-id".to_string()));
        assert!(config.oidc_client_secret.is_none());
        assert!(config.oidc_issuer_url.is_none());
        assert!(config.oidc_redirect_uri.is_none());

        // Clean up
        env::remove_var("OIDC_ENABLED");
        env::remove_var("OIDC_CLIENT_ID");
        env::remove_var("DATABASE_URL");
        env::remove_var("JWT_SECRET");
    }

    #[test]
    fn test_oidc_disabled_with_config_present() {
        // OIDC disabled but config present
        env::set_var("OIDC_ENABLED", "false");
        env::set_var("OIDC_CLIENT_ID", "test-client-id");
        env::set_var("OIDC_CLIENT_SECRET", "test-client-secret");
        env::set_var("OIDC_ISSUER_URL", "https://provider.example.com");
        env::set_var("OIDC_REDIRECT_URI", "http://localhost:8000/auth/oidc/callback");
        env::set_var("DATABASE_URL", "postgresql://test:test@localhost/test");
        env::set_var("JWT_SECRET", "test-secret");

        let config = Config::from_env().unwrap();

        assert!(!config.oidc_enabled);
        assert_eq!(config.oidc_client_id, Some("test-client-id".to_string()));
        assert_eq!(config.oidc_client_secret, Some("test-client-secret".to_string()));
        assert_eq!(config.oidc_issuer_url, Some("https://provider.example.com".to_string()));
        assert_eq!(config.oidc_redirect_uri, Some("http://localhost:8000/auth/oidc/callback".to_string()));

        // Clean up
        env::remove_var("OIDC_ENABLED");
        env::remove_var("OIDC_CLIENT_ID");
        env::remove_var("OIDC_CLIENT_SECRET");
        env::remove_var("OIDC_ISSUER_URL");
        env::remove_var("OIDC_REDIRECT_URI");
        env::remove_var("DATABASE_URL");
        env::remove_var("JWT_SECRET");
    }

    #[test]
    fn test_oidc_empty_values() {
        env::set_var("OIDC_ENABLED", "true");
        env::set_var("OIDC_CLIENT_ID", "");
        env::set_var("OIDC_CLIENT_SECRET", "");
        env::set_var("OIDC_ISSUER_URL", "");
        env::set_var("OIDC_REDIRECT_URI", "");
        env::set_var("DATABASE_URL", "postgresql://test:test@localhost/test");
        env::set_var("JWT_SECRET", "test-secret");

        let config = Config::from_env().unwrap();

        assert!(config.oidc_enabled);
        // Empty string values should be converted to Some(empty_string)
        assert_eq!(config.oidc_client_id, Some("".to_string()));
        assert_eq!(config.oidc_client_secret, Some("".to_string()));
        assert_eq!(config.oidc_issuer_url, Some("".to_string()));
        assert_eq!(config.oidc_redirect_uri, Some("".to_string()));

        // Clean up
        env::remove_var("OIDC_ENABLED");
        env::remove_var("OIDC_CLIENT_ID");
        env::remove_var("OIDC_CLIENT_SECRET");
        env::remove_var("OIDC_ISSUER_URL");
        env::remove_var("OIDC_REDIRECT_URI");
        env::remove_var("DATABASE_URL");
        env::remove_var("JWT_SECRET");
    }

    #[test]
    fn test_oidc_config_validation_output() {
        // Test that validation warnings are properly formatted
        env::set_var("OIDC_ENABLED", "true");
        env::set_var("DATABASE_URL", "postgresql://test:test@localhost/test");
        env::set_var("JWT_SECRET", "test-secret");
        // Missing required OIDC fields

        // This should succeed but show warnings
        let config = Config::from_env().unwrap();
        assert!(config.oidc_enabled);
        assert!(config.oidc_client_id.is_none());

        // Clean up
        env::remove_var("OIDC_ENABLED");
        env::remove_var("DATABASE_URL");
        env::remove_var("JWT_SECRET");
    }

    #[test]
    fn test_oidc_complete_configuration() {
        env::set_var("OIDC_ENABLED", "true");
        env::set_var("OIDC_CLIENT_ID", "my-app-client-id");
        env::set_var("OIDC_CLIENT_SECRET", "super-secret-client-secret");
        env::set_var("OIDC_ISSUER_URL", "https://auth.example.com");
        env::set_var("OIDC_REDIRECT_URI", "https://myapp.com/auth/callback");
        env::set_var("DATABASE_URL", "postgresql://test:test@localhost/test");
        env::set_var("JWT_SECRET", "test-secret");

        let config = Config::from_env().unwrap();

        assert!(config.oidc_enabled);
        assert_eq!(config.oidc_client_id.unwrap(), "my-app-client-id");
        assert_eq!(config.oidc_client_secret.unwrap(), "super-secret-client-secret");
        assert_eq!(config.oidc_issuer_url.unwrap(), "https://auth.example.com");
        assert_eq!(config.oidc_redirect_uri.unwrap(), "https://myapp.com/auth/callback");

        // Clean up
        env::remove_var("OIDC_ENABLED");
        env::remove_var("OIDC_CLIENT_ID");
        env::remove_var("OIDC_CLIENT_SECRET");
        env::remove_var("OIDC_ISSUER_URL");
        env::remove_var("OIDC_REDIRECT_URI");
        env::remove_var("DATABASE_URL");
        env::remove_var("JWT_SECRET");
    }

    #[test] 
    fn test_oidc_config_precedence() {
        // Clean up any existing env vars first
        env::remove_var("OIDC_ENABLED");
        env::remove_var("OIDC_CLIENT_ID");
        env::remove_var("OIDC_CLIENT_SECRET");
        env::remove_var("OIDC_ISSUER_URL");
        env::remove_var("OIDC_REDIRECT_URI");
        env::remove_var("DATABASE_URL");
        env::remove_var("JWT_SECRET");
        
        // Test that environment variables take precedence
        env::set_var("OIDC_ENABLED", "true");
        env::set_var("OIDC_CLIENT_ID", "env-client-id");
        env::set_var("DATABASE_URL", "postgresql://test:test@localhost/test");
        env::set_var("JWT_SECRET", "test-secret");

        let config = Config::from_env().unwrap();

        assert!(config.oidc_enabled);
        assert_eq!(config.oidc_client_id.unwrap(), "env-client-id");

        // Clean up
        env::remove_var("OIDC_ENABLED");
        env::remove_var("OIDC_CLIENT_ID");
        env::remove_var("DATABASE_URL");
        env::remove_var("JWT_SECRET");
    }
}