/*! 
 * Source Update API Tests
 * 
 * Tests for the PUT /api/sources/{id} endpoint
 */

use serde_json::json;
use uuid::Uuid;

use readur::{
    models::{UpdateSource, WebDAVSourceConfig, LocalFolderSourceConfig, S3SourceConfig, SourceType},
};

#[test]
fn test_update_source_payload_serialization() {
    // Test WebDAV update payload
    let webdav_update = UpdateSource {
        name: Some("Updated WebDAV Source".to_string()),
        enabled: Some(true),
        config: Some(json!({
            "server_url": "https://cloud.example.com",
            "username": "testuser",
            "password": "testpass",
            "watch_folders": ["/Documents", "/Pictures"],
            "file_extensions": [".pdf", ".txt", ".docx"],
            "auto_sync": true,
            "sync_interval_minutes": 60,
            "server_type": "nextcloud"
        })),
    };

    // Test serialization
    let serialized = serde_json::to_string(&webdav_update).unwrap();
    assert!(!serialized.is_empty());

    // Test deserialization back
    let deserialized: UpdateSource = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized.name, webdav_update.name);
    assert_eq!(deserialized.enabled, webdav_update.enabled);
}

#[test]
fn test_webdav_config_validation() {
    let config_json = json!({
        "server_url": "https://cloud.example.com",
        "username": "testuser",
        "password": "testpass",
        "watch_folders": ["/Documents"],
        "file_extensions": [".pdf", ".txt"],
        "auto_sync": true,
        "sync_interval_minutes": 60,
        "server_type": "nextcloud"
    });

    // This should deserialize successfully
    let config: Result<WebDAVSourceConfig, _> = serde_json::from_value(config_json);
    assert!(config.is_ok());

    let webdav_config = config.unwrap();
    assert_eq!(webdav_config.server_url, "https://cloud.example.com");
    assert_eq!(webdav_config.username, "testuser");
    assert_eq!(webdav_config.auto_sync, true);
    assert_eq!(webdav_config.sync_interval_minutes, 60);
    assert_eq!(webdav_config.server_type, Some("nextcloud".to_string()));
}

#[test]
fn test_local_folder_config_validation() {
    let config_json = json!({
        "watch_folders": ["/home/user/documents"],
        "file_extensions": [".pdf", ".txt"],
        "auto_sync": true,
        "sync_interval_minutes": 30,
        "recursive": true,
        "follow_symlinks": false
    });

    let config: Result<LocalFolderSourceConfig, _> = serde_json::from_value(config_json);
    assert!(config.is_ok());

    let local_config = config.unwrap();
    assert_eq!(local_config.watch_folders, vec!["/home/user/documents"]);
    assert_eq!(local_config.recursive, true);
    assert_eq!(local_config.follow_symlinks, false);
}

#[test]
fn test_s3_config_validation() {
    let config_json = json!({
        "bucket_name": "my-bucket",
        "region": "us-east-1",
        "access_key_id": "AKIAIOSFODNN7EXAMPLE",
        "secret_access_key": "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
        "endpoint_url": "https://s3.amazonaws.com",
        "prefix": "documents/",
        "watch_folders": ["/uploads"],
        "file_extensions": [".pdf", ".docx"],
        "auto_sync": true,
        "sync_interval_minutes": 120
    });

    let config: Result<S3SourceConfig, _> = serde_json::from_value(config_json);
    assert!(config.is_ok());

    let s3_config = config.unwrap();
    assert_eq!(s3_config.bucket_name, "my-bucket");
    assert_eq!(s3_config.region, "us-east-1");
    assert_eq!(s3_config.endpoint_url, Some("https://s3.amazonaws.com".to_string()));
    assert_eq!(s3_config.prefix, Some("documents/".to_string()));
}

#[test]
fn test_invalid_webdav_config() {
    // Missing required fields
    let invalid_config = json!({
        "server_url": "https://cloud.example.com",
        // Missing username and password
        "watch_folders": ["/Documents"],
        "file_extensions": [".pdf"],
        "auto_sync": true,
        "sync_interval_minutes": 60
    });

    let config: Result<WebDAVSourceConfig, _> = serde_json::from_value(invalid_config);
    assert!(config.is_err());
}

#[test]
fn test_config_validation_for_type() {
    // This mimics the validation function in routes/sources.rs
    fn validate_config_for_type(
        source_type: &SourceType,
        config: &serde_json::Value,
    ) -> Result<(), &'static str> {
        match source_type {
            SourceType::WebDAV => {
                let _: WebDAVSourceConfig =
                    serde_json::from_value(config.clone()).map_err(|_| "Invalid WebDAV configuration")?;
                Ok(())
            }
            SourceType::LocalFolder => {
                let _: LocalFolderSourceConfig =
                    serde_json::from_value(config.clone()).map_err(|_| "Invalid Local Folder configuration")?;
                Ok(())
            }
            SourceType::S3 => {
                let _: S3SourceConfig =
                    serde_json::from_value(config.clone()).map_err(|_| "Invalid S3 configuration")?;
                Ok(())
            }
        }
    }

    // Test valid WebDAV config
    let webdav_config = json!({
        "server_url": "https://cloud.example.com",
        "username": "testuser",
        "password": "testpass",
        "watch_folders": ["/Documents"],
        "file_extensions": [".pdf"],
        "auto_sync": true,
        "sync_interval_minutes": 60,
        "server_type": "nextcloud"
    });

    assert!(validate_config_for_type(&SourceType::WebDAV, &webdav_config).is_ok());

    // Test invalid config for WebDAV (missing password)
    let invalid_webdav_config = json!({
        "server_url": "https://cloud.example.com",
        "username": "testuser",
        // missing password
        "watch_folders": ["/Documents"],
        "file_extensions": [".pdf"],
        "auto_sync": true,
        "sync_interval_minutes": 60
    });

    assert!(validate_config_for_type(&SourceType::WebDAV, &invalid_webdav_config).is_err());
}

#[test]
fn test_update_source_partial_updates() {
    // Test updating only name
    let name_only_update = UpdateSource {
        name: Some("New Name".to_string()),
        enabled: None,
        config: None,
    };

    let serialized = serde_json::to_string(&name_only_update).unwrap();
    let deserialized: UpdateSource = serde_json::from_str(&serialized).unwrap();
    
    assert_eq!(deserialized.name, Some("New Name".to_string()));
    assert_eq!(deserialized.enabled, None);
    assert_eq!(deserialized.config, None);

    // Test updating only enabled status
    let enabled_only_update = UpdateSource {
        name: None,
        enabled: Some(false),
        config: None,
    };

    let serialized = serde_json::to_string(&enabled_only_update).unwrap();
    let deserialized: UpdateSource = serde_json::from_str(&serialized).unwrap();
    
    assert_eq!(deserialized.name, None);
    assert_eq!(deserialized.enabled, Some(false));
    assert_eq!(deserialized.config, None);
}

#[test]
fn test_frontend_payload_format() {
    // This test matches exactly what the frontend sends
    let frontend_payload = json!({
        "name": "My WebDAV Source",
        "enabled": true,
        "config": {
            "server_url": "https://cloud.example.com",
            "username": "testuser",
            "password": "testpass",
            "watch_folders": ["/Documents", "/Pictures"],
            "file_extensions": [".pdf", ".txt", ".docx"],
            "auto_sync": true,
            "sync_interval_minutes": 60,
            "server_type": "nextcloud"
        }
    });

    // Test that this can be deserialized into UpdateSource
    let update: Result<UpdateSource, _> = serde_json::from_value(frontend_payload);
    assert!(update.is_ok());

    let update_source = update.unwrap();
    assert_eq!(update_source.name, Some("My WebDAV Source".to_string()));
    assert_eq!(update_source.enabled, Some(true));
    assert!(update_source.config.is_some());

    // Test that the config can be validated as WebDAV
    if let Some(config) = &update_source.config {
        let webdav_config: Result<WebDAVSourceConfig, _> = serde_json::from_value(config.clone());
        assert!(webdav_config.is_ok());
    }
}

#[test]
fn test_empty_arrays_and_optional_fields() {
    // Test with empty arrays (should be valid)
    let config_with_empty_arrays = json!({
        "server_url": "https://cloud.example.com",
        "username": "testuser",
        "password": "testpass",
        "watch_folders": [],  // Empty array
        "file_extensions": [], // Empty array
        "auto_sync": false,
        "sync_interval_minutes": 0,
        "server_type": null  // Null optional field
    });

    let config: Result<WebDAVSourceConfig, _> = serde_json::from_value(config_with_empty_arrays);
    assert!(config.is_ok());

    let webdav_config = config.unwrap();
    assert!(webdav_config.watch_folders.is_empty());
    assert!(webdav_config.file_extensions.is_empty());
    assert_eq!(webdav_config.server_type, None);
}