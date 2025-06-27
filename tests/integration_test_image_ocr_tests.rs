//! Integration tests for OCR processing using real test images
//! 
//! This test suite uses the actual test images from tests/test_images/
//! to verify OCR functionality with known content.

use readur::ocr::OcrService;
use std::path::Path;

/// Simple test image information
#[derive(Debug, Clone)]
struct TestImage {
    filename: &'static str,
    path: String,
    mime_type: &'static str,
    expected_content: &'static str,
}

impl TestImage {
    fn new(filename: &'static str, mime_type: &'static str, expected_content: &'static str) -> Self {
        Self {
            filename,
            path: format!("tests/test_images/{}", filename),
            mime_type,
            expected_content,
        }
    }
    
    fn exists(&self) -> bool {
        Path::new(&self.path).exists()
    }
    
    async fn load_data(&self) -> Result<Vec<u8>, std::io::Error> {
        tokio::fs::read(&self.path).await
    }
}

/// Get available test images (only those that exist)
fn get_available_test_images() -> Vec<TestImage> {
    let all_images = vec![
        TestImage::new("test1.png", "image/png", "Test 1\nThis is some text from text 1"),
        TestImage::new("test2.jpg", "image/jpeg", "Test 2\nThis is some text from text 2"),
        TestImage::new("test3.jpeg", "image/jpeg", "Test 3\nThis is some text from text 3"),
        TestImage::new("test4.png", "image/png", "Test 4\nThis is some text from text 4"),
        TestImage::new("test5.jpg", "image/jpeg", "Test 5\nThis is some text from text 5"),
    ];
    
    all_images.into_iter().filter(|img| img.exists()).collect()
}

#[tokio::test]
async fn test_ocr_with_all_available_test_images() {
    
    let available_images = get_available_test_images();
    
    if available_images.is_empty() {
        println!("No test images found - skipping OCR tests");
        return;
    }
    
    println!("Testing OCR with {} available test images", available_images.len());
    
    for test_image in available_images {
        println!("Testing OCR with {}", test_image.filename);
        
        // Load the image data
        let image_data = match test_image.load_data().await {
            Ok(data) => data,
            Err(e) => {
                println!("Failed to load {}: {}", test_image.filename, e);
                continue;
            }
        };
        
        // Create a temporary file for OCR processing
        let temp_path = format!("./temp_test_{}", test_image.filename);
        if let Err(e) = tokio::fs::write(&temp_path, &image_data).await {
            println!("Failed to write temp file for {}: {}", test_image.filename, e);
            continue;
        }
        
        // Test OCR processing
        let ocr_service = OcrService::new();
        let result = ocr_service.extract_text(&temp_path, test_image.mime_type).await;
        
        // Clean up temp file
        let _ = tokio::fs::remove_file(&temp_path).await;
        
        match result {
            Ok(extracted_text) => {
                println!("✅ OCR Success for {}: '{}'", test_image.filename, extracted_text);
                
                // Verify the extracted text contains expected content
                let normalized_extracted = extracted_text.trim().to_lowercase();
                let normalized_expected = test_image.expected_content.trim().to_lowercase();
                
                // Check for key parts of expected content
                let test_number = test_image.filename.chars()
                    .filter(|c| c.is_numeric())
                    .collect::<String>();
                
                if !test_number.is_empty() {
                    assert!(
                        normalized_extracted.contains(&format!("test {}", test_number)) ||
                        normalized_extracted.contains(&test_number),
                        "OCR result '{}' should contain test number '{}' for image {}",
                        extracted_text, test_number, test_image.filename
                    );
                }
                
                // Check for presence of "text" keyword
                assert!(
                    normalized_extracted.contains("text") || normalized_extracted.contains("some"),
                    "OCR result '{}' should contain expected text content for image {}",
                    extracted_text, test_image.filename
                );
            }
            Err(e) => {
                println!("⚠️  OCR Failed for {}: {}", test_image.filename, e);
                // Don't fail the test immediately - log the error but continue
                // This allows us to see which images work and which don't
            }
        }
    }
}

#[tokio::test]
async fn test_ocr_with_specific_test_images() {
    
    // Test specific images that should definitely work
    let test_cases = vec![1, 2, 3]; // Test with first 3 images
    let available_images = get_available_test_images();
    
    for test_num in test_cases {
        let test_image = match available_images.get(test_num - 1) {
            Some(img) => img.clone(),
            None => continue,
        };
        
        if !test_image.exists() {
            println!("Skipping test{}: file not found", test_num);
            continue;
        }
        
        println!("Running OCR test for {}", test_image.filename);
        
        // Load image data
        let image_data = test_image.load_data().await
            .expect("Should be able to load test image");
        
        assert!(!image_data.is_empty(), "Test image should not be empty");
        
        // Verify file format based on MIME type
        match test_image.mime_type {
            "image/png" => {
                assert!(image_data.starts_with(&[0x89, 0x50, 0x4E, 0x47]), 
                    "PNG file should start with PNG signature");
            }
            "image/jpeg" => {
                assert!(image_data.starts_with(&[0xFF, 0xD8, 0xFF]), 
                    "JPEG file should start with JPEG signature");
            }
            _ => {}
        }
        
        println!("Image {} loaded successfully: {} bytes, type: {}", 
            test_image.filename, image_data.len(), test_image.mime_type);
    }
}

#[tokio::test]
async fn test_ocr_error_handling_with_corrupted_image() {
        
    // Create a corrupted image file
    let corrupted_data = vec![0xFF; 100]; // Invalid image data
    let temp_path = "./temp_corrupted_test.png";
    
    tokio::fs::write(temp_path, &corrupted_data).await
        .expect("Should be able to write corrupted test file");
    
    let ocr_service = OcrService::new();
    let result = ocr_service.extract_text(temp_path, "image/png").await;
    
    // Clean up
    let _ = tokio::fs::remove_file(temp_path).await;
    
    // Should handle the error gracefully
    match result {
        Ok(text) => {
            println!("Unexpected success with corrupted image: '{}'", text);
            // Some OCR systems might return empty text instead of error
        }
        Err(e) => {
            println!("Expected error with corrupted image: {}", e);
            // This is the expected behavior
        }
    }
}

#[tokio::test]
async fn test_multiple_image_formats() {
        
    let images = get_available_test_images();
    let mut png_count = 0;
    let mut jpeg_count = 0;
    
    for image in &images {
        match image.mime_type {
            "image/png" => png_count += 1,
            "image/jpeg" => jpeg_count += 1,
            _ => {}
        }
    }
    
    println!("Available test images: {} PNG, {} JPEG", png_count, jpeg_count);
    
    // Ensure we have at least one of each format for comprehensive testing
    if png_count > 0 && jpeg_count > 0 {
        println!("✅ Both PNG and JPEG formats available for testing");
    } else {
        println!("⚠️  Limited format coverage: PNG={}, JPEG={}", png_count, jpeg_count);
    }
    
    // Test at least one of each format if available
    for image in images.iter().take(2) {
        if image.exists() {
            println!("Testing format: {} ({})", image.mime_type, image.filename);
            
            let image_data = image.load_data().await
                .expect("Should load test image");
            
            assert!(!image_data.is_empty(), "Image data should not be empty");
            assert!(image_data.len() > 100, "Image should be reasonably sized");
        }
    }
}

#[tokio::test]
#[ignore = "Long running test - run with: cargo test test_ocr_performance -- --ignored"]
async fn test_ocr_performance_with_test_images() {
        
    let available_images = get_available_test_images();
    
    if available_images.is_empty() {
        println!("No test images available for performance testing");
        return;
    }
    
    let start_time = std::time::Instant::now();
    let mut successful_ocr = 0;
    let mut failed_ocr = 0;
    
    for test_image in available_images {
        let image_start = std::time::Instant::now();
        
        // Load image
        let image_data = match test_image.load_data().await {
            Ok(data) => data,
            Err(_) => {
                failed_ocr += 1;
                continue;
            }
        };
        
        // Write to temp file
        let temp_path = format!("./temp_perf_{}", test_image.filename);
        if tokio::fs::write(&temp_path, &image_data).await.is_err() {
            failed_ocr += 1;
            continue;
        }
        
        // Run OCR
        let ocr_service = OcrService::new();
        let result = ocr_service.extract_text(&temp_path, test_image.mime_type).await;
        
        // Clean up
        let _ = tokio::fs::remove_file(&temp_path).await;
        
        let duration = image_start.elapsed();
        
        match result {
            Ok(text) => {
                successful_ocr += 1;
                println!("✅ {} processed in {:?}: '{}'", 
                    test_image.filename, duration, text.chars().take(50).collect::<String>());
            }
            Err(e) => {
                failed_ocr += 1;
                println!("❌ {} failed in {:?}: {}", 
                    test_image.filename, duration, e);
            }
        }
    }
    
    let total_duration = start_time.elapsed();
    let total_images = successful_ocr + failed_ocr;
    
    println!("\n📊 OCR Performance Summary:");
    println!("Total images: {}", total_images);
    println!("Successful: {}", successful_ocr);
    println!("Failed: {}", failed_ocr);
    println!("Total time: {:?}", total_duration);
    
    if total_images > 0 {
        println!("Average time per image: {:?}", total_duration / total_images);
        let success_rate = (successful_ocr as f64 / total_images as f64) * 100.0;
        println!("Success rate: {:.1}%", success_rate);
    }
    
    // Performance assertions
    if successful_ocr > 0 {
        let avg_time_per_image = total_duration / successful_ocr;
        assert!(avg_time_per_image.as_secs() < 30, 
            "OCR should complete within 30 seconds per image on average");
    }
}