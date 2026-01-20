pub mod api;
pub mod enhanced;
pub mod enhanced_processing;
pub mod error;
pub mod health;
pub mod queue;
pub mod tests;
pub mod xml_extractor;

use anyhow::{anyhow, Result};
use std::path::Path;
use crate::ocr::error::OcrError;
use crate::ocr::health::OcrHealthChecker;

#[cfg(feature = "ocr")]
use tesseract::Tesseract;

pub struct OcrService {
    health_checker: OcrHealthChecker,
    temp_dir: String,
}

/// Configuration for the OCR service
#[derive(Debug, Clone)]
pub struct OcrConfig {
    /// Temporary directory for processing
    pub temp_dir: String,
}

impl Default for OcrConfig {
    fn default() -> Self {
        Self {
            temp_dir: std::env::var("TEMP_DIR").unwrap_or_else(|_| "/tmp".to_string()),
        }
    }
}

impl OcrService {
    pub fn new() -> Self {
        Self {
            health_checker: OcrHealthChecker::new(),
            temp_dir: std::env::var("TEMP_DIR").unwrap_or_else(|_| "/tmp".to_string()),
        }
    }

    /// Create OCR service with configuration
    pub fn new_with_config(config: OcrConfig) -> Self {
        Self {
            health_checker: OcrHealthChecker::new(),
            temp_dir: config.temp_dir,
        }
    }

    pub async fn extract_text_from_image(&self, file_path: &str) -> Result<String> {
        self.extract_text_from_image_with_lang(file_path, "eng").await
    }

    pub async fn extract_text_from_image_with_lang(&self, file_path: &str, lang: &str) -> Result<String> {
        #[cfg(feature = "ocr")]
        {
            // Perform health checks first
            self.health_checker.check_tesseract_installation()
                .map_err(|e: OcrError| anyhow!(e))?;
            self.health_checker.validate_language_combination(lang)
                .map_err(|e: OcrError| anyhow!(e))?;
            
            let mut tesseract = Tesseract::new(None, Some(lang))
                .map_err(|e| anyhow!(OcrError::InitializationFailed { 
                    details: e.to_string() 
                }))?
                .set_image(file_path)?;
            
            let text = tesseract.get_text()
                .map_err(|e| anyhow!(OcrError::InitializationFailed { 
                    details: format!("Failed to extract text: {}", e) 
                }))?;
            
            Ok(text.trim().to_string())
        }
        
        #[cfg(not(feature = "ocr"))]
        {
            Err(anyhow!(OcrError::TesseractNotInstalled))
        }
    }

    pub async fn extract_text_from_pdf(&self, file_path: &str) -> Result<String> {
        #[cfg(feature = "ocr")]
        {
            // Check if ocrmypdf is available
            let ocrmypdf_check = tokio::process::Command::new("ocrmypdf")
                .arg("--version")
                .output()
                .await;
                
            if ocrmypdf_check.is_err() || !ocrmypdf_check.unwrap().status.success() {
                return Err(anyhow!(
                    "ocrmypdf is not available. Please install ocrmypdf: \
                    On Ubuntu/Debian: 'apt-get install ocrmypdf'. \
                    On macOS: 'brew install ocrmypdf'."
                ));
            }
            
            // Create temporary file for text extraction
            let temp_dir = std::env::var("TEMP_DIR").unwrap_or_else(|_| "/tmp".to_string());
            let temp_text_path = format!("{}/pdf_text_{}.txt", temp_dir, std::process::id());
            
            // Progressive extraction with fallback strategies
            // Strategy 1: pdftotext for existing text (fastest)
            let mut output = tokio::process::Command::new("pdftotext")
                .arg("-layout")  // Preserve layout
                .arg(file_path)
                .arg(&temp_text_path)
                .output()
                .await?;
                
            if output.status.success() {
                // Check if we got substantial text
                if let Ok(text) = tokio::fs::read_to_string(&temp_text_path).await {
                    let word_count = text.split_whitespace().count();
                    if word_count > 5 {
                        let _ = tokio::fs::remove_file(&temp_text_path).await;
                        return Ok(text.trim().to_string());
                    }
                }
            }
            
            if !output.status.success() {
                // Strategy 2: ocrmypdf sidecar (when pdftotext fails)
                output = tokio::process::Command::new("ocrmypdf")
                    .arg("--sidecar")    // Extract text to sidecar file
                    .arg(&temp_text_path)
                    .arg(file_path)
                    .arg("-")  // Dummy output
                    .output()
                    .await?;
                    
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let stdout = String::from_utf8_lossy(&output.stdout);

                    // Clean up temp file on error
                    let _ = tokio::fs::remove_file(&temp_text_path).await;

                    // Last resort: try direct text extraction
                    match self.extract_text_from_pdf_bytes(file_path).await {
                        Ok(text) if !text.trim().is_empty() => {
                            return Ok(text);
                        }
                        Ok(_) => {
                            // Empty text from direct extraction
                        }
                        Err(_) => {
                            // Direct extraction also failed
                        }
                    }

                    // Provide a detailed error message with troubleshooting steps
                    return Err(anyhow!(
                        "OCR failed for PDF '{}' after trying multiple strategies.\n\
                        \n\
                        Error from ocrmypdf:\n{}\n{}\n\
                        \n\
                        Possible causes and solutions:\n\
                        1. PDF contains very large images (high DPI scans) - try reducing image resolution before upload\n\
                        2. Insufficient memory - increase container memory limits or process smaller batches\n\
                        3. Corrupted PDF - try re-scanning or re-creating the PDF\n\
                        4. Unsupported PDF format - ensure the PDF is not password-protected or malformed\n\
                        \n\
                        For large documents, consider splitting into smaller files before uploading.",
                        file_path, stderr, stdout
                    ));
                }
            }
            
            // Read the extracted text
            let text = tokio::fs::read_to_string(&temp_text_path).await?;
            
            // Clean up temporary file
            let _ = tokio::fs::remove_file(&temp_text_path).await;
            
            Ok(text.trim().to_string())
        }
        
        #[cfg(not(feature = "ocr"))]
        {
            Err(anyhow!(OcrError::TesseractNotInstalled))
        }
    }

    /// Extract text from Office documents using XML extraction
    pub async fn extract_text_from_office_document(
        &self,
        file_path: &str,
        mime_type: &str,
    ) -> Result<crate::ocr::enhanced::OcrResult> {
        // Use XML extraction directly
        let xml_extractor = crate::ocr::xml_extractor::XmlOfficeExtractor::new(
            self.temp_dir.clone()
        );
        
        let result = xml_extractor.extract_text_from_office(file_path, mime_type).await?;
        // Convert OfficeExtractionResult to OcrResult for backward compatibility
        Ok(crate::ocr::enhanced::OcrResult {
            text: result.text,
            confidence: result.confidence,
            processing_time_ms: result.processing_time_ms,
            word_count: result.word_count,
            preprocessing_applied: vec![format!("XML extraction - {}", result.extraction_method)],
            processed_image_path: None,
        })
    }

    /// Extract text from Office documents with custom configuration
    pub async fn extract_text_from_office_document_with_config(
        &self,
        file_path: &str,
        mime_type: &str,
    ) -> Result<crate::ocr::enhanced::OcrResult> {
        // Use the same XML extraction logic as the basic method
        self.extract_text_from_office_document(file_path, mime_type).await
    }

    pub async fn extract_text(&self, file_path: &str, mime_type: &str) -> Result<String> {
        self.extract_text_with_lang(file_path, mime_type, "eng").await
    }

    pub async fn extract_text_with_lang(&self, file_path: &str, mime_type: &str, lang: &str) -> Result<String> {
        match mime_type {
            "application/pdf" => self.extract_text_from_pdf(file_path).await,
            // Office document types - use fallback strategy if available
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document" |
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" |
            "application/vnd.openxmlformats-officedocument.presentationml.presentation" |
            "application/msword" |
            "application/vnd.ms-excel" |
            "application/vnd.ms-powerpoint" => {
                let result = self.extract_text_from_office_document(file_path, mime_type).await?;
                Ok(result.text)
            }
            "image/png" | "image/jpeg" | "image/jpg" | "image/tiff" | "image/bmp" => {
                self.extract_text_from_image_with_lang(file_path, lang).await
            }
            "text/plain" => {
                let text = tokio::fs::read_to_string(file_path).await?;
                Ok(text)
            }
            _ => {
                if self.is_image_file(file_path) {
                    self.extract_text_from_image_with_lang(file_path, lang).await
                } else {
                    Err(anyhow!(OcrError::InvalidImageFormat { 
                        details: format!("Unsupported MIME type: {}", mime_type) 
                    }))
                }
            }
        }
    }

    /// Last resort: extract readable text directly from PDF bytes
    async fn extract_text_from_pdf_bytes(&self, file_path: &str) -> Result<String> {
        let bytes = tokio::fs::read(file_path).await?;
        
        // Look for readable ASCII text in the PDF
        let mut ascii_text = String::new();
        let mut current_word = String::new();
        
        for &byte in &bytes {
            if byte >= 32 && byte <= 126 {  // Printable ASCII
                current_word.push(byte as char);
            } else {
                if current_word.len() > 3 {  // Only keep words longer than 3 characters
                    ascii_text.push_str(&current_word);
                    ascii_text.push(' ');
                }
                current_word.clear();
            }
        }
        
        // Add the last word if it's long enough
        if current_word.len() > 3 {
            ascii_text.push_str(&current_word);
        }
        
        // Clean up the text
        let cleaned_text = ascii_text
            .split_whitespace()
            .filter(|word| word.len() > 1)  // Filter out single characters
            .collect::<Vec<_>>()
            .join(" ");
        
        if cleaned_text.trim().is_empty() {
            Err(anyhow!("No readable text found in PDF"))
        } else {
            Ok(cleaned_text)
        }
    }

    pub fn is_image_file(&self, file_path: &str) -> bool {
        if let Some(extension) = Path::new(file_path)
            .extension()
            .and_then(|ext| ext.to_str())
        {
            let ext_lower = extension.to_lowercase();
            matches!(ext_lower.as_str(), "png" | "jpg" | "jpeg" | "tiff" | "bmp" | "gif")
        } else {
            false
        }
    }


    /// Check if Office document extraction is available
    pub fn supports_office_documents(&self) -> bool {
        true // XML extraction is always available
    }

    /// Get supported MIME types
    pub fn get_supported_mime_types(&self) -> Vec<&'static str> {
        let mut types = vec![
            "application/pdf",
            "image/png",
            "image/jpeg", 
            "image/jpg",
            "image/tiff",
            "image/bmp",
            "text/plain",
        ];

        // Office document types are always supported via XML extraction
        types.extend_from_slice(&[
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            "application/msword",
            "application/vnd.ms-excel",
            "application/vnd.ms-powerpoint",
        ]);

        types
    }
}