use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Datelike};
use tracing::{debug, info, warn, error};
use serde_json;
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;
use futures::stream::StreamExt;
use tokio::io::{AsyncRead, AsyncReadExt};

#[cfg(feature = "s3")]
use aws_sdk_s3::Client;
#[cfg(feature = "s3")]
use aws_credential_types::Credentials;
#[cfg(feature = "s3")]
use aws_types::region::Region as AwsRegion;
#[cfg(feature = "s3")]
use aws_sdk_s3::primitives::ByteStream;
#[cfg(feature = "s3")]
use aws_sdk_s3::types::{CompletedPart, CompletedMultipartUpload};

use crate::models::{FileIngestionInfo, S3SourceConfig};
use crate::storage::StorageBackend;

/// Threshold for using streaming multipart uploads (100MB)
const STREAMING_THRESHOLD: usize = 100 * 1024 * 1024;

/// Multipart upload chunk size (16MB - AWS minimum is 5MB, we use 16MB for better performance)
const MULTIPART_CHUNK_SIZE: usize = 16 * 1024 * 1024;

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
                                    relative_path: key.clone(),
                                    full_path: format!("s3://{}/{}", self.config.bucket_name, key), // S3 full path includes bucket
                                    #[allow(deprecated)]
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

    // ========================================
    // DIRECT STORAGE OPERATIONS
    // ========================================

    /// Store a file directly to S3 with structured path
    pub async fn store_document(&self, user_id: Uuid, document_id: Uuid, filename: &str, data: &[u8]) -> Result<String> {
        #[cfg(not(feature = "s3"))]
        {
            return Err(anyhow!("S3 support not compiled in"));
        }
        
        #[cfg(feature = "s3")]
        {
            let key = self.generate_document_key(user_id, document_id, filename);
            
            // Use streaming upload for large files
            if data.len() > STREAMING_THRESHOLD {
                info!("Using streaming multipart upload for large file: {} ({} bytes)", key, data.len());
                self.store_file_multipart(&key, data, None).await?;
            } else {
                self.store_file(&key, data, None).await?;
            }
            
            Ok(key)
        }
    }

    /// Store a thumbnail to S3
    pub async fn store_thumbnail(&self, user_id: Uuid, document_id: Uuid, data: &[u8]) -> Result<String> {
        #[cfg(not(feature = "s3"))]
        {
            return Err(anyhow!("S3 support not compiled in"));
        }
        
        #[cfg(feature = "s3")]
        {
            let key = format!("thumbnails/{}/{}_thumb.jpg", user_id, document_id);
            self.store_file(&key, data, Some(self.get_image_metadata())).await?;
            Ok(key)
        }
    }

    /// Store a processed image to S3
    pub async fn store_processed_image(&self, user_id: Uuid, document_id: Uuid, data: &[u8]) -> Result<String> {
        #[cfg(not(feature = "s3"))]
        {
            return Err(anyhow!("S3 support not compiled in"));
        }
        
        #[cfg(feature = "s3")]
        {
            let key = format!("processed_images/{}/{}_processed.png", user_id, document_id);
            self.store_file(&key, data, Some(self.get_image_metadata())).await?;
            Ok(key)
        }
    }

    /// Generic file storage method
    async fn store_file(&self, key: &str, data: &[u8], metadata: Option<HashMap<String, String>>) -> Result<()> {
        #[cfg(not(feature = "s3"))]
        {
            return Err(anyhow!("S3 support not compiled in"));
        }
        
        #[cfg(feature = "s3")]
        {
            info!("Storing file to S3: {}/{}", self.config.bucket_name, key);

            let key_owned = key.to_string();
            let data_owned = data.to_vec();
            let metadata_owned = metadata.clone();
            let bucket_name = self.config.bucket_name.clone();
            let client = self.client.clone();

            self.retry_operation(&format!("store_file: {}", key), || {
                let key = key_owned.clone();
                let data = data_owned.clone();
                let metadata = metadata_owned.clone();
                let bucket_name = bucket_name.clone();
                let client = client.clone();
                let content_type = self.get_content_type_from_key(&key);

                async move {
                    let mut put_request = client
                        .put_object()
                        .bucket(&bucket_name)
                        .key(&key)
                        .body(ByteStream::from(data));

                    // Add metadata if provided
                    if let Some(meta) = metadata {
                        for (k, v) in meta {
                            put_request = put_request.metadata(k, v);
                        }
                    }

                    // Set content type based on file extension
                    if let Some(ct) = content_type {
                        put_request = put_request.content_type(ct);
                    }

                    put_request.send().await
                        .map_err(|e| anyhow!("Failed to store file {}: {}", key, e))?;

                    Ok(())
                }
            }).await?;

            info!("Successfully stored file: {}", key);
            Ok(())
        }
    }

    /// Store large files using multipart upload for better performance and memory usage
    async fn store_file_multipart(&self, key: &str, data: &[u8], metadata: Option<HashMap<String, String>>) -> Result<()> {
        #[cfg(not(feature = "s3"))]
        {
            return Err(anyhow!("S3 support not compiled in"));
        }
        
        #[cfg(feature = "s3")]
        {
            info!("Starting multipart upload for file: {}/{} ({} bytes)", self.config.bucket_name, key, data.len());

            let key_owned = key.to_string();
            let data_owned = data.to_vec();
            let metadata_owned = metadata.clone();
            let bucket_name = self.config.bucket_name.clone();
            let client = self.client.clone();

            self.retry_operation(&format!("store_file_multipart: {}", key), || {
                let key = key_owned.clone();
                let data = data_owned.clone();
                let metadata = metadata_owned.clone();
                let bucket_name = bucket_name.clone();
                let client = client.clone();
                let content_type = self.get_content_type_from_key(&key);

                async move {
                    // Step 1: Initiate multipart upload
                    let mut create_request = client
                        .create_multipart_upload()
                        .bucket(&bucket_name)
                        .key(&key);

                    // Add metadata if provided
                    if let Some(meta) = metadata {
                        for (k, v) in meta {
                            create_request = create_request.metadata(k, v);
                        }
                    }

                    // Set content type based on file extension
                    if let Some(ct) = content_type {
                        create_request = create_request.content_type(ct);
                    }

                    let create_response = create_request.send().await
                        .map_err(|e| anyhow!("Failed to initiate multipart upload for {}: {}", key, e))?;
                    
                    let upload_id = create_response.upload_id()
                        .ok_or_else(|| anyhow!("Missing upload ID in multipart upload response"))?;
                    
                    info!("Initiated multipart upload for {}: {}", key, upload_id);

                    // Step 2: Upload parts in chunks
                    let mut completed_parts = Vec::new();
                    let total_chunks = (data.len() + MULTIPART_CHUNK_SIZE - 1) / MULTIPART_CHUNK_SIZE;
                    
                    for (chunk_index, chunk) in data.chunks(MULTIPART_CHUNK_SIZE).enumerate() {
                        let part_number = (chunk_index + 1) as i32;
                        
                        debug!("Uploading part {} of {} for {} ({} bytes)", 
                               part_number, total_chunks, key, chunk.len());

                        let upload_part_response = client
                            .upload_part()
                            .bucket(&bucket_name)
                            .key(&key)
                            .upload_id(upload_id)
                            .part_number(part_number)
                            .body(ByteStream::from(chunk.to_vec()))
                            .send()
                            .await
                            .map_err(|e| anyhow!("Failed to upload part {} for {}: {}", part_number, key, e))?;

                        let etag = upload_part_response.e_tag()
                            .ok_or_else(|| anyhow!("Missing ETag in upload part response"))?;

                        completed_parts.push(
                            CompletedPart::builder()
                                .part_number(part_number)
                                .e_tag(etag)
                                .build()
                        );
                        
                        debug!("Successfully uploaded part {} for {}", part_number, key);
                    }

                    // Step 3: Complete multipart upload
                    let completed_multipart_upload = CompletedMultipartUpload::builder()
                        .set_parts(Some(completed_parts))
                        .build();

                    client
                        .complete_multipart_upload()
                        .bucket(&bucket_name)
                        .key(&key)
                        .upload_id(upload_id)
                        .multipart_upload(completed_multipart_upload)
                        .send()
                        .await
                        .map_err(|e| {
                            // If completion fails, try to abort the multipart upload
                            let abort_client = client.clone();
                            let abort_bucket = bucket_name.clone();
                            let abort_key = key.clone();
                            let abort_upload_id = upload_id.to_string();
                            
                            tokio::spawn(async move {
                                if let Err(abort_err) = abort_client
                                    .abort_multipart_upload()
                                    .bucket(abort_bucket)
                                    .key(abort_key)
                                    .upload_id(abort_upload_id)
                                    .send()
                                    .await
                                {
                                    error!("Failed to abort multipart upload: {}", abort_err);
                                }
                            });
                            
                            anyhow!("Failed to complete multipart upload for {}: {}", key, e)
                        })?;

                    info!("Successfully completed multipart upload for {}", key);
                    Ok(())
                }
            }).await?;

            Ok(())
        }
    }

    /// Retrieve a file from S3
    pub async fn retrieve_file(&self, key: &str) -> Result<Vec<u8>> {
        #[cfg(not(feature = "s3"))]
        {
            return Err(anyhow!("S3 support not compiled in"));
        }
        
        #[cfg(feature = "s3")]
        {
            info!("Retrieving file from S3: {}/{}", self.config.bucket_name, key);

            let key_owned = key.to_string();
            let bucket_name = self.config.bucket_name.clone();
            let client = self.client.clone();

            let bytes = self.retry_operation(&format!("retrieve_file: {}", key), || {
                let key = key_owned.clone();
                let bucket_name = bucket_name.clone();
                let client = client.clone();

                async move {
                    let response = client
                        .get_object()
                        .bucket(&bucket_name)
                        .key(&key)
                        .send()
                        .await
                        .map_err(|e| anyhow!("Failed to retrieve file {}: {}", key, e))?;

                    let body = response.body.collect().await
                        .map_err(|e| anyhow!("Failed to read file body: {}", e))?;

                    Ok(body.into_bytes().to_vec())
                }
            }).await?;

            info!("Successfully retrieved file: {} ({} bytes)", key, bytes.len());
            Ok(bytes)
        }
    }

    /// Delete a file from S3
    pub async fn delete_file(&self, key: &str) -> Result<()> {
        #[cfg(not(feature = "s3"))]
        {
            return Err(anyhow!("S3 support not compiled in"));
        }
        
        #[cfg(feature = "s3")]
        {
            info!("Deleting file from S3: {}/{}", self.config.bucket_name, key);

            self.client
                .delete_object()
                .bucket(&self.config.bucket_name)
                .key(key)
                .send()
                .await
                .map_err(|e| anyhow!("Failed to delete file {}: {}", key, e))?;

            info!("Successfully deleted file: {}", key);
            Ok(())
        }
    }

    /// Check if a file exists in S3
    pub async fn file_exists(&self, key: &str) -> Result<bool> {
        #[cfg(not(feature = "s3"))]
        {
            return Err(anyhow!("S3 support not compiled in"));
        }
        
        #[cfg(feature = "s3")]
        {
            match self.client
                .head_object()
                .bucket(&self.config.bucket_name)
                .key(key)
                .send()
                .await
            {
                Ok(_) => Ok(true),
                Err(e) => {
                    let error_msg = e.to_string();
                    if error_msg.contains("NotFound") || error_msg.contains("404") {
                        Ok(false)
                    } else {
                        Err(anyhow!("Failed to check file existence {}: {}", key, e))
                    }
                }
            }
        }
    }

    /// Delete all files for a document (document, thumbnail, processed image)
    pub async fn delete_document_files(&self, user_id: Uuid, document_id: Uuid, filename: &str) -> Result<()> {
        #[cfg(not(feature = "s3"))]
        {
            return Err(anyhow!("S3 support not compiled in"));
        }
        
        #[cfg(feature = "s3")]
        {
            let document_key = self.generate_document_key(user_id, document_id, filename);
            let thumbnail_key = format!("thumbnails/{}/{}_thumb.jpg", user_id, document_id);
            let processed_key = format!("processed_images/{}/{}_processed.png", user_id, document_id);

            let mut errors = Vec::new();

            // Delete document file
            if let Err(e) = self.delete_file(&document_key).await {
                if !e.to_string().contains("NotFound") {
                    errors.push(format!("Document: {}", e));
                }
            }

            // Delete thumbnail
            if let Err(e) = self.delete_file(&thumbnail_key).await {
                if !e.to_string().contains("NotFound") {
                    errors.push(format!("Thumbnail: {}", e));
                }
            }

            // Delete processed image
            if let Err(e) = self.delete_file(&processed_key).await {
                if !e.to_string().contains("NotFound") {
                    errors.push(format!("Processed image: {}", e));
                }
            }

            if !errors.is_empty() {
                return Err(anyhow!("Failed to delete some files: {}", errors.join("; ")));
            }

            info!("Successfully deleted all files for document {}", document_id);
            Ok(())
        }
    }

    // ========================================
    // HELPER METHODS
    // ========================================

    /// Generate a structured S3 key for a document
    fn generate_document_key(&self, user_id: Uuid, document_id: Uuid, filename: &str) -> String {
        let now = chrono::Utc::now();
        let year = now.year();
        let month = now.month();
        
        // Extract file extension
        let extension = std::path::Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        
        if extension.is_empty() {
            format!("documents/{}/{:04}/{:02}/{}", user_id, year, month, document_id)
        } else {
            format!("documents/{}/{:04}/{:02}/{}.{}", user_id, year, month, document_id, extension)
        }
    }

    /// Get content type from S3 key/filename
    fn get_content_type_from_key(&self, key: &str) -> Option<String> {
        let extension = std::path::Path::new(key)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        Some(Self::get_mime_type(&extension))
    }

    /// Get metadata for image files
    fn get_image_metadata(&self) -> HashMap<String, String> {
        let mut metadata = HashMap::new();
        metadata.insert("generated-by".to_string(), "readur".to_string());
        metadata.insert("created-at".to_string(), chrono::Utc::now().to_rfc3339());
        metadata
    }

    /// Retry wrapper for S3 operations with exponential backoff
    async fn retry_operation<T, F, Fut>(&self, operation_name: &str, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        const MAX_RETRIES: u32 = 3;
        const BASE_DELAY_MS: u64 = 100;
        
        let mut last_error = None;
        
        for attempt in 0..=MAX_RETRIES {
            match operation().await {
                Ok(result) => {
                    if attempt > 0 {
                        info!("S3 operation '{}' succeeded after {} retries", operation_name, attempt);
                    }
                    return Ok(result);
                }
                Err(e) => {
                    last_error = Some(e);
                    
                    if attempt < MAX_RETRIES {
                        let delay_ms = BASE_DELAY_MS * 2u64.pow(attempt);
                        warn!("S3 operation '{}' failed (attempt {}/{}), retrying in {}ms: {}", 
                              operation_name, attempt + 1, MAX_RETRIES + 1, delay_ms, last_error.as_ref().unwrap());
                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    }
                }
            }
        }
        
        error!("S3 operation '{}' failed after {} attempts: {}", 
               operation_name, MAX_RETRIES + 1, last_error.as_ref().unwrap());
        Err(last_error.unwrap())
    }
}

// Implement StorageBackend trait for S3Service
#[async_trait]
impl StorageBackend for S3Service {
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
    async fn store_document(&self, user_id: Uuid, document_id: Uuid, filename: &str, data: &[u8]) -> Result<String> {
        // Generate S3 key
        let key = self.generate_document_key(user_id, document_id, filename);
        
        // Use streaming upload for large files
        if data.len() > STREAMING_THRESHOLD {
            info!("Using streaming multipart upload for large file: {} ({} bytes)", key, data.len());
            self.store_file_multipart(&key, data, None).await?;
        } else {
            self.store_file(&key, data, None).await?;
        }
        
        Ok(format!("s3://{}", key))
    }

    async fn store_thumbnail(&self, user_id: Uuid, document_id: Uuid, data: &[u8]) -> Result<String> {
        let key = format!("thumbnails/{}/{}_thumb.jpg", user_id, document_id);
        self.store_file(&key, data, Some(self.get_image_metadata())).await?;
        Ok(format!("s3://{}", key))
    }

    async fn store_processed_image(&self, user_id: Uuid, document_id: Uuid, data: &[u8]) -> Result<String> {
        let key = format!("processed_images/{}/{}_processed.png", user_id, document_id);
        self.store_file(&key, data, Some(self.get_image_metadata())).await?;
        Ok(format!("s3://{}", key))
    }

    async fn retrieve_file(&self, path: &str) -> Result<Vec<u8>> {
        // Handle s3:// prefix if present
        let key = if path.starts_with("s3://") {
            path.strip_prefix("s3://").unwrap_or(path)
        } else {
            path
        };
        
        #[cfg(not(feature = "s3"))]
        {
            return Err(anyhow!("S3 support not compiled in"));
        }
        
        #[cfg(feature = "s3")]
        {
            info!("Retrieving file from S3: {}/{}", self.config.bucket_name, key);

            let key_owned = key.to_string();
            let bucket_name = self.config.bucket_name.clone();
            let client = self.client.clone();

            let bytes = self.retry_operation(&format!("retrieve_file: {}", key), || {
                let key = key_owned.clone();
                let bucket_name = bucket_name.clone();
                let client = client.clone();

                async move {
                    let response = client
                        .get_object()
                        .bucket(&bucket_name)
                        .key(&key)
                        .send()
                        .await
                        .map_err(|e| anyhow!("Failed to retrieve file {}: {}", key, e))?;

                    let body = response.body.collect().await
                        .map_err(|e| anyhow!("Failed to read file body: {}", e))?;

                    Ok(body.into_bytes().to_vec())
                }
            }).await?;

            info!("Successfully retrieved file: {} ({} bytes)", key, bytes.len());
            Ok(bytes)
        }
    }

    async fn delete_document_files(&self, user_id: Uuid, document_id: Uuid, filename: &str) -> Result<()> {
        #[cfg(not(feature = "s3"))]
        {
            return Err(anyhow!("S3 support not compiled in"));
        }
        
        #[cfg(feature = "s3")]
        {
            let document_key = self.generate_document_key(user_id, document_id, filename);
            let thumbnail_key = format!("thumbnails/{}/{}_thumb.jpg", user_id, document_id);
            let processed_key = format!("processed_images/{}/{}_processed.png", user_id, document_id);

            let mut errors = Vec::new();

            // Delete document file
            if let Err(e) = self.delete_file(&document_key).await {
                if !e.to_string().contains("NotFound") {
                    errors.push(format!("Document: {}", e));
                }
            }

            // Delete thumbnail
            if let Err(e) = self.delete_file(&thumbnail_key).await {
                if !e.to_string().contains("NotFound") {
                    errors.push(format!("Thumbnail: {}", e));
                }
            }

            // Delete processed image
            if let Err(e) = self.delete_file(&processed_key).await {
                if !e.to_string().contains("NotFound") {
                    errors.push(format!("Processed image: {}", e));
                }
            }

            if !errors.is_empty() {
                return Err(anyhow!("Failed to delete some files: {}", errors.join("; ")));
            }

            info!("Successfully deleted all files for document {}", document_id);
            Ok(())
        }
    }

    async fn file_exists(&self, path: &str) -> Result<bool> {
        // Handle s3:// prefix if present
        let key = if path.starts_with("s3://") {
            path.strip_prefix("s3://").unwrap_or(path)
        } else {
            path
        };
        
        #[cfg(not(feature = "s3"))]
        {
            return Err(anyhow!("S3 support not compiled in"));
        }
        
        #[cfg(feature = "s3")]
        {
            match self.client
                .head_object()
                .bucket(&self.config.bucket_name)
                .key(key)
                .send()
                .await
            {
                Ok(_) => Ok(true),
                Err(e) => {
                    let error_msg = e.to_string();
                    if error_msg.contains("NotFound") || error_msg.contains("404") {
                        Ok(false)
                    } else {
                        Err(anyhow!("Failed to check file existence {}: {}", key, e))
                    }
                }
            }
        }
    }

    fn storage_type(&self) -> &'static str {
        "s3"
    }

    async fn initialize(&self) -> Result<()> {
        self.test_connection().await?;
        info!("S3 storage backend initialized successfully");
        Ok(())
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