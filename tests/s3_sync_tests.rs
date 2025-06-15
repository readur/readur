/*!
 * S3 Sync Service Unit Tests
 * 
 * Tests for S3 synchronization functionality including:
 * - AWS S3 and MinIO compatibility
 * - Credential handling and validation
 * - Bucket operations and permissions
 * - Object listing and metadata
 * - Prefix-based filtering
 * - Error handling and retry logic
 * - Regional and endpoint configuration
 */

use std::collections::HashMap;
use uuid::Uuid;
use chrono::Utc;
use serde_json::json;

use readur::{
    models::{S3SourceConfig, SourceType},
};

/// Create a test S3 configuration for AWS
fn create_test_aws_s3_config() -> S3SourceConfig {
    S3SourceConfig {
        bucket: "test-documents-bucket".to_string(),
        region: "us-east-1".to_string(),
        access_key_id: "AKIAIOSFODNN7EXAMPLE".to_string(),
        secret_access_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
        prefix: "documents/".to_string(),
        endpoint_url: None, // Use AWS S3
        auto_sync: true,
        sync_interval_minutes: 120,
        file_extensions: vec![".pdf".to_string(), ".txt".to_string(), ".docx".to_string()],
    }
}

/// Create a test S3 configuration for MinIO
fn create_test_minio_config() -> S3SourceConfig {
    S3SourceConfig {
        bucket: "minio-test-bucket".to_string(),
        region: "us-east-1".to_string(),
        access_key_id: "minioadmin".to_string(),
        secret_access_key: "minioadmin".to_string(),
        prefix: "".to_string(),
        endpoint_url: Some("https://minio.example.com".to_string()),
        auto_sync: true,
        sync_interval_minutes: 60,
        file_extensions: vec![".pdf".to_string(), ".jpg".to_string()],
    }
}

#[test]
fn test_s3_config_creation_aws() {
    let config = create_test_aws_s3_config();
    
    assert_eq!(config.bucket, "test-documents-bucket");
    assert_eq!(config.region, "us-east-1");
    assert!(!config.access_key_id.is_empty());
    assert!(!config.secret_access_key.is_empty());
    assert_eq!(config.prefix, "documents/");
    assert!(config.endpoint_url.is_none()); // AWS S3
    assert!(config.auto_sync);
    assert_eq!(config.sync_interval_minutes, 120);
    assert_eq!(config.file_extensions.len(), 3);
}

#[test]
fn test_s3_config_creation_minio() {
    let config = create_test_minio_config();
    
    assert_eq!(config.bucket, "minio-test-bucket");
    assert_eq!(config.region, "us-east-1");
    assert_eq!(config.access_key_id, "minioadmin");
    assert_eq!(config.secret_access_key, "minioadmin");
    assert_eq!(config.prefix, "");
    assert!(config.endpoint_url.is_some());
    assert_eq!(config.endpoint_url.unwrap(), "https://minio.example.com");
    assert_eq!(config.sync_interval_minutes, 60);
}

#[test]
fn test_s3_config_validation() {
    let config = create_test_aws_s3_config();
    
    // Test bucket name validation
    assert!(!config.bucket.is_empty());
    assert!(config.bucket.len() >= 3 && config.bucket.len() <= 63);
    assert!(!config.bucket.contains(' '));
    assert!(!config.bucket.contains('_'));
    assert!(config.bucket.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'));
    
    // Test region validation
    assert!(!config.region.is_empty());
    assert!(is_valid_aws_region(&config.region));
    
    // Test credentials validation
    assert!(!config.access_key_id.is_empty());
    assert!(!config.secret_access_key.is_empty());
    assert!(config.access_key_id.len() >= 16);
    assert!(config.secret_access_key.len() >= 16);
    
    // Test sync interval validation
    assert!(config.sync_interval_minutes > 0);
    
    // Test file extensions validation
    assert!(!config.file_extensions.is_empty());
    for ext in &config.file_extensions {
        assert!(ext.starts_with('.'));
    }
}

fn is_valid_aws_region(region: &str) -> bool {
    let valid_regions = vec![
        "us-east-1", "us-east-2", "us-west-1", "us-west-2",
        "eu-west-1", "eu-west-2", "eu-west-3", "eu-central-1",
        "ap-northeast-1", "ap-northeast-2", "ap-southeast-1", "ap-southeast-2",
        "ap-south-1", "sa-east-1", "ca-central-1"
    ];
    valid_regions.contains(&region)
}

#[test]
fn test_s3_endpoint_url_validation() {
    let test_cases = vec![
        ("https://s3.amazonaws.com", true),
        ("https://minio.example.com", true),
        ("https://storage.googleapis.com", true),
        ("http://localhost:9000", true), // MinIO development
        ("ftp://invalid.com", false),
        ("not-a-url", false),
        ("", false),
    ];
    
    for (endpoint, should_be_valid) in test_cases {
        let is_valid = validate_endpoint_url(endpoint);
        assert_eq!(is_valid, should_be_valid, "Endpoint validation failed for: {}", endpoint);
    }
}

fn validate_endpoint_url(url: &str) -> bool {
    if url.is_empty() {
        return false;
    }
    url.starts_with("http://") || url.starts_with("https://")
}

#[test]
fn test_s3_object_key_handling() {
    let prefix = "documents/";
    let filename = "test file (1).pdf";
    
    // Test object key construction
    let object_key = construct_object_key(prefix, filename);
    assert_eq!(object_key, "documents/test file (1).pdf");
    
    // Test key normalization for S3
    let normalized_key = normalize_s3_key(&object_key);
    assert!(!normalized_key.contains("//"));
    assert!(!normalized_key.starts_with('/'));
    
    // Test key extraction from full path
    let extracted_filename = extract_filename_from_key(&object_key);
    assert_eq!(extracted_filename, filename);
}

fn construct_object_key(prefix: &str, filename: &str) -> String {
    if prefix.is_empty() {
        filename.to_string()
    } else if prefix.ends_with('/') {
        format!("{}{}", prefix, filename)
    } else {
        format!("{}/{}", prefix, filename)
    }
}

fn normalize_s3_key(key: &str) -> String {
    let mut normalized = key.to_string();
    
    // Remove leading slash
    if normalized.starts_with('/') {
        normalized = normalized[1..].to_string();
    }
    
    // Remove double slashes
    while normalized.contains("//") {
        normalized = normalized.replace("//", "/");
    }
    
    normalized
}

fn extract_filename_from_key(key: &str) -> &str {
    key.split('/').last().unwrap_or(key)
}

#[test]
fn test_s3_metadata_structure() {
    use std::collections::HashMap;
    
    let object_metadata = S3ObjectMetadata {
        key: "documents/test.pdf".to_string(),
        size: 1048576, // 1MB
        last_modified: Utc::now(),
        etag: "d41d8cd98f00b204e9800998ecf8427e".to_string(),
        content_type: "application/pdf".to_string(),
        metadata: {
            let mut map = HashMap::new();
            map.insert("original-filename".to_string(), "test.pdf".to_string());
            map.insert("upload-user".to_string(), "testuser".to_string());
            map
        },
    };
    
    assert_eq!(object_metadata.key, "documents/test.pdf");
    assert_eq!(object_metadata.size, 1048576);
    assert!(!object_metadata.etag.is_empty());
    assert_eq!(object_metadata.content_type, "application/pdf");
    assert!(!object_metadata.metadata.is_empty());
    
    // Test filename extraction
    let filename = extract_filename_from_key(&object_metadata.key);
    assert_eq!(filename, "test.pdf");
    
    // Test size validation
    assert!(object_metadata.size > 0);
    
    // Test ETag format (MD5 hash)
    assert_eq!(object_metadata.etag.len(), 32);
    assert!(object_metadata.etag.chars().all(|c| c.is_ascii_hexdigit()));
}

#[derive(Debug, Clone)]
struct S3ObjectMetadata {
    key: String,
    size: u64,
    last_modified: chrono::DateTime<Utc>,
    etag: String,
    content_type: String,
    metadata: HashMap<String, String>,
}

#[test]
fn test_prefix_filtering() {
    let config = create_test_aws_s3_config();
    let prefix = &config.prefix;
    
    let test_objects = vec![
        "documents/file1.pdf",
        "documents/subfolder/file2.txt",
        "images/photo.jpg",
        "temp/cache.tmp",
        "documents/report.docx",
    ];
    
    let filtered_objects: Vec<_> = test_objects.iter()
        .filter(|obj| obj.starts_with(prefix))
        .collect();
    
    assert_eq!(filtered_objects.len(), 3); // Only documents/* objects
    assert!(filtered_objects.contains(&&"documents/file1.pdf"));
    assert!(filtered_objects.contains(&&"documents/subfolder/file2.txt"));
    assert!(filtered_objects.contains(&&"documents/report.docx"));
    assert!(!filtered_objects.contains(&&"images/photo.jpg"));
}

#[test]
fn test_file_extension_filtering_s3() {
    let config = create_test_aws_s3_config();
    let allowed_extensions = &config.file_extensions;
    
    let test_objects = vec![
        "documents/report.pdf",
        "documents/notes.txt",
        "documents/presentation.pptx",
        "documents/spreadsheet.xlsx",
        "documents/document.docx",
        "documents/archive.zip",
        "documents/image.jpg",
    ];
    
    let filtered_objects: Vec<_> = test_objects.iter()
        .filter(|obj| {
            let filename = extract_filename_from_key(obj);
            let extension = extract_extension(filename);
            allowed_extensions.contains(&extension)
        })
        .collect();
    
    assert!(filtered_objects.contains(&&"documents/report.pdf"));
    assert!(filtered_objects.contains(&&"documents/notes.txt"));
    assert!(filtered_objects.contains(&&"documents/document.docx"));
    assert!(!filtered_objects.contains(&&"documents/presentation.pptx"));
    assert!(!filtered_objects.contains(&&"documents/archive.zip"));
}

fn extract_extension(filename: &str) -> String {
    if let Some(pos) = filename.rfind('.') {
        filename[pos..].to_lowercase()
    } else {
        String::new()
    }
}

#[test]
fn test_etag_change_detection_s3() {
    let old_etag = "d41d8cd98f00b204e9800998ecf8427e";
    let new_etag = "098f6bcd4621d373cade4e832627b4f6";
    let same_etag = "d41d8cd98f00b204e9800998ecf8427e";
    
    // Test change detection
    assert_ne!(old_etag, new_etag, "Different ETags should indicate object change");
    assert_eq!(old_etag, same_etag, "Same ETags should indicate no change");
    
    // Test ETag normalization (S3 sometimes includes quotes)
    let quoted_etag = "\"d41d8cd98f00b204e9800998ecf8427e\"";
    let normalized_etag = quoted_etag.trim_matches('"');
    assert_eq!(normalized_etag, old_etag);
    
    // Test multipart upload ETag format (contains dash)
    let multipart_etag = "d41d8cd98f00b204e9800998ecf8427e-2";
    assert!(multipart_etag.contains('-'));
    let base_etag = multipart_etag.split('-').next().unwrap();
    assert_eq!(base_etag.len(), 32);
}

#[test]
fn test_aws_vs_minio_differences() {
    let aws_config = create_test_aws_s3_config();
    let minio_config = create_test_minio_config();
    
    // AWS S3 uses standard endpoints
    assert!(aws_config.endpoint_url.is_none());
    
    // MinIO uses custom endpoints
    assert!(minio_config.endpoint_url.is_some());
    let minio_endpoint = minio_config.endpoint_url.unwrap();
    assert!(minio_endpoint.starts_with("https://"));
    
    // Both should support the same regions format
    assert!(is_valid_aws_region(&aws_config.region));
    assert!(is_valid_aws_region(&minio_config.region));
    
    // MinIO often uses simpler credentials
    assert_eq!(minio_config.access_key_id, "minioadmin");
    assert_eq!(minio_config.secret_access_key, "minioadmin");
    
    // AWS uses more complex credential formats
    assert!(aws_config.access_key_id.len() >= 16);
    assert!(aws_config.secret_access_key.len() >= 16);
}

#[test]
fn test_s3_error_handling_scenarios() {
    // Test various error scenarios
    
    // Invalid bucket name
    let invalid_bucket_config = S3SourceConfig {
        bucket: "Invalid_Bucket_Name!".to_string(), // Invalid characters
        region: "us-east-1".to_string(),
        access_key_id: "test".to_string(),
        secret_access_key: "test".to_string(),
        prefix: "".to_string(),
        endpoint_url: None,
        auto_sync: true,
        sync_interval_minutes: 60,
        file_extensions: vec![".pdf".to_string()],
    };
    
    assert!(invalid_bucket_config.bucket.contains('_'));
    assert!(invalid_bucket_config.bucket.contains('!'));
    
    // Empty credentials
    let empty_creds_config = S3SourceConfig {
        bucket: "test-bucket".to_string(),
        region: "us-east-1".to_string(),
        access_key_id: "".to_string(), // Empty
        secret_access_key: "".to_string(), // Empty
        prefix: "".to_string(),
        endpoint_url: None,
        auto_sync: true,
        sync_interval_minutes: 60,
        file_extensions: vec![".pdf".to_string()],
    };
    
    assert!(empty_creds_config.access_key_id.is_empty());
    assert!(empty_creds_config.secret_access_key.is_empty());
    
    // Invalid region
    let invalid_region_config = S3SourceConfig {
        bucket: "test-bucket".to_string(),
        region: "invalid-region".to_string(),
        access_key_id: "test".to_string(),
        secret_access_key: "test".to_string(),
        prefix: "".to_string(),
        endpoint_url: None,
        auto_sync: true,
        sync_interval_minutes: 60,
        file_extensions: vec![".pdf".to_string()],
    };
    
    assert!(!is_valid_aws_region(&invalid_region_config.region));
}

#[test]
fn test_s3_performance_considerations() {
    // Test performance-related configurations
    
    let performance_config = S3PerformanceConfig {
        max_concurrent_requests: 10,
        request_timeout_seconds: 30,
        retry_attempts: 3,
        retry_backoff_base_ms: 1000,
        use_multipart_threshold_mb: 100,
        multipart_chunk_size_mb: 5,
    };
    
    assert!(performance_config.max_concurrent_requests > 0);
    assert!(performance_config.max_concurrent_requests <= 100); // Reasonable limit
    assert!(performance_config.request_timeout_seconds >= 5);
    assert!(performance_config.retry_attempts <= 5); // Don't retry too many times
    assert!(performance_config.retry_backoff_base_ms >= 100);
    assert!(performance_config.use_multipart_threshold_mb >= 5); // AWS minimum is 5MB
    assert!(performance_config.multipart_chunk_size_mb >= 5); // AWS minimum is 5MB
}

#[derive(Debug, Clone)]
struct S3PerformanceConfig {
    max_concurrent_requests: u32,
    request_timeout_seconds: u32,
    retry_attempts: u32,
    retry_backoff_base_ms: u64,
    use_multipart_threshold_mb: u64,
    multipart_chunk_size_mb: u64,
}

#[test]
fn test_s3_retry_logic() {
    // Test exponential backoff for retry logic
    fn calculate_s3_retry_delay(attempt: u32, base_delay_ms: u64) -> u64 {
        let max_delay_ms = 60_000; // 60 seconds max for S3
        let jitter_factor = 0.1; // 10% jitter
        
        let delay = base_delay_ms * 2_u64.pow(attempt.saturating_sub(1));
        let with_jitter = (delay as f64 * (1.0 + jitter_factor)) as u64;
        std::cmp::min(with_jitter, max_delay_ms)
    }
    
    assert_eq!(calculate_s3_retry_delay(1, 1000), 1100);   // ~1.1 seconds
    assert_eq!(calculate_s3_retry_delay(2, 1000), 2200);   // ~2.2 seconds
    assert_eq!(calculate_s3_retry_delay(3, 1000), 4400);   // ~4.4 seconds
    assert!(calculate_s3_retry_delay(10, 1000) <= 60000);  // Capped at 60 seconds
}

#[test]
fn test_s3_url_presigning() {
    // Test URL presigning concepts (for download URLs)
    let object_key = "documents/test.pdf";
    let bucket = "test-bucket";
    let region = "us-east-1";
    let expiry_seconds = 3600; // 1 hour
    
    // Construct what a presigned URL would look like
    let base_url = format!("https://{}.s3.{}.amazonaws.com/{}", bucket, region, object_key);
    let presigned_url = format!("{}?X-Amz-Expires={}&X-Amz-Signature=...", base_url, expiry_seconds);
    
    assert!(presigned_url.contains("amazonaws.com"));
    assert!(presigned_url.contains("X-Amz-Expires"));
    assert!(presigned_url.contains("X-Amz-Signature"));
    assert!(presigned_url.contains(&expiry_seconds.to_string()));
    
    // Test expiry validation
    assert!(expiry_seconds > 0);
    assert!(expiry_seconds <= 7 * 24 * 3600); // Max 7 days for AWS
}

#[test]
fn test_s3_batch_operations() {
    // Test batch operations for better performance
    let object_keys = vec![
        "documents/file1.pdf",
        "documents/file2.txt",
        "documents/file3.docx",
        "documents/file4.pdf",
        "documents/file5.txt",
    ];
    
    // Test batching logic
    let batch_size = 2;
    let batches: Vec<Vec<&str>> = object_keys.chunks(batch_size).map(|chunk| chunk.to_vec()).collect();
    
    assert_eq!(batches.len(), 3); // 5 items / 2 = 3 batches
    assert_eq!(batches[0].len(), 2);
    assert_eq!(batches[1].len(), 2);
    assert_eq!(batches[2].len(), 1); // Last batch has remainder
    
    // Test total items preservation
    let total_items: usize = batches.iter().map(|b| b.len()).sum();
    assert_eq!(total_items, object_keys.len());
}

#[test]
fn test_s3_content_type_detection() {
    let test_files = vec![
        ("document.pdf", "application/pdf"),
        ("image.jpg", "image/jpeg"),
        ("image.png", "image/png"),
        ("text.txt", "text/plain"),
        ("data.json", "application/json"),
        ("archive.zip", "application/zip"),
        ("unknown.xyz", "application/octet-stream"), // Default
    ];
    
    for (filename, expected_content_type) in test_files {
        let detected_type = detect_content_type(filename);
        assert_eq!(detected_type, expected_content_type, 
                   "Content type detection failed for: {}", filename);
    }
}

fn detect_content_type(filename: &str) -> &'static str {
    match filename.split('.').last().unwrap_or("").to_lowercase().as_str() {
        "pdf" => "application/pdf",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "txt" => "text/plain",
        "json" => "application/json",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
}

#[test]
fn test_s3_storage_classes() {
    // Test different S3 storage classes for cost optimization
    let storage_classes = vec![
        "STANDARD",
        "STANDARD_IA",
        "ONEZONE_IA", 
        "GLACIER",
        "DEEP_ARCHIVE",
        "INTELLIGENT_TIERING",
    ];
    
    for storage_class in storage_classes {
        assert!(!storage_class.is_empty());
        assert!(storage_class.chars().all(|c| c.is_ascii_uppercase() || c == '_'));
    }
    
    // Test default storage class
    let default_storage_class = "STANDARD";
    assert_eq!(default_storage_class, "STANDARD");
}

#[test]
fn test_concurrent_download_safety() {
    use std::sync::{Arc, Mutex};
    use std::thread;
    
    let download_stats = Arc::new(Mutex::new(DownloadStats {
        files_downloaded: 0,
        bytes_downloaded: 0,
        errors: 0,
    }));
    
    let mut handles = vec![];
    
    // Simulate concurrent downloads
    for i in 0..5 {
        let stats = Arc::clone(&download_stats);
        let handle = thread::spawn(move || {
            // Simulate download
            let file_size = 1024 * (i + 1); // Variable file sizes
            
            let mut stats = stats.lock().unwrap();
            stats.files_downloaded += 1;
            stats.bytes_downloaded += file_size;
        });
        handles.push(handle);
    }
    
    // Wait for all downloads to complete
    for handle in handles {
        handle.join().unwrap();
    }
    
    let final_stats = download_stats.lock().unwrap();
    assert_eq!(final_stats.files_downloaded, 5);
    assert!(final_stats.bytes_downloaded > 0);
    assert_eq!(final_stats.errors, 0);
}

#[derive(Debug, Clone)]
struct DownloadStats {
    files_downloaded: u32,
    bytes_downloaded: u64,
    errors: u32,
}