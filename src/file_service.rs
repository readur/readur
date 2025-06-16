use anyhow::Result;
use chrono::Utc;
use std::path::Path;
use tokio::fs;
use uuid::Uuid;

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
        
        let file_path = Path::new(&self.upload_path).join(&saved_filename);
        
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).await?;
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
            tags: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            user_id,
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

    pub async fn read_file(&self, file_path: &str) -> Result<Vec<u8>> {
        let data = fs::read(file_path).await?;
        Ok(data)
    }

    #[cfg(feature = "ocr")]
    pub async fn get_or_generate_thumbnail(&self, file_path: &str, filename: &str) -> Result<Vec<u8>> {
        // Create thumbnails directory if it doesn't exist
        let thumbnails_dir = Path::new(&self.upload_path).join("thumbnails");
        if !thumbnails_dir.exists() {
            fs::create_dir_all(&thumbnails_dir).await?;
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

        // Generate thumbnail
        let thumbnail_data = self.generate_thumbnail(file_path, filename).await?;
        
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
                // For PDFs, we'd need pdf2image or similar
                // For now, return a placeholder
                self.generate_placeholder_thumbnail("PDF").await
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
}