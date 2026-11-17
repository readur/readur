//! Deleting objects, and the [`StorageBackend`] implementation the rest of
//! the server stores files through. Included from `s3_service.rs` as
//! `backend`.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use tracing::info;
use uuid::Uuid;

#[cfg(feature = "s3")]
use crate::services::s3_client::describe_error;
use crate::storage::StorageBackend;

use super::{keys, ObjectDeletion, S3Service};

impl S3Service {
    /// Delete one object. A key the store reports missing is not an error:
    /// the object is gone either way.
    pub async fn delete_file(&self, key: &str) -> Result<()> {
        self.delete_object(key).await.map(|_| ())
    }

    /// Delete one object, telling a deleted object from one the store
    /// reported missing (classified from the typed service error, see
    /// [`keys::is_missing_object`]). Other failures are retried, then returned.
    pub async fn delete_object(&self, key: &str) -> Result<ObjectDeletion> {
        #[cfg(not(feature = "s3"))]
        {
            return Err(anyhow!("S3 support not compiled in"));
        }

        #[cfg(feature = "s3")]
        {
            info!("Deleting file from S3: {}/{}", self.config.bucket_name, key);
            let key_owned = key.to_string();
            let bucket_name = self.config.bucket_name.clone();
            let client = self.client.clone();

            let outcome = self
                .retry_operation(&format!("delete_file: {}", key), || {
                    let key = key_owned.clone();
                    let bucket_name = bucket_name.clone();
                    let client = client.clone();
                    async move {
                        match client
                            .delete_object()
                            .bucket(&bucket_name)
                            .key(&key)
                            .send()
                            .await
                        {
                            Ok(_) => Ok(ObjectDeletion::Deleted),
                            Err(e) if keys::is_missing_object(&e) => {
                                Ok(ObjectDeletion::AlreadyGone)
                            }
                            Err(e) => Err(anyhow!(
                                "Failed to delete file {}: {}",
                                key,
                                describe_error(&e)
                            )),
                        }
                    }
                })
                .await?;

            match outcome {
                ObjectDeletion::Deleted => info!("Successfully deleted file: {}", key),
                ObjectDeletion::AlreadyGone => info!("File already absent from S3: {}", key),
            }
            Ok(outcome)
        }
    }
}

// Implement StorageBackend trait for S3Service
#[async_trait]
impl StorageBackend for S3Service {
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
    async fn store_document(
        &self,
        user_id: Uuid,
        document_id: Uuid,
        filename: &str,
        data: &[u8],
    ) -> Result<String> {
        let key = S3Service::store_document(self, user_id, document_id, filename, data).await?;
        Ok(format!("{}{}", keys::STORAGE_PATH_SCHEME, key))
    }

    async fn store_thumbnail(
        &self,
        user_id: Uuid,
        document_id: Uuid,
        data: &[u8],
    ) -> Result<String> {
        let key = S3Service::store_thumbnail(self, user_id, document_id, data).await?;
        Ok(format!("{}{}", keys::STORAGE_PATH_SCHEME, key))
    }

    async fn store_processed_image(
        &self,
        user_id: Uuid,
        document_id: Uuid,
        data: &[u8],
    ) -> Result<String> {
        let key = S3Service::store_processed_image(self, user_id, document_id, data).await?;
        Ok(format!("{}{}", keys::STORAGE_PATH_SCHEME, key))
    }

    async fn retrieve_file(&self, path: &str) -> Result<Vec<u8>> {
        // A recorded `s3://<key>` path names the key verbatim (prefix
        // included); a bare key is accepted as-is.
        let key = keys::key_from_stored_path(path).unwrap_or(path);

        #[cfg(not(feature = "s3"))]
        {
            return Err(anyhow!("S3 support not compiled in"));
        }

        #[cfg(feature = "s3")]
        {
            info!(
                "Retrieving file from S3: {}/{}",
                self.config.bucket_name, key
            );
            let bytes = self.fetch_object_with_retry("retrieve_file", key).await?;
            info!(
                "Successfully retrieved file: {} ({} bytes)",
                key,
                bytes.len()
            );
            Ok(bytes)
        }
    }

    async fn delete_document_files(
        &self,
        user_id: Uuid,
        document_id: Uuid,
        file_path: &str,
    ) -> Result<()> {
        // The document object is deleted by the key recorded at upload; the
        // key embeds the upload month, so recomputing it would miss every
        // document not uploaded in the current month.
        let mut targets = Vec::with_capacity(5);
        match keys::key_from_stored_path(file_path) {
            Some(key) => targets.push(("Document", key.to_string())),
            None => info!(
                "File {} of document {} is not an S3 storage path; nothing to delete from the bucket",
                file_path, document_id
            ),
        }
        targets.extend(keys::derived_object_keys(
            self.config.prefix.as_deref(),
            user_id,
            document_id,
        ));

        let mut errors = Vec::new();
        for (label, key) in &targets {
            if let Err(e) = self.delete_object(key).await {
                errors.push(format!("{}: {}", label, e));
            }
        }
        if !errors.is_empty() {
            return Err(anyhow!(
                "Failed to delete some files: {}",
                errors.join("; ")
            ));
        }

        info!(
            "Successfully deleted all files for document {}",
            document_id
        );
        Ok(())
    }

    async fn file_exists(&self, path: &str) -> Result<bool> {
        // A recorded `s3://<key>` path names the key verbatim (prefix
        // included); a bare key is accepted as-is.
        let key = keys::key_from_stored_path(path).unwrap_or(path);

        #[cfg(not(feature = "s3"))]
        {
            return Err(anyhow!("S3 support not compiled in"));
        }

        #[cfg(feature = "s3")]
        {
            match self
                .client
                .head_object()
                .bucket(&self.config.bucket_name)
                .key(key)
                .send()
                .await
            {
                Ok(_) => Ok(true),
                // HeadObject reports a missing key as a NotFound service error.
                Err(e) if e.as_service_error().map(|s| s.is_not_found()) == Some(true) => Ok(false),
                Err(e) => Err(anyhow!(
                    "Failed to check file existence {}: {}",
                    key,
                    describe_error(&e)
                )),
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
