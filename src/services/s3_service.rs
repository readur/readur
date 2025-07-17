use anyhow::{anyhow, Result};
use chrono::DateTime;
use tracing::{debug, info, warn};
use serde_json;

#[cfg(feature = "s3")]
use aws_sdk_s3::Client;
#[cfg(feature = "s3")]
use aws_credential_types::Credentials;
#[cfg(feature = "s3")]
use aws_types::region::Region as AwsRegion;

use crate::models::{FileIngestionInfo, S3SourceConfig};

#[derive(Debug, Clone)]
pub struct S3Service {
    #[cfg(feature = "s3")]
    client: Client,
    config: S3SourceConfig,
}

impl S3Service {
    pub async fn new(config: S3SourceConfig) -> Result<Self> {
        #[cfg(not(feature = "s3"))]
        {
            return Err(anyhow!("S3 support not compiled in. Enable the 's3' feature to use S3 sources."));
        }
        
        #[cfg(feature = "s3")]
        {
        // Validate required fields
        if config.bucket_name.is_empty() {
            return Err(anyhow!("Bucket name is required"));
        }
        if config.access_key_id.is_empty() {
            return Err(anyhow!("Access key ID is required"));
        }
        if config.secret_access_key.is_empty() {
            return Err(anyhow!("Secret access key is required"));
        }

        // Create S3 client with custom configuration
        let credentials = Credentials::new(
            &config.access_key_id,
            &config.secret_access_key,
            None, // session token
            None, // expiry
            "readur-s3-source"
        );

        let region = if config.region.is_empty() {
            "us-east-1".to_string()
        } else {
            config.region.clone()
        };

        let mut s3_config_builder = aws_sdk_s3::config::Builder::new()
            .region(AwsRegion::new(region))
            .credentials_provider(credentials)
            .behavior_version_latest();

        // Set custom endpoint if provided (for S3-compatible services)
        if let Some(endpoint_url) = &config.endpoint_url {
            if !endpoint_url.is_empty() {
                s3_config_builder = s3_config_builder.endpoint_url(endpoint_url);
                info!("Using custom S3 endpoint: {}", endpoint_url);
            }
        }

        let s3_config = s3_config_builder.build();
        let client = Client::from_conf(s3_config);

        Ok(Self { 
            #[cfg(feature = "s3")]
            client, 
            config 
        })
        }
    }

    /// Discover files in a specific S3 prefix (folder)
    pub async fn discover_files_in_folder(&self, folder_path: &str) -> Result<Vec<FileIngestionInfo>> {
        #[cfg(not(feature = "s3"))]
        {
            return Err(anyhow!("S3 support not compiled in"));
        }
        
        #[cfg(feature = "s3")]
        {
        info!("Scanning S3 bucket: {} prefix: {}", self.config.bucket_name, folder_path);

        let mut files = Vec::new();
        let mut continuation_token: Option<String> = None;

        loop {
            let mut list_request = self.client
                .list_objects_v2()
                .bucket(&self.config.bucket_name)
                .prefix(folder_path);

            if let Some(token) = &continuation_token {
                list_request = list_request.continuation_token(token);
            }

            match list_request.send().await {
                Ok(response) => {
                    if let Some(contents) = response.contents {
                        for object in contents {
                            if let Some(key) = object.key {
                                // Skip "directories" (keys ending with /)
                                if key.ends_with('/') {
                                    continue;
                                }

                                // Check file extension
                                let extension = std::path::Path::new(&key)
                                    .extension()
                                    .and_then(|ext| ext.to_str())
                                    .unwrap_or("")
                                    .to_lowercase();

                                if !self.config.file_extensions.contains(&extension) {
                                    debug!("Skipping S3 object with unsupported extension: {}", key);
                                    continue;
                                }

                                let file_name = std::path::Path::new(&key)
                                    .file_name()
                                    .and_then(|name| name.to_str())
                                    .unwrap_or(&key)
                                    .to_string();

                                let size = object.size.unwrap_or(0);
                                let last_modified = object.last_modified
                                    .and_then(|dt| {
                                        // Convert AWS DateTime to chrono DateTime
                                        let timestamp = dt.secs();
                                        DateTime::from_timestamp(timestamp, 0)
                                    });

                                let etag = object.e_tag.unwrap_or_else(|| {
                                    // Generate a fallback ETag if none provided
                                    format!("fallback-{}", &key.chars().take(16).collect::<String>())
                                });

                                // Remove quotes from ETag if present
                                let etag = etag.trim_matches('"').to_string();

                                let mime_type = Self::get_mime_type(&extension);

                                // Build additional metadata from S3 object properties
                                let mut metadata_map = serde_json::Map::new();
                                
                                // Add S3-specific metadata
                                if let Some(storage_class) = &object.storage_class {
                                    metadata_map.insert("storage_class".to_string(), serde_json::Value::String(storage_class.as_str().to_string()));
                                }
                                
                                if let Some(owner) = &object.owner {
                                    if let Some(display_name) = &owner.display_name {
                                        metadata_map.insert("owner_display_name".to_string(), serde_json::Value::String(display_name.clone()));
                                    }
                                    if let Some(id) = &owner.id {
                                        metadata_map.insert("owner_id".to_string(), serde_json::Value::String(id.clone()));
                                    }
                                }
                                
                                // Store the S3 key for reference
                                metadata_map.insert("s3_key".to_string(), serde_json::Value::String(key.clone()));
                                
                                // Add bucket name for reference
                                metadata_map.insert("s3_bucket".to_string(), serde_json::Value::String(self.config.bucket_name.clone()));
                                
                                // If we have region info, add it
                                metadata_map.insert("s3_region".to_string(), serde_json::Value::String(self.config.region.clone()));
                                
                                let file_info = FileIngestionInfo {
                                    path: key.clone(),
                                    name: file_name,
                                    size,
                                    mime_type,
                                    last_modified,
                                    etag,
                                    is_directory: false,
                                    created_at: None, // S3 doesn't provide creation time, only last modified
                                    permissions: None, // S3 uses different permission model (ACLs/policies)
                                    owner: object.owner.as_ref().and_then(|o| o.display_name.clone()),
                                    group: None, // S3 doesn't have Unix-style groups
                                    metadata: if metadata_map.is_empty() { None } else { Some(serde_json::Value::Object(metadata_map)) },
                                };

                                files.push(file_info);
                            }
                        }
                    }

                    // Check if there are more results
                    if response.is_truncated == Some(true) {
                        continuation_token = response.next_continuation_token;
                    } else {
                        break;
                    }
                }
                Err(e) => {
                    return Err(anyhow!("Failed to list S3 objects: {}", e));
                }
            }
        }

        info!("Found {} files in S3 bucket {} prefix {}", files.len(), self.config.bucket_name, folder_path);
        Ok(files)
        }
    }

    /// Download file content from S3
    pub async fn download_file(&self, object_key: &str) -> Result<Vec<u8>> {
        #[cfg(not(feature = "s3"))]
        {
            return Err(anyhow!("S3 support not compiled in"));
        }
        
        #[cfg(feature = "s3")]
        {
        info!("Downloading S3 object: {}/{}", self.config.bucket_name, object_key);

        let response = self.client
            .get_object()
            .bucket(&self.config.bucket_name)
            .key(object_key)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to download S3 object {}: {}", object_key, e))?;

        let body = response.body.collect().await
            .map_err(|e| anyhow!("Failed to read S3 object body: {}", e))?;

        let bytes = body.into_bytes().to_vec();
        info!("Downloaded S3 object {} ({} bytes)", object_key, bytes.len());
        
        Ok(bytes)
        }
    }

    /// Test S3 connection and access to bucket
    pub async fn test_connection(&self) -> Result<String> {
        #[cfg(not(feature = "s3"))]
        {
            return Err(anyhow!("S3 support not compiled in"));
        }
        
        #[cfg(feature = "s3")]
        {
            info!("Testing S3 connection to bucket: {}", self.config.bucket_name);

            // Test bucket access by listing objects with a limit
            let response = self.client
                .list_objects_v2()
                .bucket(&self.config.bucket_name)
                .max_keys(1)
                .send()
                .await
                .map_err(|e| anyhow!("Failed to access S3 bucket {}: {}", self.config.bucket_name, e))?;

            // Test if we can get bucket region (additional validation)
            let _head_bucket_response = self.client
                .head_bucket()
                .bucket(&self.config.bucket_name)
                .send()
                .await
                .map_err(|e| anyhow!("Cannot access bucket {}: {}", self.config.bucket_name, e))?;

            let object_count = response.key_count.unwrap_or(0);
            
            Ok(format!(
                "Successfully connected to S3 bucket '{}' (found {} objects)",
                self.config.bucket_name, object_count
            ))
        }
    }

    /// Get estimated file count and size for all watch folders
    pub async fn estimate_sync(&self) -> Result<(usize, i64)> {
        let mut total_files = 0;
        let mut total_size = 0i64;

        for folder in &self.config.watch_folders {
            match self.discover_files_in_folder(folder).await {
                Ok(files) => {
                    total_files += files.len();
                    total_size += files.iter().map(|f| f.size).sum::<i64>();
                }
                Err(e) => {
                    warn!("Failed to estimate folder {}: {}", folder, e);
                }
            }
        }

        Ok((total_files, total_size))
    }

    /// Get MIME type based on file extension
    fn get_mime_type(extension: &str) -> String {
        match extension {
            "pdf" => "application/pdf",
            "txt" => "text/plain",
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "tiff" | "tif" => "image/tiff",
            "bmp" => "image/bmp",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "doc" => "application/msword",
            "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "xls" => "application/vnd.ms-excel",
            "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            "ppt" => "application/vnd.ms-powerpoint",
            "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            _ => "application/octet-stream",
        }.to_string()
    }

    pub fn get_config(&self) -> &S3SourceConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_s3_config_creation() {
        let config = S3SourceConfig {
            bucket_name: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            access_key_id: "test-key".to_string(),
            secret_access_key: "test-secret".to_string(),
            endpoint_url: None,
            prefix: None,
            watch_folders: vec!["documents/".to_string()],
            file_extensions: vec!["pdf".to_string(), "txt".to_string()],
            auto_sync: true,
            sync_interval_minutes: 60,
        };

        // This will create the client but won't test actual S3 access
        let service = S3Service::new(config).await;
        #[cfg(feature = "s3")]
        assert!(service.is_ok());
        #[cfg(not(feature = "s3"))]
        assert!(service.is_err());
    }

    #[test]
    fn test_mime_type_detection() {
        assert_eq!(S3Service::get_mime_type("pdf"), "application/pdf");
        assert_eq!(S3Service::get_mime_type("jpg"), "image/jpeg");
        assert_eq!(S3Service::get_mime_type("txt"), "text/plain");
        assert_eq!(S3Service::get_mime_type("unknown"), "application/octet-stream");
    }
}