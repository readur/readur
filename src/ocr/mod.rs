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
}

impl OcrService {
    pub fn new() -> Self {
        Self {
            health_checker: OcrHealthChecker::new(),
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
                    // Final fallback: minimal processing (may skip large pages)
                    output = tokio::process::Command::new("ocrmypdf")
                        .arg("--skip-big")   // Skip very large pages to avoid memory issues
                        .arg("--sidecar")
                        .arg(&temp_text_path)
                        .arg(file_path)
                        .arg("-")
                        .output()
                        .await?;
                        
                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
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
                        
                        return Err(anyhow!("Failed to extract text from PDF after trying multiple strategies: {}", stderr));
                    }
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

    pub async fn extract_text(&self, file_path: &str, mime_type: &str) -> Result<String> {
        self.extract_text_with_lang(file_path, mime_type, "eng").await
    }

    pub async fn extract_text_with_lang(&self, file_path: &str, mime_type: &str, lang: &str) -> Result<String> {
        match mime_type {
            "application/pdf" => self.extract_text_from_pdf(file_path).await,
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
}