//! Local filesystem storage backend implementation

use anyhow::Result;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{info, error, warn, debug};
use uuid::Uuid;

use super::StorageBackend;
use crate::utils::security::{validate_filename, validate_and_sanitize_path, validate_path_within_base};

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
                        debug!("Cache hit for file path: {} -> {}", file_path, resolved_path);
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
            let documents_path = Path::new(&self.upload_path).join("documents").join(file_path);
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
            return Err(anyhow::anyhow!("Failed to create documents directory: {}", e));
        }
        
        fs::write(&file_path, data).await?;
        
        Ok(file_path.to_string_lossy().to_string())
    }
}

#[async_trait]
impl StorageBackend for LocalStorageBackend {
    async fn store_document(&self, _user_id: Uuid, document_id: Uuid, filename: &str, data: &[u8]) -> Result<String> {
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
        validate_path_within_base(
            &file_path.to_string_lossy(), 
            &self.upload_path
        )?;
        
        // Ensure the documents directory exists
        fs::create_dir_all(&documents_dir).await?;
        
        // Validate data size (prevent extremely large files from causing issues)
        if data.len() > 1_000_000_000 { // 1GB limit
            return Err(anyhow::anyhow!("File too large for storage (max 1GB)"));
        }
        
        fs::write(&file_path, data).await?;
        
        // Invalidate any cached negative results for this path
        let path_str = file_path.to_string_lossy().to_string();
        self.invalidate_cache_entry(&path_str).await;
        
        info!("Stored document locally: {}", file_path.display());
        Ok(path_str)
    }

    async fn store_thumbnail(&self, _user_id: Uuid, document_id: Uuid, data: &[u8]) -> Result<String> {
        let thumbnails_dir = self.get_thumbnails_path();
        fs::create_dir_all(&thumbnails_dir).await?;

        let thumbnail_filename = format!("{}_thumb.jpg", document_id);
        let thumbnail_path = thumbnails_dir.join(&thumbnail_filename);
        
        // Validate that the final path is within our base directory
        validate_path_within_base(
            &thumbnail_path.to_string_lossy(), 
            &self.upload_path
        )?;
        
        // Validate data size for thumbnails (should be much smaller)
        if data.len() > 10_000_000 { // 10MB limit for thumbnails
            return Err(anyhow::anyhow!("Thumbnail too large (max 10MB)"));
        }
        
        fs::write(&thumbnail_path, data).await?;
        
        // Invalidate any cached negative results for this path
        let path_str = thumbnail_path.to_string_lossy().to_string();
        self.invalidate_cache_entry(&path_str).await;
        
        info!("Stored thumbnail locally: {}", thumbnail_path.display());
        Ok(path_str)
    }

    async fn store_processed_image(&self, _user_id: Uuid, document_id: Uuid, data: &[u8]) -> Result<String> {
        let processed_dir = self.get_processed_images_path();
        fs::create_dir_all(&processed_dir).await?;

        let processed_filename = format!("{}_processed.png", document_id);
        let processed_path = processed_dir.join(&processed_filename);
        
        // Validate that the final path is within our base directory
        validate_path_within_base(
            &processed_path.to_string_lossy(), 
            &self.upload_path
        )?;
        
        // Validate data size for processed images
        if data.len() > 50_000_000 { // 50MB limit for processed images
            return Err(anyhow::anyhow!("Processed image too large (max 50MB)"));
        }
        
        fs::write(&processed_path, data).await?;
        
        // Invalidate any cached negative results for this path
        let path_str = processed_path.to_string_lossy().to_string();
        self.invalidate_cache_entry(&path_str).await;
        
        info!("Stored processed image locally: {}", processed_path.display());
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
        if data.len() > 1_000_000_000 { // 1GB limit
            warn!("Attempted to read extremely large file: {} ({} bytes)", resolved_path, data.len());
            return Err(anyhow::anyhow!("File too large to read safely"));
        }
        
        Ok(data)
    }

    async fn delete_document_files(&self, _user_id: Uuid, document_id: Uuid, filename: &str) -> Result<()> {
        let mut deleted_files = Vec::new();
        let mut serious_errors = Vec::new();

        // Helper function to safely delete a file
        let storage_backend = self;
        async fn safe_delete(path: &Path, serious_errors: &mut Vec<String>, backend: &LocalStorageBackend) -> Option<String> {
            match fs::remove_file(path).await {
                Ok(_) => {
                    info!("Deleted file: {}", path.display());
                    let path_str = path.to_string_lossy().to_string();
                    
                    // Invalidate cache entry for the deleted file
                    backend.invalidate_cache_entry(&path_str).await;
                    
                    Some(path_str)
                }
                Err(e) => {
                    match e.kind() {
                        std::io::ErrorKind::NotFound => {
                            info!("File already deleted: {}", path.display());
                            // Still invalidate cache in case it was cached as existing
                            let path_str = path.to_string_lossy().to_string();
                            backend.invalidate_cache_entry(&path_str).await;
                            None
                        }
                        _ => {
                            serious_errors.push(format!("Failed to delete file {}: {}", path.display(), e));
                            None
                        }
                    }
                }
            }
        }

        // Try multiple strategies to find and delete the main document file
        let extension = Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        
        // Strategy 1: Try document ID-based filename (new structured approach)
        let document_filename = if extension.is_empty() {
            document_id.to_string()
        } else {
            format!("{}.{}", document_id, extension)
        };
        let main_file_structured = self.get_documents_path().join(&document_filename);
        
        // Strategy 2: Try original filename in documents directory
        let main_file_original = self.get_documents_path().join(filename);
        
        // Strategy 3: Try in the base upload directory (legacy flat structure)
        let main_file_legacy = Path::new(&self.upload_path).join(filename);
        
        // Try to delete main document file using all strategies
        let main_file_candidates = [
            &main_file_structured,
            &main_file_original,  
            &main_file_legacy,
        ];
        
        let mut main_file_deleted = false;
        for candidate_path in &main_file_candidates {
            if candidate_path.exists() {
                if let Some(deleted_path) = safe_delete(candidate_path, &mut serious_errors, storage_backend).await {
                    deleted_files.push(deleted_path);
                    main_file_deleted = true;
                    break; // Only delete the first match we find
                }
            }
        }
        
        if !main_file_deleted {
            info!("Main document file not found in any expected location for document {}", document_id);
        }

        // Delete thumbnail if it exists
        let thumbnail_filename = format!("{}_thumb.jpg", document_id);
        let thumbnail_path = self.get_thumbnails_path().join(&thumbnail_filename);
        if let Some(deleted_path) = safe_delete(&thumbnail_path, &mut serious_errors, storage_backend).await {
            deleted_files.push(deleted_path);
        }

        // Delete processed image if it exists
        let processed_image_filename = format!("{}_processed.png", document_id);
        let processed_image_path = self.get_processed_images_path().join(&processed_image_filename);
        if let Some(deleted_path) = safe_delete(&processed_image_path, &mut serious_errors, storage_backend).await {
            deleted_files.push(deleted_path);
        }

        // Only fail if there were serious errors (not "file not found")
        if !serious_errors.is_empty() {
            error!("Serious errors occurred while deleting files for document {}: {}", document_id, serious_errors.join("; "));
            return Err(anyhow::anyhow!("File deletion errors: {}", serious_errors.join("; ")));
        }

        if deleted_files.is_empty() {
            info!("No files needed deletion for document {} (all files already removed)", document_id);
        } else {
            info!("Successfully deleted {} files for document {}", deleted_files.len(), document_id);
        }

        Ok(())
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
            "temp",            // Temporary files during processing
            "backups",         // Document backups
        ];
        
        for dir in directories.iter() {
            let dir_path = base_path.join(dir);
            if let Err(e) = fs::create_dir_all(&dir_path).await {
                error!("Failed to create directory {:?}: {}", dir_path, e);
                return Err(anyhow::anyhow!("Failed to create directory structure: {}", e));
            }
            info!("Ensured directory exists: {:?}", dir_path);
        }
        
        Ok(())
    }
}