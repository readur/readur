//! Storage backend abstraction for document management
//! 
//! This module provides a clean abstraction over different storage backends
//! (local filesystem, S3, etc.) with a unified interface.

use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

pub mod local;
pub mod factory;

/// Core storage backend trait that all storage implementations must implement
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Support for downcasting to concrete types (for backward compatibility)
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        None // Default implementation returns None
    }
    /// Store a document file
    /// Returns the storage path/key where the document was stored
    async fn store_document(&self, user_id: Uuid, document_id: Uuid, filename: &str, data: &[u8]) -> Result<String>;
    
    /// Store a thumbnail image
    /// Returns the storage path/key where the thumbnail was stored
    async fn store_thumbnail(&self, user_id: Uuid, document_id: Uuid, data: &[u8]) -> Result<String>;
    
    /// Store a processed image (e.g., OCR processed image)
    /// Returns the storage path/key where the processed image was stored
    async fn store_processed_image(&self, user_id: Uuid, document_id: Uuid, data: &[u8]) -> Result<String>;
    
    /// Retrieve file data by storage path/key
    async fn retrieve_file(&self, path: &str) -> Result<Vec<u8>>;
    
    /// Delete all files associated with a document (document, thumbnail, processed image)
    async fn delete_document_files(&self, user_id: Uuid, document_id: Uuid, filename: &str) -> Result<()>;
    
    /// Check if a file exists at the given path/key
    async fn file_exists(&self, path: &str) -> Result<bool>;
    
    /// Get a human-readable identifier for this storage backend type
    fn storage_type(&self) -> &'static str;
    
    /// Initialize the storage backend (create directories, validate access, etc.)
    async fn initialize(&self) -> Result<()>;
}

/// Storage configuration enum for different backend types
#[derive(Debug, Clone)]
pub enum StorageConfig {
    /// Local filesystem storage
    Local {
        upload_path: String,
    },
    /// S3-compatible storage
    #[cfg(feature = "s3")]
    S3 {
        s3_config: crate::models::S3SourceConfig,
        /// Optional local fallback path for hybrid scenarios
        fallback_path: Option<String>,
    },
}