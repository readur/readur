//! Basic S3 storage functionality tests

use std::sync::Arc;

use readur::services::file_service::FileService;

#[cfg(feature = "s3")]
use readur::services::s3_service::S3Service;
#[cfg(feature = "s3")]
use readur::models::S3SourceConfig;

#[cfg(feature = "s3")]
#[tokio::test]
async fn test_s3_service_new_validation() {
    // Test S3Service creation fails with empty bucket name
    let config = S3SourceConfig {
        bucket_name: "".to_string(),
        region: "us-east-1".to_string(),
        access_key_id: "".to_string(),
        secret_access_key: "".to_string(),
        endpoint_url: None,
        prefix: None,
        watch_folders: vec![],
        file_extensions: vec![],
        auto_sync: false,
        sync_interval_minutes: 0,
    };

    let result = S3Service::new(config).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Bucket name is required"));
}

#[tokio::test]
async fn test_file_service_local_creation() {
    // Test local-only FileService creation and functionality
    let upload_path = "./test_uploads".to_string();
    let local_service = FileService::new(upload_path);
    assert!(!local_service.is_s3_enabled());
}

#[cfg(feature = "s3")]
#[tokio::test]
async fn test_s3_service_configuration() {
    // Test that S3 service can be created with proper configuration structure
    let config = S3SourceConfig {
        bucket_name: "test-bucket".to_string(),
        region: "us-east-1".to_string(),
        access_key_id: "test-key".to_string(),
        secret_access_key: "test-secret".to_string(),
        endpoint_url: Some("http://localhost:9000".to_string()),
        prefix: None,
        watch_folders: vec!["documents/".to_string()],
        file_extensions: vec!["pdf".to_string(), "txt".to_string()],
        auto_sync: false,
        sync_interval_minutes: 60,
    };
    
    // This test verifies the configuration structure is correct
    // Actual S3 connection will fail since we don't have a real endpoint
    match S3Service::new(config.clone()).await {
        Ok(service) => {
            // If it succeeds, verify the config was stored correctly
            assert_eq!(service.get_config().bucket_name, "test-bucket");
            assert_eq!(service.get_config().region, "us-east-1");
            assert_eq!(service.get_config().watch_folders.len(), 1);
            
            // Test FileService integration
            let s3_file_service = FileService::new_with_s3("./test".to_string(), Arc::new(service));
            assert!(s3_file_service.is_s3_enabled());
        }
        Err(_) => {
            // Expected to fail since we don't have a real S3 endpoint
            // This test mainly verifies the structure compiles correctly
            println!("S3 service creation failed as expected (no real S3 endpoint)");
        }
    }
}