use anyhow::{anyhow, Result};
use tracing::{debug, info, warn};
use std::panic::{catch_unwind, AssertUnwindSafe};

#[cfg(feature = "ocr")]
use image::{DynamicImage, ImageBuffer, Luma, GenericImageView};
#[cfg(feature = "ocr")]
use imageproc::{
    contrast::adaptive_threshold,
    morphology::{close, open},
    filter::{median_filter, gaussian_blur_f32},
    distance_transform::Norm,
};
#[cfg(feature = "ocr")]
use tesseract::{Tesseract, PageSegMode, OcrEngineMode};

use crate::models::Settings;
use crate::services::file_service::FileService;

#[derive(Debug, Clone)]
pub struct ImageQualityStats {
    pub average_brightness: f32,
    pub contrast_ratio: f32,
    pub noise_level: f32,
    pub sharpness: f32,
}

#[derive(Debug, Clone)]
pub struct OcrResult {
    pub text: String,
    pub confidence: f32,
    pub processing_time_ms: u64,
    pub word_count: usize,
    pub preprocessing_applied: Vec<String>,
    pub processed_image_path: Option<String>,
}

pub struct EnhancedOcrService {
    pub temp_dir: String,
    pub file_service: FileService,
}

impl EnhancedOcrService {
    pub fn new(temp_dir: String) -> Self {
        let upload_path = std::env::var("UPLOAD_PATH").unwrap_or_else(|_| "./uploads".to_string());
        let file_service = FileService::new(upload_path);
        Self { temp_dir, file_service }
    }

    /// Extract text from image with high-quality OCR settings
    #[cfg(feature = "ocr")]
    pub async fn extract_text_from_image(&self, file_path: &str, settings: &Settings) -> Result<OcrResult> {
        let start_time = std::time::Instant::now();
        info!("Starting enhanced OCR for image: {}", file_path);
        
        let mut preprocessing_applied = Vec::new();
        
        // Load and preprocess the image
        let (processed_image_path, mut preprocess_steps) = if settings.enable_image_preprocessing {
            let (processed_path, steps) = self.preprocess_image(file_path, settings).await?;
            (processed_path, steps)
        } else {
            (file_path.to_string(), Vec::new())
        };
        
        preprocessing_applied.extend(preprocess_steps);

        // Move CPU-intensive OCR operations to blocking thread pool
        let processed_image_path_clone = processed_image_path.clone();
        let settings_clone = settings.clone();
        let temp_dir = self.temp_dir.clone();
        
        let ocr_result = tokio::task::spawn_blocking(move || -> Result<(String, f32)> {
            // Configure Tesseract with optimal settings
            let ocr_service = EnhancedOcrService::new(temp_dir);
            let mut tesseract = ocr_service.configure_tesseract(&processed_image_path_clone, &settings_clone)?;
            
            // Extract text with confidence
            let text = tesseract.get_text()?.trim().to_string();
            let confidence = ocr_service.calculate_overall_confidence(&mut tesseract)?;
            
            Ok((text, confidence))
        }).await??;
        
        let (text, confidence) = ocr_result;
        
        let processing_time = start_time.elapsed().as_millis() as u64;
        let word_count = text.split_whitespace().count();
        
        debug!(
            "OCR completed: {} words, {:.1}% confidence, {}ms",
            word_count, confidence, processing_time
        );
        
        // Return the processed image path if different from original (caller will handle cleanup/saving)
        let result_processed_image_path = if processed_image_path != file_path {
            Some(processed_image_path.clone())
        } else {
            None
        };
        
        let result = OcrResult {
            text,
            confidence,
            processing_time_ms: processing_time,
            word_count,
            preprocessing_applied,
            processed_image_path: result_processed_image_path,
        };
        
        // Clean up temporary files if not saved for review
        if let Some(ref temp_path) = result.processed_image_path {
            if !settings.save_processed_images {
                let _ = tokio::fs::remove_file(temp_path).await;
            }
        }
        
        Ok(result)
    }

    /// Preprocess image for optimal OCR quality, especially for challenging conditions
    #[cfg(feature = "ocr")]
    async fn preprocess_image(&self, input_path: &str, settings: &Settings) -> Result<(String, Vec<String>)> {
        // Resolve the file path first
        let resolved_path = self.resolve_file_path(input_path).await?;
        let img = image::open(&resolved_path)?;
        let mut processed_img = img;
        let mut preprocessing_applied = Vec::new();
        
        info!("Original image dimensions: {}x{}", processed_img.width(), processed_img.height());
        
        // Apply orientation detection and correction
        if settings.ocr_detect_orientation {
            processed_img = self.detect_and_correct_orientation(processed_img)?;
        }
        
        // Aggressively upscale low-resolution images for better OCR
        processed_img = self.smart_resize_for_ocr(processed_img, settings.ocr_dpi)?;
        
        // Convert to grayscale for better OCR
        let gray_img = processed_img.to_luma8();
        let mut processed_gray = gray_img;
        
        // Analyze image quality and apply appropriate enhancements
        let quality_stats = self.analyze_image_quality(&processed_gray);
        info!("Image quality analysis: brightness={:.1}, contrast={:.1}, noise_level={:.1}, sharpness={:.1}", 
               quality_stats.average_brightness, quality_stats.contrast_ratio, quality_stats.noise_level, quality_stats.sharpness);
        
        // Determine if image needs enhancement based on quality thresholds
        let needs_enhancement = self.needs_enhancement(&quality_stats, settings);
        
        if !needs_enhancement {
            info!("Image quality is good, skipping enhancement steps");
        } else {
            info!("Image quality needs improvement, applying selective enhancements");
            
            // Apply brightness correction only for very dim images
            if quality_stats.average_brightness < 50.0 || settings.ocr_brightness_boost > 0.0 {
                processed_gray = self.enhance_brightness_and_contrast(processed_gray, &quality_stats, settings)?;
                preprocessing_applied.push("Brightness/contrast correction".to_string());
            }
            
            // Apply noise removal only for very noisy images
            if quality_stats.noise_level > 0.25 || (settings.ocr_remove_noise && settings.ocr_noise_reduction_level > 1) {
                processed_gray = self.adaptive_noise_removal(processed_gray, &quality_stats, settings)?;
                preprocessing_applied.push("Noise reduction".to_string());
            }
            
            // Apply contrast enhancement only for very low contrast images
            if quality_stats.contrast_ratio < 0.2 || (settings.ocr_enhance_contrast && settings.ocr_adaptive_threshold_window_size > 0) {
                let original_gray = processed_gray.clone();
                match self.adaptive_contrast_enhancement(processed_gray, &quality_stats, settings) {
                    Ok(enhanced) => {
                        processed_gray = enhanced;
                        preprocessing_applied.push("Contrast enhancement".to_string());
                    }
                    Err(e) => {
                        warn!("Contrast enhancement failed, using alternative method: {}", e);
                        // Fallback to basic contrast enhancement
                        processed_gray = self.apply_alternative_contrast_enhancement(original_gray.clone(), &quality_stats, settings)
                            .unwrap_or_else(|_| {
                                warn!("Alternative contrast enhancement also failed, using original image");
                                original_gray
                            });
                        preprocessing_applied.push("Basic contrast enhancement".to_string());
                    }
                }
            }
            
            // Apply sharpening only for very blurry images
            if quality_stats.sharpness < 0.2 || settings.ocr_sharpening_strength > 0.5 {
                processed_gray = self.sharpen_image(processed_gray, settings)?;
                preprocessing_applied.push("Image sharpening".to_string());
            }
            
            // Apply morphological operations only if explicitly enabled and image needs it
            if settings.ocr_morphological_operations && quality_stats.noise_level > 0.15 {
                processed_gray = self.apply_morphological_operations(processed_gray)?;
                preprocessing_applied.push("Morphological operations".to_string());
            }
        }
        
        // Save processed image to temporary file
        let temp_filename = format!("processed_{}_{}.png", 
            std::process::id(), 
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_millis()
        );
        let temp_path = format!("{}/{}", self.temp_dir, temp_filename);
        
        let dynamic_processed = DynamicImage::ImageLuma8(processed_gray);
        dynamic_processed.save(&temp_path)?;
        
        info!("Processed image saved to: {}", temp_path);
        Ok((temp_path, preprocessing_applied))
    }

    /// Determine if image needs enhancement based on quality thresholds
    #[cfg(feature = "ocr")]
    fn needs_enhancement(&self, stats: &ImageQualityStats, settings: &Settings) -> bool {
        // If user wants to skip enhancement entirely, respect that
        if settings.ocr_skip_enhancement {
            info!("OCR enhancement disabled by user setting");
            return false;
        }
        
        // Use user-configurable thresholds
        let brightness_threshold = settings.ocr_quality_threshold_brightness;
        let contrast_threshold = settings.ocr_quality_threshold_contrast;
        let noise_threshold = settings.ocr_quality_threshold_noise;
        let sharpness_threshold = settings.ocr_quality_threshold_sharpness;
        
        // Check if any metric falls below acceptable quality thresholds
        let needs_brightness_fix = stats.average_brightness < brightness_threshold;
        let needs_contrast_fix = stats.contrast_ratio < contrast_threshold;
        let needs_noise_fix = stats.noise_level > noise_threshold;
        let needs_sharpening = stats.sharpness < sharpness_threshold;
        
        // Also check if user has explicitly enabled aggressive enhancement
        let user_wants_enhancement = settings.ocr_brightness_boost > 0.0 ||
                                    settings.ocr_contrast_multiplier > 1.0 ||
                                    settings.ocr_noise_reduction_level > 1 ||
                                    settings.ocr_sharpening_strength > 0.0;
        
        let needs_enhancement = needs_brightness_fix || needs_contrast_fix || needs_noise_fix || needs_sharpening || user_wants_enhancement;
        
        info!("Enhancement decision: brightness_ok={}, contrast_ok={}, noise_ok={}, sharpness_ok={}, user_enhancement={}, needs_enhancement={}", 
              !needs_brightness_fix, !needs_contrast_fix, !needs_noise_fix, !needs_sharpening, user_wants_enhancement, needs_enhancement);
        
        needs_enhancement
    }
    
    /// Configure Tesseract with optimal settings
    #[cfg(feature = "ocr")]
    fn configure_tesseract(&self, image_path: &str, settings: &Settings) -> Result<Tesseract> {
        let mut tesseract = Tesseract::new(None, Some(&settings.ocr_language))?;
        
        // Set the image
        tesseract = tesseract.set_image(image_path)?;
        
        // Configure Page Segmentation Mode (PSM)
        let psm = match settings.ocr_page_segmentation_mode {
            0 => PageSegMode::PsmOsdOnly,
            1 => PageSegMode::PsmAutoOsd,
            2 => PageSegMode::PsmAutoOnly,
            3 => PageSegMode::PsmAuto,
            4 => PageSegMode::PsmSingleColumn,
            5 => PageSegMode::PsmSingleBlockVertText,
            6 => PageSegMode::PsmSingleBlock,
            7 => PageSegMode::PsmSingleLine,
            8 => PageSegMode::PsmSingleWord,
            9 => PageSegMode::PsmCircleWord,
            10 => PageSegMode::PsmSingleChar,
            11 => PageSegMode::PsmSparseText,
            12 => PageSegMode::PsmSparseTextOsd,
            13 => PageSegMode::PsmRawLine,
            _ => PageSegMode::PsmAuto, // Default fallback
        };
        tesseract.set_page_seg_mode(psm);
        
        // Configure OCR Engine Mode (OEM)
        let _oem = match settings.ocr_engine_mode {
            0 => OcrEngineMode::TesseractOnly,
            1 => OcrEngineMode::LstmOnly,
            2 => OcrEngineMode::TesseractLstmCombined,
            3 => OcrEngineMode::Default,
            _ => OcrEngineMode::Default, // Default fallback
        };
        
        // Note: set_engine_mode may not be available in the current tesseract crate version
        // We'll configure this differently if needed
        
        // Basic configuration - skip advanced settings that might cause issues
        // Only set essential variables that are widely supported
        
        Ok(tesseract)
    }
    
    /// Calculate overall confidence score using Tesseract's mean confidence
    #[cfg(feature = "ocr")]
    fn calculate_overall_confidence(&self, tesseract: &mut Tesseract) -> Result<f32> {
        // Use Tesseract's built-in mean confidence calculation
        let confidence = tesseract.mean_text_conf();
        
        // Convert from i32 to f32 and ensure it's within valid range
        let confidence_f32 = confidence as f32;
        
        // Clamp confidence to valid range (0.0 to 100.0)
        let clamped_confidence = confidence_f32.max(0.0).min(100.0);
        
        debug!("Tesseract confidence: {} -> {:.1}%", confidence, clamped_confidence);
        
        Ok(clamped_confidence)
    }
    
    /// Detect and correct image orientation
    #[cfg(feature = "ocr")]
    fn detect_and_correct_orientation(&self, img: DynamicImage) -> Result<DynamicImage> {
        // For now, we'll implement basic rotation detection
        // In a production system, you might want to use Tesseract's OSD or advanced algorithms
        let (width, height) = img.dimensions();
        
        // If image is wider than tall by significant margin, it might need rotation
        if width as f32 / height as f32 > 2.0 {
            Ok(img.rotate90())
        } else {
            Ok(img)
        }
    }
    
    /// Smart resize for OCR - optimize image size for best OCR performance
    #[cfg(feature = "ocr")]
    fn smart_resize_for_ocr(&self, img: DynamicImage, _target_dpi: i32) -> Result<DynamicImage> {
        let (width, height) = img.dimensions();
        let max_dimension = width.max(height);
        let min_dimension = width.min(height);
        
        // Calculate optimal dimensions for OCR
        let mut new_width = width;
        let mut new_height = height;
        
        // Scale DOWN large images for better OCR performance and memory efficiency
        if max_dimension > 2048 {
            let scale_factor = 2048.0 / max_dimension as f32;
            new_width = (width as f32 * scale_factor) as u32;
            new_height = (height as f32 * scale_factor) as u32;
            info!("Scaling down large image ({}x{}) by factor {:.2}x to {}x{} for optimal OCR", 
                  width, height, scale_factor, new_width, new_height);
        }
        // Scale UP very small images that would produce poor OCR results
        else if min_dimension < 300 {
            let scale_factor = 600.0 / min_dimension as f32;
            new_width = (width as f32 * scale_factor) as u32;
            new_height = (height as f32 * scale_factor) as u32;
            info!("Scaling up small image ({}x{}) by factor {:.2}x to {}x{} for better OCR", 
                  width, height, scale_factor, new_width, new_height);
        }
        
        if new_width != width || new_height != height {
            // Use Lanczos3 for best quality upscaling
            Ok(img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3))
        } else {
            Ok(img)
        }
    }
    
    /// Analyze image quality metrics
    #[cfg(feature = "ocr")]
    fn analyze_image_quality(&self, img: &ImageBuffer<Luma<u8>, Vec<u8>>) -> ImageQualityStats {
        let (width, height) = img.dimensions();
        let pixel_count = (width as u64) * (height as u64);
        
        // For very large images, use sampling to avoid performance issues and overflow
        let (average_brightness, variance) = if pixel_count > 4_000_000 { // > 4 megapixels
            self.analyze_quality_sampled(img)
        } else {
            self.analyze_quality_full(img)
        };
        
        let contrast_ratio = variance.sqrt() / 255.0;
        
        // Estimate noise level using local variance
        let noise_level = self.estimate_noise_level(img);
        
        // Estimate sharpness using gradient magnitude
        let sharpness = self.estimate_sharpness(img);
        
        ImageQualityStats {
            average_brightness,
            contrast_ratio,
            noise_level,
            sharpness,
        }
    }
    
    /// Analyze quality for normal-sized images (< 4 megapixels)
    #[cfg(feature = "ocr")]
    fn analyze_quality_full(&self, img: &ImageBuffer<Luma<u8>, Vec<u8>>) -> (f32, f32) {
        let pixels: Vec<u8> = img.pixels().map(|p| p[0]).collect();
        let pixel_count = pixels.len() as f32;
        
        // Calculate average brightness using u64 to prevent overflow
        let sum: u64 = pixels.iter().map(|&p| p as u64).sum();
        let average_brightness = sum as f32 / pixel_count;
        
        // Calculate variance
        let variance: f32 = pixels.iter()
            .map(|&p| {
                let diff = p as f32 - average_brightness;
                diff * diff
            })
            .sum::<f32>() / pixel_count;
            
        (average_brightness, variance)
    }
    
    /// Analyze quality for large images using sampling
    #[cfg(feature = "ocr")]
    fn analyze_quality_sampled(&self, img: &ImageBuffer<Luma<u8>, Vec<u8>>) -> (f32, f32) {
        let (width, height) = img.dimensions();
        let mut pixel_sum = 0u64;
        let mut sample_count = 0u32;
        
        // Sample every 10th pixel to avoid overflow and improve performance
        for y in (0..height).step_by(10) {
            for x in (0..width).step_by(10) {
                pixel_sum += img.get_pixel(x, y)[0] as u64;
                sample_count += 1;
            }
        }
        
        let average_brightness = if sample_count > 0 {
            pixel_sum as f32 / sample_count as f32
        } else {
            128.0 // Default middle brightness
        };
        
        // Calculate variance using sampled pixels
        let mut variance_sum = 0.0f32;
        for y in (0..height).step_by(10) {
            for x in (0..width).step_by(10) {
                let pixel_value = img.get_pixel(x, y)[0] as f32;
                let diff = pixel_value - average_brightness;
                variance_sum += diff * diff;
            }
        }
        
        let variance = if sample_count > 0 {
            variance_sum / sample_count as f32
        } else {
            0.0
        };
        
        (average_brightness, variance)
    }
    
    /// Estimate noise level in image
    #[cfg(feature = "ocr")]
    fn estimate_noise_level(&self, img: &ImageBuffer<Luma<u8>, Vec<u8>>) -> f32 {
        let (width, height) = img.dimensions();
        let mut noise_sum = 0.0f32;
        let mut sample_count = 0u32;
        
        // Sample every 10th pixel to estimate noise
        for y in (5..height-5).step_by(10) {
            for x in (5..width-5).step_by(10) {
                let center = img.get_pixel(x, y)[0] as f32;
                let mut neighbor_sum = 0.0f32;
                let mut neighbor_count = 0u32;
                
                // Check 3x3 neighborhood
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 { continue; }
                        let neighbor = img.get_pixel((x as i32 + dx) as u32, (y as i32 + dy) as u32)[0] as f32;
                        neighbor_sum += neighbor;
                        neighbor_count += 1;
                    }
                }
                
                let neighbor_avg = neighbor_sum / neighbor_count as f32;
                let local_variance = (center - neighbor_avg).abs();
                noise_sum += local_variance;
                sample_count += 1;
            }
        }
        
        if sample_count > 0 {
            (noise_sum / sample_count as f32) / 255.0
        } else {
            0.0
        }
    }
    
    /// Estimate image sharpness using gradient magnitude
    #[cfg(feature = "ocr")]
    fn estimate_sharpness(&self, img: &ImageBuffer<Luma<u8>, Vec<u8>>) -> f32 {
        let (width, height) = img.dimensions();
        let mut gradient_sum = 0.0f32;
        let mut sample_count = 0u64; // Use u64 to prevent overflow
        
        // For large images, sample pixels to avoid performance issues and overflow
        let total_pixels = (width as u64) * (height as u64);
        let step_size = if total_pixels > 4_000_000 { 10 } else { 1 }; // Sample every 10th pixel for large images
        
        // Calculate gradients for interior pixels
        for y in (1..height-1).step_by(step_size) {
            for x in (1..width-1).step_by(step_size) {
                let _center = img.get_pixel(x, y)[0] as f32;
                let left = img.get_pixel(x-1, y)[0] as f32;
                let right = img.get_pixel(x+1, y)[0] as f32;
                let top = img.get_pixel(x, y-1)[0] as f32;
                let bottom = img.get_pixel(x, y+1)[0] as f32;
                
                let grad_x = (right - left) / 2.0;
                let grad_y = (bottom - top) / 2.0;
                let gradient_magnitude = (grad_x * grad_x + grad_y * grad_y).sqrt();
                
                gradient_sum += gradient_magnitude;
                sample_count += 1;
            }
        }
        
        if sample_count > 0 {
            (gradient_sum / sample_count as f32) / 255.0
        } else {
            0.0
        }
    }
    
    /// Enhanced brightness and contrast correction for dim images
    #[cfg(feature = "ocr")]
    fn enhance_brightness_and_contrast(&self, img: ImageBuffer<Luma<u8>, Vec<u8>>, stats: &ImageQualityStats, settings: &Settings) -> Result<ImageBuffer<Luma<u8>, Vec<u8>>> {
        let (width, height) = img.dimensions();
        let mut enhanced = ImageBuffer::new(width, height);
        
        // Calculate enhancement parameters based on image statistics and user settings
        let brightness_boost = if settings.ocr_brightness_boost > 0.0 {
            settings.ocr_brightness_boost  // Use user-configured value
        } else if stats.average_brightness < 50.0 {
            60.0 - stats.average_brightness  // Aggressive boost for very dim images
        } else if stats.average_brightness < 80.0 {
            30.0 - (stats.average_brightness - 50.0) * 0.5  // Moderate boost
        } else {
            0.0  // No boost needed
        };
        
        let contrast_multiplier = if settings.ocr_contrast_multiplier > 0.0 {
            settings.ocr_contrast_multiplier  // Use user-configured value
        } else if stats.contrast_ratio < 0.2 {
            2.5  // Aggressive contrast boost for flat images
        } else if stats.contrast_ratio < 0.4 {
            1.8  // Moderate contrast boost
        } else {
            1.2  // Slight boost
        };
        
        info!("Applying brightness boost: {:.1}, contrast multiplier: {:.1}", brightness_boost, contrast_multiplier);
        
        for (x, y, pixel) in img.enumerate_pixels() {
            let original_value = pixel[0] as f32;
            
            // Apply brightness and contrast enhancement
            let enhanced_value = ((original_value + brightness_boost) * contrast_multiplier).round();
            let clamped_value = enhanced_value.max(0.0).min(255.0) as u8;
            
            enhanced.put_pixel(x, y, Luma([clamped_value]));
        }
        
        Ok(enhanced)
    }
    
    /// Adaptive noise removal based on detected noise level
    #[cfg(feature = "ocr")]
    fn adaptive_noise_removal(&self, img: ImageBuffer<Luma<u8>, Vec<u8>>, stats: &ImageQualityStats, settings: &Settings) -> Result<ImageBuffer<Luma<u8>, Vec<u8>>> {
        let mut processed = img;
        
        // Use user-configured noise reduction level if specified
        let noise_level = if settings.ocr_noise_reduction_level > 0 {
            settings.ocr_noise_reduction_level
        } else if stats.noise_level > 0.2 {
            3  // Heavy noise
        } else if stats.noise_level > 0.1 {
            2  // Moderate noise
        } else {
            1  // Light noise
        };
        
        match noise_level {
            3 => {
                // Heavy noise - apply multiple filters
                processed = median_filter(&processed, 2, 2);  // Larger median filter
                processed = gaussian_blur_f32(&processed, 0.8);  // More blur
                info!("Applied heavy noise reduction");
            },
            2 => {
                // Moderate noise
                processed = median_filter(&processed, 1, 1);
                processed = gaussian_blur_f32(&processed, 0.5);
                info!("Applied moderate noise reduction");
            },
            1 | _ => {
                // Light noise or clean image
                processed = median_filter(&processed, 1, 1);
                info!("Applied light noise reduction");
            }
        }
        
        Ok(processed)
    }
    
    /// Adaptive contrast enhancement based on image quality
    #[cfg(feature = "ocr")]
    fn adaptive_contrast_enhancement(&self, img: ImageBuffer<Luma<u8>, Vec<u8>>, stats: &ImageQualityStats, settings: &Settings) -> Result<ImageBuffer<Luma<u8>, Vec<u8>>> {
        // Choose threshold size based on image dimensions and quality
        let (width, height) = img.dimensions();
        let min_dimension = width.min(height);
        
        // Check if image is too large for safe adaptive threshold processing
        // The integral image calculation can overflow with large images
        if width as u64 * height as u64 > 1_500_000 {
            info!("Image too large for adaptive threshold ({}x{}), using alternative contrast enhancement", width, height);
            return self.apply_alternative_contrast_enhancement(img, stats, settings);
        }
        
        let threshold_size = if settings.ocr_adaptive_threshold_window_size > 0 {
            // Use user-configured window size
            settings.ocr_adaptive_threshold_window_size as u32
        } else if stats.contrast_ratio < 0.2 {
            // Low contrast - use smaller windows for more aggressive local adaptation
            (min_dimension / 20).max(11).min(31)
        } else {
            // Good contrast - use larger windows
            (min_dimension / 15).max(15).min(41)
        };
        
        // Ensure odd number for threshold size
        let threshold_size = if threshold_size % 2 == 0 { threshold_size + 1 } else { threshold_size };
        
        info!("Applying adaptive threshold with window size: {}", threshold_size);
        
        // Wrap in panic-safe block to catch overflow errors
        let enhanced = catch_unwind(AssertUnwindSafe(|| {
            adaptive_threshold(&img, threshold_size)
        }));
        
        match enhanced {
            Ok(result) => Ok(result),
            Err(_) => {
                warn!("Adaptive threshold panicked (likely overflow), using alternative method");
                self.apply_alternative_contrast_enhancement(img, stats, settings)
            }
        }
    }
    
    /// Alternative contrast enhancement for large images to avoid overflow
    #[cfg(feature = "ocr")]
    fn apply_alternative_contrast_enhancement(&self, img: ImageBuffer<Luma<u8>, Vec<u8>>, stats: &ImageQualityStats, settings: &Settings) -> Result<ImageBuffer<Luma<u8>, Vec<u8>>> {
        let (width, height) = img.dimensions();
        let mut enhanced = ImageBuffer::new(width, height);
        
        // Use histogram equalization instead of adaptive threshold for large images
        if settings.ocr_histogram_equalization {
            info!("Applying histogram equalization for contrast enhancement (user enabled)");
        } else {
            info!("Applying histogram equalization for contrast enhancement (fallback)");
        }
        
        // Calculate histogram using u64 to prevent overflow
        let mut histogram = [0u64; 256];
        for pixel in img.pixels() {
            histogram[pixel[0] as usize] += 1;
        }
        
        // Calculate cumulative distribution function
        let total_pixels = (width as u64) * (height as u64);
        let mut cdf = [0u64; 256];
        cdf[0] = histogram[0];
        for i in 1..256 {
            cdf[i] = cdf[i - 1] + histogram[i];
        }
        
        // Create lookup table for histogram equalization
        let mut lookup = [0u8; 256];
        for i in 0..256 {
            if cdf[i] > 0 {
                lookup[i] = ((cdf[i] as f64 / total_pixels as f64) * 255.0) as u8;
            }
        }
        
        // Apply histogram equalization
        for (x, y, pixel) in img.enumerate_pixels() {
            let old_value = pixel[0];
            let new_value = lookup[old_value as usize];
            enhanced.put_pixel(x, y, Luma([new_value]));
        }
        
        // Apply additional contrast stretching if needed
        if stats.contrast_ratio < 0.3 {
            enhanced = self.apply_contrast_stretching(enhanced)?;
        }
        
        Ok(enhanced)
    }
    
    /// Apply contrast stretching to improve dynamic range
    #[cfg(feature = "ocr")]
    fn apply_contrast_stretching(&self, img: ImageBuffer<Luma<u8>, Vec<u8>>) -> Result<ImageBuffer<Luma<u8>, Vec<u8>>> {
        let (width, height) = img.dimensions();
        let mut enhanced = ImageBuffer::new(width, height);
        
        // Find min and max values
        let mut min_val = 255u8;
        let mut max_val = 0u8;
        
        for pixel in img.pixels() {
            let val = pixel[0];
            min_val = min_val.min(val);
            max_val = max_val.max(val);
        }
        
        // Avoid division by zero
        if max_val == min_val {
            return Ok(img);
        }
        
        let range = max_val - min_val;
        
        // Apply contrast stretching
        for (x, y, pixel) in img.enumerate_pixels() {
            let old_value = pixel[0];
            let new_value = (((old_value - min_val) as f32 / range as f32) * 255.0) as u8;
            enhanced.put_pixel(x, y, Luma([new_value]));
        }
        
        Ok(enhanced)
    }
    
    /// Sharpen blurry images
    #[cfg(feature = "ocr")]
    fn sharpen_image(&self, img: ImageBuffer<Luma<u8>, Vec<u8>>, settings: &Settings) -> Result<ImageBuffer<Luma<u8>, Vec<u8>>> {
        let (width, height) = img.dimensions();
        let mut sharpened = ImageBuffer::new(width, height);
        
        // Unsharp mask kernel - enhances edges
        let kernel = [
            [0.0, -1.0, 0.0],
            [-1.0, 5.0, -1.0],
            [0.0, -1.0, 0.0],
        ];
        
        for y in 1..height-1 {
            for x in 1..width-1 {
                let mut sum = 0.0;
                
                for ky in 0..3 {
                    for kx in 0..3 {
                        let px = img.get_pixel(x + kx - 1, y + ky - 1)[0] as f32;
                        sum += px * kernel[ky as usize][kx as usize];
                    }
                }
                
                let sharpened_value = sum.round().max(0.0).min(255.0) as u8;
                sharpened.put_pixel(x, y, Luma([sharpened_value]));
            }
        }
        
        // Copy border pixels
        for y in 0..height {
            for x in 0..width {
                if x == 0 || x == width-1 || y == 0 || y == height-1 {
                    sharpened.put_pixel(x, y, *img.get_pixel(x, y));
                }
            }
        }
        
        info!("Applied image sharpening");
        Ok(sharpened)
    }
    
    /// Apply morphological operations for text clarity
    #[cfg(feature = "ocr")]
    fn apply_morphological_operations(&self, img: ImageBuffer<Luma<u8>, Vec<u8>>) -> Result<ImageBuffer<Luma<u8>, Vec<u8>>> {
        // Apply opening to remove small noise
        let opened = open(&img, Norm::LInf, 1);
        
        // Apply closing to fill small gaps in text
        let closed = close(&opened, Norm::LInf, 1);
        
        Ok(closed)
    }
    
    /// Extract text from PDF with size and time limits
    #[cfg(feature = "ocr")]
    pub async fn extract_text_from_pdf(&self, file_path: &str, settings: &Settings) -> Result<OcrResult> {
        let start_time = std::time::Instant::now();
        info!("Extracting text from PDF: {}", file_path);
        
        // Check file size before loading into memory
        let metadata = tokio::fs::metadata(file_path).await?;
        let file_size = metadata.len();
        
        // Limit PDF size to 100MB to prevent memory exhaustion
        const MAX_PDF_SIZE: u64 = 100 * 1024 * 1024; // 100MB
        if file_size > MAX_PDF_SIZE {
            return Err(anyhow!(
                "PDF file too large: {:.1} MB (max: {:.1} MB). Consider splitting the PDF.",
                file_size as f64 / (1024.0 * 1024.0),
                MAX_PDF_SIZE as f64 / (1024.0 * 1024.0)
            ));
        }
        
        let bytes = tokio::fs::read(file_path).await?;
        
        // Check if it's a valid PDF (handles leading null bytes)
        if !is_valid_pdf(&bytes) {
            return Err(anyhow!(
                "Invalid PDF file: Missing or corrupted PDF header. File size: {} bytes, Header: {:?}", 
                bytes.len(),
                bytes.get(0..50).unwrap_or(&[]).iter().map(|&b| {
                    if b >= 32 && b <= 126 { b as char } else { '.' }
                }).collect::<String>()
            ));
        }
        
        // Clean the PDF data (remove leading null bytes)
        let clean_bytes = clean_pdf_data(&bytes);
        
        // Add timeout and panic recovery for PDF extraction
        let extraction_result = tokio::time::timeout(
            std::time::Duration::from_secs(120), // 2 minute timeout
            tokio::task::spawn_blocking(move || {
                // Catch panics from pdf-extract library
                catch_unwind(AssertUnwindSafe(|| {
                    pdf_extract::extract_text_from_mem(&clean_bytes)
                }))
            })
        ).await;
        
        let text = match extraction_result {
            Ok(Ok(Ok(Ok(text)))) => text,
            Ok(Ok(Ok(Err(e)))) => {
                return Err(anyhow!(
                    "PDF text extraction failed for file '{}' (size: {} bytes): {}. This may indicate a corrupted or unsupported PDF format.",
                    file_path, file_size, e
                ));
            }
            Ok(Ok(Err(_panic))) => {
                // pdf-extract panicked (e.g., missing unicode map, corrupted font encoding)
                // For now, gracefully handle this common issue
                use tracing::debug;
                debug!("PDF text extraction failed for '{}' due to font encoding issues. This is a known limitation with certain PDF files.", file_path);
                
                return Err(anyhow!(
                    "PDF text extraction failed due to font encoding issues in '{}' (size: {} bytes). This PDF uses non-standard fonts or character encoding that cannot be processed. To extract text from this PDF, consider: 1) Converting it to images and uploading those instead, 2) Using a different PDF viewer to re-save the PDF with standard encoding, or 3) Using external tools to convert the PDF to a more compatible format.",
                    file_path, file_size
                ));
            }
            Ok(Err(e)) => {
                return Err(anyhow!("PDF extraction task failed: {}", e));
            }
            Err(_) => {
                return Err(anyhow!(
                    "PDF extraction timed out after 2 minutes for file '{}' (size: {} bytes). The PDF may be corrupted or too complex.",
                    file_path, file_size
                ));
            }
        };
        
        // Limit extracted text size to prevent memory issues
        const MAX_TEXT_SIZE: usize = 10 * 1024 * 1024; // 10MB of text
        let trimmed_text = if text.len() > MAX_TEXT_SIZE {
            warn!("PDF text too large ({} chars), truncating to {} chars", text.len(), MAX_TEXT_SIZE);
            format!("{}... [TEXT TRUNCATED DUE TO SIZE]", &text[..MAX_TEXT_SIZE])
        } else {
            text.trim().to_string()
        };
        
        let processing_time = start_time.elapsed().as_millis() as u64;
        let word_count = self.count_words_safely(&trimmed_text);
        
        // Debug logging to understand PDF extraction issues
        debug!(
            "PDF extraction debug - File: '{}' | Raw text length: {} | Trimmed text length: {} | Word count: {} | First 200 chars: {:?}",
            file_path,
            text.len(),
            trimmed_text.len(),
            word_count,
            trimmed_text.chars().take(200).collect::<String>()
        );
        
        // Smart detection: assess if text extraction quality is good enough
        if self.is_text_extraction_quality_sufficient(&trimmed_text, word_count, file_size) {
            info!("PDF text extraction successful for '{}', using extracted text", file_path);
            Ok(OcrResult {
                text: trimmed_text,
                confidence: 95.0, // PDF text extraction is generally high confidence
                processing_time_ms: processing_time,
                word_count,
                preprocessing_applied: vec!["PDF text extraction".to_string()],
                processed_image_path: None,
            })
        } else {
            info!("PDF text extraction insufficient for '{}' ({} words), falling back to OCR", file_path, word_count);
            // Fall back to OCR using ocrmypdf
            self.extract_text_from_pdf_with_ocr(file_path, settings, start_time).await
        }
    }
    
    /// Assess if text extraction quality is sufficient or if OCR fallback is needed
    #[cfg(feature = "ocr")]
    fn is_text_extraction_quality_sufficient(&self, text: &str, word_count: usize, file_size: u64) -> bool {
        // If we got no words at all, definitely need OCR
        if word_count == 0 {
            return false;
        }
        
        // For very small files, low word count might be normal
        if file_size < 50_000 && word_count >= 1 {
            return true;
        }
        
        // Calculate word density (words per KB)
        let file_size_kb = (file_size as f64) / 1024.0;
        let word_density = (word_count as f64) / file_size_kb;
        
        // Reasonable thresholds based on typical PDF content:
        // - Text-based PDFs typically have 50-200 words per KB
        // - Below 5 words per KB suggests mostly images/scanned content
        const MIN_WORD_DENSITY: f64 = 5.0;
        const MIN_WORDS_FOR_LARGE_FILES: usize = 10;
        
        if word_density < MIN_WORD_DENSITY && word_count < MIN_WORDS_FOR_LARGE_FILES {
            debug!("PDF appears to be image-based: {} words in {:.1} KB (density: {:.2} words/KB)", 
                   word_count, file_size_kb, word_density);
            return false;
        }
        
        // Additional check: if text is mostly non-alphanumeric, might be extraction artifacts
        let alphanumeric_chars = text.chars().filter(|c| c.is_alphanumeric()).count();
        let alphanumeric_ratio = if text.len() > 0 {
            (alphanumeric_chars as f64) / (text.len() as f64)
        } else {
            0.0
        };
        
        // If less than 30% alphanumeric content, likely poor extraction
        if alphanumeric_ratio < 0.3 {
            debug!("PDF text has low alphanumeric content: {:.1}% ({} of {} chars)", 
                   alphanumeric_ratio * 100.0, alphanumeric_chars, text.len());
            return false;
        }
        
        debug!("PDF text extraction quality sufficient: {} words, {:.2} words/KB, {:.1}% alphanumeric", 
               word_count, word_density, alphanumeric_ratio * 100.0);
        true
    }
    
    /// Extract text from PDF using OCR (ocrmypdf) for image-based or poor-quality PDFs
    #[cfg(feature = "ocr")]
    async fn extract_text_from_pdf_with_ocr(&self, file_path: &str, settings: &Settings, start_time: std::time::Instant) -> Result<OcrResult> {
        info!("Starting OCR extraction for PDF: {}", file_path);
        
        // Check if ocrmypdf is available
        if !self.is_ocrmypdf_available().await {
            return Err(anyhow!(
                "ocrmypdf is not available on this system. To extract text from image-based PDFs like '{}', please install ocrmypdf. \
                On Ubuntu/Debian: 'apt-get install ocrmypdf'. \
                On macOS: 'brew install ocrmypdf'. \
                Alternatively, convert the PDF to images and upload those instead.",
                file_path
            ));
        }
        
        // Generate temporary file path for OCR'd PDF
        let temp_ocr_filename = format!("ocr_{}_{}.pdf", 
            std::process::id(), 
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_millis()
        );
        let temp_ocr_path = format!("{}/{}", self.temp_dir, temp_ocr_filename);
        
        // Run ocrmypdf to create searchable PDF
        let ocrmypdf_result = tokio::time::timeout(
            std::time::Duration::from_secs(300), // 5 minute timeout for OCR
            tokio::task::spawn_blocking({
                let file_path = file_path.to_string();
                let temp_ocr_path = temp_ocr_path.clone();
                move || {
                    std::process::Command::new("ocrmypdf")
                        .arg("--force-ocr")  // OCR even if text is detected
                        .arg("-O2")          // Optimize level 2 (balanced quality/speed)
                        .arg("--deskew")     // Correct skewed pages
                        .arg("--clean")      // Clean up artifacts
                        .arg("--language")
                        .arg("eng")          // English language
                        .arg(&file_path)
                        .arg(&temp_ocr_path)
                        .output()
                }
            })
        ).await;
        
        let ocrmypdf_output = match ocrmypdf_result {
            Ok(Ok(output)) => output?,
            Ok(Err(e)) => return Err(anyhow!("Failed to join ocrmypdf task: {}", e)),
            Err(_) => return Err(anyhow!("ocrmypdf timed out after 5 minutes for file '{}'", file_path)),
        };
        
        if !ocrmypdf_output.status.success() {
            let stderr = String::from_utf8_lossy(&ocrmypdf_output.stderr);
            let stdout = String::from_utf8_lossy(&ocrmypdf_output.stdout);
            return Err(anyhow!(
                "ocrmypdf failed for '{}': Exit code {}\nStderr: {}\nStdout: {}",
                file_path, ocrmypdf_output.status.code().unwrap_or(-1), stderr, stdout
            ));
        }
        
        // Extract text from the OCR'd PDF
        let ocr_text_result = tokio::task::spawn_blocking({
            let temp_ocr_path = temp_ocr_path.clone();
            move || -> Result<String> {
                let bytes = std::fs::read(&temp_ocr_path)?;
                let text = pdf_extract::extract_text_from_mem(&bytes)?;
                Ok(text.trim().to_string())
            }
        }).await??;
        
        // Clean up temporary file
        let _ = tokio::fs::remove_file(&temp_ocr_path).await;
        
        let processing_time = start_time.elapsed().as_millis() as u64;
        let word_count = self.count_words_safely(&ocr_text_result);
        
        info!("OCR extraction completed for '{}': {} words in {}ms", 
              file_path, word_count, processing_time);
        
        Ok(OcrResult {
            text: ocr_text_result,
            confidence: 85.0, // OCR is generally lower confidence than direct text extraction
            processing_time_ms: processing_time,
            word_count,
            preprocessing_applied: vec!["OCR via ocrmypdf".to_string()],
            processed_image_path: None,
        })
    }
    
    /// Check if ocrmypdf is available on the system
    #[cfg(feature = "ocr")]
    async fn is_ocrmypdf_available(&self) -> bool {
        match tokio::process::Command::new("ocrmypdf")
            .arg("--version")
            .output()
            .await
        {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }
    
    #[cfg(not(feature = "ocr"))]
    fn is_text_extraction_quality_sufficient(&self, _text: &str, _word_count: usize, _file_size: u64) -> bool {
        // When OCR is disabled, always accept text extraction results
        true
    }
    
    #[cfg(not(feature = "ocr"))]
    async fn is_ocrmypdf_available(&self) -> bool {
        false // OCR feature not enabled
    }
    
    #[cfg(not(feature = "ocr"))]
    async fn extract_text_from_pdf_with_ocr(&self, file_path: &str, _settings: &Settings, _start_time: std::time::Instant) -> Result<OcrResult> {
        Err(anyhow::anyhow!("OCR feature not enabled - cannot process image-based PDF: {}", file_path))
    }
    
    /// Resolve file path to actual location, handling both old and new directory structures
    async fn resolve_file_path(&self, file_path: &str) -> Result<String> {
        // Use the FileService's resolve_file_path method
        self.file_service.resolve_file_path(file_path).await
    }

    /// Extract text from any supported file type with enhanced logging
    pub async fn extract_text_with_context(&self, file_path: &str, mime_type: &str, filename: &str, file_size: i64, settings: &Settings) -> Result<OcrResult> {
        // Format file size for better readability
        let file_size_mb = file_size as f64 / (1024.0 * 1024.0);
        
        info!(
            "Starting OCR extraction | File: '{}' | Type: {} | Size: {:.2} MB | Path: {}", 
            filename, mime_type, file_size_mb, file_path
        );
        
        self.extract_text(file_path, mime_type, settings).await
    }

    /// Extract text from any supported file type
    pub async fn extract_text(&self, file_path: &str, mime_type: &str, settings: &Settings) -> Result<OcrResult> {
        // Resolve the actual file path
        let resolved_path = self.resolve_file_path(file_path).await?;
        match mime_type {
            "application/pdf" => {
                #[cfg(feature = "ocr")]
                {
                    self.extract_text_from_pdf(&resolved_path, settings).await
                }
                #[cfg(not(feature = "ocr"))]
                {
                    Err(anyhow::anyhow!("OCR feature not enabled"))
                }
            }
            mime if mime.starts_with("image/") => {
                #[cfg(feature = "ocr")]
                {
                    self.extract_text_from_image(&resolved_path, settings).await
                }
                #[cfg(not(feature = "ocr"))]
                {
                    Err(anyhow::anyhow!("OCR feature not enabled"))
                }
            }
            "text/plain" => {
                let start_time = std::time::Instant::now();
                
                // Check file size before loading into memory
                let metadata = tokio::fs::metadata(&resolved_path).await?;
                let file_size = metadata.len();
                
                // Limit text file size to 50MB to prevent memory exhaustion
                const MAX_TEXT_FILE_SIZE: u64 = 50 * 1024 * 1024; // 50MB
                if file_size > MAX_TEXT_FILE_SIZE {
                    return Err(anyhow!(
                        "Text file too large: {:.1} MB (max: {:.1} MB). Consider splitting the file.",
                        file_size as f64 / (1024.0 * 1024.0),
                        MAX_TEXT_FILE_SIZE as f64 / (1024.0 * 1024.0)
                    ));
                }
                
                let text = tokio::fs::read_to_string(&resolved_path).await?;
                
                // Limit text content size in memory
                const MAX_TEXT_CONTENT_SIZE: usize = 10 * 1024 * 1024; // 10MB of text content
                let trimmed_text = if text.len() > MAX_TEXT_CONTENT_SIZE {
                    warn!("Text file content too large ({} chars), truncating to {} chars", text.len(), MAX_TEXT_CONTENT_SIZE);
                    format!("{}... [TEXT TRUNCATED DUE TO SIZE]", &text[..MAX_TEXT_CONTENT_SIZE])
                } else {
                    text.trim().to_string()
                };
                
                let processing_time = start_time.elapsed().as_millis() as u64;
                let word_count = self.count_words_safely(&trimmed_text);
                
                Ok(OcrResult {
                    text: trimmed_text,
                    confidence: 100.0, // Plain text is 100% confident
                    processing_time_ms: processing_time,
                    word_count,
                    preprocessing_applied: vec!["Plain text read".to_string()],
                    processed_image_path: None, // No image processing for plain text
                })
            }
            _ => Err(anyhow::anyhow!("Unsupported file type: {}", mime_type)),
        }
    }
    
    /// Safely count words to prevent overflow on very large texts
    #[cfg(feature = "ocr")]
    pub fn count_words_safely(&self, text: &str) -> usize {
        // For very large texts, sample to estimate word count to prevent overflow
        if text.len() > 1_000_000 { // > 1MB of text
            // Sample first 100KB and extrapolate
            let sample_size = 100_000;
            let sample_text = &text[..sample_size.min(text.len())];
            let sample_words = self.count_words_in_text(sample_text);
            let estimated_total = (sample_words as f64 * (text.len() as f64 / sample_size as f64)) as usize;
            
            // Cap at reasonable maximum to prevent display issues
            estimated_total.min(10_000_000) // Max 10M words
        } else {
            self.count_words_in_text(text)
        }
    }

    #[cfg(feature = "ocr")]
    fn count_words_in_text(&self, text: &str) -> usize {
        let whitespace_words = text.split_whitespace().count();
        
        // If we have exactly 1 "word" but it's very long (likely continuous text), try enhanced detection
        // OR if we have no whitespace words but text exists
        let is_continuous_text = whitespace_words == 1 && text.len() > 15; // 15+ chars suggests it might be continuous
        let is_no_words = whitespace_words == 0 && !text.trim().is_empty();
        
        if is_continuous_text || is_no_words {
            // Count total alphanumeric characters first
            let alphanumeric_chars = text.chars().filter(|c| c.is_alphanumeric()).count();
            
            // If no alphanumeric content, it's pure punctuation/symbols
            if alphanumeric_chars == 0 {
                return 0;
            }
            
            // For continuous text, look for word boundaries using multiple strategies
            let mut word_count = 0;
            
            // Strategy 1: Count transitions from lowercase to uppercase (camelCase detection)
            let chars: Vec<char> = text.chars().collect();
            let mut camel_transitions = 0;
            
            for i in 1..chars.len() {
                let prev_char = chars[i-1];
                let curr_char = chars[i];
                
                // Count transitions from lowercase letter to uppercase letter
                if prev_char.is_lowercase() && curr_char.is_uppercase() {
                    camel_transitions += 1;
                }
                // Count transitions from letter to digit or digit to letter
                else if (prev_char.is_alphabetic() && curr_char.is_numeric()) ||
                        (prev_char.is_numeric() && curr_char.is_alphabetic()) {
                    camel_transitions += 1;
                }
            }
            
            // If we found camelCase transitions, estimate words
            if camel_transitions > 0 {
                word_count = camel_transitions + 1; // +1 for the first word
            }
            
            // Strategy 2: If no camelCase detected, estimate based on character count
            if word_count == 0 {
                // Estimate based on typical word length (4-6 characters per word)
                word_count = (alphanumeric_chars / 5).max(1);
            }
            
            word_count
        } else {
            whitespace_words
        }
    }
    
    /// Validate OCR result quality
    #[cfg(feature = "ocr")]
    pub fn validate_ocr_quality(&self, result: &OcrResult, settings: &Settings) -> bool {
        // Check minimum confidence threshold
        if result.confidence < settings.ocr_min_confidence {
            warn!(
                "OCR result below confidence threshold: {:.1}% < {:.1}%", 
                result.confidence, settings.ocr_min_confidence
            );
            return false;
        }
        
        // Check if text is reasonable (not just noise)
        if result.word_count == 0 {
            warn!("OCR result contains no words");
            return false;
        }
        
        // Check for reasonable character distribution
        let total_chars = result.text.len();
        if total_chars == 0 {
            return false;
        }
        
        let alphanumeric_chars = result.text.chars().filter(|c| c.is_alphanumeric()).count();
        let alphanumeric_ratio = alphanumeric_chars as f32 / total_chars as f32;
        
        // Expect at least 30% alphanumeric characters for valid text
        if alphanumeric_ratio < 0.3 {
            warn!(
                "OCR result has low alphanumeric ratio: {:.1}%", 
                alphanumeric_ratio * 100.0
            );
            return false;
        }
        
        true
    }
}

#[cfg(not(feature = "ocr"))]
impl EnhancedOcrService {
    pub async fn extract_text_from_image(&self, _file_path: &str, _settings: &Settings) -> Result<OcrResult> {
        Err(anyhow::anyhow!("OCR feature not enabled"))
    }
    
    pub async fn extract_text_from_pdf(&self, _file_path: &str, _settings: &Settings) -> Result<OcrResult> {
        Err(anyhow::anyhow!("OCR feature not enabled"))
    }
    
    
    pub fn validate_ocr_quality(&self, _result: &OcrResult, _settings: &Settings) -> bool {
        false
    }
}

/// Check if the given bytes represent a valid PDF file
/// Handles PDFs with leading null bytes or whitespace
fn is_valid_pdf(data: &[u8]) -> bool {
    if data.len() < 5 {
        return false;
    }
    
    // Find the first occurrence of "%PDF-" in the first 1KB of the file
    // Some PDFs have leading null bytes or other metadata
    let search_limit = data.len().min(1024);
    let search_data = &data[0..search_limit];
    
    for i in 0..=search_limit.saturating_sub(5) {
        if &search_data[i..i+5] == b"%PDF-" {
            return true;
        }
    }
    
    false
}

/// Remove leading null bytes and return clean PDF data
/// Returns the original data if no PDF header is found
fn clean_pdf_data(data: &[u8]) -> Vec<u8> {
    if data.len() < 5 {
        return data.to_vec();
    }
    
    // Find the first occurrence of "%PDF-" in the first 1KB
    let search_limit = data.len().min(1024);
    
    for i in 0..=search_limit.saturating_sub(5) {
        if &data[i..i+5] == b"%PDF-" {
            return data[i..].to_vec();
        }
    }
    
    // If no PDF header found, return original data
    data.to_vec()
}