use crate::ocr::error::OcrError;
use crate::ocr::health::OcrHealthChecker;
use anyhow::{anyhow, Result};
use image::DynamicImage;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::time::timeout;

#[cfg(feature = "ocr")]
use tesseract::{Tesseract, PageSegMode};

pub struct EnhancedOcrService {
    health_checker: OcrHealthChecker,
    max_image_width: u32,
    max_image_height: u32,
    ocr_timeout_seconds: u64,
    min_confidence_threshold: f32,
}

impl EnhancedOcrService {
    pub fn new() -> Self {
        Self {
            health_checker: OcrHealthChecker::new(),
            max_image_width: 10000,
            max_image_height: 10000,
            ocr_timeout_seconds: 120,
            min_confidence_threshold: 60.0,
        }
    }
    
    pub fn with_limits(mut self, max_width: u32, max_height: u32) -> Self {
        self.max_image_width = max_width;
        self.max_image_height = max_height;
        self
    }
    
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.ocr_timeout_seconds = seconds;
        self
    }
    
    pub async fn extract_text_with_validation(&self, file_path: &str, lang: &str) -> Result<String> {
        // Perform pre-flight checks
        self.preflight_checks(lang)?;
        
        // Load and validate image
        let image = self.load_and_validate_image(file_path)?;
        
        // Check memory requirements
        let (width, height) = (image.width(), image.height());
        self.health_checker.validate_memory_for_image(width, height)
            .map_err(|e| anyhow!(e))?;
        
        // Perform OCR with timeout
        let text = self.perform_ocr_with_timeout(file_path, lang).await?;
        
        Ok(text)
    }
    
    fn preflight_checks(&self, lang: &str) -> Result<()> {
        // Check Tesseract installation
        self.health_checker.check_tesseract_installation()
            .map_err(|e| anyhow!(e))?;
        
        // Check CPU requirements
        self.health_checker.validate_cpu_requirements()
            .map_err(|e| anyhow!(e))?;
        
        // Check language data
        self.health_checker.check_language_data(lang)
            .map_err(|e| anyhow!(e))?;
        
        Ok(())
    }
    
    fn load_and_validate_image(&self, file_path: &str) -> Result<DynamicImage> {
        // Check file permissions
        if !Path::new(file_path).exists() {
            return Err(anyhow!("File not found: {}", file_path));
        }
        
        let metadata = std::fs::metadata(file_path)
            .map_err(|_| OcrError::PermissionDenied { 
                path: file_path.to_string() 
            })?;
        
        if !metadata.is_file() {
            return Err(anyhow!("Path is not a file: {}", file_path));
        }
        
        // Try to load image
        let image = image::open(file_path)
            .map_err(|e| OcrError::InvalidImageFormat { 
                details: e.to_string() 
            })?;
        
        // Validate dimensions
        if image.width() > self.max_image_width || image.height() > self.max_image_height {
            return Err(OcrError::ImageTooLarge {
                width: image.width(),
                height: image.height(),
                max_width: self.max_image_width,
                max_height: self.max_image_height,
            }.into());
        }
        
        Ok(image)
    }
    
    async fn perform_ocr_with_timeout(&self, file_path: &str, lang: &str) -> Result<String> {
        let file_path = file_path.to_string();
        let lang = lang.to_string();
        let timeout_duration = Duration::from_secs(self.ocr_timeout_seconds);
        let min_confidence = self.min_confidence_threshold;
        
        let ocr_future = tokio::task::spawn_blocking(move || {
            Self::perform_ocr_internal(&file_path, &lang, min_confidence)
        });
        
        match timeout(timeout_duration, ocr_future).await {
            Ok(Ok(result)) => result,
            Ok(Err(e)) => Err(anyhow!("OCR task failed: {}", e)),
            Err(_) => Err(OcrError::OcrTimeout { 
                seconds: self.ocr_timeout_seconds 
            }.into()),
        }
    }
    
    #[cfg(feature = "ocr")]
    fn perform_ocr_internal(file_path: &str, lang: &str, min_confidence: f32) -> Result<String> {
        let start_time = Instant::now();
        
        // Initialize Tesseract with error handling
        let mut tesseract = Tesseract::new(None, Some(lang))
            .map_err(|e| OcrError::InitializationFailed { 
                details: e.to_string() 
            })?;
        
        // Set optimal parameters for various hardware
        tesseract.set_page_seg_mode(PageSegMode::PsmAuto);
        
        let mut tesseract = tesseract
            .set_variable("tessedit_do_invert", "0")?
            .set_variable("edges_max_children_per_outline", "40")?;
        
        // For low-end hardware, use faster but less accurate settings
        if let Ok(available_mem) = std::env::var("OCR_LOW_MEMORY_MODE") {
            if available_mem == "true" {
                tesseract = tesseract
                    .set_variable("textord_heavy_nr", "0")?
                    .set_variable("cube_debug_level", "0")?;
            }
        }
        
        tesseract = tesseract.set_image(file_path)
            .map_err(|e| OcrError::InvalidImageFormat { 
                details: e.to_string() 
            })?;
        
        // Get text with confidence check
        let text = tesseract.get_text()
            .map_err(|e| OcrError::InitializationFailed { 
                details: e.to_string() 
            })?;
        
        // Get mean confidence
        let confidence = tesseract.mean_text_conf();
        
        if confidence < min_confidence as i32 {
            return Err(OcrError::LowConfidence { 
                score: confidence as f32, 
                threshold: min_confidence 
            }.into());
        }
        
        let elapsed = start_time.elapsed();
        tracing::info!("OCR completed in {:?} with confidence: {}%", elapsed, confidence);
        
        Ok(text.trim().to_string())
    }
    
    #[cfg(not(feature = "ocr"))]
    fn perform_ocr_internal(_file_path: &str, _lang: &str, _min_confidence: f32) -> Result<String> {
        Err(anyhow!("OCR feature is disabled. Recompile with --features ocr"))
    }
    
    pub async fn extract_with_fallback(&self, file_path: &str, lang: &str) -> Result<String> {
        // Try primary extraction
        match self.extract_text_with_validation(file_path, lang).await {
            Ok(text) => Ok(text),
            Err(e) => {
                // Check if error is recoverable
                if let Some(ocr_error) = e.downcast_ref::<OcrError>() {
                    if ocr_error.is_recoverable() {
                        // Try with reduced quality settings
                        self.extract_with_reduced_quality(file_path, lang).await
                    } else {
                        Err(e)
                    }
                } else {
                    Err(e)
                }
            }
        }
    }
    
    async fn extract_with_reduced_quality(&self, file_path: &str, lang: &str) -> Result<String> {
        // Downsample image for lower memory usage
        let image = self.load_and_validate_image(file_path)?;
        let resized = self.resize_for_ocr(image);
        
        // Save temporary resized image
        let temp_path = format!("{}_resized.png", file_path);
        resized.save(&temp_path)
            .map_err(|e| anyhow!("Failed to save resized image: {}", e))?;
        
        // Try OCR on resized image
        let result = self.perform_ocr_with_timeout(&temp_path, lang).await;
        
        // Clean up
        let _ = std::fs::remove_file(&temp_path);
        
        result
    }
    
    fn resize_for_ocr(&self, image: DynamicImage) -> DynamicImage {
        let (width, height) = (image.width(), image.height());
        
        // Target dimensions for low memory mode
        let max_dimension = 2000;
        
        if width > max_dimension || height > max_dimension {
            let scale = max_dimension as f32 / width.max(height) as f32;
            let new_width = (width as f32 * scale) as u32;
            let new_height = (height as f32 * scale) as u32;
            
            image.resize(new_width, new_height, image::imageops::FilterType::Lanczos3)
        } else {
            image
        }
    }
    
    pub async fn get_diagnostics(&self) -> String {
        let diagnostics = self.health_checker.get_full_diagnostics();
        format!("{}", diagnostics)
    }
}