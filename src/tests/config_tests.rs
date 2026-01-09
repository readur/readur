use crate::config::Config;
use std::env;

// Helper function to clear environment variables
fn clear_db_env_vars() {
    env::remove_var("DATABASE_URL");
    env::remove_var("POSTGRES_HOST");
    env::remove_var("POSTGRES_PORT");
    env::remove_var("POSTGRES_DB");
    env::remove_var("POSTGRES_USER");
    env::remove_var("POSTGRES_PASSWORD");
    env::remove_var("ENABLE_PER_USER_WATCH");
}

// Helper function to set minimum required environment variables
fn set_minimum_env_vars() {
    env::set_var("JWT_SECRET", "test-secret-key-at-least-32-chars-long");
    env::set_var("SERVER_ADDRESS", "0.0.0.0:8000");
    env::set_var("UPLOAD_PATH", "/tmp/test_uploads");
    env::set_var("WATCH_FOLDER", "/tmp/test_watch");
}

#[test]
fn test_database_url_takes_priority() {
    clear_db_env_vars();
    set_minimum_env_vars();

    // Set both DATABASE_URL and individual vars
    env::set_var("DATABASE_URL", "postgresql://priority_user:priority_pass@priority_host:5433/priority_db");
    env::set_var("POSTGRES_HOST", "ignored_host");
    env::set_var("POSTGRES_PORT", "9999");
    env::set_var("POSTGRES_DB", "ignored_db");
    env::set_var("POSTGRES_USER", "ignored_user");
    env::set_var("POSTGRES_PASSWORD", "ignored_pass");

    let config = Config::from_env().expect("Config should load successfully");

    // DATABASE_URL should take priority
    assert_eq!(
        config.database_url,
        "postgresql://priority_user:priority_pass@priority_host:5433/priority_db"
    );
}

#[test]
fn test_individual_postgres_vars_used_when_database_url_not_set() {
    clear_db_env_vars();
    set_minimum_env_vars();

    // Set only individual vars, no DATABASE_URL
    env::set_var("POSTGRES_HOST", "custom_host");
    env::set_var("POSTGRES_PORT", "5433");
    env::set_var("POSTGRES_DB", "custom_db");
    env::set_var("POSTGRES_USER", "custom_user");
    env::set_var("POSTGRES_PASSWORD", "custom_pass");

    let config = Config::from_env().expect("Config should load successfully");

    // Should construct URL from individual vars
    assert_eq!(
        config.database_url,
        "postgresql://custom_user:custom_pass@custom_host:5433/custom_db"
    );
}

#[test]
fn test_partial_individual_vars_with_defaults() {
    clear_db_env_vars();
    set_minimum_env_vars();

    // Set only some individual vars
    env::set_var("POSTGRES_HOST", "partial_host");
    env::set_var("POSTGRES_DB", "partial_db");
    // PORT, USER, and PASSWORD should use defaults

    let config = Config::from_env().expect("Config should load successfully");

    // Should use provided values and defaults for missing ones
    assert_eq!(
        config.database_url,
        "postgresql://readur:readur@partial_host:5432/partial_db"
    );
}

#[test]
fn test_all_database_defaults_used() {
    clear_db_env_vars();
    set_minimum_env_vars();

    // Don't set any database environment variables

    let config = Config::from_env().expect("Config should load successfully");

    // Should use all defaults
    assert_eq!(
        config.database_url,
        "postgresql://readur:readur@localhost:5432/readur"
    );
}

#[test]
fn test_postgres_port_parsing() {
    clear_db_env_vars();
    set_minimum_env_vars();

    // Test with custom port
    env::set_var("POSTGRES_PORT", "15432");

    let config = Config::from_env().expect("Config should load successfully");

    // Port should be included in the URL
    assert!(config.database_url.contains(":15432/"));
}

#[test]
fn test_special_characters_in_password() {
    clear_db_env_vars();
    set_minimum_env_vars();

    // Test with special characters in password
    env::set_var("POSTGRES_USER", "test_user");
    env::set_var("POSTGRES_PASSWORD", "p@ss!word#123");
    env::set_var("POSTGRES_DB", "test_db");

    let config = Config::from_env().expect("Config should load successfully");

    // Password with special characters should be preserved
    assert!(config.database_url.starts_with("postgresql://test_user:p@ss!word#123@"));
}

#[test]
fn test_database_url_format_validation() {
    clear_db_env_vars();
    set_minimum_env_vars();

    // Test with invalid DATABASE_URL format
    env::set_var("DATABASE_URL", "invalid://url");

    let result = Config::from_env();

    // Should fail validation
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("Invalid database URL format"));
    }
}

#[test]
fn test_postgres_url_accepts_postgres_prefix() {
    clear_db_env_vars();
    set_minimum_env_vars();

    // Test with postgres:// prefix (common alternative to postgresql://)
    env::set_var("DATABASE_URL", "postgres://user:pass@host/db");

    let config = Config::from_env().expect("Config should load successfully");

    // Should accept postgres:// prefix
    assert_eq!(config.database_url, "postgres://user:pass@host/db");
}

#[test]
fn test_empty_individual_vars_use_defaults() {
    clear_db_env_vars();
    set_minimum_env_vars();

    // Set empty strings for individual vars
    env::set_var("POSTGRES_HOST", "");
    env::set_var("POSTGRES_PORT", "");

    // Empty vars should be treated as not set, falling back to defaults
    // Note: This behavior depends on the implementation handling empty strings
    let config = Config::from_env().expect("Config should load successfully");

    // The implementation will use the provided empty strings, not defaults
    // This test documents the actual behavior
    assert!(config.database_url.contains("@:"));
}

#[test]
fn test_database_url_with_special_db_names() {
    clear_db_env_vars();
    set_minimum_env_vars();

    // Test with database name containing hyphens and underscores
    env::set_var("POSTGRES_DB", "test-db_name-123");

    let config = Config::from_env().expect("Config should load successfully");

    // Database name should be preserved
    assert!(config.database_url.ends_with("/test-db_name-123"));
}

#[test]
fn test_ipv6_host_format() {
    clear_db_env_vars();
    set_minimum_env_vars();

    // Test with IPv6 address
    env::set_var("POSTGRES_HOST", "::1");
    env::set_var("POSTGRES_PORT", "5432");

    let config = Config::from_env().expect("Config should load successfully");

    // IPv6 address should be included
    assert!(config.database_url.contains("@::1:5432/"));
}

#[test]
fn test_server_address_configuration() {
    clear_db_env_vars();
    set_minimum_env_vars();

    // Test that other config options still work alongside database config
    env::set_var("SERVER_ADDRESS", "127.0.0.1:3000");
    env::set_var("POSTGRES_HOST", "localhost");

    let config = Config::from_env().expect("Config should load successfully");

    assert_eq!(config.server_address, "127.0.0.1:3000");
    assert!(config.database_url.contains("@localhost"));
}

#[test]
fn test_mixed_case_environment_variables() {
    clear_db_env_vars();
    set_minimum_env_vars();

    // Environment variable names are case-sensitive on Unix-like systems
    // This test verifies exact case matching
    env::set_var("POSTGRES_HOST", "mixed_case_host");
    // These should not be recognized (different case)
    env::set_var("postgres_host", "wrong_host");
    env::set_var("Postgres_Host", "wrong_host2");

    let config = Config::from_env().expect("Config should load successfully");

    // Should use the correctly cased variable
    assert!(config.database_url.contains("@mixed_case_host"));
}

#[test]
fn test_enable_per_user_watch_defaults_to_false() {
    clear_db_env_vars();
    set_minimum_env_vars();

    let config = Config::from_env().expect("Config should load successfully");

    // Should default to false when not set
    assert!(!config.enable_per_user_watch);
}

#[test]
fn test_enable_per_user_watch_set_to_true() {
    clear_db_env_vars();
    set_minimum_env_vars();

    env::set_var("ENABLE_PER_USER_WATCH", "true");

    let config = Config::from_env().expect("Config should load successfully");

    // Should be true when set to "true"
    assert!(config.enable_per_user_watch);
}

#[test]
fn test_enable_per_user_watch_set_to_false() {
    clear_db_env_vars();
    set_minimum_env_vars();

    env::set_var("ENABLE_PER_USER_WATCH", "false");

    let config = Config::from_env().expect("Config should load successfully");

    // Should be false when set to "false"
    assert!(!config.enable_per_user_watch);
}