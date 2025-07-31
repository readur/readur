#[cfg(test)]
mod tests {
    use readur::config::Config;
    use std::env;
    use std::sync::Mutex;
    
    // Mutex to ensure OIDC tests run sequentially to avoid race conditions
    static OIDC_TEST_MUTEX: Mutex<()> = Mutex::new(());
    
    // Helper function to safely run a test with environment isolation
    fn run_with_env_isolation<F, R>(test_fn: F) -> R 
    where 
        F: FnOnce() -> R,
    {
        let _guard = OIDC_TEST_MUTEX.lock().unwrap();
        
        // Store original environment values
        let original_values: Vec<(String, Option<String>)> = vec![
            "OIDC_ENABLED",
            "OIDC_CLIENT_ID", 
            "OIDC_CLIENT_SECRET",
            "OIDC_ISSUER_URL",
            "OIDC_REDIRECT_URI",
            "DATABASE_URL",
            "JWT_SECRET",
        ].into_iter().map(|key| {
            (key.to_string(), env::var(key).ok())
        }).collect();
        
        // Clean up environment first
        for (key, _) in &original_values {
            env::remove_var(key);
        }
        
        // Run the test
        let result = test_fn();
        
        // Restore original environment
        for (key, original_value) in original_values {
            env::remove_var(&key);
            if let Some(value) = original_value {
                env::set_var(&key, value);
            }
        }
        
        result
    }

    fn create_base_config() -> Config {
        Config {
            database_url: "postgresql://test:test@localhost/test".to_string(),
            server_address: "127.0.0.1:8000".to_string(),
            jwt_secret: "test-secret".to_string(),
            upload_path: "./test-uploads".to_string(),
            watch_folder: "./test-watch".to_string(),
        user_watch_base_dir: "./user_watch".to_string(),
        enable_per_user_watch: false,            allowed_file_types: vec!["pdf".to_string()],
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
        run_with_env_isolation(|| {
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
        });
    }

    #[test]
    fn test_oidc_enabled_variations() {
        run_with_env_isolation(|| {
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
                // Clean up environment for each iteration
                env::remove_var("OIDC_ENABLED");
                env::remove_var("DATABASE_URL");
                env::remove_var("JWT_SECRET");
                
                env::set_var("OIDC_ENABLED", value);
                env::set_var("DATABASE_URL", "postgresql://test:test@localhost/test");
                env::set_var("JWT_SECRET", "test-secret");

                let config = Config::from_env().unwrap();
                assert_eq!(config.oidc_enabled, expected, "Failed for value: {}", value);
            }
        });
    }

    #[test]
    fn test_oidc_partial_config() {
        run_with_env_isolation(|| {
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
        });
    }

    #[test]
    fn test_oidc_disabled_with_config_present() {
        run_with_env_isolation(|| {
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
        });
    }

    #[test]
    fn test_oidc_empty_values() {
        run_with_env_isolation(|| {
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
        });
    }

    #[test]
    fn test_oidc_config_validation_output() {
        run_with_env_isolation(|| {
            // Test that validation warnings are properly formatted
            env::set_var("OIDC_ENABLED", "true");
            env::set_var("DATABASE_URL", "postgresql://test:test@localhost/test");
            env::set_var("JWT_SECRET", "test-secret");
            // Missing required OIDC fields

            // This should succeed but show warnings
            let config = Config::from_env().unwrap();
            assert!(config.oidc_enabled);
            assert!(config.oidc_client_id.is_none());
        });
    }

    #[test]
    fn test_oidc_complete_configuration() {
        run_with_env_isolation(|| {
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
        });
    }

    #[test] 
    fn test_oidc_config_precedence() {
        run_with_env_isolation(|| {
            // Test that environment variables take precedence
            env::set_var("OIDC_ENABLED", "true");
            env::set_var("OIDC_CLIENT_ID", "env-client-id");
            env::set_var("DATABASE_URL", "postgresql://test:test@localhost/test");
            env::set_var("JWT_SECRET", "test-secret");

            let config = Config::from_env().unwrap();

            assert!(config.oidc_enabled);
            assert_eq!(config.oidc_client_id.unwrap(), "env-client-id");
        });
    }
}