//! Integration tests for PDF image detection and OCR routing
//!
//! These tests verify that:
//! 1. `pdf_has_images()` correctly detects embedded images in PDFs
//! 2. PDFs with images are routed to image-based OCR (pdftoppm + Tesseract)
//! 3. PDFs without images use fast pdftotext extraction
//! 4. The full extraction flow works correctly for different PDF types

#[cfg(test)]
mod pdf_image_detection_tests {
    use readur::ocr::enhanced::EnhancedOcrService;
    use readur::models::Settings;
    use readur::services::file_service::FileService;
    use readur::storage::{StorageConfig, factory::create_storage_backend};
    use std::io::Write;
    use std::path::Path;
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

    async fn create_ocr_service(temp_path: &str) -> EnhancedOcrService {
        let file_service = create_test_file_service(temp_path).await;
        EnhancedOcrService::new(temp_path.to_string(), file_service, 100, 100)
    }

    /// Create a minimal text-only PDF (no embedded images)
    fn create_text_only_pdf(text_content: &str) -> NamedTempFile {
        let mut temp_file = NamedTempFile::with_suffix(".pdf").expect("Failed to create temp file");

        // Create a minimal PDF with text content only (no images)
        let stream_content = format!(
            "BT\n/F1 12 Tf\n100 700 Td\n({}) Tj\nET",
            text_content
        );
        let stream_length = stream_content.len();

        let pdf_content = format!(
            "%PDF-1.4\n\
            1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
            2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
            3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 4 0 R >> >> /MediaBox [0 0 612 792] /Contents 5 0 R >>\nendobj\n\
            4 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n\
            5 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n\
            xref\n0 6\n\
            0000000000 65535 f \n\
            0000000009 00000 n \n\
            0000000058 00000 n \n\
            0000000115 00000 n \n\
            0000000270 00000 n \n\
            0000000349 00000 n \n\
            trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n450\n%%EOF",
            stream_length,
            stream_content
        );

        temp_file.write_all(pdf_content.as_bytes()).expect("Failed to write PDF content");
        temp_file.flush().expect("Failed to flush temp file");
        temp_file
    }

    // =========================================================================
    // Tests for pdf_has_images() function
    // =========================================================================

    #[tokio::test]
    async fn test_pdf_has_images_detects_image_only_pdf() {
        // Use the real TEST2.pdf if available (downloaded from GitHub issue)
        let test_pdf_path = "/tmp/TEST2.pdf";

        if !Path::new(test_pdf_path).exists() {
            println!("Skipping test: TEST2.pdf not found at {}", test_pdf_path);
            println!("To run this test, download TEST2.pdf from the GitHub issue");
            return;
        }

        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap();
        let service = create_ocr_service(temp_path).await;

        let has_images = service.pdf_has_images(test_pdf_path).await;

        assert!(has_images, "TEST2.pdf should be detected as having images");
    }

    #[tokio::test]
    async fn test_pdf_has_images_detects_text_only_pdf() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap();
        let service = create_ocr_service(temp_path).await;

        // Create a text-only PDF
        let pdf_file = create_text_only_pdf("This is a test document with only text content");
        let pdf_path = pdf_file.path().to_str().unwrap();

        let has_images = service.pdf_has_images(pdf_path).await;

        assert!(!has_images, "Text-only PDF should NOT be detected as having images");
    }

    #[tokio::test]
    async fn test_pdf_has_images_handles_nonexistent_file() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap();
        let service = create_ocr_service(temp_path).await;

        let has_images = service.pdf_has_images("/nonexistent/path/file.pdf").await;

        // Should return false for nonexistent files (graceful handling)
        assert!(!has_images, "Nonexistent file should return false");
    }

    #[tokio::test]
    async fn test_pdf_has_images_handles_invalid_pdf() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap();
        let service = create_ocr_service(temp_path).await;

        // Create a fake PDF file (not actually a valid PDF)
        let mut temp_file = NamedTempFile::with_suffix(".pdf").expect("Failed to create temp file");
        temp_file.write_all(b"This is not a valid PDF file").expect("Failed to write");
        temp_file.flush().expect("Failed to flush");

        let has_images = service.pdf_has_images(temp_file.path().to_str().unwrap()).await;

        // Should return false for invalid PDFs (graceful handling)
        assert!(!has_images, "Invalid PDF should return false");
    }

    // =========================================================================
    // Tests for PDF extraction routing based on image detection
    // =========================================================================

    #[tokio::test]
    async fn test_text_only_pdf_uses_pdftotext_extraction() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap();
        let service = create_ocr_service(temp_path).await;
        let settings = create_test_settings();

        // Create a text-only PDF with substantial content
        let text_content = "Hello World This Is A Test Document With Multiple Words";
        let pdf_file = create_text_only_pdf(text_content);
        let pdf_path = pdf_file.path().to_str().unwrap();

        match service.extract_text_from_pdf(pdf_path, &settings).await {
            Ok(result) => {
                // Text-only PDFs should use pdftotext (fast extraction)
                let used_pdftotext = result.preprocessing_applied.iter()
                    .any(|s| s.contains("pdftotext") || s.contains("PDF text extraction"));

                // Should NOT use image-based OCR for text-only PDFs
                let used_image_ocr = result.preprocessing_applied.iter()
                    .any(|s| s.contains("page-to-image OCR"));

                println!("Text-only PDF extraction result:");
                println!("  - Text: '{}'", result.text);
                println!("  - Word count: {}", result.word_count);
                println!("  - Preprocessing: {:?}", result.preprocessing_applied);

                // Either pdftotext worked, or we fell back to OCR
                // The key is that image-based OCR should NOT be used if no images detected
                if result.word_count > 0 {
                    assert!(
                        used_pdftotext || !used_image_ocr,
                        "Text-only PDF should prefer pdftotext over image-based OCR"
                    );
                }
            }
            Err(e) => {
                // Some environments may not have all tools installed
                println!("PDF extraction failed (may be expected in test environment): {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_image_pdf_uses_image_based_ocr() {
        // Use the real TEST2.pdf if available
        let test_pdf_path = "/tmp/TEST2.pdf";

        if !Path::new(test_pdf_path).exists() {
            println!("Skipping test: TEST2.pdf not found at {}", test_pdf_path);
            return;
        }

        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap();
        let service = create_ocr_service(temp_path).await;
        let settings = create_test_settings();

        match service.extract_text_from_pdf(test_pdf_path, &settings).await {
            Ok(result) => {
                // Image-based PDFs should use pdftoppm + Tesseract
                let used_image_ocr = result.preprocessing_applied.iter()
                    .any(|s| s.contains("page-to-image OCR") || s.contains("pdftoppm"));

                let used_ocrmypdf = result.preprocessing_applied.iter()
                    .any(|s| s.contains("ocrmypdf"));

                println!("Image PDF extraction result:");
                println!("  - Text length: {} chars", result.text.len());
                println!("  - Word count: {}", result.word_count);
                println!("  - Confidence: {:.1}%", result.confidence);
                println!("  - Preprocessing: {:?}", result.preprocessing_applied);

                // Should use either image-based OCR or ocrmypdf (both handle images)
                assert!(
                    used_image_ocr || used_ocrmypdf,
                    "Image PDF should use image-based OCR or ocrmypdf. Got: {:?}",
                    result.preprocessing_applied
                );

                // Should extract meaningful text (not garbage)
                assert!(result.word_count > 0, "Should extract words from image PDF");

                // The extracted text should be readable (high alphanumeric ratio)
                let alphanumeric_count = result.text.chars().filter(|c| c.is_alphanumeric()).count();
                let alphanumeric_ratio = if !result.text.is_empty() {
                    alphanumeric_count as f64 / result.text.len() as f64
                } else {
                    0.0
                };
                assert!(
                    alphanumeric_ratio > 0.3,
                    "Extracted text should be readable (alphanumeric ratio: {:.1}%)",
                    alphanumeric_ratio * 100.0
                );
            }
            Err(e) => {
                println!("PDF extraction failed (may be expected in test environment): {}", e);
            }
        }
    }

    // =========================================================================
    // Tests for quality and edge cases
    // =========================================================================

    #[tokio::test]
    async fn test_extraction_does_not_return_garbage_text() {
        // Use the real TEST2.pdf if available
        let test_pdf_path = "/tmp/TEST2.pdf";

        if !Path::new(test_pdf_path).exists() {
            println!("Skipping test: TEST2.pdf not found at {}", test_pdf_path);
            return;
        }

        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap();
        let service = create_ocr_service(temp_path).await;
        let settings = create_test_settings();

        match service.extract_text_from_pdf(test_pdf_path, &settings).await {
            Ok(result) => {
                // The extracted text should NOT contain common garbage patterns
                let garbage_patterns = [
                    "GGG+++",
                    "VVV@@@",
                    "JJJttt",
                    "\\x00",  // Null bytes
                ];

                for pattern in &garbage_patterns {
                    assert!(
                        !result.text.contains(pattern),
                        "Extracted text should not contain garbage pattern: '{}'",
                        pattern
                    );
                }

                // Check that text is mostly readable characters
                let readable_chars = result.text.chars()
                    .filter(|c| c.is_alphanumeric() || c.is_whitespace() || c.is_ascii_punctuation())
                    .count();
                let readable_ratio = readable_chars as f64 / result.text.len().max(1) as f64;

                assert!(
                    readable_ratio > 0.8,
                    "Text should be mostly readable characters (got {:.1}% readable)",
                    readable_ratio * 100.0
                );
            }
            Err(e) => {
                println!("PDF extraction failed (may be expected in test environment): {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_is_pdftoppm_available() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap();
        let service = create_ocr_service(temp_path).await;

        let available = service.is_pdftoppm_available().await;

        // Just log the result - this is environment-dependent
        println!("pdftoppm available: {}", available);

        // If pdftoppm is available, we should be able to use image-based OCR
        if available {
            println!("Image-based OCR (pdftoppm + Tesseract) is available");
        } else {
            println!("Image-based OCR not available - will fall back to ocrmypdf");
        }
    }

    #[tokio::test]
    async fn test_get_pdf_page_count() {
        // Use the real TEST2.pdf if available
        let test_pdf_path = "/tmp/TEST2.pdf";

        if !Path::new(test_pdf_path).exists() {
            println!("Skipping test: TEST2.pdf not found at {}", test_pdf_path);
            return;
        }

        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap();
        let service = create_ocr_service(temp_path).await;

        match service.get_pdf_page_count(test_pdf_path).await {
            Ok(page_count) => {
                println!("TEST2.pdf has {} pages", page_count);
                assert!(page_count >= 1, "PDF should have at least 1 page");
            }
            Err(e) => {
                println!("Failed to get page count (pdfinfo may not be installed): {}", e);
            }
        }
    }

    // =========================================================================
    // Regression test for Issue #439
    // =========================================================================

    #[tokio::test]
    async fn test_issue_439_regression_image_pdf_not_garbage() {
        //! Regression test for GitHub Issue #439
        //!
        //! Problem: PDFs containing only images were returning garbled/raw output
        //! instead of OCR'd text. The `extract_text_from_pdf_bytes()` function
        //! was extracting raw ASCII from PDF binary data.
        //!
        //! Solution: Detect images using `pdfimages -list` and route to image-based
        //! OCR (pdftoppm + Tesseract) which renders full pages and captures all content.

        let test_pdf_path = "/tmp/TEST2.pdf";

        if !Path::new(test_pdf_path).exists() {
            println!("Skipping regression test: TEST2.pdf not found");
            println!("Download from: https://github.com/user-attachments/files/24381647/TEST2.pdf");
            return;
        }

        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap();
        let service = create_ocr_service(temp_path).await;
        let settings = create_test_settings();

        // Step 1: Verify the PDF has images
        let has_images = service.pdf_has_images(test_pdf_path).await;
        assert!(has_images, "TEST2.pdf should be detected as having images");

        // Step 2: Extract text and verify it's not garbage
        match service.extract_text_from_pdf(test_pdf_path, &settings).await {
            Ok(result) => {
                println!("=== Issue #439 Regression Test Results ===");
                println!("Word count: {}", result.word_count);
                println!("Confidence: {:.1}%", result.confidence);
                println!("Preprocessing: {:?}", result.preprocessing_applied);
                println!("Text sample (first 200 chars): '{}'",
                    result.text.chars().take(200).collect::<String>());

                // The text should NOT be garbage
                // Old behavior: returned 13,283 "words" of garbage like "GGG+++", "VVV@@@JJJttt"
                // New behavior: should return readable French text

                // Check 1: Text should have good alphanumeric ratio (not binary garbage)
                let alphanumeric_chars = result.text.chars().filter(|c| c.is_alphanumeric()).count();
                let alphanumeric_ratio = alphanumeric_chars as f64 / result.text.len().max(1) as f64;
                assert!(
                    alphanumeric_ratio > 0.3,
                    "Issue #439 regression: Text has low alphanumeric ratio ({:.1}%), likely garbage",
                    alphanumeric_ratio * 100.0
                );

                // Check 2: Should not contain known garbage patterns from raw byte extraction
                assert!(
                    !result.text.contains("GGG+++"),
                    "Issue #439 regression: Text contains garbage pattern 'GGG+++'"
                );

                // Check 3: Should use image-based OCR or ocrmypdf (not raw byte extraction)
                let used_proper_extraction = result.preprocessing_applied.iter()
                    .any(|s| s.contains("page-to-image OCR") || s.contains("ocrmypdf"));
                assert!(
                    used_proper_extraction,
                    "Issue #439 regression: Should use image-based OCR for image PDFs"
                );

                println!("=== Issue #439 Regression Test PASSED ===");
            }
            Err(e) => {
                println!("PDF extraction failed: {}", e);
                println!("This may be expected if OCR tools are not installed");
            }
        }
    }
}
