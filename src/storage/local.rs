//! Local filesystem storage backend implementation

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::StorageBackend;
use crate::utils::security::{
    validate_and_sanitize_path, validate_filename, validate_path_within_base,
};

/// Local filesystem storage backend
pub struct LocalStorageBackend {
    upload_path: String,
    /// Cache for resolved file paths to reduce filesystem calls
    path_cache: Arc<RwLock<HashMap<String, Option<String>>>>,
}

impl LocalStorageBackend {
    /// Create a new local storage backend
    pub fn new(upload_path: String) -> Self {
        Self {
            upload_path,
            path_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the base upload path
    pub fn get_upload_path(&self) -> &str {
        &self.upload_path
    }

    /// Get path for documents subdirectory
    pub fn get_documents_path(&self) -> PathBuf {
        Path::new(&self.upload_path).join("documents")
    }

    /// Get path for thumbnails subdirectory
    pub fn get_thumbnails_path(&self) -> PathBuf {
        Path::new(&self.upload_path).join("thumbnails")
    }

    /// Get path for processed images subdirectory
    pub fn get_processed_images_path(&self) -> PathBuf {
        Path::new(&self.upload_path).join("processed_images")
    }

    /// Get path for temporary files subdirectory
    pub fn get_temp_path(&self) -> PathBuf {
        Path::new(&self.upload_path).join("temp")
    }

    /// Get path for backups subdirectory
    pub fn get_backups_path(&self) -> PathBuf {
        Path::new(&self.upload_path).join("backups")
    }

    /// Resolve file path, handling both old and new directory structures with caching
    pub async fn resolve_file_path(&self, file_path: &str) -> Result<String> {
        // Check cache first
        {
            let cache = self.path_cache.read().await;
            if let Some(cached_result) = cache.get(file_path) {
                return match cached_result {
                    Some(resolved_path) => {
                        debug!(
                            "Cache hit for file path: {} -> {}",
                            file_path, resolved_path
                        );
                        Ok(resolved_path.clone())
                    }
                    None => {
                        debug!("Cache hit for non-existent file: {}", file_path);
                        Err(anyhow::anyhow!("File not found: {} (cached)", file_path))
                    }
                };
            }
        }

        // Generate candidate paths in order of likelihood
        let candidates = self.generate_path_candidates(file_path);

        // Check candidates efficiently
        let mut found_path = None;
        for candidate in &candidates {
            match tokio::fs::metadata(candidate).await {
                Ok(metadata) => {
                    if metadata.is_file() {
                        found_path = Some(candidate.clone());
                        debug!("Found file at: {}", candidate);
                        break;
                    }
                }
                Err(_) => {
                    // File doesn't exist at this path, continue to next candidate
                    debug!("File not found at: {}", candidate);
                }
            }
        }

        // Cache the result
        {
            let mut cache = self.path_cache.write().await;
            cache.insert(file_path.to_string(), found_path.clone());

            // Prevent cache from growing too large
            if cache.len() > 10000 {
                // Clear oldest 20% of entries (simple cache eviction)
                let to_remove: Vec<String> = cache.keys().take(2000).cloned().collect();
                for key in to_remove {
                    cache.remove(&key);
                }
                debug!("Evicted cache entries to prevent memory growth");
            }
        }

        match found_path {
            Some(path) => {
                if path != file_path {
                    info!("Resolved file path: {} -> {}", file_path, path);
                }
                Ok(path)
            }
            None => {
                debug!("File not found in any candidate location: {}", file_path);
                Err(anyhow::anyhow!(
                    "File not found: {} (checked {} locations)",
                    file_path,
                    candidates.len()
                ))
            }
        }
    }

    /// Generate candidate paths for file resolution
    fn generate_path_candidates(&self, file_path: &str) -> Vec<String> {
        let mut candidates = Vec::new();

        // 1. Original path (most likely for new files)
        candidates.push(file_path.to_string());

        // 2. For legacy compatibility - try structured directory
        if file_path.starts_with("./uploads/") && !file_path.contains("/documents/") {
            candidates.push(file_path.replace("./uploads/", "./uploads/documents/"));
        }

        // 3. Try without ./ prefix in structured directory
        if file_path.starts_with("uploads/") && !file_path.contains("/documents/") {
            candidates.push(file_path.replace("uploads/", "uploads/documents/"));
        }

        // 4. Try relative to our configured upload path
        if !file_path.starts_with(&self.upload_path) {
            let relative_path = Path::new(&self.upload_path).join(file_path);
            candidates.push(relative_path.to_string_lossy().to_string());

            // Also try in documents subdirectory
            let documents_path = Path::new(&self.upload_path)
                .join("documents")
                .join(file_path);
            candidates.push(documents_path.to_string_lossy().to_string());
        }

        // 5. Try absolute path if it looks like a filename only
        if !file_path.contains('/') && !file_path.contains('\\') {
            // Try in documents directory
            let abs_documents_path = Path::new(&self.upload_path)
                .join("documents")
                .join(file_path);
            candidates.push(abs_documents_path.to_string_lossy().to_string());
        }

        candidates
    }

    /// Clear the path resolution cache (useful for testing or after file operations)
    pub async fn clear_path_cache(&self) {
        let mut cache = self.path_cache.write().await;
        cache.clear();
        debug!("Cleared path resolution cache");
    }

    /// Invalidate cache entry for a specific path
    pub async fn invalidate_cache_entry(&self, file_path: &str) {
        let mut cache = self.path_cache.write().await;
        cache.remove(file_path);
        debug!("Invalidated cache entry for: {}", file_path);
    }

    /// Delete the document file recorded at `file_path`. A recorded
    /// `s3://<key>` path is resolved below the upload directory, as
    /// `FileService::read_file` does. Resolution falls back to the legacy
    /// layouts; a path that resolves outside the upload directory is refused.
    async fn delete_stored_file(
        &self,
        document_id: Uuid,
        file_path: &str,
        report: &mut DeletionReport,
    ) {
        let recorded = file_path.strip_prefix("s3://").unwrap_or(file_path);
        let sanitized = match validate_and_sanitize_path(recorded) {
            Ok(path) => path,
            Err(e) => {
                report.errors.push(format!(
                    "Refusing to delete document file {}: {}",
                    file_path, e
                ));
                return;
            }
        };
        let resolved = match self.resolve_file_path(&sanitized).await {
            Ok(path) => path,
            Err(_) => {
                info!(
                    "Document file {} for document {} is already gone",
                    file_path, document_id
                );
                return;
            }
        };
        if let Err(e) = validate_path_within_base(&resolved, &self.upload_path) {
            report.errors.push(format!(
                "Refusing to delete document file {} outside the upload directory: {}",
                resolved, e
            ));
            return;
        }
        self.invalidate_cache_entry(&sanitized).await;
        self.delete_path(Path::new(&resolved), report).await;
    }

    /// Thumbnails and OCR review images derived from a document. Their paths
    /// are not recorded, so every name a writer uses is covered:
    /// `<id>_thumb.jpg` and `<stem of the stored file>_thumb.jpg` (the
    /// thumbnail cache keys by file stem), `<id>_processed.png`, and the OCR
    /// queue's timestamped `<id>_processed_<ts>.<ext>` review copies.
    async fn derived_file_paths(
        &self,
        document_id: Uuid,
        file_path: &str,
        report: &mut DeletionReport,
    ) -> Vec<PathBuf> {
        let thumbnails = self.get_thumbnails_path();
        let mut paths = vec![thumbnails.join(format!("{}_thumb.jpg", document_id))];
        if let Some(stem) = Path::new(file_path).file_stem().and_then(|s| s.to_str()) {
            if !stem.is_empty() && stem != document_id.to_string() {
                paths.push(thumbnails.join(format!("{}_thumb.jpg", stem)));
            }
        }

        let processed_dir = self.get_processed_images_path();
        let processed_prefix = format!("{}_processed", document_id);
        match fs::read_dir(&processed_dir).await {
            Ok(mut entries) => loop {
                match entries.next_entry().await {
                    Ok(Some(entry)) => {
                        if entry
                            .file_name()
                            .to_string_lossy()
                            .starts_with(&processed_prefix)
                        {
                            paths.push(entry.path());
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        report.errors.push(format!(
                            "Failed to list {}: {}",
                            processed_dir.display(),
                            e
                        ));
                        break;
                    }
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                report
                    .errors
                    .push(format!("Failed to list {}: {}", processed_dir.display(), e))
            }
        }
        paths
    }

    /// Delete one file; a file that is already gone is not an error.
    async fn delete_path(&self, path: &Path, report: &mut DeletionReport) {
        let path_str = path.to_string_lossy().to_string();
        match fs::remove_file(path).await {
            Ok(()) => {
                info!("Deleted file: {}", path.display());
                report.deleted += 1;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                debug!("File already deleted: {}", path.display());
            }
            Err(e) => {
                report
                    .errors
                    .push(format!("Failed to delete file {}: {}", path.display(), e))
            }
        }
        self.invalidate_cache_entry(&path_str).await;
    }

    /// Save a file with generated UUID filename (legacy method)
    pub async fn save_file(&self, filename: &str, data: &[u8]) -> Result<String> {
        let file_id = Uuid::new_v4();
        let extension = Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        let saved_filename = if extension.is_empty() {
            file_id.to_string()
        } else {
            format!("{}.{}", file_id, extension)
        };

        // Save to documents subdirectory
        let documents_dir = self.get_documents_path();
        let file_path = documents_dir.join(&saved_filename);

        // Ensure the documents directory exists
        if let Err(e) = fs::create_dir_all(&documents_dir).await {
            error!("Failed to create documents directory: {}", e);
            return Err(anyhow::anyhow!(
                "Failed to create documents directory: {}",
                e
            ));
        }

        fs::write(&file_path, data).await?;

        Ok(file_path.to_string_lossy().to_string())
    }
}

/// Outcome of deleting a document's files: what was removed and what failed.
#[derive(Default)]
struct DeletionReport {
    deleted: usize,
    errors: Vec<String>,
}

impl DeletionReport {
    fn finish(self, document_id: Uuid) -> Result<()> {
        if !self.errors.is_empty() {
            error!(
                "Errors occurred while deleting files for document {}: {}",
                document_id,
                self.errors.join("; ")
            );
            return Err(anyhow::anyhow!(
                "File deletion errors: {}",
                self.errors.join("; ")
            ));
        }
        if self.deleted == 0 {
            info!(
                "No files needed deletion for document {} (all files already removed)",
                document_id
            );
        } else {
            info!(
                "Successfully deleted {} files for document {}",
                self.deleted, document_id
            );
        }
        Ok(())
    }
}

#[async_trait]
impl StorageBackend for LocalStorageBackend {
    async fn store_document(
        &self,
        _user_id: Uuid,
        document_id: Uuid,
        filename: &str,
        data: &[u8],
    ) -> Result<String> {
        // Validate and sanitize the filename
        let sanitized_filename = validate_filename(filename)?;

        let extension = Path::new(&sanitized_filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        let document_filename = if extension.is_empty() {
            document_id.to_string()
        } else {
            format!("{}.{}", document_id, extension)
        };

        let documents_dir = self.get_documents_path();
        let file_path = documents_dir.join(&document_filename);

        // Validate that the final path is within our base directory
        validate_path_within_base(&file_path.to_string_lossy(), &self.upload_path)?;

        // Ensure the documents directory exists
        fs::create_dir_all(&documents_dir).await?;

        // Validate data size (prevent extremely large files from causing issues)
        if data.len() > 1_000_000_000 {
            // 1GB limit
            return Err(anyhow::anyhow!("File too large for storage (max 1GB)"));
        }

        fs::write(&file_path, data).await?;

        // Invalidate any cached negative results for this path
        let path_str = file_path.to_string_lossy().to_string();
        self.invalidate_cache_entry(&path_str).await;

        info!("Stored document locally: {}", file_path.display());
        Ok(path_str)
    }

    async fn store_thumbnail(
        &self,
        _user_id: Uuid,
        document_id: Uuid,
        data: &[u8],
    ) -> Result<String> {
        let thumbnails_dir = self.get_thumbnails_path();
        fs::create_dir_all(&thumbnails_dir).await?;

        let thumbnail_filename = format!("{}_thumb.jpg", document_id);
        let thumbnail_path = thumbnails_dir.join(&thumbnail_filename);

        // Validate that the final path is within our base directory
        validate_path_within_base(&thumbnail_path.to_string_lossy(), &self.upload_path)?;

        // Validate data size for thumbnails (should be much smaller)
        if data.len() > 10_000_000 {
            // 10MB limit for thumbnails
            return Err(anyhow::anyhow!("Thumbnail too large (max 10MB)"));
        }

        fs::write(&thumbnail_path, data).await?;

        // Invalidate any cached negative results for this path
        let path_str = thumbnail_path.to_string_lossy().to_string();
        self.invalidate_cache_entry(&path_str).await;

        info!("Stored thumbnail locally: {}", thumbnail_path.display());
        Ok(path_str)
    }

    async fn store_processed_image(
        &self,
        _user_id: Uuid,
        document_id: Uuid,
        data: &[u8],
    ) -> Result<String> {
        let processed_dir = self.get_processed_images_path();
        fs::create_dir_all(&processed_dir).await?;

        let processed_filename = format!("{}_processed.png", document_id);
        let processed_path = processed_dir.join(&processed_filename);

        // Validate that the final path is within our base directory
        validate_path_within_base(&processed_path.to_string_lossy(), &self.upload_path)?;

        // Validate data size for processed images
        if data.len() > 50_000_000 {
            // 50MB limit for processed images
            return Err(anyhow::anyhow!("Processed image too large (max 50MB)"));
        }

        fs::write(&processed_path, data).await?;

        // Invalidate any cached negative results for this path
        let path_str = processed_path.to_string_lossy().to_string();
        self.invalidate_cache_entry(&path_str).await;

        info!(
            "Stored processed image locally: {}",
            processed_path.display()
        );
        Ok(path_str)
    }

    async fn retrieve_file(&self, path: &str) -> Result<Vec<u8>> {
        // Validate and sanitize the input path
        let sanitized_path = validate_and_sanitize_path(path)?;

        let resolved_path = self.resolve_file_path(&sanitized_path).await?;

        // Validate that the resolved path is within our base directory
        validate_path_within_base(&resolved_path, &self.upload_path)?;

        let data = fs::read(&resolved_path).await?;

        // Additional safety check on file size when reading
        if data.len() > 1_000_000_000 {
            // 1GB limit
            warn!(
                "Attempted to read extremely large file: {} ({} bytes)",
                resolved_path,
                data.len()
            );
            return Err(anyhow::anyhow!("File too large to read safely"));
        }

        Ok(data)
    }

    async fn delete_document_files(
        &self,
        _user_id: Uuid,
        document_id: Uuid,
        file_path: &str,
    ) -> Result<()> {
        let mut report = DeletionReport::default();

        // The document file is deleted at the path recorded for it, never at
        // a path guessed from its id or filename (which could name a file
        // that belongs to another document, and misses renamed layouts).
        self.delete_stored_file(document_id, file_path, &mut report)
            .await;
        for path in self
            .derived_file_paths(document_id, file_path, &mut report)
            .await
        {
            self.delete_path(&path, &mut report).await;
        }

        report.finish(document_id)
    }

    async fn file_exists(&self, path: &str) -> Result<bool> {
        // Validate and sanitize the input path
        let sanitized_path = match validate_and_sanitize_path(path) {
            Ok(p) => p,
            Err(_) => return Ok(false), // Invalid paths don't exist
        };

        match self.resolve_file_path(&sanitized_path).await {
            Ok(resolved_path) => {
                // Additional validation that the resolved path is within base directory
                match validate_path_within_base(&resolved_path, &self.upload_path) {
                    Ok(_) => Ok(true),
                    Err(_) => Ok(false), // Paths outside base directory don't "exist" for us
                }
            }
            Err(_) => Ok(false),
        }
    }

    fn storage_type(&self) -> &'static str {
        "local"
    }

    async fn initialize(&self) -> Result<()> {
        let base_path = Path::new(&self.upload_path);

        // Create subdirectories for organized file storage
        let directories = [
            "documents",        // Final uploaded documents
            "thumbnails",       // Document thumbnails
            "processed_images", // OCR processed images for review
            "temp",             // Temporary files during processing
            "backups",          // Document backups
        ];

        for dir in directories.iter() {
            let dir_path = base_path.join(dir);
            if let Err(e) = fs::create_dir_all(&dir_path).await {
                error!("Failed to create directory {:?}: {}", dir_path, e);
                return Err(anyhow::anyhow!(
                    "Failed to create directory structure: {}",
                    e
                ));
            }
            info!("Ensured directory exists: {:?}", dir_path);
        }

        Ok(())
    }
}
