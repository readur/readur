use anyhow::Result;
use chrono::Utc;
use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid;
use tracing::{info, warn, error};

use crate::models::Document;

#[cfg(feature = "ocr")]
use image::{DynamicImage, ImageFormat, imageops::FilterType};

#[derive(Clone)]
pub struct FileService {
    upload_path: String,
}

impl FileService {
    pub fn new(upload_path: String) -> Self {
        Self { upload_path }
    }

    /// Initialize the upload directory structure
    pub async fn initialize_directory_structure(&self) -> Result<()> {
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

    /// Get the path for a specific subdirectory
    pub fn get_subdirectory_path(&self, subdir: &str) -> PathBuf {
        Path::new(&self.upload_path).join(subdir)
    }

    /// Get the documents directory path
    pub fn get_documents_path(&self) -> PathBuf {
        self.get_subdirectory_path("documents")
    }

    /// Get the thumbnails directory path
    pub fn get_thumbnails_path(&self) -> PathBuf {
        self.get_subdirectory_path("thumbnails")
    }

    /// Get the processed images directory path
    pub fn get_processed_images_path(&self) -> PathBuf {
        self.get_subdirectory_path("processed_images")
    }

    /// Get the temp directory path
    pub fn get_temp_path(&self) -> PathBuf {
        self.get_subdirectory_path("temp")
    }

    /// Migrate existing files from the root upload directory to the structured format
    pub async fn migrate_existing_files(&self) -> Result<()> {
        let base_path = Path::new(&self.upload_path);
        let documents_dir = self.get_documents_path();
        let thumbnails_dir = self.get_thumbnails_path();
        
        info!("Starting migration of existing files to structured directories...");
        let mut migrated_count = 0;
        let mut thumbnail_count = 0;
        
        // Read all files in the base upload directory
        let mut entries = fs::read_dir(base_path).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let file_path = entry.path();
            
            // Skip directories and already structured subdirectories
            if file_path.is_dir() {
                continue;
            }
            
            if let Some(filename) = file_path.file_name().and_then(|n| n.to_str()) {
                // Handle thumbnail files
                if filename.ends_with("_thumb.jpg") {
                    let new_path = thumbnails_dir.join(filename);
                    if let Err(e) = fs::rename(&file_path, &new_path).await {
                        warn!("Failed to migrate thumbnail {}: {}", filename, e);
                    } else {
                        thumbnail_count += 1;
                        info!("Migrated thumbnail: {} -> {:?}", filename, new_path);
                    }
                }
                // Handle regular document files
                else {
                    let new_path = documents_dir.join(filename);
                    if let Err(e) = fs::rename(&file_path, &new_path).await {
                        warn!("Failed to migrate document {}: {}", filename, e);
                    } else {
                        migrated_count += 1;
                        info!("Migrated document: {} -> {:?}", filename, new_path);
                    }
                }
            }
        }
        
        info!("Migration completed: {} documents, {} thumbnails moved to structured directories", 
              migrated_count, thumbnail_count);
        Ok(())
    }

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

    pub fn create_document(
        &self,
        filename: &str,
        original_filename: &str,
        file_path: &str,
        file_size: i64,
        mime_type: &str,
        user_id: Uuid,
        file_hash: Option<String>,
        original_created_at: Option<chrono::DateTime<chrono::Utc>>,
        original_modified_at: Option<chrono::DateTime<chrono::Utc>>,
        source_path: Option<String>,
        source_type: Option<String>,
        source_id: Option<Uuid>,
        file_permissions: Option<i32>,
        file_owner: Option<String>,
        file_group: Option<String>,
        source_metadata: Option<serde_json::Value>,
    ) -> Document {
        Document {
            id: Uuid::new_v4(),
            filename: filename.to_string(),
            original_filename: original_filename.to_string(),
            file_path: file_path.to_string(),
            file_size,
            mime_type: mime_type.to_string(),
            content: None,
            ocr_text: None,
            ocr_confidence: None,
            ocr_word_count: None,
            ocr_processing_time_ms: None,
            ocr_status: Some("pending".to_string()),
            ocr_error: None,
            ocr_completed_at: None,
            ocr_retry_count: None,
            ocr_failure_reason: None,
            tags: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            user_id,
            file_hash,
            original_created_at,
            original_modified_at,
            source_path,
            source_type,
            source_id,
            file_permissions,
            file_owner,
            file_group,
            source_metadata,
        }
    }

    pub fn is_allowed_file_type(&self, filename: &str, allowed_types: &[String]) -> bool {
        if let Some(extension) = Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
        {
            let ext_lower = extension.to_lowercase();
            allowed_types.contains(&ext_lower)
        } else {
            false
        }
    }

    /// Resolve file path to actual location, handling both old and new directory structures
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

    pub async fn read_file(&self, file_path: &str) -> Result<Vec<u8>> {
        let resolved_path = self.resolve_file_path(file_path).await?;
        let data = fs::read(&resolved_path).await?;
        Ok(data)
    }

    #[cfg(feature = "ocr")]
    pub async fn get_or_generate_thumbnail(&self, file_path: &str, filename: &str) -> Result<Vec<u8>> {
        // Use the structured thumbnails directory
        let thumbnails_dir = self.get_thumbnails_path();
        if !thumbnails_dir.exists() {
            if let Err(e) = fs::create_dir_all(&thumbnails_dir).await {
                error!("Failed to create thumbnails directory: {}", e);
                return Err(anyhow::anyhow!("Failed to create thumbnails directory: {}", e));
            }
        }

        // Generate thumbnail filename based on original file path
        let file_stem = Path::new(file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let thumbnail_path = thumbnails_dir.join(format!("{}_thumb.jpg", file_stem));

        // Check if thumbnail already exists
        if thumbnail_path.exists() {
            return self.read_file(&thumbnail_path.to_string_lossy()).await;
        }

        // Resolve file path and generate thumbnail
        let resolved_path = self.resolve_file_path(file_path).await?;
        let thumbnail_data = self.generate_thumbnail(&resolved_path, filename).await?;
        
        // Save thumbnail to cache
        fs::write(&thumbnail_path, &thumbnail_data).await?;
        
        Ok(thumbnail_data)
    }

    #[cfg(feature = "ocr")]
    async fn generate_thumbnail(&self, file_path: &str, filename: &str) -> Result<Vec<u8>> {
        let file_data = self.read_file(file_path).await?;
        
        // Determine file type from extension
        let extension = Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "jpg" | "jpeg" | "png" | "bmp" | "tiff" | "gif" => {
                self.generate_image_thumbnail(&file_data).await
            }
            "pdf" => {
                self.generate_pdf_thumbnail(&file_data).await
            }
            "txt" => {
                self.generate_text_thumbnail(&file_data).await
            }
            "doc" | "docx" => {
                self.generate_placeholder_thumbnail("DOC").await
            }
            _ => {
                // For other file types, generate a placeholder
                self.generate_placeholder_thumbnail(&extension.to_uppercase()).await
            }
        }
    }

    #[cfg(feature = "ocr")]
    async fn generate_image_thumbnail(&self, file_data: &[u8]) -> Result<Vec<u8>> {
        let img = image::load_from_memory(file_data)?;
        let thumbnail = img.resize(200, 200, FilterType::Lanczos3);
        
        // Convert to RGB if the image has an alpha channel (RGBA)
        // JPEG doesn't support transparency, so we need to remove the alpha channel
        let rgb_thumbnail = match thumbnail {
            image::DynamicImage::ImageRgba8(_) => {
                // Convert RGBA to RGB by compositing against a white background
                let rgb_img = image::DynamicImage::ImageRgb8(
                    thumbnail.to_rgb8()
                );
                rgb_img
            },
            _ => thumbnail, // Already RGB or other compatible format
        };
        
        let mut buffer = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut buffer);
        rgb_thumbnail.write_to(&mut cursor, ImageFormat::Jpeg)?;
        
        Ok(buffer)
    }

    #[cfg(feature = "ocr")]
    async fn generate_pdf_thumbnail(&self, file_data: &[u8]) -> Result<Vec<u8>> {
        use std::process::Command;
        use tokio::fs;
        use uuid::Uuid;
        
        // Create a temporary file for the PDF
        let temp_id = Uuid::new_v4();
        let temp_pdf_path = format!("/tmp/pdf_thumb_{}.pdf", temp_id);
        let temp_png_path = format!("/tmp/pdf_thumb_{}.png", temp_id);
        
        // Write PDF data to temporary file
        if let Err(e) = fs::write(&temp_pdf_path, file_data).await {
            error!("Failed to write temporary PDF file: {}", e);
            return self.generate_placeholder_thumbnail("PDF").await;
        }
        
        // Use pdftoppm to convert first page to PNG
        let output = Command::new("pdftoppm")
            .arg("-f").arg("1")          // First page only
            .arg("-l").arg("1")          // Last page (same as first)
            .arg("-scale-to").arg("200") // Scale to 200px width
            .arg("-png")                 // Output as PNG
            .arg(&temp_pdf_path)
            .arg(&format!("/tmp/pdf_thumb_{}", temp_id)) // Output prefix
            .output();
        
        // Clean up temporary PDF file
        let _ = fs::remove_file(&temp_pdf_path).await;
        
        match output {
            Ok(result) if result.status.success() => {
                // pdftoppm adds "-1" to the filename for the first page
                let actual_png_path = format!("/tmp/pdf_thumb_{}-1.png", temp_id);
                
                // Read the generated PNG file
                match fs::read(&actual_png_path).await {
                    Ok(png_data) => {
                        // Clean up temporary PNG file
                        let _ = fs::remove_file(&actual_png_path).await;
                        
                        // Convert PNG to JPEG thumbnail
                        match image::load_from_memory(&png_data) {
                            Ok(img) => {
                                // Resize to 200x200 maintaining aspect ratio
                                let thumbnail = img.resize(200, 200, image::imageops::FilterType::Lanczos3);
                                
                                // Convert to JPEG
                                let mut buffer = Vec::new();
                                let mut cursor = std::io::Cursor::new(&mut buffer);
                                if thumbnail.write_to(&mut cursor, ImageFormat::Jpeg).is_ok() {
                                    Ok(buffer)
                                } else {
                                    self.generate_placeholder_thumbnail("PDF").await
                                }
                            }
                            Err(_) => self.generate_placeholder_thumbnail("PDF").await,
                        }
                    }
                    Err(_) => {
                        let _ = fs::remove_file(&actual_png_path).await;
                        self.generate_placeholder_thumbnail("PDF").await
                    }
                }
            }
            _ => {
                // Clean up any potential PNG files
                let _ = fs::remove_file(&temp_png_path).await;
                let _ = fs::remove_file(&format!("/tmp/pdf_thumb_{}-1.png", temp_id)).await;
                self.generate_placeholder_thumbnail("PDF").await
            }
        }
    }

    #[cfg(feature = "ocr")]
    async fn generate_text_thumbnail(&self, file_data: &[u8]) -> Result<Vec<u8>> {
        use image::Rgb;
        
        // Convert bytes to text
        let text = String::from_utf8_lossy(file_data);
        self.generate_text_based_thumbnail(&text, "TXT", Rgb([34, 139, 34])).await
    }

    #[cfg(feature = "ocr")]
    async fn generate_text_based_thumbnail(&self, text: &str, file_type: &str, bg_color: image::Rgb<u8>) -> Result<Vec<u8>> {
        use image::{RgbImage, Rgb, DynamicImage, ImageFormat};
        
        let width = 200;
        let height = 200;
        let mut img = RgbImage::new(width, height);
        
        // Fill background
        for pixel in img.pixels_mut() {
            *pixel = bg_color;
        }
        
        // Add file type indicator at the top
        let text_color = Rgb([255, 255, 255]); // White text
        let preview_text = if text.len() > 300 {
            format!("{}\n{}", file_type, &text[..300].trim())
        } else {
            format!("{}\n{}", file_type, text.trim())
        };
        
        // Simple text rendering - just place some characters as visual indicators
        // For a more sophisticated approach, you'd use a text rendering library
        let lines: Vec<&str> = preview_text.lines().take(15).collect();
        for (line_idx, line) in lines.iter().enumerate() {
            let y_offset = 20 + (line_idx * 12);
            if y_offset >= height as usize - 10 { break; }
            
            // Simple character placement (very basic text rendering)
            for (char_idx, _) in line.chars().take(25).enumerate() {
                let x_offset = 10 + (char_idx * 7);
                if x_offset >= width as usize - 10 { break; }
                
                // Draw a simple "character" representation as white pixels
                if x_offset < width as usize && y_offset < height as usize {
                    if let Some(pixel) = img.get_pixel_mut_checked(x_offset as u32, y_offset as u32) {
                        *pixel = text_color;
                    }
                    // Add some thickness
                    if let Some(pixel) = img.get_pixel_mut_checked(x_offset as u32 + 1, y_offset as u32) {
                        *pixel = text_color;
                    }
                }
            }
        }
        
        let dynamic_img = DynamicImage::ImageRgb8(img);
        let mut buffer = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut buffer);
        dynamic_img.write_to(&mut cursor, ImageFormat::Jpeg)?;
        
        Ok(buffer)
    }

    #[cfg(feature = "ocr")]
    async fn generate_placeholder_thumbnail(&self, file_type: &str) -> Result<Vec<u8>> {
        // Create a simple colored rectangle as placeholder
        use image::{RgbImage, Rgb};
        
        let mut img = RgbImage::new(200, 200);
        
        // Different colors for different file types
        let color = match file_type {
            "PDF" => Rgb([220, 38, 27]),   // Red for PDF
            "TXT" => Rgb([34, 139, 34]),   // Green for text
            "DOC" | "DOCX" => Rgb([41, 128, 185]), // Blue for Word docs
            _ => Rgb([108, 117, 125]),     // Gray for unknown
        };
        
        // Fill with solid color
        for pixel in img.pixels_mut() {
            *pixel = color;
        }
        
        let dynamic_img = DynamicImage::ImageRgb8(img);
        let mut buffer = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut buffer);
        dynamic_img.write_to(&mut cursor, ImageFormat::Jpeg)?;
        
        Ok(buffer)
    }

    #[cfg(not(feature = "ocr"))]
    pub async fn get_or_generate_thumbnail(&self, _file_path: &str, _filename: &str) -> Result<Vec<u8>> {
        anyhow::bail!("Thumbnail generation requires OCR feature")
    }

    pub async fn delete_document_files(&self, document: &Document) -> Result<()> {
        let mut deleted_files = Vec::new();
        let mut serious_errors = Vec::new();

        // Helper function to safely delete a file, handling concurrent deletion scenarios
        async fn safe_delete(path: &Path, serious_errors: &mut Vec<String>) -> Option<String> {
            match fs::remove_file(path).await {
                Ok(_) => {
                    info!("Deleted file: {}", path.display());
                    Some(path.to_string_lossy().to_string())
                }
                Err(e) => {
                    match e.kind() {
                        std::io::ErrorKind::NotFound => {
                            // File already deleted (possibly by concurrent request) - this is fine
                            info!("File already deleted: {}", path.display());
                            None
                        }
                        _ => {
                            // Other errors (permissions, I/O errors, etc.) are serious
                            warn!("Failed to delete file {}: {}", path.display(), e);
                            serious_errors.push(format!("Failed to delete file {}: {}", path.display(), e));
                            None
                        }
                    }
                }
            }
        }

        // Delete main document file
        let main_file = Path::new(&document.file_path);
        if let Some(deleted_path) = safe_delete(&main_file, &mut serious_errors).await {
            deleted_files.push(deleted_path);
        }

        // Delete thumbnail if it exists
        let thumbnail_filename = format!("{}_thumb.jpg", document.id);
        let thumbnail_path = self.get_thumbnails_path().join(&thumbnail_filename);
        if let Some(deleted_path) = safe_delete(&thumbnail_path, &mut serious_errors).await {
            deleted_files.push(deleted_path);
        }

        // Delete processed image if it exists
        let processed_image_filename = format!("{}_processed.png", document.id);
        let processed_image_path = self.get_processed_images_path().join(&processed_image_filename);
        if let Some(deleted_path) = safe_delete(&processed_image_path, &mut serious_errors).await {
            deleted_files.push(deleted_path);
        }

        // Only fail if there were serious errors (not "file not found")
        if !serious_errors.is_empty() {
            error!("Serious errors occurred while deleting files for document {}: {}", document.id, serious_errors.join("; "));
            return Err(anyhow::anyhow!("File deletion errors: {}", serious_errors.join("; ")));
        }

        if deleted_files.is_empty() {
            info!("No files needed deletion for document {} (all files already removed)", document.id);
        } else {
            info!("Successfully deleted {} files for document {}", deleted_files.len(), document.id);
        }

        Ok(())
    }
}