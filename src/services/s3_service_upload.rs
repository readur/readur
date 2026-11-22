//! Writing objects: documents, thumbnails and processed images, with a
//! multipart upload for large files. Included from `s3_service.rs` as
//! `upload`.

use std::collections::HashMap;

use anyhow::{anyhow, Result};
#[cfg(feature = "s3")]
use aws_sdk_s3::primitives::ByteStream;
#[cfg(feature = "s3")]
use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};
#[cfg(feature = "s3")]
use tracing::{debug, error, info};
use uuid::Uuid;

#[cfg(feature = "s3")]
use crate::services::s3_client::describe_error;

use super::{keys, S3Service};

/// Threshold for using streaming multipart uploads (100MB)
const STREAMING_THRESHOLD: usize = 100 * 1024 * 1024;

/// Multipart upload chunk size (16MB - AWS minimum is 5MB, we use 16MB for better performance)
const MULTIPART_CHUNK_SIZE: usize = 16 * 1024 * 1024;

impl S3Service {
    /// Store a file directly to S3 with structured path
    pub async fn store_document(
        &self,
        user_id: Uuid,
        document_id: Uuid,
        filename: &str,
        data: &[u8],
    ) -> Result<String> {
        #[cfg(not(feature = "s3"))]
        {
            return Err(anyhow!("S3 support not compiled in"));
        }

        #[cfg(feature = "s3")]
        {
            let key = self.generate_document_key(user_id, document_id, filename);

            // Use streaming upload for large files
            if data.len() > STREAMING_THRESHOLD {
                info!(
                    "Using streaming multipart upload for large file: {} ({} bytes)",
                    key,
                    data.len()
                );
                self.store_file_multipart(&key, data, None).await?;
            } else {
                self.store_file(&key, data, None).await?;
            }

            Ok(key)
        }
    }

    /// Store a thumbnail to S3
    pub async fn store_thumbnail(
        &self,
        user_id: Uuid,
        document_id: Uuid,
        data: &[u8],
    ) -> Result<String> {
        #[cfg(not(feature = "s3"))]
        {
            return Err(anyhow!("S3 support not compiled in"));
        }

        #[cfg(feature = "s3")]
        {
            let key = keys::thumbnail_key(self.config.prefix.as_deref(), user_id, document_id);
            self.store_file(&key, data, Some(self.get_image_metadata()))
                .await?;
            Ok(key)
        }
    }

    /// Store a processed image to S3
    pub async fn store_processed_image(
        &self,
        user_id: Uuid,
        document_id: Uuid,
        data: &[u8],
    ) -> Result<String> {
        #[cfg(not(feature = "s3"))]
        {
            return Err(anyhow!("S3 support not compiled in"));
        }

        #[cfg(feature = "s3")]
        {
            let key =
                keys::processed_image_key(self.config.prefix.as_deref(), user_id, document_id);
            self.store_file(&key, data, Some(self.get_image_metadata()))
                .await?;
            Ok(key)
        }
    }

    /// Generic file storage method
    async fn store_file(
        &self,
        key: &str,
        data: &[u8],
        metadata: Option<HashMap<String, String>>,
    ) -> Result<()> {
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

                    put_request.send().await.map_err(|e| {
                        anyhow!("Failed to store file {}: {}", key, describe_error(&e))
                    })?;

                    Ok(())
                }
            })
            .await?;

            info!("Successfully stored file: {}", key);
            Ok(())
        }
    }

    /// Store large files using multipart upload for better performance and memory usage
    async fn store_file_multipart(
        &self,
        key: &str,
        data: &[u8],
        metadata: Option<HashMap<String, String>>,
    ) -> Result<()> {
        #[cfg(not(feature = "s3"))]
        {
            return Err(anyhow!("S3 support not compiled in"));
        }

        #[cfg(feature = "s3")]
        {
            info!(
                "Starting multipart upload for file: {}/{} ({} bytes)",
                self.config.bucket_name,
                key,
                data.len()
            );

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

                    let create_response = create_request.send().await.map_err(|e| {
                        anyhow!(
                            "Failed to initiate multipart upload for {}: {}",
                            key,
                            describe_error(&e)
                        )
                    })?;

                    let upload_id = create_response
                        .upload_id()
                        .ok_or_else(|| anyhow!("Missing upload ID in multipart upload response"))?;

                    info!("Initiated multipart upload for {}: {}", key, upload_id);

                    // Step 2: Upload parts in chunks
                    let mut completed_parts = Vec::new();
                    let total_chunks = data.len().div_ceil(MULTIPART_CHUNK_SIZE);

                    for (chunk_index, chunk) in data.chunks(MULTIPART_CHUNK_SIZE).enumerate() {
                        let part_number = (chunk_index + 1) as i32;

                        debug!(
                            "Uploading part {} of {} for {} ({} bytes)",
                            part_number,
                            total_chunks,
                            key,
                            chunk.len()
                        );

                        let upload_part_response = client
                            .upload_part()
                            .bucket(&bucket_name)
                            .key(&key)
                            .upload_id(upload_id)
                            .part_number(part_number)
                            .body(ByteStream::from(chunk.to_vec()))
                            .send()
                            .await
                            .map_err(|e| {
                                anyhow!(
                                    "Failed to upload part {} for {}: {}",
                                    part_number,
                                    key,
                                    describe_error(&e)
                                )
                            })?;

                        let etag = upload_part_response
                            .e_tag()
                            .ok_or_else(|| anyhow!("Missing ETag in upload part response"))?;

                        completed_parts.push(
                            CompletedPart::builder()
                                .part_number(part_number)
                                .e_tag(etag)
                                .build(),
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
                                    error!(
                                        "Failed to abort multipart upload: {}",
                                        describe_error(&abort_err)
                                    );
                                }
                            });

                            anyhow!(
                                "Failed to complete multipart upload for {}: {}",
                                key,
                                describe_error(&e)
                            )
                        })?;

                    info!("Successfully completed multipart upload for {}", key);
                    Ok(())
                }
            })
            .await?;

            Ok(())
        }
    }

    /// Key for a document being stored now. Dated by the upload month, so it
    /// is only valid at upload time: later operations use the recorded key.
    fn generate_document_key(&self, user_id: Uuid, document_id: Uuid, filename: &str) -> String {
        keys::document_key(
            self.config.prefix.as_deref(),
            user_id,
            document_id,
            filename,
            chrono::Utc::now(),
        )
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
}
