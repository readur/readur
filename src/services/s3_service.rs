//! S3 client used both as a sync source (listing and downloading a bucket's
//! files) and as a storage backend. Split across sibling files:
//! `s3_service_keys.rs` (object-key layout), `s3_service_upload.rs` (writes),
//! `s3_service_backend.rs` (deletes and the `StorageBackend` impl) and
//! `s3_service_migration.rs` (paging for the `migrate_to_s3` tool).

use anyhow::{anyhow, Result};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

#[cfg(feature = "s3")]
use crate::services::s3_client::describe_error;
#[cfg(feature = "s3")]
use crate::services::s3_listing::{
    list_objects, listing_prefix, ListingContext, MAX_LISTING_PAGES,
};
#[cfg(feature = "s3")]
use aws_sdk_s3::Client;

use crate::models::{FileIngestionInfo, S3SourceConfig};
use crate::utils::extension_filter::ExtensionFilter;
use crate::utils::sync_cancelled::{cancelled, sleep_unless_cancelled};

#[path = "s3_service_keys.rs"]
mod keys;

#[path = "s3_service_migration.rs"]
pub mod migration;

#[path = "s3_service_upload.rs"]
mod upload;

#[path = "s3_service_backend.rs"]
mod backend;

/// Result of deleting one object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectDeletion {
    /// DeleteObject succeeded. AWS also answers this way for a key that did
    /// not exist; it cannot be told apart.
    Deleted,
    /// The store reported the object missing (`NoSuchKey`, or a bare 404),
    /// so there was nothing to delete.
    AlreadyGone,
}

/// Upper bound on reading one object's body. The SDK's attempt timeout ends
/// once the response headers arrive; the streamed body is read here.
const DOWNLOAD_BODY_TIMEOUT: Duration = Duration::from_secs(600);

/// Wait before the second attempt of a retried S3 operation; doubled per attempt.
const RETRY_BASE_DELAY_MS: u64 = 100;

/// Check that a downloaded body matches the Content-Length the server sent.
pub fn verify_content_length(key: &str, expected: Option<i64>, actual: usize) -> Result<()> {
    match expected {
        Some(expected) if expected >= 0 && expected as u64 != actual as u64 => Err(anyhow!(
            "S3 object {} download incomplete: expected {} bytes, received {}",
            key,
            expected,
            actual
        )),
        _ => Ok(()),
    }
}

/// The `Range` header value asking for an object's first `max_bytes + 1`
/// bytes (the end offset is inclusive). Getting more than `max_bytes` back
/// is how a bounded read tells the object is over the limit.
pub fn prefix_range(max_bytes: u64) -> String {
    format!("bytes=0-{max_bytes}")
}

/// One GetObject attempt: fetch (the whole object, or only `range`), read
/// the body within a bound, and verify the length. Retrying is the caller's
/// job. A ranged request the store refuses as unsatisfiable means an empty
/// object (every range sent starts at byte 0), so it yields no bytes.
#[cfg(feature = "s3")]
async fn fetch_object(
    client: &Client,
    bucket: &str,
    key: &str,
    range: Option<&str>,
) -> Result<Vec<u8>> {
    let request = client
        .get_object()
        .bucket(bucket)
        .key(key)
        .set_range(range.map(str::to_string));
    let response = match request.send().await {
        Ok(response) => response,
        Err(e) if range.is_some() && keys::is_unsatisfiable_range(&e) => return Ok(Vec::new()),
        Err(e) => {
            return Err(anyhow!(
                "Failed to download S3 object {}: {}",
                key,
                describe_error(&e)
            ))
        }
    };

    let expected = response.content_length();
    let body = tokio::time::timeout(DOWNLOAD_BODY_TIMEOUT, response.body.collect())
        .await
        .map_err(|_| {
            anyhow!(
                "Timed out reading S3 object {} after {:?}",
                key,
                DOWNLOAD_BODY_TIMEOUT
            )
        })?
        .map_err(|e| {
            anyhow!(
                "Failed to read S3 object body {}: {}",
                key,
                describe_error(&e)
            )
        })?;

    let bytes = body.into_bytes().to_vec();
    verify_content_length(key, expected, bytes.len())?;
    Ok(bytes)
}

#[derive(Debug, Clone)]
pub struct S3Service {
    #[cfg(feature = "s3")]
    client: Client,
    config: S3SourceConfig,
    /// Normalised form of `config.file_extensions`; empty allows every key.
    extension_filter: ExtensionFilter,
    /// Checked before each listing page and each download attempt.
    cancellation_token: Option<CancellationToken>,
    /// Wait before the second attempt of an operation; doubled per attempt.
    retry_base_delay: Duration,
}

impl S3Service {
    pub async fn new(config: S3SourceConfig) -> Result<Self> {
        #[cfg(not(feature = "s3"))]
        {
            return Err(anyhow!(
                "S3 support not compiled in. Enable the 's3' feature to use S3 sources."
            ));
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

            let client = crate::services::s3_client::build_client(&config).await;

            let extension_filter = ExtensionFilter::new(&config.file_extensions);
            Ok(Self {
                #[cfg(feature = "s3")]
                client,
                config,
                extension_filter,
                cancellation_token: None,
                retry_base_delay: Duration::from_millis(RETRY_BASE_DELAY_MS),
            })
        }
    }

    /// Attach a token that stops listing (between pages) and downloads
    /// (before each attempt) once cancelled.
    pub fn with_cancellation_token(mut self, token: CancellationToken) -> Self {
        self.cancellation_token = Some(token);
        self
    }

    /// Overrides the retry wait so a test can observe it being cut short.
    #[cfg(test)]
    fn with_retry_base_delay(mut self, delay: Duration) -> Self {
        self.retry_base_delay = delay;
        self
    }

    /// Error when the attached token (if any) has been cancelled.
    fn check_cancelled(&self, what: &str) -> Result<()> {
        match &self.cancellation_token {
            Some(token) if token.is_cancelled() => Err(cancelled(format!("S3 {what}"))),
            _ => Ok(()),
        }
    }

    /// URLs to pass through the outbound destination checks before
    /// connecting; see [`crate::services::s3_client::outbound_urls`].
    pub fn outbound_urls(config: &S3SourceConfig) -> Vec<String> {
        crate::services::s3_client::outbound_urls(config)
    }

    /// Discover files in a specific S3 prefix (folder)
    pub async fn discover_files_in_folder(
        &self,
        folder_path: &str,
    ) -> Result<Vec<FileIngestionInfo>> {
        #[cfg(not(feature = "s3"))]
        {
            return Err(anyhow!("S3 support not compiled in"));
        }

        #[cfg(feature = "s3")]
        {
            // Object keys are returned in full, so the stored relative path is
            // exactly what download_file() expects regardless of the prefix.
            let prefix = listing_prefix(self.config.prefix.as_deref(), folder_path);
            info!(
                "Scanning S3 bucket: {} prefix: '{}'",
                self.config.bucket_name, prefix
            );

            let ctx = ListingContext {
                client: &self.client,
                bucket: &self.config.bucket_name,
                region: &self.config.region,
                filter: &self.extension_filter,
                cancellation: self.cancellation_token.as_ref(),
                max_pages: MAX_LISTING_PAGES,
            };
            let files = list_objects(&ctx, &prefix).await?;

            if files.is_empty()
                && self
                    .config
                    .prefix
                    .as_deref()
                    .is_some_and(|p| !p.trim().trim_matches('/').is_empty())
            {
                warn!(
                "No objects found in S3 bucket {} under '{}' (source prefix '{}' joined with watch folder '{}'); \
                 check that the prefix and the watch folder do not both include the same path",
                self.config.bucket_name, prefix, self.config.prefix.as_deref().unwrap_or(""), folder_path
            );
            }
            info!(
                "Found {} files in S3 bucket {} prefix '{}'",
                files.len(),
                self.config.bucket_name,
                prefix
            );
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
            info!(
                "Downloading S3 object: {}/{}",
                self.config.bucket_name, object_key
            );
            let bytes = self
                .fetch_object_with_retry("download_file", object_key)
                .await?;
            info!(
                "Downloaded S3 object {} ({} bytes)",
                object_key,
                bytes.len()
            );
            Ok(bytes)
        }
    }

    /// GetObject with the service's retry policy, a bounded body read and a
    /// Content-Length check. Used by both the sync download path and the
    /// storage-backend read path.
    #[cfg(feature = "s3")]
    async fn fetch_object_with_retry(&self, operation: &str, key: &str) -> Result<Vec<u8>> {
        self.fetch_object_range_with_retry(operation, key, None)
            .await
    }

    /// [`fetch_object_with_retry`](Self::fetch_object_with_retry), optionally
    /// limited to a byte `range` (a `Range` header value).
    #[cfg(feature = "s3")]
    async fn fetch_object_range_with_retry(
        &self,
        operation: &str,
        key: &str,
        range: Option<String>,
    ) -> Result<Vec<u8>> {
        let key_owned = key.to_string();
        let bucket_name = self.config.bucket_name.clone();
        let client = self.client.clone();

        self.retry_operation(&format!("{}: {}", operation, key), || {
            let key = key_owned.clone();
            let bucket_name = bucket_name.clone();
            let client = client.clone();
            let range = range.clone();
            async move { fetch_object(&client, &bucket_name, &key, range.as_deref()).await }
        })
        .await
    }

    /// Test S3 connection and access to bucket
    pub async fn test_connection(&self) -> Result<String> {
        #[cfg(not(feature = "s3"))]
        {
            return Err(anyhow!("S3 support not compiled in"));
        }

        #[cfg(feature = "s3")]
        {
            info!(
                "Testing S3 connection to bucket: {}",
                self.config.bucket_name
            );

            // Test bucket access by listing objects with a limit
            let response = self
                .client
                .list_objects_v2()
                .bucket(&self.config.bucket_name)
                .max_keys(1)
                .send()
                .await
                .map_err(|e| {
                    anyhow!(
                        "Failed to access S3 bucket {}: {}",
                        self.config.bucket_name,
                        describe_error(&e)
                    )
                })?;

            // Test if we can get bucket region (additional validation)
            let _head_bucket_response = self
                .client
                .head_bucket()
                .bucket(&self.config.bucket_name)
                .send()
                .await
                .map_err(|e| {
                    anyhow!(
                        "Cannot access bucket {}: {}",
                        self.config.bucket_name,
                        describe_error(&e)
                    )
                })?;

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
    pub(crate) fn get_mime_type(extension: &str) -> String {
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
        }
        .to_string()
    }

    pub fn get_config(&self) -> &S3SourceConfig {
        &self.config
    }

    /// Retry wrapper for S3 operations with exponential backoff
    async fn retry_operation<T, F, Fut>(&self, operation_name: &str, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        const MAX_RETRIES: u32 = 3;

        let mut last_error = None;

        for attempt in 0..=MAX_RETRIES {
            self.check_cancelled(operation_name)?;
            match operation().await {
                Ok(result) => {
                    if attempt > 0 {
                        info!(
                            "S3 operation '{}' succeeded after {} retries",
                            operation_name, attempt
                        );
                    }
                    return Ok(result);
                }
                Err(e) => {
                    last_error = Some(e);

                    if attempt < MAX_RETRIES {
                        let delay = self.retry_base_delay * 2u32.pow(attempt);
                        warn!(
                            "S3 operation '{}' failed (attempt {}/{}), retrying in {:?}: {}",
                            operation_name,
                            attempt + 1,
                            MAX_RETRIES + 1,
                            delay,
                            last_error.as_ref().unwrap()
                        );
                        // A stop request ends the wait, and the operation, right away.
                        sleep_unless_cancelled(
                            delay,
                            self.cancellation_token.as_ref(),
                            &format!("S3 {operation_name}"),
                        )
                        .await?;
                    }
                }
            }
        }

        error!(
            "S3 operation '{}' failed after {} attempts: {}",
            operation_name,
            MAX_RETRIES + 1,
            last_error.as_ref().unwrap()
        );
        Err(last_error.unwrap())
    }
}

#[cfg(all(test, feature = "s3"))]
#[path = "s3_service_wiremock_tests.rs"]
mod wiremock_tests;

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
            force_path_style: None,
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
    fn content_length_mismatch_is_an_error() {
        assert!(verify_content_length("k", None, 5).is_ok());
        assert!(verify_content_length("k", Some(5), 5).is_ok());
        assert!(
            verify_content_length("k", Some(-1), 5).is_ok(),
            "unknown length is not checked"
        );
        let err = verify_content_length("k", Some(100), 5)
            .unwrap_err()
            .to_string();
        assert!(err.contains("expected 100 bytes, received 5"), "{err}");
    }

    #[test]
    fn test_mime_type_detection() {
        assert_eq!(S3Service::get_mime_type("pdf"), "application/pdf");
        assert_eq!(S3Service::get_mime_type("jpg"), "image/jpeg");
        assert_eq!(S3Service::get_mime_type("txt"), "text/plain");
        assert_eq!(
            S3Service::get_mime_type("unknown"),
            "application/octet-stream"
        );
    }
}
