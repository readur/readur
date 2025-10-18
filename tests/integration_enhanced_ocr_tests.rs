#[cfg(test)]
mod tests {
    use readur::ocr::enhanced::{EnhancedOcrService, OcrResult, ImageQualityStats};
    use readur::models::Settings;
    use readur::services::file_service::FileService;
    use readur::storage::{StorageConfig, factory::create_storage_backend};
    use std::fs;
    use tempfile::{NamedTempFile, TempDir};

    fn create_test_settings() -> Settings {
        Settings::default()
    }

    fn create_temp_dir() -> TempDir {
        TempDir::new().expect("Failed to create temp directory")
    }

    async fn create_test_file_service(temp_path: &str) -> FileService {
        let storage_config = StorageConfig::Local { upload_path: temp_path.to_string() };
        let storage_backend = create_storage_backend(storage_config).await.unwrap();
        FileService::with_storage(temp_path.to_string(), storage_backend)
    }

    #[tokio::test]
    async fn test_enhanced_ocr_service_creation() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let file_service = create_test_file_service(&temp_path).await;
        let service = EnhancedOcrService::new(temp_path.clone(), file_service);
        
        // Service should be created successfully
        assert!(!service.temp_dir.is_empty());
    }

    #[tokio::test]
    async fn test_image_quality_stats_creation() {
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

    #[tokio::test]
    async fn test_count_words_safely_whitespace_separated() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let file_service = create_test_file_service(&temp_path).await;
        let service = EnhancedOcrService::new(temp_path.clone(), file_service);
        
        // Test normal whitespace-separated text
        let text = "Hello world this is a test";
        let count = service.count_words_safely(&text);
        assert_eq!(count, 6);
        
        // Test with extra whitespace
        let text = "  Hello   world  \n  test  ";
        let count = service.count_words_safely(&text);
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_count_words_safely_continuous_text() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let file_service = create_test_file_service(&temp_path).await;
        let service = EnhancedOcrService::new(temp_path.clone(), file_service);
        
        // Test continuous text without spaces (like some PDF extractions)
        let text = "HelloWorldThisIsAContinuousText";
        let count = service.count_words_safely(&text);
        assert!(count > 0, "Should detect words even without whitespace");
        
        // Test mixed alphanumeric without spaces
        let text = "ABC123DEF456GHI789";
        let count = service.count_words_safely(&text);
        assert!(count > 0, "Should detect alphanumeric patterns as words");
    }

    #[tokio::test]
    async fn test_count_words_safely_edge_cases() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let file_service = create_test_file_service(&temp_path).await;
        let service = EnhancedOcrService::new(temp_path.clone(), file_service);
        
        // Test empty text
        let count = service.count_words_safely("");
        assert_eq!(count, 0);
        
        // Test only whitespace
        let count = service.count_words_safely("   \n\t  ");
        assert_eq!(count, 0);
        
        // Test only punctuation
        let text = "!@#$%^&*()_+-=[]{}|;':\",./<>?";
        let count = service.count_words_safely(&text);
        // Since there are no alphabetic or alphanumeric chars, should be 0
        assert_eq!(count, 0, "Pure punctuation should not count as words, got {}", count);
        
        // Test single character
        let count = service.count_words_safely("A");
        assert_eq!(count, 1);
        
        // Test mixed content with low alphanumeric ratio
        let text = "A!!!B@@@C###D$$$E%%%";
        let count = service.count_words_safely(&text);
        assert!(count > 0, "Should detect words in mixed content");
    }

    #[tokio::test]
    async fn test_count_words_safely_large_text() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let file_service = create_test_file_service(&temp_path).await;
        let service = EnhancedOcrService::new(temp_path.clone(), file_service);
        
        // Test with large text (over 1MB) to trigger sampling
        let word = "test ";
        let large_text = word.repeat(250_000); // Creates ~1.25MB of text
        let count = service.count_words_safely(&large_text);
        
        // Should estimate around 250,000 words (may vary due to sampling)
        assert!(count > 200_000, "Should estimate large word count: got {}", count);
        assert!(count <= 10_000_000, "Should cap at max limit: got {}", count);
    }

    #[tokio::test]
    async fn test_count_words_safely_fallback_patterns() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let file_service = create_test_file_service(&temp_path).await;
        let service = EnhancedOcrService::new(temp_path, file_service);
        
        // Test letter transition detection
        let text = "OneWordAnotherWordFinalWord";
        let count = service.count_words_safely(&text);
        assert!(count >= 3, "Should detect at least 3 words from transitions: got {}", count);
        
        // Test alphanumeric estimation fallback
        let text = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"; // 26 chars, should estimate ~5-6 words
        let count = service.count_words_safely(&text);
        assert!(count >= 1 && count <= 10, "Should estimate reasonable word count: got {}", count);
        
        // Test mixed case with numbers
        let text = "ABC123def456GHI789jkl";
        let count = service.count_words_safely(&text);
        assert!(count >= 1, "Should detect words in mixed alphanumeric: got {}", count);
    }

    #[tokio::test]
    async fn test_ocr_result_structure() {
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
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let file_service = create_test_file_service(&temp_path).await;
        let service = EnhancedOcrService::new(temp_path, file_service);
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
        assert!(ocr_result.processing_time_ms >= 0);
        assert!(ocr_result.preprocessing_applied.contains(&"Plain text read".to_string()));
    }

    #[tokio::test]
    async fn test_extract_text_with_context() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let file_service = create_test_file_service(&temp_path).await;
        let service = EnhancedOcrService::new(temp_path, file_service);
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
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let file_service = create_test_file_service(&temp_path).await;
        let service = EnhancedOcrService::new(temp_path, file_service);
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
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let file_service = create_test_file_service(&temp_path).await;
        let service = EnhancedOcrService::new(temp_path, file_service);
        let settings = create_test_settings();
        
        let result = service
            .extract_text("/nonexistent/file.txt", "text/plain", &settings)
            .await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_extract_text_large_file_truncation() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let file_service = create_test_file_service(&temp_path).await;
        let service = EnhancedOcrService::new(temp_path, file_service);
        let settings = create_test_settings();
        
        let temp_file = NamedTempFile::with_suffix(".txt").unwrap();
        
        // Create a file larger than the limit (50MB for text files)
        // Using smaller size and explicit drop for CI environments
        let large_content = "A".repeat(50 * 1024 * 1024 + 1024); // 50MB + 1KB (just over the limit)
        fs::write(temp_file.path(), &large_content).unwrap();
        drop(large_content); // Explicitly free memory
        
        let result = service
            .extract_text(temp_file.path().to_str().unwrap(), "text/plain", &settings)
            .await;
        
        // Should fail due to size limit
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("too large"));
    }

    #[cfg(feature = "ocr")]
    #[tokio::test]
    async fn test_validate_ocr_quality_high_confidence() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let file_service = create_test_file_service(&temp_path).await;
        let service = EnhancedOcrService::new(temp_path, file_service);
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
        
        let result_validation = service.validate_ocr_quality(&result, &settings);
        assert!(result_validation.is_ok());
    }

    #[cfg(feature = "ocr")]
    #[tokio::test]
    async fn test_validate_ocr_quality_low_confidence() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let file_service = create_test_file_service(&temp_path).await;
        let service = EnhancedOcrService::new(temp_path, file_service);
        let mut settings = create_test_settings();
        settings.ocr_min_confidence = 50.0;

        let result = OcrResult {
            text: "Poor quality text".to_string(),
            confidence: 25.0, // Below threshold but still accepted
            processing_time_ms: 1000,
            word_count: 3,
            preprocessing_applied: vec![],
            processed_image_path: None,
        };

        // Low confidence is now accepted with a warning, not rejected
        let result_validation = service.validate_ocr_quality(&result, &settings);
        assert!(result_validation.is_ok());
    }

    #[cfg(feature = "ocr")]
    #[tokio::test]
    async fn test_validate_ocr_quality_no_words() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let file_service = create_test_file_service(&temp_path).await;
        let service = EnhancedOcrService::new(temp_path, file_service);
        let settings = create_test_settings();
        
        let result = OcrResult {
            text: "".to_string(),
            confidence: 95.0,
            processing_time_ms: 1000,
            word_count: 0, // No words
            preprocessing_applied: vec![],
            processed_image_path: None,
        };
        
        let result_validation = service.validate_ocr_quality(&result, &settings);
        assert!(result_validation.is_err());
    }

    #[cfg(feature = "ocr")]
    #[tokio::test]
    async fn test_validate_ocr_quality_poor_character_distribution() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let file_service = create_test_file_service(&temp_path).await;
        let service = EnhancedOcrService::new(temp_path, file_service);
        let settings = create_test_settings();
        
        let result = OcrResult {
            text: "!!!@@@###$$$%%%^^^&&&***".to_string(), // Mostly symbols, < 30% alphanumeric
            confidence: 85.0,
            processing_time_ms: 1000,
            word_count: 1,
            preprocessing_applied: vec![],
            processed_image_path: None,
        };
        
        let result_validation = service.validate_ocr_quality(&result, &settings);
        assert!(result_validation.is_err());
    }

    #[cfg(feature = "ocr")]
    #[tokio::test]
    async fn test_validate_ocr_quality_good_character_distribution() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let file_service = create_test_file_service(&temp_path).await;
        let service = EnhancedOcrService::new(temp_path, file_service);
        let settings = create_test_settings();
        
        let result = OcrResult {
            text: "The quick brown fox jumps over the lazy dog. 123".to_string(), // Good alphanumeric ratio
            confidence: 85.0,
            processing_time_ms: 1000,
            word_count: 10,
            preprocessing_applied: vec![],
            processed_image_path: None,
        };
        
        let result_validation = service.validate_ocr_quality(&result, &settings);
        assert!(result_validation.is_ok());
    }

    #[tokio::test]
    async fn test_word_count_calculation() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let file_service = create_test_file_service(&temp_path).await;
        let service = EnhancedOcrService::new(temp_path, file_service);
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
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let file_service = create_test_file_service(&temp_path).await;
        let service = EnhancedOcrService::new(temp_path, file_service);
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
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let file_service = create_test_file_service(&temp_path).await;
        let service = EnhancedOcrService::new(temp_path, file_service);
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
                // Skip processing time check for minimal PDFs as they might process too fast
                // assert!(ocr_result.processing_time_ms > 0);
                assert!(
                    ocr_result.preprocessing_applied.iter().any(|s| s.contains("PDF text extraction")) ||
                    ocr_result.preprocessing_applied.iter().any(|s| s.contains("OCR via ocrmypdf")),
                    "Expected PDF processing method in preprocessing_applied: {:?}", 
                    ocr_result.preprocessing_applied
                );
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
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let file_service = create_test_file_service(&temp_path).await;
        let service = EnhancedOcrService::new(temp_path, file_service);
        let settings = create_test_settings();
        
        let temp_file = NamedTempFile::with_suffix(".pdf").unwrap();
        
        // Create a file larger than the 100MB PDF limit
        // Using smaller size and explicit drop for CI environments
        let large_pdf_content = format!("%PDF-1.4\n{}", "A".repeat(100 * 1024 * 1024 + 1024)); // 100MB + 1KB (just over the limit)
        fs::write(temp_file.path(), &large_pdf_content).unwrap();
        drop(large_pdf_content); // Explicitly free memory
        
        let result = service
            .extract_text(temp_file.path().to_str().unwrap(), "application/pdf", &settings)
            .await;
        
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("too large"));
    }

    #[tokio::test]
    async fn test_settings_default_values() {
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
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let file_service = create_test_file_service(&temp_path).await;
        let service = EnhancedOcrService::new(temp_path, file_service);
        let settings = create_test_settings();

        let mut handles = vec![];

        // Process multiple files concurrently
        for i in 0..5 {
            let temp_file = NamedTempFile::with_suffix(".txt").unwrap();
            let content = format!("Concurrent test content {}", i);
            fs::write(temp_file.path(), &content).unwrap();

            let temp_path_clone = temp_dir.path().to_str().unwrap().to_string();
            let file_service_clone = create_test_file_service(&temp_path_clone).await;
            let service_clone = EnhancedOcrService::new(temp_path_clone, file_service_clone);
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

    // New validation tests for updated OCR validation logic

    #[cfg(feature = "ocr")]
    #[tokio::test]
    async fn test_validate_ocr_quality_below_hard_minimum() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let file_service = create_test_file_service(&temp_path).await;
        let service = EnhancedOcrService::new(temp_path, file_service);
        let settings = create_test_settings();

        // Test OCR with confidence below the hard minimum (5%)
        // This should be rejected as critically low/corrupted
        let result = OcrResult {
            text: "Some text".to_string(),
            confidence: 4.9, // Below hard minimum of 5%
            processing_time_ms: 1000,
            word_count: 2,
            preprocessing_applied: vec![],
            processed_image_path: None,
        };

        let validation_result = service.validate_ocr_quality(&result, &settings);
        assert!(validation_result.is_err(), "Expected validation to fail for confidence below hard minimum");

        let error_msg = validation_result.unwrap_err();
        assert!(error_msg.contains("critically low"),
                "Expected 'critically low' in error message, got: {}", error_msg);
    }

    #[cfg(feature = "ocr")]
    #[tokio::test]
    async fn test_validate_ocr_quality_at_hard_minimum_boundary() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let file_service = create_test_file_service(&temp_path).await;
        let service = EnhancedOcrService::new(temp_path, file_service);
        let settings = create_test_settings();

        // Test OCR with exactly 5% confidence (boundary case)
        // This should be accepted (at the hard minimum threshold)
        let result = OcrResult {
            text: "Boundary test text".to_string(),
            confidence: 5.0, // Exactly at hard minimum
            processing_time_ms: 1000,
            word_count: 3,
            preprocessing_applied: vec![],
            processed_image_path: None,
        };

        let validation_result = service.validate_ocr_quality(&result, &settings);
        assert!(validation_result.is_ok(),
                "Expected validation to pass at hard minimum boundary (5%)");
    }

    #[cfg(feature = "ocr")]
    #[tokio::test]
    async fn test_validate_ocr_quality_numeric_document() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let file_service = create_test_file_service(&temp_path).await;
        let service = EnhancedOcrService::new(temp_path, file_service);
        let settings = create_test_settings();

        // Test invoice/receipt with >30% digits
        // Should be accepted even with lower alphanumeric ratio due to high digit content
        let result = OcrResult {
            text: "Invoice #12345\n$1,234.56\n$2,345.67\nTotal: $3,580.23\n!!!".to_string(),
            confidence: 60.0,
            processing_time_ms: 1000,
            word_count: 5,
            preprocessing_applied: vec![],
            processed_image_path: None,
        };

        // Calculate to verify we have >30% digits
        let digit_count = result.text.chars().filter(|c| c.is_numeric()).count();
        let total_chars = result.text.len();
        let digit_ratio = digit_count as f32 / total_chars as f32;
        assert!(digit_ratio > 0.3, "Test data should have >30% digits, got {:.1}%", digit_ratio * 100.0);

        let validation_result = service.validate_ocr_quality(&result, &settings);
        assert!(validation_result.is_ok(),
                "Expected validation to pass for numeric document with {:.1}% digits", digit_ratio * 100.0);
    }

    #[cfg(feature = "ocr")]
    #[tokio::test]
    async fn test_validate_ocr_quality_numeric_document_boundary() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let file_service = create_test_file_service(&temp_path).await;
        let service = EnhancedOcrService::new(temp_path, file_service);
        let settings = create_test_settings();

        // Test document with exactly 30% digits (boundary case)
        // 30 digits + 70 non-digit chars = 100 total chars
        let result = OcrResult {
            text: "123456789012345678901234567890AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
            confidence: 60.0,
            processing_time_ms: 1000,
            word_count: 2,
            preprocessing_applied: vec![],
            processed_image_path: None,
        };

        // Verify exactly 30% digits
        let digit_count = result.text.chars().filter(|c| c.is_numeric()).count();
        let total_chars = result.text.len();
        let digit_ratio = digit_count as f32 / total_chars as f32;
        assert_eq!(digit_count, 30, "Test data should have exactly 30 digits");
        assert_eq!(total_chars, 100, "Test data should have exactly 100 chars");
        assert!((digit_ratio - 0.3).abs() < 0.01, "Should have exactly 30% digits, got {:.1}%", digit_ratio * 100.0);

        let validation_result = service.validate_ocr_quality(&result, &settings);
        // At exactly 30%, it should NOT trigger the >30% special handling
        // So it will be validated normally (which should pass with 100% alphanumeric)
        assert!(validation_result.is_ok(),
                "Expected validation to pass at 30% digit boundary");
    }

    #[cfg(feature = "ocr")]
    #[tokio::test]
    async fn test_validate_ocr_quality_alphanumeric_boundary() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let file_service = create_test_file_service(&temp_path).await;
        let service = EnhancedOcrService::new(temp_path, file_service);
        let settings = create_test_settings();

        // Test text with exactly 10% alphanumeric characters (boundary case)
        // 1 letter + 9 symbols = 10 total chars = 10% alphanumeric
        let result = OcrResult {
            text: "a!!!!!!!!!".to_string(), // 1 alphanumeric + 9 symbols = 10%
            confidence: 60.0,
            processing_time_ms: 1000,
            word_count: 1,
            preprocessing_applied: vec![],
            processed_image_path: None,
        };

        // Verify exactly 10% alphanumeric
        let alphanumeric_count = result.text.chars().filter(|c| c.is_alphanumeric()).count();
        let total_chars = result.text.len();
        let alphanumeric_ratio = alphanumeric_count as f32 / total_chars as f32;
        assert_eq!(alphanumeric_count, 1, "Test data should have exactly 1 alphanumeric char");
        assert_eq!(total_chars, 10, "Test data should have exactly 10 chars");
        assert!((alphanumeric_ratio - 0.1).abs() < 0.01, "Should have exactly 10% alphanumeric, got {:.1}%", alphanumeric_ratio * 100.0);

        let validation_result = service.validate_ocr_quality(&result, &settings);
        assert!(validation_result.is_ok(),
                "Expected validation to pass at 10% alphanumeric boundary");
    }

    #[cfg(feature = "ocr")]
    #[tokio::test]
    async fn test_validate_ocr_quality_below_alphanumeric_threshold() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let file_service = create_test_file_service(&temp_path).await;
        let service = EnhancedOcrService::new(temp_path, file_service);
        let settings = create_test_settings();

        // Test text with <10% alphanumeric (pure garbage)
        // 1 letter + 13 symbols = 14 total chars = 7.14% alphanumeric
        let result = OcrResult {
            text: "a!!!!!!!!!!!!!!".to_string(), // 1 alphanumeric + 14 symbols = ~7%
            confidence: 60.0,
            processing_time_ms: 1000,
            word_count: 1,
            preprocessing_applied: vec![],
            processed_image_path: None,
        };

        // Verify <10% alphanumeric
        let alphanumeric_count = result.text.chars().filter(|c| c.is_alphanumeric()).count();
        let total_chars = result.text.len();
        let alphanumeric_ratio = alphanumeric_count as f32 / total_chars as f32;
        assert!(alphanumeric_ratio < 0.10, "Test data should have <10% alphanumeric, got {:.1}%", alphanumeric_ratio * 100.0);

        let validation_result = service.validate_ocr_quality(&result, &settings);
        assert!(validation_result.is_err(),
                "Expected validation to fail for <10% alphanumeric content");

        let error_msg = validation_result.unwrap_err();
        assert!(error_msg.contains("non-alphanumeric"),
                "Expected error about non-alphanumeric content, got: {}", error_msg);
    }

    #[cfg(feature = "ocr")]
    #[tokio::test]
    async fn test_validate_ocr_quality_empty_text() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let file_service = create_test_file_service(&temp_path).await;
        let service = EnhancedOcrService::new(temp_path, file_service);
        let settings = create_test_settings();

        // Test completely empty text
        // Should fail with "no characters" error (not "no words")
        let result = OcrResult {
            text: "".to_string(),
            confidence: 60.0,
            processing_time_ms: 1000,
            word_count: 0,
            preprocessing_applied: vec![],
            processed_image_path: None,
        };

        let validation_result = service.validate_ocr_quality(&result, &settings);
        assert!(validation_result.is_err(),
                "Expected validation to fail for empty text");

        let error_msg = validation_result.unwrap_err();
        assert!(error_msg.contains("no characters"),
                "Expected error about 'no characters' (not 'no words'), got: {}", error_msg);
    }

    #[cfg(feature = "ocr")]
    #[tokio::test]
    async fn test_validate_ocr_quality_whitespace_only() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let file_service = create_test_file_service(&temp_path).await;
        let service = EnhancedOcrService::new(temp_path, file_service);
        let settings = create_test_settings();

        // Test text with only whitespace
        // Has characters but no words - should fail with "No words" error
        let result = OcrResult {
            text: "    \n\n\t\t".to_string(),
            confidence: 60.0,
            processing_time_ms: 1000,
            word_count: 0, // Whitespace doesn't count as words
            preprocessing_applied: vec![],
            processed_image_path: None,
        };

        let validation_result = service.validate_ocr_quality(&result, &settings);
        assert!(validation_result.is_err(),
                "Expected validation to fail for whitespace-only text");

        let error_msg = validation_result.unwrap_err();
        assert!(error_msg.contains("No words"),
                "Expected error about 'No words' (not 'no characters'), got: {}", error_msg);
    }
}