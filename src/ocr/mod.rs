pub mod api;
pub mod enhanced;
pub mod enhanced_processing;
pub mod error;
pub mod health;
pub mod queue;
pub mod tests;

use anyhow::{anyhow, Result};
use std::path::Path;
use std::panic::{catch_unwind, AssertUnwindSafe};
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
            let bytes = std::fs::read(file_path)?;
            let text = match catch_unwind(AssertUnwindSafe(|| {
                pdf_extract::extract_text_from_mem(&bytes)
            })) {
                Ok(Ok(text)) => text,
                Ok(Err(e)) => return Err(anyhow!("Failed to extract text from PDF: {}", e)),
                Err(_) => return Err(anyhow!("PDF extraction panicked due to invalid content stream in file: {}", file_path)),
            };
            
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