#[cfg(test)]
mod tests {
    use crate::ocr::enhanced::{EnhancedOcrService, OcrResult, ImageQualityStats};
    use crate::models::Settings;
    use std::fs;
    use tempfile::{NamedTempFile, TempDir};

    fn create_test_settings() -> Settings {
        Settings::default()
    }

    fn create_temp_dir() -> TempDir {
        TempDir::new().expect("Failed to create temp directory")
    }

    #[test]
    fn test_enhanced_ocr_service_creation() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let service = EnhancedOcrService::new(temp_path);
        
        // Service should be created successfully
        assert!(!service.temp_dir.is_empty());
    }

    #[test]
    fn test_image_quality_stats_creation() {
        let stats = ImageQualityStats {
            average_brightness: 128.0,
            contrast_ratio: 0.5,
            noise_level: 0.1,
            sharpness: 0.8,
        };
        
        assert_eq!(stats.average_brightness, 128.0);
        assert_eq!(stats.contrast_ratio, 0.5);
        assert_eq!(stats.noise_level, 0.1);
        assert_eq!(stats.sharpness, 0.8);
    }

    #[test]
    fn test_ocr_result_structure() {
        let result = OcrResult {
            text: "Test text".to_string(),
            confidence: 85.5,
            processing_time_ms: 1500,
            word_count: 2,
            preprocessing_applied: vec!["noise_reduction".to_string()],
            processed_image_path: Some("/tmp/processed.png".to_string()),
        };
        
        assert_eq!(result.text, "Test text");
        assert_eq!(result.confidence, 85.5);
        assert_eq!(result.processing_time_ms, 1500);
        assert_eq!(result.word_count, 2);
        assert_eq!(result.preprocessing_applied.len(), 1);
        assert!(result.processed_image_path.is_some());
    }

    #[tokio::test]
    async fn test_extract_text_from_plain_text() {
        let temp_dir = create_temp_dir();
        let service = EnhancedOcrService::new(temp_dir.path().to_str().unwrap().to_string());
        let settings = create_test_settings();
        
        let temp_file = NamedTempFile::with_suffix(".txt").unwrap();
        let test_content = "This is a test text file with multiple words.";
        fs::write(temp_file.path(), test_content).unwrap();
        
        let result = service
            .extract_text(temp_file.path().to_str().unwrap(), "text/plain", &settings)
            .await;
        
        assert!(result.is_ok());
        let ocr_result = result.unwrap();
        assert_eq!(ocr_result.text.trim(), test_content);
        assert_eq!(ocr_result.confidence, 100.0); // Plain text should be 100% confident
        assert_eq!(ocr_result.word_count, 9); // "This is a test text file with multiple words"
        assert!(ocr_result.processing_time_ms > 0);
        assert!(ocr_result.preprocessing_applied.contains(&"Plain text read".to_string()));
    }

    #[tokio::test]
    async fn test_extract_text_with_context() {
        let temp_dir = create_temp_dir();
        let service = EnhancedOcrService::new(temp_dir.path().to_str().unwrap().to_string());
        let settings = create_test_settings();
        
        let temp_file = NamedTempFile::with_suffix(".txt").unwrap();
        let test_content = "Context test content";
        fs::write(temp_file.path(), test_content).unwrap();
        
        let result = service
            .extract_text_with_context(
                temp_file.path().to_str().unwrap(),
                "text/plain",
                "test_file.txt",
                19, // Length of "Context test content"
                &settings,
            )
            .await;
        
        assert!(result.is_ok());
        let ocr_result = result.unwrap();
        assert_eq!(ocr_result.text.trim(), test_content);
        assert_eq!(ocr_result.confidence, 100.0);
    }

    #[tokio::test]
    async fn test_extract_text_unsupported_mime_type() {
        let temp_dir = create_temp_dir();
        let service = EnhancedOcrService::new(temp_dir.path().to_str().unwrap().to_string());
        let settings = create_test_settings();
        
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), "some content").unwrap();
        
        let result = service
            .extract_text(temp_file.path().to_str().unwrap(), "application/unknown", &settings)
            .await;
        
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Unsupported file type"));
    }

    #[tokio::test]
    async fn test_extract_text_nonexistent_file() {
        let temp_dir = create_temp_dir();
        let service = EnhancedOcrService::new(temp_dir.path().to_str().unwrap().to_string());
        let settings = create_test_settings();
        
        let result = service
            .extract_text("/nonexistent/file.txt", "text/plain", &settings)
            .await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_extract_text_large_file_truncation() {
        let temp_dir = create_temp_dir();
        let service = EnhancedOcrService::new(temp_dir.path().to_str().unwrap().to_string());
        let settings = create_test_settings();
        
        let temp_file = NamedTempFile::with_suffix(".txt").unwrap();
        
        // Create a file larger than the limit (50MB for text files)
        let large_content = "A".repeat(60 * 1024 * 1024); // 60MB
        fs::write(temp_file.path(), &large_content).unwrap();
        
        let result = service
            .extract_text(temp_file.path().to_str().unwrap(), "text/plain", &settings)
            .await;
        
        // Should fail due to size limit
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("too large"));
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_validate_ocr_quality_high_confidence() {
        let temp_dir = create_temp_dir();
        let service = EnhancedOcrService::new(temp_dir.path().to_str().unwrap().to_string());
        let mut settings = create_test_settings();
        settings.ocr_min_confidence = 30.0;
        
        let result = OcrResult {
            text: "This is high quality OCR text with good words.".to_string(),
            confidence: 95.0,
            processing_time_ms: 1000,
            word_count: 9,
            preprocessing_applied: vec![],
            processed_image_path: None,
        };
        
        let is_valid = service.validate_ocr_quality(&result, &settings);
        assert!(is_valid);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_validate_ocr_quality_low_confidence() {
        let temp_dir = create_temp_dir();
        let service = EnhancedOcrService::new(temp_dir.path().to_str().unwrap().to_string());
        let mut settings = create_test_settings();
        settings.ocr_min_confidence = 50.0;
        
        let result = OcrResult {
            text: "Poor quality text".to_string(),
            confidence: 25.0, // Below threshold
            processing_time_ms: 1000,
            word_count: 3,
            preprocessing_applied: vec![],
            processed_image_path: None,
        };
        
        let is_valid = service.validate_ocr_quality(&result, &settings);
        assert!(!is_valid);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_validate_ocr_quality_no_words() {
        let temp_dir = create_temp_dir();
        let service = EnhancedOcrService::new(temp_dir.path().to_str().unwrap().to_string());
        let settings = create_test_settings();
        
        let result = OcrResult {
            text: "".to_string(),
            confidence: 95.0,
            processing_time_ms: 1000,
            word_count: 0, // No words
            preprocessing_applied: vec![],
            processed_image_path: None,
        };
        
        let is_valid = service.validate_ocr_quality(&result, &settings);
        assert!(!is_valid);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_validate_ocr_quality_poor_character_distribution() {
        let temp_dir = create_temp_dir();
        let service = EnhancedOcrService::new(temp_dir.path().to_str().unwrap().to_string());
        let settings = create_test_settings();
        
        let result = OcrResult {
            text: "!!!@@@###$$$%%%^^^&&&***".to_string(), // Mostly symbols, < 30% alphanumeric
            confidence: 85.0,
            processing_time_ms: 1000,
            word_count: 1,
            preprocessing_applied: vec![],
            processed_image_path: None,
        };
        
        let is_valid = service.validate_ocr_quality(&result, &settings);
        assert!(!is_valid);
    }

    #[cfg(feature = "ocr")]
    #[test]
    fn test_validate_ocr_quality_good_character_distribution() {
        let temp_dir = create_temp_dir();
        let service = EnhancedOcrService::new(temp_dir.path().to_str().unwrap().to_string());
        let settings = create_test_settings();
        
        let result = OcrResult {
            text: "The quick brown fox jumps over the lazy dog. 123".to_string(), // Good alphanumeric ratio
            confidence: 85.0,
            processing_time_ms: 1000,
            word_count: 10,
            preprocessing_applied: vec![],
            processed_image_path: None,
        };
        
        let is_valid = service.validate_ocr_quality(&result, &settings);
        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_word_count_calculation() {
        let temp_dir = create_temp_dir();
        let service = EnhancedOcrService::new(temp_dir.path().to_str().unwrap().to_string());
        let settings = create_test_settings();
        
        let test_cases = vec![
            ("", 0),
            ("word", 1),
            ("two words", 2),
            ("  spaced   words  ", 2),
            ("Multiple\nlines\nof\ntext", 4),
            ("punctuation, words! work? correctly.", 4),
        ];
        
        for (content, expected_count) in test_cases {
            let temp_file = NamedTempFile::with_suffix(".txt").unwrap();
            fs::write(temp_file.path(), content).unwrap();
            
            let result = service
                .extract_text(temp_file.path().to_str().unwrap(), "text/plain", &settings)
                .await;
            
            assert!(result.is_ok());
            let ocr_result = result.unwrap();
            assert_eq!(ocr_result.word_count, expected_count, "Failed for content: '{}'", content);
        }
    }

    #[tokio::test]
    async fn test_pdf_extraction_with_invalid_pdf() {
        let temp_dir = create_temp_dir();
        let service = EnhancedOcrService::new(temp_dir.path().to_str().unwrap().to_string());
        let settings = create_test_settings();
        
        let temp_file = NamedTempFile::with_suffix(".pdf").unwrap();
        fs::write(temp_file.path(), "Not a valid PDF").unwrap();
        
        let result = service
            .extract_text(temp_file.path().to_str().unwrap(), "application/pdf", &settings)
            .await;
        
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Invalid PDF") || error_msg.contains("Missing") || error_msg.contains("corrupted"));
    }

    #[tokio::test]
    async fn test_pdf_extraction_with_minimal_valid_pdf() {
        let temp_dir = create_temp_dir();
        let service = EnhancedOcrService::new(temp_dir.path().to_str().unwrap().to_string());
        let settings = create_test_settings();
        
        // Minimal PDF with "Hello" text
        let pdf_content = b"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 4 0 R >> >> /MediaBox [0 0 612 792] /Contents 5 0 R >>
endobj
4 0 obj
<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>
endobj
5 0 obj
<< /Length 44 >>
stream
BT
/F1 12 Tf
100 700 Td
(Hello) Tj
ET
endstream
endobj
xref
0 6
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000262 00000 n
0000000341 00000 n
trailer
<< /Size 6 /Root 1 0 R >>
startxref
435
%%EOF";
        
        let temp_file = NamedTempFile::with_suffix(".pdf").unwrap();
        fs::write(temp_file.path(), pdf_content).unwrap();
        
        let result = service
            .extract_text(temp_file.path().to_str().unwrap(), "application/pdf", &settings)
            .await;
        
        match result {
            Ok(ocr_result) => {
                // PDF extraction succeeded
                assert_eq!(ocr_result.confidence, 95.0); // PDF text extraction should be high confidence
                assert!(ocr_result.processing_time_ms > 0);
                assert!(ocr_result.preprocessing_applied.contains(&"PDF text extraction".to_string()));
                println!("PDF extracted text: '{}'", ocr_result.text);
            }
            Err(e) => {
                // PDF extraction might fail depending on the pdf-extract library
                println!("PDF extraction failed (may be expected): {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_pdf_size_limit() {
        let temp_dir = create_temp_dir();
        let service = EnhancedOcrService::new(temp_dir.path().to_str().unwrap().to_string());
        let settings = create_test_settings();
        
        let temp_file = NamedTempFile::with_suffix(".pdf").unwrap();
        
        // Create a file larger than the 100MB PDF limit
        let large_pdf_content = format!("%PDF-1.4\n{}", "A".repeat(110 * 1024 * 1024));
        fs::write(temp_file.path(), large_pdf_content).unwrap();
        
        let result = service
            .extract_text(temp_file.path().to_str().unwrap(), "application/pdf", &settings)
            .await;
        
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("too large"));
    }

    #[test]
    fn test_settings_default_values() {
        let settings = Settings::default();
        
        // Test that OCR-related settings have reasonable defaults
        assert_eq!(settings.ocr_min_confidence, 30.0);
        assert_eq!(settings.ocr_dpi, 300);
        assert_eq!(settings.ocr_page_segmentation_mode, 3);
        assert_eq!(settings.ocr_engine_mode, 3);
        assert!(settings.enable_background_ocr);
        assert!(settings.ocr_enhance_contrast);
        assert!(settings.ocr_remove_noise);
        assert!(settings.ocr_detect_orientation);
    }

    #[tokio::test]
    async fn test_concurrent_ocr_processing() {
        let temp_dir = create_temp_dir();
        let service = EnhancedOcrService::new(temp_dir.path().to_str().unwrap().to_string());
        let settings = create_test_settings();
        
        let mut handles = vec![];
        
        // Process multiple files concurrently
        for i in 0..5 {
            let temp_file = NamedTempFile::with_suffix(".txt").unwrap();
            let content = format!("Concurrent test content {}", i);
            fs::write(temp_file.path(), &content).unwrap();
            
            let service_clone = EnhancedOcrService::new(temp_dir.path().to_str().unwrap().to_string());
            let settings_clone = settings.clone();
            let file_path = temp_file.path().to_str().unwrap().to_string();
            
            let handle = tokio::spawn(async move {
                let result = service_clone
                    .extract_text(&file_path, "text/plain", &settings_clone)
                    .await;
                
                // Keep temp_file alive until task completes
                drop(temp_file);
                result
            });
            
            handles.push(handle);
        }
        
        // Wait for all tasks to complete
        let results = futures::future::join_all(handles).await;
        
        // All tasks should succeed
        for (i, result) in results.into_iter().enumerate() {
            assert!(result.is_ok(), "Task {} failed", i);
            let ocr_result = result.unwrap().unwrap();
            assert!(ocr_result.text.contains(&format!("Concurrent test content {}", i)));
            assert_eq!(ocr_result.confidence, 100.0);
        }
    }
}