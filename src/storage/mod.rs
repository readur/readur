//! Storage backend abstraction for document management
//!
//! This module provides a clean abstraction over different storage backends
//! (local filesystem, S3, etc.) with a unified interface.

use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

pub mod factory;
pub mod local;

/// Core storage backend trait that all storage implementations must implement
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Support for downcasting to concrete types (for backward compatibility)
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        None // Default implementation returns None
    }
    /// Store a document file
    /// Returns the storage path/key where the document was stored
    async fn store_document(
        &self,
        user_id: Uuid,
        document_id: Uuid,
        filename: &str,
        data: &[u8],
    ) -> Result<String>;

    /// Store a thumbnail image
    /// Returns the storage path/key where the thumbnail was stored
    async fn store_thumbnail(
        &self,
        user_id: Uuid,
        document_id: Uuid,
        data: &[u8],
    ) -> Result<String>;

    /// Store a processed image (e.g., OCR processed image)
    /// Returns the storage path/key where the processed image was stored
    async fn store_processed_image(
        &self,
        user_id: Uuid,
        document_id: Uuid,
        data: &[u8],
    ) -> Result<String>;

    /// Retrieve file data by storage path/key
    async fn retrieve_file(&self, path: &str) -> Result<Vec<u8>>;

    /// Retrieve file data by storage path/key, reading at most
    /// `max_bytes + 1` bytes: a file longer than `max_bytes` is reported as
    /// [`LimitedRead::TooLarge`] without being loaded. Use this wherever a
    /// size limit must bound memory: the size recorded at upload describes
    /// the received bytes, and the stored file can differ (e.g. an image
    /// re-encoded on rotation).
    ///
    /// Deliberately has no default: an implementation built on
    /// `retrieve_file` would read the whole file and bound nothing.
    async fn retrieve_file_limited(&self, path: &str, max_bytes: u64) -> Result<LimitedRead>;

    /// Delete all files associated with a document (document, thumbnail, processed image).
    ///
    /// `file_path` is the storage path recorded for the document at upload
    /// (`documents.file_path`, as returned by [`store_document`](Self::store_document)).
    /// The document file is deleted at exactly that path: backends must never
    /// recompute it, since the layout may embed upload-time data (the S3 key
    /// carries the upload month) or have changed since. A file that is
    /// already gone is not an error; any other failure is.
    async fn delete_document_files(
        &self,
        user_id: Uuid,
        document_id: Uuid,
        file_path: &str,
    ) -> Result<()>;

    /// Check if a file exists at the given path/key
    async fn file_exists(&self, path: &str) -> Result<bool>;

    /// Get a human-readable identifier for this storage backend type
    fn storage_type(&self) -> &'static str;

    /// Initialize the storage backend (create directories, validate access, etc.)
    async fn initialize(&self) -> Result<()>;
}

/// Outcome of a size-bounded read ([`StorageBackend::retrieve_file_limited`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LimitedRead {
    /// The whole file, at most `max_bytes` long.
    Within(Vec<u8>),
    /// The file is longer than `max_bytes`; only enough of it was read to
    /// tell, so its exact size is not known.
    TooLarge,
}

impl LimitedRead {
    /// Classify a buffer holding at most `max_bytes + 1` leading bytes of a
    /// file: more than `max_bytes` means the file is over the limit.
    pub fn from_prefix(bytes: Vec<u8>, max_bytes: u64) -> Self {
        if bytes.len() as u64 > max_bytes {
            Self::TooLarge
        } else {
            Self::Within(bytes)
        }
    }
}

/// Read `reader` to its end or to `max_bytes + 1` bytes, whichever comes
/// first, and classify the result (see [`LimitedRead::from_prefix`]).
pub async fn read_limited<R>(reader: R, max_bytes: u64) -> std::io::Result<LimitedRead>
where
    R: tokio::io::AsyncRead + Unpin,
{
    use tokio::io::AsyncReadExt;

    let mut buf = Vec::new();
    reader
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut buf)
        .await?;
    Ok(LimitedRead::from_prefix(buf, max_bytes))
}

/// Storage configuration enum for different backend types
#[derive(Debug, Clone)]
pub enum StorageConfig {
    /// Local filesystem storage
    Local { upload_path: String },
    /// S3-compatible storage
    #[cfg(feature = "s3")]
    S3 {
        s3_config: crate::models::S3SourceConfig,
        /// Optional local fallback path for hybrid scenarios
        fallback_path: Option<String>,
    },
}
