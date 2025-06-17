//! Test utilities for loading and working with test images and data
//! 
//! This module provides utilities for loading test images from the tests/test_images/
//! directory and working with them in unit and integration tests.

use std::path::Path;

/// Test image information with expected OCR content
#[derive(Debug, Clone)]
pub struct TestImage {
    pub filename: &'static str,
    pub path: String,
    pub mime_type: &'static str,
    pub expected_content: &'static str,
}

impl TestImage {
    pub fn new(filename: &'static str, mime_type: &'static str, expected_content: &'static str) -> Self {
        Self {
            filename,
            path: format!("tests/test_images/{}", filename),
            mime_type,
            expected_content,
        }
    }
    
    pub fn exists(&self) -> bool {
        Path::new(&self.path).exists()
    }
    
    pub async fn load_data(&self) -> Result<Vec<u8>, std::io::Error> {
        tokio::fs::read(&self.path).await
    }
}

/// Get all available test images with their expected OCR content
pub fn get_test_images() -> Vec<TestImage> {
    vec![
        TestImage::new("test1.png", "image/png", "Test 1\nThis is some text from text 1"),
        TestImage::new("test2.jpg", "image/jpeg", "Test 2\nThis is some text from text 2"),
        TestImage::new("test3.jpeg", "image/jpeg", "Test 3\nThis is some text from text 3"),
        TestImage::new("test4.png", "image/png", "Test 4\nThis is some text from text 4"),
        TestImage::new("test5.jpg", "image/jpeg", "Test 5\nThis is some text from text 5"),
        TestImage::new("test6.jpeg", "image/jpeg", "Test 6\nThis is some text from text 6"),
        TestImage::new("test7.png", "image/png", "Test 7\nThis is some text from text 7"),
        TestImage::new("test8.jpeg", "image/jpeg", "Test 8\nThis is some text from text 8"),
        TestImage::new("test9.png", "image/png", "Test 9\nThis is some text from text 9"),
    ]
}

/// Get a specific test image by number (1-9)
pub fn get_test_image(number: u8) -> Option<TestImage> {
    if number < 1 || number > 9 {
        return None;
    }
    
    get_test_images().into_iter().nth((number - 1) as usize)
}

/// Load test image data by filename
pub async fn load_test_image(filename: &str) -> Result<Vec<u8>, std::io::Error> {
    let path = format!("tests/test_images/{}", filename);
    tokio::fs::read(path).await
}

/// Check if test images directory exists and is accessible
pub fn test_images_available() -> bool {
    Path::new("tests/test_images").exists()
}

/// Get available test images (only those that exist on filesystem)
pub fn get_available_test_images() -> Vec<TestImage> {
    get_test_images()
        .into_iter()
        .filter(|img| img.exists())
        .collect()
}

/// Skip test macro for conditional testing based on test image availability
#[macro_export]
macro_rules! skip_if_no_test_images {
    () => {
        if !crate::test_utils::test_images_available() {
            println!("Skipping test: test images directory not available");
            return;
        }
    };
}

/// Skip test macro for specific test image
#[macro_export]
macro_rules! skip_if_test_image_missing {
    ($image:expr) => {
        if !$image.exists() {
            println!("Skipping test: {} not found", $image.filename);
            return;
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_paths_are_valid() {
        let images = get_test_images();
        assert_eq!(images.len(), 9);
        
        for (i, image) in images.iter().enumerate() {
            assert_eq!(image.filename, format!("test{}.{}", i + 1, 
                if image.mime_type == "image/png" { "png" } 
                else if image.filename.ends_with(".jpg") { "jpg" }
                else { "jpeg" }
            ));
            assert!(image.expected_content.starts_with(&format!("Test {}", i + 1)));
        }
    }

    #[test]
    fn test_get_specific_image() {
        let image1 = get_test_image(1).unwrap();
        assert_eq!(image1.filename, "test1.png");
        assert_eq!(image1.mime_type, "image/png");
        assert!(image1.expected_content.contains("Test 1"));

        let image5 = get_test_image(5).unwrap();
        assert_eq!(image5.filename, "test5.jpg");
        assert_eq!(image5.mime_type, "image/jpeg");
        assert!(image5.expected_content.contains("Test 5"));

        // Invalid numbers should return None
        assert!(get_test_image(0).is_none());
        assert!(get_test_image(10).is_none());
    }
}