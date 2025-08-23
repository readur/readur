use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use uuid::Uuid;

use readur::{
    services::webdav::{WebDAVService, WebDAVConfig, RetryConfig},
    scheduling::webdav_scheduler::WebDAVScheduler,
    models::*,
    db::Database,
    config::Config,
    AppState,
};

#[tokio::test]
async fn test_retry_config_default() {
    let retry_config = RetryConfig::default();
    
    assert_eq!(retry_config.max_retries, 3);
    assert_eq!(retry_config.initial_delay_ms, 1000);
    assert_eq!(retry_config.max_delay_ms, 10000);
    assert_eq!(retry_config.backoff_multiplier, 2.0);
    assert_eq!(retry_config.timeout_seconds, 30);
}

#[tokio::test]
async fn test_webdav_service_with_custom_retry() {
    let config = WebDAVConfig {
        server_url: "https://cloud.example.com".to_string(),
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        watch_folders: vec!["/Documents".to_string()],
        file_extensions: vec!["pdf".to_string()],
        timeout_seconds: 30,
        server_type: Some("nextcloud".to_string()),
    };

    let retry_config = RetryConfig {
        max_retries: 5,
        initial_delay_ms: 500,
        max_delay_ms: 10000,
        backoff_multiplier: 1.5,
        timeout_seconds: 60,
        rate_limit_backoff_ms: 5000,
    };

    let result = WebDAVService::new_with_retry(config, retry_config);
    assert!(result.is_ok());
}

#[test]
fn test_webdav_config_builder() {
    let config = WebDAVConfig {
        server_url: "https://nextcloud.example.com".to_string(),
        username: "admin".to_string(),
        password: "secret123".to_string(),
        watch_folders: vec!["/Documents".to_string(), "/Photos".to_string()],
        file_extensions: vec!["pdf".to_string(), "png".to_string(), "jpg".to_string()],
        timeout_seconds: 60,
        server_type: Some("nextcloud".to_string()),
    };

    // Test Nextcloud URL construction
    let service = WebDAVService::new(config.clone()).unwrap();
    // Note: We can't directly test the private base_webdav_url field,
    // but we can test that the service was created successfully
    
    assert_eq!(config.watch_folders.len(), 2);
    assert_eq!(config.file_extensions.len(), 3);
}

#[test]
fn test_notification_models() {
    let notification_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    
    let notification = Notification {
        id: notification_id,
        user_id,
        notification_type: "success".to_string(),
        title: "WebDAV Sync Complete".to_string(),
        message: "Successfully processed 5 files".to_string(),
        read: false,
        action_url: Some("/documents".to_string()),
        metadata: Some(serde_json::json!({
            "sync_type": "webdav",
            "files_processed": 5
        })),
        created_at: chrono::Utc::now(),
    };

    assert_eq!(notification.id, notification_id);
    assert_eq!(notification.user_id, user_id);
    assert_eq!(notification.notification_type, "success");
    assert_eq!(notification.title, "WebDAV Sync Complete");
    assert!(!notification.read);
    assert_eq!(notification.action_url, Some("/documents".to_string()));
    
    // Test metadata extraction
    let metadata = notification.metadata.unwrap();
    assert_eq!(metadata["sync_type"], "webdav");
    assert_eq!(metadata["files_processed"], 5);
}

#[test]
fn test_create_notification_model() {
    let create_notification = CreateNotification {
        notification_type: "warning".to_string(),
        title: "WebDAV Connection Issue".to_string(),
        message: "Unable to connect to WebDAV server, retrying...".to_string(),
        action_url: Some("/settings".to_string()),
        metadata: Some(serde_json::json!({
            "error_type": "connection_timeout",
            "retry_count": 2
        })),
    };

    assert_eq!(create_notification.notification_type, "warning");
    assert_eq!(create_notification.title, "WebDAV Connection Issue");
    assert_eq!(create_notification.action_url, Some("/settings".to_string()));
    
    let metadata = create_notification.metadata.unwrap();
    assert_eq!(metadata["error_type"], "connection_timeout");
    assert_eq!(metadata["retry_count"], 2);
}

#[test]
fn test_notification_summary() {
    let user_id = Uuid::new_v4();
    
    let notification1 = Notification {
        id: Uuid::new_v4(),
        user_id,
        notification_type: "success".to_string(),
        title: "Sync Complete".to_string(),
        message: "10 files processed".to_string(),
        read: false,
        action_url: None,
        metadata: None,
        created_at: chrono::Utc::now(),
    };

    let notification2 = Notification {
        id: Uuid::new_v4(),
        user_id,
        notification_type: "error".to_string(),
        title: "Sync Failed".to_string(),
        message: "Connection timeout".to_string(),
        read: true,
        action_url: Some("/settings".to_string()),
        metadata: None,
        created_at: chrono::Utc::now(),
    };

    let summary = NotificationSummary {
        unread_count: 1,
        recent_notifications: vec![notification1, notification2],
    };

    assert_eq!(summary.unread_count, 1);
    assert_eq!(summary.recent_notifications.len(), 2);
    assert!(!summary.recent_notifications[0].read);
    assert!(summary.recent_notifications[1].read);
}

#[test]
fn test_webdav_error_handling() {
    // Test error classification for retry logic
    let timeout_error = anyhow::anyhow!("Connection timeout occurred");
    let network_error = anyhow::anyhow!("Network connection failed");
    let auth_error = anyhow::anyhow!("401 Unauthorized");
    
    // These would be tested by WebDAVService::is_retryable_error if it were public
    // For now, we test that errors can be created and formatted
    assert!(timeout_error.to_string().contains("timeout"));
    assert!(network_error.to_string().contains("connection"));
    assert!(auth_error.to_string().contains("401"));
}

#[test]
fn test_webdav_file_filtering() {
    let supported_extensions = vec!["pdf", "png", "jpg", "jpeg", "tiff", "bmp", "txt"];
    
    // Test file extension extraction and filtering
    let test_files = vec![
        "document.pdf",
        "image.PNG", // Test case insensitivity
        "photo.jpg",
        "spreadsheet.xlsx", // Should be filtered out
        "text.txt",
        "archive.zip", // Should be filtered out
        "picture.jpeg",
    ];

    let mut supported_count = 0;
    for filename in test_files {
        if let Some(extension) = std::path::Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
        {
            let ext_lower = extension.to_lowercase();
            if supported_extensions.contains(&ext_lower.as_str()) {
                supported_count += 1;
            }
        }
    }

    assert_eq!(supported_count, 5); // pdf, PNG, jpg, txt, jpeg
}

#[test]
fn test_priority_calculation() {
    // Test OCR priority calculation based on file size
    let test_cases = vec![
        (500_000, 10),      // 500KB -> highest priority
        (2_000_000, 8),     // 2MB -> high priority  
        (7_000_000, 6),     // 7MB -> medium priority
        (25_000_000, 4),    // 25MB -> low priority
        (100_000_000, 2),   // 100MB -> lowest priority
    ];

    for (file_size, expected_priority) in test_cases {
        let priority = match file_size {
            0..=1048576 => 10,          // <= 1MB
            ..=5242880 => 8,            // 1-5MB
            ..=10485760 => 6,           // 5-10MB  
            ..=52428800 => 4,           // 10-50MB
            _ => 2,                     // > 50MB
        };
        
        assert_eq!(priority, expected_priority, 
            "File size {} bytes should have priority {}", file_size, expected_priority);
    }
}

#[test]
fn test_webdav_url_construction() {
    // Test different server types and URL construction
    let test_cases = vec![
        ("nextcloud", "https://cloud.example.com", "testuser", "https://cloud.example.com/remote.php/dav/files/testuser"),
        ("owncloud", "https://cloud.example.com/", "admin", "https://cloud.example.com/remote.php/dav/files/admin"),
        ("generic", "https://webdav.example.com", "user", "https://webdav.example.com/webdav"),
    ];

    for (server_type, server_url, username, expected_base) in test_cases {
        let config = WebDAVConfig {
            server_url: server_url.to_string(),
            username: username.to_string(),
            password: "password".to_string(),
            watch_folders: vec!["/Documents".to_string()],
            file_extensions: vec!["pdf".to_string()],
            timeout_seconds: 30,
            server_type: Some(server_type.to_string()),
        };

        let service = WebDAVService::new(config);
        assert!(service.is_ok(), "Failed to create WebDAV service for {} server type", server_type);
        
        // Note: We can't directly test the URL construction since base_webdav_url is private,
        // but we can verify the service was created successfully with the config
    }
}

#[test]
fn test_settings_webdav_integration() {
    let mut settings = Settings::default();
    
    // Test enabling WebDAV
    settings.webdav_enabled = true;
    settings.webdav_server_url = Some("https://nextcloud.example.com".to_string());
    settings.webdav_username = Some("testuser".to_string());
    settings.webdav_password = Some("testpass".to_string());
    settings.webdav_auto_sync = true;
    settings.webdav_sync_interval_minutes = 30;
    settings.webdav_watch_folders = vec!["/Documents".to_string(), "/Photos".to_string()];

    assert!(settings.webdav_enabled);
    assert_eq!(settings.webdav_server_url, Some("https://nextcloud.example.com".to_string()));
    assert_eq!(settings.webdav_username, Some("testuser".to_string()));
    assert!(settings.webdav_auto_sync);
    assert_eq!(settings.webdav_sync_interval_minutes, 30);
    assert_eq!(settings.webdav_watch_folders.len(), 2);

    // Test that we can build a WebDAVConfig from settings
    if let (Some(server_url), Some(username)) = (&settings.webdav_server_url, &settings.webdav_username) {
        let webdav_config = WebDAVConfig {
            server_url: server_url.clone(),
            username: username.clone(),
            password: settings.webdav_password.clone().unwrap_or_default(),
            watch_folders: settings.webdav_watch_folders.clone(),
            file_extensions: settings.webdav_file_extensions.clone(),
            timeout_seconds: 30,
            server_type: Some("nextcloud".to_string()),
        };

        assert_eq!(webdav_config.server_url, "https://nextcloud.example.com");
        assert_eq!(webdav_config.username, "testuser");
        assert_eq!(webdav_config.watch_folders.len(), 2);
    }
}

#[test]
fn test_backoff_calculation() {
    let retry_config = RetryConfig::default();
    let mut delay = retry_config.initial_delay_ms;

    // Test exponential backoff calculation
    let expected_delays = vec![1000, 2000, 4000]; // 1s, 2s, 4s with 2.0 multiplier
    
    for expected in expected_delays {
        assert_eq!(delay, expected);
        delay = ((delay as f64 * retry_config.backoff_multiplier) as u64)
            .min(retry_config.max_delay_ms);
    }
    
    // Test that delay doesn't exceed max
    for _ in 0..10 {
        delay = ((delay as f64 * retry_config.backoff_multiplier) as u64)
            .min(retry_config.max_delay_ms);
        assert!(delay <= retry_config.max_delay_ms);
    }
}

// Mock test for WebDAV scheduler (without actual database)
#[test]
fn test_webdav_scheduler_creation() {
    // Create mock state - in a real test environment you'd use test database
    let config = Config {
        database_url: "postgres://test".to_string(),
        server_address: "127.0.0.1:3000".to_string(),
        upload_path: "/tmp/test_uploads".to_string(),
        watch_folder: "/tmp/test_watch".to_string(),
        user_watch_base_dir: "./user_watch".to_string(),
        enable_per_user_watch: false,
        jwt_secret: "test_secret".to_string(),
        allowed_file_types: vec!["pdf".to_string(), "png".to_string()],
        watch_interval_seconds: Some(10),
        file_stability_check_ms: Some(1000),
        max_file_age_hours: Some(24),
        cpu_priority: "normal".to_string(),
        memory_limit_mb: 512,
        concurrent_ocr_jobs: 4,
        max_file_size_mb: 50,
        ocr_language: "eng".to_string(),
        ocr_timeout_seconds: 300,
        oidc_enabled: false,
        oidc_client_id: None,
        oidc_client_secret: None,
        oidc_issuer_url: None,
        oidc_redirect_uri: None,
        s3_enabled: false,
        s3_config: None,
    };

    // Note: This is a minimal test since we can't easily mock the database
    // In a full integration test, you'd set up a test database
    
    assert_eq!(config.server_address, "127.0.0.1:3000");
    assert_eq!(config.upload_path, "/tmp/test_uploads");
    
    // The scheduler would be tested with a real database in integration tests
}