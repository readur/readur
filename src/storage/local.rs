//! Local filesystem storage backend implementation

use anyhow::Result;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{info, error};
use uuid::Uuid;

use super::StorageBackend;

/// Local filesystem storage backend
pub struct LocalStorageBackend {
    upload_path: String,
}

impl LocalStorageBackend {
    /// Create a new local storage backend
    pub fn new(upload_path: String) -> Self {
        Self { upload_path }
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

    /// Resolve file path, handling both old and new directory structures
    pub async fn resolve_file_path(&self, file_path: &str) -> Result<String> {
        // If the file exists at the given path, use it
        if Path::new(file_path).exists() {
            return Ok(file_path.to_string());
        }
        
        // Try to find the file in the new structured directory
        if file_path.starts_with("./uploads/") && !file_path.contains("/documents/") {
            let new_path = file_path.replace("./uploads/", "./uploads/documents/");
            if Path::new(&new_path).exists() {
                info!("Found file in new structured directory: {} -> {}", file_path, new_path);
                return Ok(new_path);
            }
        }
        
        // Try without the ./ prefix
        if file_path.starts_with("uploads/") && !file_path.contains("/documents/") {
            let new_path = file_path.replace("uploads/", "uploads/documents/");
            if Path::new(&new_path).exists() {
                info!("Found file in new structured directory: {} -> {}", file_path, new_path);
                return Ok(new_path);
            }
        }
        
        // File not found in any expected location
        Err(anyhow::anyhow!("File not found: {} (checked original path and structured directory)", file_path))
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
        let extension = Path::new(filename)
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
        
        // Ensure the documents directory exists
        fs::create_dir_all(&documents_dir).await?;
        
        fs::write(&file_path, data).await?;
        
        info!("Stored document locally: {}", file_path.display());
        Ok(file_path.to_string_lossy().to_string())
    }

    async fn store_thumbnail(&self, _user_id: Uuid, document_id: Uuid, data: &[u8]) -> Result<String> {
        let thumbnails_dir = self.get_thumbnails_path();
        fs::create_dir_all(&thumbnails_dir).await?;

        let thumbnail_filename = format!("{}_thumb.jpg", document_id);
        let thumbnail_path = thumbnails_dir.join(&thumbnail_filename);
        
        fs::write(&thumbnail_path, data).await?;
        
        info!("Stored thumbnail locally: {}", thumbnail_path.display());
        Ok(thumbnail_path.to_string_lossy().to_string())
    }

    async fn store_processed_image(&self, _user_id: Uuid, document_id: Uuid, data: &[u8]) -> Result<String> {
        let processed_dir = self.get_processed_images_path();
        fs::create_dir_all(&processed_dir).await?;

        let processed_filename = format!("{}_processed.png", document_id);
        let processed_path = processed_dir.join(&processed_filename);
        
        fs::write(&processed_path, data).await?;
        
        info!("Stored processed image locally: {}", processed_path.display());
        Ok(processed_path.to_string_lossy().to_string())
    }

    async fn retrieve_file(&self, path: &str) -> Result<Vec<u8>> {
        let resolved_path = self.resolve_file_path(path).await?;
        let data = fs::read(&resolved_path).await?;
        Ok(data)
    }

    async fn delete_document_files(&self, _user_id: Uuid, document_id: Uuid, filename: &str) -> Result<()> {
        let mut deleted_files = Vec::new();
        let mut serious_errors = Vec::new();

        // Helper function to safely delete a file
        async fn safe_delete(path: &Path, serious_errors: &mut Vec<String>) -> Option<String> {
            match fs::remove_file(path).await {
                Ok(_) => {
                    info!("Deleted file: {}", path.display());
                    Some(path.to_string_lossy().to_string())
                }
                Err(e) => {
                    match e.kind() {
                        std::io::ErrorKind::NotFound => {
                            info!("File already deleted: {}", path.display());
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

        // Delete main document file (try to find it first)
        let extension = Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        
        let document_filename = if extension.is_empty() {
            document_id.to_string()
        } else {
            format!("{}.{}", document_id, extension)
        };
        
        let main_file = self.get_documents_path().join(&document_filename);
        if let Some(deleted_path) = safe_delete(&main_file, &mut serious_errors).await {
            deleted_files.push(deleted_path);
        }

        // Delete thumbnail if it exists
        let thumbnail_filename = format!("{}_thumb.jpg", document_id);
        let thumbnail_path = self.get_thumbnails_path().join(&thumbnail_filename);
        if let Some(deleted_path) = safe_delete(&thumbnail_path, &mut serious_errors).await {
            deleted_files.push(deleted_path);
        }

        // Delete processed image if it exists
        let processed_image_filename = format!("{}_processed.png", document_id);
        let processed_image_path = self.get_processed_images_path().join(&processed_image_filename);
        if let Some(deleted_path) = safe_delete(&processed_image_path, &mut serious_errors).await {
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
        match self.resolve_file_path(path).await {
            Ok(_) => Ok(true),
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